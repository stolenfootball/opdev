//! `OpDev` command-line entry point.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use opdev_ci::{Capability, TemplateContext, adapter_for, infer_gitlab_image, write_new};
use opdev_core::{
    AggregateVerdict, EXTENSION_PROTOCOL_VERSION, Gate, Outcome, PROJECT_SCHEMA_VERSION, RuleId,
    VerificationMethod, embedded_catalog, embedded_profiles, resolve_profile,
};
use opdev_engine::{CheckOptions, CheckReport, evaluate, plan_checks, reaggregate};
use opdev_project::{
    CiProvider, EVIDENCE_PATH, EvidenceBootstrap, FileChange, MANIFEST_PATH, ProjectManifest,
    discover, reconcile_agent_files, staged_fingerprint, validate_experiment,
};
use opdev_release::{
    EvidenceRequest, PackageFormat, PackageInput, PackageRequest, generate_evidence,
    package_release,
};
use opdev_remote::{RemoteCapability, audit};
use semver::{Version, VersionReq};
use serde::Deserialize;

mod adoption;
mod ci_run;
mod delegation;
mod doctor;
mod documentation;
mod evidence_bundle;
mod evidence_prepare;
mod execution_reuse;
mod inspection;
mod layout;
mod local_state;
mod migration;
mod state_io;
mod test_execution;
mod test_report;
mod upgrade;
mod views;
mod workflow;
mod workflow_records;

#[derive(Debug, Parser)]
#[command(name = "opdev", version, about = "Evidence-driven software delivery")]
struct Cli {
    /// Opt in to experimental compact report and evidence views for this invocation.
    #[arg(long, global = true)]
    experimental_compact: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Initialize or reconcile `OpDev` in a software project.
    Init(InitArgs),
    /// Evaluate project requirements.
    Check(CheckArgs),
    /// Inspect a saved check report without executing project commands.
    Report(ReportArgs),
    /// Inspect structured test evidence without qualifying a change.
    TestReport(test_report::TestReportArgs),
    /// Run one canonical suite and emit tool-neutral JSON execution evidence.
    TestExecution(test_execution::TestExecutionArgs),
    /// Explain missing, contradictory, or unverified capabilities.
    Doctor(doctor::DoctorArgs),
    /// Resolve document ownership without writing files or approving changes.
    Documentation(documentation::DocumentationArgs),
    /// Inspect a proposed .opdev directory standard without migration or qualification.
    Layout(layout::LayoutArgs),
    /// Resolve or inspect private continuation state; saved state never qualifies a change.
    State(local_state::StateArgs),
    /// Inspect resumable references or explicitly append an attributed workflow event.
    Workflow(workflow::WorkflowArgs),
    /// Validate a bounded worker assignment/result without dispatch or qualification.
    Delegation(delegation::DelegationArgs),
    /// Generate or inspect a first-class CI configuration.
    Ci(CiArgs),
    /// Preview an upgrade, or apply an explicitly reviewed guidance plan.
    Upgrade(upgrade::UpgradeArgs),
    /// Show CLI and protocol versions.
    Version,
    /// Inspect the embedded normative rule catalog.
    Rules(RulesArgs),
    /// Inspect exact-version assurance profiles bundled with this release.
    Profiles(ProfilesArgs),
    /// Package already-built artifacts and generate deterministic release evidence.
    Release(ReleaseArgs),
    /// Prepare repository-state binding for reviewable project evidence.
    Evidence(EvidenceArgs),
    /// Verify compatibility between an installed agent plugin and this CLI.
    Plugin(PluginArgs),
    /// Validate an experiment plan without running tests or qualifying delivery.
    Experiment(ExperimentArgs),
    /// Assess project-specific practices and verify adoption completion.
    Adoption(adoption::AdoptionArgs),
}

#[derive(Debug, Args)]
struct ExperimentArgs {
    #[command(subcommand)]
    command: ExperimentCommand,
}

