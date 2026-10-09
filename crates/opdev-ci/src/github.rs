//! Conservative offline GitHub wiring inspection, never execution/protection proof.

use crate::{Capability, CiInspection, TemplateContext, gitlab, passed};
use opdev_core::Outcome;
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path};

type Problem = (Outcome, String);
type Result<T> = std::result::Result<T, Problem>;
const DIRECTORY: &str = ".github/workflows";
const MAX_FILES: usize = 64;
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_DEPTH: usize = 8;

fn unknown(message: impl Into<String>) -> Problem {
    (Outcome::Unverified, message.into())
}

fn invalid(message: impl Into<String>) -> Problem {
    (Outcome::Error, message.into())
}

fn capability(problem: Problem) -> Capability {
    Capability {
        outcome: problem.0,
        evidence: vec![],
        diagnostic: Some(problem.1),
    }
}

fn all(problem: Problem) -> CiInspection {
    let item = capability(problem);
    CiInspection {
        configuration: item.clone(),
        pre_merge: item.clone(),
        post_merge: item.clone(),
        integrity: item,
    }
}

pub(crate) fn inspect(root: &Path, trunk: &str) -> CiInspection {
    let documents = match read(root) {
        Ok(documents) => documents,
        Err(problem) => return all(problem),
    };
    let pre = stage(&documents, trunk, false);
    let post = stage(&documents, trunk, true);
    let integrity = match (&pre, &post) {
        (Ok(a), Ok(b)) => passed(
            &root.join(DIRECTORY),
            &format!(
                "Configured runtime paths reviewed at {a} and {b}; no execution or remote enforcement verified"
            ),
        ),
        (Err(problem), _) | (_, Err(problem)) => capability(problem.clone()),
    };
    let convert = |result: Result<String>| match result {
        Ok(location) => passed(
            &root.join(location),
            "Supported event, exact checkout, stage and failure-propagating gate are configured; execution and required-check protection remain separate",
        ),
        Err(problem) => capability(problem),
    };
    CiInspection {
        configuration: passed(
            &root.join(DIRECTORY),
            "Bounded GitHub workflow inventory parsed; no job was executed",
        ),
        pre_merge: convert(pre),
        post_merge: convert(post),
        integrity,
    }
}

fn linked(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    metadata.file_type().is_symlink()
}

fn read(root: &Path) -> Result<BTreeMap<String, Value>> {
    for part in [".github", DIRECTORY] {
        let metadata = match fs::symlink_metadata(root.join(part)) {
            Ok(value) => value,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err((Outcome::MigrationRequired, "No GitHub workflows found in .github/workflows; inspect the project's CI location before proposing changes".into())),
            Err(_) => return Err(invalid("GitHub workflow directory is unreadable")),
        };
        if linked(&metadata) || !metadata.is_dir() {
            return Err(invalid(
                "GitHub workflow directory must be a real directory without linked children",
            ));
        }
    }
    let entries = fs::read_dir(root.join(DIRECTORY))
        .map_err(|_| invalid("GitHub workflow directory is unreadable"))?;
    let mut documents = BTreeMap::new();
    let mut bytes = 0;
    let mut entries_seen = 0;
    for entry in entries {
        entries_seen += 1;
        if entries_seen > 256 {
            return Err(unknown(
                "GitHub workflow directory exceeds the 256-entry inspection limit",
            ));
        }
        let entry = entry.map_err(|_| invalid("GitHub workflow directory entry is unreadable"))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("GitHub workflow filename must be UTF-8"))?;
        if !matches!(
            Path::new(&name).extension().and_then(|ext| ext.to_str()),
            Some("yml" | "yaml")
        ) {
            continue;
        }
        if documents.len() >= MAX_FILES {
            return Err(unknown("GitHub workflow inventory exceeds 64 files"));
        }
        let path = format!("{DIRECTORY}/{name}");
        let source = gitlab::read_local(root, &path).map_err(|e| (e.outcome, e.diagnostic))?;
        bytes += source.len();
        if bytes > MAX_BYTES {
            return Err(unknown("GitHub workflow input exceeds 4 MiB"));
        }
        let value: Value = serde_saphyr::from_str(&source)
            .map_err(|_| invalid(format!("Invalid GitHub workflow YAML in {path}")))?;
        let jobs = value
            .get("jobs")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid(format!("GitHub workflow {path} needs a jobs mapping")))?;
        if !value.is_object() || value.get("on").is_none() || jobs.is_empty() {
            return Err(invalid(format!(
                "GitHub workflow {path} needs events and nonempty jobs"
            )));
        }
        if jobs.len() > 256 {
            return Err(unknown("GitHub workflow exceeds 256 jobs"));
        }
        documents.insert(path, value);
    }
    if documents.is_empty() {
        return Err((
            Outcome::MigrationRequired,
            "No .yml or .yaml GitHub workflows found".into(),
        ));
    }
    Ok(documents)
}

