//! GitHub transports fixed unit artifacts; the scoped judge decides their acceptance.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
#[derive(Debug, PartialEq, Eq)]
pub enum LatestAttempt {
    Missing,
    Ambiguous,
    Selected(usize),
}

/// GitHub job pages may contain earlier executions and carry successful jobs forward.
pub fn latest_attempt(attempts: &[Option<u64>]) -> (result: LatestAttempt)
    ensures
        match result {
            LatestAttempt::Missing =>
                forall|i: int| 0 <= i < attempts.len() ==> attempts@[i] is None,
            LatestAttempt::Ambiguous =>
                exists|i: int, j: int| 0 <= i < attempts.len() && 0 <= j < attempts.len()
                    && i != j && attempts@[i] is Some && attempts@[j] == attempts@[i]
                    && forall|k: int| 0 <= k < attempts.len() && attempts@[k] is Some
                        ==> attempts@[k].unwrap() <= attempts@[i].unwrap(),
            LatestAttempt::Selected(i) =>
                i < attempts.len() && attempts@[i as int] is Some
                    && forall|j: int| 0 <= j < attempts.len() && j != i && attempts@[j] is Some
                        ==> attempts@[j].unwrap() < attempts@[i as int].unwrap(),
        },
{
    let mut latest: Option<(usize, u64)> = None;
    let mut tied = false;
    let mut i = 0;
    while i < attempts.len()
        invariant
            i <= attempts.len(),
            match latest {
                None => forall|j: int| 0 <= j < i ==> attempts@[j] is None,
                Some((index, value)) =>
                    index < i && attempts@[index as int] == Some(value)
                    && (forall|j: int| 0 <= j < i && attempts@[j] is Some
                        ==> attempts@[j].unwrap() <= value)
                    && (tied <==> exists|j: int| 0 <= j < i && j != index
                        && attempts@[j] == Some(value)),
            },
        decreases attempts.len() - i,
    {
        if let Some(attempt) = attempts[i] {
            match latest {
                Some((_, value)) if attempt <= value => {
                    tied = tied || attempt == value;
                }
                _ => {
                    latest = Some((i, attempt));
                    tied = false;
                }
            }
        }
        i += 1;
    }
    match latest {
        None => LatestAttempt::Missing,
        Some((index, value)) => {
            if tied {
                assert(exists|j: int| 0 <= j < attempts.len() && j != index
                    && attempts@[j] == Some(value));
                LatestAttempt::Ambiguous
            } else {
                LatestAttempt::Selected(index)
            }
        }
    }
}

pub fn execution_identity(
    observed_run: Option<u64>, run: u64, observed_head: Option<&str>, head: &str,
    id: Option<u64>, attempt: Option<u64>, earliest: u64, parent: u64,
) -> (valid: bool)
    ensures valid == (observed_run == Some(run) && observed_head is Some
        && observed_head.unwrap()@ == head@ && id is Some && id.unwrap() > 0
        && attempt is Some && earliest <= attempt.unwrap() <= parent),
{
    if let (Some(r), Some(h), Some(id), Some(attempt)) = (observed_run, observed_head, id, attempt) {
        r == run && <&str>::exec_eq(h, head) && id > 0 && earliest <= attempt && attempt <= parent
    } else {
        false
    }
}

/// The producer may be older than the listed job, within the selected detector's history.
pub fn original_attempt(detector: u64, producer: u64, execution: u64) -> (valid: bool)
    ensures valid == (detector <= producer <= execution),
{
    detector <= producer && producer <= execution
}

pub fn original_field(observed: Option<&str>, expected: &str) -> (matches: bool)
    ensures matches == (observed is Some && observed.unwrap()@ == expected@),
{
    match observed {
        Some(value) => <&str>::exec_eq(value, expected),
        None => false,
    }
}

pub struct AcquisitionBudget<D> {
    pub deadline: D,
    pub stdout_remaining: usize,
    pub stderr_remaining: usize,
}

