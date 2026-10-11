//! Bounded additive policy definitions. Resolving data never executes or fetches it.
use std::collections::{BTreeMap, BTreeSet, HashSet};

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Capability, CapabilityState, SafeguardPolicy, TestStage};

mod source;
pub use source::{PolicySnapshot, inspect_file, load_selected, portable_path};

/// Maximum encoded size of one current definition.
pub const MAX_PACK_BYTES: usize = 256 * 1024;

/// Explicit project selection; content identity and values are decision material.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PolicySelection {
    /// Portable policy ID, also its filename stem.
    pub id: String,
    /// Exact owner-assigned version, not a moving alias.
    pub version: String,
    /// SHA-256 of the canonical typed definition, independent of checkout newlines.
    pub definition_sha256: String,
    /// Every declared parameter must be explicitly supplied and actually used.
    #[serde(default, deserialize_with = "unique_map")]
    pub parameters: BTreeMap<String, ParameterValue>,
}

/// Bounded scalar, not code, interpolation, paths or executable arguments.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ParameterValue {
    /// Explicit boolean selection.
    Boolean(bool),
    /// Integer constrained by the corresponding definition.
    Integer(i64),
    /// Exact member of a declared finite vocabulary.
    Choice(String),
}

/// Permitted parameter types; no coercion and no implicit defaults.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParameterDefinition {
    /// True or false; affects only the declared additional applicability predicate.
    Boolean,
    /// Bounded integers, used for exact applicability comparisons, not inferred metrics.
    Integer {
        /// Inclusive lower bound.
        minimum: i64,
        /// Inclusive upper bound.
        maximum: i64,
    },
    /// A nonempty finite set of exact string values.
    Choice {
        /// Distinct bounded values.
        values: Vec<String>,
    },
}

/// A deliberately small applicability vocabulary, not a general expression language.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Applicability {
    /// Applies without a capability condition.
    #[default]
    Always,
    /// Conjunction of present capabilities and exact selected parameter values.
    All {
        /// Missing or unknown facts do not establish applicability or absence.
        #[serde(default)]
        capabilities: Vec<Capability>,
        /// Names must exist in the pack's parameter schema.
        #[serde(default, deserialize_with = "unique_map")]
        parameters: BTreeMap<String, ParameterValue>,
    },
}

/// Whether a current automated path is mandatory for this added control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationRequirement {
    /// At least one mapped current automated condition, criterion or extension.
    Automated,
    /// Current automated verification or attributed durable manual observation.
    Observation,
}

/// One additive duty; it cannot target or redefine an existing core rule.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationControl {
    /// Stable namespaced ORG-PACK-001 ID.
    pub id: String,
    /// Concrete outcome to verify, not an executable command.
    pub statement: String,
    /// Existing owner/standard reference, not fetched during loading.
    pub source: String,
    /// Explicit nonempty supported verification boundaries.
    pub stages: Vec<TestStage>,
    /// Reviewed capability/parameter conditions.
    pub applicability: Applicability,
    /// Required evidence route; review adequacy alone is not execution.
    pub verification: VerificationRequirement,
}

/// Strict current definition, not test code, a work backlog or execution history.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyPack {
    /// Definition format, currently 1.
    pub schema: u32,
    /// Portable ID matching the project selection and filename.
    pub id: String,
    /// Exact owner-assigned revision.
    pub version: String,
    /// Short human-readable policy name.
    pub title: String,
    /// Original policy authority; attribution does not authenticate consent.
    pub source: String,
    /// Typed bounded applicability parameters, all required at selection.
    #[serde(default, deserialize_with = "unique_map")]
    pub parameters: BTreeMap<String, ParameterDefinition>,
    /// One to 32 distinct additional duties.
    pub controls: Vec<OrganizationControl>,
}

/// Pure resolution. This object grants no approval and establishes no verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedPolicyPack {
    /// Original explicit project selection.
    pub selection: PolicySelection,
    /// Validated current definition.
    pub definition: PolicyPack,
    /// Semantic identity of both the definition and the actual selected values.
    pub resolution_sha256: String,
}

