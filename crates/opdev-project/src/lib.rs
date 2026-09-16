//! Project contracts, repository discovery, and initialization for `OpDev`.

#![forbid(unsafe_code)]

mod adoption;
mod bootstrap;
mod discovery;
mod evidence;
mod experiment;
mod manifest;

pub use adoption::{
    ADOPTION_PATH, AdoptionCatalog, AdoptionDecision, AdoptionError, AdoptionRecord,
    AdoptionReview, AdoptionState, AdoptionWorkflow, MainOption, Practice, Requirement,
    adoption_catalog,
};
pub use bootstrap::{
    AgentFilePreview, BootstrapError, FileChange, ManagedFile, apply_agent_preview,
    preview_agent_files, reconcile_agent_files,
};
pub use discovery::{Discovery, DiscoveryError, discover};
pub use evidence::{
    ChangeEvidence, ChangeEvidenceReview, EVIDENCE_PATH, EvidenceAssertion, EvidenceBootstrap,
    EvidenceError, EvidenceLedger, EvidenceReview, ReviewDecision, staged_fingerprint,
};
pub use experiment::{ExperimentError, validate_experiment};
pub use manifest::{
    Artifact, Assurance, AuthorityKind, AuthorityRef, ChangeTests, CiConfig, CiProvider,
    CommandSpec, Context, Coverage, CoverageMode, Delivery, DeliveryMode, DeliveryStatus,
    Environment, EscapedDefectRegressions, ExtensionCheck, ExtensionStage, Extensions, FlakePolicy,
    ManifestError, Operations, Profile, Project, ProjectKind, ProjectManifest, Quality,
    QualityRisk, Recovery, RecoveryStrategy, TestStage, TestSuite, Testing,
};

/// Repository-relative location of an initialized project contract.
pub const MANIFEST_PATH: &str = ".opdev/project.yaml";
