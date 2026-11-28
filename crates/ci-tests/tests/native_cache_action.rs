#[cfg(verus_keep_ghost)]
use super::policy::single_projection as output_policy;
#[cfg(not(verus_keep_ghost))]
use chrono_ci as output_policy;
use output_policy::{
    OutputChange, OutputError, loaded_output, output_change, workflow_size_allowed,
};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
fn expect_output(actual: bool)
    requires actual,
{
    if !actual {
        panic!();
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn workflow_output_error_priority_and_size_boundary() {
    let directory = ".github/workflows/";
    expect_output(workflow_size_allowed(directory, 500_000));
    expect_output(!workflow_size_allowed(directory, 500_001));
    expect_output(matches!(loaded_output(Err(()), "", "marker"), Err(OutputError::InvalidUtf8)));
    expect_output(matches!(loaded_output(Ok(None), "", "marker"), Ok(false)));
    expect_output(matches!(loaded_output(Ok(Some("marker")), "marker", "marker"), Ok(true)));
}

#[cfg(verus_keep_ghost)]
fn ownership_precedes_equality(text: &str, expected: &str, marker: &str)
    requires text.spec_bytes().len() < marker.spec_bytes().len(),
{
    let collision = loaded_output(Ok(Some(text)), text, marker);
    let different = loaded_output(Ok(Some(text)), expected, marker);
    expect_output(matches!(collision, Err(OutputError::Collision)));
    expect_output(matches!(different, Err(OutputError::Collision)));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn workflow_output_modes_apply_only_to_missing_or_drifted_outputs() {
    expect_output(matches!(output_change(true, false), OutputChange::Keep));
    expect_output(matches!(output_change(true, true), OutputChange::Keep));
    expect_output(matches!(output_change(false, true), OutputChange::Drift));
    expect_output(matches!(output_change(false, false), OutputChange::Write));
}
}

#[cfg(not(verus_keep_ghost))]
use std::{fs, path::Path};
#[cfg(not(verus_keep_ghost))]
use tempfile::tempdir;

#[cfg(not(verus_keep_ghost))]
const CONFIG: &str = ".chrono-harness/ci/check.json";
#[cfg(not(verus_keep_ghost))]
const REGISTRY: &str = ".chrono-harness/cache.json";

#[cfg(not(verus_keep_ghost))]
fn configured_jobs(native_jobs: &[&str], cache_jobs: &[&str]) -> String {
    let mut native = Vec::new();
    let mut selected = Vec::new();
    for job in native_jobs {
        native.push(format!(r#""{job}":{{"action_path":".github/actions/{job}/action.yml","command":["./native.sh","two words"]}}"#));
    }
    for job in cache_jobs {
        selected.push(format!(r#""{job}":{{"need":"check","output_use":"compile","consumer":"build","prepare":["./prepare"],"caches":["cargo"]}}"#));
    }
    let pin = "1".repeat(40);
    format!(
        r#"{{"schema":"chrono-github-cache-actions/v1","jobs":{{{}}},"persistent_cache":{{"schema":"chrono-github-cache/v1","program":".chrono-harness/bin/cache","config":"{REGISTRY}","restore_action":"actions/cache/restore@{pin}","save_action":"actions/cache/save@{pin}","save_on_push_only":true,"jobs":{{{}}}}}}}"#,
        native.join(","),
        selected.join(",")
    )
}

#[cfg(not(verus_keep_ghost))]
fn source(jobs: &[&str]) -> String {
    configured_jobs(jobs, jobs)
}

#[cfg(not(verus_keep_ghost))]
fn write(root: &Path, path: &str, text: &str) {
    let target = root.join(path);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, text).unwrap();
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn native_host_command_has_one_writer_and_verify_preserves_drift() {
    let root = tempdir().unwrap();
    let dir = root.path();
    write(
        dir,
        REGISTRY,
        r#"{"schema":"chrono-cache/v2","require_primary_checkout":true,"caches":{"cargo":{"consumers":["build"]}}}"#,
    );
    write(dir, CONFIG, &source(&["build", "again"]));
    let error = chrono_ci::generate(dir, CONFIG, false).unwrap_err();
    assert!(error.contains("E_CACHE_SAVE_OWNERSHIP"), "{error}");
    assert!(!dir.join(".github").exists());
    write(dir, CONFIG, &source(&["build"]));
    assert!(chrono_ci::generate(dir, CONFIG, false).unwrap());
    assert!(!chrono_ci::generate(dir, CONFIG, true).unwrap());
    let output = ".github/actions/build/action.yml";
    let text = fs::read_to_string(dir.join(output)).unwrap();
    let sequence = [
        "actions/cache/restore@",
        "Recover failed cache restores",
        "'./native.sh' 'two words'",
        "Seal producer cache contents",
        "actions/cache/save@",
    ];
    let positions = sequence.map(|needle| text.find(needle).unwrap());
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(text.matches("'./native.sh' 'two words'").count(), 1);
    for (bad_bytes, error) in [
        (b"\xff# host edit\n".as_slice(), "owned output is not UTF-8"),
        (
            b"# host edit\n".as_slice(),
            "unowned workflow collision: .github/actions/build/action.yml",
        ),
    ] {
        fs::write(dir.join(output), bad_bytes).unwrap();
        for verify in [true, false] {
            assert_eq!(chrono_ci::generate(dir, CONFIG, verify).unwrap_err(), error);
            assert_eq!(fs::read(dir.join(output)).unwrap(), bad_bytes);
        }
    }
    let drift = format!("{text}# host edit\n");
    write(dir, output, &drift);
    assert!(
        chrono_ci::generate(dir, CONFIG, true)
            .unwrap_err()
            .contains("drift")
    );
    assert_eq!(fs::read_to_string(dir.join(output)).unwrap(), drift);
    fs::remove_file(dir.join(output)).unwrap();
    assert_eq!(
        chrono_ci::generate(dir, CONFIG, true).unwrap_err(),
        "generated native cache action drift: .github/actions/build/action.yml",
    );
    assert!(!dir.join(output).exists());
    assert!(chrono_ci::generate(dir, CONFIG, false).unwrap());
    assert_eq!(fs::read_to_string(dir.join(output)).unwrap(), text);
    for _ in 0..2 {
        assert!(!chrono_ci::generate(dir, CONFIG, true).unwrap());
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn both_public_modes_reject_unregistered_combinations_and_leave_outputs_alone() {
    let root = tempdir().unwrap();
    let dir = root.path();
    write(
        dir,
        REGISTRY,
        r#"{"schema":"chrono-cache/v2","require_primary_checkout":true,"caches":{"cargo":{"consumers":["build"]}}}"#,
    );
    let normal = source(&["build"]);
    let downloads = normal.replace(r#""jobs":{"build":{"need""#,r#""downloads":{"schema":"chrono-github-downloads/v1","namespace":"host","environment":{},"entries":{}},"jobs":{"build":{"need""#);
    assert_ne!(downloads, normal);
    for (path, config, error) in [
        (
            "policy/actions.json",
            normal.clone(),
            "host CI registration",
        ),
        (
            CONFIG,
            configured_jobs(&[], &[]),
            "requires its registered job",
        ),
        (
            CONFIG,
            configured_jobs(&["build"], &[]),
            "requires its registered job",
        ),
        (
            CONFIG,
            configured_jobs(&["build"], &["build", "extra"]),
            "requires its registered job",
        ),
        (
            CONFIG,
            configured_jobs(&["other"], &["build"]),
            "requires its registered job",
        ),
        (CONFIG, downloads, "register native download directories"),
        (
            CONFIG,
            normal.replace(
                ".github/actions/build/action.yml",
                "actions/build/action.yml",
            ),
            "distinct .github/actions",
        ),
    ] {
        write(dir, path, &config);
        for verify in [false, true] {
            let found = chrono_ci::generate(dir, path, verify).unwrap_err();
            assert!(found.contains(error), "{found}");
            assert!(!dir.join(".github").exists());
        }
    }

    let two = source(&["a", "b"]).replace(
        r#""caches":["cargo"]"#,
        r#""save_caches":[],"caches":["cargo"]"#,
    );
    let shared = two.replace(
        ".github/actions/b/action.yml",
        ".github/actions/a/action.yml",
    );
    write(dir, CONFIG, &shared);
    let error = chrono_ci::generate(dir, CONFIG, false).unwrap_err();
    assert!(error.contains("distinct .github/actions"), "{error}");
    assert!(!dir.join(".github").exists());

    write(dir, CONFIG, &two);
    assert!(chrono_ci::generate(dir, CONFIG, false).unwrap());
    assert!(!chrono_ci::generate(dir, CONFIG, true).unwrap());
    let output = dir.join(".github/actions/b/action.yml");
    let before = fs::read(&output).unwrap();
    write(dir, CONFIG, &shared);
    let error = chrono_ci::generate(dir, CONFIG, true).unwrap_err();
    assert!(error.contains("distinct .github/actions"), "{error}");
    assert_eq!(fs::read(output).unwrap(), before);
}
