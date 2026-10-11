//! Pure, exact standard selection. No execution, network lookup or permission grant.
use opdev_core::{AssuranceProfile, ProfileStatus};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::TestStage;

/// How a selected additional standard affects the actual verification boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StandardMode {
    /// Inspect guidance; never produce a conformance verdict.
    Guidance,
    /// Evaluate independently without blocking the engineering baseline.
    Assess,
    /// Add a blocking obligation at explicitly selected stages.
    Require,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimumcd() -> StandardSelection {
        StandardSelection {
            name: "minimumcd".into(),
            version: "1".into(),
            mode: StandardMode::Assess,
            stages: vec![TestStage::PreMerge, TestStage::PostMerge],
            level: None,
        }
    }

    #[test]
    fn exact_selection_rejects_ambiguous_pins_and_boundaries() -> anyhow::Result<()> {
        let selection = minimumcd();
        assert!(selection.resolve()?.complete_mapping);
        for (name, version, level) in [
            ("minimumcd", "latest", None),
            ("minimumcd", "1", Some("invented")),
            ("invented", "1", None),
            ("opdev-core", "1", None),
        ] {
            let mut invalid = selection.clone();
            invalid.name = name.into();
            invalid.version = version.into();
            invalid.level = level.map(str::to_owned);
            assert!(invalid.resolve().is_err());
        }
        let mut invalid = selection.clone();
        invalid.stages.clear();
        assert!(invalid.resolve().is_err());
        invalid.mode = StandardMode::Guidance;
        assert!(invalid.resolve().is_ok());
        invalid.stages = vec![TestStage::PreMerge];
        assert!(invalid.resolve().is_err());
        invalid.mode = StandardMode::Require;
        invalid.stages.push(TestStage::PreMerge);
        assert!(invalid.resolve().is_err());
        Ok(())
    }

    #[test]
    fn identity_is_order_independent_but_binds_mode_and_actual_stages() -> anyhow::Result<()> {
        let mut selection = minimumcd();
        let original = selection.resolve()?.definition_sha256;
        selection.stages.reverse();
        assert_eq!(selection.resolve()?.definition_sha256, original);
        selection.mode = StandardMode::Require;
        assert_ne!(selection.resolve()?.definition_sha256, original);
        selection.mode = StandardMode::Assess;
        selection.stages.pop();
        assert_ne!(selection.resolve()?.definition_sha256, original);
        Ok(())
    }

    #[test]
    fn derived_and_format_profiles_never_claim_complete_conformance() -> anyhow::Result<()> {
        for (name, version) in [
            ("nist-ssdf-derived", "1.1"),
            ("openssf-osps-baseline-derived", "2026.02.19"),
            ("slsa-build-provenance", "1.2"),
            ("cyclonedx-sbom", "1.5"),
        ] {
            let mut selection = minimumcd();
            selection.name = name.into();
            selection.version = version.into();
            assert!(!selection.resolve()?.complete_mapping);
        }
        Ok(())
    }
}

/// Project-owned selection under the existing engineering-policy decision authority.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StandardSelection {
    /// Exact bundled mapping identity.
    pub name: String,
    /// Exact mapping version, never latest.
    pub version: String,
    /// Guidance, independent assessment or additional obligation.
    pub mode: StandardMode,
    /// Exact lifecycle stages; guidance has no assessment stage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<TestStage>,
    /// Optional exact level understood by this mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
}

/// Read-only resolution, not evidence or authenticated consent.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedStandard {
    /// Actual project selection.
    pub selection: StandardSelection,
    /// Exact embedded mapping and its limitations.
    pub profile: AssuranceProfile,
    /// Semantic identity including selection and mapping, not an execution digest.
    pub definition_sha256: String,
    /// Whether this mapping can establish its complete stated assessment scope.
    pub complete_mapping: bool,
}

impl StandardSelection {
    /// Resolve bounded inputs without running project commands or fetching a standard.
    ///
    /// # Errors
    /// Rejects unknown pins/levels, baseline-as-standard and ambiguous stage selections.
    pub fn resolve(&self) -> anyhow::Result<ResolvedStandard> {
        let profile =
            opdev_core::resolve_profile(&self.name, &self.version, self.level.as_deref())?;
        anyhow::ensure!(
            profile.status != ProfileStatus::Normative,
            "The engineering baseline is mandatory, not a selectable external standard."
        );
        let stages: std::collections::BTreeSet<_> = self
            .stages
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<_, _>>()?;
        anyhow::ensure!(
            stages.len() == self.stages.len() && stages.len() <= TestStage::ALL.len(),
            "Standard stages must be unique supported boundaries."
        );
        anyhow::ensure!(
            if self.mode == StandardMode::Guidance {
                stages.is_empty()
            } else {
                !stages.is_empty()
            },
            "Guidance has no assessment stages; assess and require need explicit nonempty stages."
        );
        let definition_sha256 = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(
                1_u32,
                &self.name,
                &self.version,
                self.mode,
                stages,
                &self.level,
                &profile,
            ))?)
        );
        let complete_mapping = profile.status == ProfileStatus::Assessment
            && !profile.requirements.is_empty()
            && profile.requirements.iter().all(|r| {
                r.coverage == opdev_core::RequirementCoverage::Full && !r.mapped_rules.is_empty()
            });
        Ok(ResolvedStandard {
            selection: self.clone(),
            profile,
            definition_sha256,
            complete_mapping,
        })
    }
}
