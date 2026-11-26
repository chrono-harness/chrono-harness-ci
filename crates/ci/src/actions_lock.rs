//! Resolve the two unit workflow actions from GitHub's native dependency lock.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn repository_identity(checkout: bool, owner: u64, repo: u64) -> (valid: bool)
    ensures valid == (owner == 44036562 && ((checkout && repo == 197814629)
        || (!checkout && repo == 192625955))),
{
    owner == 44036562 && repo == (if checkout { 197814629 } else { 192625955 })
}
}

#[cfg(not(verus_keep_ghost))]
pub(crate) const PATH: &str = ".github/workflows/actions.lock";

#[cfg(not(verus_keep_ghost))]
pub(crate) fn resolve(bytes: &[u8], workflow: &str) -> Result<(String, String), String> {
    use serde_yaml_ng::Value;
    // Value rejects duplicate mapping keys, including repeated dependency records.
    let lock: Value = serde_yaml_ng::from_slice(bytes).map_err(|e| format!("{PATH}: {e}"))?;
    if lock["version"].as_str() != Some("v0.0.3") {
        return Err(format!("{PATH}: unsupported workflow lock version"));
    }
    let uses = lock["workflows"][workflow]
        .as_sequence()
        .ok_or_else(|| format!("{PATH}: no dependencies for {workflow}"))?;
    let resolve_action = |repo: &str, checkout: bool| -> Result<String, String> {
        let mut matched = None;
        for item in uses {
            let name = item
                .as_str()
                .ok_or_else(|| format!("{PATH}: invalid entry in {workflow}"))?;
            let Some((source, reference)) = name.split_once('@') else {
                return Err(format!("{PATH}: invalid dependency key in {workflow}"));
            };
            if source.eq_ignore_ascii_case(repo) {
                if matched.is_some() || source != repo {
                    return Err(format!(
                        "{PATH}: ambiguous or noncanonical {repo} in {workflow}"
                    ));
                }
                matched = Some((name, reference));
            }
        }
        let (name, reference) =
            matched.ok_or_else(|| format!("{PATH}: missing {repo} in {workflow}"))?;
        let dep = &lock["dependencies"][name];
        let sha = dep["commit"].as_str().and_then(|v| v.strip_prefix("sha1-"));
        let valid_ref = !reference.is_empty()
            && reference
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'/'));
        if !valid_ref
            || dep
                .get("hostname")
                .is_some_and(|host| host.as_str() != Some("github.com"))
            || dep["ref"].as_str() != Some(reference)
            || !sha.is_some_and(|v| v.len() == 40 && crate::verified_literals::full_oid(v))
            || !matches!((dep["owner_id"].as_u64(), dep["repo_id"].as_u64()), (Some(owner), Some(repo)) if repository_identity(checkout, owner, repo))
        {
            return Err(format!(
                "{PATH}: invalid locked reference, commit or repository identity for {name}"
            ));
        }
        Ok(name.to_string())
    };
    Ok((
        resolve_action("actions/checkout", true)?,
        resolve_action("actions/upload-artifact", false)?,
    ))
}
