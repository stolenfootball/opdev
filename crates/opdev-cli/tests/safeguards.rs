//! Generic risk objectives must reach real canonical assertions, not generic pass prose.
use opdev_project::{MANIFEST_PATH, TrackedEvidence, discover, staged_fingerprint};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn git(root: &Path, args: &[&str]) -> Result {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(())
}
fn cli(root: &Path, args: &[&str]) -> Result<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root)
        .args(args)
        .output()?)
}

const CONTRACT: &str = "Existing callers keep integer addition, including negative values. Retained version-1 data upgrades to version 2 without dropping fields. An interrupted conversion keeps original bytes readable. No consumer transition is needed for this compatible change.\n";
const APP: &str = "def add(a, b):\n    return a + b\n\ndef migrate(record, interrupted=False):\n    if interrupted:\n        raise RuntimeError('interrupted')\n    return {**record, 'version': 2}\n";
const TEST: &str = "import unittest\nfrom app import add, migrate\nclass ContractTests(unittest.TestCase):\n    def test_old_callers(self):\n        self.assertEqual(add(7, -3), 4)\n        self.assertEqual(add(-5, 2), -3)\n    def test_retained_records(self):\n        old = {'version': 1, 'value': 9, 'extra': 'keep'}\n        self.assertEqual(migrate(old), {'version': 2, 'value': 9, 'extra': 'keep'})\n        self.assertEqual(old, {'version': 1, 'value': 9, 'extra': 'keep'})\n    def test_interruption_recovery(self):\n        old = {'version': 1, 'value': 9}\n        with self.assertRaises(RuntimeError):\n            migrate(old, interrupted=True)\n        self.assertEqual(old, {'version': 1, 'value': 9})\n        self.assertEqual(migrate(old), {'version': 2, 'value': 9})\n";

fn fixture() -> Result<tempfile::TempDir> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    git(root, &["init", "-q", "-b", "main"])?;
    fs::write(root.join("contract.md"), CONTRACT)?;
    fs::write(root.join("app.py"), APP)?;
    fs::write(root.join("test_app.py"), TEST)?;
    let mut manifest = serde_json::to_value(discover(root)?.manifest)?;
    manifest["schema"] = json!(3);
    manifest["assurance"] = json!({"profiles":[], "engineering":{"version":"1", "review_reference":"fixture:policy"},
        "safeguards":{"version":1,"review_reference":"fixture:explicit-choice", "capabilities":{}}});
    for cap in opdev_project::Capability::ALL {
        let name = serde_json::to_value(cap)?
            .as_str()
            .ok_or("name")?
            .to_owned();
        manifest["assurance"]["safeguards"]["capabilities"][&name] = json!({
            "state": if matches!(cap, opdev_project::Capability::PersistentData | opdev_project::Capability::PublicContract) {"present"} else {"absent"},
            "rationale":"Neutral fixture capability assessment; no production claim", "authority":"contract.md"});
    }
    manifest["commands"] = json!({"check":{"argv":[if cfg!(windows) {"python"} else {"python3"},"-B","-m","unittest"],"timeout_seconds":30}});
    manifest["testing"]["suites"] =
        json!([{"id":"check","command":"check","stages":["local","pre_merge","post_merge"]}]);
    fs::create_dir_all(root.join(".opdev"))?;
    fs::write(
        root.join(MANIFEST_PATH),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    git(root, &["add", "."])?;
    let source = TrackedEvidence::bind(root, "contract.md".into(), "Existing callers".into())?;
    let target = TrackedEvidence::bind(root, "test_app.py".into(), "self.assertEqual".into())?;
    let mut conditions = vec![];
    let mut mappings = vec![];
    for (id, assertion) in [
        (
            "callers",
            "Negative and positive old callers retain arithmetic results",
        ),
        (
            "data",
            "Retained extra fields and original dictionary remain intact",
        ),
        (
            "recovery",
            "Interruption raises and preserves original data; retry transforms it correctly",
        ),
    ] {
        conditions
            .push(json!({"id":id,"statement":assertion,"authority":"contract.md","source":source}));
        mappings.push(json!({"condition":id,"method":"automated","target":target,"assertion":assertion,
            "discriminating_case":"Incorrect arithmetic, dropped extra fields or mutation before interruption fails the exact assertions", "suite":"check","outcome":"passed"}));
    }
    let ledger = json!({"schema":2,"project":[],"changes":[{"fingerprint":"", "work":"fixture:safeguards", "assertions":[],
        "acceptance":{"scope":"behavioral","rationale":"Reviewed API and in-memory retained-data transition fixture; not production migration proof", "conditions":conditions,"verifications":mappings,
            "safeguards":{"version":1,"impacts":{"public_contract":{"impact":"affected","rationale":"Public caller behavior changes"},
                "persistent_data":{"impact":"affected","rationale":"Retained record format changes"}},
                "objectives":{"consumer_compatibility":["callers"],"consumer_transition":["callers"],"retained_data_compatibility":["data"],"data_recovery":["recovery"]}},
            "review":{"outcome":"passed","reviewer":"synthetic fixture reviewer","reference":"fixture:review","rationale":"Explicit fixture oracle only","subject_sha256":""}}}]});
    save(root, ledger)?;
    Ok(temp)
}

