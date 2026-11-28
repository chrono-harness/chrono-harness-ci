//! Fixed full-governance context transport; the configured judges own admission.
#[cfg(not(verus_keep_ghost))]
use super::{
    OutputChange, action, full_oid, output_change, output_preflight, scalar, shell, write_file,
};
#[cfg(not(verus_keep_ghost))]
use crate::units::Scope;
#[cfg(not(verus_keep_ghost))]
use crate::{
    codec::decode,
    files::{no_symlink_parents, relative_path, sha256},
    units::{self, path_overlap as overlap},
};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Deserializer, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{fs, path::Path};

#[cfg(not(verus_keep_ghost))]
const MARKER: &str = "# chrono-ci: owned github-full-ci/v1\n";
#[cfg(not(verus_keep_ghost))]
const ADOPTED: &str = ".chrono-harness/ci/full.json";

#[cfg(not(verus_keep_ghost))]
fn marker(path: &str) -> String {
    format!("{MARKER}# chrono-ci: full source {}\n", scalar(path))
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub workflow_path: String,
    pub name: String,
    pub runs_on: String,
    pub checkout_action: String,
    pub upload_artifact_action: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_hidden_files: bool,
    pub timeout_minutes: u32,
    pub bootstrap: Vec<String>,
    pub runner: String,
    pub check_config: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "scope_present"
    )]
    pub scope: Option<Scope>,
    pub context_path: String,
    pub preparation_path: String,
    pub artifact_directory: String,
}

#[cfg(not(verus_keep_ghost))]
fn scope_present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Scope>, D::Error> {
    Scope::deserialize(d).map(Some)
}

