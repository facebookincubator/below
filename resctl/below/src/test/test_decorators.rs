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

fn decor_function(item: &f64) -> String {
    format!("{} MB", item)
}

fn highlight_if_function(item: &f64) -> Option<cursive::theme::BaseColor> {
    if *item > 10.0 {
        Some(BaseColor::Green)
    } else if *item > 0.0 {
        Some(BaseColor::Red)
    } else {
        None
    }
}

#[derive(BelowDecor)]
struct SubField {
    #[bttr(title = "Field A", width = 7)]
    field_a: Option<f64>,
    #[bttr(title = "Field B", width = 8)]
    field_b: Option<f64>,
}

impl SubField {
    fn new() -> Self {
        Self {
            field_a: Some(1.1),
            field_b: Some(2.2),
        }
    }
}

#[allow(unused)]
#[derive(BelowDecor)]
struct TestModel {
    #[bttr(
        title = "Usage",
        unit = "%",
        width = 7,
        cmp = true,
        title_width = 7,
        highlight_if = "highlight_if_function(&$)"
    )]
    usage_pct: Option<f64>,
    #[bttr(title = "User", unit = "%", width = 7, cmp = true)]
    #[blink("SubField$get_field_a")]
    user_pct: Option<f64>,
    #[bttr(
        title = "System",
        unit = "%",
        none_mark = "0.0",
        width = 7,
        precision = 1,
        highlight_if = "if $ > 0.0 {Some(BaseColor::Red)} else {None}"
    )]
    system_pct: Option<f64>,
    #[bttr(
        title = "L1 Cache",
        decorator = "decor_function(&$)",
        prefix = "\"-->\"",
        depth = "5",
        width = 7
    )]
    cache_usage: Option<f64>,
    #[blink("SubField$get_field_a")]
    loopback: Option<f64>,
    #[blink("SubField$get_field_b")]
    route: Option<f64>,
    something_else: Option<f64>,
    #[bttr(title = "Aggr", cmp = true, width = 5, precision = 2)]
    #[blink("SubField$get_field_a")]
    #[blink("SubField$get_field_b")]
    pub aggr: Option<f64>,
    pub no_show: Option<f64>,
}

impl TestModel {
    fn new() -> Self {
        Self {
            usage_pct: Some(12.6),
            user_pct: None,
            system_pct: Some(2.222),
            cache_usage: Some(100.0),
            something_else: Some(0.0),
            loopback: None,
            route: None,
            aggr: None,
            no_show: None,
        }
    }
}

#[test]
fn test_bdecor_field_function() {
    let mut model = TestModel::new();
    let subfield = SubField::new();
    assert_eq!(model.get_usage_pct_value().unwrap(), 12.6);
    assert_eq!(model.get_system_pct_value().unwrap(), 2.222);
    assert_eq!(model.get_cache_usage_value().unwrap(), 100.0);
    assert_eq!(model.get_usage_pct_str_styled().source(), "12.6%  ");
    assert_eq!(model.get_system_pct_str_styled().source(), "2.2%   ");
    assert_eq!(model.get_usage_pct_str(), "12.6%");
    assert_eq!(model.get_system_pct_str(), "2.2%");
    assert_eq!(model.get_aggr_str_styled(&subfield).source(), "3.30 ");
    assert_eq!(model.get_aggr_str(&subfield), "3.30");
    model.system_pct = None;
    assert_eq!(model.get_system_pct_str_styled().source(), "0.0    ");
    assert_eq!(model.get_cache_usage_str_styled().source(), "  -->10");
    assert_eq!(model.get_user_pct_value(&subfield).unwrap(), 1.1);
    assert_eq!(model.get_user_pct_str_styled(&subfield).source(), "1.1%   ");
    assert_eq!(model.get_user_pct_str(&subfield), "1.1%");
    assert_eq!(model.get_loopback_str_styled(&subfield).source(), "1.1    ");
    assert_eq!(model.get_route_str_styled(&subfield).source(), "2.2     ");
    assert_eq!(model.get_loopback_str(&subfield), "1.1");
    assert_eq!(model.get_route_str(&subfield), "2.2");
    assert_eq!(
        model.get_field_line(&subfield).source(),
        "12.6%   1.1%    0.0       -->10 1.1     2.2      3.30  "
    );
    assert_eq!(
        model.get_csv_field(&subfield),
        "12.6%,1.1%,0.0,100 MB,1.1,2.2,3.30,"
    );
    assert_eq!(model.something_else, Some(0.0));
}

