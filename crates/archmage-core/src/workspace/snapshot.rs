use super::{GitRepository, git};
use crate::error::{Error, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{ffi::OsStr, fs};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SnapshotEntry {
    pub path: String,
    pub kind: String,
    pub content_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorkspaceSnapshot {
    pub id: String,
    pub baseline: String,
    pub head: String,
    pub index_diff_hash: String,
    pub worktree_diff_hash: String,
    pub entries: Vec<SnapshotEntry>,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn snapshot(repo: &GitRepository) -> Result<WorkspaceSnapshot> {
    let head = git::text(&repo.root, &[OsStr::new("rev-parse"), OsStr::new("HEAD")])?;
    let index = git::output(
        &repo.root,
        &[
            OsStr::new("diff"),
            OsStr::new("--cached"),
            OsStr::new("--binary"),
            OsStr::new(&repo.baseline),
        ],
    )?;
    let working = git::output(
        &repo.root,
        &[
            OsStr::new("diff"),
            OsStr::new("--binary"),
            OsStr::new(&repo.baseline),
        ],
    )?;
    let paths = git::output(
        &repo.root,
        &[
            OsStr::new("ls-files"),
            OsStr::new("-z"),
            OsStr::new("--cached"),
            OsStr::new("--others"),
            OsStr::new("--exclude-standard"),
        ],
    )?;
    let mut entries = Vec::new();
    for raw in paths.split(|b| *b == 0).filter(|p| !p.is_empty()) {
        let path = std::str::from_utf8(raw).map_err(|_| {
            Error::Workspace("non-UTF-8 repository paths are unsupported in S0".into())
        })?;
        let absolute = repo.root.join(path);
        match fs::symlink_metadata(&absolute) {
            Ok(meta) if meta.file_type().is_symlink() => {
                let target = fs::read_link(&absolute)?;
                entries.push(SnapshotEntry {
                    path: path.into(),
                    kind: "symlink".into(),
                    content_hash: hash(target.as_os_str().as_encoded_bytes()),
                });
            }
            Ok(meta) if meta.is_file() => entries.push(SnapshotEntry {
                path: path.into(),
                kind: "file".into(),
                content_hash: hash(&fs::read(&absolute)?),
            }),
            Ok(_) => entries.push(SnapshotEntry {
                path: path.into(),
                kind: "other".into(),
                content_hash: String::new(),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => entries.push(SnapshotEntry {
                path: path.into(),
                kind: "missing".into(),
                content_hash: String::new(),
            }),
            Err(e) => return Err(e.into()),
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let mut result = WorkspaceSnapshot {
        id: String::new(),
        baseline: repo.baseline.clone(),
        head,
        index_diff_hash: hash(&index),
        worktree_diff_hash: hash(&working),
        entries,
    };
    result.id = hash(&serde_json::to_vec(&result).map_err(|e| Error::Workspace(e.to_string()))?);
    Ok(result)
}
