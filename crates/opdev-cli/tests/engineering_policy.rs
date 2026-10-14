//! Real CLI policy preview, legacy preservation and honest assessment boundaries.
use opdev_project::{MANIFEST_PATH, ProjectManifest};
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn policy_three_is_explicit_and_reports_unverified_without_tests_or_review() -> TestResult {
    use opdev_project::{Capability, CapabilityFact, CapabilityState, SafeguardPolicy};
    let dir = fixture()?;
    let path = dir.path().join(MANIFEST_PATH);
    let mut project = ProjectManifest::load(&path)?;
    project.schema = 4;
    project.assurance.profiles.clear();
    project.assurance.engineering = Some(opdev_core::EngineeringPolicy {
        version: "3".into(),
        minimumcd: None,
        review_reference: "synthetic explicit decision".into(),
        maintenance_branches: vec![],
    });
    project.assurance.safeguards = Some(SafeguardPolicy {
        version: 2,
        review_reference: "synthetic explicit assessment".into(),
        capabilities: Capability::for_version(2)
            .iter()
            .map(|&capability| {
                (
                    capability,
                    CapabilityFact {
                        state: CapabilityState::Unknown,
                        rationale: "unresolved test fixture".into(),
                        authority: "contracts".into(),
                    },
                )
            })
            .collect(),
    });
    fs::write(&path, project.to_yaml()?)?;
    let output = cli(dir.path(), &["check", "--no-exec", "--format", "json"])?;
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["schema"], 4);
    assert_eq!(report["catalog_version"], 5);
    assert_eq!(report["engineering"]["version"], "3");
    let schema: Value = serde_json::from_str(include_str!("../../../schema/report.schema.json"))?;
    assert!(jsonschema::is_valid(&schema, &report));
    let project_schema: Value =
        serde_json::from_str(include_str!("../../../schema/project.schema.json"))?;
    assert!(jsonschema::is_valid(
        &project_schema,
        &serde_json::to_value(&project)?
    ));
    for id in [
        "OPDEV-VERIFY-001",
        "OPDEV-REVIEW-001",
        "OPDEV-LIFECYCLE-001",
    ] {
        assert_eq!(
            report["rules"]
                .as_array()
                .ok_or("rules")?
                .iter()
                .find(|r| r["rule_id"] == id)
                .ok_or("rule")?["outcome"],
            "unverified"
        );
    }
    project
        .assurance
        .safeguards
        .as_mut()
        .ok_or("safeguards")?
        .version = 1;
    assert!(
        project.to_yaml().is_err(),
        "policy 3 must not silently retain incomplete capability questions"
    );
    assert!(!jsonschema::is_valid(
        &project_schema,
        &serde_json::to_value(&project)?
    ));
    project
        .assurance
        .engineering
        .as_mut()
        .ok_or("engineering")?
        .version = "2".into();
    assert!(
        project.to_yaml().is_err(),
        "version-1 safeguards must not reinterpret new capabilities"
    );
    assert!(!jsonschema::is_valid(
        &project_schema,
        &serde_json::to_value(&project)?
    ));
    check_policy_three_preview(dir.path(), project)
}

fn check_policy_three_preview(root: &Path, mut project: ProjectManifest) -> TestResult {
    use opdev_project::{Capability, CapabilityState};
    let path = root.join(MANIFEST_PATH);
    project
        .assurance
        .safeguards
        .as_mut()
        .ok_or("safeguards")?
        .capabilities
        .retain(|cap, _| Capability::ALL.contains(cap));
    fs::write(&path, project.to_yaml()?)?;
    let before = fs::read(&path)?;
    let preview = cli(
        root,
        &[
            "upgrade",
            "--engineering-policy",
            "3",
            "--policy-review-reference",
            "fixture:decision",
            "--format",
            "json",
        ],
    )?;
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    assert_eq!(
        fs::read(&path)?,
        before,
        "preview must not migrate the project"
    );
    let preview: Value = serde_json::from_slice(&preview.stdout)?;
    let candidate =
        ProjectManifest::from_yaml(preview["candidate_yaml"].as_str().ok_or("candidate")?)?;
    assert_eq!(candidate.catalog()?.catalog_version, 5);
    assert_eq!(
        opdev_project::clean_adoption::target_name(&candidate),
        "clean-4"
    );
    assert_eq!(opdev_project::project_adoption_catalog(&candidate), 3);
    for cap in [Capability::SensitiveData, Capability::HighConsequence] {
        assert_eq!(
            candidate
                .assurance
                .safeguards
                .as_ref()
                .ok_or("safeguards")?
                .capabilities[&cap]
                .state,
            CapabilityState::Unknown
        );
    }
    Ok(())
}

