#[cfg(verus_keep_ghost)]
#[path = "actions_lock.rs"]
mod lock_expectations;

#[cfg(verus_keep_ghost)]
#[path = "proof_support/ci/cache.rs"]
mod cache_expectations;

#[cfg(verus_keep_ghost)]
#[path = "proof_support/ci/gating.rs"]
mod gating_expectations;

#[cfg(verus_keep_ghost)]
#[path = "proof_support/ci/initial_short.rs"]
mod initial_expectations;

#[cfg(verus_keep_ghost)]
#[path = "../../ci/src/verified_literals.rs"]
mod policy;

#[cfg(verus_keep_ghost)]
#[path = "proof_support/ci/release.rs"]
mod release_expectations;

#[cfg(verus_keep_ghost)]
#[path = "proof_support/ci/retained_receipts.rs"]
mod retained_receipts;

#[cfg(verus_keep_ghost)]
#[path = "proof_support/ci/short.rs"]
mod short_copy;

#[cfg(verus_keep_ghost)]
#[path = "proof_support/ci/startup.rs"]
mod startup_expectations;

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

#[cfg(verus_keep_ghost)]
verus! {
#[cfg(verus_keep_ghost)]
fn wire_integer_boundary() {
    let exact = policy::codec::needs_binary64(9007199254740991);
    let boundary = policy::codec::needs_binary64(9007199254740992);
    let rounded = policy::codec::needs_binary64(9007199254740993);
    assert(!exact && boundary && rounded);
}

#[cfg(verus_keep_ghost)]
fn digest_wire_pairs(digest: &[u8; 32], at: usize)
    requires at < 32,
{
    let text = policy::files::digest_ascii(digest);
    assert(text@[2 * at as int] == policy::files::hex_ascii(digest@[at as int] as int / 16));
    assert(text@[2 * at as int + 1] == policy::files::hex_ascii(digest@[at as int] as int % 16));
}


#[cfg(verus_keep_ghost)]
fn required_check_input_admission(n: usize, flag: &str, seen: bool, value: Option<&str>, absolute: bool, same: bool) {
    use policy::check_inputs::*;
    let paired = paired_arguments(n);
    assert(paired == (n > 1 && n % 2 != 0));
    let accepted = admissible_option(flag, seen);
    assert(accepted == (!seen && (flag@ == "--event-env"@ || flag@ == "--payload-env"@
        || flag@ == "--revision-env"@ || flag@ == "--repository-env"@ || flag@ == "--config"@)));
    let config = config_argument(value);
    match value {
        None => { assert(config is Err); }
        Some(path) => { assert(config is Ok && config.unwrap()@ == path@); }
    }
    let bound = bounded_request(n);
    assert(bound == (n < 67108865));
    let compare = || -> (result: Result<bool, String>)
        ensures result == Ok::<bool, String>(same),
    { Ok(same) };
    let observed = source_root(absolute, compare);
    assert(observed == Ok::<bool, String>(absolute && same));
}

#[cfg(verus_keep_ghost)]
fn a_two_parent_event_selects_only_the_first_parent(parents: &[String], head: &str)
    requires parents.len() == 2,
        policy::object_id(vstd::utf8::encode_utf8(parents@[0]@)), policy::object_id(head.spec_bytes()),
        parents@[1]@ == head@,
{
    let selected = policy::pull_request_base(parents, head);
    assert(selected matches Some(base) && base@ == parents@[0]@);
}

#[cfg(verus_keep_ghost)]
fn wrong_ancestry_cannot_supply_a_delta_baseline(parents: &[String], head: &str)
    requires parents.len() != 2 || parents@[1]@ != head@,
{
    let selected = policy::pull_request_base(parents, head);
    assert(selected is None);
}

#[cfg(verus_keep_ghost)]
fn cache_actions_require_the_same_registered_jobs_and_distinct_outputs() {
    use policy::cache_action as action;
    proof {
        reveal_strlit("a"); reveal_strlit("b");
        reveal_strlit(".chrono-harness/ci/");
        assert("a"@[0] != "b"@[0]);
    }
    let ordinary = action::invalid_configuration(&["a", "b"], &["a", "b"], false);
    let different = action::invalid_configuration(&["a"], &["b"], false);
    let empty = action::invalid_configuration(&[], &[], false);
    let downloads = action::invalid_configuration(&["a"], &["a"], true);
    let shared = action::output_path(".github/actions/build/action.yml".as_bytes(), false);
    let missing_prefix = action::registration("chrono-github-cache-actions/v1", &[]);
    assert(ordinary.is_none());
    assert(matches!(different, Some(action::RegistrationError::MissingJob)));
    assert(matches!(empty, Some(action::RegistrationError::MissingJob)));
    assert(matches!(downloads, Some(action::RegistrationError::Downloads)));
    assert(!shared && !missing_prefix);
}

#[cfg(verus_keep_ghost)]
fn receipt_failure_batch_exists_for_either_checkout() {
    let empty = retained_receipts::needs_failure_batch(false, false);
    let main_only = retained_receipts::needs_failure_batch(true, false);
    let linked_only = retained_receipts::needs_failure_batch(false, true);
    let both = retained_receipts::needs_failure_batch(true, true);
    assert(!empty && main_only && linked_only && both);
}

#[cfg(verus_keep_ghost)]
fn failed_fixture_copies_regular_files_and_descends_only_into_directories() {
    let file = short_copy::short_copy_member(false, true);
    assert(matches!(file, short_copy::CopyMember::File));
    let directory = short_copy::short_copy_member(true, false);
    assert(matches!(directory, short_copy::CopyMember::Directory));
    let other = short_copy::short_copy_member(false, false);
    assert(matches!(other, short_copy::CopyMember::Skip));
}

#[cfg(verus_keep_ghost)]
fn initial_inventory_rejects_unregistered_files_and_collection_avoids_extra_acquisition() {
    let registered = initial_expectations::initial_inventory_exit(false);
    let unregistered = initial_expectations::initial_inventory_exit(true);
    let empty = gating_expectations::parent_lookup_count(false);
    let selected = gating_expectations::parent_lookup_count(true);
    assert(registered == 0 && unregistered == 1);
    assert(empty == 1 && selected == 2);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn native_job_terminal_outcomes_never_select_another_detail() {
    proof {
        reveal_strlit("completed"); reveal_strlit("success"); reveal_strlit("failure");
        assert("success"@[0] != "failure"@[0]);
    }
    let passed = policy::native_job_state(Some("completed"),Some("success"),Some(false),None);
    let failed = policy::native_job_state(Some("completed"),Some("failure"),Some(false),Some(1));
    assert(matches!(passed, policy::NativeJobState::Succeeded));
    assert(matches!(failed, policy::NativeJobState::Rejected));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn native_job_detail_requires_an_explicit_null_and_positive_exact_id() {
    proof {
        reveal_strlit("in_progress"); reveal_strlit("completed"); reveal_strlit("queued");
        reveal_strlit("waiting"); reveal_strlit("pending"); reveal_strlit("requested");
        assert("in_progress"@.len() != "completed"@.len());
    }
    let detail = policy::native_job_state(Some("in_progress"),None,Some(true),Some(91));
    let missing = policy::native_job_state(Some("in_progress"),None,None,Some(91));
    let valued = policy::native_job_state(Some("in_progress"),None,Some(false),Some(91));
    let zero = policy::native_job_state(Some("in_progress"),None,Some(true),Some(0));
    let absent = policy::native_job_state(Some("in_progress"),None,Some(true),None);
    match detail { policy::NativeJobState::ReadDetail { id } => assert(id == 91), _ => assert(false) }
    assert(matches!(missing,policy::NativeJobState::Rejected));
    assert(matches!(valued,policy::NativeJobState::Rejected));
    assert(matches!(zero,policy::NativeJobState::Rejected));
    assert(matches!(absent,policy::NativeJobState::Rejected));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn native_job_fields_keep_absent_distinct_from_present_and_compare_all_bytes() {
    let one = [1u8,2];
    let different = [1u8,3];
    proof { assert(one@ =~= seq![1u8,2]); assert(different@ =~= seq![1u8,3]); }
    let same = policy::observed_field_equal(Some(&one),Some(&one));
    let changed = policy::observed_field_equal(Some(&one),Some(&different));
    let absent = policy::observed_field_equal(None,Some(&one));
    let both_absent = policy::observed_field_equal(None,None);
    assert(same && !changed && !absent && both_absent);
    let short = policy::native_job_binding(&[(None,None)]);
    assert(!short);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_publisher_lookup_keeps_the_first_full_match() {
    proof {
        reveal_strlit("build"); reveal_strlit("publish"); reveal_strlit("publishing");
        assert("build"@.len() != "publish"@.len());
        assert("publishing"@.len() != "publish"@.len());
    }
    let jobs = ["build","publish","publish"];
    proof { assert(jobs@ =~= seq!["build","publish","publish"]); }
    let selected = policy::publisher_index(&jobs,"publish");
    let prefix = policy::publisher_index(&jobs,"publishing");
    assert(selected == Some(1usize) && prefix == None::<usize>);
    let dependencies = policy::publisher_dependencies(1,false);
    let missing = policy::publisher_dependencies(0,false);
    let always = policy::publisher_dependencies(1,true);
    assert(dependencies && !missing && !always);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_requires_v2_and_the_successful_workflow_trigger() {
    proof {
        reveal_strlit("chrono-github-release/v2"); reveal_strlit("chrono-github-release/v1");
        assert("chrono-github-release/v2"@[23] != "chrono-github-release/v1"@[23]);
    }
    let valid = policy::delivery_mode("chrono-github-release/v2",0);
    let push = policy::delivery_mode("chrono-github-release/v2",1);
    let legacy = policy::delivery_mode("chrono-github-release/v1",0);
    assert(valid && !push && !legacy);
}

#[cfg(verus_keep_ghost)]
fn delivery_checkout_uses_the_landing_source_and_build_keeps_its_source() {
    let delivery = policy::delivery_source(true);
    let build = policy::delivery_source(false);
    assert(delivery@ == "${{ github.event.workflow_run.head_sha || inputs.source || github.sha }}"@);
    assert(build@ == "${{ inputs.source || github.sha }}"@);
}

// The native workflow test checks the emitted guard at runtime.
#[cfg(verus_keep_ghost)]
fn push_only_cache_saving_has_an_exact_native_event_guard() {
    let push = policy::cache_save_event_guard(true);
    let every_event = policy::cache_save_event_guard(false);
    assert(push@ == "github.event_name == 'push' && "@);
    assert(every_event@ == ""@);
}

#[cfg(verus_keep_ghost)]
fn download_layout_requires_one_lock_and_preserves_whole_file_domains() {
    use policy::download_projection::{download_domain, DownloadDomain};
    let ordinary = download_domain(false, 2);
    let pinned = download_domain(true, 1);
    let missing = download_domain(true, 0);
    let ambiguous = download_domain(true, 2);
    assert(ordinary is Files);
    assert(pinned is InstallLayout);
    assert(missing is Invalid && ambiguous is Invalid);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn download_contracts_reject_each_missing_obligation() {
    let good = policy::valid_download_entry(1, 1, 1, 1, true, true, true, true);
    assert(good);
    let absent_paths = policy::valid_download_entry(0, 1, 1, 1, true, true, true, true);
    let absent_domain = policy::valid_download_entry(1, 0, 1, 1, true, true, true, true);
    let absent_sources = policy::valid_download_entry(1, 1, 0, 1, true, true, true, true);
    let absent_consumer = policy::valid_download_entry(1, 1, 1, 0, true, true, true, true);
    let duplicate = policy::valid_download_entry(1, 1, 1, 1, false, true, true, true);
    let bad_writer = policy::valid_download_entry(1, 1, 1, 1, true, false, true, true);
    let incompatible = policy::valid_download_entry(1, 1, 1, 1, true, true, false, true);
    let no_purpose = policy::valid_download_entry(1, 1, 1, 1, true, true, true, false);
    assert(!absent_paths && !absent_domain && !absent_sources && !absent_consumer
        && !duplicate && !bad_writer && !incompatible && !no_purpose);
    let missing = policy::single_download_binding(0);
    let one = policy::single_download_binding(1);
    let ambiguous = policy::single_download_binding(2);
    assert(!missing && one && !ambiguous);
    let empty = policy::valid_download_path(&[]);
    let path = policy::valid_download_path(&[97u8, 47, 98, 46, 108, 111, 99, 107]);
    let wildcard = policy::valid_download_path(&[97u8, 47, 42]);
    let quote = policy::valid_download_path(&[97u8, 39]);
    assert(!empty && path && !wildcard && !quote);
    let artifact = policy::valid_download_artifact(Some("p"), "p", Some("o"), "o", Some(false), Some("download-cache"));
    let tracked_result = policy::valid_download_artifact(Some("p"), "p", Some("o"), "o", Some(true), Some("download-cache"));
    let missing_kind = policy::valid_download_artifact(Some("p"), "p", Some("o"), "o", Some(false), None);
    let missing_owner = policy::valid_download_artifact(Some("p"), "p", None, "o", Some(false), Some("download-cache"));
    let wrong_owner = policy::valid_download_artifact(Some("p"), "p", Some("p"), "o", Some(false), Some("download-cache"));
    let evidence = policy::valid_download_artifact(Some("p"), "p", Some("o"), "o", Some(false), Some("execution-evidence"));
    proof {
        reveal_strlit("p"); reveal_strlit("o");
        reveal_strlit("download-cache"); reveal_strlit("execution-evidence");
        assert("p"@[0] != "o"@[0]);
        assert("download-cache"@.len() != "execution-evidence"@.len());
    }
    assert(artifact && !tracked_result && !missing_kind && !missing_owner && !wrong_owner && !evidence);
}

fn expect_cache_registry(schema: Option<&str>, primary_checkout: bool, expected: bool)
    requires expected == policy::cache_registry(schema, primary_checkout),
{
    let actual = policy::valid_cache_registry(schema, primary_checkout);
    assert(actual == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cache_registry_requires_current_schema_and_primary_checkout() {
    proof {
        reveal_strlit("chrono-cache/v2"); reveal_strlit("chrono-cache/v1");
        reveal_strlit("chrono-cache/v20"); reveal_strlit("chrono-cache/v2\n");
        reveal_strlit("");
        assert("chrono-cache/v2"@[14] != "chrono-cache/v1"@[14]);
        assert("chrono-cache/v2"@.len() != "chrono-cache/v20"@.len());
        assert("chrono-cache/v2"@.len() != "chrono-cache/v2\n"@.len());
        assert("chrono-cache/v2"@.len() != ""@.len());
    }
    expect_cache_registry(Some("chrono-cache/v2"), true, true);
    expect_cache_registry(Some("chrono-cache/v2"), false, false);
    expect_cache_registry(Some("chrono-cache/v1"), true, false);
    expect_cache_registry(Some("chrono-cache/v20"), true, false);
    expect_cache_registry(Some("chrono-cache/v2\n"), true, false);
    expect_cache_registry(Some(""), true, false);
    expect_cache_registry(None, true, false);
    expect_cache_registry(None, false, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cache_names_keep_the_ascii_alphabet_and_128_byte_limit() {
    let alphabet = policy::valid_cache_name(&[48u8, 57, 65, 90, 97, 122, 45, 95, 46, 58]);
    let longest = policy::valid_cache_name(&[97u8; 128]);
    let too_long = policy::valid_cache_name(&[97u8; 129]);
    let empty = policy::valid_cache_name(&[]);
    let slash = policy::valid_cache_name(&[97u8, 47, 98]);
    let space = policy::valid_cache_name(&[97u8, 32]);
    let unicode = policy::valid_cache_name(&[228u8, 184, 173]);
    let nul = policy::valid_cache_name(&[97u8, 0]);
    assert(alphabet && longest && !too_long && !empty && !slash && !space && !unicode && !nul);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cache_text_membership_preserves_full_fields_and_nonadjacent_duplicates() {
    proof {
        reveal_strlit("a"); reveal_strlit("bb"); reveal_strlit("ccc");
        assert("a"@.len() != "bb"@.len());
        assert("a"@.len() != "ccc"@.len());
        assert("bb"@.len() != "ccc"@.len());
    }
    let values = ["a", "bb", "ccc"];
    let duplicate = ["a", "bb", "a"];
    proof {
        assert(values@ =~= seq!["a", "bb", "ccc"]);
        assert(duplicate@ =~= seq!["a", "bb", "a"]);
    }
    let found = policy::cache_has_text(&values, "bb");
    let absent = policy::cache_has_text(&["bb", "ccc"], "a");
    let unique = policy::cache_has_unique_text(&values);
    let repeated = policy::cache_has_unique_text(&duplicate);
    let empty_unique = policy::cache_has_unique_text(&[]);
    let reordered = policy::cache_is_subset(&["ccc", "a"], &values);
    let outside = policy::cache_is_subset(&["a"], &["bb", "ccc"]);
    let no_saves = policy::cache_is_subset(&[], &values);
    assert(found && !absent && unique && !repeated && empty_unique && reordered && !outside && no_saves);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cache_preparation_keeps_empty_arguments_but_requires_a_literal_program() {
    proof {
        reveal_strlit("a"); reveal_strlit(""); reveal_strlit("${{");
        reveal_strlit("a\n"); reveal_strlit("a\r"); reveal_strlit("a\0");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("a") && vstd::string::is_ascii("")
            && vstd::string::is_ascii("${{") && vstd::string::is_ascii("a\n")
            && vstd::string::is_ascii("a\r") && vstd::string::is_ascii("a\0"));
        assert("a".spec_bytes() =~= seq![97u8]);
        assert("".spec_bytes() =~= Seq::<u8>::empty());
        assert("${{".spec_bytes() =~= seq![36u8, 123, 123]);
        assert("a\n".spec_bytes() =~= seq![97u8, 10]);
        assert("a\r".spec_bytes() =~= seq![97u8, 13]);
        assert("a\0".spec_bytes() =~= seq![97u8, 0]);
    }
    let valid = policy::valid_cache_preparation(&["a", ""]);
    let no_program = policy::valid_cache_preparation(&[]);
    let empty_program = policy::valid_cache_preparation(&["", "a"]);
    let template = policy::valid_cache_preparation(&["a", "${{"]);
    let newline = policy::valid_cache_preparation(&["a", "a\n"]);
    let carriage = policy::valid_cache_preparation(&["a", "a\r"]);
    let nul = policy::valid_cache_preparation(&["a", "a\0"]);
    assert(valid && !no_program && !empty_program && !template && !newline && !carriage && !nul);
}

fn expect_native_cache_consumer(
    known_job: bool, name: &str, need: &str, output_use: &str, preparation: &[&str],
    caches: &[&str], saves: &[&str], after_bootstrap: &[&str], expected: bool,
)
    requires expected == policy::cache_consumer(known_job, name, need, output_use,
        preparation@, caches@, saves@, after_bootstrap@),
{
    let actual = policy::valid_cache_consumer(known_job, name, need, output_use,
        preparation, caches, saves, after_bootstrap);
    assert(actual == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn native_cache_consumers_require_each_obligation_and_keep_read_only_jobs() {
    proof {
        reveal_strlit("a"); reveal_strlit("bb"); reveal_strlit("");
        assert("a"@.len() != "bb"@.len());
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("a") && vstd::string::is_ascii("bb")
            && vstd::string::is_ascii(""));
        assert("a".spec_bytes() =~= seq![97u8]);
        assert("bb".spec_bytes() =~= seq![98u8, 98]);
        assert("".spec_bytes() =~= Seq::<u8>::empty());
    }
    expect_native_cache_consumer(true, "a", "a", "a", &["a", ""], &["a", "bb"], &["bb", "a"], &["a"], true);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a"], &[], &[], true);
    expect_native_cache_consumer(false, "a", "a", "a", &["a"], &["a"], &[], &[], false);
    expect_native_cache_consumer(true, "", "a", "a", &["a"], &["a"], &[], &[], false);
    expect_native_cache_consumer(true, "a", "", "a", &["a"], &["a"], &[], &[], false);
    expect_native_cache_consumer(true, "a", "a", "", &["a"], &["a"], &[], &[], false);
    expect_native_cache_consumer(true, "a", "a", "a", &[], &["a"], &[], &[], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &[], &[], &[], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a", "a"], &[], &[], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a"], &["a", "a"], &[], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a"], &["bb"], &[], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a"], &["a"], &["a", "a"], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a", "bb"], &["a"], &["bb"], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a"], &[], &["a"], false);
}

fn expect(bytes: &[u8], expected: bool)
    requires expected == policy::literal(bytes@),
{
    let actual = policy::valid_literal(bytes);
    assert(actual == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_and_ordinary_utf8_text_are_accepted() {
    expect(&[], true);
    expect(&[97u8, 32, 9, 127], true);
    expect(&[228u8, 184, 173, 230, 150, 135], true);
    expect(&[36u8, 72, 79, 77, 69, 32, 96, 97, 96], true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn forbidden_bytes_fail_at_both_ends_and_inside_text() {
    expect(&[0u8], false);
    expect(&[10u8], false);
    expect(&[13u8], false);
    expect(&[0u8, 97], false);
    expect(&[97u8, 10, 98], false);
    expect(&[97u8, 13], false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_template_openers_fail_at_every_boundary() {
    expect(&[36u8, 123, 123], false);
    expect(&[36u8, 123, 123, 97], false);
    expect(&[97u8, 36, 123, 123], false);
    expect(&[97u8, 36, 123, 123, 98], false);
    expect(&[36u8, 36, 123, 123], false);
    expect(&[36u8, 123, 36, 123, 123], false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn partial_openers_and_nonadjacent_bytes_are_accepted() {
    expect(&[36u8], true);
    expect(&[36u8, 123], true);
    expect(&[123u8, 123], true);
    expect(&[36u8, 123, 97, 123], true);
    expect(&[36u8, 123, 36, 123], true);
    expect(&[36u8, 123, 32, 123], true);
}

}

#[cfg(not(verus_keep_ghost))]
#[test]
fn installed_runner_hashes_across_the_read_buffer_boundary() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("runner"), vec![b'a'; 65_537]).unwrap();
    assert_eq!(
        chrono_ci::units::installed_runner_sha256(root.path(), "runner").unwrap(),
        "008ffc88d3c96a9f307524eb361e47c5222a887fc45fa0c1fb8d429c5c23b430"
    );
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn nested_event_digest_matches_the_runner_binary64_wire() {
    let event = serde_json::json!({
        "key": "9007199254740993",
        "array": [9_007_199_254_740_991u64, 9_007_199_254_740_992u64,
            9_007_199_254_740_993u64, -9_007_199_254_740_993i64, u64::MAX]
    });
    // Independent digest of the runner's canonical bytes, including the rounded u64 maximum.
    assert_eq!(
        chrono_ci::value_digest(&event).unwrap(),
        "189685511b045bfa1f343d2c6c7dc720eaf7369102c0dc44da88753ccfac85f7"
    );
}
