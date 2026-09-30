ALTER TABLE consultations ADD COLUMN result_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(result_json));
ALTER TABLE consultations ADD COLUMN result_sha256 TEXT NOT NULL DEFAULT '';
CREATE TABLE consultation_processes(
 id TEXT PRIMARY KEY,
 run_id TEXT NOT NULL REFERENCES runs(id),
 source_attempt_id TEXT NOT NULL REFERENCES attempts(id),
 target_capability TEXT NOT NULL,
 process_pid INTEGER NOT NULL,
 process_pgid INTEGER NOT NULL,
 process_started TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('running','finished','cancelled')),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX consultation_processes_by_run ON consultation_processes(run_id,state);
INSERT INTO schema_migrations(version) VALUES(12);
