#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {
fn failed_inventory_keeps_its_failed_outcome() {
    let status = super::policy::inventory_output::listing_status(false);
    assert(status@ == "failed"@);
}
}
