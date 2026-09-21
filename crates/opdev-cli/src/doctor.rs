//! Read-only, scoped readiness diagnostics. Never qualification or repair.
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Context, Result, bail};
use clap::Args;
use opdev_core::{Outcome, PROJECT_SCHEMA_VERSION};
use opdev_engine::{ProgramLocation, inspect_program};
use opdev_project::{
    AuthorityKind, CiProvider, CoverageMode, DeliveryStatus, MANIFEST_PATH, ProjectManifest,
    RecoveryStrategy,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    OutputFormat,
    inspection::{self, Inventory},
};

const LIMIT: u64 = 1024 * 1024;
const LIMITS: &str = "Read-only observations, not launch verification, completed adoption, passing tests, CI qualification or release readiness. No project commands or repairs executed. Review paths before sharing this report.";

#[derive(Debug, Args)]
pub(super) struct DoctorArgs {
    /// Directory to inspect; runtime diagnostics also work outside initialized projects.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Installed or candidate plugin directory to inspect without executing its scripts.
    #[arg(long)]
    plugin_root: Option<PathBuf>,
    /// Include existing read-only provider observations using current credentials.
    #[arg(long)]
    remote: bool,
    /// Human text or schema-versioned JSON, with identical exit semantics.
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    format: OutputFormat,
}

#[derive(Serialize)]
struct Finding {
    id: String,
    scope: &'static str,
    subject: String,
    outcome: Outcome,
    /// Applies to the explicitly inspected prerequisites, never a core gate.
    required: bool,
    observation: String,
    next_step: String,
}

#[derive(Serialize)]
struct Runtime {
    path: Option<PathBuf>,
    version: &'static str,
    sha256: Option<String>,
    os: &'static str,
    arch: &'static str,
    wsl_environment_hint: bool,
    project_schemas: Vec<u32>,
    evidence_schemas: Vec<u32>,
    capabilities: Vec<&'static str>,
}

#[derive(Serialize)]
struct Report {
    schema: u32,
    root: PathBuf,
    runtime: Runtime,
    remote_requested: bool,
    exit_code: u8,
    project_verification: Outcome,
    limits: &'static str,
    findings: Vec<Finding>,
    #[serde(skip)]
    plugin_root: Option<PathBuf>,
}

impl Report {
    #[allow(clippy::too_many_arguments)] // Explicit diagnostic contract, not a positional CLI.
    fn add(
        &mut self,
        id: &str,
        scope: &'static str,
        subject: impl Into<String>,
        outcome: Outcome,
        required: bool,
        observation: impl Into<String>,
        next_step: impl Into<String>,
    ) {
        self.findings.push(Finding {
            id: id.into(),
            scope,
            subject: subject.into(),
            outcome,
            required,
            observation: observation.into(),
            next_step: if outcome == Outcome::Passed {
                "No repair indicated; this observation does not verify behavior or qualify a gate."
                    .into()
            } else {
                next_step.into()
            },
        });
    }

    fn inspection_error(&mut self, id: &str, subject: &str) {
        self.add(id, "inspection", subject, Outcome::Error, true,
            "Could not inspect this input safely (unreadable, linked, oversized, malformed or unsupported). Input contents are not echoed.",
            "Inspect the named input and access permissions locally; correct it or select a compatible runtime, then rerun doctor. Nothing was changed.");
    }

    fn finish(&mut self) {
        self.exit_code = if self.findings.iter().any(|f| f.outcome == Outcome::Error) {
            2
        } else {
            u8::from(self.findings.iter().any(|f| {
                f.required && !matches!(f.outcome, Outcome::Passed | Outcome::NotApplicable)
            }))
        };
    }
}

