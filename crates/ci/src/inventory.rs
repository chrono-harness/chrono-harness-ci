//! Verify explicit Cargo/libtest groups; never discover dependencies or select tests.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn listing_status<S, E>(listing: &Result<S, E>) -> (status: &'static str)
    ensures status@ == if listing.is_ok() { "passed"@ } else { "failed"@ },
{
    if listing.is_ok() { "passed" } else { "failed" }
}

pub fn inventory_summary<S, E, P>(listing: Result<S, E>, publication: Result<(), P>)
    -> (result: Result<Result<S, E>, (Result<S, E>, P)>)
    ensures result == match publication {
        Ok(()) => Ok(listing),
        Err(error) => Err((listing, error)),
    },
{
    match publication {
        Ok(()) => Ok(listing),
        Err(error) => Err((listing, error)),
    }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::storage::Storage;
#[cfg(not(verus_keep_ghost))]
use crate::{
    codec::decode,
    files::{file_identity, no_symlink_parents, sha256},
    observed::CommandSpec,
};
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::{Duration, Instant},
};

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema: String,
    projects: String,
    project: String,
    unfiltered_action: String,
    groups: BTreeMap<String, String>,
    binaries: BTreeMap<String, Vec<String>>,
    tools: BTreeMap<String, String>,
    report_directory: String,
    timeout_seconds: f64,
    output_limit_bytes: usize,
}

#[cfg(not(verus_keep_ghost))]
fn string(value: &Value) -> Result<&str, String> {
    value
        .as_str()
        .ok_or_else(|| "inventory string field missing".into())
}

#[cfg(not(verus_keep_ghost))]
fn parse_listing(
    stdout: &str,
    stderr: &str,
    binaries: &[String],
) -> Result<BTreeSet<String>, String> {
    let observed = stderr
        .lines()
        .filter_map(|line| line.split_once("Running "))
        .map(|(_, line)| {
            let path = line
                .rsplit_once('(')
                .and_then(|(_, path)| path.strip_suffix(')'))
                .ok_or("malformed listing executable")?;
            let name = Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("malformed listing executable")?;
            let normalized = match name.rsplit_once('-') {
                Some((name, hash))
                    if crate::verified_inventory::valid_binary_hash_suffix(hash.as_bytes()) =>
                {
                    name
                }
                _ => name,
            };
            Ok(normalized.to_owned())
        })
        .collect::<Result<Vec<_>, String>>()?;
    if !crate::verified_inventory::same_name_sequence(&observed, binaries) {
        return Err(format!("listing binaries differ: {observed:?}"));
    }
    let mut suites = Vec::new();
    let mut names = Vec::new();
    for line in stdout.lines() {
        if let Some(name) = line.strip_suffix(": test") {
            names.push(name.to_owned());
        } else if let Some((tests, benches)) = line.split_once(", ").and_then(|(tests, benches)| {
            let number = |text: &str, singular: &str, plural: &str| {
                text.strip_suffix(singular)
                    .or_else(|| text.strip_suffix(plural))?
                    .parse::<usize>()
                    .ok()
            };
            Some((
                number(tests, " test", " tests")?,
                number(benches, " benchmark", " benchmarks")?,
            ))
        }) {
            if !crate::verified_inventory::valid_suite_counts(
                tests,
                benches,
                names.len(),
                names.iter().collect::<BTreeSet<_>>().len(),
            ) {
                return Err("malformed/duplicate libtest listing".into());
            }
            suites.push(std::mem::take(&mut names));
        } else if !line.trim().is_empty() {
            return Err(format!("unexpected libtest listing: {line}"));
        }
    }
    if !crate::verified_inventory::valid_complete_listing(
        names.is_empty(),
        suites.len(),
        binaries.len(),
    ) {
        return Err("incomplete libtest listing".into());
    }
    Ok(binaries
        .iter()
        .zip(suites)
        .flat_map(|(binary, suite)| {
            suite
                .into_iter()
                .map(move |name| format!("{binary}::{name}"))
        })
        .collect())
}