fn event<'a>(workflow: &'a Value, name: &str) -> Option<&'a Value> {
    let on = workflow.get("on")?;
    if on.as_str() == Some(name)
        || on
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(name)))
    {
        Some(&Value::Null)
    } else {
        on.as_object()?.get(name)
    }
}

fn admitted(workflow: &Value, trunk: &str, post: bool) -> Result<bool> {
    let Some(settings) = event(workflow, if post { "push" } else { "pull_request" }) else {
        return Ok(false);
    };
    if settings.is_null() {
        return Ok(true);
    }
    let settings = settings
        .as_object()
        .ok_or_else(|| invalid("GitHub event settings must be a mapping"))?;
    if settings
        .keys()
        .any(|key| !matches!(key.as_str(), "branches" | "types"))
    {
        return Err(unknown(
            "Event paths, exclusions, tags or other filters need effective-provider review; an always-required check is not established",
        ));
    }
    if let Some(branches) = settings.get("branches") {
        let branches = branches
            .as_array()
            .ok_or_else(|| invalid("GitHub branches must be an array"))?;
        if branches.iter().any(|v| {
            v.as_str()
                .is_none_or(|s| s.contains(['!', '*', '?', '[', '$']))
        }) {
            return Err(unknown(
                "Dynamic/glob branch filters need provider review; declared trunk coverage is unverified",
            ));
        }
        if !branches.iter().any(|v| v.as_str() == Some(trunk)) {
            return Ok(false);
        }
    }
    if let Some(types) = settings.get("types") {
        let types = types
            .as_array()
            .ok_or_else(|| invalid("GitHub event types must be an array"))?;
        if post
            || !["opened", "synchronize", "reopened"]
                .iter()
                .all(|kind| types.iter().any(|v| v.as_str() == Some(kind)))
        {
            return Err(unknown(
                "Selected event activity types do not establish every required change check",
            ));
        }
    }
    Ok(true)
}

fn expression(value: &str) -> &str {
    let value = value.trim();
    value
        .strip_prefix("${{")
        .and_then(|v| v.strip_suffix("}}"))
        .unwrap_or(value)
        .trim()
}

fn enabled(value: Option<&Value>, post: bool) -> Result<bool> {
    let Some(value) = value else {
        return Ok(true);
    };
    if let Some(value) = value.as_bool() {
        return Ok(value);
    }
    let raw = value
        .as_str()
        .ok_or_else(|| invalid("GitHub condition must be a boolean or expression"))?;
    let value = expression(raw).replace('"', "'");
    match value.as_str() {
        "true" | "success()" => Ok(true),
        "false" => Ok(false),
        "github.event_name == 'push'" | "github.event_name != 'pull_request'" => Ok(post),
        "github.event_name == 'pull_request'" | "github.event_name != 'push'" => Ok(!post),
        _ => Err(unknown(
            "Conditional gate/setup needs explicit provider review; unsupported expressions are not assumed true",
        )),
    }
}

fn required(value: &Value) -> Result<()> {
    match value.get("continue-on-error") {
        None | Some(Value::Bool(false)) => Ok(()),
        Some(Value::Bool(true)) => Err((
            Outcome::Failed,
            "The OpDev gate or its setup allows failure; it cannot establish required verification"
                .into(),
        )),
        _ => Err(unknown(
            "Dynamic continue-on-error does not establish required verification",
        )),
    }
}

fn hint(job: &Value) -> bool {
    job.get("uses").is_some()
        || job["steps"].as_array().is_some_and(|steps| {
            steps.iter().any(|s| {
                s["run"].as_str().is_some_and(|run| {
                    lines(run)
                        .iter()
                        .any(|line| line.contains("opdev") && line.contains("check"))
                })
            })
        })
}

