#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// The existing root-inventory assertions require failure for unregistered files.
pub(super) fn initial_inventory_exit(unregistered: bool) -> (exit: i32)
    ensures unregistered ==> exit == 1, !unregistered ==> exit == 0,
{
    if unregistered { 1 } else { 0 }
}
}
