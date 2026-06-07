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

//! Open-source remote-viewing server (the counterpart to `store::RemoteStore`).
//!
//! A small blocking HTTP server: GET `/get_frame?timestamp=<unix>&direction=<forward|reverse>`
//! returns a CBOR-encoded `Option<(u64, DataFrame)>` read from the local store.
//!
//! Authentication is a shared bearer token sourced from a Kubernetes Secret via
//! the `BELOW_REMOTE_TOKEN` env var (or the file named by `BELOW_REMOTE_TOKEN_FILE`).
//! When a token is configured, requests must present a matching
//! `Authorization: Bearer <token>` header. TLS is intentionally delegated to the
//! platform (service mesh / ingress).

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context;
use anyhow::Result;
use slog::error;
use slog::info;
use slog::warn;
use store::DataFrame;
use store::Direction;
use store::read_next_sample;
use tiny_http::Response;
use tiny_http::Server;

/// Default port. Keep in sync with `DEFAULT_REMOTE_PORT` in the remote client
/// (`store/src/open_source/remote_store.rs`).
const DEFAULT_REMOTE_PORT: u16 = 1969;
/// Number of worker threads serving requests.
const NUM_WORKERS: usize = 4;

struct ServerState {
    logger: slog::Logger,
    store_dir: PathBuf,
    /// Required bearer token. `None` means unauthenticated access is allowed.
    token: Option<String>,
}

/// Start the remote-viewing server. Binding happens synchronously so a failure
/// (e.g. address in use) is reported to the caller; request handling runs on
/// detached worker threads for the lifetime of the process.
pub fn start(logger: slog::Logger, store_dir: PathBuf, port: Option<u16>) -> Result<()> {
    let port = port.unwrap_or(DEFAULT_REMOTE_PORT);
    let addr = format!("0.0.0.0:{port}");
    let server = Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("Failed to bind remote viewing server on {addr}: {e}"))?;

    let token = read_token()?;
    if token.is_some() {
        info!(logger, "Remote viewing server listening on {addr} (token auth)");
    } else {
        warn!(
            logger,
            "Remote viewing server listening on {addr} WITHOUT authentication. Set \
             BELOW_REMOTE_TOKEN (or BELOW_REMOTE_TOKEN_FILE) to require a token."
        );
    }

    let state = Arc::new(ServerState {
        logger,
        store_dir,
        token,
    });
    let server = Arc::new(server);

    for _ in 0..NUM_WORKERS {
        let server = server.clone();
        let state = state.clone();
        thread::Builder::new()
            .name("below_remote_server".to_owned())
            .spawn(move || {
                for request in server.incoming_requests() {
                    handle(&state, request);
                }
            })
            .context("Failed to spawn remote server worker")?;
    }

    Ok(())
}

fn handle(state: &ServerState, request: tiny_http::Request) {
    let response = match build_response(state, &request) {
        Ok(bytes) => Response::from_data(bytes),
        Err((code, msg)) => {
            if code != 404 {
                warn!(state.logger, "Remote request rejected ({code}): {msg}");
            }
            Response::from_string(msg).with_status_code(code)
        }
    };
    if let Err(e) = request.respond(response) {
        error!(state.logger, "Failed to send remote response: {e}");
    }
}

/// Returns the CBOR response body, or an `(http_status, message)` error.
fn build_response(
    state: &ServerState,
    request: &tiny_http::Request,
) -> std::result::Result<Vec<u8>, (u16, String)> {
    // Authenticate first so unauthorized callers learn nothing about the URL.
    if let Some(expected) = &state.token {
        let presented = request
            .headers()
            .iter()
            .find(|h| h.field.equiv("Authorization"))
            .map(|h| h.value.as_str())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        if !constant_time_eq(presented.as_bytes(), expected.as_bytes()) {
            return Err((401, "unauthorized".to_owned()));
        }
    }

    let url = request.url();
    let query = url
        .strip_prefix("/get_frame?")
        .ok_or((404, "not found".to_owned()))?;

    let mut timestamp: Option<u64> = None;
    let mut direction: Option<Direction> = None;
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("timestamp", v)) => {
                timestamp = Some(v.parse().map_err(|_| (400, "bad timestamp".to_owned()))?);
            }
            Some(("direction", "forward")) => direction = Some(Direction::Forward),
            Some(("direction", "reverse")) => direction = Some(Direction::Reverse),
            Some(("direction", _)) => return Err((400, "bad direction".to_owned())),
            _ => {}
        }
    }
    let timestamp = timestamp.ok_or((400, "missing timestamp".to_owned()))?;
    let direction = direction.ok_or((400, "missing direction".to_owned()))?;

    let target = UNIX_EPOCH + Duration::from_secs(timestamp);
    let found = read_next_sample(&state.store_dir, target, direction, state.logger.clone())
        .map_err(|e| (500, format!("store read failed: {e:#}")))?;

    let frame: Option<(u64, DataFrame)> = found.map(|(st, df)| {
        let ts = st
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        (ts, df)
    });

    serde_cbor::to_vec(&frame).map_err(|e| (500, format!("serialize failed: {e}")))
}

/// Constant-time byte-slice comparison to avoid leaking the token via timing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
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
