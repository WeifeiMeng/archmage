//! Runtime-owned immutable files. Models never choose artifact IDs or paths.
use crate::{
    error::{Error, Result},
    store::Store,
};
use rusqlite::{TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub struct ArtifactStore {
    root: PathBuf,
}
const MAX_BYTES: usize = 16 * 1024 * 1024;

impl ArtifactStore {
    pub fn open(root: &Path) -> Result<Self> {
        if fs::symlink_metadata(root).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::Preflight("artifact root cannot be symlink".into()));
        }
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(root)?;
        Ok(Self {
            root: fs::canonicalize(root)?,
        })
    }

    pub fn publish(
        &self,
        store: &mut Store,
        attempt: &str,
        bytes: &[u8],
        media_type: &str,
    ) -> Result<String> {
        if bytes.len() > MAX_BYTES {
            return Err(Error::Limit("artifact too large".into()));
        }
        if store.attempt(attempt)?.state != "running" {
            return Err(Error::Conflict);
        }
        let id = uuid::Uuid::new_v4().to_string();
        let hash = format!("{:x}", Sha256::digest(bytes));
        let mut file = tempfile::NamedTempFile::new_in(&self.root)?;
        file.write_all(bytes)?;
        file.as_file().sync_all()?;
        // Never overwrite an existing canonical artifact, even on an ID collision.
        file.persist_noclobber(self.root.join(&id))
            .map_err(|e| Error::Io(e.error))?;
        fs::File::open(&self.root)?.sync_all()?;
        let tx = store
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (state,node,run):(String,String,String)=tx.query_row(
            "SELECT a.state,n.id,n.run_id FROM attempts a JOIN node_runs n ON n.id=a.node_run_id WHERE a.id=?1",
            [attempt],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        if state != "running" {
            return Err(Error::Conflict);
        }
        tx.execute("INSERT INTO artifacts(id,attempt_id,content_hash,byte_length,media_type) VALUES(?1,?2,?3,?4,?5)",
            params![id,attempt,hash,bytes.len() as i64,media_type])?;
        crate::store::transitions::event(
            &tx,
            &run,
            "artifact.created",
            Some(&node),
            Some(attempt),
            serde_json::json!({"id":id,"sha256":hash}),
        )?;
        tx.commit()?;
        Ok(id)
    }

    pub fn read_committed(&self, store: &Store, id: &str) -> Result<Vec<u8>> {
        let parsed = uuid::Uuid::parse_str(id)
            .map_err(|_| Error::Preflight("invalid artifact ID".into()))?;
        if parsed.to_string() != id {
            return Err(Error::Preflight("noncanonical artifact ID".into()));
        }
        let (hash,len):(String,i64)=store.db.query_row(
            "SELECT f.content_hash,f.byte_length FROM artifacts f JOIN attempts a ON a.id=f.attempt_id WHERE f.id=?1 AND a.state='succeeded'",
            [id],|r|Ok((r.get(0)?,r.get(1)?)))?;
        let path = self.root.join(id);
        if !fs::symlink_metadata(&path)?.file_type().is_file() {
            return Err(Error::Preflight("artifact must be a regular file".into()));
        }
        let mut bytes = Vec::new();
        fs::File::open(path)?
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_BYTES
            || bytes.len() as i64 != len
            || format!("{:x}", Sha256::digest(&bytes)) != hash
        {
            return Err(Error::Preflight("artifact content integrity failed".into()));
        }
        Ok(bytes)
    }
}
