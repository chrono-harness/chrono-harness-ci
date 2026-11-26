//! Explicit units scheduled as jobs within one parent workflow.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn lock_selected(path: Option<&str>, checkout: &str, upload: &str) -> (result: Result<bool, &'static str>)
    ensures
        result == Ok(false) <==> path.is_none(),
        result == Ok(true) <==> path.is_some() && path.unwrap()@ == ".github/workflows/actions.lock"@
                    && checkout@.len() == 0 && upload@.len() == 0,
{
    match path {
        None => Ok(false),
        Some(p) => {
            if <&str>::exec_eq(p, ".github/workflows/actions.lock") && checkout.is_empty() && upload.is_empty() {
                Ok(true)
            } else {
                Err("actions_lock requires the GitHub workflow lockfile as the sole checkout/upload source")
            }
        }
    }
}
}
#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use crate::files::file_identity;
#[cfg(not(verus_keep_ghost))]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Scope {
    Unit { unit: String },
    Collect { manifest: String },
}

#[cfg(not(verus_keep_ghost))]
impl Scope {
    pub(crate) fn short_argv(&self) -> Vec<String> {
        match self {
            Self::Unit { unit } => vec!["--unit".into(), unit.clone()],
            Self::Collect { .. } => vec!["--collect".into()],
        }
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Unit {
    pub tests: Vec<String>,
    pub report_path: String,
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_'))
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn id(s: &str) -> Result<(), String> {
    valid_id(s)
        .then_some(())
        .ok_or_else(|| format!("invalid CI unit ID: {s:?}"))
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn artifact_path(path: &str) -> Result<(), String> {
    relative_path(path)?;
    if !path.starts_with(".chrono-harness/state/") {
        return Err(
            "unit reports and collection manifests must reside in .chrono-harness/state/".into(),
        );
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn path_overlap(a: &str, b: &str) -> bool {
    let (a, b) = (Path::new(a), Path::new(b));
    a.starts_with(b) || b.starts_with(a)
}

#[cfg(not(verus_keep_ghost))]
pub const SCHEMA: &str = "chrono-github-units/v1";
#[cfg(not(verus_keep_ghost))]
pub const FULL_SCHEMA: &str = "chrono-github-units/v2";

/// Locate the configured runner, including a file linked to the host's package manager.
/// Parent directories are part of the host checkout and must be real directories.
#[cfg(not(verus_keep_ghost))]
pub(crate) fn installed_runner_path(root: &Path, runner: &str) -> Result<PathBuf, String> {
    crate::files::relative_path(runner)?;
    let path = Path::new(runner);
    if let Some(parent) = path
        .parent()
        .and_then(Path::to_str)
        .filter(|p| !p.is_empty())
    {
        no_symlink_parents(root, parent)?;
    }
    Ok(root.join(path))
}
#[cfg(not(verus_keep_ghost))]
pub fn installed_runner_sha256(root: &Path, runner: &str) -> Result<String, String> {
    Ok(file_identity(&installed_runner_path(root, runner)?)?.0)
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullContexts {
    pub collection: String,
    pub units: BTreeMap<String, String>,
}

#[cfg(not(verus_keep_ghost))]
fn full_contexts_present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<FullContexts>, D::Error> {
    FullContexts::deserialize(d).map(Some)
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::string_present"
    )]
    pub actions_lock: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirements_program: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persistent_cache: Option<super::cache::Config>,
    pub job_gating: super::gating::Config,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "full_contexts_present"
    )]
    pub full_contexts: Option<FullContexts>,
    pub schema: String,
    pub collection: super::Config,
    pub units: BTreeMap<String, Workflow>,
    pub gather: Gather,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    pub name: String,
    pub runs_on: String,
    pub timeout_minutes: u32,
    pub bootstrap: Vec<String>,
    pub context_path: String,
    pub artifact_directory: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadRetry {
    pub max_attempts: u32,
    pub delay_seconds: u64,
    pub http_statuses: Vec<u16>,
}

#[cfg(not(verus_keep_ghost))]
fn download_retry_present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<DownloadRetry>, D::Error> {
    DownloadRetry::deserialize(d).map(Some)
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gather {
    pub program: String,
    pub inherit_environment: Vec<String>,
    pub credential_environment: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
    pub manifest_path: String,
    pub download_directory: String,
    pub report_path: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "download_retry_present"
    )]
    pub download_retry: Option<DownloadRetry>,
}

#[cfg(not(verus_keep_ghost))]
impl Config {
    pub fn workflow(&self, unit: Option<&str>) -> Result<super::Config, String> {
        let mut c = self.collection.clone();
        if let Some(unit) = unit {
            let u = self.units.get(unit).ok_or("unregistered workflow unit")?;
            c.name = u.name.clone();
            c.runs_on = u.runs_on.clone();
            c.timeout_minutes = u.timeout_minutes;
            c.bootstrap = u.bootstrap.clone();
            c.context_path = u.context_path.clone();
            c.artifact_directory = u.artifact_directory.clone();
        }
        Ok(c)
    }

    pub fn scope(&self, unit: Option<&str>) -> Scope {
        match unit {
            Some(unit) => Scope::Unit { unit: unit.into() },
            None => Scope::Collect {
                manifest: self.gather.manifest_path.clone(),
            },
        }
    }
}

#[cfg(not(verus_keep_ghost))]
pub fn validate(c: &Config) -> Result<(), String> {
    if let Some(cache) = &c.persistent_cache {
        let mut jobs: BTreeSet<_> = c.units.keys().map(|id| super::gating::job_id(id)).collect();
        jobs.insert("aggregate".into());
        jobs.insert("detect".into());
        super::cache::validate(cache, &jobs)?;
    }
    if !matches!(c.schema.as_str(), SCHEMA | FULL_SCHEMA)
        || c.units.is_empty()
        || c.collection.initial_inventory.is_some()
    {
        return Err("unit workflows require explicit units and the scoped initial contract".into());
    }
    match (&c.full_contexts, c.schema.as_str()) {
        (None, SCHEMA) => {}
        (Some(contexts), FULL_SCHEMA)
            if contexts.units.keys().collect::<Vec<_>>() == c.units.keys().collect::<Vec<_>>() =>
        {
            let mut inputs = vec![(&contexts.collection, c.workflow(None)?)];
            for (id, path) in &contexts.units {
                inputs.push((path, c.workflow(Some(id))?));
            }
            for (path, workflow) in inputs {
                crate::units::artifact_path(path)?;
                if !Path::new(path).starts_with(&workflow.artifact_directory)
                    || crate::units::path_overlap(path, &workflow.context_path)
                {
                    return Err("full context must be a separate declared upload input".into());
                }
            }
        }
        _ => {
            return Err(
                "v1 units are scoped; v2 requires exact full context mappings and short workflows"
                    .into(),
            );
        }
    }
    if let Some(contexts) = &c.full_contexts {
        let outputs: Vec<_> = std::iter::once(&c.collection.context_path)
            .chain(c.units.values().map(|u| &u.context_path))
            .chain([
                &c.gather.manifest_path,
                &c.gather.report_path,
                &c.gather.download_directory,
            ])
            .collect();
        for input in std::iter::once(&contexts.collection).chain(contexts.units.values()) {
            if outputs
                .iter()
                .any(|output| crate::units::path_overlap(input, output))
            {
                return Err("full context overlaps provider publication".into());
            }
        }
        let mut roots = vec![&c.collection.artifact_directory];
        for unit in c.units.values() {
            if roots
                .iter()
                .any(|root| crate::units::path_overlap(&unit.artifact_directory, root))
            {
                return Err("full units require separate upload ownership".into());
            }
            roots.push(&unit.artifact_directory);
        }
    }
    super::gating::validate(c, &c.job_gating)?;
    if !lock_selected(
        c.actions_lock.as_deref(),
        &c.collection.checkout_action,
        &c.collection.upload_artifact_action,
    )? {
        super::validate_action_pins(&c.collection)?;
    }
    super::validate_policy(&c.collection, c.schema == FULL_SCHEMA)?;
    let mut names = BTreeSet::from([c.collection.name.clone()]);
    let mut contexts = BTreeSet::from([c.collection.context_path.clone()]);
    for (id, unit) in &c.units {
        crate::units::id(id)?;
        super::validate_policy(&c.workflow(Some(id))?, c.schema == FULL_SCHEMA)?;
        if !names.insert(unit.name.clone()) || !contexts.insert(unit.context_path.clone()) {
            return Err("unit job name/context collision".into());
        }
    }
    crate::observed::CommandSpec {
        program: c.gather.program.clone(),
        args: vec![],
        env: BTreeMap::new(),
        timeout_seconds: c.gather.timeout_seconds,
        output_limit_bytes: c.gather.output_limit_bytes,
    }
    .validate()?;
    let inherited: BTreeSet<_> = c.gather.inherit_environment.iter().collect();
    let credentials: BTreeSet<_> = c.gather.credential_environment.iter().collect();
    if inherited.len() != c.gather.inherit_environment.len()
        || credentials.len() != c.gather.credential_environment.len()
        || c.gather
            .inherit_environment
            .iter()
            .chain(c.gather.environment.keys())
            .any(|s| s.is_empty() || s.contains(['=', '\0']))
        || c.gather.environment.values().any(|s| s.contains('\0'))
        || credentials.iter().any(|s| !inherited.contains(s))
        || c.gather.environment.keys().any(|s| credentials.contains(s))
        || ["GH_TOKEN", "GITHUB_TOKEN"].iter().any(|key| {
            inherited.iter().any(|s| s.as_str() == *key)
                && !credentials.iter().any(|s| s.as_str() == *key)
        })
    {
        return Err("invalid transport environment or credential omission declaration".into());
    }
    if let Some(retry) = &c.gather.download_retry {
        let statuses: BTreeSet<_> = retry.http_statuses.iter().copied().collect();
        if !(1..=10).contains(&retry.max_attempts)
            || retry.delay_seconds == 0
            || retry.delay_seconds > 60
            || retry.delay_seconds >= c.gather.timeout_seconds
            || statuses.is_empty()
            || statuses.len() != retry.http_statuses.len()
            || statuses.iter().any(|status| !(400..=599).contains(status))
        {
            return Err("invalid bounded artifact download retry policy".into());
        }
    }
    for path in [&c.gather.manifest_path, &c.gather.report_path] {
        crate::units::artifact_path(path)?;
        if !path.starts_with(&c.collection.artifact_directory) || !contexts.insert(path.clone()) {
            return Err(
                "collection output collision or outside uploaded artifact directory".into(),
            );
        }
    }
    if !c.gather.download_directory.ends_with('/') {
        return Err("download_directory must end /".into());
    }
    crate::units::artifact_path(c.gather.download_directory.trim_end_matches('/'))?;
    if !c
        .gather
        .download_directory
        .starts_with(&c.collection.artifact_directory)
        || contexts
            .iter()
            .any(|p| p.starts_with(&c.gather.download_directory))
    {
        return Err("download directory overlaps context/collection outputs".into());
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub(super) struct ScopedProfile {
    pub schema: String,
    pub judge: crate::observed::CommandSpec,
    pub units: Value,
    pub report_path: String,
    pub retained_reference: bool,
}
#[cfg(not(verus_keep_ghost))]
pub(super) fn scoped_profile(root: &Path, c: &Config) -> Result<ScopedProfile, String> {
    crate::runtime_session::Session::query(
        root,
        &c.collection.runner,
        "retained-artifacts-v1",
        json!({"operation":"scoped-check-profile","path":c.collection.check_config}),
    )
}

#[cfg(not(verus_keep_ghost))]
pub(super) struct Profile {
    pub units: BTreeMap<String, Unit>,
    pub report_path: String,
    pub judge: Option<crate::observed::CommandSpec>,
}
#[cfg(not(verus_keep_ghost))]
pub(super) fn profile(root: &Path, c: &Config) -> Result<Profile, String> {
    let profile = if c.schema == FULL_SCHEMA {
        let raw = crate::runtime_session::Session::selected_config(
            root,
            &c.collection.runner,
            &c.collection.check_config,
        )?;
        let block: Option<Value> = crate::runtime_session::Session::query(
            root,
            &c.collection.runner,
            "judge-evidence-v1",
            json!({"operation":"full-execution-units","profile":raw}),
        )?;
        let block = block.ok_or("full provider requires registered full units")?;
        Profile {
            units: serde_json::from_value(block["units"].clone()).map_err(|e| e.to_string())?,
            report_path: block["report_path"]
                .as_str()
                .ok_or("full collection report path")?
                .into(),
            judge: None,
        }
    } else {
        let profile = scoped_profile(root, c)?;
        if profile.schema != "chrono-ci-check/v3" {
            return Err("v1 unit provider requires chrono-ci-check/v3".into());
        }
        Profile {
            units: serde_json::from_value(profile.units).map_err(|e| e.to_string())?,
            report_path: profile.report_path,
            judge: Some(profile.judge),
        }
    };
    let units = &profile.units;
    if units.keys().collect::<Vec<_>>() != c.units.keys().collect::<Vec<_>>() {
        return Err("workflow units differ from check profile assignments".into());
    }
    for (id, unit) in units {
        crate::units::artifact_path(&unit.report_path)?;
        if !Path::new(&unit.report_path).starts_with(&c.units[id].artifact_directory)
            || crate::units::path_overlap(&unit.report_path, &c.units[id].context_path)
        {
            return Err("unit report must be uploaded and separate from context".into());
        }
    }
    if !Path::new(&profile.report_path).starts_with(&c.collection.artifact_directory)
        || [
            &c.collection.context_path,
            &c.gather.manifest_path,
            &c.gather.report_path,
            &c.gather.download_directory,
        ]
        .iter()
        .any(|path| crate::units::path_overlap(path, &profile.report_path))
    {
        return Err("collection report collides with provider outputs".into());
    }
    if let Some(contexts) = &c.full_contexts {
        for input in std::iter::once(&contexts.collection).chain(contexts.units.values()) {
            if crate::units::path_overlap(input, &profile.report_path)
                || units
                    .values()
                    .any(|u| crate::units::path_overlap(input, &u.report_path))
            {
                return Err("full context overlaps check report publication".into());
            }
        }
    }
    Ok(profile)
}

#[cfg(not(verus_keep_ghost))]
pub fn render(c: &Config, config_path: &str) -> Result<BTreeMap<String, String>, String> {
    render_from(c, config_path, |_| {
        Err("unit rendering requires the registered actions_lock input".into())
    })
}

#[cfg(not(verus_keep_ghost))]
pub fn render_from(
    c: &Config,
    config_path: &str,
    read: impl FnOnce(&str) -> Result<Vec<u8>, String>,
) -> Result<BTreeMap<String, String>, String> {
    validate(c)?;
    relative_path(config_path)?;
    if !config_path.starts_with(".chrono-harness/ci/") {
        return Err("unit source must belong to host CI configuration".into());
    }
    if let Some(path) = &c.actions_lock {
        let (checkout, upload) =
            super::actions_lock::resolve(&read(path)?, &c.collection.workflow_path)?;
        let mut resolved = c.clone();
        resolved.collection.checkout_action = checkout;
        resolved.collection.upload_artifact_action = upload;
        return super::gating::render(&resolved, config_path);
    }
    super::gating::render(c, config_path)
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn read_lock(root: &Path, path: &str) -> Result<Vec<u8>, String> {
    fs::read(no_symlink_parents(root, path)?).map_err(|e| format!("{path}: {e}"))
}

#[cfg(not(verus_keep_ghost))]
fn projection_changes(
    root: &Path,
    path: &str,
    c: &Config,
    verify: bool,
    read: impl FnOnce(&str) -> Result<Vec<u8>, String>,
) -> Result<Vec<(String, String)>, String> {
    profile(root, c)?;
    super::cache::validate_registry(root, c.persistent_cache.as_ref())?;
    let outputs = render_from(c, path, read)?;
    let mut writes = vec![];
    for (path, bytes) in &outputs {
        if !output_preflight(root, path, bytes, super::gating::MARKER)? {
            if verify {
                return Err(format!("generated unit workflow drift: {path}"));
            }
            writes.push((path.clone(), bytes.clone()));
        }
    }
    Ok(writes)
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn verify_fixed(
    root: &Path,
    path: &str,
    c: &Config,
    read: impl FnOnce(&str) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    projection_changes(root, path, c, true, read).map(|_| ())
}

#[cfg(not(verus_keep_ghost))]
pub fn generate(root: &Path, path: &str, c: &Config, verify: bool) -> Result<bool, String> {
    let writes = projection_changes(root, path, c, verify, |input| read_lock(root, input))?;
    // Preflight every output before modifying any of them; I/O failure is reported, never atomicity claimed.
    for (path, bytes) in &writes {
        write_file(root, path, bytes.as_bytes())?;
    }
    Ok(!writes.is_empty())
}

#[cfg(not(verus_keep_ghost))]
pub fn init(root: &Path, incoming: Config) -> Result<bool, String> {
    let path = ".chrono-harness/ci/units.json";
    let existing = no_symlink_parents(root, path)?.exists();
    let c: Config = if existing {
        decode(&fs::read(root.join(path)).map_err(|e| e.to_string())?)?
    } else {
        incoming
    };
    if !existing {
        projection_changes(root, path, &c, false, |input| read_lock(root, input))?;
        write_file(
            root,
            path,
            &serde_json::to_vec_pretty(&c).map_err(|e| e.to_string())?,
        )?;
    }
    Ok(generate(root, path, &c, false)? || !existing)
}

#[cfg(not(verus_keep_ghost))]
pub fn prepare(
    root: &Path,
    path: &str,
    c: &Config,
    unit: Option<&str>,
    event: &str,
    payload: &Value,
    revision: &str,
) -> Result<Value, String> {
    prepare_scope(
        root,
        path,
        c,
        unit,
        Some(c.scope(unit)),
        event,
        payload,
        revision,
    )
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn prepare_all(
    root: &Path,
    path: &str,
    c: &Config,
    event: &str,
    payload: &Value,
    revision: &str,
) -> Result<Value, String> {
    prepare_scope(root, path, c, None, None, event, payload, revision)
}
#[cfg(not(verus_keep_ghost))]
#[allow(clippy::too_many_arguments)]
fn prepare_scope(
    root: &Path,
    path: &str,
    c: &Config,
    unit: Option<&str>,
    scope: Option<Scope>,
    event: &str,
    payload: &Value,
    revision: &str,
) -> Result<Value, String> {
    generate(root, path, c, true)?;
    let profile = profile(root, c)?;
    let w = c.workflow(unit)?;
    let detection = super::gating::native_detection(root, path, c, event, payload, revision)?;
    let mut context = super::prepare_policy_fixed(
        root,
        &w,
        event,
        payload,
        revision,
        c.schema == FULL_SCHEMA,
        Some(&detection),
    )?;
    context["detection"] = detection;
    context["native_run"] =
        serde_json::to_value(super::gating::native_run(unit)?).map_err(|e| e.to_string())?;
    let argv = context["canonical_argv"]
        .as_array_mut()
        .ok_or("missing canonical command")?;
    if let Some(scope) = &scope {
        match scope {
            Scope::Unit { unit } => argv.extend([json!("--unit"), json!(unit)]),
            Scope::Collect { .. } => argv.push(json!("--collect")),
        }
    }
    context["scope"] = serde_json::to_value(scope).map_err(|e| e.to_string())?;
    context["provider_sha256"] = json!(file_identity(&root.join(path))?.0);
    context["check_config_sha256"] = json!(file_identity(&root.join(&w.check_config))?.0);
    let candidate_owned = context["candidate"]
        .as_str()
        .ok_or("candidate missing")?
        .to_string();
    let candidate = candidate_owned.as_str();
    let facts = event_git::EventGit::new(root, &w, candidate)?;
    for input in [path, w.check_config.as_str(), w.workflow_path.as_str()]
        .into_iter()
        .chain(c.actions_lock.as_deref())
    {
        if facts.show(candidate, input)?.as_bytes()
            != fs::read(root.join(input)).map_err(|e| e.to_string())?
        {
            return Err(format!(
                "unit provider input differs from fixed candidate: {input}"
            ));
        }
    }
    let pins = if let Some(judge) = &profile.judge {
        let judge = crate::files::resolve_program(root, &judge.program, None)?;
        json!({"runner_sha256":installed_runner_sha256(root, &w.runner)?,"judge_sha256":file_identity(&judge)?.0})
    } else {
        super::full::executable_pins(root, &w.check_config, &w.runner)?
    };
    context["executables"] = pins;
    if let Some(contexts) = &c.full_contexts {
        let path = match unit {
            Some(unit) => &contexts.units[unit],
            None => &contexts.collection,
        };
        let raw = fs::read(no_symlink_parents(root, path)?)
            .map_err(|e| format!("missing registered exact full context {path}: {e}"))?;
        let ctx = super::full::context_input(&raw)?;
        if ctx["base"] != context["base"]
            || ctx["candidate"] != context["candidate"]
            || context["initial"] == true
        {
            return Err("exact full context disagrees with automatic event endpoints".into());
        }
        context["full_context"] = json!({"path":path,"sha256":crate::files::sha256(&raw),"semantic_digest":crate::codec::digest(&ctx)?});
    }
    Ok(context)
}
