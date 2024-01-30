use super::*;
use model::SingleTcModel;

pub struct Tc {
    opts: GeneralOpt,
    fields: Vec<TcField>,
}

impl Tc {
    pub fn new(opts: &GeneralOpt, fields: Vec<TcField>) -> Self {
        Self {
            opts: opts.to_owned(),
            fields,
        }
    }
}

impl Dumper for Tc {
    fn dump_model(
        &self,
        ctx: &CommonFieldContext,
        model: &model::Model,
        output: &mut dyn Write,
        round: &mut usize,
        comma_flag: bool,
    ) -> Result<IterExecResult> {
        let tcs: Vec<&SingleTcModel> = match &model.tc {
            Some(tc_model) => tc_model.tc.iter().collect(),
            None => Vec::new(),
        };
        if tcs.is_empty() {
            return Ok(IterExecResult::Skip);
        }

        let mut json_output = json!([]);

        tcs.into_iter()
            .map(|tc| {
                match self.opts.output_format {
                    Some(OutputFormat::Raw) | None => write!(
                        output,
                        "{}",
                        print::dump_raw(
                            &self.fields,
                            ctx,
                            tc,
                            *round,
                            self.opts.repeat_title,
                            self.opts.disable_title,
                            self.opts.raw
                        )
                    )?,
                    Some(OutputFormat::Csv) => write!(
                        output,
                        "{}",
                        print::dump_csv(
                            &self.fields,
                            ctx,
                            tc,
                            *round,
                            self.opts.disable_title,
                            self.opts.raw
                        )
                    )?,
                    Some(OutputFormat::Tsv) => write!(
                        output,
                        "{}",
                        print::dump_tsv(
                            &self.fields,
                            ctx,
                            tc,
                            *round,
                            self.opts.disable_title,
                            self.opts.raw
                        )
                    )?,
                    Some(OutputFormat::KeyVal) => write!(
                        output,
                        "{}",
                        print::dump_kv(&self.fields, ctx, tc, self.opts.raw)
                    )?,
                    Some(OutputFormat::Json) => {
                        let par = print::dump_json(&self.fields, ctx, tc, self.opts.raw);
                        json_output.as_array_mut().unwrap().push(par);
                    }
                    Some(OutputFormat::OpenMetrics) => {
                        write!(output, "{}", print::dump_openmetrics(&self.fields, ctx, tc))?
                    }
                }
                *round += 1;
                Ok(())
            })
            .collect::<Result<Vec<_>>>()?;

        match (self.opts.output_format, comma_flag) {
            (Some(OutputFormat::Json), true) => write!(output, ",{}", json_output)?,
            (Some(OutputFormat::Json), false) => write!(output, "{}", json_output)?,
            (Some(OutputFormat::OpenMetrics), _) => (),
            _ => writeln!(output)?,
        };

        Ok(IterExecResult::Success)
    }
}
