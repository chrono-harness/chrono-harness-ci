//! Blocking JSON lines connection to a single native runner capability.
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

pub(crate) struct Session {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    name: &'static str,
}
impl Session {
    pub fn open(root: &Path, program: &Path, name: &'static str) -> Result<Self, String> {
        let mut command = Command::new(program);
        command
            .arg("exec-with-descriptors-v1")
            .arg(program)
            .arg(name)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|e| format!("{} {name}: {e}", program.display()))?;
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().expect("piped stdout"));
        Ok(Self {
            child,
            input,
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
        let mut child = Command::new(&program)
            .arg("exec-with-descriptors-v1")
            .arg(&program)
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
        let stdin = self.input.as_mut().unwrap();
        serde_json::to_writer(&mut *stdin, &command).map_err(|e| e.to_string())?;
        stdin
            .write_all(b"\n")
            .and_then(|_| stdin.flush())
            .map_err(|e| e.to_string())?;
        let mut line = Vec::new();
        self.output
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if line.last() != Some(&b'\n') {
            return Err("E_PROTOCOL: incomplete runner response".into());
        }
        serde_json::from_slice::<Result<T, String>>(&line)
            .map_err(|e| format!("E_PROTOCOL: {e}"))?
    }
    pub fn close(mut self) -> Result<(), String> {
        drop(self.input.take());
        let status = self.child.wait().map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("{} exited {status}", self.name))
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        drop(self.input.take());
        let _ = self.child.wait();
    }
}
