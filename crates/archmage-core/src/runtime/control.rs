use crate::error::{Error, Result};
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
};

pub struct RunOwnerLock {
    _file: File,
}

impl RunOwnerLock {
    pub fn acquire(root: &Path, run: &str) -> Result<Self> {
        if run.is_empty() || !run.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err(Error::Preflight("invalid run ID for owner lock".into()));
        }
        fs::create_dir_all(root)?;
        let path = root.join(format!("{run}.lock"));
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::Preflight(
                "owner lock path cannot be a symlink".into(),
            ));
        }
        let mut options = OpenOptions::new();
        options.create(true).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(path)?;
        FileExt::try_lock_exclusive(&file).map_err(|_| Error::Conflict)?;
        Ok(Self { _file: file })
    }
}
