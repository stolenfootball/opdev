use std::collections::BTreeSet;
use std::io::{self, Read};

use opdev_core::Outcome;
use roxmltree::{Document, Node, ParsingOptions};
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_BYTES: u64 = 8 * 1024 * 1024;
const MAX_NODES: u32 = 100_000;

/// Read-only observations about one report, never execution or gate qualification.
#[derive(Debug, Serialize)]
pub struct TestReportInspection {
    /// Independent inspection output schema.
    pub schema: u32,
    /// SHA-256 of the exact input bytes; not proof of origin or freshness.
    pub input_sha256: String,
    /// Outcome of the supported report observations only.
    pub outcome: Outcome,
    /// Actual case elements, not a trusted producer summary.
    pub cases: usize,
    /// Cases containing a failure, excluding cases also containing an error.
    pub failures: usize,
    /// Cases containing an error (a reported test error, not a parser failure).
    pub errors: usize,
    /// Cases containing a skip, excluding failed/error cases.
    pub skipped: usize,
    /// Recognized retry/flaky elements; not a complete attempt count.
    pub retry_signals: usize,
    /// Revision, execution identity and freshness remain unverified.
    pub qualification: Outcome,
    /// Fixed diagnostics without copied report text, names or captured output.
    pub diagnostics: Vec<String>,
}

/// Inspect bounded UTF-8 `JUnit` XML without executing commands or resolving entities.
///
/// # Errors
/// Returns an I/O error for unreadable, oversized, malformed or unsupported-root XML.
/// XML parser diagnostics deliberately do not echo potentially sensitive input.
pub fn inspect_junit(reader: impl Read) -> io::Result<TestReportInspection> {
    let mut bytes = Vec::new();
    reader.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(invalid("JUnit input exceeds the 8 MiB limit"));
    }
    let xml = std::str::from_utf8(&bytes).map_err(|_| invalid("JUnit input must be UTF-8"))?;
    let doc = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: MAX_NODES,
            entity_resolver: None,
        },
    )
    .map_err(|_| invalid("invalid JUnit XML, prohibited DTD, or XML node limit exceeded"))?;
    let root = doc.root_element();
    if root.tag_name().namespace().is_some()
        || !matches!(root.tag_name().name(), "testsuite" | "testsuites")
    {
        return Err(invalid(
            "expected an unnamespaced testsuite or testsuites root",
        ));
    }
    let mut report = TestReportInspection {
        schema: 1,
        input_sha256: format!("{:x}", Sha256::digest(&bytes)),
        outcome: Outcome::Unverified,
        cases: 0,
        failures: 0,
        errors: 0,
        skipped: 0,
        retry_signals: 0,
        qualification: Outcome::Unverified,
        diagnostics: vec![],
    };
    let mut uncertain = BTreeSet::new();
    let mut identities = BTreeSet::new();
    for node in root.descendants().filter(Node::is_element) {
        if node.ancestors().take(66).count() > 65 {
            return Err(invalid("JUnit nesting exceeds the 64-level limit"));
        }
        let name = node.tag_name().name();
        if node.tag_name().namespace().is_some() {
            uncertain.insert("Namespaced elements are not supported by this inspector.");
        }
        match name {
            "testsuites" | "testsuite" => {
                if node != root && !parent_is(node, &["testsuites", "testsuite"]) {
                    uncertain.insert("Unsupported suite nesting was observed.");
                }
                check_totals(node, &mut uncertain);
            }
            "testcase" => {
                report.cases += 1;
                // Suite name/class/name collisions are ambiguous across aggregated runs.
                let identity = (
                    node.parent()
                        .and_then(|p| p.attribute("name"))
                        .unwrap_or_default(),
                    node.attribute("classname").unwrap_or_default(),
                    node.attribute("name").unwrap_or_default(),
                );
                if !identities.insert(identity) {
                    uncertain.insert("Duplicate case identities may conceal repeated attempts.");
                }
                inspect_case(node, &mut report, &mut uncertain);
            }
            "failure" | "error" | "skipped" => {
                if !parent_is(node, &["testcase"]) {
                    uncertain.insert("A result element is outside a supported case position.");
                }
            }
            "flakyFailure" | "flakyError" | "rerunFailure" | "rerunError" => {
                report.retry_signals += 1;
                uncertain
                    .insert("Retry/flaky evidence needs review; a final success cannot erase it.");
            }
            "system-out" | "system-err" | "properties" | "property" => {}
            _ => {
                uncertain.insert("Unsupported elements prevent a complete interpretation.");
            }
        }
    }
    if report.cases == 0 {
        uncertain.insert("No test cases were observed; summary counts alone are insufficient.");
    }
    if report.skipped > 0 {
        uncertain.insert("Skipped cases require an applicability or quarantine review.");
    }
    report.outcome = if report.failures > 0 || report.errors > 0 {
        Outcome::Failed
    } else if uncertain.is_empty() {
        Outcome::Passed
    } else {
        Outcome::Unverified
    };
    report.diagnostics = uncertain.into_iter().map(str::to_owned).collect();
    report.diagnostics.push(
        "Report inspection only: execution, revision, freshness and complete retry history remain unverified. No gate or evidence ledger was updated.".into(),
    );
    Ok(report)
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn inspect_case(
    node: Node<'_, '_>,
    report: &mut TestReportInspection,
    uncertain: &mut BTreeSet<&'static str>,
) {
    if !parent_is(node, &["testsuite"]) {
        uncertain.insert("A case is outside a supported suite position.");
    }
    if node.attribute("name").unwrap_or_default().trim().is_empty() {
        uncertain.insert("A test case has no usable identity.");
    }
    if node.attributes().any(|attribute| {
        !matches!(
            attribute.name(),
            "name" | "classname" | "time" | "file" | "line" | "assertions"
        )
    }) {
        uncertain.insert("Producer-specific case attributes need review.");
    }
    let error = child_is(node, "error");
    let failure = child_is(node, "failure");
    let skipped = child_is(node, "skipped");
    if error {
        report.errors += 1;
    } else if failure {
        report.failures += 1;
    } else if skipped {
        report.skipped += 1;
    }
    if usize::from(error) + usize::from(failure) + usize::from(skipped) > 1 {
        uncertain.insert("A case contains conflicting result elements.");
    }
}

