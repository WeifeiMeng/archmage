use crate::error::{Error, Result};
use serde::Serialize;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Serialize)]
pub struct Config {
    pub data_root: PathBuf,
    pub max_parallelism: usize,
    pub provider_capacity: usize,
    pub node_timeout_seconds: u64,
    pub max_line_bytes: usize,
    pub max_result_bytes: usize,
    pub max_stream_bytes: usize,
}

impl Config {
    /// Resolves paths without creating state. Environment reading belongs to the caller.
    pub fn resolve(override_root: Option<&Path>, source: Option<&Path>) -> Result<Self> {
        let default =
            directories::ProjectDirs::from("dev", "archmage", "archmage").ok_or_else(|| {
                Error::Preflight("cannot determine application data directory".into())
            })?;
        let root = override_root.unwrap_or(default.data_local_dir());
        if root.as_os_str().is_empty() || root.components().any(|c| c == Component::ParentDir) {
            return Err(Error::Preflight(
                "data root must not be empty or contain '..'".into(),
            ));
        }
        let data_root = resolve_existing_ancestor(root)?;
        if data_root.parent().is_none()
            || directories::BaseDirs::new().is_some_and(|b| b.home_dir() == data_root)
            || data_root == fs::canonicalize(std::env::temp_dir())?
        {
            return Err(Error::Preflight(
                "data root must be a dedicated application directory".into(),
            ));
        }
        if data_root.exists() && !data_root.is_dir() {
            return Err(Error::Preflight("data root is not a directory".into()));
        }
        if let Some(repo) = source {
            let repo = fs::canonicalize(repo)?;
            if data_root.starts_with(repo) {
                return Err(Error::Preflight(
                    "data root must be outside source repository".into(),
                ));
            }
        }
        Ok(Self {
            data_root,
            max_parallelism: 2,
            provider_capacity: 2,
            node_timeout_seconds: 1800,
            max_line_bytes: 8 * 1024 * 1024,
            max_result_bytes: 8 * 1024 * 1024,
            max_stream_bytes: 128 * 1024 * 1024,
        })
    }

    pub fn prepare(&self) -> Result<()> {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&self.data_root)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if fs::metadata(&self.data_root)?.permissions().mode() & 0o077 != 0 {
                return Err(Error::Preflight(
                    "existing data directory must have mode 0700".into(),
                ));
            }
        }
        // An actual create detects ACL/read-only-filesystem failures, not just mode bits.
        let _probe = tempfile::NamedTempFile::new_in(&self.data_root)?;
        Ok(())
    }
}

/// Canonicalize the longest existing prefix, including symlinks, without creating directories.
fn resolve_existing_ancestor(path: &Path) -> Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let mut cursor = absolute.as_path();
    let mut missing = Vec::new();
    loop {
        match fs::symlink_metadata(cursor) {
            Ok(_) => break,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                missing.push(
                    cursor
                        .file_name()
                        .ok_or_else(|| Error::Preflight("invalid data root".into()))?,
                );
                cursor = cursor
                    .parent()
                    .ok_or_else(|| Error::Preflight("invalid data root".into()))?;
            }
            Err(e) => return Err(e.into()),
        }
    }
    let mut resolved = fs::canonicalize(cursor)?;
    if !resolved.is_dir() {
        return Err(Error::Preflight(
            "data root ancestor is not a directory".into(),
        ));
    }
    for part in missing.into_iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}
