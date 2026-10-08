//! Evidence collection, safe command execution, and strict gate aggregation.

#![forbid(unsafe_code)]
mod acceptance;
mod assessment;
pub use assessment::{EngineeringAssessment, FrameworkAssessment, RequirementAssessment};

mod command;
mod evaluator;
mod execution_policy;
mod execution_record;
pub use execution_policy::{
    ExecutionPolicy, ProducerPolicy, prepare_execution_bindings, run_canonical_producer,
};
mod plan;
mod report;
mod review;
pub use execution_record::{
    ExecutionBinding, ExecutionRecord, ValidatedExecutions, validate_producer_records,
};
pub use review::ValidatedReview;
mod test_execution;
mod test_report;

pub use command::{CommandError, Execution, ProgramLocation, execute, inspect_program};
pub use evaluator::{
    CheckOptions, EvaluationError, evaluate, evaluate_with_executions, evaluate_with_review,
    reaggregate,
};
pub use plan::{CheckPlan, PlannedCommand, plan_checks};
pub use report::{CheckKind, CheckReport, CheckResult};
pub use test_execution::{TestExecutionReceipt, observe_test_execution};
pub use test_report::{TestReportInspection, inspect_junit};
