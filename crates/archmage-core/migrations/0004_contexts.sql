CREATE TABLE contexts(
 id TEXT PRIMARY KEY,
 run_id TEXT NOT NULL REFERENCES runs(id),
 consumer_node_id TEXT NOT NULL REFERENCES node_runs(id),
 content_hash TEXT NOT NULL CHECK(length(content_hash)=64),
 prompt_version TEXT NOT NULL,
 rendered_bytes BLOB NOT NULL,
 manifest_json TEXT NOT NULL CHECK(json_valid(manifest_json)),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE handoffs(
 id TEXT PRIMARY KEY,
 context_id TEXT NOT NULL REFERENCES contexts(id),
 producer_attempt_id TEXT NOT NULL REFERENCES attempts(id),
 artifact_id TEXT NOT NULL REFERENCES artifacts(id),
 consumer_node_id TEXT NOT NULL REFERENCES node_runs(id),
 input_key TEXT NOT NULL,
 content_hash TEXT NOT NULL CHECK(length(content_hash)=64),
 UNIQUE(context_id,input_key)
);
ALTER TABLE attempts ADD COLUMN context_id TEXT REFERENCES contexts(id);
INSERT INTO schema_migrations(version) VALUES(4);
