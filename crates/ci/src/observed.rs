//! The provider retains bounded bytes and its own business receipt from the installed process owner.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// A capture error from the owner takes precedence over incomplete streams.
pub fn capture_error<'a>(failure: Option<&'a str>, stdout_complete: bool, stderr_complete: bool)
    -> (error: Option<&'a str>)
    ensures match failure {
        Some(original) => error == Some(original),
        None => if stdout_complete && stderr_complete { error is None } else {
            error is Some && error.unwrap()@ == "incomplete observed command streams"@
        },
    },
{
    match failure {
        Some(error) => Some(error),
        None if stdout_complete && stderr_complete => None,
        None => Some("incomplete observed command streams"),
    }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::files::sha256;
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};

#[cfg(not(verus_keep_ghost))]
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommandSpec {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}
#[cfg(not(verus_keep_ghost))]
impl CommandSpec {
    pub fn validate(&self) -> Result<(), String> {
        if self.program.is_empty()
            || self.program.contains('\0')
            || self.args.iter().any(|a| a.contains('\0'))
            || self
                .env
                .iter()
                .any(|(k, v)| k.is_empty() || k.contains(['=', '\0']) || v.contains('\0'))
            || self.timeout_seconds == 0
            || !(1..=64 * 1024 * 1024).contains(&self.output_limit_bytes)
        {
            return Err("invalid command/bounds".into());
        }
        Ok(())
    }
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
pub(crate) struct Observation {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub executable: PathBuf,
    pub sha256: String,
    pub environment_digest: String,
    pub exit_code: i32,
    pub failure: Option<String>,
    pub stdout_complete: bool,
    pub stderr_complete: bool,
    launcher_pid: Option<u32>,
    child_pid: Option<u32>,
    stdout_size: usize,
    stderr_size: usize,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    #[serde(skip)]
    pub stdout_bytes: Vec<u8>,
    #[serde(skip)]
    pub stderr_bytes: Vec<u8>,
}

#[cfg(not(verus_keep_ghost))]
impl Observation {
    pub fn capture_error(&self) -> Option<&str> {
        capture_error(
            self.failure.as_deref(),
            self.stdout_complete,
            self.stderr_complete,
        )
    }
}

#[cfg(not(verus_keep_ghost))]
fn receive(mut input: impl Read, limit: usize) -> Result<Observation, String> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    loop {
        let mut header = [0u8; 9];
        input
            .read_exact(&mut header)
            .map_err(|e| format!("incomplete observed command: {e}"))?;
        let size = usize::try_from(u64::from_be_bytes(header[1..].try_into().unwrap()))
            .map_err(|_| "observed length overflow")?;
        let stream = match header[0] {
            b'O' => &mut stdout,
            b'E' => &mut stderr,
            b'T' => {
                if size > 2 * 1024 * 1024 {
                    return Err("oversize observed terminal record".into());
                }
                let mut bytes = vec![0u8; size];
                input.read_exact(&mut bytes).map_err(|e| e.to_string())?;
                let mut p: Observation = crate::decode(&bytes)?;
                if input.read(&mut [0]).map_err(|e| e.to_string())? != 0
                    || p.stdout_size != stdout.len()
                    || p.stderr_size != stderr.len()
                    || p.stdout_sha256 != sha256(&stdout)
                    || p.stderr_sha256 != sha256(&stderr)
                {
                    return Err("observed command stream identity differs".into());
                }
                p.stdout_bytes = stdout;
                p.stderr_bytes = stderr;
                return Ok(p);
            }
            _ => return Err("unknown observed command frame".into()),
        };
        if size > limit.saturating_sub(stream.len()) {
            return Err("observed output exceeds command bound".into());
        }
        let offset = stream.len();
        stream.resize(offset + size, 0);
        input
            .read_exact(&mut stream[offset..])
            .map_err(|e| e.to_string())?;
    }
}

/// Use the registered program with the source process's input, deadline and ownership descriptors.
#[cfg(not(verus_keep_ghost))]
pub(crate) fn run(
    root: &Path,
    runner: &Path,
    spec: &CommandSpec,
    input: &[u8],
    digest: &str,
    timeout: Duration,
) -> Result<Observation, String> {
    let mut child = crate::storage::command(runner)?
        .arg("observed-command-v1")
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let launcher = child.id();
    let mut dest = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let mut errors = child.stderr.take().unwrap();
    let (received, sent, diagnostics) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || -> Result<(), String> {
            serde_json::to_writer(
                &mut dest,
                &serde_json::json!({"command":spec,"expected_sha256":digest,"timeout":timeout}),
            )
            .map_err(|e| e.to_string())?;
            dest.write_all(b"\n")
                .and_then(|_| dest.write_all(input))
                .map_err(|e| e.to_string())
        });
        let diagnostic = scope.spawn(move || -> std::io::Result<Vec<u8>> {
            let mut data = Vec::new();
            errors.read_to_end(&mut data)?;
            Ok(data)
        });
        let received = receive(output, spec.output_limit_bytes);
        (received, writer.join(), diagnostic.join())
    });
    let status = child.wait().map_err(|e| e.to_string())?;
    let diagnostics = diagnostics
        .map_err(|_| "observed diagnostic reader panicked")?
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(String::from_utf8_lossy(&diagnostics).trim_end().into());
    }
    sent.map_err(|_| "observed input writer panicked")??;
    let p = received?;
    if p.launcher_pid != Some(launcher)
        || p.child_pid.is_none_or(|pid| pid == 0)
        || p.sha256 != digest
    {
        return Err("observed process identity differs".into());
    }
    Ok(p)
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn installed(root: &Path) -> Result<PathBuf, String> {
    let local = root.join(".chrono-harness/bin/chrono-harness");
    if local.is_file() {
        return Ok(local);
    }
    crate::files::resolve_program(root, "chrono-harness", None)
}