#[test]
fn policy_three_adoption_reopens_changed_duties_without_losing_existing_facts() -> TestResult {
    use opdev_project::{AdoptionRecord, AdoptionState};
    let mut record = AdoptionRecord::pending_for_catalog(2)?;
    for decision in record.practices.values_mut() {
        decision.state = AdoptionState::Implemented;
        decision.reason = "Retained previous assessment; needs review where duties changed".into();
    }
    let old_reason = record.practices["setup"].reason.clone();
    record.migrate_catalog(3)?;
    assert_eq!(record.catalog_version, 3);
    assert!(record.review.is_none());
    for id in [
        "testing",
        "review",
        "observability",
        "sensitive_data",
        "domain_assurance",
        "lifecycle",
    ] {
        assert_eq!(record.practices[id].state, AdoptionState::Pending, "{id}");
    }
    assert_eq!(record.practices["setup"].state, AdoptionState::Implemented);
    assert_eq!(record.practices["setup"].reason, old_reason);
    let round_trip = AdoptionRecord::from_yaml(&record.to_yaml()?)?;
    assert_eq!(round_trip.practices.len(), 23);
    assert!(
        record.migrate_catalog(2).is_err(),
        "no silent obligation removal"
    );
    Ok(())
}

#[test]
fn policy_two_requires_explicit_schema_and_capabilities_without_claiming_execution() -> TestResult {
    use opdev_project::{Capability, CapabilityFact, CapabilityState, SafeguardPolicy};
    let dir = fixture()?;
    let path = dir.path().join(MANIFEST_PATH);
    let mut project = ProjectManifest::load(&path)?;
    project.schema = 4;
    project.assurance.profiles.clear();
    project.assurance.engineering = Some(opdev_core::EngineeringPolicy {
        version: "2".into(),
        minimumcd: Some("1".into()),
        review_reference: "synthetic approved migration".into(),
        maintenance_branches: vec![],
    });
    assert!(
        project.to_yaml().is_err(),
        "missing capabilities are not an opt-out"
    );
    project.assurance.safeguards = Some(SafeguardPolicy {
        version: 1,
        review_reference: "synthetic capability review".into(),
        capabilities: Capability::ALL
            .into_iter()
            .map(|c| {
                (
                    c,
                    CapabilityFact {
                        state: CapabilityState::Absent,
                        rationale: "isolated no-product fixture".into(),
                        authority: "contracts".into(),
                    },
                )
            })
            .collect(),
    });
    fs::write(&path, project.to_yaml()?)?;
    let output = cli(dir.path(), &["check", "--no-exec", "--format", "json"])?;
    assert!(
        !output.status.success(),
        "selection does not establish verification"
    );
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["schema"], 3);
    assert_eq!(report["catalog_version"], 4);
    assert_eq!(report["engineering"]["stage"], "local");
    assert_eq!(
        report["engineering"]["definition_sha256"],
        opdev_core::resolve_engineering_policy("2")?.definition_sha256
    );
    let rules = report["rules"].as_array().ok_or("rules")?;
    assert_eq!(rules.len(), 46);
    for id in ["OPDEV-FLOW-001", "OPDEV-RECOVERY-001"] {
        assert_eq!(
            rules.iter().find(|r| r["rule_id"] == id).ok_or("rule")?["outcome"],
            "unverified"
        );
    }
    let schema: Value = serde_json::from_str(include_str!("../../../schema/report.schema.json"))?;
    assert!(jsonschema::is_valid(&schema, &report));
    let mut parsed: opdev_engine::CheckReport = serde_json::from_value(report)?;
    parsed
        .engineering
        .as_mut()
        .ok_or("engineering")?
        .definition_sha256 = Some("0".repeat(64));
    assert!(
        opdev_engine::reaggregate(&mut parsed).is_err(),
        "wrong definition cannot qualify"
    );
    parsed
        .engineering
        .as_mut()
        .ok_or("engineering")?
        .definition_sha256 = Some(opdev_core::resolve_engineering_policy("2")?.definition_sha256);
    parsed.engineering.as_mut().ok_or("engineering")?.stage = None;
    assert!(
        opdev_engine::reaggregate(&mut parsed).is_err(),
        "stage cannot disappear"
    );
    project.schema = 3;
    assert!(
        project.to_yaml().is_err(),
        "no reinterpretation of schema 3"
    );
    project.schema = 4;
    project
        .assurance
        .engineering
        .as_mut()
        .ok_or("policy")?
        .version = "latest".into();
    assert!(project.to_yaml().is_err(), "no floating semantics");
    assert!(
        project.catalog().is_err(),
        "unknown identity cannot fall back"
    );
    Ok(())
}

