//! Bound event acquisition; the selected reader owns process evidence.
use crate::{Config, full_oid};
use serde_json::{Value, json};
use std::path::Path;

pub(crate) struct EventGit {
    runtime: std::cell::RefCell<crate::runtime_session::Session>,
    pub bound: bool,
}
impl EventGit {
    pub(crate) fn open(
        root: &Path,
        program: &str,
        config: &str,
        candidate: &str,
        observe_failure: bool,
    ) -> Result<Self, String> {
        crate::units::installed_runner_sha256(root, program)?;
        let mut runtime =
            crate::runtime_session::Session::open(root, &root.join(program), "git-facts-v1")?;
        #[derive(serde::Deserialize)]
        struct Header {
            bound: bool,
        }
        let header:Header=runtime.call(json!({"mode":"config","config":config,"base":candidate,"candidate":candidate,"observe_failure":observe_failure}))?;
        Ok(Self {
            runtime: std::cell::RefCell::new(runtime),
            bound: header.bound,
        })
    }
    pub(crate) fn new(root: &Path, c: &Config, candidate: &str) -> Result<Self, String> {
        if !full_oid(candidate) {
            return Err("invalid full candidate OID".into());
        }
        let config = c
            .facts_config
            .as_deref()
            .ok_or("CI requires facts_config")?;
        let facts = Self::open(root, &c.runner, config, candidate, false)?;
        if !facts.bound {
            return Err("CI requires a full v3 facts_config".into());
        }
        facts.verify_config(candidate)?;
        if facts.head().map_err(|e| facts.error(e))?.trim() != candidate {
            return Err(facts.error("checkout is not exact event candidate".into()));
        }
        Ok(facts)
    }
    fn request<T: serde::de::DeserializeOwned>(&self, request: Value) -> Result<T, String> {
        self.runtime.borrow_mut().call(request)
    }
    pub(crate) fn head(&self) -> Result<String, String> {
        self.request(json!({"op":"head"}))
    }
    pub(crate) fn check_ref(&self, reference: &str) -> Result<(), String> {
        self.request(json!({"op":"check-reference","reference":reference}))
    }
    pub(crate) fn verify_config(&self, candidate: &str) -> Result<(), String> {
        self.request(json!({"op":"verify-config","candidate":candidate}))
    }
    pub(crate) fn blob(&self, commit: &str, path: &str) -> Result<Vec<u8>, String> {
        self.request(json!({"op":"blob-bytes","commit":commit,"path":path}))
    }
    pub(crate) fn show(&self, commit: &str, path: &str) -> Result<String, String> {
        self.request(json!({"op":"show-text","commit":commit,"path":path}))
    }
    pub(crate) fn available(&self, commit: &str) -> Result<bool, String> {
        self.request(json!({"op":"commit-available","commit":commit}))
    }
    pub(crate) fn fetch(&self, commit: &str, shallow: bool) -> Result<String, String> {
        self.request(json!({"op":"fetch-commit","commit":commit,"shallow":shallow}))
    }
    pub(crate) fn require_commit(&self, oid: &str) -> Result<(), String> {
        if !self.available(oid)? {
            self.fetch(oid, true)?;
            if !self.available(oid)? {
                return Err("fetched commit remains missing".into());
            }
        }
        if self
            .request::<String>(json!({"op":"resolve-commit","commit":oid}))?
            .trim()
            != oid
        {
            return Err("commit identity mismatch".into());
        }
        Ok(())
    }
    pub(crate) fn remote_baseline(&self, reference: &str) -> Result<String, String> {
        self.check_ref(reference)?;
        let observed: String =
            self.request(json!({"op":"remote-reference","reference":reference}))?;
        let fields: Vec<_> = observed.split_whitespace().collect();
        if fields.len() != 2 || fields[1] != reference || !full_oid(fields[0]) {
            return Err(format!(
                "missing or ambiguous remote baseline ref: {reference}"
            ));
        }
        let base = fields[0].to_owned();
        self.require_commit(&base)?;
        Ok(base)
    }
    pub(crate) fn changed_paths(
        &self,
        base: Option<&str>,
        candidate: &str,
    ) -> Result<Vec<String>, String> {
        self.request(json!({"op":"changed-paths","base":base,"candidate":candidate}))
    }
    pub(crate) fn parents(&self, commit: &str) -> Result<Vec<String>, String> {
        self.request(json!({"op":"parents","commit":commit}))
    }
    pub(crate) fn pull_request_baseline(
        &self,
        workflow_revision: &str,
        head: &str,
    ) -> Result<String, String> {
        self.require_commit(workflow_revision)?;
        let parents = self.parents(workflow_revision)?;
        let base = parents.first().map(String::as_str).unwrap_or("");
        let second = parents.get(1).map(String::as_str).unwrap_or("");
        if !chrono_harness::verified_identifiers::valid_merge_parents(
            parents.len(),
            base.as_bytes(),
            second.as_bytes(),
            head.as_bytes(),
        ) {
            return Err("pull-request workflow revision must have exactly two parents and the event head as second parent".into());
        }
        Ok(parents[0].clone())
    }
    pub(crate) fn close(self) -> Result<(), String> {
        self.runtime.into_inner().close()
    }
    pub(crate) fn observation(&self) -> Result<Value, String> {
        self.request(json!({"op":"observation"}))
    }
    pub(crate) fn record(&self, context: &mut Value) -> Result<(), String> {
        context["git_facts"] = self.observation()?;
        Ok(())
    }
    pub(crate) fn error(&self, message: String) -> String {
        match self.observation() {
            Ok(observation) => format!(
                "E_CI_GIT: {}",
                json!({"message":message,"git_facts":observation})
            ),
            Err(error) => format!(
                "E_CI_GIT: {}",
                json!({"message":message,"observation_error":error})
            ),
        }
    }
}
