//! Native validator boundaries, with no worker dispatch or executable PATH.
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn subject() -> Value {
    json!({"schema":1,"source_sha256":"a".repeat(64),"configuration_sha256":"b".repeat(64),"stage":"pre_merge","artifact_sha256":null})
}

fn assignment(id: &str, owned: &str) -> Value {
    json!({"schema":1,"protocol":"delegation.v1","id":id,"parent_work":"tracker:neutral/1",
        "role":"implementer","outcome":"Fix the accepted empty-input behavior",
        "original_authorities":["tracker:neutral/1#acceptance"],"subject":subject(),
        "acceptance_sha256":"c".repeat(64),"owned_paths":[owned],
        "allowed_actions":["read","edit","focused_check"],"checks":["empty-input"],
        "model":"approved-model","effort":"medium","max_seconds":60,"max_tokens":1000,
        "stop_condition":"Return after focused check or a blocking ambiguity",
        "return_contract":"delegation-result.schema.json"})
}

fn result() -> Value {
    json!({"schema":1,"assignment_id":"one","subject":subject(),"acceptance_sha256":"c".repeat(64),
        "status":"completed","observed_model":"approved-model","observed_effort":"medium",
        "elapsed_seconds":10,"input_tokens":100,"cached_input_tokens":80,"output_tokens":20,
        "changed_paths":["src/lib.rs"],"performed_actions":["read","edit","focused_check"],
        "checks_executed":["empty-input"],"evidence":["local:diff","local:check"],"findings":[],
        "limitations":["Controller must inspect actual assertions"],"unresolved_decisions":[]})
}

fn run(
    root: &Path,
    assignment: &Value,
    result: Option<&Value>,
    active: &Value,
) -> Result<(i32, String), Box<dyn std::error::Error>> {
    for (name, value) in [
        ("assignment.json", assignment),
        ("subject.json", &subject()),
        ("active.json", active),
    ] {
        fs::write(root.join(name), serde_json::to_vec(value)?)?;
    }
    let mut command = Command::new(env!("CARGO_BIN_EXE_opdev"));
    command.current_dir(root).env("PATH", "").args([
        "delegation",
        "--assignment",
        "assignment.json",
        "--subject",
        "subject.json",
        "--acceptance-sha256",
        &"c".repeat(64),
        "--active",
        "active.json",
    ]);
    if let Some(result) = result {
        fs::write(root.join("result.json"), serde_json::to_vec(result)?)?;
        command.args(["--result", "result.json"]);
    }
    let before: Vec<_> = fs::read_dir(root)?
        .map(|p| {
            let p = p?;
            Ok::<_, std::io::Error>((p.file_name(), fs::read(p.path())?))
        })
        .collect::<Result<_, _>>()?;
    let output = command.output()?;
    for (name, bytes) in &before {
        assert_eq!(&fs::read(root.join(name))?, bytes);
    }
    assert_eq!(fs::read_dir(root)?.count(), before.len());
    Ok((
        output.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    ))
}

#[test]
fn schemas_and_native_validation_preserve_files_without_qualifying()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let assignment = assignment("one", "src");
    let result = result();
    for (schema, value) in [
        (
            include_str!("../../../schema/delegation-assignment.schema.json"),
            &assignment,
        ),
        (
            include_str!("../../../schema/delegation-result.schema.json"),
            &result,
        ),
    ] {
        assert!(jsonschema::is_valid(
            &serde_json::from_str::<Value>(schema)?,
            value
        ));
    }
    let (code, text) = run(root.path(), &assignment, Some(&result), &json!([]))?;
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("\"qualification\": \"unverified\""));
    assert!(!root.path().join(".git").exists());
    Ok(())
}

#[test]
fn stale_omitted_and_unauthorized_worker_claims_are_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let assignment = assignment("one", "src");
    for (pointer, value) in [
        ("/subject/source_sha256", json!("d".repeat(64))),
        ("/subject/configuration_sha256", json!("d".repeat(64))),
        ("/subject/stage", json!("post_merge")),
        ("/acceptance_sha256", json!("e".repeat(64))),
        ("/changed_paths", json!(["outside/file"])),
        ("/changed_paths", json!(["src/../outside"])),
        ("/performed_actions", json!(["read"])),
        ("/performed_actions", json!(["publish"])),
        ("/checks_executed", json!(["canonical-full-suite"])),
        ("/checks_executed", json!(["empty-input", "empty-input"])),
        ("/schema", json!(2)),
    ] {
        let mut result = result();
        *result.pointer_mut(pointer).ok_or("missing fixture field")? = value;
        let (code, text) = run(root.path(), &assignment, Some(&result), &json!([]))?;
        assert_eq!(code, 2, "{pointer}: {text}");
    }
    let mut assignment = assignment;
    assignment["role"] = json!("acceptance_reviewer");
    assert_eq!(run(root.path(), &assignment, None, &json!([]))?.0, 2);
    Ok(())
}

#[test]
fn incomplete_attempts_and_unknown_or_exceeded_budgets_remain_visible()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    for (field, value) in [
        ("status", json!("interrupted")),
        ("status", json!("unavailable")),
        ("observed_model", Value::Null),
        ("observed_effort", json!("high")),
        ("elapsed_seconds", Value::Null),
        ("elapsed_seconds", json!(61)),
        ("input_tokens", Value::Null),
        ("input_tokens", json!(1001)),
        ("evidence", json!([])),
    ] {
        let mut result = result();
        result[field] = value;
        let (code, text) = run(
            root.path(),
            &assignment("one", "src"),
            Some(&result),
            &json!([]),
        )?;
        assert_eq!(code, 1, "{field}: {text}");
        assert!(text.contains("unverified"));
    }
    Ok(())
}

#[test]
fn concurrent_writer_ownership_is_checked_across_the_entire_active_set()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let current = assignment("one", "src");
    for active in [
        json!([assignment("two", "SRC/lib.rs")]),
        json!([
            assignment("two", "tests"),
            assignment("three", "tests/unit")
        ]),
        json!([assignment("two", "tests"), assignment("two", "docs")]),
    ] {
        assert_eq!(run(root.path(), &current, None, &active)?.0, 2);
    }
    assert_eq!(
        run(
            root.path(),
            &current,
            None,
            &json!([assignment("two", "src-other")])
        )?
        .0,
        0
    );
    Ok(())
}
