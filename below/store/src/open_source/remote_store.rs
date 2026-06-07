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
//! When neither is set the client sends no credentials (for use inside a
//! trusted network or behind a TLS-terminating proxy/service mesh).
//!
//! Transport security (TLS) is intentionally delegated to the platform (service
//! mesh or ingress), matching how this is typically deployed in Kubernetes.

use std::io::Read;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;

use crate::DataFrame;
use crate::Direction;

/// Default port used when none is supplied.
pub const DEFAULT_REMOTE_PORT: u16 = 1969;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct RemoteStore {
    base_url: String,
    token: Option<String>,
    agent: ureq::Agent,
}

impl RemoteStore {
    pub fn new(host: String, port: Option<u16>) -> Result<RemoteStore> {
        let port = port.unwrap_or(DEFAULT_REMOTE_PORT);
        let agent = ureq::AgentBuilder::new()
            .timeout(REQUEST_TIMEOUT)
            .build();
        Ok(RemoteStore {
            base_url: format!("http://{host}:{port}"),
            token: read_token()?,
            agent,
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