#[test]
fn maintained_release_line_is_not_a_second_development_trunk() -> TestResult {
    use opdev_project::{AdoptionRecord, AdoptionWorkflow, MainOption};
    let dir = fixture()?;
    let mut project = ProjectManifest::load(&dir.path().join(MANIFEST_PATH))?;
    let mut record = AdoptionRecord::pending()?;
    record.workflow = Some(AdoptionWorkflow {
        integration_branches: vec!["main".into()],
        release_source: "support/1.x".into(),
        main_option: MainOption::AlreadyMain,
        references: vec!["reviewed branch/CI policy".into()],
    });
    assert!(
        !record.workflow_blockers(&project).is_empty(),
        "legacy policy unchanged"
    );
    project.schema = 3;
    project.assurance.profiles.clear();
    project.assurance.engineering = Some(opdev_core::EngineeringPolicy {
        version: "1".into(),
        minimumcd: Some("1".into()),
        review_reference: "synthetic developer decision".into(),
        maintenance_branches: vec![opdev_core::MaintenanceBranch {
            name: "support/1.x".into(),
            supported_version: "1.x".into(),
            authority: "existing support policy".into(),
        }],
    });
    assert!(
        record.workflow_blockers(&project).is_empty(),
        "reviewed maintained line allowed"
    );
    let report = opdev_engine::evaluate(
        dir.path(),
        &project,
        opdev_engine::CheckOptions {
            execute_checks: false,
            ..opdev_engine::CheckOptions::pre_merge()
        },
    )?;
    assert_eq!(
        report
            .rules
            .iter()
            .find(|r| r.rule_id.as_str() == "OPDEV-BRANCH-001")
            .ok_or("branch rule")?
            .outcome,
        opdev_core::Outcome::Unverified,
        "a branch declaration does not qualify its behavior"
    );
    record
        .workflow
        .as_mut()
        .ok_or("workflow")?
        .integration_branches
        .push("support/1.x".into());
    assert!(
        !record.workflow_blockers(&project).is_empty(),
        "maintenance label cannot hide a competing integration stream"
    );
    Ok(())
}

fn cli(root: &Path, args: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root)
        .args(args)
        .output()
}

fn fixture() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    assert!(
        Command::new("git")
            .args(["init", "-q", "-b", "main"])
            .arg(dir.path())
            .status()?
            .success()
    );
    assert!(
        cli(dir.path(), &["init", "--legacy-policy"])?
            .status
            .success()
    );
    Ok(dir)
}

#[test]
fn migration_preview_is_read_only_and_preserves_unrelated_policy() -> TestResult {
    let dir = fixture()?;
    let root = dir.path();
    let before = fs::read(root.join(MANIFEST_PATH))?;
    let args = [
        "upgrade",
        "--engineering-policy",
        "1",
        "--policy-review-reference",
        "synthetic developer approval",
        "--format",
        "json",
    ];
    let preview = cli(root, &args)?;
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let result: Value = serde_json::from_slice(&preview.stdout)?;
    assert_eq!(result["written"], false);
    assert_eq!(result["project_verification"], "unverified");
    assert_eq!(before, fs::read(root.join(MANIFEST_PATH))?);
    let mut old = ProjectManifest::load(&root.join(MANIFEST_PATH))?;
    let mut next = ProjectManifest::from_yaml(result["candidate_yaml"].as_str().ok_or("yaml")?)?;
    assert_eq!(next.schema, 3);
    assert_eq!(
        next.assurance
            .engineering
            .as_ref()
            .ok_or("policy")?
            .minimumcd
            .as_deref(),
        Some("1"),
        "keep retains legacy assessment intent"
    );
    next.schema = old.schema;
    next.assurance.engineering = None;
    old.assurance.profiles.retain(|p| p.name != "opdev-core");
    assert_eq!(
        old, next,
        "commands, authorities, testing, delivery and other profile pins preserved"
    );
    let mut invalid = args.to_vec();
    invalid.extend(["--apply", "anything"]);
    assert!(!cli(root, &invalid)?.status.success());
    assert_eq!(before, fs::read(root.join(MANIFEST_PATH))?);
    Ok(())
}

