CREATE TABLE consultations(
 id TEXT PRIMARY KEY,
 run_id TEXT NOT NULL REFERENCES runs(id),
 source_node_id TEXT NOT NULL REFERENCES node_runs(id),
 source_attempt_id TEXT NOT NULL REFERENCES attempts(id),
 target_capability TEXT NOT NULL,
 round INTEGER NOT NULL CHECK(round>=0),
 decision TEXT NOT NULL CHECK(decision IN ('approved','changes_requested')),
 summary TEXT NOT NULL CHECK(length(trim(summary))>0),
 suggestions_json TEXT NOT NULL CHECK(json_valid(suggestions_json)),
 thread_id TEXT NOT NULL CHECK(length(trim(thread_id))>0),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 UNIQUE(source_attempt_id,target_capability)
);
CREATE INDEX consultations_by_run ON consultations(run_id,source_attempt_id);
CREATE TABLE revision_feedback(
 id TEXT PRIMARY KEY,
 node_id TEXT NOT NULL REFERENCES node_runs(id),
 source_attempt_id TEXT NOT NULL REFERENCES attempts(id),
 suggestions_json TEXT NOT NULL CHECK(json_valid(suggestions_json)),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
INSERT INTO schema_migrations(version) VALUES(11);
