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

use super::*;

#[derive(PartialEq)]
pub enum IterExecResult {
    Success,
    Skip,
}

/// Dumps (a portion of) the Model to some output in specific format.
pub trait Dumper {
    fn dump_model(
        &self,
        ctx: &CommonFieldContext,
        model: &model::Model,
        output: &mut dyn Write,
        round: &mut usize,
        //  comma_flag is for JSON output, if it set to true, we will have a "," before the output JSON.
        // This is because in case of filter and delete empty val, we have no idea if the current
        // value is the LAST value.
        comma_flag: bool,
    ) -> Result<IterExecResult>;
}

/// Called by dump commands to dump Models in continuous time steps. The actual
/// dump logic for different Models in each time step is handled by specific
/// Dumper implementations. This function is responsible for retrieving Models
/// and handling formatting between time steps.
pub fn dump_timeseries(
    mut advance: Advance,
    time_begin: SystemTime,
    time_end: SystemTime,
    dumper: &dyn Dumper,
    output: &mut dyn Write,
    output_format: Option<OutputFormat>,
    br: Option<String>,
    errs: Receiver<Error>,
) -> Result<()> {
    let mut model = match advance.jump_sample_to(time_begin) {
        Some(m) => m,
        None => bail!(
            "No initial sample could be found!\n\
            You may have provided a time in the future or no data was recorded during the provided time. \
            Please check your input and timezone.\n\
            If you are using remote, please make sure the below service on target host is running."
        ),
    };

    cliutil::check_initial_sample_time_in_time_range(model.timestamp, time_begin, time_end)?;

    let json = output_format == Some(OutputFormat::Json);
    let csv = output_format == Some(OutputFormat::Csv);
    let openmetrics = output_format == Some(OutputFormat::OpenMetrics);

    let mut round = 0;

    if json {
        write!(output, "[")?;
    }

    loop {
        // Received external error, e.g. stop signal
        if let Ok(e) = errs.try_recv() {
            bail!(e);
        }
        let ctx = CommonFieldContext {
            timestamp: model
                .timestamp
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs() as i64,
            hostname: model.system.hostname.clone(),
        };
        // Base on the exec result, we will determine if we need to generate the line breaker, etc
        let comma_flag = round != 0;
        let res = match dumper.dump_model(&ctx, &model, output, &mut round, comma_flag) {
            Ok(res) => res,
            Err(e) => {
                // Swallow BrokenPipe error for write. Rust runtime will ignore SIGPIPE by default and
                // propagating EPIPE upwards to the application in the form of an IoError::BrokenPipe.
                if e.downcast_ref::<std::io::Error>()
                    .map_or(false, |e| e.kind() == std::io::ErrorKind::BrokenPipe)
                {
                    return Ok(());
                } else {
                    return Err(e);
                }
            }
        };

        if advance.get_next_ts() > time_end {
            break;
        }

        model = match advance.advance(Direction::Forward) {
            Some(m) => m,
            None => break,
        };

        if res == IterExecResult::Skip {
            continue;
        }

        if json {
            writeln!(output)?;
        } else if br.is_some() && !csv {
            writeln!(output, "{}", br.as_ref().unwrap())?;
        }
    }

    if json {
        write!(output, "]")?;
    } else if openmetrics {
        writeln!(output, "# EOF")?;
    }

    cliutil::check_final_sample_time_with_requested_time(model.timestamp, time_end);

    Ok(())
}
