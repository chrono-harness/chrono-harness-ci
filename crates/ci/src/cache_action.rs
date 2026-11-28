//! Project registered caches around a host's own build inside an existing workflow.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {
pub open spec fn path_part(path: Seq<u8>, offset: int, part: Seq<u8>) -> bool {
    0 <= offset && offset + part.len() <= path.len()
        && forall|i: int| 0 <= i < part.len() ==> path[offset+i] == part[i]
}

pub(crate) fn has_part(path: &[u8], offset: usize, part: &[u8]) -> (valid: bool)
    ensures valid == path_part(path@, offset as int, part@),
{
    if offset > path.len() || part.len() > path.len() - offset { return false; }
    let mut i = 0;
    while i < part.len()
        invariant i <= part.len(), offset <= path.len(), part.len() <= path.len() - offset,
            forall|j: int| 0 <= j < i ==> path@[offset+j] == part@[j],
        decreases part.len() - i,
    {
        if path[offset + i] != part[i] { return false; }
        i += 1;
    }
    true
}

pub fn registration(schema: &str, path: &[u8]) -> (valid: bool)
    ensures valid == (schema@ == "chrono-github-cache-actions/v1"@
        && path_part(path@, 0, ".chrono-harness/ci/".spec_bytes())),
{
    <&str>::exec_eq(schema,"chrono-github-cache-actions/v1")
        && has_part(path, 0, ".chrono-harness/ci/".as_bytes())
}

/// Both key iterators come from BTreeMap and use the same ordering.
pub fn matching_jobs(native: &[&str], caches: &[&str]) -> (valid: bool)
    ensures valid == (native.len() > 0 && native.len() == caches.len()
        && forall|i: int| 0 <= i < native.len() ==> native@[i]@ == caches@[i]@),
{
    if native.len() == 0 || native.len() != caches.len() { return false; }
    let mut i = 0;
    while i < native.len()
        invariant native.len() > 0, native.len() == caches.len(), i <= native.len(),
            forall|j: int| 0 <= j < i ==> native@[j]@ == caches@[j]@,
        decreases native.len() - i,
    {
        if !<&str>::exec_eq(native[i], caches[i]) { return false; }
        i += 1;
    }
    true
}

pub enum RegistrationError { MissingJob, Downloads }

pub fn invalid_configuration(native: &[&str], caches: &[&str], downloads: bool)
    -> (error: Option<RegistrationError>)
    ensures ({
        let same = native.len() > 0 && native.len() == caches.len()
          && forall|i: int| 0 <= i < native.len() ==> native@[i]@ == caches@[i]@;
        match error {
          Some(RegistrationError::MissingJob) => !same,
          Some(RegistrationError::Downloads) => same && downloads,
          None => same && !downloads,
        }
    }),
{
    if !matching_jobs(native, caches) { Some(RegistrationError::MissingJob) }
    else if downloads { Some(RegistrationError::Downloads) }
    else { None }
}

pub fn output_path(path: &[u8], first_writer: bool) -> (valid: bool)
    ensures valid == (first_writer && path_part(path@, 0, ".github/actions/".spec_bytes())
        && path_part(path@, path.len() - "/action.yml".spec_bytes().len(), "/action.yml".spec_bytes())),
{
    let suffix = "/action.yml".as_bytes();
    first_writer && path.len() >= suffix.len() && has_part(path,0,".github/actions/".as_bytes())
        && has_part(path, path.len() - suffix.len(), suffix)
}
}

#[cfg(not(verus_keep_ghost))]
use crate::{
    OutputChange, cache, files::relative_path, output_change, output_preflight, verified_literals,
    write_file,
};
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[cfg(not(verus_keep_ghost))]
pub(crate) const SCHEMA: &str = "chrono-github-cache-actions/v1";
#[cfg(not(verus_keep_ghost))]
const MARKER: &str = "# chrono-ci: owned native cache action/v1\n";

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    schema: String,
    jobs: BTreeMap<String, NativeJob>,
    persistent_cache: cache::Config,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeJob {
    action_path: String,
    command: Vec<String>,
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn generate(
    root: &Path,
    config_path: &str,
    cfg: &Config,
    verify: bool,
) -> Result<bool, String> {
    if !registration(&cfg.schema, config_path.as_bytes()) {
        return Err("native cache actions require a host CI registration".into());
    }
    let native_jobs: Vec<_> = cfg.jobs.keys().map(String::as_str).collect();
    let cache_jobs: Vec<_> = cfg
        .persistent_cache
        .jobs
        .keys()
        .map(String::as_str)
        .collect();
    match invalid_configuration(
        &native_jobs,
        &cache_jobs,
        cfg.persistent_cache.downloads.is_some(),
    ) {
        Some(RegistrationError::MissingJob) => {
            return Err("each native cache action requires its registered job".into());
        }
        Some(RegistrationError::Downloads) => return Err(
            "register native download directories as planner caches or use the workflow provider"
                .into(),
        ),
        None => {}
    }
    let jobs = cfg.jobs.keys().cloned().collect();
    cache::validate(&cfg.persistent_cache, &jobs)?;
    cache::validate_registry(root, Some(&cfg.persistent_cache))?;
    let mut outputs = BTreeSet::new();
    let mut changed = Vec::new();
    for (job, native) in &cfg.jobs {
        let path = &native.action_path;
        relative_path(path)?;
        if !output_path(path.as_bytes(), outputs.insert(path)) {
            return Err(format!(
                "expected a distinct .github/actions/*/action.yml for {job}"
            ));
        }
        let args = native
            .command
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if !verified_literals::valid_cache_preparation(&args) {
            return Err(format!(
                "expected a nonempty literal native command for {job}"
            ));
        }
        let cmd = args
            .iter()
            .map(|arg| crate::shell(arg))
            .collect::<Vec<_>>()
            .join(" ");
        let label = "Run registered native command";
        let command_step = format!(
            "      - name: {label}\n        shell: bash\n        run: |\n          {cmd}\n"
        );
        // Existing projection supplies restore, recovery, original command, seal and save steps.
        let steps = cache::project(
            Some(&cfg.persistent_cache),
            job,
            command_step,
            "",
            label,
            label,
            None,
            None,
            None,
        )?;
        let name = crate::scalar(job);
        let desc = crate::scalar(&cfg.persistent_cache.jobs[job].output_use);
        let action = format!(
            "{MARKER}name: {name}\ndescription: {desc}\nruns:\n  using: composite\n  steps:\n{steps}"
        );
        match output_change(output_preflight(root, path, &action, MARKER)?, verify) {
            OutputChange::Keep => {}
            OutputChange::Drift => {
                return Err(format!("generated native cache action drift: {path}"));
            }
            OutputChange::Write => changed.push((path, action)),
        }
    }
    for (path, action) in &changed {
        write_file(root, path, action.as_bytes())?;
    }
    Ok(!changed.is_empty())
}
