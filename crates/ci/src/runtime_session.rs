//! Blocking JSON lines connection to a single native runner capability.
#[cfg(not(verus_keep_ghost))]
use serde::de::DeserializeOwned;
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::{
    io::{BufRead, BufReader, Write},
    os::{fd::OwnedFd, unix::net::UnixStream},
    path::Path,
    process::{Child, ChildStdout, Stdio},
};

#[cfg(not(verus_keep_ghost))]
pub(crate) struct Session {
    child: Child,
    pub(crate) socket: Option<UnixStream>,
    output: BufReader<ChildStdout>,
    name: &'static str,
}
#[cfg(not(verus_keep_ghost))]
impl Session {
    pub fn open(root: &Path, program: &Path, name: &'static str) -> Result<Self, String> {
        let (socket, input) = UnixStream::pair().map_err(|e| e.to_string())?;
        let mut command = crate::storage::command(program)?;
        command
            .arg(name)
            .current_dir(root)
            .stdin(Stdio::from(OwnedFd::from(input)))
            .stdout(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|e| format!("{} {name}: {e}", program.display()))?;
        let output = BufReader::new(child.stdout.take().expect("piped stdout"));
        Ok(Self {
            child,
            socket: Some(socket),
            output,
            name,
        })
    }
    pub(crate) fn query<T: DeserializeOwned>(
        root: &Path,
        runner: &str,
        capability: &'static str,
        input: Value,
    ) -> Result<T, String> {
        let runner = crate::units::installed_runner_path(root, runner)?;
        let mut session = Self::open(root, &runner, capability)?;
        let response = session.call(input)?;
        session.close()?;
        Ok(response)
    }

    pub(crate) fn selected_config(root: &Path, runner: &str, path: &str) -> Result<Value, String> {
        let program = crate::units::installed_runner_path(root, runner)?;
        let mut child = crate::storage::command(&program)?
            .arg("selected-host-config-v1")
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let stdin = child.stdin.take().unwrap();
        let sent = serde_json::to_writer(stdin, path);
        let output = child.wait_with_output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim_end().into());
        }
        sent.map_err(|e| e.to_string())?;
        crate::decode(&output.stdout)
    }

    pub fn call<T: DeserializeOwned>(&mut self, command: Value) -> Result<T, String> {
        let stdin = self.socket.as_mut().unwrap();
        serde_json::to_writer(&mut *stdin, &command).map_err(|e| e.to_string())?;
        stdin
            .write_all(b"\n")
            .and_then(|_| stdin.flush())
            .map_err(|e| e.to_string())?;
        let mut line = Vec::new();
        self.output
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if !complete_line(&line) {
            return Err("E_PROTOCOL: incomplete runner response".into());
        }
        serde_json::from_slice::<Result<T, String>>(&line)
            .map_err(|e| format!("E_PROTOCOL: {e}"))?
    }
    pub fn close(mut self) -> Result<(), String> {
        if let Some(socket) = self.socket.take() {
            let _ = socket.shutdown(std::net::Shutdown::Write);
        }
        let status = self.child.wait().map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("{} exited {status}", self.name))
        }
    }
}
#[cfg(not(verus_keep_ghost))]
impl Drop for Session {
    fn drop(&mut self) {
        if let Some(socket) = self.socket.take() {
            let _ = socket.shutdown(std::net::Shutdown::Write);
        }
        let _ = self.child.wait();
    }
}

#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
verus! {
pub fn complete_line(bytes: &[u8]) -> (valid: bool)
    ensures valid == (bytes.len() > 0 && bytes@[bytes.len()-1] == 10u8),
{
    bytes.len() > 0 && bytes[bytes.len()-1] == 10u8
}
}
