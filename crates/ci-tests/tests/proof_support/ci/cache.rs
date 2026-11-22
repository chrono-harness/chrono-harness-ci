#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn complete_release_jobs(registered: usize, selector: bool, projected: usize) -> (valid: bool)
    ensures valid == (projected as int == registered as int + if selector { 1int } else { 0int }),
{
    projected >= registered && projected - registered == if selector { 1 } else { 0 }
}
}
