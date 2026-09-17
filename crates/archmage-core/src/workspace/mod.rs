mod git;
mod snapshot;

pub use snapshot::{SnapshotEntry, WorkspaceSnapshot, snapshot};

use crate::{
    error::{Error, Result},
    store::Store,
};
use serde::Serialize;
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct GitRepository {
    pub root: PathBuf,
    pub common_dir: PathBuf,
    pub baseline: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvisionState {
    Ready,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProvisionedWorkspace {
    pub path: PathBuf,
    pub branch: String,
    pub baseline: String,
    pub state: ProvisionState,
}

pub fn inspect_repository(path: &Path) -> Result<GitRepository> {
    let requested = fs::canonicalize(path)?;
    let root_text = git::text(
        &requested,
        &[OsStr::new("rev-parse"), OsStr::new("--show-toplevel")],
    )?;
    let root = fs::canonicalize(root_text)?;
    let baseline = git::text(
        &root,
        &[
            OsStr::new("rev-parse"),
            OsStr::new("--verify"),
            OsStr::new("HEAD"),
        ],
    )
    .map_err(|_| Error::Preflight("repository must have a committed HEAD".into()))?;
    let common_text = git::text(
        &root,
        &[OsStr::new("rev-parse"), OsStr::new("--git-common-dir")],
    )?;
    let common_path = PathBuf::from(common_text);
    let common_dir = fs::canonicalize(if common_path.is_absolute() {
        common_path
    } else {
        root.join(common_path)
    })?;
    Ok(GitRepository {
        root,
        common_dir,
        baseline,
    })
}

pub fn provision(
    store: &mut Store,
    run: &str,
    source: &Path,
    worktree_root: &Path,
) -> Result<ProvisionedWorkspace> {
    let repo = inspect_repository(source)?;
    fs::create_dir_all(worktree_root)?;
    let worktree_root = fs::canonicalize(worktree_root)?;
    let path = worktree_root.join(run);
    let branch = format!("archmage/run-{run}");
    let path_text = path
        .to_str()
        .ok_or_else(|| Error::Preflight("worktree path must be UTF-8 in S0".into()))?;
    store.record_workspace_intent(
        run,
        repo.root
            .to_str()
            .ok_or_else(|| Error::Preflight("source path must be UTF-8 in S0".into()))?,
        path_text,
        &branch,
        &repo.baseline,
        repo.common_dir
            .to_str()
            .ok_or_else(|| Error::Preflight("git common dir must be UTF-8 in S0".into()))?,
    )?;
    git::run(
        &repo.root,
        &[
            OsStr::new("worktree"),
            OsStr::new("add"),
            OsStr::new("-b"),
            OsStr::new(&branch),
            path.as_os_str(),
            OsStr::new(&repo.baseline),
        ],
    )?;
    store.complete_workspace(run)?;
    Ok(ProvisionedWorkspace {
        path,
        branch,
        baseline: repo.baseline,
        state: ProvisionState::Ready,
    })
}