impl Inventory for Report {
    fn root(&self) -> &Path {
        &self.root
    }
    fn version(&self) -> &str {
        self.runtime.version
    }
    fn read(&mut self, key: &str, path: &Path) -> Result<Option<String>> {
        let base = if key.starts_with("plugin:") {
            self.plugin_root
                .as_deref()
                .context("plugin directory is unavailable")?
        } else {
            &self.root
        };
        read_text(base, path)
    }
    fn finding(&mut self, component: &str, outcome: Outcome, detail: impl Into<String>) {
        if outcome == Outcome::Error {
            self.inspection_error(&format!("inventory.{component}"), component);
            return;
        }
        let required = component == "plugin" && self.plugin_root.is_some()
            || component == "runtime_pin" && outcome == Outcome::Failed;
        let next = match component {
            "plugin" | "runtime_pin" => {
                "If plugin use is intended, verify the selected runtime with the existing runtime lookup; request setup or an upgrade separately if needed."
            }
            _ => {
                "Review CI scripts, includes, variables and signature configuration together; matching pins do not qualify the effective CI runtime."
            }
        };
        self.add(
            &format!("inventory.{component}"),
            "inventory",
            component,
            outcome,
            required,
            detail,
            next,
        );
    }
    fn source_finding(
        &mut self,
        component: &str,
        subject: &str,
        outcome: Outcome,
        detail: impl Into<String>,
    ) {
        if outcome == Outcome::Error {
            self.inspection_error(&format!("inventory.{component}"), subject);
        } else {
            self.add(&format!("inventory.{component}"), "inventory", subject, outcome, false, detail,
                "Review this CI file together with includes, variables and signature configuration; no effective runtime or qualification was established.");
        }
    }
}

pub(super) fn run(args: &DoctorArgs) -> Result<ExitCode> {
    let report = inspect(args);
    match args.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputFormat::Human => print_report(&report),
    }
    Ok(ExitCode::from(report.exit_code))
}

fn inspect(args: &DoctorArgs) -> Report {
    let mut report = Report {
        schema: 1,
        root: args.root.clone(),
        remote_requested: args.remote,
        runtime: Runtime {
            path: std::env::current_exe().ok(),
            version: env!("CARGO_PKG_VERSION"),
            sha256: None,
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            wsl_environment_hint: std::env::var_os("WSL_DISTRO_NAME").is_some()
                || std::env::var_os("WSL_INTEROP").is_some(),
            project_schemas: (1..=PROJECT_SCHEMA_VERSION).collect(),
            evidence_schemas: vec![1, 2],
            capabilities: vec!["doctor.v1", "upgrade.preview", "evidence.acceptance-digest"],
        },
        exit_code: 0,
        project_verification: Outcome::Unverified,
        limits: LIMITS,
        findings: Vec::new(),
        plugin_root: args
            .plugin_root
            .as_ref()
            .and_then(|p| p.canonicalize().ok()),
    };
    match report
        .runtime
        .path
        .as_ref()
        .map(|path| hash_file(path))
        .transpose()
    {
        Ok(Some(digest)) => {
            report.runtime.sha256 = Some(digest);
            report.add("runtime.identity", "runtime", "running CLI", Outcome::Passed, true,
                "Exact executable digest and this build's capabilities recorded. Version equality alone does not establish capability equality or authenticity.",
                "Use this executable for the intended operation; verify installation authenticity through the existing installer when needed.");
        }
        _ => report.inspection_error("runtime.identity", "running CLI executable"),
    }
    report.add("environment.scope", "runtime", "current process environment", Outcome::Passed, false,
        "Only this process's OS, architecture and filesystem are inspected. WSL environment variables are hints, not proof of a usable distribution.",
        "Run doctor in the environment selected for verification. No WSL distribution, container, shell or tool manager is started.");
    if inspection::inspect_plugin(&mut report, args.plugin_root.as_deref()).is_err() {
        report.inspection_error(
            "inventory.plugin",
            "--plugin-root compatibility contract/runtime.lock",
        );
    }
    match locate_root(&args.root) {
        Ok(root) => {
            report.root = root;
            inspect_project(&mut report);
        }
        Err(_) => report.inspection_error("project.root", "--root directory"),
    }
    report.finish();
    report
}