fn parent_is(node: Node<'_, '_>, names: &[&str]) -> bool {
    node.parent()
        .is_some_and(|p| names.contains(&p.tag_name().name()))
}

fn child_is(node: Node<'_, '_>, name: &str) -> bool {
    node.children().any(|child| child.has_tag_name(name))
}

fn check_totals(node: Node<'_, '_>, uncertain: &mut BTreeSet<&'static str>) {
    if node.attributes().any(|attribute| {
        !matches!(
            attribute.name(),
            "name"
                | "id"
                | "package"
                | "timestamp"
                | "hostname"
                | "time"
                | "file"
                | "tests"
                | "failures"
                | "errors"
                | "skipped"
                | "disabled"
                | "assertions"
        )
    }) {
        uncertain.insert("Producer-specific suite attributes need review.");
    }
    for (attribute, result_element) in [
        ("tests", None),
        ("failures", Some("failure")),
        ("errors", Some("error")),
        ("skipped", Some("skipped")),
    ] {
        if let Some(value) = node.attribute(attribute) {
            let observed = node
                .descendants()
                .filter(|n| n.has_tag_name("testcase"))
                .filter(|n| result_element.is_none_or(|result| child_is(*n, result)))
                .count();
            if value.parse::<usize>() != Ok(observed) {
                uncertain.insert("Declared summary counts differ from observed case elements.");
            }
        }
    }
    if node.attribute("disabled").is_some_and(|value| value != "0") {
        uncertain.insert("Disabled cases require review.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_actual_cases_and_never_qualifies() -> io::Result<()> {
        let report =
            inspect_junit(b"<testsuite tests='1'><testcase name='ok'/></testsuite>".as_slice())?;
        assert_eq!(report.outcome, Outcome::Passed);
        assert_eq!(report.cases, 1);
        assert_eq!(report.qualification, Outcome::Unverified);
        assert_eq!(report.input_sha256.len(), 64);
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../schema/test-report-inspection.schema.json"
        ))?;
        assert!(jsonschema::is_valid(
            &schema,
            &serde_json::to_value(&report)?
        ));
        Ok(())
    }

    #[test]
    fn detects_failed_error_skipped_and_retry_cases() -> io::Result<()> {
        let report = inspect_junit(
            br"<testsuites><testsuite tests='4'>
            <testcase name='a'><failure>private content</failure></testcase>
            <testcase name='b'><error/></testcase><testcase name='c'><skipped/></testcase>
            <testcase name='d'><flakyFailure/></testcase></testsuite></testsuites>"
                .as_slice(),
        )?;
        assert_eq!(report.outcome, Outcome::Failed);
        assert_eq!(
            (
                report.cases,
                report.failures,
                report.errors,
                report.skipped,
                report.retry_signals
            ),
            (4, 1, 1, 1, 1)
        );
        assert!(!serde_json::to_string(&report)?.contains("private content"));
        Ok(())
    }

    #[test]
    fn ambiguous_and_empty_reports_cannot_pass() -> io::Result<()> {
        for xml in [
            "<testsuite tests='20'/>",
            "<testsuites/>",
            "<testsuite><testcase name='a'><skipped/></testcase></testsuite>",
            "<testsuite><testcase name='a'><rerunError/></testcase></testsuite>",
            "<testsuite><testcase name='a'/><testcase name='a'/></testsuite>",
            "<testsuite><testcase/></testsuite>",
            "<testsuite><testcase name='a' status='notrun'/></testsuite>",
            "<testsuite status='failed'><testcase name='a'/></testsuite>",
            "<testsuite tests='2'><testcase name='a'/></testsuite>",
            "<testsuite><testcase name='a'><unknown/></testcase></testsuite>",
            "<testsuite><wrapper><testcase name='a'/></wrapper></testsuite>",
            "<testsuite><failure/><testcase name='a'/></testsuite>",
        ] {
            assert_eq!(
                inspect_junit(xml.as_bytes())?.outcome,
                Outcome::Unverified,
                "{xml}"
            );
        }
        Ok(())
    }

    #[test]
    fn malformed_oversized_and_entity_inputs_are_errors() {
        for xml in [
            "",
            "<testsuite>",
            "<other/>",
            "<!DOCTYPE testsuite [<!ENTITY private 'value'>]><testsuite>&private;</testsuite>",
        ] {
            assert!(inspect_junit(xml.as_bytes()).is_err());
        }
        assert!(inspect_junit(io::repeat(b' ').take(MAX_BYTES + 1)).is_err());
        let many_nodes = format!(
            "<testsuite>{}</testsuite>",
            "<testcase/>".repeat(MAX_NODES as usize)
        );
        assert!(inspect_junit(many_nodes.as_bytes()).is_err());
        let deep = format!("{}{}", "<testsuite>".repeat(65), "</testsuite>".repeat(65));
        assert!(inspect_junit(deep.as_bytes()).is_err());
        assert!(inspect_junit([0xff].as_slice()).is_err());
    }
}
