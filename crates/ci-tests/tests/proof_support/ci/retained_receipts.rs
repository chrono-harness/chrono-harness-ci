#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn needs_failure_batch(main_present: bool, linked_present: bool) -> (needed: bool)
    ensures needed == (main_present || linked_present),
{
    main_present || linked_present
}
}
