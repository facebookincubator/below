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

// A convenience macro of generating the implementation of Dget.
// The first argument is the struct name that implementing the Dget.
// The following arguments are a recurrent pattern that defines which
// tag we want to show if user specify --all.
#[macro_export]
macro_rules! make_dget {
    ($name: ident, $($item: expr,)*) => {
        impl Dget for $name {
            fn get_title_fns(
                & self,
            ) -> & Vec<Box<dyn Fn(&Self::DataType, &Self::Model) -> String>> {
                &self.title_fns
            }

            fn get_field_fns(
                & self,
            ) -> & Vec<Box<dyn Fn(&Self::DataType, &Self::Model) -> String>> {
                &self.field_fns
            }

            fn get_data(& self) -> & Self::DataType {
                &self.data
            }

            fn get_time_end(& self) -> & SystemTime {
                &self.time_end
            }

            fn get_advance(& self) -> & Advance {
                &self.advance
            }

            fn get_advance_mut(& mut self) -> & mut Advance {
                &mut self.advance
            }

            fn get_opts(& self) -> & command::GeneralOpt {
                &self.opts
            }

            fn get_opts_mut(&mut self) -> &mut command::GeneralOpt {
                &mut self.opts
            }

            fn get_all_classes() -> Vec<Self::FieldsType> {
                vec![$($item,)*]
            }
        }
    };
}

// All the get functions. Please refer to tmain.rs for a bigger picture.
pub trait Dget
where
    Self: DumpType,
{
    fn get_title_fns(&self) -> &Vec<Box<dyn Fn(&Self::DataType, &Self::Model) -> String>>;
    fn get_field_fns(&self) -> &Vec<Box<dyn Fn(&Self::DataType, &Self::Model) -> String>>;
    fn get_data(&self) -> &Self::DataType;
    fn get_time_end(&self) -> &SystemTime;
    fn get_advance(&self) -> &Advance;
    fn get_advance_mut(&mut self) -> &mut Advance;
    fn get_opts(&self) -> &command::GeneralOpt;
    fn get_opts_mut(&mut self) -> &mut command::GeneralOpt;
    fn get_all_classes() -> Vec<Self::FieldsType>;
}
