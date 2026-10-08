//! Capability facts and exact-change safeguards reuse the ordinary acceptance review.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Capabilities, not project-kind or filename heuristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Retained user or application data and formats.
    PersistentData,
    /// A contract consumed outside its implementation boundary.
    PublicContract,
    /// Installed or otherwise distributed software.
    Distribution,
    /// Untrusted input, authorization, secrets or privileged execution.
    SecurityBoundary,
    /// Software requiring operation and incident diagnosis.
    Operations,
    /// Human-facing interfaces, including command-line interfaces.
    UserInterface,
    /// Outcomes dependent on measured data/model or user effectiveness.
    Effectiveness,
}

/// Explicitly unresolved is distinct from reviewed absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    /// Actual capability is present.
    Present,
    /// Actual capability is absent, with reviewed rationale and authority.
    Absent,
    /// Applicability has not been resolved.
    Unknown,
}

/// Durable reviewed fact; attribution alone is not authenticated consent.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityFact {
    /// Actual applicability.
    pub state: CapabilityState,
    /// Capability-based explanation, never absence of a tool configuration.
    pub rationale: String,
    /// Existing source/design/decision authority for the fact.
    pub authority: String,
}

/// Explicit selection preserves previous policy semantics until migration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SafeguardPolicy {
    /// Supported mapping version, currently 1.
    pub version: u32,
    /// Actual policy decision reference, not inferred consent.
    pub review_reference: String,
    /// Missing entries remain unknown; this is not an opt-out list.
    pub capabilities: BTreeMap<Capability, CapabilityFact>,
}

/// Change impact on a known-present capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Impact {
    /// Requires mapped safeguard objectives.
    Affected,
    /// Requires an exact-change reviewed explanation; not a capability waiver.
    Unaffected,
}

/// Source-bound through the containing acceptance review.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityImpact {
    /// Actual change impact.
    pub impact: Impact,
    /// Why this change does or does not affect the capability.
    pub rationale: String,
}

/// A bounded objective mapped to existing acceptance condition IDs, not test tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SafeguardObjective {
    /// Retained records/formats remain readable or deliberately migrated.
    RetainedDataCompatibility,
    /// Interrupted/failed migrations preserve or recover valid data.
    DataRecovery,
    /// Representative existing callers retain source/wire/semantic contracts.
    ConsumerCompatibility,
    /// An intentional change has a usable transition, or compatibility needs no migration.
    ConsumerTransition,
    /// Supported installation/update inputs and resulting artifacts behave correctly.
    InstallationUpdate,
    /// Failed installation/update is recoverable for the affected distribution.
    DistributionRecovery,
    /// Applicable input/auth/secret/privilege negative cases and trust boundaries.
    SecurityControls,
    /// Diagnosable health/failure and appropriate operational recovery.
    OperationalRecovery,
    /// Meaningful interface-specific accessibility target and verification.
    Accessibility,
    /// Measured utility with scope/data limitations separate from correctness.
    EffectivenessEvaluation,
}

impl Capability {
    /// All version-1 capability questions. Unselected entries are unresolved.
    pub const ALL: [Self; 7] = [
        Self::PersistentData,
        Self::PublicContract,
        Self::Distribution,
        Self::SecurityBoundary,
        Self::Operations,
        Self::UserInterface,
        Self::Effectiveness,
    ];

    /// Required objectives for an affected capability.
    #[must_use]
    pub const fn objectives(self) -> &'static [SafeguardObjective] {
        use SafeguardObjective as O;
        match self {
            Self::PersistentData => &[O::RetainedDataCompatibility, O::DataRecovery],
            Self::PublicContract => &[O::ConsumerCompatibility, O::ConsumerTransition],
            Self::Distribution => &[O::InstallationUpdate, O::DistributionRecovery],
            Self::SecurityBoundary => &[O::SecurityControls],
            Self::Operations => &[O::OperationalRecovery],
            Self::UserInterface => &[O::Accessibility],
            Self::Effectiveness => &[O::EffectivenessEvaluation],
        }
    }
}

/// Additional coverage inside the existing exact-change acceptance payload.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SafeguardReview {
    /// Version of these impact/objective mappings.
    pub version: u32,
    /// Exactly the present capabilities; absence cannot be asserted here.
    pub impacts: BTreeMap<Capability, CapabilityImpact>,
    /// Links to inventoried conditions that carry the ordinary test/review mappings.
    pub objectives: BTreeMap<SafeguardObjective, Vec<String>>,
}