/// Structural predicate result; a reviewed absence still needs current acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplicabilityResult {
    /// The declared conjunction is satisfied.
    Applicable,
    /// At least one reviewed fact/explicit selector is false.
    NotApplicable,
    /// No false fact establishes absence, and at least one capability is unknown.
    Unknown,
}

fn text(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}

/// Portable exact filename stem; no traversal, case aliases or Windows device names.
#[must_use]
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.as_bytes()[0].is_ascii_lowercase()
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        && crate::clean_adoption::valid_path(&format!(".opdev/policies/{id}.json"))
}

fn version(value: &str) -> bool {
    text(value, 64)
        && !value.eq_ignore_ascii_case("latest")
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}

impl PolicySelection {
    /// Validate shape without looking up files or invoking tools.
    /// # Errors
    /// Unsupported IDs, pins, revisions or excessive selection data are rejected.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            valid_id(&self.id) && version(&self.version),
            "Policy selection needs a portable exact ID and version; no moving alias."
        );
        ensure!(
            self.definition_sha256.len() == 64
                && self
                    .definition_sha256
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
            "Policy selection needs a lowercase SHA-256 definition pin."
        );
        ensure!(
            self.parameters.len() <= 16 && self.parameters.keys().all(|k| valid_id(k)),
            "Policy selection has invalid or excessive parameters."
        );
        Ok(())
    }
}

impl ParameterDefinition {
    fn validate(&self) -> Result<()> {
        let valid = match self {
            Self::Boolean => true,
            Self::Integer { minimum, maximum } => {
                (-1_000_000..=1_000_000).contains(minimum)
                    && (-1_000_000..=1_000_000).contains(maximum)
                    && minimum <= maximum
            }
            Self::Choice { values } => {
                !values.is_empty()
                    && values.len() <= 32
                    && values.iter().all(|v| text(v, 128))
                    && values.iter().collect::<BTreeSet<_>>().len() == values.len()
            }
        };
        ensure!(valid, "Policy parameter needs valid bounded values.");
        Ok(())
    }

    fn accepts(&self, value: &ParameterValue) -> bool {
        match (self, value) {
            (Self::Boolean, ParameterValue::Boolean(_)) => true,
            (Self::Integer { minimum, maximum }, ParameterValue::Integer(value)) => {
                (minimum..=maximum).contains(&value)
            }
            (Self::Choice { values }, ParameterValue::Choice(value)) => values.contains(value),
            _ => false,
        }
    }
}