fn inspect_project(report: &mut Report) {
    let path = report.root.join(MANIFEST_PATH);
    match read_text(&report.root, &path) {
        Ok(None) => {
            report.add("project.contract", "project", MANIFEST_PATH, Outcome::NotApplicable, false,
                "No project contract found; runtime-only inspection. Project readiness was not assessed and adoption was not started.",
                "For substantive development, offer OpDev adoption and wait for consent; routine operations need no adoption.");
            if report.remote_requested {
                report.add(
                    "remote.blocked",
                    "remote",
                    "provider observations",
                    Outcome::Unverified,
                    true,
                    "Requested remote inspection needs an existing valid project contract.",
                    "Select an initialized project; doctor does not create a contract.",
                );
            }
        }
        Ok(Some(source)) => {
            if let Ok(manifest) = ProjectManifest::from_yaml(&source) {
                report.add("project.contract", "project", MANIFEST_PATH, Outcome::Passed, true,
                        format!("Project schema {} parsed and validated; no policies selected or changed.", manifest.schema),
                        "Review command and authority prerequisites below; a valid contract is not completed adoption.");
                inspect_commands(report, &manifest);
                inspect_authorities(report, &manifest);
                inspect_gaps(report, &manifest);
                if report.remote_requested {
                    inspect_remote(report, &manifest);
                }
            } else {
                report.inspection_error("project.contract", MANIFEST_PATH);
                report.add("project.dependents", "project", "commands, authorities, gaps and remote", Outcome::Unverified, true,
                        "Dependent checks were not attempted because the project contract is invalid or unsupported.",
                        "Resolve project.contract first; independent runtime and inventory observations remain available.");
            }
        }
        Err(_) => report.inspection_error("project.contract", MANIFEST_PATH),
    }
    if !report.remote_requested {
        report.add("remote.not_requested", "remote", "provider observations", Outcome::Unverified, false,
            "Remote access and provider settings were not inspected; no network or credential lookup was requested.",
            "Use --remote only when read-only provider inspection is relevant and authorized.");
    }
    if guard_path(&report.root, &report.root.join(".github/workflows")).is_err()
        || inspection::inspect_ci(report).is_err()
    {
        report.inspection_error("inventory.ci", "local CI configuration paths");
    }
}

fn inspect_commands(report: &mut Report, manifest: &ProjectManifest) {
    for (name, command) in &manifest.commands {
        let directory = report
            .root
            .join(command.working_directory.as_deref().unwrap_or("."));
        let subject = format!("commands.{name}");
        let cwd = inspect_path(&report.root, &directory, true);
        report.add("command.directory", "commands", &subject, cwd.0, true, cwd.1,
            "Correct the declared working directory or access permissions; doctor never creates directories.");
        if cwd.0 != Outcome::Passed {
            report.add(
                "command.executable",
                "commands",
                &subject,
                Outcome::Unverified,
                true,
                "Executable inspection depends on the unresolved working directory.",
                "Resolve command.directory first.",
            );
            continue;
        }
        let program = &command.argv[0];
        let (outcome, observation) = match inspect_program(program) {
            ProgramLocation::Located(path) => (Outcome::Passed, format!("Executable file found at {}. It was not run; version, dependencies, architecture and launch permissions remain unchecked.", path.display())),
            ProgramLocation::Missing => (Outcome::Failed, "No executable file found by the unambiguous local lookup (or no Unix execute bits). No command was run.".into()),
            ProgramLocation::Unverified(reason) => (Outcome::Unverified, reason.into()),
            ProgramLocation::Error(io::ErrorKind::PermissionDenied) => (Outcome::Unverified, "Insufficient permission to inspect executable availability; absence is not established.".into()),
            ProgramLocation::Error(kind) => (Outcome::Error, format!("Executable filesystem inspection could not finish: {kind:?}. This is not proof of absence.")),
        };
        report.add("command.executable", "commands", &subject, outcome, true, observation,
            "Review the declared executable and current PATH in the intended environment; request installation separately if required.");
        // Do not infer inner commands from argv: wrappers and shims can be arbitrary.
        report.add("command.execution", "execution", subject, Outcome::Unverified, false,
            "Command arguments, scripts and nested environments were not executed or verified.",
            "For WSL, containers, shells or tool-manager shims, inspect inside the intended environment; run reviewed canonical checks separately.");
    }
}

fn inspect_authorities(report: &mut Report, manifest: &ProjectManifest) {
    for (name, authority) in &manifest.authorities {
        if authority.kind == AuthorityKind::Path {
            let (outcome, observation) =
                inspect_path(&report.root, &report.root.join(&authority.location), false);
            report.add("authority.path", "authorities", format!("authorities.{name}"), outcome, true, observation,
                "Review the existing authority location and access permissions; do not create or relocate project documents just to match a convention.");
        } else {
            report.add("authority.external", "authorities", format!("authorities.{name}"), Outcome::Unverified, false,
                "External authority was not fetched; its URL and any embedded credentials are not echoed.",
                "Use the declared authority when the task needs it; doctor never follows arbitrary authority URLs.");
        }
    }
}

