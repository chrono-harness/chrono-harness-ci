#[cfg(unix)]
#[test]
fn installed_runner_can_link_to_its_release_outside_the_checkout() {
    use chrono_ci::units::installed_runner_sha256;
    use std::{fs, os::unix::fs::symlink};

    let root = tempfile::tempdir().unwrap();
    let releases = tempfile::tempdir().unwrap();
    let exe = releases.path().join("chrono-harness");
    fs::write(&exe, b"abc").unwrap();
    let bin = root.path().join("bin");
    fs::create_dir(&bin).unwrap();
    symlink(&exe, bin.join("runner")).unwrap();
    let digest = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert_eq!(
        installed_runner_sha256(root.path(), "bin/runner").unwrap(),
        digest
    );
    fs::write(root.path().join("runner"), b"abc").unwrap();
    assert_eq!(
        installed_runner_sha256(root.path(), "runner").unwrap(),
        digest
    );
    symlink(releases.path(), root.path().join("other")).unwrap();
    assert!(installed_runner_sha256(root.path(), "other/chrono-harness").is_err());
    assert!(installed_runner_sha256(root.path(), "../runner").is_err());
    fs::remove_file(exe).unwrap();
    assert!(installed_runner_sha256(root.path(), "bin/runner").is_err());
}

#[test]
fn offline_initial_inventory_rejects_invalid_scope_and_dependency_graph() {
    use chrono_ci::initial::{Binding, Profile};
    let binding = |id: &str, after: &[&str]| Binding {
        id: id.into(),
        executable: format!("bin/{id}"),
        version: "1".into(),
        sha256: Some("e".repeat(64)),
        argv: vec![],
        selector: "every-initial".into(),
        after: after.iter().map(|id| id.to_string()).collect(),
        modes: vec!["inventory".into()],
    };
    let mut profile = Profile {
        schema: "chrono-initial-check/v1".into(),
        host_config: ".chrono-harness/config.json".into(),
        judges: vec![binding("last", &["first"]), binding("first", &[])],
        timeout_seconds: 1,
        stdout_limit_bytes: 64 * 1024 * 1024,
    };
    assert!(profile.validate().is_ok());
    for edges in [
        vec!["missing".into()],
        vec!["last".into()],
        vec!["first".into(), "first".into()],
    ] {
        profile.judges[0].after = edges;
        assert!(profile.validate().is_err());
    }
    profile.judges[0].after = vec!["first".into()];
    profile.judges[0].modes = vec!["evaluate".into()];
    assert!(profile.validate().is_err());
    profile.judges[0].modes = vec!["inventory".into()];
    profile.judges[0].sha256 = Some("A".repeat(64));
    assert!(profile.validate().is_err());
}
#[test]
fn repeated_nested_members_are_rejected_before_projection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ci.json");
    std::fs::write(
        &path,
        br#"{"settings":[{"name":"original","name":"changed"}]}"#,
    )
    .unwrap();
    let error = chrono_ci::load(&path).unwrap_err();
    assert!(error.contains("duplicate JSON member name"), "{error}");
}