#[test]
fn test_bdecor_field_highlight() {
    let mut model = TestModel::new();

    // Test regular function call
    assert_eq!(model.get_usage_pct_str(), "12.6%");
    let usage_pct = model.get_usage_pct_str_styled();
    assert_eq!(
        usage_pct,
        StyledString::styled(usage_pct.source(), Color::Light(BaseColor::Green))
    );
    assert_ne!(usage_pct, StyledString::plain(usage_pct.source()));
    model.usage_pct = Some(8.0);
    let usage_pct = model.get_usage_pct_str_styled();
    assert_eq!(
        usage_pct,
        StyledString::styled(
            usage_pct.source(),
            cursive::theme::Color::Light(BaseColor::Red)
        )
    );

    // Test lambda
    assert_eq!(model.get_system_pct_str(), "2.2%");
    assert_eq!(model.get_system_pct_str_styled().source(), "2.2%   ");
    assert_eq!(
        model.get_system_pct_str_styled(),
        StyledString::styled("2.2%   ", Color::Light(BaseColor::Red))
    );
    assert_ne!(
        model.get_system_pct_str_styled(),
        StyledString::plain("2.2%   ")
    );
}

#[test]
fn test_bdecor_title_function() {
    let model = TestModel::new();
    let subfield = SubField::new();
    assert_eq!(model.get_user_pct_title(&subfield), "User");
    assert_eq!(model.get_loopback_title(&subfield), "Field A");
    assert_eq!(model.get_route_title(&subfield), "Field B");
    assert_eq!(model.get_user_pct_title_styled(&subfield), "User   ");
    assert_eq!(model.get_loopback_title_styled(&subfield), "Field A");
    assert_eq!(model.get_route_title_styled(&subfield), "Field B ");
    assert_eq!(
        model.get_title_line(&subfield),
        "Usage   User    System  L1 Cach Field A Field B  Aggr  "
    );
    assert_eq!(
        model.get_csv_title(&subfield),
        "Usage,User,System,L1 Cache,Field A,Field B,Aggr,"
    );
}

