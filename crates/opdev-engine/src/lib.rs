//! Evidence collection, safe command execution, and strict gate aggregation.

#![forbid(unsafe_code)]

mod command;
mod evaluator;
mod report;
mod test_report;

pub use command::{CommandError, Execution, execute};
pub use evaluator::{CheckOptions, EvaluationError, evaluate, reaggregate};
pub use report::{CheckKind, CheckReport, CheckResult};
pub use test_report::{TestReportInspection, inspect_junit};
