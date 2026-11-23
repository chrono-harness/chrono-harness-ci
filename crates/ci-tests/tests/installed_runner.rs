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