#[test]
fn test_bdecor_cmp_function() {
    let model = TestModel::new();
    let mut m1 = TestModel::new();
    m1.usage_pct = Some(13.0);
    let mut arr = vec![TestModel::new(), TestModel::new(), m1];
    arr.sort_by(|a, b| {
        model
            .cmp_by_usage_pct(a, b)
            .unwrap_or(std::cmp::Ordering::Equal)
            .reverse()
    });

    assert_eq!(arr[0].get_usage_pct_value().unwrap(), 13.0);
    arr[0].usage_pct = Some(11.0);
    arr.sort_by(|a, b| {
        model
            .cmp_by_usage_pct(a, b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    assert_eq!(arr[0].get_usage_pct_value().unwrap(), 11.0);
}

#[test]
fn test_bdecor_interleave() {
    let model = TestModel::new();
    let subfield = SubField::new();
    let lines = model.get_interleave_line(": ", &subfield);
    let mut string_lines = String::new();
    for line in lines {
        string_lines += line.source();
        string_lines += "\n";
    }
    assert_eq!(
        string_lines,
        "Usage  : 12.6%  \nUser   : 1.1%   \nSystem : 2.2%   \nL1 Cach:   -->10\nField A: 1.1    \nField B : 2.2     \nAggr : 3.30 \n"
    );
}

#[test]
fn compound_decorator() {
    static FIO: Lazy<Arc<RwLock<String>>> = Lazy::new(|| Arc::new(RwLock::new(String::new())));
    static TIO: Lazy<Arc<RwLock<String>>> = Lazy::new(|| Arc::new(RwLock::new(String::new())));

    struct FakeFileIO(Sender<bool>, Sender<bool>);
    impl io::Write for FakeFileIO {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.1.send(true).unwrap();
            let mut file_io = FIO.write().unwrap();
            let content = String::from_utf8(buf.to_vec()).unwrap();
            let content_size = content.len();
            *file_io += &content;
            // Depend on the ending char to sendout notification.
            if content.ends_with('\n') {
                self.0.send(true).unwrap();
            }
            Ok(content_size)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct FakeTermIO(Sender<bool>);
    impl io::Write for FakeTermIO {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut term_io = TIO.write().unwrap();
            let content = String::from_utf8(buf.to_vec()).unwrap();
            let content_size = content.len();
            *term_io += &content;
            // Depend on the ending char to sendout notification.
            if content.ends_with('\n') {
                self.0.send(true).unwrap();
            }
            Ok(content_size)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let (ftx, frx) = channel::<bool>();
    let (ttx, trx) = channel::<bool>();
    let (rtx, rrx) = channel::<bool>();

    let decorator = logutil::CompoundDecorator::new(FakeFileIO(ftx, rtx), FakeTermIO(ttx));
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let logger = slog::Logger::root(drain, o!());

    error!(logger, "Go both");
    let timeout = Duration::from_secs(3);
    frx.recv_timeout(timeout).expect("failed in file logging.");
    trx.recv_timeout(timeout).expect("failed in term logging.");
    {
        let file = FIO.read().unwrap();
        let term = TIO.read().unwrap();
        assert_eq!(&file[file.len() - 8..], "Go both\n");
        assert_eq!(&term[term.len() - 8..], "Go both\n");
    }

    logutil::set_current_log_target(logutil::TargetLog::File);

    error!(logger, "Go file");
    frx.recv_timeout(timeout).expect("failed in file logging");
    {
        let file = FIO.read().unwrap();
        let term = TIO.read().unwrap();
        assert_eq!(&file[file.len() - 8..], "Go file\n");
        assert_eq!(&term[term.len() - 8..], "Go both\n");
    }

    logutil::set_current_log_target(logutil::TargetLog::Term);

    error!(logger, "Go term");
    trx.recv_timeout(timeout).expect("failed in term logging");
    {
        let file = FIO.read().unwrap();
        let term = TIO.read().unwrap();
        assert_eq!(&file[file.len() - 8..], "Go file\n");
        assert_eq!(&term[term.len() - 8..], "Go term\n");
    }

    logutil::set_current_log_target(logutil::TargetLog::All);

    error!(logger, "Go both");
    frx.recv_timeout(timeout).expect("failed in file logging.");
    trx.recv_timeout(timeout).expect("failed in term logging.");
    {
        let file = FIO.read().unwrap();
        let term = TIO.read().unwrap();
        assert_eq!(&file[file.len() - 8..], "Go both\n");
        assert_eq!(&term[term.len() - 8..], "Go both\n");
    }
    rrx.try_iter().count();

    // Testing race condition during change target and flush
    logutil::set_current_log_target(logutil::TargetLog::File);
    error!(
        logger,
        "Something really long that will take multiple writes"
    );
    rrx.recv_timeout(timeout)
        .expect("Race logger initial wait failed.");
    logutil::set_current_log_target(logutil::TargetLog::Term);
    frx.recv_timeout(timeout)
        .expect("file logger raced with term logger");
    if trx.recv_timeout(timeout).is_ok() {
        panic!("Term logger raced with file logger");
    }
}
