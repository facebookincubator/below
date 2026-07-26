// Copyright (c) Facebook, Inc. and its affiliates.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Open-source client for below's remote-viewing mode.
//!
//! Speaks a tiny HTTP+CBOR protocol to a remote `below record` server: a GET to
//! `/get_frame?timestamp=<unix>&direction=<forward|reverse>` returns a
//! CBOR-encoded `Option<(u64, DataFrame)>`.
//!
//! Authentication is a shared bearer token, intended to be delivered as a
//! Kubernetes Secret. The token is read from the `BELOW_REMOTE_TOKEN`
//! environment variable, or from the file named by `BELOW_REMOTE_TOKEN_FILE`.
//!
//! TLS is opt-in via environment variables (so no per-command flags are
//! required):
//!   * `BELOW_REMOTE_TLS=1`        -- connect over HTTPS, verifying the server
//!                                    against the system trust store.
//!   * `BELOW_REMOTE_CA_FILE=<pem>`-- connect over HTTPS, verifying against this
//!                                    CA bundle (for self-signed / internal CAs).
//!   * `BELOW_REMOTE_TLS_INSECURE=1` -- connect over HTTPS without verifying the
//!                                    server certificate (testing only).

use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use rustls::ClientConfig;
use rustls::DigitallySignedStruct;
use rustls::RootCertStore;
use rustls::SignatureScheme;
use rustls::client::danger::HandshakeSignatureValid;
use rustls::client::danger::ServerCertVerified;
use rustls::client::danger::ServerCertVerifier;
use rustls::crypto::CryptoProvider;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::ServerName;
use rustls::pki_types::UnixTime;

use crate::DataFrame;
use crate::Direction;

/// Default port used when none is supplied.
pub const DEFAULT_REMOTE_PORT: u16 = 1969;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// How the client should verify the server when TLS is enabled.
enum TlsMode {
    /// Verify against the system trust store.
    Default,
    /// Verify against a specific CA bundle (PEM).
    Ca(PathBuf),
    /// Do not verify the server certificate (testing only).
    Insecure,
}

pub struct RemoteStore {
    base_url: String,
    token: Option<String>,
    agent: ureq::Agent,
}

impl RemoteStore {
    pub fn new(host: String, port: Option<u16>) -> Result<RemoteStore> {
        let port = port.unwrap_or(DEFAULT_REMOTE_PORT);
        let tls = read_tls_mode();
        let scheme = if tls.is_some() { "https" } else { "http" };

        let mut builder = ureq::AgentBuilder::new().timeout(REQUEST_TIMEOUT);
        match &tls {
            Some(TlsMode::Ca(path)) => {
                builder = builder.tls_config(Arc::new(build_ca_config(path)?));
            }
            Some(TlsMode::Insecure) => {
                builder = builder.tls_config(Arc::new(build_insecure_config()));
            }
            // Default TLS uses ureq's built-in rustls config (system roots);
            // plain HTTP needs no TLS config.
            Some(TlsMode::Default) | None => {}
        }

        Ok(RemoteStore {
            base_url: format!("{scheme}://{host}:{port}"),
            token: read_token()?,
            agent: builder.build(),
        })
    }

    pub fn get_frame(
        &mut self,
        timestamp: u64,
        direction: Direction,
    ) -> Result<Option<(SystemTime, DataFrame)>> {
        let direction = match direction {
            Direction::Forward => "forward",
            Direction::Reverse => "reverse",
        };
        let url = format!(
            "{}/get_frame?timestamp={timestamp}&direction={direction}",
            self.base_url
        );

        let mut req = self.agent.get(&url);
        if let Some(token) = &self.token {
            req = req.set("Authorization", &format!("Bearer {token}"));
        }

        let resp = match req.call() {
            Ok(resp) => resp,
            Err(ureq::Error::Status(401, _)) => {
                bail!(
                    "Remote server rejected credentials (401). Set BELOW_REMOTE_TOKEN (or \
                     BELOW_REMOTE_TOKEN_FILE) to match the server's token."
                )
            }
            Err(ureq::Error::Status(code, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                bail!("Remote server returned HTTP {code}: {body}")
            }
            Err(e) => return Err(anyhow!(e).context("Remote frame request failed")),
        };

        let mut buf = Vec::new();
        resp.into_reader()
            .read_to_end(&mut buf)
            .context("Failed to read remote response body")?;
        let frame: Option<(u64, DataFrame)> =
            serde_cbor::from_slice(&buf).context("Failed to deserialize remote frame")?;

        Ok(frame.map(|(ts, df)| (UNIX_EPOCH + Duration::from_secs(ts), df)))
    }
}

/// Determine the TLS mode from the environment, or `None` for plain HTTP.
fn read_tls_mode() -> Option<TlsMode> {
    if env_truthy("BELOW_REMOTE_TLS_INSECURE") {
        return Some(TlsMode::Insecure);
    }
    if let Some(ca) = std::env::var_os("BELOW_REMOTE_CA_FILE") {
        return Some(TlsMode::Ca(PathBuf::from(ca)));
    }
    if env_truthy("BELOW_REMOTE_TLS") {
        return Some(TlsMode::Default);
    }
    None
}

fn env_truthy(key: &str) -> bool {
    match std::env::var(key) {
        Ok(v) => matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => false,
    }
}

/// Build a TLS client config that verifies the server against a CA bundle.
fn build_ca_config(ca_path: &Path) -> Result<ClientConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut roots = RootCertStore::empty();
    let mut reader = BufReader::new(
        File::open(ca_path).with_context(|| format!("Failed to open CA file {ca_path:?}"))?,
    );
    for cert in rustls_pemfile::certs(&mut reader) {
        let cert = cert.with_context(|| format!("Failed to parse CA file {ca_path:?}"))?;
        roots
            .add(cert)
            .context("Failed to add CA certificate to root store")?;
    }
    if roots.is_empty() {
        bail!("No certificates found in CA file {ca_path:?}");
    }
    Ok(ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .context("Failed to initialize TLS protocol versions")?
        .with_root_certificates(roots)
        .with_no_client_auth())
}

/// Build a TLS client config that skips server certificate verification.
fn build_insecure_config() -> ClientConfig {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .expect("ring provider supports default protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(InsecureVerifier { provider }))
        .with_no_client_auth()
}

/// Read the bearer token from `BELOW_REMOTE_TOKEN` or `BELOW_REMOTE_TOKEN_FILE`.
fn read_token() -> Result<Option<String>> {
    if let Ok(token) = std::env::var("BELOW_REMOTE_TOKEN") {
        if !token.is_empty() {
            return Ok(Some(token));
        }
    }
    if let Ok(path) = std::env::var("BELOW_REMOTE_TOKEN_FILE") {
        let token = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read BELOW_REMOTE_TOKEN_FILE {path:?}"))?;
        return Ok(Some(token.trim().to_owned()));
    }
    Ok(None)
}

/// Verifier that accepts any server certificate. Insecure; testing only.
#[derive(Debug)]
struct InsecureVerifier {
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for InsecureVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}
