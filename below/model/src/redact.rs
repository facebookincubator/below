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

//! Shortens data that may be sensitive before a sample leaves the host.

use std::path::Path;

use crate::Sample;

/// Bumped whenever [`redact_sample`] changes what it removes, so a consumer
/// can tell which rules a redacted sample went through.
pub const REDACTION_VERSION: u32 = 1;

/// Cuts each process's command line to argv[0], since the arguments may be
/// sensitive. Python processes keep `<comm> ::<script>` instead, so they can
/// still be told apart.
pub fn redact_sample(sample: &mut Sample) {
    for pid_info in sample.processes.values_mut() {
        let Some(cmdline) = pid_info.cmdline_vec.as_mut() else {
            continue;
        };
        match pid_info.stat.comm.as_deref() {
            Some(comm) if comm.starts_with("python") => {
                *cmdline = vec![parse_python_cmdline(cmdline, comm)];
            }
            _ => cmdline.truncate(1),
        }
    }
}

/// Take the first none option argument as script name.
fn parse_python_cmdline(cmdline: &[String], comm: &str) -> String {
    let mut cmd_iter = cmdline.iter();
    if let Some(v) = cmd_iter.next().filter(|&v| v.starts_with("[xarexec]")) {
        return format!(
            "{} ::[xarexec] {}",
            comm,
            Path::new(v).file_name().map_or_else(
                || v.to_string(),
                |script_name| script_name.to_string_lossy().to_string()
            )
        );
    }

    for opt in cmd_iter {
        // skip the cmd as string opt
        if opt == "-" || opt == "-c" {
            return comm.into();
        } else if opt.starts_with('-') {
            continue;
        } else {
            return format!(
                "{} ::{}",
                comm,
                Path::new(opt).file_name().map_or_else(
                    || opt.to_string(),
                    |script_name| script_name.to_string_lossy().to_string()
                )
            );
        }
    }

    comm.into()
}

#[cfg(test)]
mod tests {
    use procfs::PidInfo;

    use super::*;

    fn redacted_cmdline(comm: &str, cmdline: Option<&[&str]>) -> Option<Vec<String>> {
        let mut pid_info = PidInfo::default();
        pid_info.stat.comm = Some(comm.to_owned());
        pid_info.cmdline_vec = cmdline.map(|args| args.iter().map(|arg| arg.to_string()).collect());
        pid_info.exe_path = Some("/usr/bin/tool".to_owned());
        let mut sample = Sample::default();
        sample.processes.insert(1, pid_info.clone());

        redact_sample(&mut sample);
        let redacted = sample.processes.remove(&1).expect("pid 1 was inserted");
        assert_eq!(
            PidInfo {
                cmdline_vec: pid_info.cmdline_vec.clone(),
                ..redacted.clone()
            },
            pid_info,
            "only the command line should change"
        );
        redacted.cmdline_vec
    }

    #[test]
    fn cmdline_keeps_argv0() {
        assert_eq!(
            redacted_cmdline("tool", Some(&["/usr/bin/tool", "--password=hunter2"])),
            Some(vec!["/usr/bin/tool".to_owned()])
        );
        assert_eq!(redacted_cmdline("kworker/0:1", Some(&[])), Some(vec![]));
        assert_eq!(redacted_cmdline("tool", None), None);
    }

    #[test]
    fn python_keeps_script_name() {
        assert_eq!(
            redacted_cmdline(
                "python3",
                Some(&[
                    "/usr/bin/python3",
                    "-u",
                    "/home/alice/train.py",
                    "--user",
                    "123"
                ])
            ),
            Some(vec!["python3 ::train.py".to_owned()])
        );
        assert_eq!(
            redacted_cmdline("python3", Some(&["python3", "-c", "print('secret')"])),
            Some(vec!["python3".to_owned()])
        );
        assert_eq!(
            redacted_cmdline(
                "python3.10",
                Some(&["[xarexec] /mnt/xarfuse/uid-0/abc/tool.xar", "--flag"])
            ),
            Some(vec!["python3.10 ::[xarexec] tool.xar".to_owned()])
        );
    }
}
