#[path = "../worktree/support/diagnostic_copy.rs"]
mod diagnostic_copy;

#[cfg(verus_keep_ghost)]
pub use diagnostic_copy::{CopyMember, short_copy_member};
