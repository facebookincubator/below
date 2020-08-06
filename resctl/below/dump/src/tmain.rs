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

// The dump trait defines the general functionality and interface for our dump module.
// We are splitting the dump trait to 5 sub-traits base on the functionality.
//
// DumpType:
// We define all of the generic types to the DumpType trait which will be required by all
// sub-traits. It defines the following types:
// Model ==> The real model typle, like CgroupModel or SingleProcessModel.
// FieldsType ==> The enum tag type we defined in command.rs, like Sys.
// DataType ==> Our struct that implement the BelowDecor per dump module.
//
// Dget:
// Dget trait defines a bunch of necessary "get" functions for our trait, since
// we are not able to access the data directly from the trait. Besides that, Dget
// will also provide minimal constraints on the implementation struct to make
// dump work. Everyfield in Dget is required. For convenience, we implment a macro
// to automatically generate the implementation of this trait. Please refer to get.rs
// for more details.
//
// Dprint:
// Dprint trait defines all the necessaray print functions in raw, csv, json, and kv.
// During the implmentation, developer's won't need to worry about how we format the
// the data, it is automatically handled by Dprint.
//
// Dfill:
// Dfill will handle the relationship between tags defines in the command.rs and processing
// implementation like compare, filtering. Besides that, Dfill will also be responsible for
// generating print functions base on user input. Please refer to the comment of generate_option
// in command.rs for concrete use case example. Dfill trait will be automatically generated
// by BelowDecor if a tag or class attribute is encountered.
//
// Dump:
// Dump trait is the central of everything. It is our main event loop.

#[derive(PartialEq)]
pub enum IterExecResult {
    Success,
    Skip,
}

pub trait Dump
where
    Self: DumpType + Dget + Dprint + Dfill,
{
    // Constructor of the implementation struct.
    // Put the constructor in a general format will help us write generic execution funciton.
    fn new(
        opts: command::GeneralOpt,
        advance: Advance,
        time: SystemTime,
        select: Option<Self::FieldsType>,
    ) -> Self;

    // Update the timestamp and datetime in DataType.
    // We suppose to implement this function in the trait, but assigning a value from the trait
    // may not be a good practice.
    fn advance_timestamp(&mut self, model: &model::Model) -> Result<()>;

    // Function that tell us how to iterate model. Will also handle print and json format here.
    fn iterate_exec<T: Write>(
        &self,
        model: &model::Model,
        output: &mut T,
        count: &mut usize,
        //  comma_flag is for JSON output, if it set to true, we will have a "," before the output JSON.
        // This is because in case of filter and delete empty val, we have no idea if the current
        // value is the LAST value.
        comma_flag: bool,
    ) -> Result<IterExecResult>;

    // Init function is responsible for calling the dfill build_* functions to build the fn vec.
    fn init(&mut self, fields: Option<Vec<Self::FieldsType>>) {
        // If all set to true or no fields are provided, we should display all
        // l0 fields. In another word, all will override opt and fields.
        let opts = self.get_opts_mut();
        if opts.everything {
            opts.default = true;
            opts.detail = true;
        }

        let mut fields = if opts.default || fields.is_none() {
            Some(Self::get_all_classes())
        } else {
            fields
        };

        // Dedup if JSON, it will break the order here. But since the output
        // format is JSON, order may not be that important.
        if let Some(OutputFormat::Json) = opts.output_format {
            let hs = fields
                .unwrap()
                .into_iter()
                .collect::<HashSet<Self::FieldsType>>();

            fields = Some(hs.into_iter().collect());
        }

        match opts.output_format {
            Some(OutputFormat::Raw) | None => {
                self.build_title_fns_styled(&fields);
                self.build_field_fns_styled(&fields);
            }
            _ => {
                self.build_title_fns(&fields);
                self.build_field_fns(&fields);
            }
        }
    }

    // The main event loop. This function will call iterate_exec on each timeslice.
    fn loop_through<T: Write>(&mut self, output: &mut T) -> Result<()> {
        let mut model = match self.get_advance_mut().advance(Direction::Forward) {
            Some(m) => m,
            None => bail!("No initial model could be found!"),
        };

        let json = self.get_opts().output_format == Some(OutputFormat::Json);
        let csv = self.get_opts().output_format == Some(OutputFormat::Csv);

        let mut round = 0;

        if json {
            write!(output, "[")?;
        }

        loop {
            self.advance_timestamp(&model)?;

            // Base on the exec result, we will determine if we need to generate the line breaker, etc
            let comma_flag = round != 0;
            let exec_res = match self.iterate_exec(&model, output, &mut round, comma_flag) {
                Ok(res) => res,
                // Swallow BrokenPipe error for write. Rust runtime will ignore SIGPIPE by default and
                // propagating EPIPE upwards to the application in the form of an IoError::BrokenPipe.
                Err(e)
                    if e.downcast_ref::<std::io::Error>()
                        .map_or_else(|| false, |e| e.kind() == std::io::ErrorKind::BrokenPipe) =>
                {
                    return Ok(())
                }
                Err(e) => return Err(e),
            };

            if self.get_advance().get_next_ts() > *self.get_time_end() {
                break;
            }

            model = match self.get_advance_mut().advance(Direction::Forward) {
                Some(m) => m,
                None => break,
            };

            if exec_res == IterExecResult::Skip {
                continue;
            }

            if !csv {
                write!(output, "\n")?;
            }
        }

        if json {
            write!(output, "]")?;
        }

        Ok(())
    }

    // The loop_through wrapper, basically to provide the output destination reference.
    fn exec(&mut self) -> Result<()> {
        match self.get_opts().output.as_ref() {
            Some(file_path) => {
                let mut file = File::create(file_path)?;
                self.loop_through(&mut file)
            }
            None => {
                let mut stdout = io::stdout();
                self.loop_through(&mut stdout)
            }
        }
    }
}
