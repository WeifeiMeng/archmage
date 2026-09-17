use super::{GitRepository, git, snapshot};
use crate::{
    artifact::ArtifactStore,
    error::{Error, Result},
    store::Store,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{ffi::OsStr, fs};

#[derive(Clone, Debug, Serialize)]
pub struct UntrackedArtifact {
    pub path: String,
    pub kind: String,
    pub sha256: String,
    pub artifact_id: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublishedChangeSet {
    pub snapshot_id: String,
    pub tracked_patch_artifact: String,
    pub untracked: Vec<UntrackedArtifact>,
    pub manifest_artifact: String,
}

pub fn publish_changeset(
    artifacts: &ArtifactStore,
    store: &mut Store,
    attempt: &str,
    repo: &GitRepository,
) -> Result<PublishedChangeSet> {
    let snapshot_id = snapshot(repo)?.id;
    let patch = git::output(
        &repo.root,
        &[
            OsStr::new("diff"),
            OsStr::new("--binary"),
            OsStr::new(&repo.baseline),
        ],
    )?;
    let tracked_patch_artifact =
        artifacts.publish(store, attempt, &patch, "application/vnd.git.diff")?;
    let raw_paths = git::output(
        &repo.root,
        &[
            OsStr::new("ls-files"),
            OsStr::new("-z"),
            OsStr::new("--others"),
            OsStr::new("--exclude-standard"),
        ],
    )?;
    let mut untracked = Vec::new();
    for raw in raw_paths.split(|b| *b == 0).filter(|p| !p.is_empty()) {
        let path = std::str::from_utf8(raw).map_err(|_| {
            Error::Workspace("non-UTF-8 repository paths are unsupported in S0".into())
        })?;
        let absolute = repo.root.join(path);
        let meta = fs::symlink_metadata(&absolute)?;
        let (kind, bytes) = if meta.file_type().is_symlink() {
            (
                "symlink",
                fs::read_link(&absolute)?
                    .as_os_str()
                    .as_encoded_bytes()
                    .to_vec(),
            )
        } else if meta.is_file() {
            ("file", fs::read(&absolute)?)
        } else {
            return Err(Error::Workspace(format!(
                "unsupported untracked entry type: {path}"
            )));
        };
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        let artifact_id = artifacts.publish(store, attempt, &bytes, "application/octet-stream")?;
        untracked.push(UntrackedArtifact {
            path: path.into(),
            kind: kind.into(),
            sha256,
            artifact_id,
        });
    }
    untracked.sort_by(|a, b| a.path.cmp(&b.path));
    let manifest_bytes = serde_json::to_vec(&serde_json::json!({
        "schema_version": 1,
        "snapshot_id": snapshot_id,
        "tracked_patch_artifact": tracked_patch_artifact,
        "untracked": untracked,
    }))
    .map_err(|e| Error::Workspace(e.to_string()))?;
    let manifest_artifact =
        artifacts.publish(store, attempt, &manifest_bytes, "application/json")?;
    Ok(PublishedChangeSet {
        snapshot_id,
        tracked_patch_artifact,
        untracked,
        manifest_artifact,
    })
}
