#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {
fn failed_inventory_keeps_its_failed_outcome() {
    use super::policy::inventory_output::{inventory_summary, listing_status};
    let failed: Result<u8, u8> = Err(7);
    let valid: Result<u8, u8> = Ok(3);
    let status = listing_status(&failed);
    let passed = listing_status(&valid);
    let unsealed = inventory_summary(failed, Err(9u8));
    let sealed: Result<(), u8> = Ok(());
    assert(status@ == "failed"@);
    assert(passed@ == "passed"@);
    assert(unsealed is Err && unsealed->Err_0 == (Err::<u8, u8>(7u8), 9u8));
    let failed_but_sealed = inventory_summary(Err::<u8, u8>(7), sealed);
    let sealed_valid = inventory_summary(valid, sealed);
    assert(failed_but_sealed->Ok_0 == Err::<u8, u8>(7u8));
    assert(sealed_valid->Ok_0 == Ok::<u8, u8>(3u8));
}
}
