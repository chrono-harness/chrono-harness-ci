use std::{fs, path::Path};
use tempfile::tempdir;

const CONFIG: &str = ".chrono-harness/ci/check.json";
const REGISTRY: &str = ".chrono-harness/cache.json";

fn source(jobs: &[&str]) -> String {
    let mut native = Vec::new();
    let mut selected = Vec::new();
    for job in jobs {
        native.push(format!(r#""{job}":{{"action_path":".github/actions/{job}/action.yml","command":["./native.sh","two words"]}}"#));
        selected.push(format!(r#""{job}":{{"need":"check","output_use":"compile","consumer":"build","prepare":["./prepare"],"caches":["cargo"]}}"#));
    }
    let pin = "1".repeat(40);
    format!(
        r#"{{"schema":"chrono-github-cache-actions/v1","jobs":{{{}}},"persistent_cache":{{"schema":"chrono-github-cache/v1","program":".chrono-harness/bin/cache","config":"{REGISTRY}","restore_action":"actions/cache/restore@{pin}","save_action":"actions/cache/save@{pin}","save_on_push_only":true,"jobs":{{{}}}}}}}"#,
        native.join(","),
        selected.join(",")
    )
}

fn write(root: &Path, path: &str, text: &str) {
    let target = root.join(path);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, text).unwrap();
}

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
    let drift = format!("{text}# host edit\n");
    write(dir, output, &drift);
    assert!(
        chrono_ci::generate(dir, CONFIG, true)
            .unwrap_err()
            .contains("drift")
    );
    assert_eq!(fs::read_to_string(dir.join(output)).unwrap(), drift);
}
