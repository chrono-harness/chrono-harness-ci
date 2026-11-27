//! Obtain DELTA and retained-original decisions from the installed adjudicator.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub enum Reply<'a> {
    Failed(&'a str),
    Exited(i32, &'a [u8]),
    Output(&'a [u8]),
}

pub fn query_reply<'a>(error: Option<&'a str>, exit: i32, stdout: &'a [u8], stderr: &'a [u8])
    -> (reply: Reply<'a>)
    ensures reply == match error {
        Some(original) => Reply::Failed(original),
        None => if exit == 0 { Reply::Output(stdout) } else { Reply::Exited(exit, stderr) },
    },
{
    match error {
        Some(original) => Reply::Failed(original),
        None if exit == 0 => Reply::Output(stdout),
        None => Reply::Exited(exit, stderr),
    }
}
}

#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use crate::{
    files::{file_identity, resolve_program},
    observed::CommandSpec,
};
#[cfg(not(verus_keep_ghost))]
use std::time::Duration;

#[cfg(not(verus_keep_ghost))]
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
    match query_reply(
        result.capture_error(),
        result.exit_code,
        &result.stdout_bytes,
        &result.stderr_bytes,
    ) {
        Reply::Failed(error) => Err(error.into()),
        Reply::Exited(code, diagnostic) => Err(format!(
            "requirements-v1 exited {code}: {}",
            String::from_utf8_lossy(diagnostic).trim_end()
        )),
        Reply::Output(bytes) => decode(bytes),
    }
}
