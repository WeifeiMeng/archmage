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
