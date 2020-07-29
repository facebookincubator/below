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

// Dprint trait defines the print fns for different format. Please refer to
// the comment of Dump trait in tmain.rs for the whole picture.
pub trait Dprint
where
    Self: DumpType + Dget,
{
    fn print_title_line<T: Write>(
        &self,
        model: &Self::Model,
        out: &mut T,
        sep: &str,
    ) -> Result<()> {
        let mut line = String::new();
        self.get_title_fns().iter().for_each(|item| {
            line.push_str(&format!("{}{}", item(self.get_data(), &model), sep));
        });

        write!(out, "{}\n", line)?;
        Ok(())
    }

    fn print_field_line<T: Write>(
        &self,
        model: &Self::Model,
        out: &mut T,
        sep: &str,
    ) -> Result<()> {
        let mut line = String::new();
        self.get_field_fns().iter().for_each(|item| {
            line.push_str(&format!("{}{}", item(self.get_data(), &model), sep));
        });

        write!(out, "{}\n", line)?;
        Ok(())
    }

    fn do_print_kv<T: Write>(&self, model: &Self::Model, out: &mut T) -> Result<()> {
        let mut paragraph = String::new();
        self.get_title_fns()
            .iter()
            .zip(self.get_field_fns().iter())
            .for_each(|(title, field)| {
                paragraph.push_str(&format!(
                    "{}: {}\n",
                    title(self.get_data(), &model),
                    field(self.get_data(), &model),
                ));
            });

        write!(out, "{}\n", paragraph)?;
        Ok(())
    }

    fn do_print_json(&self, model: &Self::Model) -> Value {
        let mut res = json!({});
        self.get_title_fns()
            .iter()
            .zip(self.get_field_fns().iter())
            .for_each(|(title, field)| {
                res[title(self.get_data(), &model)] = json!(field(self.get_data(), &model))
            });
        res
    }

    fn do_print_raw<T: Write>(
        &self,
        model: &Self::Model,
        output: &mut T,
        round: usize,
    ) -> Result<()> {
        let repeat = self.get_opts().repeat_title.unwrap_or(0);
        let disable_title = self.get_opts().disable_title;
        if !disable_title && (round == 0 || (repeat != 0 && round % repeat == 0)) {
            self.print_title_line(&model, output, " ")?;
        }
        self.print_field_line(&model, output, " ")
    }

    fn do_print_csv<T: Write>(
        &self,
        model: &Self::Model,
        output: &mut T,
        round: usize,
    ) -> Result<()> {
        let disable_title = self.get_opts().disable_title;
        if !disable_title && round == 0 {
            self.print_title_line(&model, output, ",")?;
        }
        self.print_field_line(&model, output, ",")
    }
}