impl PolicyPack {
    /// Parse bounded strict data; never evaluate code or fetch external references.
    /// # Errors
    /// Malformed, duplicate, unsupported or semantically inconsistent data fails closed.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() <= MAX_PACK_BYTES,
            "Policy definition exceeds 256 KiB; no partial definition loaded."
        );
        let pack: Self = serde_json::from_slice(bytes)
            .map_err(|_| anyhow::anyhow!("Invalid policy JSON shape or duplicate fields; inspect the definition locally. No content executed."))?;
        pack.validate()?;
        Ok(pack)
    }

    /// Validate the closed definition language, including all parameter uses.
    /// # Errors
    /// Unknown bounds, IDs, references or ambiguous applicability are rejected.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema == 1 && valid_id(&self.id) && version(&self.version),
            "Unsupported policy definition identity."
        );
        ensure!(
            text(&self.title, 256) && text(&self.source, 2048),
            "Policy title and source must be bounded nonempty references."
        );
        ensure!(
            self.parameters.len() <= 16 && self.parameters.keys().all(|k| valid_id(k)),
            "Policy parameters need distinct portable names and a bound of 16."
        );
        for definition in self.parameters.values() {
            definition.validate()?;
        }
        ensure!(
            !self.controls.is_empty() && self.controls.len() <= 32,
            "Policy definition needs 1 to 32 controls; empty is not conformance."
        );
        let mut ids = HashSet::new();
        let mut used_parameters = BTreeSet::new();
        for control in &self.controls {
            let prefix = format!("ORG-{}-", self.id.to_ascii_uppercase());
            ensure!(
                control
                    .id
                    .strip_prefix(&prefix)
                    .is_some_and(|n| n.len() == 3 && n.bytes().all(|c| c.is_ascii_digit()))
                    && ids.insert(&control.id),
                "Policy controls need distinct namespaced ORG-PACK-001 IDs; core IDs cannot be replaced."
            );
            ensure!(
                text(&control.statement, 4096) && text(&control.source, 2048),
                "Policy control statement/source is empty, unsafe or oversized."
            );
            ensure!(
                !control.stages.is_empty()
                    && control.stages.len() <= TestStage::ALL.len()
                    && control.stages.iter().collect::<HashSet<_>>().len() == control.stages.len(),
                "Policy control needs explicit unique supported stages."
            );
            if let Applicability::All {
                capabilities,
                parameters,
            } = &control.applicability
            {
                ensure!(
                    !capabilities.is_empty() || !parameters.is_empty(),
                    "An empty applicability conjunction is ambiguous; use always."
                );
                ensure!(
                    capabilities.len() <= Capability::ALL.len()
                        && capabilities.iter().collect::<BTreeSet<_>>().len() == capabilities.len(),
                    "Capability predicates must be bounded and unique."
                );
                for (name, value) in parameters {
                    ensure!(
                        self.parameters.get(name).is_some_and(|p| p.accepts(value)),
                        "Predicate references an unknown or invalid typed parameter."
                    );
                    used_parameters.insert(name);
                }
            }
        }
        ensure!(
            self.parameters.keys().all(|p| used_parameters.contains(p)),
            "Every declared parameter must be used by a supported predicate; inert metadata is not verification."
        );
        Ok(())
    }

    /// Exact canonical definition identity, stable across whitespace and object-key order.
    /// # Errors
    /// Invalid definitions are never assigned a usable identity.
    pub fn definition_sha256(&self) -> Result<String> {
        self.validate()?;
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json_canonicalizer::to_vec(self)?)
        ))
    }

    /// Bind a current definition to the explicit project selection without executing it.
    /// # Errors
    /// Changed pins, missing/extra/wrong-typed values and identity mismatch fail closed.
    pub fn resolve(&self, selection: &PolicySelection) -> Result<ResolvedPolicyPack> {
        selection.validate()?;
        ensure!(
            self.id == selection.id
                && self.version == selection.version
                && self.definition_sha256()? == selection.definition_sha256,
            "Policy definition changed or does not match its selected identity; review the current proposal. No checks ran."
        );
        ensure!(
            selection.parameters.len() == self.parameters.len()
                && self.parameters.iter().all(|(name, def)| selection
                    .parameters
                    .get(name)
                    .is_some_and(|v| def.accepts(v))),
            "Select every declared parameter with its exact supported type and bounds; no values were defaulted."
        );
        Ok(ResolvedPolicyPack {
            selection: selection.clone(),
            definition: self.clone(),
            resolution_sha256: format!(
                "{:x}",
                Sha256::digest(serde_json_canonicalizer::to_vec(&(1_u32, self, selection))?)
            ),
        })
    }
}

impl Applicability {
    /// Evaluate only declared facts; no project-kind, filename or absent-tool inference.
    #[must_use]
    pub fn evaluate(
        &self,
        safeguards: Option<&SafeguardPolicy>,
        values: &BTreeMap<String, ParameterValue>,
    ) -> ApplicabilityResult {
        let Self::All {
            capabilities,
            parameters,
        } = self
        else {
            return ApplicabilityResult::Applicable;
        };
        let mut unknown = false;
        for capability in capabilities {
            match safeguards
                .and_then(|s| s.capabilities.get(capability))
                .map(|f| f.state)
            {
                Some(CapabilityState::Present) => {}
                Some(CapabilityState::Absent) => return ApplicabilityResult::NotApplicable,
                Some(CapabilityState::Unknown) | None => unknown = true,
            }
        }
        for (key, expected) in parameters {
            match values.get(key) {
                Some(actual) if actual == expected => {}
                Some(_) => return ApplicabilityResult::NotApplicable,
                None => unknown = true,
            }
        }
        if unknown {
            ApplicabilityResult::Unknown
        } else {
            ApplicabilityResult::Applicable
        }
    }
}