fn save(root: &Path, mut ledger: Value) -> Result {
    git(root, &["add", "."])?;
    ledger["changes"][0]["fingerprint"] = json!(staged_fingerprint(root)?);
    let mut ledger: opdev_project::EvidenceLedger = serde_json::from_value(ledger)?;
    let change = &mut ledger.changes[0];
    let acceptance = change.acceptance.as_mut().ok_or("acceptance")?;
    acceptance.review.subject_sha256 = acceptance.digest(&change.fingerprint, &change.work)?;
    fs::write(root.join(".opdev/evidence.yaml"), ledger.to_yaml()?)?;
    git(root, &["add", "."])?;
    Ok(())
}
fn ledger(root: &Path) -> Result<Value> {
    Ok(serde_saphyr::from_str(&fs::read_to_string(
        root.join(".opdev/evidence.yaml"),
    )?)?)
}
fn check(root: &Path, extra: &[&str]) -> Result<Value> {
    let mut argv = vec!["check", "--ci", "--format", "json"];
    argv.extend_from_slice(extra);
    let output = cli(root, &argv)?;
    assert!(
        matches!(output.status.code(), Some(0 | 1)),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout)?;
    Ok(report["rules"]
        .as_array()
        .ok_or("rules")?
        .iter()
        .find(|r| r["rule_id"] == "OPDEV-TEST-002")
        .ok_or("rule")?
        .clone())
}

#[test]
fn real_api_and_data_assertions_pass_then_detect_incompatibility_and_interruption_loss() -> Result {
    for mode in [
        "healthy",
        "bad-api",
        "bad-data",
        "bad-recovery",
        "no-execution",
    ] {
        let f = fixture()?;
        let root = f.path();
        let changed = match mode {
            "bad-api" => APP.replace("a + b", "a - b"),
            "bad-data" => APP.replace(
                "{**record, 'version': 2}",
                "{'version': 2, 'value': record['value']}",
            ),
            "bad-recovery" => APP.replace(
                "raise RuntimeError",
                "record.clear()\n        raise RuntimeError",
            ),
            _ => APP.into(),
        };
        fs::write(root.join("app.py"), changed)?;
        save(root, ledger(root)?)?;
        let result = check(
            root,
            if mode == "no-execution" {
                &["--no-exec"]
            } else {
                &[]
            },
        )?;
        assert_eq!(
            result["outcome"],
            match mode {
                "healthy" => "passed",
                "no-execution" => "unverified",
                _ => "failed",
            },
            "{mode}: {result}"
        );
    }
    Ok(())
}

#[test]
fn missing_unknown_contradictory_or_unmapped_capabilities_cannot_use_a_green_suite() -> Result {
    for mode in [
        "missing-review",
        "missing-fact",
        "unknown",
        "false-absence",
        "omitted-present",
        "missing-objective",
        "unknown-condition",
        "no-material",
    ] {
        let f = fixture()?;
        let root = f.path();
        let mut record = ledger(root)?;
        let mut project: Value = serde_json::from_slice(&fs::read(root.join(MANIFEST_PATH))?)?;
        let acceptance = &mut record["changes"][0]["acceptance"];
        match mode {
            "missing-review" => {
                acceptance
                    .as_object_mut()
                    .ok_or("object")?
                    .remove("safeguards");
            }
            "missing-fact" => {
                project["assurance"]["safeguards"]["capabilities"]
                    .as_object_mut()
                    .ok_or("caps")?
                    .remove("persistent_data");
            }
            "unknown" => {
                project["assurance"]["safeguards"]["capabilities"]["persistent_data"]["state"] =
                    json!("unknown");
            }
            "false-absence" => {
                project["assurance"]["safeguards"]["capabilities"]["persistent_data"]["state"] =
                    json!("absent");
            }
            "omitted-present" => {
                acceptance["safeguards"]["impacts"]
                    .as_object_mut()
                    .ok_or("impacts")?
                    .remove("persistent_data");
            }
            "missing-objective" => {
                acceptance["safeguards"]["objectives"]
                    .as_object_mut()
                    .ok_or("objectives")?
                    .remove("data_recovery");
            }
            "unknown-condition" => {
                acceptance["safeguards"]["objectives"]["data_recovery"] = json!(["invented"]);
            }
            _ => {
                acceptance["scope"] = json!("no_material_conditions");
                acceptance["conditions"] = json!([]);
                acceptance["verifications"] = json!([]);
            }
        }
        fs::write(root.join(MANIFEST_PATH), serde_json::to_vec(&project)?)?;
        save(root, record)?;
        let result = check(root, &[])?;
        assert_eq!(result["outcome"], "unverified", "{mode}: {result}");
        assert!(
            !result["diagnostic"]
                .as_str()
                .ok_or("diagnostic")?
                .is_empty()
        );
    }
    Ok(())
}

