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

#[test]
fn pull_request_selection_uses_the_observed_first_parent() {
    use chrono_ci::verified_literals::pull_request_base;

    for width in [40, 64] {
        let base = "a".repeat(width);
        let head = "b".repeat(width);
        let parents = vec![base.clone(), head.clone()];
        assert_eq!(pull_request_base(&parents, &head), Some(base.as_str()));
        for rejected in [
            vec![],
            vec![base.clone()],
            vec![base.clone(), head.clone(), base.clone()],
            vec![head.clone(), base.clone()],
            vec!["0".repeat(width), head.clone()],
            vec!["A".repeat(width), head.clone()],
        ] {
            assert_eq!(pull_request_base(&rejected, &head), None);
        }
        for malformed in ["a".repeat(39), "0".repeat(width), "G".repeat(width)] {
            let parents = vec![base.clone(), malformed.clone()];
            assert_eq!(pull_request_base(&parents, &malformed), None);
        }
    }
}

#[test]
fn native_check_inputs_admits_paired_workflow_arguments_and_bounded_requests() {
    use std::io::{Seek, Write};
    use std::os::unix::fs::symlink;
    use std::process::{Command, Stdio};

    let root = tempfile::tempdir().unwrap();
    let host = root.path().join("host");
    let alias = root.path().join("alias");
    std::fs::create_dir_all(host.join(".chrono-harness/ci")).unwrap();
    symlink(&host, &alias).unwrap();
    std::fs::write(host.join(".chrono-harness/ci/units.json"), b"?").unwrap();
    let args = [
        "--config",
        ".chrono-harness/ci/units.json",
        "--event-env",
        "GITHUB_EVENT_NAME",
        "--payload-env",
        "GITHUB_EVENT_PATH",
        "--revision-env",
        "CHRONO_WORKFLOW_REVISION",
        "--repository-env",
        "GITHUB_REPOSITORY",
    ];
    let check = |flags: &[&str], request: &[u8], diagnostic: &str| {
        let mut stdin = tempfile::tempfile().unwrap();
        stdin.write_all(request).unwrap();
        stdin.rewind().unwrap();
        let output = Command::new(std::env::var_os("CHRONO_TEST_CI").unwrap())
            .current_dir(&host)
            .arg("check-inputs")
            .args(flags)
            .env_clear()
            .stdin(Stdio::from(stdin))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let message = std::str::from_utf8(&output.stderr).unwrap();
        assert!(
            message.starts_with("E_CI: ") && message.contains(diagnostic),
            "{flags:?}: {message}"
        );
    };
    check(&[], b"", "check-inputs requires declared paired arguments");
    check(
        &["--config"],
        b"",
        "check-inputs requires declared paired arguments",
    );
    check(
        &["--other", "x"],
        b"",
        "unknown/duplicate input producer argument",
    );
    for pair in args.chunks_exact(2) {
        let mut duplicate = args.to_vec();
        duplicate.extend(pair);
        check(&duplicate, b"", "unknown/duplicate input producer argument");
    }
    check(&args[2..], b"", "input producer --config missing");
    let original = include_str!("fixtures/native-check-input.json");
    let recorded_root = "/home/runner/work/chrono-harness-example/chrono-harness-example";
    let relative = original.replace(recorded_root, ".");
    let root_error = "invalid check input request/source/root";
    check(&args, relative.as_bytes(), root_error);
    check(
        &args,
        original
            .replace(recorded_root, root.path().to_str().unwrap())
            .as_bytes(),
        root_error,
    );
    // A canonical alias reaches config decoding after the root check.
    check(
        &args,
        original
            .replace(recorded_root, alias.to_str().unwrap())
            .as_bytes(),
        "expected value",
    );
    let limit = 64 * 1024 * 1024;
    let mut padded = relative.into_bytes();
    padded.resize(limit - 1, b' ');
    check(&args, &padded, root_error);
    padded.push(b' ');
    check(&args, &padded, root_error);
    padded.push(b' ');
    check(&args, &padded, "check input request exceeds bound");
}
