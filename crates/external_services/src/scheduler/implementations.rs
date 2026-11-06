//! Scheduler implementations

pub mod invokr;
pub mod noop;

pub use invokr::InvokrClient;
pub use noop::NoopSchedulerClient;
