CREATE TABLE workspaces(
 id TEXT PRIMARY KEY,
 run_id TEXT NOT NULL UNIQUE REFERENCES runs(id),
 source_root TEXT NOT NULL,
 path TEXT NOT NULL UNIQUE,
 branch TEXT NOT NULL,
 baseline TEXT NOT NULL CHECK(length(baseline)>0),
 common_dir TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('provisioning','ready')),
 revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
INSERT INTO schema_migrations(version) VALUES(2);
