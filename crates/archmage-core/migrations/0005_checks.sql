CREATE TABLE check_runs(
 evidence_id TEXT PRIMARY KEY,
 attempt_id TEXT NOT NULL REFERENCES attempts(id),
 check_id TEXT NOT NULL,
 command TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('in_progress','completed')),
 evidence_json TEXT CHECK(evidence_json IS NULL OR json_valid(evidence_json)),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 UNIQUE(attempt_id,check_id)
);
INSERT INTO schema_migrations(version) VALUES(5);