#[test]
fn policy_two_preview_requires_explicit_legacy_choice_and_preserves_standard_boundaries()
-> TestResult {
    let dir = fixture()?;
    let root = dir.path();
    let before = fs::read(root.join(MANIFEST_PATH))?;
    let args = [
        "upgrade",
        "--engineering-policy",
        "2",
        "--policy-review-reference",
        "fixture actual decision",
        "--format",
        "json",
    ];
    let missing = cli(root, &args)?;
    assert_eq!(missing.status.code(), Some(2));
    assert!(String::from_utf8(missing.stderr)?.contains("explicitly choose"));
    assert_eq!(fs::read(root.join(MANIFEST_PATH))?, before);
    // The shortcut must not silently discard a selected mode or boundary.
    let mut manifest = ProjectManifest::load(&root.join(MANIFEST_PATH))?;
    manifest.schema = 4;
    manifest
        .assurance
        .profiles
        .retain(|p| p.name != "opdev-core");
    manifest.assurance.engineering = Some(opdev_core::EngineeringPolicy {
        version: "2".into(),
        minimumcd: None,
        review_reference: "fixture original decision".into(),
        maintenance_branches: vec![],
    });
    manifest.assurance.safeguards = Some(serde_json::from_value(
        serde_json::json!({"version":1,"review_reference":"fixture capability decision","capabilities":{}}),
    )?);
    manifest.assurance.standards = serde_json::from_value(
        serde_json::json!([{"name":"minimumcd","version":"1","mode":"require","stages":["delivery"]}]),
    )?;
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    let current = fs::read(root.join(MANIFEST_PATH))?;
    let preserved = cli(root, &args)?;
    assert!(
        preserved.status.success(),
        "{}",
        String::from_utf8_lossy(&preserved.stderr)
    );
    let preview: Value = serde_json::from_slice(&preserved.stdout)?;
    let candidate =
        ProjectManifest::from_yaml(preview["candidate_yaml"].as_str().ok_or("candidate")?)?;
    assert_eq!(candidate.assurance.standards, manifest.assurance.standards);
    let mut removal = args.to_vec();
    removal.extend(["--minimumcd-assessment", "none"]);
    assert_eq!(cli(root, &removal)?.status.code(), Some(2));
    assert_eq!(fs::read(root.join(MANIFEST_PATH))?, current);
    Ok(())
}

#[test]
fn cli_reports_separate_assessment_without_claiming_verification() -> TestResult {
    for kind in ["library", "cli", "service"] {
        let dir = fixture()?;
        let root = dir.path();
        let legacy: Value = serde_json::from_slice(
            &cli(root, &["check", "--no-exec", "--format", "json"])?.stdout,
        )?;
        assert_eq!(legacy["schema"], 1);
        assert_eq!(legacy["catalog_version"], 2);
        assert!(legacy.get("engineering").is_none());
        assert!(
            !cli(root, &["check", "--no-exec", "--require-minimumcd"])?
                .status
                .success()
        );
        let preview: Value = serde_json::from_slice(
            &cli(
                root,
                &[
                    "upgrade",
                    "--engineering-policy",
                    "1",
                    "--minimumcd-assessment",
                    "none",
                    "--policy-review-reference",
                    "synthetic approval",
                    "--format",
                    "json",
                ],
            )?
            .stdout,
        )?;
        let mut manifest =
            ProjectManifest::from_yaml(preview["candidate_yaml"].as_str().ok_or("yaml")?)?;
        let mut value = serde_json::to_value(&manifest)?;
        value["project"]["kind"] = Value::String(kind.into());
        manifest = ProjectManifest::from_yaml(&serde_json::to_string(&value)?)?;
        fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
        let human = String::from_utf8(cli(root, &["check", "--no-exec"])?.stdout)?;
        assert!(human.contains("MinimumCD assessment: not requested; no compliance claim."));
        let report: Value = serde_json::from_slice(
            &cli(root, &["check", "--no-exec", "--format", "json"])?.stdout,
        )?;
        assert_eq!(report["schema"], 2);
        assert_eq!(report["catalog_version"], 3);
        assert!(report["engineering"]["minimumcd"].is_null());
        assert_eq!(report["rules"].as_array().ok_or("rules")?.len(), 42);
        manifest
            .assurance
            .engineering
            .as_mut()
            .ok_or("policy")?
            .minimumcd = Some("1".into());
        fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
        let output = cli(
            root,
            &[
                "check",
                "--no-exec",
                "--require-minimumcd",
                "--format",
                "json",
            ],
        )?;
        assert_eq!(output.status.code(), Some(1));
        let report: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(report["engineering"]["minimumcd"]["verdict"], "blocked");
        assert_eq!(
            report["engineering"]["minimumcd"]["requirements"]
                .as_array()
                .ok_or("requirements")?
                .len(),
            19
        );
        assert!(jsonschema::is_valid(
            &serde_json::from_str(include_str!("../../../schema/report.schema.json"))?,
            &report
        ));
    }
    Ok(())
}

