#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub enum CopyMember { Directory, File, Skip }

pub fn short_copy_member(directory: bool, file: bool) -> (member: CopyMember)
    ensures
        matches!(member, CopyMember::Directory) == directory,
        matches!(member, CopyMember::File) == (!directory && file),
        matches!(member, CopyMember::Skip) == (!directory && !file),
{
    if directory { CopyMember::Directory } else if file { CopyMember::File } else { CopyMember::Skip }
}
}