#[cfg(not(verus_keep_ghost))]
struct Inventory<'a> {
    root: &'a Path,
    config: Config,
    project: &'a Value,
    receipts: Vec<Value>,
    storage: Storage,
    published: bool,
}
#[cfg(not(verus_keep_ghost))]
impl Inventory<'_> {
    fn write(&self, name: &str, bytes: &[u8]) -> Result<String, String> {
        let path = format!("{}/{name}", self.config.report_directory);
        let destination = no_symlink_parents(self.root, &path)?;
        if self.published {
            self.storage
                .replace(&self.config.report_directory, name, bytes)?;
            return Ok(path);
        }
        fs::create_dir_all(
            destination
                .parent()
                .ok_or("inventory output parent missing")?,
        )
        .map_err(|e| e.to_string())?;
        fs::write(destination, bytes).map_err(|e| e.to_string())?;
        Ok(path)
    }
    fn listing(&mut self, action: &str, ignored: bool) -> Result<BTreeSet<String>, String> {
        let row = &self.project["actions"][action];
        let tool = string(&row["tool"])?;
        let program = self
            .config
            .tools
            .get(tool)
            .ok_or_else(|| format!("inventory tool missing: {tool}"))?;
        let mut argv = vec![program.clone()];
        for arg in row["argv"]
            .as_array()
            .ok_or("inventory action argv missing")?
        {
            argv.push(string(arg)?.to_owned());
        }
        if !argv.iter().any(|arg| arg == "--") {
            argv.push("--".into());
        }
        if ignored {
            argv.push("--ignored".into());
        }
        argv.push("--list".into());
        // Action IDs also name retained original files; validate before spawning.
        crate::files::relative_path(action)?;
        let label = format!("{action}{}", if ignored { "-ignored" } else { "" });
        let environment: BTreeMap<_, _> = std::env::vars().collect();
        let executable = crate::files::resolve_program(
            self.root,
            program,
            environment.get("PATH").map(String::as_str),
        )?;
        let spec = CommandSpec {
            program: executable
                .to_str()
                .ok_or("inventory executable UTF8")?
                .into(),
            args: argv[1..].to_vec(),
            env: environment,
            timeout_seconds: self.config.timeout_seconds.ceil() as u64,
            output_limit_bytes: self.config.output_limit_bytes,
        };
        let start = Instant::now();
        let process = crate::observed::run(
            self.root,
            &crate::observed::installed(self.root)?,
            &spec,
            &[],
            &file_identity(&executable)?.0,
            Duration::from_secs_f64(self.config.timeout_seconds),
        )?;
        let stdout = self.write(&format!("{label}.stdout"), &process.stdout_bytes)?;
        let stderr = self.write(&format!("{label}.stderr"), &process.stderr_bytes)?;
        self.receipts.push(
            json!({"action":action,"operation":string(&row["operation"])?,
            "argv":argv,"exit_code":process.exit_code,"error":process.failure,
            "stdout_complete":process.stdout_complete,"stderr_complete":process.stderr_complete,
            "duration_seconds":start.elapsed().as_secs_f64(),"stdout":stdout,"stderr":stderr,
            "stdout_sha256":process.stdout_sha256,"stderr_sha256":process.stderr_sha256}),
        );
        self.write(
            "receipts.json",
            &serde_json::to_vec_pretty(&self.receipts).map_err(|e| e.to_string())?,
        )?;
        if let Some(failure) = process.capture_error() {
            return Err(format!("failed listing: {label}: {failure}"));
        }
        if process.exit_code != 0 {
            return Err(format!(
                "failed listing: {label} exit {}",
                process.exit_code
            ));
        }
        parse_listing(
            std::str::from_utf8(&process.stdout_bytes).map_err(|e| e.to_string())?,
            std::str::from_utf8(&process.stderr_bytes).map_err(|e| e.to_string())?,
            self.config
                .binaries
                .get(action)
                .ok_or("inventory action binaries missing")?,
        )
    }
    fn collect(&mut self, config_bytes: &[u8], projects_bytes: &[u8]) -> Result<Value, String> {
        let all = self.config.unfiltered_action.clone();
        let groups = self.config.groups.clone();
        let complete = self.listing(&all, false)?;
        let ignored = self.listing(&all, true)?;
        let observed = groups
            .iter()
            .map(|(id, action)| Ok((id.clone(), self.listing(action, false)?)))
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        crate::verified_inventory::check_inventory_partition(
            &complete,
            &ignored,
            &observed,
            |union| format!("{:?}", union.symmetric_difference(&complete)),
            &|parts: [&str; 2]| parts.concat(),
        )?;
        let report = json!({"schema":self.config.schema,"project":self.config.project,
        "config_sha256":sha256(config_bytes),"projects_sha256":sha256(projects_bytes),
        "complete":complete,"groups":observed,"receipts":self.receipts});
        self.write(
            "inventory.json",
            &serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
        )?;
        Ok(json!({"complete":complete.len(),"groups":observed.iter()
        .map(|(id,tests)|(id,tests.len())).collect::<BTreeMap<_,_>>() }))
    }
}

