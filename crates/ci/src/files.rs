//! File and executable identities for the CI provider.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub open spec fn hex_ascii(n: int) -> int { if n < 10 { 48 + n } else { 87 + n } }

fn hex_char(n: u8) -> (encoded: u8)
    requires n < 16,
    ensures encoded == hex_ascii(n as int), encoded < 128,
{
    if n < 10 { 48 + n } else { 97 + (n - 10) }
}

pub open spec fn encoded_at(digest: Seq<u8>, i: int) -> int {
    let byte = digest[i / 2] as int;
    hex_ascii(if i % 2 == 0 { byte / 16 } else { byte % 16 })
}

/// The file and JSON wire formats carry two lowercase ASCII digits per SHA-256 byte.
pub fn digest_ascii(digest: &[u8; 32]) -> (text: [u8; 64])
    ensures forall|j: int| 0 <= j < 64 ==> #[trigger] text@[j] == encoded_at(digest@, j),
{
    let mut text = [0u8; 64];
    let mut i = 0usize;
    while i < 64
        invariant i <= 64, forall|j: int| 0 <= j < i ==> #[trigger] text@[j] == encoded_at(digest@, j),
        decreases 64 - i,
    {
        let byte = digest[i / 2];
        text[i] = hex_char(if i % 2 == 0 { byte / 16 } else { byte % 16 });
        i += 1;
    }
    text
}
}

#[cfg(not(verus_keep_ghost))]
fn digest_text(digest: [u8; 32]) -> String {
    String::from_utf8(digest_ascii(&digest).to_vec()).expect("hexadecimal digits are ASCII")
}
#[cfg(not(verus_keep_ghost))]
use sha2::{Digest, Sha256};
#[cfg(not(verus_keep_ghost))]
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[cfg(not(verus_keep_ghost))]
pub fn sha256(data: &[u8]) -> String {
    digest_text(Sha256::digest(data).into())
}
#[cfg(not(verus_keep_ghost))]
fn copy_hashed(mut input: impl Read, mut output: impl Write) -> Result<(String, u64), String> {
    let mut buffer = [0; 65536];
    let mut hash = Sha256::new();
    let mut total = 0u64;
    loop {
        let n = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        hash.update(&buffer[..n]);
        total = total.checked_add(n as u64).ok_or("stream size overflow")?;
    }
    Ok((digest_text(hash.finalize().into()), total))
}
#[cfg(not(verus_keep_ghost))]
pub fn file_identity(path: &Path) -> Result<(String, u64), String> {
    if !fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .is_file()
    {
        return Err(format!("not a regular input file: {}", path.display()));
    }
    let input = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !input.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err(format!("not a regular input file: {}", path.display()));
    }
    copy_hashed(input, std::io::sink())
}
#[cfg(not(verus_keep_ghost))]
pub fn resolve_program(
    root: &Path,
    program: &str,
    search: Option<&str>,
) -> Result<PathBuf, String> {
    let root = std::path::absolute(root).map_err(|e| e.to_string())?;
    let error = || format!("executable not found: {program}");
    if program.contains('/') {
        let candidate = root.join(program);
        return candidate.is_file().then_some(candidate).ok_or_else(error);
    }
    let path = search
        .map(std::ffi::OsString::from)
        .or_else(|| std::env::var_os("PATH"))
        .ok_or("PATH is missing")?;
    std::env::split_paths(&path)
        .map(|dir| root.join(dir).join(program))
        .find(|candidate| candidate.is_file())
        .ok_or_else(error)
}
/// Check the runner wire's lexical relative path before using native host filesystem APIs.
#[cfg(not(verus_keep_ghost))]
pub fn relative_path(relative: &str) -> Result<(), String> {
    if relative.is_empty()
        || relative == "."
        || relative.starts_with("./")
        || relative.starts_with('/')
        || relative.split('/').any(|part| part == "..")
        || relative
            .bytes()
            .any(|b| matches!(b, 0 | b'\n' | b'\r' | b'\\'))
    {
        return Err(format!("invalid relative path: {relative:?}"));
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
pub fn no_symlink_parents(root: &Path, relative: &str) -> Result<PathBuf, String> {
    relative_path(relative)?;
    let mut path = root.to_owned();
    for part in Path::new(relative).components() {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(format!("symlink path is not allowed: {}", path.display()));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(path)
}