#[cfg(not(verus_keep_ghost))]
fn literal(value: &str) -> Result<(), String> {
    if !crate::verified_literals::valid_literal(value.as_bytes()) {
        return Err("full CI settings must be literal single-line values".into());
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn validate(c: &Config) -> Result<(), String> {
    if !workflow_header(
        &c.schema,
        &c.name,
        &c.runs_on,
        c.timeout_minutes,
        c.bootstrap.first().map(String::as_str),
    ) {
        return Err("invalid full CI schema/name/runner/bootstrap/bounds".into());
    }
    for value in [&c.name, &c.runs_on, &c.artifact_directory]
        .into_iter()
        .chain(c.bootstrap.iter())
    {
        literal(value)?;
    }
    let files = [
        &c.workflow_path,
        &c.runner,
        &c.check_config,
        &c.context_path,
        &c.preparation_path,
    ];
    for (i, path) in files.iter().enumerate() {
        relative_path(path)?;
        literal(path)?;
        if files[..i].iter().any(|other| overlap(path, other)) {
            return Err("full CI paths must have distinct ownership".into());
        }
    }
    relative_path(c.artifact_directory.trim_end_matches('/'))?;
    let policy = Path::new(".chrono-harness");
    let state = policy.join("state");
    let artifact = Path::new(&c.artifact_directory);
    let check = Path::new(&c.check_config);
    if !Path::new(&c.workflow_path).starts_with(".github/workflows")
        || !c.workflow_path.ends_with(".yml")
        || !check.starts_with(policy)
        || check == policy
        || check.starts_with(&state)
        || !artifact.starts_with(&state)
        || !c.artifact_directory.ends_with('/')
        || c.artifact_directory.contains(['*', '?', '[', ']', '!'])
        || [&c.context_path, &c.preparation_path]
            .iter()
            .any(|p| !Path::new(p).starts_with(artifact) || Path::new(p) == artifact)
        || [&c.workflow_path, &c.runner, &c.check_config]
            .iter()
            .any(|p| Path::new(p).starts_with(artifact))
    {
        return Err("invalid full CI policy/artifact ownership".into());
    }
    match &c.scope {
        Some(Scope::Unit { unit }) => units::id(unit)?,
        Some(Scope::Collect { manifest }) => {
            units::artifact_path(manifest)?;
            literal(manifest)?;
            if artifact.starts_with(manifest) || files.iter().any(|p| overlap(manifest, p)) {
                return Err("full CI collection manifest overlaps owned input/output paths".into());
            }
        }
        None => {}
    }
    action(&c.checkout_action, "actions/checkout")?;
    action(&c.upload_artifact_action, "actions/upload-artifact")?;
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
fn source_path(c: &Config, path: &str) -> Result<(), String> {
    relative_path(path)?;
    literal(path)?;
    let source = Path::new(path);
    let policy = Path::new(".chrono-harness");
    if !source.starts_with(policy)
        || source == policy
        || source.starts_with(policy.join("state"))
        || [
            &c.workflow_path,
            &c.runner,
            &c.check_config,
            &c.context_path,
            &c.preparation_path,
        ]
        .iter()
        .any(|p| overlap(path, p))
    {
        return Err("full CI source must be a separate host policy input".into());
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
pub fn argv(c: &Config) -> Vec<String> {
    let mut args = vec![c.runner.clone(), "check".into()];
    if let Some(scope) = &c.scope {
        args.extend(scope.short_argv());
    }
    args
}

#[cfg(not(verus_keep_ghost))]
pub fn render(c: &Config, config_path: &str) -> Result<String, String> {
    validate(c)?;
    source_path(c, config_path)?;
    let check = argv(c)
        .iter()
        .map(|arg| shell(arg))
        .collect::<Vec<_>>()
        .join(" ");
    let marker = marker(config_path);
    Ok(format!(
        r#"{marker}name: {name}
on:
  workflow_dispatch:
    inputs:
      context:
        description: Exact full context JSON; provision its referenced evidence through the registered bootstrap
        required: true
        type: string
permissions:
  contents: read
jobs:
  check:
    runs-on: {runner}
    timeout-minutes: {timeout}
    steps:
      - name: Check out supplied candidate
        uses: {checkout}
        with:
          ref: ${{{{ fromJSON(inputs.context).candidate }}}}
          fetch-depth: 0
          persist-credentials: true
      - name: Bootstrap registered tools and evidence
        shell: bash
        run: |
          {bootstrap}
      - name: Canonical full harness check
        shell: bash
        env:
          CHRONO_CHECK_SOURCE: ci
          CHRONO_WORKFLOW_REVISION: ${{{{ github.workflow_sha }}}}
        run: |
          {check}
      - name: Preserve original full check evidence
        if: ${{{{ always() }}}}
        uses: {upload}
        with:
          name: chrono-full-check-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}
          path: {artifacts}
          if-no-files-found: error
{hidden_files}"#,
        name = scalar(&c.name),
        runner = scalar(&c.runs_on),
        timeout = c.timeout_minutes,
        checkout = c.checkout_action,
        upload = c.upload_artifact_action,
        hidden_files = super::artifact_hidden_files(c.include_hidden_files),
        bootstrap = c
            .bootstrap
            .iter()
            .map(|v| shell(v))
            .collect::<Vec<_>>()
            .join(" "),
        artifacts = scalar(&c.artifact_directory),
    ))
}

#[cfg(not(verus_keep_ghost))]
fn scope_preflight(root: &Path, c: &Config) -> Result<(), String> {
    if let Some(Scope::Collect { manifest }) = &c.scope {
        // Evidence provisioning and admission remain caller/bootstrap and full-core work.
        no_symlink_parents(root, manifest)?;
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn generate(root: &Path, path: &str, c: &Config, verify: bool) -> Result<bool, String> {
    let output = render(c, path)?;
    scope_preflight(root, c)?;
    let same = output_preflight(root, &c.workflow_path, &output, &marker(path))?;
    match output_change(same, verify) {
        OutputChange::Keep => Ok(false),
        OutputChange::Drift => Err(format!(
            "generated full CI workflow drift: {}",
            c.workflow_path
        )),
        OutputChange::Write => {
            write_file(root, &c.workflow_path, output.as_bytes())?;
            Ok(true)
        }
    }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn init(root: &Path, incoming: Config) -> Result<bool, String> {
    let target = no_symlink_parents(root, ADOPTED)?;
    let existing = target.exists();
    let c = if existing {
        decode(&fs::read(target).map_err(|e| e.to_string())?)?
    } else {
        incoming
    };
    let rendered = render(&c, ADOPTED)?;
    scope_preflight(root, &c)?;
    let same = output_preflight(root, &c.workflow_path, &rendered, &marker(ADOPTED))?;
    if !existing {
        write_file(
            root,
            ADOPTED,
            &(serde_json::to_string_pretty(&c).map_err(|e| e.to_string())? + "\n").into_bytes(),
        )?;
    }
    if !same {
        write_file(root, &c.workflow_path, rendered.as_bytes())?;
    }
    Ok(!existing || !same)
}

#[cfg(not(verus_keep_ghost))]
fn matching_output(root: &Path, path: &str, bytes: &[u8]) -> Result<bool, String> {
    match fs::read(no_symlink_parents(root, path)?) {
        Ok(current) if current == bytes => Ok(true),
        Ok(_) => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

/// The dispatch and automatic units consumers share exact full context parsing.
#[cfg(not(verus_keep_ghost))]
pub(crate) fn context_input(bytes: &[u8]) -> Result<Value, String> {
    let ctx: Value = decode(bytes)?;
    if ctx["schema_version"] != 2
        || !matches!(ctx["run_kind"].as_str(), Some("integration" | "delivery"))
    {
        return Err("full CI requires explicit context v2 and integration/delivery role".into());
    }
    for key in ["base", "candidate"] {
        if !ctx[key].as_str().is_some_and(full_oid) {
            return Err(format!("full context {key} OID required"));
        }
    }
    Ok(ctx)
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn executable_pins(root: &Path, config: &str, runner: &str) -> Result<Value, String> {
    let cfg = crate::runtime_session::Session::selected_config(root, runner, config)?;
    let path = cfg["registries"]["judges"]
        .as_str()
        .ok_or("full judges registry")?;
    let registry: Value =
        decode(&fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?)?;
    let raw = registry.get("judges").ok_or("full judges")?;
    let program = crate::units::installed_runner_path(root, runner)?;
    let mut session = crate::runtime_session::Session::open(root, &program, "judge-evidence-v1")?;
    session.call::<()>(json!({"operation":"validate-schedule","bindings":raw}))?;
    let bindings = raw
        .as_array()
        .ok_or("full judge bindings must be an array")?;
    let ids: std::collections::BTreeSet<_> = bindings
        .iter()
        .map(|binding| binding["id"].as_str().ok_or("full judge ID"))
        .collect::<Result<_, _>>()?;
    if ids
        != [
            "registration",
            "filemap",
            "routes",
            "projects",
            "cost",
            "mixed",
            "workflow",
        ]
        .into_iter()
        .collect()
    {
        return Err("full provider requires all seven judge bindings".into());
    }
    let mut pins = Vec::with_capacity(bindings.len());
    for binding in bindings {
        session.call::<String>(json!({"operation":"bound-judge-path","binding":binding}))?;
        pins.push(json!({"id":binding["id"],"sha256":binding["sha256"]}));
    }
    session.close()?;
    Ok(
        json!({"runner_sha256":super::units::installed_runner_sha256(root, runner)?,"judge_sha256":crate::codec::digest(&pins)?,"judges":pins}),
    )
}
/// Preserve caller context bytes separately from transport observations. Does not judge its policy.
#[cfg(not(verus_keep_ghost))]
pub fn prepare(
    root: &Path,
    path: &str,
    c: &Config,
    context: &[u8],
    workflow_revision: &str,
) -> Result<Value, String> {
    validate(c)?;
    source_path(c, path)?;
    scope_preflight(root, c)?;
    let ctx = context_input(context)?;
    let base = ctx["base"].as_str().unwrap();
    let candidate = ctx["candidate"].as_str().unwrap();
    if !full_oid(workflow_revision) {
        return Err("workflow source revision must be full OID".into());
    }
    let source = fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
    if serde_json::to_value(c).map_err(|e| e.to_string())? != decode::<Value>(&source)? {
        return Err("full CI source differs from selected configuration".into());
    }
    let facts = crate::event_git::EventGit::open(root, &c.runner, &c.check_config, candidate, true)
        .map_err(|error| format!("E_FULL_CI_GIT: {error}"))?;
    if !facts.bound {
        return Err("full CI requires candidate full config v3 Git binding".into());
    }
    let result = (|| {
        facts.verify_config(candidate)?;
        if facts.head()? != format!("{candidate}\n") {
            return Err("full CI checkout is not supplied candidate".into());
        }
        if facts.blob(candidate, path)? != source {
            return Err("full CI source differs from fixed candidate".into());
        }
        // Acquisition never replaces or derives caller context fields.
        for oid in [base, workflow_revision] {
            if !facts.available(oid)? {
                facts.fetch(oid, false)?;
            }
            if !facts.available(oid)? {
                return Err("full CI fixed commit remains missing".into());
            }
        }
        if facts.blob(workflow_revision, &c.workflow_path)? != render(c, path)?.as_bytes() {
            return Err("full CI workflow revision differs from declared projection".into());
        }
        let report = json!({"schema":"chrono-full-ci-inputs/v1","status":"prepared",
            "workflow_source_revision":workflow_revision,"config_path":path,"config_sha256":sha256(&source),
            "base":base,"candidate":candidate,"context":{"path":c.context_path,"sha256":sha256(context)},
            "canonical_argv":argv(c),"governance":"not-evaluated","parity":"unestablished"});
        Ok(report)
    })();
    let mut report: Value = result.map_err(|message: String| {
        format!(
            "E_FULL_CI_GIT: {}",
            match facts.observation() {
                Ok(observation) => json!({"message":message,"git_facts":observation}),
                Err(error) => json!({"message":message,"observation_error":error}),
            }
        )
    })?;
    let mut retained = crate::preparation::Originals::open(root, &c.runner)?;
    let original = retained.retain(
        &format!("{}preparation/", c.artifact_directory),
        "native-context",
        context,
    )?;
    retained.close()?;
    report["context"]["current_path"] = json!(c.context_path);
    report["context"]["path"] = json!(original.path);
    report["originals"] = json!([original]);
    report["git_facts"] = facts.observation()?;
    facts.close()?;
    if fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())? != source {
        return Err("full CI source changed during preparation".into());
    }
    let report_bytes =
        (serde_json::to_string_pretty(&report).map_err(|e| e.to_string())? + "\n").into_bytes();
    let context_same = matching_output(root, &c.context_path, context)?;
    let report_same = matching_output(root, &c.preparation_path, &report_bytes)?;
    if !context_same {
        write_file(root, &c.context_path, context)?;
    }
    if !report_same {
        write_file(root, &c.preparation_path, &report_bytes)?;
    }
    Ok(report)
}

use vstd::contrib::exec_spec::ExecSpecEq;

#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
verus! {
pub const SCHEMA: &'static str = "chrono-github-full-ci/v2";
pub fn workflow_header(schema: &str, name: &str, runner: &str, timeout: u32, command: Option<&str>)
    -> (valid: bool)
    ensures valid == (schema@ == "chrono-github-full-ci/v2"@ && name@ != ""@ && runner@ != ""@
        && timeout > 0 && match command { Some(first) => first@ != ""@, None => false }),
{
    <&str>::exec_eq(schema,SCHEMA)
        && !<&str>::exec_eq(name,"") && !<&str>::exec_eq(runner,"") && timeout > 0
        && match command { Some(first) => !<&str>::exec_eq(first,""), None => false }
}
}