#[test]
fn reviewed_unaffected_or_absent_capabilities_do_not_force_unrelated_checks() -> Result {
    let f = fixture()?;
    let root = f.path();
    let mut record = ledger(root)?;
    let safeguards = &mut record["changes"][0]["acceptance"]["safeguards"];
    safeguards["impacts"]["persistent_data"] = json!({"impact":"unaffected","rationale":"This fixture task affects callers only; retained format and persistence code are unchanged"});
    safeguards["objectives"]
        .as_object_mut()
        .ok_or("objectives")?
        .remove("retained_data_compatibility");
    safeguards["objectives"]
        .as_object_mut()
        .ok_or("objectives")?
        .remove("data_recovery");
    save(root, record)?;
    assert_eq!(check(root, &[])?["outcome"], "passed");
    Ok(())
}

#[test]
fn changed_impact_invalidates_review_and_legacy_omission_keeps_digest_stable() -> Result {
    let f = fixture()?;
    let root = f.path();
    let mut record = ledger(root)?;
    record["changes"][0]["acceptance"]["safeguards"]["impacts"]["persistent_data"]["rationale"] =
        json!("changed after review");
    fs::write(
        root.join(".opdev/evidence.yaml"),
        serde_json::to_vec(&record)?,
    )?;
    assert_eq!(check(root, &[])?["outcome"], "unverified");
    let mut acceptance: opdev_project::AcceptanceEvidence =
        serde_json::from_value(record["changes"][0]["acceptance"].clone())?;
    acceptance.safeguards = None;
    let value = serde_json::to_value(&acceptance)?;
    assert!(value.get("safeguards").is_none());
    let original = json!({"protocol":1,"fingerprint":"test","work":"test","scope":acceptance.scope,"rationale":acceptance.rationale,"conditions":acceptance.conditions,"verifications":acceptance.verifications});
    assert_eq!(
        acceptance.digest("test", "test")?,
        format!("{:x}", Sha256::digest(serde_json::to_vec(&original)?))
    );
    Ok(())
}