#[derive(Debug, Subcommand)]
enum ExperimentCommand {
    /// Validate an active YAML/JSON record and its project test-suite references.
    Validate {
        /// Record path, relative to the current working directory.
        path: PathBuf,
        /// Directory inside the initialized Git repository.
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

#[derive(Debug, Args)]
struct PluginArgs {
    #[command(subcommand)]
    command: PluginCommand,
}

#[derive(Debug, Subcommand)]
enum PluginCommand {
    /// Verify a packaged plugin compatibility contract against this CLI.
    Verify(PluginVerifyArgs),
}

#[derive(Debug, Args)]
struct PluginVerifyArgs {
    /// Packaged `opdev-compatibility.json` contract.
    #[arg(long)]
    contract: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PluginCompatibility {
    schema: u32,
    plugin: PluginIdentity,
    requires: PluginRequirements,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PluginIdentity {
    name: String,
    version: Version,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PluginRequirements {
    cli: VersionReq,
}

#[derive(Debug, Args)]
struct InitArgs {
    /// Explicit reviewed strict layout for new projects; existing layouts need migration.
    #[arg(long, requires = "engineering_policy", value_parser = clap::value_parser!(u32).range(1..=1))]
    layout_version: Option<u32>,
    /// Explicit reviewed engineering policy for a new project (currently 1).
    #[arg(long, requires_all = ["policy_review_reference", "minimumcd_assessment"])]
    engineering_policy: Option<String>,
    /// Actual developer choice/delegation reference, not authenticated by the CLI.
    #[arg(long, requires = "engineering_policy")]
    policy_review_reference: Option<String>,
    /// Explicit choice of separate `MinimumCD` assessment, not an engineering waiver.
    #[arg(long, requires = "engineering_policy", value_parser = ["1", "none"])]
    minimumcd_assessment: Option<String>,
    /// Directory inside the Git repository to initialize.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Print the discovered contract without writing files.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Args)]
#[allow(clippy::struct_excessive_bools)] // Independent CLI switches, constrained by clap.
struct CheckArgs {
    /// Exact provider archive selection for the explicitly selected semantic-review policy.
    #[arg(long, requires = "review_acceptance_sha256", conflicts_with = "plan")]
    review_locator: Option<PathBuf>,
    /// Independently selected acceptance identity; archive contents cannot choose it.
    #[arg(long, requires = "review_locator")]
    review_acceptance_sha256: Option<String>,
    /// Retain this attempt outside Git, including unfinished or failed execution; never reuse it as qualification.
    #[arg(long, conflicts_with = "plan")]
    retain_state: bool,
    /// Also require the selected `MinimumCD` assessment; does not authorize release.
    #[arg(long, conflicts_with = "plan")]
    require_minimumcd: bool,
    /// Directory inside the initialized Git repository.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Preview selected invocations without execution, remote access or qualification.
    #[arg(long, conflicts_with_all = ["remote", "no_exec", "report", "reuse_ci_policy"])]
    plan: bool,
    /// Evaluate CI-specific requirements.
    #[arg(long)]
    ci: bool,
    /// Require the delivery gate and execute declared delivery suites; use before publication.
    #[arg(long, requires = "ci", conflicts_with = "no_exec")]
    delivery: bool,
    /// Include read-only remote provider auditing.
    #[arg(long)]
    remote: bool,
    /// Validate and aggregate without executing project commands.
    #[arg(long)]
    no_exec: bool,
    /// Reuse provider-authenticated results from this CI run under a reviewed policy (network access).
    #[arg(long, requires_all = ["ci", "execution_environment"], conflicts_with = "delivery")]
    reuse_ci_policy: Option<PathBuf>,
    /// Nonsecret actual environment identity supplied by the reviewed producer configuration.
    #[arg(long, requires = "reuse_ci_policy")]
    execution_environment: Option<String>,
    /// Verify integrated-trunk suites/extensions using the integration gate, not release readiness.
    #[arg(long, requires = "ci", conflicts_with = "delivery")]
    post_merge: bool,
    /// Report presentation.
    #[arg(long, value_enum, default_value_t = CheckFormat::Human)]
    format: CheckFormat,
    /// Save the full JSON report to a new file (required for summary output).
    #[arg(long)]
    report: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CheckFormat {
    Human,
    Json,
    Summary,
}

#[derive(Debug, Args)]
struct ReportArgs {
    #[command(subcommand)]
    command: ReportCommand,
}

#[derive(Debug, Subcommand)]
enum ReportCommand {
    /// Print a compact JSON view; exit 1 if any recorded gate is blocked.
    Summarize { path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Args)]
struct CiArgs {
    #[command(subcommand)]
    command: CiCommand,
}

#[derive(Debug, Subcommand)]
enum CiCommand {
    /// Run a reviewed canonical producer once and emit its versioned execution record.
    Execute(execution_reuse::ExecuteArgs),
    /// Verify an explicitly selected remote CI run, not whole-project qualification.
    VerifyRun(ci_run::VerifyRunArgs),
    /// Render a pinned baseline configuration.
    Generate(CiGenerateArgs),
    /// Inspect the initialized project's local CI configuration.
    Inspect(CiInspectArgs),
}

#[derive(Debug, Args)]
struct CiGenerateArgs {
    /// Repository directory.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Provider to render.
    #[arg(long, value_enum)]
    provider: ProviderArg,
    /// Exact `OpDev` release used by CI.
    #[arg(long, default_value = env!("CARGO_PKG_VERSION"))]
    opdev_version: String,
    /// GitLab job image; inferred from exact project toolchain metadata when omitted.
    #[arg(long)]
    image: Option<String>,
    /// Create the provider file; otherwise print it to standard output.
    #[arg(long)]
    write: bool,
}

#[derive(Debug, Args)]
struct CiInspectArgs {
    /// Directory inside the initialized Git repository.
    #[arg(long, default_value = ".")]
    root: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ProviderArg {
    Github,
    Gitlab,
}

impl From<ProviderArg> for CiProvider {
    fn from(value: ProviderArg) -> Self {
        match value {
            ProviderArg::Github => Self::Github,
            ProviderArg::Gitlab => Self::Gitlab,
        }
    }
}

#[derive(Debug, Args)]
struct RulesArgs {
    /// Exact catalog to inspect; legacy 2 or explicitly migrated engineering 3.
    #[arg(long, default_value_t = 2)]
    catalog_version: u32,
    /// Show one stable rule ID instead of listing the catalog.
    #[arg(long)]
    id: Option<RuleId>,
}

#[derive(Debug, Args)]
struct ProfilesArgs {
    /// Show one stable profile name instead of listing bundled profiles.
    #[arg(long, requires = "version")]
    name: Option<String>,
    /// Exact profile version; floating aliases such as `latest` are rejected.
    #[arg(long, requires = "name")]
    version: Option<String>,
}

#[derive(Debug, Args)]
struct ReleaseArgs {
    #[command(subcommand)]
    command: ReleaseCommand,
}

#[derive(Debug, Subcommand)]
enum ReleaseCommand {
    /// Create a deterministic archive from explicit source-to-destination mappings.
    Package(ReleasePackageArgs),
    /// Bind artifacts, source, and an existing `CycloneDX` SBOM by digest.
    Evidence(ReleaseEvidenceArgs),
}

#[derive(Debug, Args)]
struct ReleasePackageArgs {
    /// Archive encoding.
    #[arg(long, value_enum)]
    format: PackageFormatArg,
    /// Regular file or directory mapping in the form SOURCE=DESTINATION; repeat as needed.
    #[arg(long, value_name = "SOURCE=DESTINATION")]
    entry: Vec<String>,
    /// Executable file mapping in the form SOURCE=DESTINATION; repeat as needed.
    #[arg(long, value_name = "SOURCE=DESTINATION")]
    executable_entry: Vec<String>,
    /// New archive path. Existing output is never replaced.
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum PackageFormatArg {
    TarGz,
    Zip,
}

impl From<PackageFormatArg> for PackageFormat {
    fn from(value: PackageFormatArg) -> Self {
        match value {
            PackageFormatArg::TarGz => Self::TarGz,
            PackageFormatArg::Zip => Self::Zip,
        }
    }
}

#[derive(Debug, Args)]
struct ReleaseEvidenceArgs {
    /// Release artifact; repeat for every artifact in this evidence bundle.
    #[arg(long, required = true)]
    artifact: Vec<PathBuf>,
    /// Existing `CycloneDX` JSON SBOM.
    #[arg(long)]
    sbom: PathBuf,
    /// Exact `CycloneDX` specification version required from the SBOM.
    #[arg(long, default_value = "1.5")]
    sbom_version: String,
    /// Stable source repository URI.
    #[arg(long)]
    source_uri: String,
    /// Exact source revision, normally the Git commit SHA.
    #[arg(long)]
    source_revision: String,
    /// Builder identity URI supplied by the build platform.
    #[arg(long)]
    builder_id: String,
    /// Honest scope or assurance limitation for this artifact and SBOM association.
    #[arg(long)]
    assurance_limitation: Option<String>,
    /// Directory in which evidence files are created without replacement.
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct EvidenceArgs {
    #[command(subcommand)]
    command: EvidenceCommand,
}

#[derive(Debug, Subcommand)]
enum EvidenceCommand {
    /// Local export, exact provider retrieval and inspection; never qualification or upload.
    Bundle(evidence_bundle::BundleArgs),
    /// Prepare, preview or apply a reviewed acceptance update to an existing ledger.
    Prepare(evidence_prepare::PrepareArgs),
    /// Print the staged index fingerprint used by change evidence.
    Fingerprint(EvidenceFingerprintArgs),
    /// Print the current acceptance review's subject digest without approving it.
    AcceptanceDigest(EvidenceFingerprintArgs),
    /// Generate or apply a fail-closed review questionnaire for a new evidence ledger.
    Bootstrap(EvidenceBootstrapArgs),
    /// Show durable and exact-current evidence without modifying the ledger.
    Show(EvidenceShowArgs),
}

#[derive(Debug, Args)]
struct EvidenceShowArgs {
    /// Directory inside the initialized Git repository.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Select only the exact staged fingerprint; historical queries are unsupported.
    #[arg(long, required = true)]
    current: bool,
    /// Limit both evidence scopes to one known rule.
    #[arg(long)]
    rule: Option<RuleId>,
}

#[derive(Debug, Args)]
struct EvidenceFingerprintArgs {
    /// Directory inside the initialized Git repository.
    #[arg(long, default_value = ".")]
    root: PathBuf,
}

#[derive(Debug, Args)]
struct EvidenceBootstrapArgs {
    /// Create a reviewed candidate outside source instead of the legacy ledger.
    #[arg(long, requires_all = ["answers", "write"])]
    output: Option<PathBuf>,
    /// Directory inside the initialized Git repository.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Completed questionnaire to validate and expand; omit to print a new questionnaire.
    #[arg(long)]
    answers: Option<PathBuf>,
    /// Create `.opdev/evidence.yaml`; otherwise print the candidate ledger.
    #[arg(long, requires = "answers")]
    write: bool,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let compact_requested = match &cli.command {
        Command::Check(args) => args.format == CheckFormat::Summary,
        Command::Report(_) => true,
        Command::Evidence(args) => matches!(args.command, EvidenceCommand::Show(_)),
        _ => false,
    };
    if compact_requested && !cli.experimental_compact {
        bail!(
            "compact views are experimental; explicitly opt in with --experimental-compact, \
             or use human/full JSON check output and the full evidence ledger"
        );
    }
    match cli.command {
        Command::Version => {
            let catalog = embedded_catalog().context("could not load the embedded rule catalog")?;
            println!("opdev {}", env!("CARGO_PKG_VERSION"));
            println!("project schema {PROJECT_SCHEMA_VERSION}");
            println!("rule catalog {}", catalog.catalog_version);
            println!("extension protocol {EXTENSION_PROTOCOL_VERSION}");
            Ok(ExitCode::SUCCESS)
        }
        Command::Rules(args) => show_rules(args).map(|()| ExitCode::SUCCESS),
        Command::TestReport(args) => test_report::run(&args),
        Command::State(args) => local_state::run(&args),
        Command::TestExecution(args) => test_execution::run(&args),
        Command::Profiles(args) => show_profiles(args).map(|()| ExitCode::SUCCESS),
        Command::Release(args) => release_command(&args).map(|()| ExitCode::SUCCESS),
        Command::Evidence(args) => evidence_command(&args).map(|()| ExitCode::SUCCESS),
        Command::Plugin(args) => plugin_command(&args),
        Command::Experiment(args) => experiment_command(&args).map(|()| ExitCode::SUCCESS),
        Command::Adoption(args) => adoption::run(&args),
        Command::Init(args) => initialize(&args).map(|()| ExitCode::SUCCESS),
        Command::Check(args) => check_project(&args),
        Command::Report(args) => match args.command {
            ReportCommand::Summarize { path } => views::summarize_file(&path),
        },
        Command::Doctor(args) => doctor::run(&args),
        Command::Documentation(args) => documentation::run(&args),
        Command::Layout(args) => layout::run(&args),
        Command::Workflow(args) => workflow::run(&args),
        Command::Delegation(args) => delegation::run(&args),
        Command::Ci(args) => ci_command(&args),
        Command::Upgrade(args) => upgrade::run(&args),
    }
}

fn experiment_command(args: &ExperimentArgs) -> Result<()> {
    match &args.command {
        ExperimentCommand::Validate { path, root } => {
            let (_, manifest) = load_project(root)?;
            let yaml = std::fs::read_to_string(path)
                .with_context(|| format!("could not read experiment record {}", path.display()))?;
            let today = i64::try_from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs()
                    / 86_400,
            )?;
            let id = validate_experiment(&yaml, &manifest, today)?;
            println!(
                "passed: experiment `{id}` record validation only; behavior, isolation, effectiveness, and delivery remain unverified by this command"
            );
        }
    }
    Ok(())
}

fn plugin_command(args: &PluginArgs) -> Result<ExitCode> {
    match &args.command {
        PluginCommand::Verify(args) => verify_plugin_compatibility(args),
    }
}

fn verify_plugin_compatibility(args: &PluginVerifyArgs) -> Result<ExitCode> {
    let source = std::fs::read_to_string(&args.contract).with_context(|| {
        format!(
            "could not read plugin compatibility contract {}",
            args.contract.display()
        )
    })?;
    let contract: PluginCompatibility = serde_json::from_str(&source).with_context(|| {
        format!(
            "invalid plugin compatibility contract {}",
            args.contract.display()
        )
    })?;
    if contract.schema != 1 {
        bail!(
            "unsupported plugin compatibility schema {}; this CLI supports schema 1",
            contract.schema
        );
    }
    if contract.plugin.name != "opdev" {
        bail!(
            "compatibility contract names plugin `{}`; expected `opdev`",
            contract.plugin.name
        );
    }

    let cli_version = Version::parse(env!("CARGO_PKG_VERSION"))
        .context("the compiled CLI version is not valid SemVer")?;
    if contract.requires.cli.matches(&cli_version) {
        println!(
            "opdev plugin {} is compatible with CLI {} ({})",
            contract.plugin.version, cli_version, contract.requires.cli
        );
        Ok(ExitCode::SUCCESS)
    } else {
        eprintln!(
            "opdev plugin {} requires CLI {}; installed CLI is {}",
            contract.plugin.version, contract.requires.cli, cli_version
        );
        Ok(ExitCode::from(1))
    }
}

fn evidence_command(args: &EvidenceArgs) -> Result<()> {
    match &args.command {
        EvidenceCommand::Bundle(args) => evidence_bundle::run(args)?,
        EvidenceCommand::Prepare(args) => evidence_prepare::run(args)?,
        EvidenceCommand::Fingerprint(args) => {
            let (root, _) = load_project(&args.root)?;
            println!("{}", staged_fingerprint(&root)?);
        }
        EvidenceCommand::Bootstrap(args) => bootstrap_evidence(args)?,
        EvidenceCommand::AcceptanceDigest(args) => {
            let (root, manifest) = load_project(&args.root)?;
            let fingerprint = staged_fingerprint(&root)?;
            let catalog = manifest.catalog()?;
            let ledger = opdev_project::EvidenceLedger::load_optional(&root, &catalog)?
                .context("acceptance digest needs a schema-2 evidence ledger")?;
            let change = ledger
                .matching_change(&fingerprint)
                .context("no evidence for the current staged change")?;
            let acceptance = change
                .acceptance
                .as_ref()
                .context("current change has no acceptance section")?;
            println!("{}", acceptance.digest(&fingerprint, &change.work)?);
        }
        EvidenceCommand::Show(args) => {
            let (root, _) = load_project(&args.root)?;
            views::show_evidence(&root, args.rule.as_ref())?;
        }
    }
    Ok(())
}

fn bootstrap_evidence(args: &EvidenceBootstrapArgs) -> Result<()> {
    let (root, manifest) = load_project(&args.root)?;
    let ledger_path = root.join(EVIDENCE_PATH);
    if ledger_path.exists() {
        bail!(
            "{} already exists; bootstrap is intentionally create-new only",
            ledger_path.display()
        );
    }

    let fingerprint = staged_fingerprint(&root)?;
    // Questionnaire discovery is not qualification. Suppress the required external
    // input only in this no-execution private assessment; bind answers to the real
    // unchanged project fingerprint and never present this intermediate report.
    let mut preparation_manifest = manifest.clone();
    preparation_manifest.assurance.review_storage = None;
    let mut report = evaluate(
        &root,
        &preparation_manifest,
        CheckOptions {
            execute_checks: false,
            ..CheckOptions::pre_merge()
        },
    )
    .context("project evaluation failed")?;
    apply_local_ci(&root, &manifest, &mut report)?;
    let catalog = manifest
        .catalog()
        .context("could not load the selected rule catalog")?;
    let (project_rules, change_rules) = evidence_candidates(&catalog, &report);

    if let Some(path) = &args.answers {
        let answers = EvidenceBootstrap::load(path)?;
        answers.validate_candidates(&project_rules, &change_rules, &fingerprint)?;
        let ledger = answers.to_ledger(&catalog)?;
        if args.write {
            let path = if let Some(output) = &args.output {
                let path = evidence_bundle::export_destination(&root, output)?;
                local_state::write_new(&path, ledger.to_yaml()?.as_bytes())?;
                path
            } else {
                if manifest.assurance.review_storage.is_some() {
                    bail!(
                        "External review policy needs --output outside source; no legacy ledger created"
                    );
                }
                ledger.write_new(&root, &catalog)?
            };
            println!("created {}", path.display());
        } else {
            print!("{}", ledger.to_yaml()?);
            eprintln!(
                "candidate only: review this expansion, then repeat with --write to create the ledger"
            );
        }
    } else {
        let questionnaire = EvidenceBootstrap::new(
            fingerprint,
            project_rules.iter().cloned(),
            change_rules.iter().cloned(),
        );
        print!("{}", questionnaire.to_review_yaml(&catalog)?);
        eprintln!(
            "all decisions are review_required; add shared evidence and explicitly review each outcome"
        );
    }
    Ok(())
}

fn evidence_candidates(
    catalog: &opdev_core::RuleCatalog,
    report: &CheckReport,
) -> (Vec<String>, Vec<String>) {
    let mut project = Vec::new();
    let mut change = Vec::new();
    for rule in &catalog.rules {
        // These change rules require typed acceptance evidence, not boolean assertions.
        if matches!(rule.id.as_str(), "OPDEV-TEST-002" | "OPDEV-TEST-003") {
            continue;
        }
        let unverified = report
            .rules
            .iter()
            .find(|result| result.rule_id == rule.id)
            .is_some_and(|result| result.outcome == Outcome::Unverified);
        let reviewable = rule.verification.contains(&VerificationMethod::Evidence)
            || rule.verification.contains(&VerificationMethod::Agent);
        if !unverified || !reviewable {
            continue;
        }
        let destination = if change_scoped_rule(rule.id.as_str()) {
            &mut change
        } else {
            &mut project
        };
        destination.push(rule.id.as_str().to_owned());
    }
    (project, change)
}

fn change_scoped_rule(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "OPDEV-WORK-001"
            | "OPDEV-DESIGN-001"
            | "MCD-TRUNK-002"
            | "MCD-TRUNK-003"
            | "MCD-FLOW-001"
            | "MCD-COMPAT-001"
            | "OPDEV-TEST-002"
            | "OPDEV-TEST-003"
            | "OPDEV-TEST-004"
            | "OPDEV-AI-001"
            | "OPDEV-LEARN-001"
            | "OPDEV-BRANCH-001"
    )
}

fn release_command(args: &ReleaseArgs) -> Result<()> {
    match &args.command {
        ReleaseCommand::Package(args) => {
            let mut inputs = args
                .entry
                .iter()
                .map(|entry| parse_package_input(entry, false))
                .collect::<Result<Vec<_>>>()?;
            inputs.extend(
                args.executable_entry
                    .iter()
                    .map(|entry| parse_package_input(entry, true))
                    .collect::<Result<Vec<_>>>()?,
            );
            package_release(&PackageRequest {
                inputs,
                format: args.format.into(),
                output: args.output.clone(),
            })?;
            println!("created {}", args.output.display());
        }
        ReleaseCommand::Evidence(args) => {
            let outputs = generate_evidence(&EvidenceRequest {
                artifacts: args.artifact.clone(),
                sbom: args.sbom.clone(),
                sbom_version: args.sbom_version.clone(),
                source_uri: args.source_uri.clone(),
                source_revision: args.source_revision.clone(),
                builder_id: args.builder_id.clone(),
                assurance_limitation: args.assurance_limitation.clone(),
                output_directory: args.output.clone(),
            })?;
            println!("created {}", outputs.checksums.display());
            println!("created {}", outputs.manifest.display());
            println!("created {}", outputs.provenance.display());
            println!(
                "Evidence is digest-bound but does not by itself establish signing, trusted-builder provenance, or a SLSA Build level."
            );
        }
    }
    Ok(())
}

fn parse_package_input(value: &str, executable: bool) -> Result<PackageInput> {
    let (source, destination) = value
        .split_once('=')
        .filter(|(source, destination)| !source.is_empty() && !destination.is_empty())
        .ok_or_else(|| anyhow::anyhow!("package entry `{value}` must use SOURCE=DESTINATION"))?;
    Ok(PackageInput {
        source: PathBuf::from(source),
        destination: destination.to_owned(),
        executable,
    })
}

fn show_profiles(args: ProfilesArgs) -> Result<()> {
    if let (Some(name), Some(version)) = (args.name, args.version) {
        let profile = resolve_profile(&name, &version, None)?;
        println!("{}", serde_json::to_string_pretty(&profile)?);
    } else {
        for profile in embedded_profiles()? {
            println!(
                "{}@{}\t{:?}\t{}",
                profile.name, profile.version, profile.status, profile.title
            );
        }
    }
    Ok(())
}

fn initialize(args: &InitArgs) -> Result<()> {
    let mut discovery = discover(&args.root).context("could not inspect the repository")?;
    let manifest_path = discovery.root.join(MANIFEST_PATH);
    let adoption = opdev_project::AdoptionRecord::load(&discovery.root)?;
    select_initial_policy(args, &mut discovery, manifest_path.exists())?;
    let catalog_version = opdev_project::project_adoption_catalog(&discovery.manifest);
    if !manifest_path.exists()
        && adoption
            .as_ref()
            .is_some_and(|record| record.catalog_version != catalog_version)
    {
        bail!(
            "partial initialization uses a different adoption inventory; retry with the original reviewed policy choices. Existing decisions were preserved."
        );
    }

    if manifest_path.exists() {
        if args.dry_run {
            print!("{}", discovery.manifest.to_yaml()?);
            return Ok(());
        }
        report_agent_changes(&reconcile_agent_files(&discovery.root)?);
        println!("OpDev files already exist at {}", manifest_path.display());
        println!(
            "{}",
            if adoption.is_some() {
                "Adoption decisions preserved; use opdev adoption status/check to resume or verify completion."
            } else {
                "Legacy project: no adoption assessment was added. Use opdev adoption start explicitly."
            }
        );
        return Ok(());
    }

    for evidence in &discovery.evidence {
        eprintln!("discovered: {evidence}");
    }
    for warning in &discovery.warnings {
        eprintln!("warning: {warning}");
    }

    if args.dry_run {
        print!("{}", discovery.manifest.to_yaml()?);
        let catalog = opdev_project::adoption_catalog_version(catalog_version)?;
        eprintln!(
            "Adoption catalog {}: {} practices require explicit review; discovery does not mark them implemented.",
            catalog.version,
            catalog.practices.len()
        );
        for practice in catalog.practices {
            eprintln!("assess {}: {}", practice.id, practice.title);
        }
    } else {
        // Create unresolved state first so interruption after writing the manifest
        // is distinguishable from a legacy project. Existing decisions are untouched.
        if adoption.is_none() {
            opdev_project::AdoptionRecord::pending_for_catalog(catalog_version)?
                .write_new(&discovery.root)?;
        }
        discovery.manifest.write_new(&manifest_path)?;
        report_agent_changes(&reconcile_agent_files(&discovery.root)?);
        println!("Created OpDev files at {}", manifest_path.display());
        println!(
            "Adoption is incomplete. Review project choices and .opdev/adoption.yaml, implement the approved plan, then run opdev adoption check."
        );
    }
    Ok(())
}

fn select_initial_policy(
    args: &InitArgs,
    discovery: &mut opdev_project::Discovery,
    exists: bool,
) -> Result<()> {
    let Some(version) = &args.engineering_policy else {
        return Ok(());
    };
    let policy = opdev_core::EngineeringPolicy {
        version: version.clone(),
        minimumcd: args
            .minimumcd_assessment
            .as_ref()
            .filter(|v| v.as_str() != "none")
            .cloned(),
        review_reference: args
            .policy_review_reference
            .clone()
            .context("policy decision reference required")?,
        maintenance_branches: vec![],
    };
    if exists {
        if discovery.manifest.assurance.engineering.as_ref() != Some(&policy) {
            bail!(
                "existing project policy preserved; use the read-only upgrade policy preview, not init, to propose a migration"
            );
        }
        if args.layout_version.is_some_and(|version| {
            discovery
                .manifest
                .layout
                .as_ref()
                .is_none_or(|layout| layout.version != version)
        }) {
            bail!(
                "existing layout preserved; init cannot migrate project files. Preview a coordinated layout migration first"
            );
        }
    } else {
        discovery.manifest.schema = 3;
        discovery
            .manifest
            .assurance
            .profiles
            .retain(|p| p.name != "opdev-core");
        discovery.manifest.assurance.engineering = Some(policy);
        discovery.manifest.layout =
            args.layout_version
                .map(|version| opdev_project::LayoutPolicy {
                    version,
                    review_reference: args.policy_review_reference.clone().unwrap_or_default(),
                });
        discovery.manifest.to_yaml()?;
        if let Some(record) = opdev_project::AdoptionRecord::load(&discovery.root)?
            && record.catalog_version != 2
        {
            bail!(
                "partial initialization has an older adoption inventory; preserve it and complete the original initialization before an explicit policy/catalog migration"
            );
        }
    }
    Ok(())
}

fn report_agent_changes(changes: &[opdev_project::ManagedFile]) {
    for file in changes {
        let action = match file.change {
            FileChange::Created => "created",
            FileChange::Updated => "updated",
            FileChange::Unchanged => "unchanged",
        };
        println!("{action} {}", file.path.display());
    }
}

fn ci_command(args: &CiArgs) -> Result<ExitCode> {
    match &args.command {
        CiCommand::Execute(args) => execution_reuse::execute(args),
        CiCommand::Generate(args) => generate_ci(args).map(|()| ExitCode::SUCCESS),
        CiCommand::Inspect(args) => inspect_ci(args).map(|()| ExitCode::SUCCESS),
        CiCommand::VerifyRun(args) => ci_run::run(args),
    }
}

fn generate_ci(args: &CiGenerateArgs) -> Result<()> {
    let discovery = discover(&args.root).context("could not inspect the repository")?;
    let provider: CiProvider = args.provider.into();
    let adapter = adapter_for(provider)?;
    let job_image = match (provider, &args.image) {
        (CiProvider::Gitlab, Some(image)) => Some(image.clone()),
        (CiProvider::Gitlab, None) => Some(infer_gitlab_image(&discovery.root)?),
        (CiProvider::Github, Some(_)) => {
            bail!("`--image` is supported only when generating GitLab CI")
        }
        _ => None,
    };
    let context = TemplateContext {
        opdev_version: args.opdev_version.clone(),
        trunk: discovery.manifest.project.trunk,
        job_image,
    };
    eprintln!(
        "This is an integration baseline, not release qualification. Review inherited CI settings and wire `opdev check --ci --delivery` (on a capable CLI) as a required predecessor of publication in the same release path."
    );
    if let Some(image) = &context.job_image {
        eprintln!(
            "Proposed OpDev job image: {image}; published GNU binaries require glibc >= 2.39 and this template targets x86_64. Preserve product images. The OpDev job disables inherited caches; any opt-in compiled cache must separate OS/libc, architecture and toolchain. Review before applying."
        );
    }
    if args.write {
        let path = write_new(adapter, &discovery.root, &context)?;
        println!("created {}", path.display());
    } else {
        print!("{}", adapter.render(&context)?);
    }
    Ok(())
}

fn inspect_ci(args: &CiInspectArgs) -> Result<()> {
    let (root, manifest) = load_project(&args.root)?;
    let adapter = adapter_for(manifest.project.ci.provider)?;
    let inspection = adapter.inspect(&root)?;
    print_capability("configuration", &inspection.configuration);
    print_capability("pre_merge", &inspection.pre_merge);
    print_capability("post_merge", &inspection.post_merge);
    print_capability("integrity", &inspection.integrity);
    Ok(())
}

fn print_capability(name: &str, capability: &Capability) {
    println!("{name}: {:?}", capability.outcome);
    if let Some(diagnostic) = &capability.diagnostic {
        println!("  {diagnostic}");
    }
}

fn selected_review(
    review_locator: Option<&PathBuf>,
    review_acceptance_sha256: Option<&str>,
    root: &Path,
    manifest: &ProjectManifest,
    stage: opdev_project::TestStage,
) -> Result<Option<opdev_engine::ValidatedReview>> {
    review_locator.map(|path| -> Result<_> {
        let acceptance = review_acceptance_sha256.context("Acceptance identity required")?;
        anyhow::ensure!(acceptance.len() == 64 && acceptance.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "Acceptance identity must be a lowercase SHA-256 digest; no provider request made");
        let bytes = local_state::read(&std::path::absolute(path)?)?.context("Semantic review locator is missing")?;
        let locator: opdev_remote::ArchiveLocator = serde_json::from_slice(&bytes)
            .map_err(|_| anyhow::anyhow!("Semantic review locator is malformed; no private content echoed"))?;
        locator.validate().map_err(anyhow::Error::msg)?;
        let selected = manifest.assurance.review_storage.as_ref().context("External semantic review policy was not selected; no provider request made")?;
        if locator.provider != selected.provider || locator.repository_id != selected.repository_id {
            bail!("Semantic review locator is outside the selected storage policy; no provider request made");
        }
        let observed = opdev_remote::retrieve_archive(&locator).map_err(anyhow::Error::msg)?;
        opdev_engine::ValidatedReview::from_archive(root, manifest, stage,
            acceptance, &observed).map_err(anyhow::Error::msg)
    }).transpose()
}

fn check_project(args: &CheckArgs) -> Result<ExitCode> {
    if args.format == CheckFormat::Summary && args.report.is_none() {
        bail!("--format summary requires --report PATH to retain the full evaluation");
    }
    if let Some(path) = &args.report
        && path.symlink_metadata().is_ok()
    {
        bail!(
            "report output {} already exists; choose a new path",
            path.display()
        );
    }
    let (root, manifest) = load_project(&args.root)?;
    if args.require_minimumcd
        && manifest
            .assurance
            .engineering
            .as_ref()
            .and_then(|p| p.minimumcd.as_ref())
            .is_none()
    {
        bail!(
            "MinimumCD assessment was not selected. Review an explicit engineering-policy migration before requiring its result; no checks ran."
        );
    }
    let mut options = if args.ci {
        CheckOptions::pre_merge()
    } else {
        CheckOptions::local()
    };
    let remote_revision = args.remote.then(|| clean_remote_revision(&root)).flatten();
    if args.delivery {
        options.test_stage = opdev_project::TestStage::Delivery;
        options.extension_stage = opdev_project::ExtensionStage::Deliver;
    }
    options.execute_checks = !args.no_exec;
    if args.post_merge {
        options.test_stage = opdev_project::TestStage::PostMerge;
        options.extension_stage = opdev_project::ExtensionStage::PostMerge;
    }
    if args.plan {
        let plan = plan_checks(&root, &manifest, options);
        if args.format == CheckFormat::Json {
            println!("{}", serde_json::to_string_pretty(&plan)?);
        } else {
            println!(
                "Execution plan: {:?} suites; {:?} extensions",
                plan.test_stage, plan.extension_stage
            );
            println!(
                "Qualification: unverified. No commands ran. Arguments may contain private project values."
            );
            for command in plan.commands {
                println!(
                    "{} ({:?}, blocking={}): {}\n  directory: {}\n  timeout: {}s",
                    command.id,
                    command.kind,
                    command.blocking,
                    serde_json::to_string(&command.argv)?,
                    command.working_directory,
                    command.timeout_seconds
                );
            }
        }
        return Ok(ExitCode::SUCCESS);
    }
    let review = selected_review(
        args.review_locator.as_ref(),
        args.review_acceptance_sha256.as_deref(),
        &root,
        &manifest,
        options.test_stage,
    )?;
    let retained = args
        .retain_state
        .then(|| local_state::Attempt::start(&root, &manifest, options))
        .transpose()?;
    let mut report = if let Some(policy) = &args.reuse_ci_policy {
        execution_reuse::evaluate(
            &root,
            &manifest,
            options,
            policy,
            args.execution_environment
                .as_deref()
                .context("execution environment is required")?,
            review.as_ref(),
        )?
    } else if let Some(review) = &review {
        opdev_engine::evaluate_with_review(&root, &manifest, options, review, None)
            .context("review-backed project evaluation failed")?
    } else {
        evaluate(&root, &manifest, options).context("project evaluation failed")?
    };
    if args.ci {
        apply_local_ci(&root, &manifest, &mut report)?;
    }
    if args.remote {
        apply_remote_audit(&root, &manifest, &mut report, remote_revision.as_deref())?;
    }
    if let Some(attempt) = retained {
        attempt.finish(&root, &manifest, &report)?;
    }
    present_check(args, &report)?;
    Ok(check_exit(args, &report))
}

fn present_check(args: &CheckArgs, report: &CheckReport) -> Result<()> {
    let source = args
        .report
        .as_ref()
        .map(|path| views::save_report(report, path))
        .transpose()?;
    match args.format {
        CheckFormat::Human => print_human_report(report),
        CheckFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        CheckFormat::Summary => {
            views::print_summary(report, source.context("summary requires a saved report")?)?;
        }
    }
    Ok(())
}

fn check_exit(args: &CheckArgs, report: &CheckReport) -> ExitCode {
    let gate = if args.delivery {
        Gate::Delivery
    } else if args.ci {
        Gate::Integration
    } else {
        Gate::Development
    };
    if report.gate_passed(gate)
        && (!args.require_minimumcd
            || report
                .engineering
                .as_ref()
                .and_then(|p| p.minimumcd.as_ref())
                .is_some_and(|a| a.verdict == AggregateVerdict::Passed))
    {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn apply_local_ci(root: &Path, manifest: &ProjectManifest, report: &mut CheckReport) -> Result<()> {
    let Ok(adapter) = adapter_for(manifest.project.ci.provider) else {
        let outcome = if manifest.project.ci.provider == CiProvider::Unconfigured {
            Outcome::MigrationRequired
        } else {
            Outcome::Unverified
        };
        let capability = Capability {
            outcome,
            evidence: Vec::new(),
            diagnostic: Some(format!(
                "No first-class local adapter is available for {:?}",
                manifest.project.ci.provider
            )),
        };
        apply_capability(report, "MCD-CI-001", &capability);
        apply_capability(report, "MCD-TEST-001", &capability);
        apply_capability(report, "MCD-TEST-002", &capability);
        reaggregate(report)?;
        return Ok(());
    };
    let inspection = adapter.inspect(root)?;
    apply_capability(report, "MCD-CI-001", &inspection.configuration);
    if inspection.integrity.outcome != Outcome::Passed {
        apply_capability(report, "MCD-CI-001", &inspection.integrity);
    }
    apply_capability(report, "MCD-TEST-001", &inspection.pre_merge);
    apply_capability(report, "MCD-TEST-002", &inspection.post_merge);
    reaggregate(report)?;
    Ok(())
}

fn apply_capability(report: &mut CheckReport, rule_id: &str, capability: &Capability) {
    if let Some(result) = report
        .rules
        .iter_mut()
        .find(|result| result.rule_id.as_str() == rule_id)
    {
        // Inspection proves configuration, not successful execution or review.
        // In particular, finding an `opdev check` command cannot clear its failure.
        if matches!(
            result.outcome,
            Outcome::Failed | Outcome::Error | Outcome::MigrationRequired
        ) || capability.outcome == Outcome::Passed
        {
            result
                .evidence
                .extend(capability.evidence.iter().cloned().map(|mut evidence| {
                    evidence.kind = "configured".into();
                    evidence
                }));
            return;
        }
        result.outcome = capability.outcome;
        result.verifier = opdev_core::VerificationSource::Ci;
        result.evidence.clone_from(&capability.evidence);
        result.diagnostic.clone_from(&capability.diagnostic);
    }
}

fn apply_remote_audit(
    root: &Path,
    manifest: &ProjectManifest,
    report: &mut CheckReport,
    revision: Option<&str>,
) -> Result<()> {
    let audit = audit(manifest).context("read-only remote audit could not start")?;
    apply_remote_capability(report, "MCD-CI-001", &audit.ci);
    // Matching provider/default branch names is supporting evidence, not proof
    // that no intermediate integration or release-promotion branch exists.
    if audit.trunk.outcome != Outcome::Passed {
        apply_remote_capability(report, "MCD-TRUNK-001", &audit.trunk);
    }

    let mut lifecycle_evidence = audit.branch_lifecycle.evidence.clone();
    lifecycle_evidence.extend(audit.trunk_protection.evidence.clone());
    let lifecycle = RemoteCapability {
        outcome: Outcome::Unverified,
        evidence: lifecycle_evidence,
        diagnostic: Some(
            "Provider settings do not prove branch origin, age, daily integration, or deletion for every branch"
                .into(),
        ),
    };
    apply_remote_capability(report, "MCD-TRUNK-002", &lifecycle);
    let mut qualification = RemoteCapability {
        outcome: Outcome::Unverified, evidence: Vec::new(),
        diagnostic: Some("Remote CI qualification: Unverified. Source must be clean and unchanged; artifact qualification remains separate.".into()),
    };
    let mut flow = qualification.clone();
    if let Some(revision) = revision
        && clean_remote_revision(root).as_deref() == Some(revision)
    {
        let result = opdev_remote::qualify_trunk(manifest, revision)?;
        qualification.outcome = result.outcome;
        qualification.evidence.push(opdev_core::Evidence {
            kind: "remote_ci_qualification".into(),
            summary: serde_json::to_string(&result)?,
            location: None,
        });
        qualification.diagnostic = Some(format!(
            "Remote CI qualification for {revision}: {:?}. Covers the selected run, required checks and reviewed merge policy; artifact qualification remains separate. Pipeline: {:?}; checks: {:?}; policy: {:?}",
            result.outcome,
            result.pipeline.diagnostic,
            result.checks.diagnostic,
            result.protection.diagnostic
        ));
        flow = qualification.clone();
        flow.outcome = match (result.pipeline.outcome, result.checks.outcome) {
            (Outcome::Failed, _) | (_, Outcome::Failed) => Outcome::Failed,
            (Outcome::Passed, Outcome::Passed) => Outcome::NotApplicable,
            _ => Outcome::Unverified,
        };
        if clean_remote_revision(root).as_deref() != Some(revision) {
            if qualification.outcome != Outcome::Failed {
                qualification.outcome = Outcome::Unverified;
            }
            if flow.outcome != Outcome::Failed {
                flow.outcome = Outcome::Unverified;
            }
            qualification.diagnostic = Some("Source changed during remote qualification; the remote snapshot cannot qualify this working tree".into());
            flow.diagnostic.clone_from(&qualification.diagnostic);
        }
    }
    // Requested remote qualification is mandatory evidence. Generic local or
    // historical assertions must not hide its absence; concrete failures survive.
    apply_required_remote(report, "MCD-CI-001", &qualification);
    apply_required_remote(report, "MCD-TEST-002", &qualification);
    apply_required_remote(report, "MCD-FLOW-001", &flow);
    reaggregate(report)?;
    Ok(())
}

fn apply_remote_capability(report: &mut CheckReport, rule_id: &str, capability: &RemoteCapability) {
    if let Some(result) = report
        .rules
        .iter_mut()
        .find(|result| result.rule_id.as_str() == rule_id)
    {
        if !remote_capability_should_replace(result.outcome, capability.outcome) {
            return;
        }
        result.outcome = capability.outcome;
        result.verifier = opdev_core::VerificationSource::Remote;
        result.evidence.clone_from(&capability.evidence);
        result.diagnostic.clone_from(&capability.diagnostic);
    }
}

fn remote_capability_should_replace(current: Outcome, remote: Outcome) -> bool {
    !matches!(
        current,
        Outcome::Failed | Outcome::Error | Outcome::MigrationRequired
    ) && (remote != Outcome::Unverified || !current.satisfies_required_rule())
}

fn apply_required_remote(report: &mut CheckReport, rule_id: &str, capability: &RemoteCapability) {
    if let Some(result) = report
        .rules
        .iter_mut()
        .find(|result| result.rule_id.as_str() == rule_id)
    {
        if matches!(
            result.outcome,
            Outcome::Failed | Outcome::Error | Outcome::MigrationRequired
        ) {
            result.evidence.extend(capability.evidence.clone());
            if let Some(remote) = &capability.diagnostic {
                result.diagnostic = Some(format!(
                    "{}; {remote}",
                    result
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Existing blocker retained")
                ));
            }
            return;
        }
        result.outcome = capability.outcome;
        result.verifier = opdev_core::VerificationSource::Remote;
        result.evidence.clone_from(&capability.evidence);
        result.diagnostic.clone_from(&capability.diagnostic);
    }
}

fn clean_remote_revision(root: &Path) -> Option<String> {
    use std::process::{Command, Stdio};
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--ignore-submodules=none",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !status.status.success() || !status.stdout.is_empty() {
        return None;
    }
    let head = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--verify", "HEAD^{commit}"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !head.status.success() {
        return None;
    }
    let sha = String::from_utf8(head.stdout).ok()?.trim().to_owned();
    (matches!(sha.len(), 40 | 64) && sha.bytes().all(|b| b.is_ascii_hexdigit())).then_some(sha)
}

fn print_human_report(report: &CheckReport) {
    println!("OpDev report for {}", report.subject);
    println!(
        "A gate is a decision about what may happen next. Blocked means at least one required check or review is not satisfied."
    );
    println!(
        "Results: failed = a requirement was not met; unverified = evidence is missing or stale; error = a tool could not complete; migration_required = setup is incomplete. None means passed."
    );
    if let Ok(catalog) = opdev_core::catalog_for_version(report.catalog_version) {
        for result in &report.rules {
            if result.outcome.satisfies_required_rule() {
                continue;
            }
            if let Some(rule) = catalog.find(&result.rule_id) {
                println!("{} [{}]: {:?}", rule.title, rule.id, result.outcome);
                if report.engineering.is_some()
                    && opdev_core::rule_class(rule.id.as_str())
                        == Some(opdev_core::RuleClass::MinimumcdAssessment)
                {
                    println!(
                        "  MinimumCD assessment only; this finding does not block engineering gates."
                    );
                }
                if let Some(diagnostic) = &result.diagnostic {
                    println!("  {diagnostic}");
                }
                if !result
                    .diagnostic
                    .as_deref()
                    .is_some_and(|text| text.contains(rule.next_step()))
                {
                    println!("  {}", rule.next_step());
                }
            }
        }
    }
    if let Some(result) = report.rules.iter().find(|rule| {
        rule.rule_id.as_str() == "MCD-TEST-002"
            && (rule.verifier == opdev_core::VerificationSource::Remote
                || rule
                    .diagnostic
                    .as_ref()
                    .is_some_and(|text| text.contains("Remote CI qualification")))
    }) && let Some(diagnostic) = &result.diagnostic
    {
        println!("{diagnostic}");
    }
    for check in &report.checks {
        println!(
            "check {}: {:?} — {}",
            check.id, check.outcome, check.summary
        );
        if let Some(stderr) = &check.stderr {
            for line in stderr.lines().take(8) {
                println!("  {line}");
            }
        }
    }
    print_assessments(report);
    print_gate_summary(report);
}

fn print_gate_summary(report: &CheckReport) {
    let mut counts = [0_u32; 6];
    for result in &report.rules {
        counts[outcome_index(result.outcome)] += 1;
    }
    println!(
        "rules: {} passed, {} failed, {} unverified, {} not_applicable, {} error, {} migration_required",
        counts[0], counts[1], counts[2], counts[3], counts[4], counts[5]
    );
    for gate in &report.gates {
        let meaning = match gate.gate {
            Gate::Development => {
                "continuing ordinary implementation (diagnosis and repair remain allowed)"
            }
            Gate::Integration => "merging into the main development branch",
            Gate::Delivery => "publishing or deploying the software",
            Gate::Compliance if report.engineering.is_some() => {
                "claiming the engineering baseline is met (MinimumCD is assessed separately)"
            }
            Gate::Compliance => "claiming the evaluated requirements are met",
        };
        println!("Decision: {meaning}");
        println!(
            "gate {:?}: {:?} ({} rules, {} checks blocking)",
            gate.gate,
            gate.verdict,
            gate.blocking_rules.len(),
            gate.blocking_checks.len()
        );
        if gate.verdict == AggregateVerdict::Blocked {
            let rules = gate
                .blocking_rules
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            if !rules.is_empty() {
                println!("  rules: {rules}");
            }
            if !gate.blocking_checks.is_empty() {
                println!("  checks: {}", gate.blocking_checks.join(", "));
            }
        }
    }
}

fn print_assessments(report: &CheckReport) {
    if let Some(policy) = &report.engineering {
        println!(
            "Engineering baseline: version {}. Operational gates below assess this baseline, not MinimumCD.",
            policy.version
        );
        match &policy.minimumcd {
            None => println!("MinimumCD assessment: not requested; no compliance claim."),
            Some(assessment) => {
                println!(
                    "MinimumCD assessment (mapping {}, source {}): {:?}",
                    assessment.version, assessment.source_version, assessment.verdict
                );
                for requirement in &assessment.requirements {
                    if !requirement.outcome.satisfies_required_rule() {
                        println!(
                            "  {}: {:?}; evidence needed: {}",
                            requirement.id,
                            requirement.outcome,
                            requirement.blocking_rules.join(", ")
                        );
                    }
                }
            }
        }
        println!("Assessment results do not authorize release, deployment or policy changes.");
    }
}

const fn outcome_index(outcome: Outcome) -> usize {
    match outcome {
        Outcome::Passed => 0,
        Outcome::Failed => 1,
        Outcome::Unverified => 2,
        Outcome::NotApplicable => 3,
        Outcome::Error => 4,
        Outcome::MigrationRequired => 5,
    }
}

fn load_project(start: &Path) -> Result<(PathBuf, ProjectManifest)> {
    let discovery = discover(start).context("could not locate the Git repository; run this command from the project folder, or pass --root with its path")?;
    let manifest_path = discovery.root.join(MANIFEST_PATH);
    if !manifest_path.exists() {
        bail!(
            "OpDev is not initialized; run `opdev init --root {}` first",
            discovery.root.display()
        );
    }
    let manifest = ProjectManifest::load(&manifest_path)
        .with_context(|| format!("could not validate `{}`", manifest_path.display()))?;
    Ok((discovery.root, manifest))
}

fn show_rules(args: RulesArgs) -> Result<()> {
    let catalog = opdev_core::catalog_for_version(args.catalog_version)
        .context("could not load the selected rule catalog")?;
    if let Some(id) = args.id {
        let rule = catalog
            .find(&id)
            .with_context(|| format!("the embedded catalog does not contain `{id}`"))?;
        println!("{}: {}", rule.id, rule.title);
        println!("{}", rule.statement);
    } else {
        for rule in catalog.rules {
            println!("{}\t{}", rule.id, rule.title);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn minimumcd_exit_is_explicit_and_cannot_replace_the_requested_gate() -> Result<()> {
        let root = tempfile::tempdir()?;
        std::fs::create_dir(root.path().join(".git"))?;
        let mut manifest = discover(root.path())?.manifest;
        manifest.schema = 3;
        manifest.assurance.profiles.clear();
        manifest.assurance.engineering = Some(opdev_core::EngineeringPolicy {
            version: "1".into(),
            minimumcd: Some("1".into()),
            review_reference: "synthetic decision".into(),
            maintenance_branches: vec![],
        });
        let mut report = evaluate(
            root.path(),
            &manifest,
            CheckOptions {
                execute_checks: false,
                ..CheckOptions::pre_merge()
            },
        )?;
        // Model reviewed findings to isolate CLI exit selection from verification.
        for rule in &mut report.rules {
            rule.outcome = Outcome::Passed;
        }
        for (id, outcome, required, expected) in [
            (
                "MCD-RECOVERY-002",
                Outcome::Unverified,
                false,
                ExitCode::SUCCESS,
            ),
            (
                "MCD-RECOVERY-002",
                Outcome::Unverified,
                true,
                ExitCode::from(1),
            ),
            ("MCD-RECOVERY-002", Outcome::Passed, true, ExitCode::SUCCESS),
            ("OPDEV-STYLE-001", Outcome::Failed, true, ExitCode::from(1)),
        ] {
            report
                .rules
                .iter_mut()
                .find(|r| r.rule_id.as_str() == id)
                .context("rule")?
                .outcome = outcome;
            reaggregate(&mut report)?;
            let mut argv = vec!["opdev", "check", "--ci"];
            if required {
                argv.push("--require-minimumcd");
            }
            let Command::Check(options) = Cli::try_parse_from(argv)?.command else {
                bail!("expected check args")
            };
            assert_eq!(check_exit(&options, &report), expected);
        }
        Ok(())
    }

    #[test]
    fn post_merge_exit_does_not_require_release_readiness() -> Result<()> {
        let root = tempfile::tempdir()?;
        std::fs::create_dir(root.path().join(".git"))?;
        let manifest = discover(root.path())?.manifest;
        let mut report = evaluate(
            root.path(),
            &manifest,
            CheckOptions {
                execute_checks: false,
                ..CheckOptions::pre_merge()
            },
        )?;
        // Isolate gate selection: model reviewed integration with delivery still missing.
        let catalog = embedded_catalog()?;
        for (rule, result) in catalog.rules.iter().zip(&mut report.rules) {
            result.outcome = if rule.gates.contains(&Gate::Delivery)
                && !rule.gates.contains(&Gate::Integration)
                && !rule.gates.contains(&Gate::Development)
            {
                Outcome::Unverified
            } else {
                Outcome::Passed
            };
        }
        reaggregate(&mut report)?;
        assert!(report.gate_passed(Gate::Integration));
        assert!(!report.gate_passed(Gate::Delivery));
        for (flags, expected) in [
            (vec![], ExitCode::SUCCESS),
            (vec!["--ci"], ExitCode::SUCCESS),
            (vec!["--ci", "--post-merge"], ExitCode::SUCCESS),
            (vec!["--ci", "--delivery"], ExitCode::from(1)),
        ] {
            let cli = Cli::try_parse_from(["opdev", "check"].into_iter().chain(flags))?;
            let Command::Check(args) = cli.command else {
                bail!("expected check args")
            };
            assert_eq!(check_exit(&args, &report), expected);
        }
        Ok(())
    }

    #[test]
    fn initialized_dry_run_preserves_contract_and_agent_files() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let root = directory.path();
        std::fs::create_dir(root.join(".git"))?;
        let manifest = discover(root)?.manifest;
        manifest.write_new(&root.join(MANIFEST_PATH))?;
        let before = std::fs::read(root.join(MANIFEST_PATH))?;
        std::fs::write(root.join("AGENTS.md"), "Project-owned instructions\n")?;
        initialize(&InitArgs {
            layout_version: None,
            root: root.to_path_buf(),
            dry_run: true,
            engineering_policy: None,
            policy_review_reference: None,
            minimumcd_assessment: None,
        })?;
        assert_eq!(std::fs::read(root.join(MANIFEST_PATH))?, before);
        assert_eq!(
            std::fs::read_to_string(root.join("AGENTS.md"))?,
            "Project-owned instructions\n"
        );
        assert!(!root.join("CLAUDE.md").exists());
        assert!(!root.join("docs").exists());
        Ok(())
    }

    #[test]
    fn initialization_does_not_scaffold_optional_documentation() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let root = directory.path();
        std::fs::create_dir(root.join(".git"))?;
        initialize(&InitArgs {
            layout_version: None,
            root: root.to_path_buf(),
            dry_run: false,
            engineering_policy: None,
            policy_review_reference: None,
            minimumcd_assessment: None,
        })?;
        for path in [
            MANIFEST_PATH,
            ".opdev/adoption.yaml",
            "AGENTS.md",
            "CLAUDE.md",
        ] {
            assert!(root.join(path).is_file(), "missing {path}");
        }
        for path in [
            ".opdev/design.md",
            ".opdev/development.md",
            ".opdev/delivery.md",
            ".opdev/specs",
            ".opdev/decisions",
            ".opdev/README.md",
            "docs",
            "spec",
            "release",
        ] {
            assert!(!root.join(path).exists(), "unexpected scaffold {path}");
        }
        Ok(())
    }

    #[test]
    fn command_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn final_command_surface_parses() {
        for arguments in [
            vec!["opdev", "init"],
            vec!["opdev", "init", "--root", ".", "--dry-run"],
            vec!["opdev", "check", "--ci", "--remote"],
            vec!["opdev", "check", "--no-exec", "--format", "json"],
            vec!["opdev", "doctor", "--remote"],
            vec!["opdev", "ci", "generate", "--provider", "gitlab"],
            vec![
                "opdev",
                "ci",
                "generate",
                "--provider",
                "gitlab",
                "--image",
                "example.test/toolchain:1",
            ],
            vec!["opdev", "ci", "inspect"],
            vec!["opdev", "upgrade"],
            vec!["opdev", "version"],
            vec![
                "opdev",
                "plugin",
                "verify",
                "--contract",
                "opdev-compatibility.json",
            ],
            vec!["opdev", "rules", "--id", "MCD-TRUNK-001"],
            vec!["opdev", "profiles"],
            vec![
                "opdev",
                "profiles",
                "--name",
                "opdev-core",
                "--version",
                "1",
            ],
            vec!["opdev", "evidence", "fingerprint"],
            vec!["opdev", "evidence", "bootstrap"],
            vec![
                "opdev",
                "evidence",
                "bootstrap",
                "--answers",
                "review.yaml",
                "--write",
            ],
            vec![
                "opdev",
                "release",
                "package",
                "--format",
                "tar-gz",
                "--executable-entry",
                "target/release/opdev=opdev",
                "--entry",
                "LICENSE=LICENSE",
                "--output",
                "opdev.tar.gz",
            ],
            vec![
                "opdev",
                "release",
                "evidence",
                "--artifact",
                "opdev.tar.gz",
                "--sbom",
                "opdev.cdx.json",
                "--source-uri",
                "https://example.test/opdev",
                "--source-revision",
                "0123456789abcdef",
                "--builder-id",
                "https://example.test/builders/1",
                "--output",
                "release",
            ],
        ] {
            assert!(Cli::try_parse_from(arguments).is_ok());
        }
    }

    #[test]
    fn plugin_versions_agree_and_accept_the_independent_cli_version() -> Result<()> {
        let cli_version = Version::parse(env!("CARGO_PKG_VERSION"))?;
        let claude_marketplace: serde_json::Value =
            serde_json::from_str(include_str!("../../../.claude-plugin/marketplace.json"))?;
        let claude_plugin: serde_json::Value = serde_json::from_str(include_str!(
            "../../../plugins/opdev/.claude-plugin/plugin.json"
        ))?;
        let codex_plugin: serde_json::Value = serde_json::from_str(include_str!(
            "../../../plugins/opdev/.codex-plugin/plugin.json"
        ))?;
        let compatibility: PluginCompatibility = serde_json::from_str(include_str!(
            "../../../plugins/opdev/opdev-compatibility.json"
        ))?;

        let expected = compatibility.plugin.version.to_string();
        assert_eq!(claude_marketplace["version"], expected);
        assert_eq!(claude_marketplace["plugins"][0]["version"], expected);
        assert_eq!(claude_plugin["version"], expected);
        assert_eq!(codex_plugin["version"], expected);
        assert_eq!(compatibility.plugin.version.to_string(), expected);
        assert!(compatibility.requires.cli.matches(&cli_version));
        Ok(())
    }

    #[test]
    fn plugin_compatibility_is_fail_closed() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let contract = directory.path().join("compatibility.json");
        std::fs::write(
            &contract,
            r#"{"schema":1,"plugin":{"name":"opdev","version":"99.0.0"},"requires":{"cli":">=99.0.0, <100.0.0"}}"#,
        )?;

        assert_eq!(
            verify_plugin_compatibility(&PluginVerifyArgs {
                contract: contract.clone()
            })?,
            ExitCode::from(1)
        );

        std::fs::write(
            &contract,
            r#"{"schema":2,"plugin":{"name":"opdev","version":"0.1.1"},"requires":{"cli":">=0.1.1, <0.2.0"}}"#,
        )?;
        assert!(
            verify_plugin_compatibility(&PluginVerifyArgs { contract }).is_err(),
            "unknown compatibility schemas must not activate"
        );
        Ok(())
    }

    #[test]
    fn configuration_inspection_preserves_execution_and_review_boundaries() -> Result<()> {
        for current in [
            Outcome::Passed,
            Outcome::Unverified,
            Outcome::Failed,
            Outcome::Error,
            Outcome::MigrationRequired,
        ] {
            for inspected in [
                Outcome::Passed,
                Outcome::Unverified,
                Outcome::Failed,
                Outcome::Error,
                Outcome::MigrationRequired,
            ] {
                let mut report = CheckReport {
                    engineering: None,
                    schema: 1,
                    catalog_version: 2,
                    subject: "fixture".into(),
                    evaluated_at: 0,
                    checks: vec![],
                    gates: vec![],
                    rules: vec![opdev_core::RuleResult {
                        rule_id: "MCD-TEST-001".parse()?,
                        catalog_version: 2,
                        outcome: current,
                        subject: "fixture".into(),
                        verifier: opdev_core::VerificationSource::Command,
                        evaluated_at: 0,
                        evidence: vec![],
                        diagnostic: Some("execution/review finding".into()),
                    }],
                };
                apply_capability(
                    &mut report,
                    "MCD-TEST-001",
                    &Capability {
                        outcome: inspected,
                        evidence: vec![],
                        diagnostic: Some("configuration finding".into()),
                    },
                );
                let preserved = matches!(
                    current,
                    Outcome::Failed | Outcome::Error | Outcome::MigrationRequired
                ) || inspected == Outcome::Passed;
                assert_eq!(
                    report.rules[0].outcome,
                    if preserved { current } else { inspected }
                );
                if preserved {
                    assert_eq!(
                        report.rules[0].diagnostic.as_deref(),
                        Some("execution/review finding")
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn mandatory_remote_evidence_cannot_be_hidden_or_erase_concrete_failures() -> Result<()> {
        for current in [
            Outcome::Passed,
            Outcome::NotApplicable,
            Outcome::Unverified,
            Outcome::Failed,
            Outcome::Error,
            Outcome::MigrationRequired,
        ] {
            let mut report = CheckReport {
                engineering: None,
                schema: 1,
                catalog_version: 1,
                subject: "fixture".into(),
                evaluated_at: 0,
                checks: Vec::new(),
                gates: Vec::new(),
                rules: vec![opdev_core::RuleResult {
                    rule_id: "MCD-TEST-002".parse()?,
                    catalog_version: 1,
                    outcome: current,
                    subject: "fixture".into(),
                    verifier: opdev_core::VerificationSource::Evidence,
                    evaluated_at: 0,
                    evidence: Vec::new(),
                    diagnostic: Some("existing".into()),
                }],
            };
            let remote = RemoteCapability {
                outcome: Outcome::Unverified,
                evidence: Vec::new(),
                diagnostic: Some("missing reviewed policy".into()),
            };
            apply_required_remote(&mut report, "MCD-TEST-002", &remote);
            let hard_failure = matches!(
                current,
                Outcome::Failed | Outcome::Error | Outcome::MigrationRequired
            );
            assert_eq!(
                report.rules[0].outcome,
                if hard_failure {
                    current
                } else {
                    Outcome::Unverified
                }
            );
            assert_eq!(
                report.rules[0].diagnostic.as_deref(),
                Some(if hard_failure {
                    "existing; missing reviewed policy"
                } else {
                    "missing reviewed policy"
                })
            );
        }
        Ok(())
    }

    #[test]
    fn remote_revision_requires_a_clean_committed_source_tree() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let root = directory.path();
        let git = |args: &[&str]| -> Result<()> {
            anyhow::ensure!(
                std::process::Command::new("git")
                    .arg("-C")
                    .arg(root)
                    .args(args)
                    .output()?
                    .status
                    .success()
            );
            Ok(())
        };
        git(&["init"])?;
        assert!(clean_remote_revision(root).is_none());
        git(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-m",
            "fixture",
        ])?;
        assert!(clean_remote_revision(root).is_some());
        std::fs::write(root.join("change.txt"), "untracked")?;
        assert!(clean_remote_revision(root).is_none());
        git(&["add", "change.txt"])?;
        assert!(clean_remote_revision(root).is_none());
        Ok(())
    }

    #[test]
    fn inconclusive_remote_audit_does_not_erase_satisfying_evidence() {
        assert!(!remote_capability_should_replace(
            Outcome::Passed,
            Outcome::Unverified
        ));
        assert!(!remote_capability_should_replace(
            Outcome::NotApplicable,
            Outcome::Unverified
        ));
        assert!(remote_capability_should_replace(
            Outcome::Passed,
            Outcome::Failed
        ));
        assert!(!remote_capability_should_replace(
            Outcome::MigrationRequired,
            Outcome::Unverified
        ));
    }

    #[test]
    fn node_and_go_bootstrap_inputs_are_smaller_without_changing_rule_outcomes()
    -> Result<(), Box<dyn std::error::Error>> {
        for (file, contents) in [
            (
                "package.json",
                r#"{"name":"bootstrap-node","scripts":{"test":"node --test"}}"#,
            ),
            ("go.mod", "module example.test/bootstrap-go\n\ngo 1.24\n"),
        ] {
            let directory = tempfile::tempdir()?;
            let root = directory.path();
            assert!(
                std::process::Command::new("git")
                    .arg("init")
                    .arg(root)
                    .status()?
                    .success()
            );
            std::fs::write(root.join(file), contents)?;
            std::fs::write(
                root.join("OPDEV_ADOPTION.md"),
                "Reviewed project policy, applicability, change scope, tests, and integration behavior.\n",
            )?;
            let discovery = discover(root)?;
            discovery.manifest.write_new(&root.join(MANIFEST_PATH))?;
            assert!(
                std::process::Command::new("git")
                    .arg("-C")
                    .arg(root)
                    .args(["add", "."])
                    .status()?
                    .success()
            );

            let fingerprint = staged_fingerprint(root)?;
            let mut report = evaluate(
                root,
                &discovery.manifest,
                CheckOptions {
                    execute_checks: false,
                    ..CheckOptions::pre_merge()
                },
            )?;
            apply_local_ci(root, &discovery.manifest, &mut report)?;
            let catalog = embedded_catalog()?;
            let (project_rules, change_rules) = evidence_candidates(&catalog, &report);
            assert!(!project_rules.is_empty());
            assert!(!change_rules.is_empty());

            let mut review = EvidenceBootstrap::new(
                fingerprint.clone(),
                project_rules.clone(),
                change_rules.clone(),
            );
            for decision in review.project.decisions.values_mut() {
                *decision = opdev_project::ReviewDecision::Passed;
            }
            for decision in review.change.decisions.values_mut() {
                *decision = opdev_project::ReviewDecision::Passed;
            }
            let shared = opdev_core::Evidence {
                kind: "review".into(),
                summary: "The adoption review records the facts supporting these decisions.".into(),
                location: Some("OPDEV_ADOPTION.md".into()),
            };
            review.project.evidence.push(shared.clone());
            review.change.evidence.push(shared);
            review.change.work = "OPDEV-15 greenfield adoption review".into();
            review.validate_candidates(&project_rules, &change_rules, &fingerprint)?;
            let review_yaml = review.to_yaml()?;
            let ledger = review.to_ledger(&catalog)?;
            let ledger_yaml = ledger.to_yaml()?;
            assert!(review_yaml.lines().count() * 2 < ledger_yaml.lines().count());
            ledger.write_new(root, &catalog)?;

            let mut verified = evaluate(
                root,
                &discovery.manifest,
                CheckOptions {
                    execute_checks: false,
                    ..CheckOptions::pre_merge()
                },
            )?;
            apply_local_ci(root, &discovery.manifest, &mut verified)?;
            for rule_id in project_rules.iter().chain(&change_rules) {
                assert_eq!(
                    verified
                        .rules
                        .iter()
                        .find(|result| result.rule_id.as_str() == rule_id)
                        .map(|result| result.outcome),
                    Some(Outcome::Passed),
                    "{file} did not preserve the accepted outcome for {rule_id}"
                );
            }
        }
        Ok(())
    }
}
