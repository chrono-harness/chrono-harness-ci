//! Release acceptance for the released version observation and original CLI failure.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn version_pattern() -> (value: &'static str)
    ensures value@ == "chrono-ci [0-9]+[.][0-9]+[.][0-9]+"@,
{ "chrono-ci [0-9]+[.][0-9]+[.][0-9]+" }

pub fn diagnostic() -> (value: &'static str)
    ensures value@ == "detect requires fixed host-root, config and github-output"@,
{ "detect requires fixed host-root, config and github-output" }

pub fn accepts(exit: i32, version_ok: bool, error_matches: bool) -> (accepted: bool)
    ensures accepted == (exit == 1 && version_ok && error_matches),
{
    exit == 1 && version_ok && error_matches
}
}

#[cfg(not(verus_keep_ghost))]
fn grep(mode: &str, pattern: &str, path: &str) -> Result<bool, String> {
    // grep interprets its documented matching mode and reports the original matching lines.
    std::process::Command::new("grep")
        .args([mode, pattern, path])
        .status()
        .map(|result| result.success())
        .map_err(|error| error.to_string())
}

#[cfg(not(verus_keep_ghost))]
fn main() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let [exit, version_file, error_file] = arguments.as_slice() else {
        return Err("usage: release_observation EXIT VERSION_FILE ERROR_FILE".into());
    };
    let exit = exit.parse::<i32>().map_err(|e| e.to_string())?;
    let version_ok = grep("-Ex", version_pattern(), version_file)?;
    let error_matches = grep("-F", diagnostic(), error_file)?;
    let admitted = accepts(exit, version_ok, error_matches);
    if admitted {
        Ok(())
    } else {
        Err(format!("unexpected release observation for {arguments:?}"))
    }
}
