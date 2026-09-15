//! Read-only validation of active experiment plans, not behavioral evidence.

use std::collections::HashSet;

use serde::Deserialize;
use thiserror::Error;

use crate::{ProjectManifest, TestStage};

const SCHEMA: &str = include_str!("../../../schema/experiment.schema.json");

/// Invalid experiment input or unavailable validation tooling.
#[derive(Debug, Error)]
pub enum ExperimentError {
    /// The record is not valid YAML or JSON.
    #[error("invalid experiment YAML: {0}")]
    Yaml(#[from] serde_saphyr::Error),
    /// Bundled schema or typed conversion failed.
    #[error("experiment schema/conversion error: {0}")]
    Json(#[from] serde_json::Error),
    /// The bundled validator could not be built.
    #[error("could not compile experiment schema: {0}")]
    Schema(String),
    /// Required metadata or cross-references are invalid.
    #[error("invalid active experiment record: {0}")]
    Invalid(String),
}

#[derive(Deserialize)]
struct Record {
    id: String,
    review_by: String,
    configurations: Vec<Configuration>,
}

#[derive(Deserialize)]
struct Configuration {
    id: String,
    exposure: String,
    suites: Vec<String>,
}

/// Validates an active record against its schema and declared project suites.
///
/// `today` is the UTC day count since the Unix epoch, supplied by the caller so
/// date-bound tests are deterministic. The review date is inclusive. Validation
/// never executes suites, fetches work items, writes evidence, or qualifies a gate.
/// Returns the record ID only after structural and cross-field validation.
///
/// # Errors
///
/// Returns [`ExperimentError`] for malformed input, missing coverage declarations,
/// duplicate configurations, or an overdue review.
pub fn validate_experiment(
    yaml: &str,
    manifest: &ProjectManifest,
    today: i64,
) -> Result<String, ExperimentError> {
    let value: serde_json::Value = serde_saphyr::from_str(yaml)?;
    let schema: serde_json::Value = serde_json::from_str(SCHEMA)?;
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| ExperimentError::Schema(error.to_string()))?;
    let errors: Vec<_> = validator
        .iter_errors(&value)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    if !errors.is_empty() {
        return Err(ExperimentError::Invalid(errors.join("; ")));
    }
    let record: Record = serde_json::from_value(value)?;
    if date_days(&record.review_by)? < today {
        return Err(ExperimentError::Invalid(format!(
            "review overdue since {}; review the decision and cleanup plan before extending it",
            record.review_by
        )));
    }
    let mut ids = HashSet::new();
    let mut stable = false;
    let mut experimental = false;
    for config in &record.configurations {
        if !ids.insert(&config.id) {
            return Err(ExperimentError::Invalid(format!(
                "duplicate configuration `{}`",
                config.id
            )));
        }
        stable |= config.exposure == "stable";
        experimental |= config.exposure == "experimental";
        let mut pre_merge = false;
        let mut post_merge = false;
        for id in &config.suites {
            let suite = manifest
                .testing
                .suites
                .iter()
                .find(|suite| &suite.id == id)
                .ok_or_else(|| {
                    ExperimentError::Invalid(format!(
                        "configuration `{}` references unknown test suite `{id}`",
                        config.id
                    ))
                })?;
            pre_merge |= suite.stages.contains(&TestStage::PreMerge);
            post_merge |= suite.stages.contains(&TestStage::PostMerge);
        }
        if !pre_merge || !post_merge {
            return Err(ExperimentError::Invalid(format!(
                "configuration `{}` requires declared pre_merge and post_merge suite coverage",
                config.id
            )));
        }
    }
    if !stable || !experimental {
        return Err(ExperimentError::Invalid(
            "declare both stable and experimental configurations".into(),
        ));
    }
    Ok(record.id)
}

// Gregorian civil date to days since 1970-01-01. Calendar validity is checked
// by JSON Schema before this conversion; checked parsing still avoids panics.
fn date_days(date: &str) -> Result<i64, ExperimentError> {
    let parts: Vec<i64> = date
        .split('-')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .map_err(|_| ExperimentError::Invalid("review_by must be YYYY-MM-DD".into()))?;
    let [year, month, day] = parts.as_slice() else {
        return Err(ExperimentError::Invalid(
            "review_by must be YYYY-MM-DD".into(),
        ));
    };
    let year = year - i64::from(*month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = month + if *month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month + 2) / 5 + day - 1;
    Ok(
        era * 146_097 + year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year
            - 719_468,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates_match_epoch_and_leap_boundaries() -> Result<(), ExperimentError> {
        assert_eq!(date_days("1970-01-01")?, 0);
        assert_eq!(date_days("2000-01-01")?, 10_957);
        assert_eq!(date_days("2024-03-01")? - date_days("2024-02-28")?, 2);
        assert_eq!(date_days("2100-03-01")? - date_days("2100-02-28")?, 1);
        assert!(date_days("invalid").is_err());
        Ok(())
    }
}