#[test]
fn capability_shapes_do_not_turn_missing_execution_into_policy_compliance() -> TestResult {
    let fixtures: Value =
        serde_json::from_str(include_str!("../../../benchmarks/policy/cases.json"))?;
    let cases = fixtures["cases"].as_array().ok_or("cases")?;
    assert_eq!(cases.len(), 6);
    for case in cases {
        let dir = fixture()?;
        let path = dir.path().join(MANIFEST_PATH);
        let initial = ProjectManifest::load(&path)?;
        for version in ["1", "2", "3"] {
            let mut project = serde_json::to_value(&initial)?;
            project["schema"] = serde_json::json!(if version == "1" { 3 } else { 4 });
            project["project"]["kind"] = case["kind"].clone();
            project["assurance"]["profiles"] = serde_json::json!([]);
            project["assurance"]["engineering"] = serde_json::json!({
                "version": version, "minimumcd": "1",
                "review_reference": "synthetic paired policy fixture"
            });
            let safeguard_version = if version == "3" { 2 } else { 1 };
            let capabilities = opdev_project::Capability::for_version(safeguard_version).iter().map(|&capability| {
                let name = serde_json::to_value(capability)?;
                let state = if case["present"].as_array().ok_or("capabilities")?.contains(&name) {
                    "present"
                } else {
                    "absent"
                };
                Ok((name.as_str().ok_or("capability name")?.to_owned(), serde_json::json!({
                    "state": state, "rationale": "Explicit neutral fixture inventory, not a project-kind inference",
                    "authority": "contracts"
                })))
            }).collect::<Result<serde_json::Map<String, Value>, Box<dyn std::error::Error>>>()?;
            project["assurance"]["safeguards"] = serde_json::json!({
                "version": safeguard_version, "review_reference": "synthetic capability inventory",
                "capabilities": capabilities
            });
            let manifest = ProjectManifest::from_yaml(&serde_json::to_string(&project)?)?;
            fs::write(&path, manifest.to_yaml()?)?;
            let before = fs::read(&path)?;
            let output = cli(dir.path(), &["check", "--no-exec", "--format", "json"])?;
            assert_eq!(output.status.code(), Some(1), "{} / {version}", case["id"]);
            let report: Value = serde_json::from_slice(&output.stdout)?;
            assert_eq!(
                report["catalog_version"],
                match version {
                    "1" => 3,
                    "2" => 4,
                    _ => 5,
                }
            );
            assert_eq!(report["engineering"]["version"], version);
            assert_eq!(report["engineering"]["minimumcd"]["verdict"], "blocked");
            assert_eq!(fs::read(&path)?, before, "assessment cannot migrate policy");
            let rules = report["rules"].as_array().ok_or("rules")?;
            let absent = !case["present"]
                .as_array()
                .ok_or("capabilities")?
                .iter()
                .any(|capability| capability == "distribution" || capability == "operations");
            assert_delivery_applicability(rules, version, absent)?;
            for id in [
                "MCD-TEST-001",
                "MCD-TEST-002",
                "OPDEV-BUILD-001",
                "OPDEV-STYLE-001",
            ] {
                let rule = rules
                    .iter()
                    .find(|rule| rule["rule_id"] == id)
                    .ok_or("retained obligation")?;
                assert_ne!(
                    rule["outcome"], "passed",
                    "{id}: no execution or review was supplied"
                );
                assert_ne!(
                    rule["outcome"], "not_applicable",
                    "{id}: kind is not an opt-out"
                );
            }
            assert_eq!(
                rules
                    .iter()
                    .any(|rule| rule["rule_id"] == "OPDEV-RECOVERY-001"),
                version != "1"
            );
        }
    }
    Ok(())
}

fn assert_delivery_applicability(rules: &[Value], version: &str, absent: bool) -> TestResult {
    for id in [
        "MCD-ARTIFACT-001",
        "MCD-ARTIFACT-002",
        "MCD-ENV-001",
        "MCD-CONFIG-002",
    ] {
        let rule = rules
            .iter()
            .find(|rule| rule["rule_id"] == id)
            .ok_or("delivery rule")?;
        assert_eq!(
            rule["outcome"],
            if version != "1" && absent {
                "unverified"
            } else {
                "migration_required"
            },
            "a source-only modern project needs reviewed absence, not an invented release; legacy and applicable delivery retain migration gaps"
        );
    }
    Ok(())
}