fn stage(documents: &BTreeMap<String, Value>, trunk: &str, post: bool) -> Result<String> {
    let mut gates = vec![];
    let mut visits = 0;
    for (path, workflow) in documents {
        if !workflow["jobs"]
            .as_object()
            .is_some_and(|jobs| jobs.values().any(hint))
        {
            continue;
        }
        if admitted(workflow, trunk, post)? {
            visit(documents, path, post, &mut vec![], &mut gates, &mut visits)?;
        }
    }
    match gates.as_slice() {
        [gate] => Ok(gate.clone()),
        [] => Err(unknown(format!(
            "No supported required {} gate was established for trunk {trunk}; inspect event, job, checkout, runtime and stage wiring",
            if post { "post-merge" } else { "pre-merge" }
        ))),
        _ => Err(unknown(
            "Multiple applicable OpDev gates are ambiguous; inspect the intended required job identities instead of selecting a convenient match",
        )),
    }
}

fn visit(
    documents: &BTreeMap<String, Value>,
    path: &str,
    post: bool,
    stack: &mut Vec<String>,
    gates: &mut Vec<String>,
    visits: &mut usize,
) -> Result<()> {
    *visits += 1;
    if *visits > 128 {
        return Err(unknown(
            "GitHub reusable workflow expansion exceeds 128 visits",
        ));
    }
    if stack.iter().any(|p| p == path) {
        return Err(invalid("GitHub reusable workflow cycle"));
    }
    if stack.len() >= MAX_DEPTH {
        return Err(unknown("GitHub reusable workflow depth exceeds 8"));
    }
    if gates.len() > 256 {
        return Err(unknown("GitHub gate expansion exceeds 256 candidates"));
    }
    let workflow = documents
        .get(path)
        .ok_or_else(|| invalid("Local reusable workflow is missing"))?;
    stack.push(path.into());
    for (name, job) in workflow["jobs"]
        .as_object()
        .ok_or_else(|| invalid("Expected jobs mapping"))?
    {
        if !hint(job) || !enabled(job.get("if"), post)? {
            continue;
        }
        required(job)?;
        if job.get("needs").is_some() || job.get("strategy").is_some() {
            return Err(unknown(format!(
                "{path} job {name}: dependency collectors/matrices need provider review; skipped predecessors or optional legs cannot be assumed successful"
            )));
        }
        if let Some(target) = job.get("uses") {
            let target = target
                .as_str()
                .ok_or_else(|| invalid("Reusable workflow reference must be a string"))?;
            let target = target.strip_prefix("./").ok_or_else(|| unknown("Remote/dynamic reusable workflows are not fetched; inspect their exact effective configuration"))?;
            if !target.starts_with(".github/workflows/")
                || target[DIRECTORY.len() + 1..].contains(['/', '\\', ':', '$', '@'])
            {
                return Err(unknown(
                    "Reusable workflow must be an explicit same-commit file directly in .github/workflows",
                ));
            }
            if job.get("with").is_some() || job.get("secrets").is_some() {
                return Err(unknown(
                    "Parameterized/secret-bearing reuse needs provider review; no input substitution is guessed",
                ));
            }
            let called = documents
                .get(target)
                .ok_or_else(|| invalid("Local reusable workflow is missing"))?;
            if event(called, "workflow_call").is_none() {
                return Err(invalid(
                    "Referenced workflow does not declare workflow_call",
                ));
            }
            visit(documents, target, post, stack, gates, visits)?;
        } else if job_gate(workflow, job, post)? {
            gates.push(format!("{path}#jobs.{name}"));
        }
    }
    stack.pop();
    Ok(())
}

fn lines(script: &str) -> Vec<&str> {
    script
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .collect()
}

fn normalized(script: &str) -> String {
    lines(script).join("\n")
}

fn directory(value: Option<&Value>) -> Result<String> {
    let value = match value {
        None => ".",
        Some(value) => value
            .as_str()
            .ok_or_else(|| unknown("Working/checkout directory is not a static string"))?,
    }
    .replace('\\', "/");
    let value = value.strip_prefix("./").unwrap_or(&value);
    if value == "." {
        return Ok(String::new());
    }
    if value.is_empty()
        || value
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
        || value.contains(['$', ':', '*', '?'])
    {
        return Err(unknown(
            "Dynamic or escaping working/checkout directories need review",
        ));
    }
    Ok(value.into())
}

fn same_checkout(step: &Value, post: bool) -> bool {
    if step["with"].get("repository").is_some() {
        return false;
    }
    if step["with"].get("sparse-checkout").is_some() || step["with"].get("filter").is_some() {
        return false;
    }
    match step["with"]["ref"].as_str().map(expression) {
        None | Some("github.sha") => post,
        Some("github.event.pull_request.head.sha") => !post,
        Some("github.event.pull_request.head.sha || github.sha") => true,
        _ => false,
    }
}

