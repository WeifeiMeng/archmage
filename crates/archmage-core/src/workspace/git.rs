use crate::error::{Error, Result};
use std::{ffi::OsStr, path::Path, process::Command};

pub(crate) fn output(cwd: &Path, args: &[&OsStr]) -> Result<Vec<u8>> {
    let result = Command::new("git").args(args).current_dir(cwd).output()?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(Error::Workspace(format!(
            "git {:?} exited {:?}: {}",
            args,
            result.status.code(),
            stderr.trim()
        )));
    }
    Ok(result.stdout)
}

pub(crate) fn text(cwd: &Path, args: &[&OsStr]) -> Result<String> {
    let bytes = output(cwd, args)?;
    let value = std::str::from_utf8(&bytes)
        .map_err(|_| Error::Workspace("git returned non-UTF-8 metadata".into()))?;
    Ok(value.trim().to_owned())
}

pub(crate) fn run(cwd: &Path, args: &[&OsStr]) -> Result<()> {
    output(cwd, args).map(|_| ())
}
