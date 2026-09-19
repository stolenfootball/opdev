//! Evidence collection, safe command execution, and strict gate aggregation.

#![forbid(unsafe_code)]
mod acceptance;

mod command;
mod evaluator;
mod report;
mod test_execution;
mod test_report;

pub use command::{CommandError, Execution, execute};
pub use evaluator::{CheckOptions, EvaluationError, evaluate, reaggregate};
pub use report::{CheckKind, CheckReport, CheckResult};
pub use test_execution::{TestExecutionReceipt, observe_test_execution};
pub use test_report::{TestReportInspection, inspect_junit};
