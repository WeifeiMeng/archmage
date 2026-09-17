mod changeset;
mod git;
mod snapshot;

pub use changeset::{PublishedChangeSet, UntrackedArtifact, publish_changeset};
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

pub fn reconcile(store: &mut Store, run: &str) -> Result<ProvisionedWorkspace> {
    let intent = store.workspace_for_run(run)?;
    if intent.state == "ready" {
        return Ok(ProvisionedWorkspace {
            path: PathBuf::from(intent.path),
            branch: intent.branch,
            baseline: intent.baseline,
            state: ProvisionState::Ready,
        });
    }
    if intent.state != "provisioning" {
        return Err(Error::Conflict);
    }
    let source = PathBuf::from(&intent.source_root);
    let listed = git::output(
        &source,
        &[
            OsStr::new("worktree"),
            OsStr::new("list"),
            OsStr::new("--porcelain"),
        ],
    )?;
    let listed = std::str::from_utf8(&listed)
        .map_err(|_| Error::Workspace("git worktree list returned non-UTF-8 metadata".into()))?;
    let expected_path = fs::canonicalize(&intent.path)
        .map_err(|_| Error::Workspace("provisioned worktree path is missing".into()))?;
    let expected_branch = format!("refs/heads/{}", intent.branch);
    let mut exact = false;
    for record in listed.split("\n\n") {
        let mut path = None;
        let mut head = None;
        let mut branch = None;
        for line in record.lines() {
            if let Some(value) = line.strip_prefix("worktree ") {
                path = Some(value);
            }
            if let Some(value) = line.strip_prefix("HEAD ") {
                head = Some(value);
            }
            if let Some(value) = line.strip_prefix("branch ") {
                branch = Some(value);
            }
        }
        if path.is_some_and(|p| fs::canonicalize(p).ok().as_ref() == Some(&expected_path)) {
            exact =
                head == Some(intent.baseline.as_str()) && branch == Some(expected_branch.as_str());
            break;
        }
    }
    if !exact {
        return Err(Error::Workspace(
            "provisioning intent does not match registered git worktree".into(),
        ));
    }
    let inspected = inspect_repository(&expected_path)?;
    if inspected.common_dir.to_string_lossy() != intent.common_dir {
        return Err(Error::Workspace("git common directory mismatch".into()));
    }
    store.complete_workspace(run)?;
    Ok(ProvisionedWorkspace {
        path: expected_path,
        branch: intent.branch,
        baseline: intent.baseline,
        state: ProvisionState::Ready,
    })
}