fn inspect_gaps(report: &mut Report, manifest: &ProjectManifest) {
    for (id, missing, observation, next) in [
        (
            "ci",
            manifest.project.ci.provider == CiProvider::Unconfigured,
            "CI provider is unconfigured.",
            "Resolve provider selection through the adoption decision flow.",
        ),
        (
            "coverage",
            manifest.testing.coverage.mode == CoverageMode::Unconfigured,
            "Coverage evidence is unconfigured.",
            "Review proportionate coverage evidence during adoption; doctor does not select a tool or threshold.",
        ),
        (
            "delivery",
            manifest.delivery.status == DeliveryStatus::MigrationRequired,
            "Delivery qualification is migration_required.",
            "Resolve the tracked delivery migration before qualified delivery.",
        ),
        (
            "recovery",
            manifest.delivery.recovery.strategy == RecoveryStrategy::Unconfigured,
            "Automated recovery is unconfigured.",
            "Review and test a project-appropriate recovery strategy before delivery.",
        ),
        (
            "commands",
            manifest.commands.is_empty(),
            "No canonical project commands are declared.",
            "Review and declare the project's actual verification commands; doctor does not infer tooling.",
        ),
    ] {
        if missing {
            report.add(
                &format!("gap.{id}"),
                "adoption_delivery",
                id,
                Outcome::MigrationRequired,
                false,
                observation,
                next,
            );
        }
    }
}

fn inspect_remote(report: &mut Report, manifest: &ProjectManifest) {
    match opdev_remote::audit(manifest) {
        Ok(audit) => append_remote(report, &audit),
        Err(_) => report.add("remote.configuration", "remote", "provider observations", Outcome::Unverified, true,
            "Remote inspection could not start with the declared provider and repository. No alternate host or login was attempted.",
            "Review the provider/remote configuration and supported provider capabilities. Authentication changes require a separate explicit action."),
    }
}