#[test]
fn remaining_capabilities_require_their_own_observed_assertions() -> Result {
    // Small representative product boundaries, not production conformance claims.
    let cases = [
        (
            "distribution",
            vec!["installation_update", "distribution_recovery"],
            "from pathlib import Path\nimport tempfile\nwith tempfile.TemporaryDirectory() as d:\n p=Path(d)/'installed'\n p.write_text('old')\n candidate=Path(d)/'candidate'\n candidate.write_text('new')\n assert p.read_text() == 'old'\n candidate.replace(p)\n assert p.read_text() == 'new'\n p.write_text('old')\n assert p.read_text() == 'old'\n",
            "candidate.replace(p)",
            "candidate.unlink()",
        ),
        (
            "security_boundary",
            vec!["security_controls"],
            "def authorized(role):\n return role == 'owner'\nassert authorized('owner')\nassert not authorized('guest')\nassert not authorized('OWNER')\n",
            "role == 'owner'",
            "True",
        ),
        (
            "operations",
            vec!["operational_recovery"],
            "def status(ready):\n return (0, 'ready') if ready else (1, 'dependency unavailable')\nassert status(True) == (0, 'ready')\nassert status(False) == (1, 'dependency unavailable')\n",
            "(1, 'dependency unavailable')\nassert",
            "(0, 'ready')\nassert",
        ),
        (
            "user_interface",
            vec!["accessibility"],
            "def message(ok):\n return 'passed' if ok else 'failed: fix the input'\nassert message(True) == 'passed'\nassert message(False) == 'failed: fix the input'\nassert '\\x1b' not in message(False)\n",
            "else 'failed: fix the input'",
            "else '\\x1b[31m'",
        ),
        (
            "effectiveness",
            vec!["effectiveness_evaluation"],
            "labels=[1,0,1,0]\npredicted=[1,0,1,0]\naccuracy=sum(a==b for a,b in zip(labels,predicted))/len(labels)\nassert accuracy >= .75\n# Synthetic frozen observations only, not real-world utility.\n",
            "predicted=[1,0,1,0]",
            "predicted=[0,1,0,1]",
        ),
    ];
    for (capability, objectives, script, original, defect) in cases {
        for mode in ["healthy", "defect", "missing-objective"] {
            let f = fixture()?;
            let root = f.path();
            let mut project: Value = serde_json::from_slice(&fs::read(root.join(MANIFEST_PATH))?)?;
            project["assurance"]["safeguards"]["capabilities"][capability]["state"] =
                json!("present");
            project["commands"]["check"]["argv"] = json!([
                if cfg!(windows) { "python" } else { "python3" },
                "-B",
                "boundary.py"
            ]);
            fs::write(root.join(MANIFEST_PATH), serde_json::to_vec(&project)?)?;
            fs::write(
                root.join("boundary.py"),
                if mode == "defect" {
                    script.replace(original, defect)
                } else {
                    script.into()
                },
            )?;
            git(root, &["add", "."])?;
            let mut record = ledger(root)?;
            let acceptance = &mut record["changes"][0]["acceptance"];
            acceptance["safeguards"]["impacts"] = json!({
                "public_contract":{"impact":"unaffected","rationale":"Separate unchanged library boundary"},
                "persistent_data":{"impact":"unaffected","rationale":"Separate unchanged record format"}});
            acceptance["safeguards"]["impacts"][capability] = json!({"impact":"affected","rationale":"This fixture changes the selected consumer boundary"});
            acceptance["conditions"] = json!([{"id":"boundary","statement":"Representative affected boundary preserves expected behavior", "authority":"fixture:boundary", "source":TrackedEvidence::bind(root,"contract.md".into(),"Existing callers".into())?}]);
            acceptance["verifications"] = json!([{"condition":"boundary","method":"automated", "target":TrackedEvidence::bind(root,"boundary.py".into(),"assert".into())?, "assertion":"Inspect explicit success/failure and boundary assertions in the selected fixture", "discriminating_case":"The paired seeded defect violates the explicit assertion rather than failing setup", "suite":"check","outcome":"passed"}]);
            acceptance["safeguards"]["objectives"] = json!({});
            for objective in &objectives {
                acceptance["safeguards"]["objectives"][*objective] = json!(["boundary"]);
            }
            if mode == "missing-objective" {
                acceptance["safeguards"]["objectives"]
                    .as_object_mut()
                    .ok_or("objectives")?
                    .remove(objectives[0]);
            }
            save(root, record)?;
            let result = check(root, &[])?;
            assert_eq!(
                result["outcome"],
                match mode {
                    "healthy" => "passed",
                    "defect" => "failed",
                    _ => "unverified",
                },
                "{capability}/{mode}: {result}"
            );
        }
    }
    Ok(())
}

#[test]
fn unsupported_policy_or_forged_opt_out_is_rejected_and_stage_execution_stays_distinct() -> Result {
    let f = fixture()?;
    let root = f.path();
    let original = fs::read(root.join(MANIFEST_PATH))?;
    for mode in ["legacy-policy", "future", "waiver", "missing-authority"] {
        let mut project: Value = serde_json::from_slice(&original)?;
        match mode {
            "legacy-policy" => {
                project["schema"] = json!(1);
                project["assurance"]
                    .as_object_mut()
                    .ok_or("assurance")?
                    .remove("engineering");
            }
            "future" => project["assurance"]["safeguards"]["version"] = json!(99),
            "waiver" => project["assurance"]["safeguards"]["waived"] = json!(["data_recovery"]),
            _ => {
                project["assurance"]["safeguards"]["capabilities"]["persistent_data"]["authority"] =
                    json!(" ");
            }
        }
        assert!(
            opdev_project::ProjectManifest::from_yaml(&serde_json::to_string(&project)?).is_err(),
            "{mode}"
        );
    }
    let mut project: Value = serde_json::from_slice(&original)?;
    project["testing"]["suites"][0]["stages"] = json!(["pre_merge"]);
    fs::write(root.join(MANIFEST_PATH), serde_json::to_vec(&project)?)?;
    save(root, ledger(root)?)?;
    assert_eq!(check(root, &[])?["outcome"], "passed");
    let result = check(root, &["--post-merge"])?;
    assert_eq!(result["outcome"], "unverified");
    assert!(
        result["diagnostic"]
            .as_str()
            .ok_or("diagnostic")?
            .contains("post_merge")
    );
    Ok(())
}