impl<D> AcquisitionBudget<D> {
    pub fn start(deadline: Option<D>, limit: usize) -> (result: Result<Self, &'static str>)
        ensures match deadline {
            Some(value) => result is Ok && result.unwrap().deadline == value
                && result.unwrap().stdout_remaining == limit && result.unwrap().stderr_remaining == limit,
            None => result is Err && result->Err_0@ == "artifact acquisition deadline overflow"@,
        },
    {
        match deadline {
            Some(deadline) => Ok(Self { deadline, stdout_remaining: limit, stderr_remaining: limit }),
            None => Err("artifact acquisition deadline overflow"),
        }
    }

    pub fn charge(&mut self, stdout: usize, stderr: usize)
        ensures self.deadline == old(self).deadline,
            self.stdout_remaining as int == if stdout <= old(self).stdout_remaining {
                old(self).stdout_remaining - stdout } else { 0 },
            self.stderr_remaining as int == if stderr <= old(self).stderr_remaining {
                old(self).stderr_remaining - stderr } else { 0 },
    {
        self.stdout_remaining = self.stdout_remaining.saturating_sub(stdout);
        self.stderr_remaining = self.stderr_remaining.saturating_sub(stderr);
    }
}

/// Durations come from the monotonic clock. A zero observation includes a past deadline.
pub fn available_time<T>(value: T, nanos: u128) -> (result: Result<T, &'static str>)
    ensures if nanos == 0 {
        result is Err && result->Err_0@ == "artifact acquisition cumulative deadline exhausted"@
    } else { result == Ok::<T, &'static str>(value) },
{
    if nanos == 0 { Err("artifact acquisition cumulative deadline exhausted") } else { Ok(value) }
}

pub fn transport_limits<T>(requested: T, requested_nanos: u128, output_limit: usize,
    budget: Option<(T, u128, usize, usize)>) -> (result: Result<(T, usize), &'static str>)
    ensures match budget {
        None => result == Ok::<(T, usize), &'static str>((requested, output_limit)),
        Some((time, nanos, stdout, stderr)) =>
            if output_limit == 0 || stdout == 0 || stderr == 0 {
                result is Err && result->Err_0@ == "artifact acquisition cumulative stream limit exhausted"@
            } else if nanos == 0 {
                result is Err && result->Err_0@ == "artifact acquisition cumulative deadline exhausted"@
            } else {
                result is Ok && result.unwrap().0 == if requested_nanos <= nanos { requested } else { time }
                    && result.unwrap().1 as int == vstd::math::min(output_limit as int, vstd::math::min(stdout as int, stderr as int))
            },
    },
{
    let Some((time, nanos, stdout, stderr)) = budget else { return Ok((requested, output_limit)); };
    let limit = output_limit.min(stdout).min(stderr);
    if limit == 0 { return Err("artifact acquisition cumulative stream limit exhausted"); }
    let available = available_time(time, nanos)?;
    let chosen = if requested_nanos <= nanos { requested } else { available };
    Ok((chosen, limit))
}
}

#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use crate::observed::Observation;
#[cfg(not(verus_keep_ghost))]
use crate::{
    files::{file_identity, sha256},
    observed::CommandSpec,
};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

#[cfg(not(verus_keep_ghost))]
struct ArtifactPage<'a>(&'a [Value]);

#[cfg(not(verus_keep_ghost))]
impl crate::verified_download::ArtifactPage for ArtifactPage<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn name(&self, i: usize) -> Option<&str> {
        self.0[i]["name"].as_str()
    }
}

#[cfg(not(verus_keep_ghost))]
impl AcquisitionBudget<Instant> {
    fn remaining(&self) -> Result<Duration, String> {
        let value = self.deadline.saturating_duration_since(Instant::now());
        Ok(available_time(value, value.as_nanos())?)
    }
}

#[cfg(not(verus_keep_ghost))]
struct Transport<'a> {
    root: &'a Path,
    config: &'a units::Gather,
    environment: BTreeMap<String, String>,
    executable: String,
    digest: String,
    observations: Vec<Value>,
    acquisition_path: Option<String>,
    runner: std::path::PathBuf,
    downloads: Option<String>,
    storage: crate::storage::Storage,
    acquisition: Option<AcquisitionBudget<Instant>>,
}

#[cfg(not(verus_keep_ghost))]
impl<'a> Transport<'a> {
    fn new(root: &'a Path, runner: &str, c: &'a units::Gather) -> Result<Self, String> {
        let mut environment = BTreeMap::new();
        for key in &c.inherit_environment {
            if let Ok(value) = std::env::var(key) {
                environment.insert(key.clone(), value);
            }
        }
        environment.extend(c.environment.clone());
        let exe = crate::files::resolve_program(
            root,
            &c.program,
            Some(environment.get("PATH").map(String::as_str).unwrap_or("")),
        )?;
        Ok(Self {
            root,
            config: c,
            environment,
            executable: exe.to_str().ok_or("non UTF-8 transport")?.into(),
            digest: file_identity(&exe)?.0,
            observations: vec![],
            acquisition_path: None,
            runner: root.join(runner),
            downloads: None,
            storage: crate::storage::Storage::open(
                root,
                &units::installed_runner_path(root, runner)?,
            )?,
            acquisition: None,
        })
    }