fn append_remote(report: &mut Report, audit: &opdev_remote::RemoteAudit) {
    for (name, capability) in [
        ("ci", &audit.ci),
        ("trunk", &audit.trunk),
        ("trunk_protection", &audit.trunk_protection),
        ("trunk_pipeline", &audit.trunk_pipeline),
        ("branch_lifecycle", &audit.branch_lifecycle),
    ] {
        // A latest-green snapshot deliberately remains unverified by the existing
        // adapter. It is not a requested qualification; concrete red still blocks.
        let required = name != "trunk_pipeline" || capability.outcome != Outcome::Unverified;
        let observation = capability.diagnostic.clone().unwrap_or_else(|| {
            capability
                .evidence
                .iter()
                .map(|item| item.summary.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        });
        report.add(&format!("remote.{name}"), "remote", name, capability.outcome, required, observation,
            "Review this specific provider observation and its visibility with existing credentials; read access does not establish write permission, exact-revision CI qualification or release readiness.");
    }
}

fn locate_root(start: &Path) -> io::Result<PathBuf> {
    let start = start.canonicalize()?;
    if !start.is_dir() {
        return Err(io::Error::other("root is not a directory"));
    }
    for directory in start.ancestors() {
        for marker in [".git", MANIFEST_PATH] {
            match fs::symlink_metadata(directory.join(marker)) {
                Ok(_) => return Ok(directory.to_path_buf()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
    }
    Ok(start)
}

// No content reads through links/junctions or special files. The explicitly
// selected root may be canonicalized, but all inspected children stay local.
fn guard_path(root: &Path, path: &Path) -> Result<()> {
    let relative = path.strip_prefix(root)?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        if !matches!(
            component,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        ) {
            bail!("path leaves inspection root");
        }
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if metadata.file_attributes() & 0x400 != 0 {
                        bail!("reparse point not inspected");
                    }
                }
                if metadata.file_type().is_symlink() {
                    bail!("linked input not inspected");
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn read_text(root: &Path, path: &Path) -> Result<Option<String>> {
    guard_path(root, path)?;
    match fs::metadata(path) {
        Ok(metadata) if !metadata.is_file() => bail!("not a regular file"),
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let mut source = String::new();
    fs::File::open(path)?
        .take(LIMIT + 1)
        .read_to_string(&mut source)?;
    if source.len() as u64 > LIMIT {
        bail!("input exceeds inspection limit");
    }
    Ok(Some(source))
}

fn inspect_path(root: &Path, path: &Path, directory: bool) -> (Outcome, String) {
    if guard_path(root, path).is_err() {
        return (Outcome::Unverified, "Path could not be safely inspected within the selected root; review links/junctions and access permissions.".into());
    }
    let observed = (|| -> io::Result<bool> {
        let metadata = fs::metadata(path)?;
        if metadata.is_dir() {
            fs::read_dir(path)?;
        } else if metadata.is_file() && !directory {
            fs::File::open(path)?;
        } else {
            return Ok(false);
        }
        Ok(true)
    })();
    match observed {
        Ok(true) => (
            Outcome::Passed,
            format!(
                "{} exists with read access in this environment; content adequacy and execution are not verified.",
                path.display()
            ),
        ),
        Ok(false) => (
            Outcome::Failed,
            "Path has the wrong type for this declared use.".into(),
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => (
            Outcome::Failed,
            "Declared path does not exist in this environment.".into(),
        ),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => (
            Outcome::Unverified,
            "Insufficient permission to inspect the declared path; absence is not established."
                .into(),
        ),
        Err(_) => (
            Outcome::Error,
            "Filesystem inspection failed; absence is not established.".into(),
        ),
    }
}

fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 16384];
    loop {
        let length = file.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        digest.update(&buffer[..length]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn printable(text: &str) -> String {
    text.chars()
        .flat_map(|ch| {
            if ch.is_control() {
                ch.escape_default().collect::<Vec<_>>()
            } else {
                vec![ch]
            }
        })
        .collect()
}

fn print_report(report: &Report) {
    println!("OpDev doctor: read-only readiness observations");
    println!("Root: {}", printable(&report.root.to_string_lossy()));
    println!(
        "CLI: {} at {} ({}/{})",
        report.runtime.version,
        printable(&report.runtime.path.as_ref().map_or_else(
            || "unavailable".into(),
            |p| p.to_string_lossy().into_owned()
        )),
        report.runtime.os,
        report.runtime.arch
    );
    println!(
        "SHA-256: {}",
        report.runtime.sha256.as_deref().unwrap_or("unavailable")
    );
    println!(
        "Supported project schemas: {:?}; evidence schemas: {:?}; capabilities: {}",
        report.runtime.project_schemas,
        report.runtime.evidence_schemas,
        report.runtime.capabilities.join(", ")
    );
    for finding in &report.findings {
        let outcome = serde_json::to_value(finding.outcome)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| "error".into());
        println!(
            "{}: {} [{}; {}] {}",
            finding.id,
            outcome,
            printable(&finding.subject),
            if finding.required {
                "selected prerequisite"
            } else {
                "informational scope"
            },
            printable(&finding.observation)
        );
        if finding.outcome != Outcome::Passed {
            println!("  Next: {}", printable(&finding.next_step));
        }
    }
    println!(
        "Inspection exit: {}. Project verification: unverified.",
        report.exit_code
    );
    println!("{}", report.limits);
}

#[cfg(test)]
mod tests {
    use super::*;
    use opdev_remote::{RemoteAudit, RemoteCapability};

    #[test]
    fn remote_permission_unknown_red_and_green_keep_distinct_exits() -> Result<()> {
        let root = tempfile::tempdir()?;
        let capability = |outcome| RemoteCapability {
            outcome,
            evidence: Vec::new(),
            diagnostic: Some("offline provider observation".into()),
        };
        for (settings, pipeline, exit) in [
            (Outcome::Unverified, Outcome::Unverified, 1),
            (Outcome::Passed, Outcome::Failed, 1),
            (Outcome::Passed, Outcome::Unverified, 0),
            (Outcome::Error, Outcome::Unverified, 2),
        ] {
            let mut report = inspect(&DoctorArgs {
                root: root.path().into(),
                plugin_root: None,
                remote: false,
                format: OutputFormat::Json,
            });
            append_remote(
                &mut report,
                &RemoteAudit {
                    repository: "fixture/project".into(),
                    ci: capability(settings),
                    trunk: capability(settings),
                    trunk_protection: capability(settings),
                    branch_lifecycle: capability(settings),
                    trunk_pipeline: capability(pipeline),
                },
            );
            report.finish();
            assert_eq!(report.exit_code, exit);
            assert_eq!(report.project_verification, Outcome::Unverified);
            assert_eq!(
                report
                    .findings
                    .iter()
                    .find(|f| f.id == "remote.trunk_protection")
                    .context("protection")?
                    .outcome,
                settings
            );
            assert_eq!(
                report
                    .findings
                    .iter()
                    .find(|f| f.id == "remote.trunk_pipeline")
                    .context("pipeline")?
                    .outcome,
                pipeline
            );
        }
        Ok(())
    }

    #[test]
    fn control_characters_cannot_hide_human_diagnostics() {
        assert_eq!(
            printable("safe\n\x1b[31m\rhidden"),
            "safe\\n\\u{1b}[31m\\rhidden"
        );
    }
}
