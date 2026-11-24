//! Project registered caches around a host's own build inside an existing workflow.
use crate::{cache, files::relative_path, output_preflight, verified_literals, write_file};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub(crate) const SCHEMA: &str = "chrono-github-cache-actions/v1";
const MARKER: &str = "# chrono-ci: owned native cache action/v1\n";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    schema: String,
    jobs: BTreeMap<String, NativeJob>,
    persistent_cache: cache::Config,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeJob {
    action_path: String,
    command: Vec<String>,
}

pub(crate) fn generate(
    root: &Path,
    config_path: &str,
    cfg: &Config,
    verify: bool,
) -> Result<bool, String> {
    if cfg.schema != SCHEMA || !config_path.starts_with(".chrono-harness/ci/") {
        return Err("native cache actions require a host CI registration".into());
    }
    let jobs = cfg.jobs.keys().cloned().collect::<BTreeSet<_>>();
    if jobs.is_empty() || jobs != cfg.persistent_cache.jobs.keys().cloned().collect() {
        return Err("each native cache action requires its registered job".into());
    }
    if cfg.persistent_cache.downloads.is_some() {
        return Err(
            "register native download directories as planner caches or use the workflow provider"
                .into(),
        );
    }
    cache::validate(&cfg.persistent_cache, &jobs)?;
    cache::validate_registry(root, Some(&cfg.persistent_cache))?;
    let mut outputs = BTreeSet::new();
    let mut changed = Vec::new();
    for (job, native) in &cfg.jobs {
        let path = &native.action_path;
        relative_path(path)?;
        if !path.starts_with(".github/actions/")
            || !path.ends_with("/action.yml")
            || !outputs.insert(path)
        {
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
        if !output_preflight(root, path, &action, MARKER)? {
            if verify {
                return Err(format!("generated native cache action drift: {path}"));
            }
            changed.push((path, action));
        }
    }
    for (path, action) in &changed {
        write_file(root, path, action.as_bytes())?;
    }
    Ok(!changed.is_empty())
}