/// Consume only the declared project, actions, groups, tool bindings and bounds.
#[cfg(not(verus_keep_ghost))]
pub fn execute(root: &Path, path: &str) -> Result<String, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let config_bytes = fs::read(no_symlink_parents(&root, path)?).map_err(|e| e.to_string())?;
    let mut config: Config = decode(&config_bytes)?;
    if config.schema != "chrono-workflow-inventory/v1" {
        return Err("unsupported inventory schema".into());
    }
    if Duration::try_from_secs_f64(config.timeout_seconds).is_err()
        || config.timeout_seconds <= 0.0
        || config.output_limit_bytes == 0
    {
        return Err("invalid inventory process bounds".into());
    }
    no_symlink_parents(&root, &config.report_directory)?;
    let projects_bytes =
        fs::read(no_symlink_parents(&root, &config.projects)?).map_err(|e| e.to_string())?;
    let projects: Value = decode(&projects_bytes)?;
    let rows: Vec<_> = projects["projects"]
        .as_array()
        .ok_or("inventory projects missing")?
        .iter()
        .filter(|row| row["id"].as_str() == Some(&config.project))
        .collect();
    if rows.len() != 1 || rows[0]["test_groups"] != json!(config.groups) {
        return Err("inventory group binding drift".into());
    }
    let storage = Storage::open(&root, &crate::observed::installed(&root)?)?;
    let published:bool=storage.call(json!({"operation":"accepts","producer":"check-original","path":format!("{}-",config.report_directory)}))?;
    if published {
        let base = no_symlink_parents(&root, &config.report_directory)?;
        let parent = base.parent().ok_or("inventory report parent")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let prefix = base
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or("inventory report basename")?;
        let relative = parent
            .strip_prefix(&root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("inventory parent UTF-8")?;
        let (path, adopted) = storage.start(
            relative,
            &format!("{prefix}-"),
            None,
            json!({"kind":"test-inventory","project":config.project,"status":"running"}),
        )?;
        if !adopted {
            return Err("inventory retention policy disappeared".into());
        }
        config.report_directory = path;
    }
    let mut inventory = Inventory {
        root: &root,
        config,
        project: rows[0],
        receipts: Vec::new(),
        storage,
        published,
    };
    let mut result = inventory.collect(&config_bytes, &projects_bytes);
    if inventory.published {
        let directory = &inventory.config.report_directory;
        let outcome = json!({"kind":"test-inventory","project":inventory.config.project,
            "status":listing_status(&result)});
        let seal = inventory
            .storage
            .publication::<()>(
                json!({"kind":"complete","path":directory,"outcome":outcome,"seal":true}),
            )
            .and_then(|()| {
                inventory
                    .storage
                    .publication::<()>(json!({"kind":"retain-diagnostic","path":directory}))
            });
        result = inventory_summary(result, seal).map_err(|(listing, error)| {
            let listing = listing.err().map(|e| format!("{e}; ")).unwrap_or_default();
            format!("{listing}inventory publication: {error}; report_directory={directory}")
        })?;
        let summary = result
            .as_mut()
            .map_err(|error| format!("{error}; report_directory={directory}"))?;
        summary["report_directory"] = json!(directory);
    }
    Ok(format!("{}\n", result?))
}
