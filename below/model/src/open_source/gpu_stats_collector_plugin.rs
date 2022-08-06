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

use anyhow::Result;
use async_trait::async_trait;

use crate::collector_plugin::AsyncCollectorPlugin;

pub type SampleType = gpu_stats::GpuSample;

pub struct GpuStatsCollectorPlugin {}

impl GpuStatsCollectorPlugin {
    pub fn new(_logger: slog::Logger) -> Result<Self> {
        Ok(Self {})
    }
}

// Wrapper plugin for GpuStatsCollector
#[async_trait]
impl AsyncCollectorPlugin for GpuStatsCollectorPlugin {
    type T = SampleType;

    async fn try_collect(&mut self) -> Result<Option<SampleType>> {
        Ok(None)
    }
}
