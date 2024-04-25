use anyhow::Result;
use async_trait::async_trait;
use slog::error;
use tc::TcStats;

use crate::collector_plugin::AsyncCollectorPlugin;

pub type SampleType = TcStats;

pub struct TcStatsCollectorPlugin {
    logger: slog::Logger,
}

impl TcStatsCollectorPlugin {
    pub fn new(logger: slog::Logger) -> Result<Self> {
        Ok(Self { logger })
    }
}

#[async_trait]
impl AsyncCollectorPlugin for TcStatsCollectorPlugin {
    type T = TcStats;

    async fn try_collect(&mut self) -> Result<Option<SampleType>> {
        let stats = match tc::tc_stats() {
            Ok(tc_stats) => Some(tc_stats),
            Err(e) => {
                error!(self.logger, "{:#}", e);
                Default::default()
            }
        };

        Ok(stats)
    }
}
