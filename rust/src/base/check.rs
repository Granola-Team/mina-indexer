//! Check trait

pub trait Check
where
    Self: std::fmt::Debug + PartialEq,
{
    /// Type-specific check - reports about specified mismatching fields
    fn check(&self, other: &Self) -> bool;

    /// Logs mismatching values
    fn log_check(&self, other: &Self, msg: &str) {
        if self.check(other) {
            log::error!("Mismatch: {}\n{:?}\n{:?}", msg, self, other)
        }
    }
}
