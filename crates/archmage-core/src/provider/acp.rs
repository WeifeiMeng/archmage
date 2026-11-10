use crate::error::{Error, Result};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcpAccess {
    ReadOnly,
    WorkspaceWrite,
}

#[derive(Clone, Debug)]
pub struct AcpWorkspacePolicy {
    root: PathBuf,
    canonical_root: PathBuf,
    access: AcpAccess,
}

impl AcpWorkspacePolicy {
    pub fn new(root: &Path, access: AcpAccess) -> Result<Self> {
        let canonical_root = root.canonicalize()?;
        if !canonical_root.is_dir() {
            return Err(Error::Preflight(
                "ACP workspace root must be a directory".into(),
            ));
        }
        Ok(Self {
            root: root.to_path_buf(),
            canonical_root,
            access,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn access(&self) -> AcpAccess {
        self.access
    }

    pub fn resolve(&self, path: &Path, for_write: bool) -> Result<PathBuf> {
        if !path.is_absolute() {
            return Err(Error::Preflight("ACP file path must be absolute".into()));
        }
        if for_write && self.access == AcpAccess::ReadOnly {
            return Err(Error::Preflight("ACP workspace is read-only".into()));
        }
        if path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(Error::Preflight("ACP path traversal is not allowed".into()));
        }
        let resolved = if path.exists() {
            path.canonicalize()?
        } else {
            let mut ancestor = path;
            while !ancestor.exists() {
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| Error::Preflight("ACP path has no existing ancestor".into()))?;
            }
            let canonical_ancestor = ancestor.canonicalize()?;
            canonical_ancestor.join(
                path.strip_prefix(ancestor)
                    .map_err(|_| Error::Preflight("ACP path could not be resolved".into()))?,
            )
        };
        if !resolved.starts_with(&self.canonical_root) {
            return Err(Error::Preflight(
                "ACP path is outside the Run worktree".into(),
            ));
        }
        Ok(resolved)
    }

    pub async fn read_text(
        &self,
        path: &Path,
        line: Option<u32>,
        limit: Option<u32>,
    ) -> Result<String> {
        let path = self.resolve(path, false)?;
        let content = tokio::fs::read_to_string(path).await?;
        let start = line.unwrap_or(1).max(1) as usize - 1;
        let count = limit.map_or(usize::MAX, |value| value as usize);
        Ok(content
            .split_inclusive('\n')
            .skip(start)
            .take(count)
            .collect())
    }

    pub async fn write_text(&self, path: &Path, content: &str) -> Result<()> {
        let path = self.resolve(path, true)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    pub fn validate_terminal(&self, cwd: &Path, command: &str, args: &[String]) -> Result<()> {
        let cwd = self.resolve(cwd, false)?;
        if !cwd.is_dir() {
            return Err(Error::Preflight(
                "ACP terminal cwd must be a directory".into(),
            ));
        }
        let command_name = Path::new(command)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(command);
        if command_name.is_empty()
            || matches!(command_name, "sudo" | "doas" | "su" | "mount" | "diskutil")
        {
            return Err(Error::Preflight(
                "ACP terminal command is not allowed".into(),
            ));
        }
        if self.access == AcpAccess::ReadOnly
            && !matches!(
                command_name,
                "rg" | "grep" | "find" | "pwd" | "ls" | "git" | "cat" | "head" | "tail"
            )
        {
            return Err(Error::Preflight(
                "ACP read-only capability cannot run this command".into(),
            ));
        }
        if command_name == "git"
            && self.access == AcpAccess::ReadOnly
            && !args.first().is_some_and(|value| {
                matches!(
                    value.as_str(),
                    "status" | "diff" | "log" | "show" | "rev-parse"
                )
            })
        {
            return Err(Error::Preflight(
                "ACP read-only capability cannot mutate Git state".into(),
            ));
        }
        if command_name == "rm" && args.iter().any(|value| value.contains('r')) {
            for target in args.iter().filter(|value| !value.starts_with('-')) {
                if target.contains('$') || target.contains('*') || target.contains('?') {
                    return Err(Error::Preflight(
                        "ACP destructive target must be explicit".into(),
                    ));
                }
                let target = Path::new(target);
                let target = if target.is_absolute() {
                    target.to_path_buf()
                } else {
                    cwd.join(target)
                };
                let normalized = target
                    .canonicalize()
                    .unwrap_or_else(|_| normalize_without_io(&target));
                let home = std::env::var_os("HOME").map(PathBuf::from);
                if normalized == Path::new("/")
                    || normalized == self.canonical_root
                    || self.canonical_root.starts_with(&normalized)
                    || home.as_deref() == Some(normalized.as_path())
                {
                    return Err(Error::Preflight(
                        "ACP destructive target is protected".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

fn normalize_without_io(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}
