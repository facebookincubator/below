use std::path::Path;

pub struct Statistics {}

impl Statistics {
    pub fn new() -> Statistics {
        Statistics {}
    }

    pub fn report_store_size<P: AsRef<Path>>(&mut self, _dir: P) {}
}
