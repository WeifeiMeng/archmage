CREATE TABLE control_requests(
 id TEXT PRIMARY KEY,
 run_id TEXT NOT NULL REFERENCES runs(id),
 kind TEXT NOT NULL CHECK(kind IN ('cancel')),
 state TEXT NOT NULL CHECK(state IN ('requested','applied')),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 applied_at TEXT
);
CREATE UNIQUE INDEX one_active_control_request ON control_requests(run_id,kind) WHERE state='requested';
INSERT INTO schema_migrations(version) VALUES(7);
