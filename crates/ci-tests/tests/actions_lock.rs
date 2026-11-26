#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {

#[cfg(verus_keep_ghost)]
fn required_unit_lock_source(path: Option<&str>, checkout: &str, upload: &str) {
    let source = super::policy::units::lock_selected(path, checkout, upload);
    assert((source == Ok::<bool, &str>(false)) == path.is_none());
    if path.is_some() {
        assert((source == Ok::<bool, &str>(true)) == (path.unwrap()@ == ".github/workflows/actions.lock"@ && checkout@.len() == 0 && upload@.len() == 0));
    }
}
#[cfg(verus_keep_ghost)]
fn required_migration_preserves_local_work(actual: &[u8], expected: &[u8]) {
    let matched = super::policy::migrate::previous_bytes_match(actual, expected);
    assert(matched == (actual@ == expected@));
}
#[cfg(verus_keep_ghost)]
fn required_workflow_action_owners(owner: u64, repository: u64) {
    let checkout = super::policy::actions_lock::repository_identity(true, owner, repository);
    let upload = super::policy::actions_lock::repository_identity(false, owner, repository);
    assert(checkout == (repository == 197814629 && owner == 44036562));
    assert(upload == (repository == 192625955 && owner == 44036562));
    assert(!(checkout && upload));
}

}

#[cfg(not(verus_keep_ghost))]
mod projection {
    use chrono_ci::units::{Config, render, render_from};

    const HOST: &str = include_str!("../../../.chrono-harness/ci/units.json");
    const SOURCE: &str = ".chrono-harness/ci/units.json";
    const WORKFLOW: &str = ".github/workflows/check.yml";
    const LOCK_PATH: &str = ".github/workflows/actions.lock";
    const LOCK: &str = include_str!("fixtures/github-actions.lock");
    const CHECKOUT: &str = "actions/checkout@11d5960a326750d5838078e36cf38b85af677262";
    const UPLOAD: &str = "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02";

    fn host(locked: bool) -> Config {
        let mut config: Config = serde_json::from_str(HOST).unwrap();
        config.collection.workflow_path = WORKFLOW.into();
        config.actions_lock = locked.then(|| LOCK_PATH.into());
        config.collection.checkout_action = if locked { "" } else { CHECKOUT }.into();
        config.collection.upload_artifact_action = if locked { "" } else { UPLOAD }.into();
        config
    }

    #[test]
    fn official_lock_changes_only_action_references_and_requires_its_single_source() {
        let pins = render_from(&host(false), SOURCE, |_| panic!("direct SHA read a lock")).unwrap();
        let config = host(true);
        let output = render_from(&config, SOURCE, |path| {
            assert_eq!(path, LOCK_PATH);
            Ok(LOCK.as_bytes().to_vec())
        })
        .unwrap();
        assert_eq!(
            output[WORKFLOW],
            pins[WORKFLOW]
                .replace(CHECKOUT, "actions/checkout@v4.4.0")
                .replace(UPLOAD, "actions/upload-artifact@v4.6.2")
        );
        assert!(render(&config, SOURCE).is_err());
        assert!(
            render_from(&config, SOURCE, |_| Err("missing fixed blob".into()))
                .unwrap_err()
                .contains("missing fixed blob")
        );
        let mut mixed = config;
        mixed.collection.checkout_action = CHECKOUT.into();
        assert!(render_from(&mixed, SOURCE, |_| panic!("duplicate authority")).is_err());
        mixed.collection.checkout_action.clear();
        mixed.actions_lock = Some(".github/other.lock".into());
        assert!(render_from(&mixed, SOURCE, |_| panic!("wrong lock source")).is_err());
    }

    #[test]
    fn missing_ambiguous_or_substituted_workflow_dependencies_fail() {
        let config = host(true);
        let malformed = [
            LOCK.replace("v0.0.3", "v0.0.4"),
            LOCK.replace(WORKFLOW, ".github/workflows/other.yml"),
            LOCK.replace("        - 'actions/checkout@v4.4.0'\n", ""),
            LOCK.replace(
                "        - 'actions/checkout@v4.4.0'",
                "        - 'actions/checkout@v4.4.0'\n        - 'actions/checkout@v5'",
            ),
            LOCK.replace("actions/checkout@", "my-org/checkout@"),
            LOCK.replace(
                "    'actions/checkout@v4.4.0':",
                "    'actions/checkout@v5.0.0':",
            ),
            LOCK.replace("ref: 'v4.4.0'", "ref: 'v4.3.0'"),
            LOCK.replace(
                "ref: 'v4.4.0'",
                "ref: 'v4.4.0'\n        hostname: 'outside.ghe.com'",
            ),
            LOCK.replace("sha1-11d5960a326750d5838078e36cf38b85af677262", "sha1-abc"),
            LOCK.replace("44036562", "1"),
            LOCK.replace("197814629", "192625955"),
            LOCK.replace(
                "repo_id: 197814629",
                "repo_id: 197814629\n        repo_id: 197814629",
            ),
            LOCK.replace("v4.4.0", "v4 # comment"),
        ];
        for invalid in malformed {
            assert_ne!(invalid, LOCK);
            assert!(
                render_from(&config, SOURCE, |_| Ok(invalid.as_bytes().to_vec())).is_err(),
                "{invalid}"
            );
        }
    }
}
