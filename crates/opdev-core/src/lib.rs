//! Normative domain types and the embedded `OpDev` rule catalog.

#![forbid(unsafe_code)]

mod assurance;
mod catalog;
mod evidence;
mod outcome;
mod policy;
mod resolution;
pub use policy::{
    EngineeringPolicy, MaintenanceBranch, RuleClass, catalog_for_version, rule_class,
};
pub use resolution::{
    PolicyChange, PolicyDisposition, PolicyResolution, PolicyResolutionError, ResolvedControl,
    resolve_engineering_policy,
};

pub use assurance::{
    AssuranceProfile, ProfileError, ProfileRequirement, ProfileSource, ProfileStatus,
    RequirementCoverage, embedded_profiles, resolve_profile,
};
pub use catalog::{
    CatalogError, Gate, Rule, RuleCatalog, RuleId, RuleKind, Source, VerificationMethod,
    embedded_catalog,
};
pub use evidence::{
    Evidence, ExtensionRequest, ExtensionResponse, GateVerdict, RuleResult, VerificationSource,
};
pub use outcome::{AggregateVerdict, Outcome};

/// Project-manifest schema understood by this release.
pub const PROJECT_SCHEMA_VERSION: u32 = 3;

/// Project-command extension protocol understood by this release.
pub const EXTENSION_PROTOCOL_VERSION: &str = "1.0.0";
