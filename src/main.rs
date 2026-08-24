use birdcage::process::{Command, Stdio};
use birdcage::{Birdcage, Exception, Sandbox};
use serde::Deserialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DESCRIPTION: &str = r#"{"name":"bash","description":"Execute Bash in an isolated workspace without network or credential access. Returns stdout and stderr. Commands time out after 120 seconds by default.","parameters":{"type":"object","properties":{"command":{"type":"string","description":"Bash command to run"},"timeout":{"type":"number","minimum":1,"maximum":600,"description":"Timeout in seconds (default 120, maximum 600)"}},"required":["command"],"additionalProperties":false},"snippet":"Execute isolated Bash commands"}"#;
const MAX_INPUT: u64 = 1024 * 1024;
const MAX_OUTPUT: usize = 16 * 1024;
const DEFAULT_TIMEOUT: u64 = 120;
const MAX_TIMEOUT: u64 = 600;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    command: String,
    timeout: Option<u64>,
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = match arguments.as_slice() {
        [operation] if operation == "describe" => {
            println!("{DESCRIPTION}");
            Ok(())
        }
        [operation, name] if operation == "run" && name == "bash" => run(),
        [operation, _] if operation == "run" => Err((2, "unknown tool".to_string())),
        _ => Err((2, "usage: bashx describe | bashx run bash".to_string())),
    };
    if let Err((status, message)) = result {
        eprintln!("bashx: {message}");
        std::process::exit(status);
    }
}

fn run() -> Result<(), (i32, String)> {
    let mut input = Vec::new();
    std::io::stdin()
        .take(MAX_INPUT + 1)
        .read_to_end(&mut input)
        .map_err(runtime_error)?;
    if input.len() as u64 > MAX_INPUT {
        return Err((2, "input exceeds 1 MiB".to_string()));
    }
    let arguments: Arguments =
        serde_json::from_slice(&input).map_err(|error| (2, format!("invalid input: {error}")))?;
    if arguments
        .timeout
        .is_some_and(|timeout| timeout == 0 || timeout > MAX_TIMEOUT)
    {
        return Err((2, format!("timeout must be between 1 and {MAX_TIMEOUT}")));
    }
    let timeout = arguments.timeout.unwrap_or(DEFAULT_TIMEOUT);
    let workspace = workspace()?;
    std::env::set_current_dir(&workspace).map_err(runtime_error)?;
    clear_environment(&workspace);

    let mut sandbox = Birdcage::new();
    allow_system(&mut sandbox)?;
    sandbox
        .add_exception(Exception::WriteAndRead(workspace))
        .map_err(sandbox_error)?;
    for name in ["HOME", "PATH", "TMPDIR", "LANG", "LC_ALL"] {
        sandbox
            .add_exception(Exception::Environment(name.to_string()))
            .map_err(sandbox_error)?;
    }

    let mut command = Command::new("/bin/bash");
    command
        .args(["--noprofile", "--norc", "-c", &arguments.command])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = sandbox.spawn(command).map_err(sandbox_error)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| runtime("missing stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| runtime("missing stderr"))?;
    let stdout_thread = std::thread::spawn(move || read_limited(stdout));
    let stderr_thread = std::thread::spawn(move || read_limited(stderr));
    let deadline = Instant::now() + Duration::from_secs(timeout);
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(runtime_error)? {
            break Some(status);
        }
        if Instant::now() >= deadline {
            child.kill().map_err(runtime_error)?;
            let _ = child.wait();
            timed_out = true;
            break None;
        }
        std::thread::sleep(Duration::from_millis(1));
    };
    let stdout = stdout_thread
        .join()
        .map_err(|_| runtime("stdout reader failed"))??;
    let stderr = stderr_thread
        .join()
        .map_err(|_| runtime("stderr reader failed"))??;
    let mut output = stdout;
    if !stderr.is_empty() {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(&stderr);
    }
    if timed_out {
        append_line(
            &mut output,
            &format!("error: command timed out after {timeout} seconds"),
        );
    } else if let Some(status) = status
        && !status.success()
    {
        append_line(&mut output, &format!("error: {status}"));
    }
    if output.is_empty() {
        output.push_str("Command completed without output");
    }
    print!("{}", tail(&output));
    std::io::stdout().flush().map_err(runtime_error)
}

fn workspace() -> Result<PathBuf, (i32, String)> {
    let path = std::env::var_os("BASHX_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    std::fs::canonicalize(path).map_err(runtime_error)
}

fn clear_environment(workspace: &Path) {
    let variables: Vec<_> = std::env::vars_os().map(|(name, _)| name).collect();
    for name in variables {
        unsafe { std::env::remove_var(name) };
    }
    unsafe {
        std::env::set_var("HOME", workspace);
        std::env::set_var("TMPDIR", workspace);
        std::env::set_var("PATH", "/usr/local/bin:/usr/bin:/bin");
        std::env::set_var("LANG", "C.UTF-8");
        std::env::set_var("LC_ALL", "C.UTF-8");
    }
}

fn allow_system(sandbox: &mut Birdcage) -> Result<(), (i32, String)> {
    for path in ["/bin", "/usr", "/lib", "/lib64"] {
        if Path::new(path).exists() {
            sandbox
                .add_exception(Exception::ExecuteAndRead(path.into()))
                .map_err(sandbox_error)?;
        }
    }
    for path in [
        "/etc/ld.so.cache",
        "/etc/localtime",
        "/etc/passwd",
        "/etc/group",
        "/etc/ssl/certs",
    ] {
        if Path::new(path).exists() {
            sandbox
                .add_exception(Exception::Read(path.into()))
                .map_err(sandbox_error)?;
        }
    }
    for path in ["/dev/null", "/dev/urandom"] {
        if Path::new(path).exists() {
            sandbox
                .add_exception(Exception::WriteAndRead(path.into()))
                .map_err(sandbox_error)?;
        }
    }
    Ok(())
}

fn read_limited(mut reader: impl Read) -> Result<String, (i32, String)> {
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        let count = reader.read(&mut chunk).map_err(runtime_error)?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() > MAX_OUTPUT {
            bytes.drain(..bytes.len() - MAX_OUTPUT);
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn append_line(output: &mut String, line: &str) {
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    output.push_str(line);
}

fn tail(output: &str) -> &str {
    if output.len() <= MAX_OUTPUT {
        return output;
    }
    let mut start = output.len() - MAX_OUTPUT;
    while !output.is_char_boundary(start) {
        start += 1;
    }
    &output[start..]
}

fn runtime(message: &str) -> (i32, String) {
    (1, message.to_string())
}

fn runtime_error(error: impl std::fmt::Display) -> (i32, String) {
    (1, error.to_string())
}

fn sandbox_error(error: impl std::fmt::Display) -> (i32, String) {
    (1, format!("sandbox failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{DESCRIPTION, MAX_OUTPUT, tail};

    #[test]
    fn descriptor_is_valid() {
        let descriptor: serde_json::Value = serde_json::from_str(DESCRIPTION).unwrap();
        assert_eq!(descriptor["name"], "bash");
    }

    #[test]
    fn output_tail_is_bounded() {
        let output = "x".repeat(MAX_OUTPUT + 100);
        assert_eq!(tail(&output).len(), MAX_OUTPUT);
    }
}