fn unique_map<'de, D, V>(deserializer: D) -> std::result::Result<BTreeMap<String, V>, D::Error>
where
    D: serde::Deserializer<'de>,
    V: Deserialize<'de>,
{
    struct Visitor<V>(std::marker::PhantomData<V>);
    impl<'de, V: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<V> {
        type Value = BTreeMap<String, V>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a bounded map of distinct parameter names")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut access: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<String, V>()? {
                if result.len() >= 16 || result.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom(
                        "duplicate or excessive policy parameters",
                    ));
                }
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Visitor(std::marker::PhantomData))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CapabilityFact;

    fn pack() -> PolicyPack {
        PolicyPack {
            schema: 1,
            id: "example".into(),
            version: "1".into(),
            title: "Example policy".into(),
            source: "synthetic owner decision; not authenticated consent".into(),
            parameters: BTreeMap::from([(
                "tier".into(),
                ParameterDefinition::Integer {
                    minimum: 1,
                    maximum: 3,
                },
            )]),
            controls: vec![OrganizationControl {
                id: "ORG-EXAMPLE-001".into(),
                statement: "Retained records recover after interrupted migration".into(),
                source: "existing architecture authority".into(),
                stages: vec![TestStage::PreMerge],
                applicability: Applicability::All {
                    capabilities: vec![Capability::PersistentData],
                    parameters: BTreeMap::from([("tier".into(), ParameterValue::Integer(2))]),
                },
                verification: VerificationRequirement::Automated,
            }],
        }
    }

    fn selection(pack: &PolicyPack) -> Result<PolicySelection> {
        Ok(PolicySelection {
            id: pack.id.clone(),
            version: pack.version.clone(),
            definition_sha256: pack.definition_sha256()?,
            parameters: BTreeMap::from([("tier".into(), ParameterValue::Integer(2))]),
        })
    }

    #[test]
    fn canonical_identity_binds_meaning_not_checkout_whitespace() -> Result<()> {
        let pack = pack();
        let selected = selection(&pack)?;
        let compact = PolicyPack::parse(&serde_json::to_vec(&pack)?)?;
        let windows = serde_json::to_string_pretty(&pack)?.replace('\n', "\r\n");
        let windows = PolicyPack::parse(windows.as_bytes())?;
        assert_eq!(compact.definition_sha256()?, windows.definition_sha256()?);
        let original = pack.resolve(&selected)?;
        let mut changed = selected.clone();
        changed
            .parameters
            .insert("tier".into(), ParameterValue::Integer(3));
        assert_ne!(
            original.resolution_sha256,
            pack.resolve(&changed)?.resolution_sha256
        );
        let mut changed = pack.clone();
        changed.controls[0].statement = "Different obligation".into();
        assert!(changed.resolve(&selected).is_err());
        let mut changed = selected;
        changed.version = "2".into();
        assert!(pack.resolve(&changed).is_err());
        Ok(())
    }

    #[test]
    fn parameters_are_explicit_typed_bounded_and_used() -> Result<()> {
        let pack = pack();
        for value in [
            ParameterValue::Integer(0),
            ParameterValue::Integer(4),
            ParameterValue::Choice("2".into()),
            ParameterValue::Boolean(true),
        ] {
            let mut selected = selection(&pack)?;
            selected.parameters.insert("tier".into(), value);
            assert!(pack.resolve(&selected).is_err());
        }
        let mut selected = selection(&pack)?;
        selected.parameters.clear();
        assert!(pack.resolve(&selected).is_err());
        selected
            .parameters
            .insert("unknown".into(), ParameterValue::Integer(2));
        assert!(pack.resolve(&selected).is_err());
        let mut invalid = pack.clone();
        invalid
            .parameters
            .insert("unused".into(), ParameterDefinition::Boolean);
        assert!(invalid.validate().is_err());
        for definition in [
            ParameterDefinition::Integer {
                minimum: 3,
                maximum: 1,
            },
            ParameterDefinition::Integer {
                minimum: 0,
                maximum: i64::MAX,
            },
            ParameterDefinition::Choice { values: vec![] },
            ParameterDefinition::Choice {
                values: vec!["same".into(), "same".into()],
            },
        ] {
            assert!(definition.validate().is_err());
        }
        assert!(ParameterDefinition::Boolean.accepts(&ParameterValue::Boolean(false)));
        assert!(
            ParameterDefinition::Choice {
                values: vec!["private".into()]
            }
            .accepts(&ParameterValue::Choice("private".into()))
        );
        Ok(())
    }

    #[test]
    fn unknown_facts_cannot_be_inferred_from_absent_configuration() -> Result<()> {
        let pack = pack();
        let selected = selection(&pack)?;
        let predicate = &pack.controls[0].applicability;
        assert_eq!(
            predicate.evaluate(None, &selected.parameters),
            ApplicabilityResult::Unknown
        );
        for (state, expected) in [
            (CapabilityState::Present, ApplicabilityResult::Applicable),
            (CapabilityState::Absent, ApplicabilityResult::NotApplicable),
            (CapabilityState::Unknown, ApplicabilityResult::Unknown),
        ] {
            let safeguards = SafeguardPolicy {
                version: 1,
                review_reference: "fixture".into(),
                capabilities: BTreeMap::from([(
                    Capability::PersistentData,
                    CapabilityFact {
                        state,
                        rationale: "fixture fact".into(),
                        authority: "contracts".into(),
                    },
                )]),
            };
            assert_eq!(
                predicate.evaluate(Some(&safeguards), &selected.parameters),
                expected
            );
        }
        assert_eq!(
            predicate.evaluate(None, &BTreeMap::new()),
            ApplicabilityResult::Unknown
        );
        // A reviewed explicit different selector makes this conjunction false; it
        // does not verify another control or waive any baseline obligation.
        let values = BTreeMap::from([("tier".into(), ParameterValue::Integer(3))]);
        assert_eq!(
            predicate.evaluate(None, &values),
            ApplicabilityResult::NotApplicable
        );
        Ok(())
    }

    #[test]
    fn parser_rejects_duplicate_unknown_and_unsafe_definition_shapes() -> Result<()> {
        let pack = pack();
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../schema/policy-pack.schema.json"))?;
        assert!(jsonschema::is_valid(&schema, &serde_json::to_value(&pack)?));
        let encoded = serde_json::to_string(&pack)?;
        let duplicate = encoded.replace("\"schema\":1", "\"schema\":1,\"schema\":1");
        assert!(PolicyPack::parse(duplicate.as_bytes()).is_err());
        let duplicate = encoded.replace("\"tier\":{\"kind\":\"integer\",\"minimum\":1,\"maximum\":3}",
            "\"tier\":{\"kind\":\"integer\",\"minimum\":1,\"maximum\":3},\"tier\":{\"kind\":\"boolean\"}");
        assert_ne!(duplicate, encoded);
        assert!(PolicyPack::parse(duplicate.as_bytes()).is_err());
        let mut unknown = serde_json::to_value(&pack)?;
        unknown["execute"] = serde_json::json!("DO_NOT_ECHO_PRIVATE_CONTENT");
        assert!(!jsonschema::is_valid(&schema, &unknown));
        let Err(error) = PolicyPack::parse(&serde_json::to_vec(&unknown)?) else {
            anyhow::bail!("unknown executable field was accepted");
        };
        assert!(!error.to_string().contains("DO_NOT_ECHO_PRIVATE_CONTENT"));
        assert!(PolicyPack::parse(&vec![b' '; MAX_PACK_BYTES + 1]).is_err());
        for id in ["../outside", "Example", "con", "nul", "example/subdir", ""] {
            assert!(!valid_id(id));
        }
        let mut invalid = pack.clone();
        invalid.controls[0].id = "OPDEV-TEST-002".into();
        assert!(invalid.validate().is_err());
        invalid = pack.clone();
        invalid.controls.push(invalid.controls[0].clone());
        assert!(invalid.validate().is_err());
        invalid = pack.clone();
        invalid.controls.clear();
        assert!(invalid.validate().is_err());
        invalid = pack;
        invalid.controls[0].stages.clear();
        assert!(invalid.validate().is_err());
        Ok(())
    }
}
