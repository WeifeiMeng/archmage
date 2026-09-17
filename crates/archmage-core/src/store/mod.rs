//! Transactional execution ledger. Scheduling, contract validation and OS ownership live above it.
mod recovery;
pub(crate) mod transitions;
use crate::{
    error::{Error, Result},
    event::Event,
};
use rusqlite::{Connection, TransactionBehavior};
use serde::Serialize;
use serde_json::Value;
use std::{path::Path, time::Duration};

pub struct Store {
    pub(crate) db: Connection,
}

#[derive(Debug, Serialize)]
pub struct StoreHealth {
    pub schema_version: i64,
    pub foreign_keys: bool,
    pub synchronous: i64,
    pub journal_mode: String,
    pub busy_timeout_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Run {
    pub id: String,
    pub state: String,
    pub revision: i64,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Node {
    pub id: String,
    pub run_id: String,
    pub key: String,
    pub state: String,
    pub revision: i64,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Attempt {
    pub id: String,
    pub node_id: String,
    pub number: i64,
    pub state: String,
    pub revision: i64,
    pub output: Option<Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkspaceRecord {
    pub id: String,
    pub run_id: String,
    pub source_root: String,
    pub path: String,
    pub branch: String,
    pub baseline: String,
    pub common_dir: String,
    pub state: String,
    pub revision: i64,
}
pub enum AttemptOutcome {
    /// Caller must have validated the contract. This is a ledger write, not a validation bypass API.
    Succeeded(Value),
    Failed(String),
    Interrupted,
    Cancelled,
}

impl Store {
    pub fn health(&self) -> Result<StoreHealth> {
        Ok(StoreHealth {
            schema_version: self.schema_version()?,
            foreign_keys: self.db.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?,
            synchronous: self.db.query_row("PRAGMA synchronous", [], |r| r.get(0))?,
            journal_mode: self.db.query_row("PRAGMA journal_mode", [], |r| r.get(0))?,
            busy_timeout_ms: self.db.query_row("PRAGMA busy_timeout", [], |r| r.get(0))?,
        })
    }
    pub fn open(path: &Path) -> Result<Self> {
        // Database sidecars inherit permissions from the database; reject a symlink target.
        if std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::Preflight("database path cannot be a symlink".into()));
        }
        let mut opts = std::fs::OpenOptions::new();
        opts.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let _file = opts.open(path)?;
        let mut db = Connection::open(path)?;
        db.busy_timeout(Duration::from_millis(5000))?;
        db.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let has_schema: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_migrations')",
            [], |r| r.get(0))?;
        if !has_schema {
            tx.execute_batch(include_str!("../../migrations/0001_initial.sql"))?;
            tx.execute_batch(include_str!("../../migrations/0002_workspaces.sql"))?;
            tx.execute_batch(include_str!("../../migrations/0003_ownership.sql"))?;
        } else {
            let mut version: i64 = tx.query_row(
                "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
                [],
                |r| r.get(0),
            )?;
            if version == 1 {
                tx.execute_batch(include_str!("../../migrations/0002_workspaces.sql"))?;
                version = 2;
            }
            if version == 2 {
                tx.execute_batch(include_str!("../../migrations/0003_ownership.sql"))?;
                version = 3;
            }
            if version != 3 {
                return Err(Error::Preflight(format!(
                    "unsupported database schema {version}"
                )));
            }
        }
        tx.commit()?;
        Ok(Self { db })
    }
    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .db
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })?)
    }
    pub fn run(&self, id: &str) -> Result<Run> {
        Ok(self.db.query_row(
            "SELECT id,state,revision FROM runs WHERE id=?1",
            [id],
            |r| {
                Ok(Run {
                    id: r.get(0)?,
                    state: r.get(1)?,
                    revision: r.get(2)?,
                })
            },
        )?)
    }
    pub fn node(&self, id: &str) -> Result<Node> {
        Ok(self.db.query_row(
            "SELECT id,run_id,node_key,state,revision FROM node_runs WHERE id=?1",
            [id],
            node_row,
        )?)
    }
    pub fn nodes(&self, run: &str) -> Result<Vec<Node>> {
        let mut q=self.db.prepare("SELECT id,run_id,node_key,state,revision FROM node_runs WHERE run_id=?1 ORDER BY ordinal")?;
        Ok(q.query_map([run], node_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn attempt(&self, id: &str) -> Result<Attempt> {
        Ok(self.db.query_row("SELECT id,node_run_id,attempt_number,state,revision,output_json FROM attempts WHERE id=?1", [id], attempt_row)?)
    }
    pub fn workspace_for_run(&self, run: &str) -> Result<WorkspaceRecord> {
        Ok(self.db.query_row(
            "SELECT id,run_id,source_root,path,branch,baseline,common_dir,state,revision FROM workspaces WHERE run_id=?1",
            [run],
            workspace_row,
        )?)
    }
    pub fn events(&self, run: &str, after: i64) -> Result<Vec<Event>> {
        let mut q=self.db.prepare("SELECT run_id,event_seq,event_type,node_id,attempt_id,schema_version,payload_json,created_at FROM events WHERE run_id=?1 AND event_seq>?2 ORDER BY event_seq")?;
        Ok(q.query_map(rusqlite::params![run, after], |r| {
            Ok(Event {
                run_id: r.get(0)?,
                sequence: r.get(1)?,
                event_type: r.get(2)?,
                node_id: r.get(3)?,
                attempt_id: r.get(4)?,
                schema_version: r.get(5)?,
                payload: parse_json(r.get::<_, String>(6)?, 6)?,
                created_at: r.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
    }
}
fn workspace_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceRecord> {
    Ok(WorkspaceRecord {
        id: r.get(0)?,
        run_id: r.get(1)?,
        source_root: r.get(2)?,
        path: r.get(3)?,
        branch: r.get(4)?,
        baseline: r.get(5)?,
        common_dir: r.get(6)?,
        state: r.get(7)?,
        revision: r.get(8)?,
    })
}
fn parse_json(text: String, column: usize) -> rusqlite::Result<Value> {
    serde_json::from_str(&text).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(column, rusqlite::types::Type::Text, Box::new(e))
    })
}
fn node_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Node> {
    Ok(Node {
        id: r.get(0)?,
        run_id: r.get(1)?,
        key: r.get(2)?,
        state: r.get(3)?,
        revision: r.get(4)?,
    })
}
fn attempt_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Attempt> {
    let output: Option<String> = r.get(5)?;
    Ok(Attempt {
        id: r.get(0)?,
        node_id: r.get(1)?,
        number: r.get(2)?,
        state: r.get(3)?,
        revision: r.get(4)?,
        output: output.map(|s| parse_json(s, 5)).transpose()?,
    })
}
