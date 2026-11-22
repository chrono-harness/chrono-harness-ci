#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// Collection without selected units must not acquire a second parent observation.
pub(super) fn parent_lookup_count(selected: bool) -> (count: usize)
    ensures selected ==> count == 2, !selected ==> count == 1,
{
    if selected { 2 } else { 1 }
}
}
