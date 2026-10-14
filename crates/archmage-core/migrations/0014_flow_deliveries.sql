CREATE TABLE flow_deliveries(
 id TEXT PRIMARY KEY,
 run_id TEXT NOT NULL REFERENCES runs(id),
 sequence INTEGER NOT NULL CHECK(sequence>0),
 source_attempt_id TEXT REFERENCES attempts(id),
 target_key TEXT NOT NULL,
 payload_artifact_id TEXT REFERENCES artifacts(id),
 state TEXT NOT NULL CHECK(state IN ('queued','running','completed','skipped_due_terminal')),
 attempt_id TEXT REFERENCES attempts(id),
 routing_json TEXT CHECK(routing_json IS NULL OR json_valid(routing_json)),
 UNIQUE(run_id,sequence)
);
CREATE INDEX flow_deliveries_queue ON flow_deliveries(run_id,state,sequence);
INSERT INTO schema_migrations(version) VALUES(14);
