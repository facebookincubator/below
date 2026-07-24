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

use std::env;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::Context;
use anyhow::Error;
use anyhow::anyhow;
use async_trait::async_trait;
use below_thrift_service::SERVICE_PORT;
use below_thrift_service_services::make_BelowService_server;
use cli_usage::UsageMetadata;
pub use exitstat::ExitstatSkelBuilder;
use fb303::fb_status;
use fb303_core_services::BaseService;
use fb303_core_services::errors::GetNameExn;
use fb303_core_services::errors::GetStatusDetailsExn;
use fb303_core_services::errors::GetStatusExn;
use fb303_core_services::make_BaseService_server;
use fb303_services::FacebookService;
use fb303_services::make_FacebookService_server;
pub use fbinit::FacebookInit;
use srserver::ThriftServer;
use srserver::ThriftServerBuilder;
use srserver_service_framework_light::AclCheckerModule;
use srserver_service_framework_light::BuildModule;
use srserver_service_framework_light::Fb303Module;
use srserver_service_framework_light::ProfileModule;
use srserver_service_framework_light::ServiceFramework;
use srserver_service_framework_light::ThriftStatsModule;
use tokio::runtime::Builder as TB;
use tokio::runtime::Runtime;
use tokio_util::sync::CancellationToken;

pub mod commands;
pub mod gpu_stats;
pub mod init;
pub mod logging;
mod remote_server;
pub mod statistics;

#[cfg(test)]
mod test;

#[derive(Clone)]
pub struct FacebookServiceImpl;

impl FacebookService for FacebookServiceImpl {}

#[async_trait]
impl BaseService for FacebookServiceImpl {
    async fn getName(&self) -> Result<String, GetNameExn> {
        Ok("Below Service".to_string())
    }

    async fn getStatusDetails(&self) -> Result<String, GetStatusDetailsExn> {
        Ok("Alive and running.".to_string())
    }

    async fn getStatus(&self) -> Result<fb_status, GetStatusExn> {
        Ok(fb_status::ALIVE)
    }
}

fn build_remote_viewing_service(
    fb: FacebookInit,
    logger: slog::Logger,
    runtime: &Runtime,
    store_dir: PathBuf,
    port: Option<u16>,
) -> anyhow::Result<ServiceFramework> {
    let logger_clone = logger.clone();
    let fb303_base = |proto| make_BaseService_server(proto, FacebookServiceImpl);
    let fb303 = move |proto| make_FacebookService_server(proto, FacebookServiceImpl, fb303_base);
    let below_svc = move |proto| {
        let service_impl = remote_server::BelowServiceImpl::new(
            // Need to clone the logger again here to closure implements
            // implements `Fn` trait
            logger_clone.clone(),
            store_dir.clone(),
        );

        make_BelowService_server(proto, service_impl, fb303)
    };

    // Reserved in port_registry.cconf
    let thrift_port = port.unwrap_or(SERVICE_PORT as u16);

    use BelowService_metadata_sys::create_metadata;
    let thrift: ThriftServer = ThriftServerBuilder::new(fb)
        .with_name("rv_thrift")
        .expect("with_name should not fail for a valid name")
        .with_num_io_worker_threads(1) // We don't do async IO
        .with_tokio_runtime_as_thread_manager(runtime)
        .with_port(thrift_port)
        .with_metadata(create_metadata())
        .with_factory(runtime.handle().clone(), move || below_svc)
        .build();

    // The ServiceFramework wrapper "consumes" the Thrift server
    let mut svc_framework = ServiceFramework::from_server("below_remote_viewing_service", thrift)
        .context("Failed to create service framework server")?;

    match env::var("BELOW_REMOTE_SERVICE_IDENTITY") {
        Ok(identity) => {
            slog::info!(logger, "Found service identity {}", identity,);
            let mut acl_checker_module = AclCheckerModule::log_only(&identity);
            acl_checker_module.enforce_acl_checks();
            match env::var("BELOW_REMOTE_CHECK_CRYPTO_AUTH_TOKENS") {
                Ok(_) => {
                    slog::info!(logger, "Checking crypto auth tokens",);
                    acl_checker_module.check_crypto_auth_tokens();
                }
                Err(_) => {
                    slog::info!(logger, "Not checking crypto auth tokens",);
                }
            }
            svc_framework.add_module(acl_checker_module)?;
        }
        Err(_) => slog::info!(logger, "Found no service identity"),
    };

    svc_framework.add_module(BuildModule)?;
    svc_framework.add_module(ThriftStatsModule)?;
    svc_framework.add_module(Fb303Module)?;
    svc_framework.add_module(ProfileModule)?;
    Ok(svc_framework)
}

/// Ensure background tasks are stopped when the main thread stops
pub struct ExitGuard {
    cancel: CancellationToken,
    task: Option<tokio::task::JoinHandle<()>>,
    rt: Runtime,
}

impl Drop for ExitGuard {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(task) = self.task.take() {
            self.rt
                .handle()
                .block_on(async move { task.await.expect("Failed to join task") });
        }
    }
}

pub fn init(
    init: init::InitToken,
    logger: slog::Logger,
    service: crate::Service,
    store_dir: PathBuf,
    errs: Sender<Error>,
) -> Option<ExitGuard> {
    // Log cli usage
    //
    // 50ms should still be imperceptible even if full timeout is hit
    let _ = UsageMetadata {
        tool: "below",
        timeout: Duration::from_millis(50),
        ..Default::default()
    }
    .log();

    if let crate::Service::On(port) = service {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        let runtime = TB::new_multi_thread()
            .worker_threads(4)
            .thread_name("background")
            .enable_all()
            .build()
            .expect("Failed to construct tokio runtime.");
        let handle = runtime.handle();
        let mut svc_framework =
            build_remote_viewing_service(init.fb, logger.clone(), &runtime, store_dir, port)
                .expect("Failed to build remote viewing service");
        let task = handle.spawn(async move {
            if let Err(e) = svc_framework
                .serve_background()
                .context("Failed to start service framework")
            {
                errs.send(e).expect("failed to send error to main thread");
                return;
            }
            tokio::select! {
                _ = cancel_clone.cancelled() => {},
                _ = stats::schedule_stats_aggregation_preview()
                    .expect("failed to schedule stats aggregation") => {
                        errs.send(anyhow!("stats aggregation task stopped unexpectedly")).expect("failed to send error to main thread");
                    }
            }
            svc_framework.stop();
            slog::debug!(logger, "Background tasks stopped");
        });
        Some(ExitGuard {
            cancel,
            task: Some(task),
            rt: runtime,
        })
    } else {
        None
    }
}
