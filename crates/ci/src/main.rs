use serde_json::{Value, json};
use std::{
    ffi::OsString,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

fn work(
    arguments: &[OsString],
    mut event: impl FnMut(Value) -> Result<(), String>,
) -> Result<u8, String> {
    let mut decoded = Vec::new();
    for (index, raw) in arguments.iter().enumerate() {
        let Some(value) = raw.to_str() else {
            event(json!({"invalid_argument":{"argv_index":index,"bytes":raw.as_encoded_bytes()}}))?;
            return Ok(2);
        };
        decoded.push(value.to_string());
    }
    match chrono_ci::dispatch(decoded.get(1..).unwrap_or_default()) {
        Ok(output) => {
            print!("{output}");
            Ok(0)
        }
        Err(error) => {
            let message = format!("E_CI: {error}");
            eprintln!("{message}");
            event(json!({"diagnostic":message}))?;
            Ok(1)
        }
    }
}

fn logged(root: &Path, arguments: &[OsString]) -> Result<u8, String> {
    let operation = arguments.get(1).and_then(|a| a.to_str()).unwrap_or("");
    let invalid = arguments.iter().any(|a| a.to_str().is_none());
    match root.join(".chrono-harness/logging.json").symlink_metadata() {
        Err(error) if error.kind() == io::ErrorKind::NotFound && !invalid => {
            return work(arguments, |_| Ok(()));
        }
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error.to_string()),
        _ => {}
    }
    let local = root.join(".chrono-harness/bin/chrono-diagnostics");
    let diagnostics = if local.is_file() {
        local.as_path()
    } else {
        Path::new("chrono-diagnostics")
    };
    let mut child = Command::new(diagnostics)
        .arg("cli-events-v1")
        .arg(root)
        .args(["chrono-ci", operation])
        .arg(std::process::id().to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("chrono-diagnostics cli-events-v1: {e}"))?;
    let mut writer = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let result: Result<u8, String> = (|| {
        let mut reply = String::new();
        reader.read_line(&mut reply).map_err(|e| e.to_string())?;
        if serde_json::from_str::<Value>(&reply).ok() != Some(json!({"ready":true})) {
            return Err("chrono-diagnostics did not open the CLI log".into());
        }
        let mut send = |value: Value| -> Result<(), String> {
            serde_json::to_writer(&mut writer, &value).map_err(|e| e.to_string())?;
            writer
                .write_all(b"\n")
                .and_then(|()| writer.flush())
                .map_err(|e| e.to_string())
        };
        let status = work(arguments, &mut send)?;
        send(json!({"exit_code":status}))?;
        Ok(status)
    })();
    drop(writer);
    let status = child.wait().map_err(|e| e.to_string())?;
    let result = result?;
    if status.code() != Some(i32::from(result)) {
        return Err(format!(
            "chrono-diagnostics cli-events-v1 exited {status} after {result}"
        ));
    }
    Ok(result)
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().collect();
    let root = arguments
        .get(1..)
        .unwrap_or_default()
        .windows(2)
        .find(|p| p[0] == "--host-root")
        .map(|pair| Ok(PathBuf::from(&pair[1])))
        .unwrap_or_else(std::env::current_dir);
    match root
        .map_err(|e| e.to_string())
        .and_then(|root| logged(&root, &arguments))
    {
        Ok(status) => ExitCode::from(status),
        Err(error) => {
            eprintln!("E_RUNTIME_LOG: {error}");
            ExitCode::from(2)
        }
    }
}
