use crate::{
    error::{Error, Result},
    process::capture,
};
use serde::Serialize;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

pub const REQUIRED_FLAGS: &[&str] = &[
    "--cd",
    "--sandbox",
    "--json",
    "--output-schema",
    "--output-last-message",
];

pub fn validate_help(help: &str) -> Result<()> {
    let missing: Vec<_> = REQUIRED_FLAGS
        .iter()
        .filter(|flag| !help.split_whitespace().any(|word| word == **flag))
        .copied()
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(Error::Preflight(format!(
            "Codex missing required flags: {}",
            missing.join(", ")
        )))
    }
}

#[derive(Debug, Serialize)]
pub struct LocalTools {
    pub codex_binary: PathBuf,
    pub codex_version: String,
    pub git_binary: PathBuf,
    pub git_version: String,
}

pub fn find_executable(name: &str) -> Result<PathBuf> {
    let path = std::env::var_os("PATH").ok_or_else(|| Error::Preflight("PATH not set".into()))?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if std::fs::metadata(&candidate)?.permissions().mode() & 0o111 == 0 {
                    continue;
                }
            }
            // Keep executable symlinks: some launchers depend on argv[0].
            return Ok(std::path::absolute(candidate)?);
        }
    }
    Err(Error::Preflight(format!(
        "{name} executable not found on PATH"
    )))
}

async fn text_command(binary: &Path, args: &[&str]) -> Result<String> {
    let args: Vec<OsString> = args.iter().map(Into::into).collect();
    let output = capture(
        binary,
        &args,
        None,
        b"",
        Duration::from_secs(10),
        1024 * 1024,
    )
    .await?;
    if output.code != Some(0) {
        return Err(Error::Preflight(
            "tool inspection returned non-zero status".into(),
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| Error::Preflight("tool inspection was not UTF-8".into()))
}

pub async fn inspect() -> Result<LocalTools> {
    let codex_binary = find_executable("codex")?;
    let git_binary = find_executable("git")?;
    let codex_version = text_command(&codex_binary, &["--version"])
        .await?
        .trim()
        .to_owned();
    validate_help(&text_command(&codex_binary, &["exec", "--help"]).await?)?;
    let git_version = text_command(&git_binary, &["--version"])
        .await?
        .trim()
        .to_owned();
    Ok(LocalTools {
        codex_binary,
        codex_version,
        git_binary,
        git_version,
    })
}

#[derive(Debug, Serialize)]
pub struct ProbeResult {
    pub thread_id: String,
    pub exit_code: i32,
    pub result: serde_json::Value,
}

/// Bounded, read-only connectivity check; does not execute a product workflow.
pub async fn readonly_probe(tools: &LocalTools) -> Result<ProbeResult> {
    use super::codex_event::Decoder;
    let temp = tempfile::tempdir()?;
    let repo = temp.path().join("repo");
    std::fs::create_dir(&repo)?;
    let git = capture(
        &tools.git_binary,
        &["init".into(), "--quiet".into(), repo.as_os_str().into()],
        None,
        b"",
        Duration::from_secs(10),
        1024 * 1024,
    )
    .await?;
    if git.code != Some(0) {
        return Err(Error::Preflight("probe Git init failed".into()));
    }
    let schema = temp.path().join("schema.json");
    let output = temp.path().join("result.json");
    std::fs::write(&schema, br#"{"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"],"additionalProperties":false}"#)?;
    let args: Vec<OsString> = vec![
        "exec".into(),
        "--cd".into(),
        repo.as_os_str().into(),
        "--sandbox".into(),
        "read-only".into(),
        "--json".into(),
        "--output-schema".into(),
        schema.as_os_str().into(),
        "--output-last-message".into(),
        output.as_os_str().into(),
        "-".into(),
    ];
    let captured = capture(
        &tools.codex_binary,
        &args,
        Some(&repo),
        b"Return the JSON object {\"ok\":true}. Do not edit files or run commands.",
        Duration::from_secs(120),
        8 * 1024 * 1024,
    )
    .await?;
    if captured.code != Some(0) {
        // Do not print untrusted stderr (which can contain credentials/configuration).
        return Err(Error::Preflight(format!(
            "Codex probe failed with exit {:?}; verify local Codex login/configuration",
            captured.code
        )));
    }
    let mut decoder = Decoder::new(8 * 1024 * 1024);
    let mut events = decoder.push(&captured.stdout)?;
    events.extend(decoder.finish()?);
    if events
        .iter()
        .any(|e| matches!(e["type"].as_str(), Some("turn.failed" | "error")))
    {
        return Err(Error::Preflight(
            "Codex probe emitted a failure event".into(),
        ));
    }
    if !events.iter().any(|e| e["type"] == "turn.completed") {
        return Err(Error::Preflight(
            "Codex probe did not complete a turn".into(),
        ));
    }
    let thread_id = events
        .iter()
        .find(|e| e["type"] == "thread.started")
        .and_then(|e| e["thread_id"].as_str())
        .filter(|id| !id.is_empty())
        .ok_or_else(|| Error::Preflight("missing Codex thread ID".into()))?
        .to_owned();
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(output)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(Error::Limit("probe result too large".into()));
    }
    let result: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| Error::Preflight("invalid probe result JSON".into()))?;
    if result != serde_json::json!({"ok": true}) {
        return Err(Error::Preflight(
            "probe result does not satisfy contract".into(),
        ));
    }
    Ok(ProbeResult {
        thread_id,
        exit_code: 0,
        result,
    })
}
