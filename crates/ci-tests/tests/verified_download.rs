//! 下载状态与停止条件的独立预期，同时由 Verus 和普通 Rust 测试执行。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../ci/src/verified_download.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_ci::verified_download as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

// Cargo builds this same release obligation as an executable for the publishing workflow.
mod release_observation {
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
    pub(super) fn evaluate_observations() -> Result<(), String> {
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
}

#[cfg(all(not(verus_keep_ghost), not(test)))]
fn main() -> Result<(), String> {
    release_observation::evaluate_observations()
}

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_accepts_the_expected_observations_only() {
    let accepted = release_observation::accepts(1, true, true);
    assert(accepted);
    let refused_1 = release_observation::accepts(2, true, true);
    assert(!refused_1);
    let refused_2 = release_observation::accepts(1, false, true);
    assert(!refused_2);
    let refused_3 = release_observation::accepts(1, true, false);
    assert(!refused_3);
}


fn expect_true(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

#[cfg(verus_keep_ghost)]
fn reappearing_on_a_different_page_is_ambiguous<P: policy::ArtifactPage>(
    pages: &[P], name: &str, first: usize, later: usize, first_row: usize, later_row: usize,
)
    requires first < later, later < pages.len(),
        first_row < pages@[first as int].names().len(), later_row < pages@[later as int].names().len(),
        pages@[first as int].names()[first_row as int] == Some(name@),
        pages@[later as int].names()[later_row as int] == Some(name@),
{
    let hit = policy::unique_artifact(pages, name);
    assert(policy::present(pages@,name@,first as int,first_row as int));
    assert(policy::present(pages@,name@,later as int,later_row as int));
    assert(hit is None);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn the_observed_run_head_id_and_url_gate_download() {
    use policy::{admit_artifact, ArtifactError};
    proof { reveal_strlit("different"); reveal_strlit("head"); reveal_strlit("url");
        assert("different"@.len() != "head"@.len());
        assert("different"@.len() != "url"@.len()); }
    let admitted = admit_artifact(Some(false),Some(9),9,Some("head"),"head",Some(7),Some("url"),"url");
    let expired = admit_artifact(Some(true),Some(9),9,Some("head"),"head",Some(7),None,"url");
    let run = admit_artifact(Some(false),Some(8),9,Some("head"),"head",None,None,"url");
    let head = admit_artifact(Some(false),Some(9),9,Some("different"),"head",Some(7),None,"url");
    let id = admit_artifact(Some(false),Some(9),9,Some("head"),"head",Some(0),None,"url");
    let url = admit_artifact(Some(false),Some(9),9,Some("head"),"head",Some(7),Some("different"),"url");
    expect_true(admitted.is_ok());
    expect_true(matches!(expired,Err(ArtifactError::Identity)));
    expect_true(matches!(run,Err(ArtifactError::Identity)));
    expect_true(matches!(head,Err(ArtifactError::Identity)));
    expect_true(matches!(id,Err(ArtifactError::MissingId)));
    expect_true(matches!(url,Err(ArtifactError::Endpoint)));
}

proof fn parsed_result_is_unique(bytes: Seq<u8>, left: Option<u16>, right: Option<u16>)
    requires policy::parsed(bytes, left), policy::parsed(bytes, right),
    ensures left == right,
    decreases if left.is_none() { 1nat } else { 0nat },
{
    match (left, right) {
        (Some(a), Some(b)) => {
            let i = choose|i: int| 0 <= i < bytes.len()
                && policy::token_at(bytes, i) && policy::code_at(bytes, i) == a;
            assert(policy::code_at(bytes, i) == b);
        },
        (Some(a), None) => {
            let i = choose|i: int| 0 <= i < bytes.len()
                && policy::token_at(bytes, i) && policy::code_at(bytes, i) == a;
            assert(!policy::prefix(bytes, bytes.len() as int, None));
            let (j, k) = choose|j: int, k: int| policy::token_at(bytes, j)
                && policy::token_at(bytes, k) && policy::code_at(bytes, j) != policy::code_at(bytes, k);
            assert(policy::code_at(bytes, j) == a && policy::code_at(bytes, k) == a);
        },
        (None, Some(b)) => {
            parsed_result_is_unique(bytes, right, left);
        },
        _ => {},
    }
}

fn expect(bytes: &[u8], expected: Option<u16>)
    requires policy::parsed(bytes@, expected),
{
    let actual = policy::http_status(bytes);
    proof { parsed_result_is_unique(bytes@, actual, expected); }
    expect_true(actual == expected);
}

#[cfg(not(verus_keep_ghost))]
#[test]
#[should_panic]
fn runtime_http_expectation_keeps_its_failure_branch() {
    expect(&[], Some(503));
}

#[cfg(not(verus_keep_ghost))]
#[test]
#[should_panic]
fn runtime_retry_expectation_keeps_its_failure_branch() {
    expect_true(false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_complete_ascii_status_tokens_are_accepted() {
    expect(&[], None);
    expect(&[72u8, 84, 84, 80, 32, 53, 48, 51], None);
    let valid: &[u8] = &[72u8, 84, 84, 80, 32, 53, 48, 51, 58];
    proof {
        assert(policy::token_at(valid@, 0));
        assert(policy::code_at(valid@, 0) == 503);
    }
    expect(valid, Some(503));
    expect(&[104u8, 84, 84, 80, 32, 53, 48, 51, 58], None);
    expect(&[72u8, 84, 84, 80, 32, 53, 255, 51, 58], None);
    expect(&[72u8, 84, 84, 80, 32, 53, 48, 51, 59], None);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn repeated_statuses_are_unambiguous_but_conflicting_statuses_are_rejected() {
    let repeated: &[u8] = &[
        72u8, 84, 84, 80, 32, 53, 48, 51, 58,
        72, 84, 84, 80, 32, 53, 48, 51, 58,
    ];
    proof {
        assert(policy::token_at(repeated@, 0) && policy::token_at(repeated@, 9));
        assert(policy::code_at(repeated@, 0) == 503 && policy::code_at(repeated@, 9) == 503);
        assert forall|i: int| policy::token_at(repeated@, i) implies i == 0 || i == 9 by {}
    }
    expect(repeated, Some(503));
    let conflict: &[u8] = &[
        72u8, 84, 84, 80, 32, 53, 48, 51, 58,
        72, 84, 84, 80, 32, 52, 48, 49, 58,
    ];
    proof {
        assert(policy::token_at(conflict@, 0) && policy::token_at(conflict@, 9));
        assert(policy::code_at(conflict@, 0) == 503 && policy::code_at(conflict@, 9) == 401);
    }
    expect(conflict, None);
    let embedded: &[u8] = &[255u8, 72, 84, 84, 80, 32, 48, 48, 48, 58, 0];
    proof {
        assert(policy::token_at(embedded@, 1));
        assert(policy::code_at(embedded@, 1) == 0);
        assert forall|i: int| policy::token_at(embedded@, i) implies i == 1 by {}
    }
    expect(embedded, Some(0));
    let maximum: &[u8] = &[72u8, 84, 84, 80, 32, 57, 57, 57, 58];
    proof {
        assert(policy::token_at(maximum@, 0));
        assert(policy::code_at(maximum@, 0) == 999);
    }
    expect(maximum, Some(999));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn every_missing_retry_obligation_stops_another_download() {
    let allowed = policy::retry(false, false, true, true, 1, 1, true);
    let successful = policy::retry(true, false, true, true, 1, 1, true);
    let failed_process = policy::retry(false, true, true, true, 1, 1, true);
    let exhausted = policy::retry(false, false, false, true, 1, 1, true);
    let unlisted = policy::retry(false, false, true, false, 1, 1, true);
    let no_stdout = policy::retry(false, false, true, true, 0, 1, true);
    let no_stderr = policy::retry(false, false, true, true, 1, 0, true);
    let deadline = policy::retry(false, false, true, true, 1, 1, false);
    expect_true(allowed && !successful && !failed_process && !exhausted && !unlisted
        && !no_stdout && !no_stderr && !deadline);
}

}