fn source_checkout(step: &Value) -> bool {
    step["with"]["repository"]
        .as_str()
        .is_some_and(|s| s.split('/').count() == 2 && !s.contains(['$', ' ', ':']))
        && step["with"]["ref"]
            .as_str()
            .is_some_and(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        && step["with"].get("sparse-checkout").is_none()
        && step["with"].get("filter").is_none()
}

fn built(script: &str) -> Option<&'static str> {
    match normalized(script).as_str() {
        "cargo build --locked -p opdev-cli" | "cargo build -p opdev-cli --locked" => Some("debug"),
        "cargo build --release --locked -p opdev-cli"
        | "cargo build --locked -p opdev-cli --release" => Some("release"),
        _ => None,
    }
}

fn reference_steps(version: &str) -> Option<Vec<Value>> {
    let rendered = crate::render_template(
        crate::GITHUB_TEMPLATE,
        &TemplateContext {
            opdev_version: version.into(),
            trunk: "main".into(),
            job_image: None,
        },
    )
    .ok()?;
    let value: Value = serde_saphyr::from_str(&rendered).ok()?;
    value["jobs"]["opdev"]["steps"].as_array().cloned()
}

fn signed_install(step: &Value) -> bool {
    let Some(version) = step["env"]["OPDEV_VERSION"].as_str() else {
        return false;
    };
    let Some(steps) = reference_steps(version) else {
        return false;
    };
    step["run"]
        .as_str()
        .zip(steps[1]["run"].as_str())
        .is_some_and(|(a, b)| normalized(a) == normalized(b))
}

fn gate(script: &str, post: bool) -> Result<Option<String>> {
    // Reviewed generated report wrapper preserves the gate's exit through report copying.
    if let Some(steps) = reference_steps("0.4.0") {
        let index = if post { 3 } else { 2 };
        if steps[index]["run"]
            .as_str()
            .is_some_and(|s| normalized(s) == normalized(script))
        {
            return Ok(Some("opdev".into()));
        }
    }
    let mut lines = lines(script);
    if lines.last() == Some(&"exit $LASTEXITCODE") {
        lines.pop();
    }
    if lines.len() != 1 {
        return Err(unknown(
            "Gate wrapper has unsupported control flow or exit handling; review the executed script rather than matching command text",
        ));
    }
    let command = lines[0].strip_prefix("& ").unwrap_or(lines[0]);
    if command.contains([';', '|', '>', '<', '`', '\n']) || command.contains("&&") {
        return Err(unknown(
            "Gate command has unsupported shell operators; failure propagation is unverified",
        ));
    }
    let args: Vec<_> = command.split_whitespace().collect();
    if args.len() < 3 || args[1] != "check" || !args.contains(&"--ci") {
        return Ok(None);
    }
    if args.contains(&"--no-exec") || args.contains(&"--plan") || args.contains(&"--delivery") {
        return Err((Outcome::Failed, "Inspection, skipped execution or delivery checks cannot stand in for this integration-stage check".into()));
    }
    if args.contains(&"--post-merge") != post {
        return Ok(None);
    }
    let mut index = 2;
    while index < args.len() {
        match args[index] {
            "--ci" | "--post-merge" | "--review-ci" => index += 1,
            "--format" | "--report" | "--review-locator" | "--review-acceptance-sha256"
                if index + 1 < args.len() =>
            {
                index += 2;
            }
            _ => {
                return Err(unknown(
                    "Unknown gate argument or scope override; inspect the actual selected source/stage",
                ));
            }
        }
    }
    let executable = args[0].replace('\\', "/");
    Ok(Some(
        executable.strip_prefix("./").unwrap_or(&executable).into(),
    ))
}

fn shell(workflow: &Value, job: &Value, step: &Value) -> Result<()> {
    let setting = step
        .get("shell")
        .or_else(|| job["defaults"]["run"].get("shell"))
        .or_else(|| workflow["defaults"]["run"].get("shell"));
    if setting.is_some_and(|v| !matches!(v.as_str(), Some("bash" | "sh" | "pwsh" | "powershell"))) {
        return Err(unknown(
            "Custom shell does not establish native failure propagation; inspect its actual exit handling",
        ));
    }
    Ok(())
}

fn environment(workflow: &Value, job: &Value, step: &Value) -> Result<()> {
    for owner in [workflow, job, step] {
        if let Some(env) = owner.get("env") {
            let env = env
                .as_object()
                .ok_or_else(|| unknown("Dynamic environment needs runtime/source review"))?;
            if env.keys().any(|key| {
                let key = key.to_ascii_uppercase();
                key.starts_with("CARGO_")
                    || key.starts_with("RUSTC")
                    || key == "RUSTFLAGS"
                    || matches!(key.as_str(), "PATH" | "ENV" | "BASH_ENV")
            }) {
                return Err(unknown(
                    "Build or shell environment overrides need explicit runtime/source review; default paths are not assumed",
                ));
            }
        }
    }
    Ok(())
}

