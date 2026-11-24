//! The optional initial inventory embedded in an offline workflow projection.
use crate::{codec::is_digest, files::relative_path};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema: String,
    pub host_config: String,
    pub judges: Vec<Binding>,
    pub timeout_seconds: u64,
    pub stdout_limit_bytes: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub id: String,
    pub executable: String,
    pub version: String,
    pub sha256: Option<String>,
    pub argv: Vec<String>,
    pub selector: String,
    pub after: Vec<String>,
    pub modes: Vec<String>,
}
impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        relative_path(&self.host_config)?;
        if self.schema != "chrono-initial-check/v1"
            || !self.host_config.starts_with(".chrono-harness/")
            || self.timeout_seconds == 0
            || !(1..=64 * 1024 * 1024).contains(&self.stdout_limit_bytes)
        {
            return Err("invalid initial profile/config/limits".into());
        }
        let mut pending = BTreeMap::new();
        for b in &self.judges {
            relative_path(&b.executable)?;
            if b.version.is_empty() || !b.sha256.as_deref().is_some_and(is_digest) {
                return Err(
                    "initial judge requires an explicit version and executable digest".into(),
                );
            }
            if b.id.is_empty() || pending.insert(&b.id, &b.after).is_some() {
                return Err("duplicate/empty judge ID".into());
            }
            if b.selector != "every-initial" || b.modes != ["inventory"] {
                return Err("unsupported judge transport selection/mode".into());
            }
            if b.after.iter().collect::<BTreeSet<_>>().len() != b.after.len() {
                return Err("duplicate predecessor".into());
            }
        }
        if pending.is_empty() {
            return Err("no configured judges".into());
        }
        for predecessor in self.judges.iter().flat_map(|b| &b.after) {
            if !pending.contains_key(predecessor) {
                return Err(format!("missing predecessor {predecessor}"));
            }
        }
        while !pending.is_empty() {
            let (&id, _) = pending
                .iter()
                .find(|(_, after)| after.iter().all(|id| !pending.contains_key(id)))
                .ok_or("judge dependency cycle")?;
            pending.remove(id);
        }
        Ok(())
    }
}
