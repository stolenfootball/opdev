use super::{CiProvider, RunExpectation, Value, Verdict, number, string};
use opdev_project::RequiredCheck;

pub(super) fn compare(
    provider: CiProvider,
    expected: &RunExpectation,
    required: &[RequiredCheck],
    rows: &[Value],
) -> Verdict {
    let mut uncertainty = None;
    for check in required {
        match compare_one(provider, expected, std::slice::from_ref(check), rows) {
            Ok(Err(failure)) => return Ok(Err(failure)),
            Err(reason) => uncertainty = Some(reason),
            Ok(Ok(())) => {}
        }
    }
    uncertainty.map_or(Ok(Ok(())), Err)
}

fn compare_one(
    provider: CiProvider,
    expected: &RunExpectation,
    required: &[RequiredCheck],
    rows: &[Value],
) -> Verdict {
    let mut missing = false;
    for check in required {
        let mut matching = Vec::new();
        let mut ids = std::collections::BTreeSet::new();
        for row in rows {
            if string(row, "name")? != check.name {
                continue;
            }
            let id = number(row, "id")?;
            if !ids.insert(id) {
                return Err("Duplicate check identity in provider collection".into());
            }
            matching.push((id, row));
        }
        // Select by provider identity, not success or trusted producer. A newer
        // wrong-source/pending/failed check cannot fall back to old green evidence.
        let Some((_, row)) = matching.into_iter().max_by_key(|(id, _)| *id) else {
            missing = true;
            continue;
        };
        let github = provider == CiProvider::Github;
        let producer = number(&row[if github { "app" } else { "creator" }], "id")?;
        if producer != check.producer_id {
            return Ok(Err(format!(
                "Required check {:?} came from an unexpected producer",
                check.name
            )));
        }
        if !string(row, if github { "head_sha" } else { "sha" })?
            .eq_ignore_ascii_case(&expected.revision)
            || (!github && string(row, "ref")? != expected.reference)
            || (!github
                && row.get("pipeline_id").is_some()
                && number(row, "pipeline_id")? != expected.run_id)
        {
            return Ok(Err(
                "Required check belongs to a different revision, ref or pipeline".into(),
            ));
        }
        let status = string(row, "status")?;
        let verdict = if github {
            if status != "completed" {
                missing = true;
                continue;
            }
            if row["conclusion"].is_null() {
                missing = true;
                continue;
            }
            string(row, "conclusion")?
        } else {
            status
        };
        match verdict.as_str() {
            "success" => {}
            "failure" | "failed" | "cancelled" | "canceled" | "timed_out" | "startup_failure" => {
                return Ok(Err(format!(
                    "Required check {:?} did not succeed",
                    check.name
                )));
            }
            _ => missing = true,
        }
    }
    if missing {
        Err(
            "One or more required checks are missing, pending, skipped or otherwise unverified"
                .into(),
        )
    } else {
        Ok(Ok(()))
    }
}