    fn run(&mut self, args: &[String]) -> Result<Vec<u8>, String> {
        let p = self.process(
            args,
            Duration::from_secs(self.config.timeout_seconds),
            self.config.output_limit_bytes,
        )?;
        if p.capture_error().is_some() || p.exit_code != 0 {
            return Err(
                "GitHub transport failed; original bounded process evidence retained".into(),
            );
        }
        Ok(p.stdout_bytes)
    }

    fn process(
        &mut self,
        args: &[String],
        remaining: Duration,
        output_limit: usize,
    ) -> Result<Observation, String> {
        let budget = self.acquisition.as_ref().map(|budget| {
            let time = budget.deadline.saturating_duration_since(Instant::now());
            (
                time,
                time.as_nanos(),
                budget.stdout_remaining,
                budget.stderr_remaining,
            )
        });
        let (remaining, output_limit) =
            transport_limits(remaining, remaining.as_nanos(), output_limit, budget)?;
        let spec = CommandSpec {
            program: self.executable.clone(),
            args: args.to_vec(),
            env: self.environment.clone(),
            timeout_seconds: self.config.timeout_seconds,
            output_limit_bytes: output_limit,
        };
        let p = crate::observed::run(self.root, &self.runner, &spec, &[], &self.digest, remaining)?;
        let mut observed_environment = spec.env;
        observed_environment.remove("CHRONO_PROCESS_FDS");
        if crate::codec::digest(&observed_environment)? != p.environment_digest {
            return Err("transport environment differs from the observed command".into());
        }
        observed_environment.retain(|key, _| !self.config.credential_environment.contains(key));
        self.observations.push(json!({"argv":p.argv,"cwd":p.cwd,"executable":p.executable,"sha256":p.sha256,
            "environment":observed_environment,"omitted_credentials":self.config.credential_environment,"original_environment_digest":p.environment_digest,
            "stdin_sha256":sha256(&[]),"stdout_complete":p.stdout_complete,"stderr_complete":p.stderr_complete,
            "stdout_bytes":p.stdout_bytes,"stderr_bytes":p.stderr_bytes,
            "stdout_sha256":p.stdout_sha256,"stderr_sha256":p.stderr_sha256,"exit_code":p.exit_code,"failure":p.failure}));
        if let Some(budget) = &mut self.acquisition {
            budget.charge(p.stdout_bytes.len(), p.stderr_bytes.len());
            budget.remaining()?;
        }
        Ok(p)
    }

    fn begin_artifact(&mut self) -> Result<(), String> {
        let deadline = Instant::now().checked_add(Duration::from_secs(self.config.timeout_seconds));
        self.acquisition = Some(AcquisitionBudget::start(
            deadline,
            self.config.output_limit_bytes,
        )?);
        Ok(())
    }

