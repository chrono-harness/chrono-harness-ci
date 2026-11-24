//! CI produces the current preparation wire response; the installed owner validates it and retains its originals.
use crate::runtime_session::Session;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
pub const RESPONSE: &str = "chrono-check-inputs/v1";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Selection {
    All,
    Unit { unit: String },
    Collect,
}
impl Selection {
    pub fn argv(&self) -> Vec<String> {
        match self {
            Self::All => vec![],
            Self::Unit { unit } => vec!["--unit".into(), unit.clone()],
            Self::Collect => vec!["--collect".into()],
        }
    }
    pub fn matches(&self, scope: &Option<crate::units::Scope>) -> bool {
        match (self, scope) {
            (Self::All, None) | (Self::Collect, Some(crate::units::Scope::Collect { .. })) => true,
            (Self::Unit { unit: a }, Some(crate::units::Scope::Unit { unit: b })) => a == b,
            _ => false,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputRequest {
    pub schema: String,
    pub host_root: PathBuf,
    pub source: String,
    pub host_config: String,
    pub host_config_sha256: String,
    pub effective_config: String,
    pub effective_config_sha256: String,
    pub profile: String,
    pub profile_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_profile: Option<ProfileBinding>,
    pub selection: Selection,
    pub native_artifacts: Option<NativeArtifacts>,
    // Local collection uses the CI collection owner after local endpoints were observed.
    pub prepared: Option<PreparedCheck>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileBinding {
    pub path: String,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub path: String,
    pub raw: Vec<u8>,
    pub sha256: String,
    pub semantic_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCheck {
    pub schema: String,
    pub request_sha256: String,
    pub source: String,
    pub profile: String,
    pub base: Option<String>,
    pub candidate: String,
    pub initial: bool,
    pub context: Option<Context>,
    pub scope: Option<crate::units::Scope>,
    pub evidence: Value,
    pub originals: Vec<Original>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeArtifacts {
    pub config_path: String,
    pub config_sha256: String,
    pub directory: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Original {
    pub path: String,
    pub sha256: String,
}

pub(crate) struct Originals(Session);
impl Originals {
    pub(crate) fn open(root: &Path, runner: &str) -> Result<Self, String> {
        let path = crate::units::installed_runner_path(root, runner)?;
        Session::open(root, &path, "retained-artifacts-v1").map(Self)
    }
    pub(crate) fn request_directory(&mut self, request: &InputRequest) -> Result<String, String> {
        self.0
            .call(json!({"operation":"validate-preparation-request","request":request}))
    }
    pub(crate) fn validate_result(
        &mut self,
        request: &InputRequest,
        result: &PreparedCheck,
    ) -> Result<(), String> {
        self.0.call(
            json!({"operation":"validate-preparation-result","request":request,"result":result}),
        )
    }
    pub(crate) fn retain(
        &mut self,
        directory: &str,
        prefix: &str,
        bytes: &[u8],
    ) -> Result<Original, String> {
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        self.0.call(json!({"operation":"retain-original-text","directory":directory,"prefix":prefix,"text":text}))
    }
    pub(crate) fn identify(&mut self, path: &str) -> Result<Original, String> {
        self.0
            .call(json!({"operation":"identify-original","path":path}))
    }
    pub(crate) fn verify(&mut self, original: &Original) -> Result<(), String> {
        self.0
            .call(json!({"operation":"verify-original","original":original}))
    }
    pub(crate) fn close(self) -> Result<(), String> {
        self.0.close()
    }
}

impl InputRequest {
    pub fn profile_for(&self, initial: bool) -> Result<&str, String> {
        if initial && self.initial_profile.is_some() {
            if self.selection != Selection::All {
                return Err("initial inventory requires the standalone bare check".into());
            }
            Ok(&self
                .initial_profile
                .as_ref()
                .ok_or("initial profile binding missing")?
                .path)
        } else {
            Ok(&self.profile)
        }
    }
}
