//! Obtain DELTA and retained-original decisions from the installed adjudicator.
use super::*;
use crate::{
    files::{file_identity, resolve_program},
    observed::CommandSpec,
};
use std::time::Duration;

pub(crate) fn request(
    root: &Path,
    config: &units::Config,
    mut query: Value,
    remaining: Option<Duration>,
) -> Result<Value, String> {
    let installed;
    let (program, path) = match &config.requirements_program {
        Some(program) => (program.as_str(), None),
        None if config.schema == units::SCHEMA => {
            installed = units::scoped_profile(root, config)?.judge;
            (
                installed.program.as_str(),
                installed.env.get("PATH").map(String::as_str),
            )
        }
        None => ("chrono-judge-ci", None),
    };
    let executable = resolve_program(root, program, path)?;
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
    let result = crate::observed::run(
        root,
        &root.join(&config.collection.runner),
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
            String::from_utf8_lossy(&result.stderr_bytes).trim_end()
        ));
    }
    decode(&result.stdout_bytes)
}