    fn artifact_metadata(
        pages: &Value,
        run: u64,
        repository: &str,
        name: &str,
        candidate: &str,
    ) -> Result<(), String> {
        use crate::verified_download::{ArtifactError, admit_artifact, unique_artifact};

        let pages = pages
            .as_array()
            .ok_or("artifact metadata pages missing")?
            .iter()
            .map(|page| {
                page["artifacts"]
                    .as_array()
                    .map(|rows| ArtifactPage(rows))
                    .ok_or("artifact metadata list missing")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let identity = "artifact missing/ambiguous/expired or run/source binding differs";
        let (page, row) = unique_artifact(&pages, name).ok_or(identity)?;
        let artifact = &pages[page].0[row];
        let id = artifact["id"].as_u64();
        let url = id.map(|id| {
            format!("https://api.github.com/repos/{repository}/actions/artifacts/{id}/zip")
        });
        admit_artifact(
            artifact["expired"].as_bool(),
            artifact["workflow_run"]["id"].as_u64(),
            run,
            artifact["workflow_run"]["head_sha"].as_str(),
            candidate,
            id,
            artifact["archive_download_url"].as_str(),
            url.as_deref().unwrap_or(""),
        )
        .map_err(|cause| {
            match cause {
                ArtifactError::Identity => identity,
                ArtifactError::MissingId => "artifact ID missing",
                ArtifactError::Endpoint => "artifact archive endpoint differs from selected ID",
            }
            .into()
        })
    }

    fn download(
        &mut self,
        run: u64,
        repository: &str,
        artifact: &str,
        directory: &str,
    ) -> Result<String, String> {
        let deadline = if let Some(budget) = &self.acquisition {
            budget.deadline
        } else {
            Instant::now()
                .checked_add(Duration::from_secs(self.config.timeout_seconds))
                .ok_or("artifact download deadline overflow")?
        };
        let absolute = no_symlink_parents(self.root, directory)?;
        if absolute.exists() {
            return Err("artifact download destination exists; retain original acquisition".into());
        }
        self.storage
            .directory(self.root, self.downloads.as_deref(), directory)?;
        let policy = self.config.download_retry.clone();
        let attempts = policy.as_ref().map_or(1, |p| p.max_attempts);
        let mut stdout_remaining = self.config.output_limit_bytes;
        let mut stderr_remaining = self.config.output_limit_bytes;
        for attempt in 1..=attempts {
            let destination = if policy.is_some() {
                let path = format!("{directory}/attempt-{attempt}");
                self.storage
                    .directory(self.root, self.downloads.as_deref(), &path)?;
                path
            } else {
                directory.into()
            };
            let output_limit = stdout_remaining.min(stderr_remaining);
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() || output_limit == 0 {
                return Err(
                    "artifact download budget exhausted; original attempts retained".into(),
                );
            }
            let args = vec![
                "run".into(),
                "download".into(),
                run.to_string(),
                "--repo".into(),
                repository.into(),
                "--name".into(),
                artifact.into(),
                "--dir".into(),
                no_symlink_parents(self.root, &destination)?
                    .to_str()
                    .ok_or("download path UTF-8")?
                    .into(),
            ];
            let p = self.process(&args, remaining, output_limit)?;
            stdout_remaining = stdout_remaining.saturating_sub(p.stdout_bytes.len());
            stderr_remaining = stderr_remaining.saturating_sub(p.stderr_bytes.len());
            let success = p.capture_error().is_none() && p.exit_code == 0;
            let status = super::verified_download::http_status(&p.stderr_bytes);
            let delay = policy
                .as_ref()
                .map(|p| Duration::from_secs(p.delay_seconds));
            let retry = super::verified_download::retry(
                p.exit_code == 0,
                p.capture_error().is_some(),
                attempt < attempts,
                policy.as_ref().is_some_and(|policy| {
                    status.is_some_and(|s| policy.http_statuses.contains(&s))
                }),
                stdout_remaining,
                stderr_remaining,
                delay.is_some_and(|d| d < deadline.saturating_duration_since(Instant::now())),
            );
            if policy.is_some() {
                self.observations
                    .last_mut()
                    .ok_or("download observation missing")?["download"] = json!({"artifact":artifact,"attempt":attempt,"max_attempts":attempts,
                        "directory":destination,"http_status":status,
                        "action":if success {"completed"} else if retry {"retry"} else {"failed"}});
            }
            if success {
                return Ok(destination);
            }
            if !retry {
                return Err(
                    "GitHub transport failed; original bounded process evidence retained".into(),
                );
            }
            std::thread::sleep(delay.ok_or("retry delay missing")?);
        }
        Err("artifact download attempts exhausted; original attempts retained".into())
    }

    fn api(&mut self, endpoint: &str, paginate: bool) -> Result<Value, String> {
        let mut args = vec!["api".into(), endpoint.into()];
        if paginate {
            args.extend(["--paginate".into(), "--slurp".into()]);
        }
        crate::codec::json(&self.run(&args)?)
    }
}

#[cfg(not(verus_keep_ghost))]
fn parent_run(
    run: &Value,
    repository: &str,
    d: &super::gating::Detection,
    c: &units::Config,
    attempt: u64,
) -> Result<(), String> {
    if run["id"] != d.run_id
        || run["run_attempt"] != attempt
        || run["head_sha"] != d.event_head
        || run["event"] != d.event
        || run["path"] != c.collection.workflow_path
        || run["repository"]["full_name"] != repository
    {
        return Err("parent run/repository/event/source/attempt mismatch".into());
    }
    // The aggregate is currently running. Waiting for parent completion would wait on itself.
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
fn validate_detector_jobs(
    jobs: &[&Value],
    d: &super::gating::Detection,
    parent_attempt: u64,
) -> Result<(), String> {
    let detector: Vec<_> = jobs
        .iter()
        .filter(|j| j["name"] == "Detect registered DELTA")
        .collect();
    let attempts: Vec<_> = detector.iter().map(|j| j["run_attempt"].as_u64()).collect();
    let latest = match latest_attempt(&attempts) {
        LatestAttempt::Missing => return Err("detector API execution missing".into()),
        LatestAttempt::Ambiguous => {
            return Err("latest detector execution did not uniquely succeed".into());
        }
        LatestAttempt::Selected(index) => detector[index],
    };
    if !execution_identity(
        latest["run_id"].as_u64(),
        d.run_id,
        latest["head_sha"].as_str(),
        &d.event_head,
        latest["id"].as_u64(),
        latest["run_attempt"].as_u64(),
        d.run_attempt,
        parent_attempt,
    ) || !matches!(
        crate::verified_literals::native_job_state(
            latest["status"].as_str(),
            latest["conclusion"].as_str(),
            None,
            None,
        ),
        crate::verified_literals::NativeJobState::Succeeded
    ) {
        return Err("latest detector execution did not uniquely succeed".into());
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
fn parent_units(
    root: &Path,
    c: &units::Config,
    repository: &str,
    context: &Value,
    transport: &mut Transport<'_>,
) -> Result<BTreeMap<String, Value>, String> {
    let d: super::gating::Detection = serde_json::from_value(context["detection"].clone())
        .map_err(|e| format!("fixed detection missing: {e}"))?;
    let needs: Value = decode(
        std::env::var(super::gating::NEEDS_ENV)
            .map_err(|_| "aggregate needs missing")?
            .as_bytes(),
    )?;
    super::gating::validate_statuses(c, &d, &needs)?;
    if d.repository != repository {
        return Err("detection repository mismatch".into());
    }
    let requirements =
        super::gating::requirements(root, c, d.base.clone(), d.candidate.clone(), d.initial)?;
    if requirements["required_units"] != json!(d.required_units)
        || crate::value_digest(&requirements["global_selected"])? != d.selection_sha256
        || requirements["changed_paths"]
            .as_array()
            .ok_or("complete delta missing")?
            .len()
            != d.changed_paths_count
    {
        return Err("aggregate obligations disagree with fixed detection".into());
    }
    let native: super::gating::NativeRun = serde_json::from_value(context["native_run"].clone())
        .map_err(|e| format!("native parent binding missing: {e}"))?;
    if native != super::gating::native_run(None)?
        || native.run_id != d.run_id
        || native.repository != d.repository
        || native.run_attempt < d.run_attempt
    {
        return Err("aggregate parent run/attempt binding mismatch".into());
    }
    let run = transport.api(
        &format!("repos/{repository}/actions/runs/{}", d.run_id),
        false,
    )?;
    parent_run(&run, repository, &d, c, native.run_attempt)?;
    let pages = transport.api(
        &format!(
            "repos/{repository}/actions/runs/{}/jobs?filter=all&per_page=100",
            d.run_id
        ),
        true,
    )?;
    let jobs: Vec<_> = pages
        .as_array()
        .ok_or("job pages missing")?
        .iter()
        .map(|page| page["jobs"].as_array().ok_or("job list missing"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();
    validate_detector_jobs(&jobs, &d, native.run_attempt)?;
    let mut completed = BTreeMap::new();
    for (unit, workflow) in &c.units {
        // needs already binds every nonrequired skip. Only selected jobs need
        // execution metadata and artifacts; skipped jobs need no API row.
        if !d.required_units.contains(unit) {
            continue;
        }
        // Independent job retries keep successful prerequisites' original attempts.
        // Choose the last actual execution inside this exact parent run, never another run.
        let relevant: Vec<_> = jobs
            .iter()
            .filter(|job| job["name"] == workflow.name)
            .collect();
        let mut attempts = Vec::with_capacity(relevant.len());
        for job in &relevant {
            let attempt = job["run_attempt"].as_u64();
            if !execution_identity(
                job["run_id"].as_u64(),
                d.run_id,
                job["head_sha"].as_str(),
                &d.event_head,
                job["id"].as_u64(),
                attempt,
                1,
                native.run_attempt,
            ) {
                return Err(format!("unit {unit} job run/attempt identity malformed"));
            }
            attempts.push(attempt);
        }
        let index = match latest_attempt(&attempts) {
            LatestAttempt::Missing => {
                return Err(format!("unit {unit} job missing in parent history"));
            }
            LatestAttempt::Ambiguous => {
                return Err(format!(
                    "unit {unit} latest native execution did not uniquely succeed"
                ));
            }
            LatestAttempt::Selected(index) => index,
        };
        let latest_attempt = attempts[index].ok_or("selected native job attempt missing")?;
        let latest = super::gating::completed_native_job(relevant[index], |id| {
            transport.api(&format!("repos/{repository}/actions/jobs/{id}"), false)
        })
        .map_err(|error| format!("unit {unit} {error}"))?;
        // API attempts can describe carried jobs. needs outputs bind the actual
        // producer; API latest status remains a separate necessary predicate.
        let production: super::gating::Production = decode(
            needs[super::gating::job_id(unit)]["outputs"]["production"]
                .as_str()
                .ok_or("original unit production output missing")?
                .as_bytes(),
        )?;
        let p = &production.native_run;
        if production.schema != "chrono-ci-production/v1"
            || p.repository != repository
            || p.run_id != d.run_id
            || p.job != super::gating::job_id(unit)
            || !original_attempt(d.run_attempt, p.run_attempt, latest_attempt)
            || !original_field(
                Some(&production.artifact),
                &format!("chrono-unit-{unit}-{}-{}", p.run_id, p.run_attempt),
            )
            || !crate::codec::is_digest(&production.context_sha256)
            || !crate::codec::is_digest(&production.report_sha256)
        {
            return Err(format!("unit {unit} original producer binding invalid"));
        }
        let mut binding = run.clone();
        binding["conclusion"] = latest["conclusion"].clone();
        binding["unit_job_id"] = latest["id"].clone();
        binding["run_attempt"] = json!(p.run_attempt);
        binding["latest_job_attempt"] = json!(latest_attempt);
        binding["production"] = json!(production);
        completed.insert(unit.clone(), binding);
    }
    Ok(completed)
}

#[cfg(not(verus_keep_ghost))]
fn inner(
    root: &Path,
    path: &str,
    c: &units::Config,
    repository: &str,
    transport: &mut Transport<'_>,
) -> Result<Value, String> {
    units::generate(root, path, c, true)?;
    let context: Value = decode(
        &fs::read(no_symlink_parents(root, &c.collection.context_path)?)
            .map_err(|e| e.to_string())?,
    )?;
    let candidate = context["candidate"]
        .as_str()
        .filter(|v| full_oid(v))
        .ok_or("fixed collection candidate missing")?;
    let event_head = context["event_head"]
        .as_str()
        .filter(|v| full_oid(v))
        .ok_or("fixed event head missing")?;
    let event = context["event"]
        .as_str()
        .ok_or("collection event missing")?;
    if !matches!(event, "push" | "pull_request") {
        return Err("automatic gathering requires push/pull_request; manual unit checks use explicit collection manifests".into());
    }
    if context["scope"] != json!(c.scope(None))
        || context["provider_sha256"] != file_identity(&root.join(path))?.0
    {
        return Err(
            "collection context does not bind current provider and collection scope".into(),
        );
    }
    let registered = units::profile(root, c)?.units;
    if context["check_config_sha256"] != file_identity(&root.join(&c.collection.check_config))?.0 {
        return Err("collection check profile changed".into());
    }
    let canonical = vec![
        c.collection.runner.clone(),
        "check".into(),
        "--collect".into(),
    ];
    if context["canonical_argv"] != json!(canonical) {
        return Err("collection command context mismatch".into());
    }
    let retained_streams = if c.schema == units::SCHEMA {
        units::scoped_profile(root, c)?.retained_reference
    } else {
        false
    };
    let completed = parent_units(root, c, repository, &context, transport)?;
    let mut reports = vec![];
    let mut identities = vec![];
    let download_prefix = {
        let parent = no_symlink_parents(root, &c.gather.download_directory)?;
        fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
        let parent = parent
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("download parent UTF-8")?;
        let (path, adopted) = transport.storage.start(
            parent,
            "gather-",
            Some("publication-recheck"),
            json!({"kind":"native-acquisition","outcome":"not-established"}),
        )?;
        transport.acquisition_path = Some(path.clone());
        transport.downloads = adopted.then(|| path.clone());
        format!("{path}/")
    };
    // Parent jobs share a completed artifact inventory and immutable workflow
    // revision. Reuse those observations only within this one acquisition.
    let mut parent_artifacts = None;
    let mut workflow_source = None;
    for (unit, run) in completed {
        let id = run["id"].as_u64().ok_or("unit run ID missing")?;
        let attempt = run["run_attempt"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("unit run attempt missing")?;
        let w = &c.units[&unit];
        let destination = format!("{download_prefix}{unit}/{id}/{attempt}");
        let artifact = run["production"]["artifact"]
            .as_str()
            .ok_or("original artifact name missing")?
            .to_owned();
        transport.begin_artifact()?;
        if parent_artifacts.is_none() {
            parent_artifacts = Some(transport.api(
                &format!("repos/{repository}/actions/runs/{id}/artifacts?per_page=100"),
                true,
            )?);
        }
        Transport::artifact_metadata(
            parent_artifacts.as_ref().unwrap(),
            id,
            repository,
            &artifact,
            event_head,
        )?;
        let destination = transport.download(id, repository, &artifact, &destination)?;
        let downloaded = |input: &str| -> Result<String, String> {
            let relative = input
                .strip_prefix(&w.artifact_directory)
                .ok_or("report/context outside uploaded unit artifacts")?;
            relative_path(relative)?;
            Ok(format!("{destination}/{relative}"))
        };
        if !original_field(
            run["production"]["context_sha256"].as_str(),
            &file_identity(&no_symlink_parents(root, &downloaded(&w.context_path)?)?)?.0,
        ) || !original_field(
            run["production"]["report_sha256"].as_str(),
            &file_identity(&no_symlink_parents(
                root,
                &downloaded(&registered[&unit].report_path)?,
            )?)?
            .0,
        ) {
            return Err(format!(
                "unit {unit} artifact differs from original production output"
            ));
        }
        if retained_streams {
            let budget = transport
                .acquisition
                .as_ref()
                .ok_or("artifact budget missing")?;
            let digest = &run["production"]["report_sha256"];
            let query = json!({"operation":"unit-originals","profile_sha256":context["check_config_sha256"],
                "unit":unit,"report":{"path":registered[&unit].report_path,"sha256":digest},
                "transport":{"source_directory":w.artifact_directory,"directory":format!("{destination}/")}});
            let receipt = super::requirements::request(root, c, query, Some(budget.remaining()?))?;
            if receipt["unit"] != unit || receipt["report_sha256"] != *digest {
                return Err(format!("unit {unit} original report receipt mismatch"));
            }
            budget.remaining()?;
        }
        let unit_context: Value = decode(
            &fs::read(no_symlink_parents(root, &downloaded(&w.context_path)?)?)
                .map_err(|e| e.to_string())?,
        )?;
        let scope = c.scope(Some(&unit));
        let expected = vec![
            c.collection.runner.clone(),
            "check".into(),
            "--unit".into(),
            unit.clone(),
        ];
        if [
            "event",
            "base",
            "candidate",
            "event_head",
            "initial",
            "provider_sha256",
            "check_config_sha256",
        ]
        .iter()
        .any(|k| unit_context[*k] != context[*k])
            || unit_context["scope"] != json!(scope)
            || unit_context["canonical_argv"] != json!(expected)
        {
            return Err(format!(
                "unit {unit} context disagrees with fixed collection inputs"
            ));
        }
        if unit_context["detection"] != context["detection"]
            || unit_context["workflow_source_revision"] != context["workflow_source_revision"]
        {
            return Err(format!(
                "unit {unit} parent run/attempt/source detection mismatch"
            ));
        }
        let native: super::gating::NativeRun =
            serde_json::from_value(unit_context["native_run"].clone())
                .map_err(|e| format!("unit native run missing: {e}"))?;
        if native.repository != repository
            || native.run_id != id
            || native.run_attempt != attempt
            || native.job != super::gating::job_id(&unit)
        {
            return Err(format!("unit {unit} artifact run/attempt/job mismatch"));
        }
        let workflow_path = &c.collection.workflow_path;
        let revision = unit_context["workflow_source_revision"]
            .as_str()
            .filter(|v| full_oid(v))
            .ok_or("workflow source revision missing")?;
        let source_args = [
            "api".into(),
            format!(
                "repos/{repository}/contents/{}?ref={revision}",
                workflow_path
            ),
            "--header".into(),
            "Accept: application/vnd.github.raw".into(),
        ];
        if workflow_source.is_none() {
            let source = transport.run(&source_args)?;
            if source != fs::read(root.join(workflow_path)).map_err(|e| e.to_string())? {
                return Err(format!("unit {unit} executed a different workflow source"));
            }
            workflow_source = Some(source);
        }
        let source = workflow_source.as_ref().unwrap();
        let report_path = downloaded(&registered[&unit].report_path)?;
        let mut input = json!({"unit":unit,"path":report_path,"sha256":file_identity(&no_symlink_parents(root,&report_path)?)?.0,
            "runner_sha256":unit_context["executables"]["runner_sha256"],"judge_sha256":unit_context["executables"]["judge_sha256"],
            "artifacts":{"source_directory":w.artifact_directory,"directory":format!("{destination}/")}});
        if c.schema == units::FULL_SCHEMA {
            if unit_context["full_context"]["semantic_digest"]
                != context["full_context"]["semantic_digest"]
                || unit_context["executables"]["judges"] != context["executables"]["judges"]
            {
                return Err("full unit context/seven-judge binding differs from collection".into());
            }
        }
        if input["artifacts"].is_null() {
            input.as_object_mut().unwrap().remove("artifacts");
        }
        reports.push(input);
        transport
            .acquisition
            .as_ref()
            .ok_or("artifact budget missing")?
            .remaining()?;
        identities.push(json!({"unit":unit,"run":id,"attempt":attempt,"conclusion":run["conclusion"],"job_id":run["unit_job_id"],"workflow_source_revision":revision,"workflow_sha256":sha256(&source)}));
    }
    if !identities.is_empty() {
        let d: super::gating::Detection =
            serde_json::from_value(context["detection"].clone()).map_err(|e| e.to_string())?;
        let fresh = transport.api(
            &format!("repos/{repository}/actions/runs/{}", d.run_id),
            false,
        )?;
        parent_run(
            &fresh,
            repository,
            &d,
            c,
            context["native_run"]["run_attempt"]
                .as_u64()
                .ok_or("parent attempt missing")?,
        )?;
        transport
            .acquisition
            .as_ref()
            .ok_or("artifact budget missing")?
            .remaining()?;
        transport.acquisition = None;
    }
    if identities.iter().any(|i| i["conclusion"] != "success") {
        return Err(format!(
            "native unit workflow failed; original downloaded reports retained: {}",
            json!(identities)
        ));
    }
    let manifest = json!({"schema":if c.schema==units::FULL_SCHEMA {"chrono-full-collection/v1"}else{"chrono-ci-collection/v1"},"reports":reports});
    let manifest_path = no_symlink_parents(root, &c.gather.manifest_path)?;
    write_file(
        root,
        &c.gather.manifest_path,
        &serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )?;
    let mut retained = crate::preparation::Originals::open(root, &c.collection.runner)?;
    let manifest_original = json!(retained.retain(
        &format!("{}preparation/", c.collection.artifact_directory),
        "manifest",
        &fs::read(&manifest_path).map_err(|e| e.to_string())?
    )?);
    retained.close()?;
    Ok(
        json!({"status":"gathered-unjudged","base":context["base"],"candidate":candidate,"units":identities,"manifest_path":c.gather.manifest_path,"manifest_sha256":file_identity(&manifest_path)?.0,"manifest_original":manifest_original}),
    )
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn gather(
    root: &Path,
    path: &str,
    c: &units::Config,
    repository: &str,
) -> Result<String, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let root = root.as_path();
    units::validate(c)?;
    let parts: Vec<_> = repository.split('/').collect();
    if parts.len() != 2 || parts.iter().any(|p| !crate::units::valid_id(p)) {
        return Err("repository must be explicit owner/name".into());
    }
    let mut transport = Transport::new(root, &c.collection.runner, &c.gather)?;
    let result = inner(root, path, c, repository, &mut transport);
    let acquisition_path = transport.acquisition_path.take();
    let storage_result = transport.downloads.take().map(|path|transport.storage.publication::<()>(json!({"kind":"complete","path":path,"outcome":{"kind":"native-acquisition","result":result.as_ref().ok(),"error":result.as_ref().err(),"completion":"not-established"}}))).transpose();
    let mut report = json!({"schema":"chrono-ci-gather/v1","repository":repository,"result":result.as_ref().ok(),"error":result.as_ref().err(),"storage_error":storage_result.as_ref().err(),"processes":transport.observations});
    let mut retained = crate::preparation::Originals::open(root, &c.collection.runner)?;
    let gather_original = retained
        .retain(
            &format!("{}preparation/", c.collection.artifact_directory),
            "native-gather",
            &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
        )?
        .path;
    retained.close()?;
    report["retained_report"] = json!(gather_original);
    if (result.is_err() || storage_result.is_err()) && transport.storage.adopted()? {
        transport
            .storage
            .publication::<()>(json!({"kind":"retain-diagnostic","path":gather_original}))?;
    }
    transport.storage.publication::<()>(json!({"kind":"reference","slot":c.gather.report_path,"path":gather_original,"inputs":acquisition_path.into_iter().collect::<Vec<_>>()}))?;
    write_file(
        root,
        &c.gather.report_path,
        &serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )?;
    transport.storage.publication::<()>(
        json!({"kind":"settle","slot":c.gather.report_path,"path":gather_original}),
    )?;
    result?;
    storage_result?;
    Ok(serde_json::to_string(&report).map_err(|e| e.to_string())? + "\n")
}
