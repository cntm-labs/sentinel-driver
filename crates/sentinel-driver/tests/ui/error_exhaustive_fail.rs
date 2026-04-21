//! Exhaustive match on `Error` without a wildcard must fail to compile
//! because the enum is `#[non_exhaustive]` starting in v1.1.0.
use sentinel_driver::Error;

fn handle(e: Error) -> &'static str {
    match e {
        Error::ConnectionClosed => "closed",
        Error::TransactionCompleted => "done",
        // intentionally no wildcard — must be a compile error
    }
}

fn main() {}
