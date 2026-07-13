#![forbid(unsafe_code)]

#[derive(Debug, PartialEq)]
pub struct BatchReport {
    pub processed: usize,
    pub failed: usize,
}

#[must_use]
pub fn empty_report() -> BatchReport {
    BatchReport {
        processed: 0,
        failed: 0,
    }
}