fn job_gate(workflow: &Value, job: &Value, post: bool) -> Result<bool> {
    if !job.get("runs-on").is_some_and(|runner| {
        runner.as_str().is_some_and(|s| !s.is_empty())
            || runner.as_array().is_some_and(|labels| {
                !labels.is_empty() && labels.iter().all(|v| v.as_str().is_some())
            })
    }) {
        return Err(unknown(
            "Gate runner selection is missing or unsupported; provider execution is not established",
        ));
    }
    let steps = job["steps"]
        .as_array()
        .ok_or_else(|| invalid("Gate job needs a steps array"))?;
    if steps.len() > 512 {
        return Err(unknown("GitHub job exceeds 512 steps"));
    }
    let mut checkouts = BTreeMap::new();
    let mut runtimes = vec![];
    let mut main_checkout = false;
    let mut count = 0;
    for step in steps {
        if !step.is_object() || (step.get("run").is_some() && step.get("uses").is_some()) {
            return Err(invalid("A workflow step must select run or uses, not both"));
        }
        // Once the gate ran, ordinary retention steps cannot undo its status.
        // A second actual gate still needs inspection and is not silently chosen.
        if count > 0
            && !step["run"].as_str().is_some_and(|run| {
                lines(run)
                    .iter()
                    .any(|s| s.contains("opdev") && s.contains("check"))
            })
        {
            continue;
        }
        if !enabled(step.get("if"), post)? {
            continue;
        }
        required(step)?;
        if let Some(action) = step["uses"].as_str() {
            if action.starts_with("actions/checkout@") {
                let path = directory(step["with"].get("path"))?;
                let exact = same_checkout(step, post);
                if path.is_empty() {
                    main_checkout = exact;
                }
                checkouts.insert(path, exact || source_checkout(step));
                runtimes.clear();
            } else if !checkouts.is_empty() || !runtimes.is_empty() {
                return Err(unknown(
                    "Unresolved action between checkout, runtime build and gate; inspect its effects rather than assuming the selected source/runtime is unchanged",
                ));
            }
            continue;
        }
        let Some(run) = step["run"].as_str() else {
            return Err(invalid("Step needs run or uses"));
        };
        shell(workflow, job, step)?;
        environment(workflow, job, step)?;
        let cwd = directory(
            step.get("working-directory")
                .or_else(|| job["defaults"]["run"].get("working-directory"))
                .or_else(|| workflow["defaults"]["run"].get("working-directory")),
        )?;
        if signed_install(step) && cwd.is_empty() && main_checkout {
            runtimes.push("opdev".into());
        } else if let Some(profile) = built(run) {
            if checkouts.get(&cwd) != Some(&true) {
                return Err(unknown(
                    "Source-built OpDev needs an exact source checkout before its locked build",
                ));
            }
            let prefix = if cwd.is_empty() {
                String::new()
            } else {
                format!("{cwd}/")
            };
            runtimes.push(format!("{prefix}target/{profile}/opdev"));
            runtimes.push(format!("{prefix}target/{profile}/opdev.exe"));
        } else if lines(run)
            .iter()
            .any(|line| line.contains("opdev") && line.contains("check"))
        {
            if let Some(executable) = gate(run, post)? {
                if !main_checkout || !cwd.is_empty() {
                    return Err(unknown(
                        "Gate checkout does not establish the selected PR head or integrated trunk source at the repository root",
                    ));
                }
                if !runtimes.contains(&executable) {
                    return Err(unknown(
                        "Gate runtime is not linked to a verified release bootstrap or exact-checkout locked source build; inspect setup without replacing it automatically",
                    ));
                }
                count += 1;
                if count > 1 {
                    return Err(unknown(
                        "Multiple gate invocations in one job need explicit source/stage review",
                    ));
                }
            }
        } else {
            // Arbitrary scripts may change source, PATH or a prepared binary. Do not
            // infer their effects; setup before checkout/build remains permissible.
            if main_checkout || !runtimes.is_empty() {
                return Err(unknown(
                    "Unresolved script between checkout, runtime setup and gate; inspect its effects on source and runtime",
                ));
            }
        }
    }
    Ok(count == 1)
}
