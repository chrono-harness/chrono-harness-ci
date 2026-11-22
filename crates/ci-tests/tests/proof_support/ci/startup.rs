#[cfg(verus_keep_ghost)]
use super::policy::gating::unit_evidence_retention;

#[cfg(verus_keep_ghost)]
use super::policy::startup::retention_days_valid;

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn startup_retention_rejects_zero_and_allows_one_day() {
    let omitted = retention_days_valid(None);
    let default_requested = retention_days_valid(Some(0));
    let one_day = retention_days_valid(Some(1));
    proof { assert(omitted && !default_requested && one_day); }
    #[cfg(not(verus_keep_ghost))]
    assert!(omitted && !default_requested && one_day);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn independent_evidence_window_for_unit_and_aggregate() {
    let unit = unit_evidence_retention(Some(1), true);
    let aggregate = unit_evidence_retention(Some(1), false);
    let omitted = unit_evidence_retention(None, true);
    proof { assert(unit == Some(1u16) && aggregate.is_none() && omitted.is_none()); }
    #[cfg(not(verus_keep_ghost))]
    assert_eq!((unit, aggregate, omitted), (Some(1), None, None));
}
}

#[test]
fn shared_producer_preserves_original_cache_save_identity() {
    let mut value = source("examples/ci-host-job-gating/.chrono-harness/ci/units.json");
    value["job_gating"]["startup"] = registration();
    value["persistent_cache"] = adoption("detect");
    let cache = value["persistent_cache"]["jobs"]["detect"]["caches"].clone();
    value["persistent_cache"]["jobs"]["detect"]["save_after_bootstrap"] = cache;
    let config = serde_json::from_value(value).unwrap();
    let rendered = chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").unwrap();
    let yaml: Value =
        serde_yaml_ng::from_str(&rendered[".github/workflows/chrono-ci.yml"]).unwrap();
    let steps = yaml["jobs"]["detect"]["steps"].as_array().unwrap();
    assert_eq!(
        steps.iter().filter(|s| s["id"] == "chrono_startup").count(),
        1
    );
    assert!(!steps.iter().any(|s| s["id"] == "chrono_cache_bootstrap"));
    for step in steps.iter().filter(|s| {
        s["name"]
            .as_str()
            .unwrap_or("")
            .starts_with("Save registered cache")
    }) {
        assert!(
            step["if"]
                .as_str()
                .unwrap()
                .contains("steps.chrono_startup.outcome == 'success'")
        );
    }
}
