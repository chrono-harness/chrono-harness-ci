//! Obtain DELTA and retained-original decisions from the installed adjudicator.
use super::*;
use chrono_harness::{CommandSpec, file_identity, resolve_program, run_process_observed_for};
use std::time::Duration;

pub(crate) fn request(
    root: &Path,
    config: &units::Config,
    mut query: Value,
    remaining: Option<Duration>,
) -> Result<Value, String> {
    let program = config
        .requirements_program
        .as_deref()
        .unwrap_or("chrono-judge-ci");
    let executable = resolve_program(root, program, None)?;
    let timeout = Duration::from_secs(u64::from(config.collection.timeout_minutes) * 60);
    let command = CommandSpec {
        program: executable
            .to_str()
            .ok_or("requirements executable path is not UTF-8")?
            .into(),
        args: vec!["requirements-v1".into()],
        // Git and full profiles observe the caller's environment. Transport
        // credentials belong to the download command alone.
        env: std::env::vars()
            .filter(|(key, _)| !config.gather.credential_environment.contains(key))
            .collect(),
        timeout_seconds: timeout.as_secs(),
        output_limit_bytes: config.gather.output_limit_bytes,
    };
    query["profile"] = json!(config.collection.check_config);
    let input = serde_json::to_vec(&query).map_err(|e| e.to_string())?;
    let result = run_process_observed_for(
        root,
        &command,
        &input,
        &file_identity(&executable)?.0,
        remaining.unwrap_or(timeout).min(timeout),
    )?;
    if let Some(failure) = result.failure {
        return Err(failure);
    }
    if result.exit_code != 0 {
        return Err(format!(
            "requirements-v1 exited {}: {}",
            result.exit_code,
            result.stderr.trim_end()
        ));
    }
    decode(&result.stdout_bytes)
}
