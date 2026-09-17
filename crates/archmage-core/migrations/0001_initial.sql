CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')));
CREATE TABLE runs(
 id TEXT PRIMARY KEY, goal TEXT NOT NULL CHECK(length(trim(goal))>0),
 state TEXT NOT NULL CHECK(state IN ('created','provisioning','running','interrupted','completed','failed','cancelled')),
 revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0),
 definition_json TEXT NOT NULL CHECK(json_valid(definition_json)),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE node_runs(
 id TEXT PRIMARY KEY, run_id TEXT NOT NULL REFERENCES runs(id),
 node_key TEXT NOT NULL, ordinal INTEGER NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('pending','ready','running','succeeded','failed','interrupted','cancelled')),
 revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0),
 UNIQUE(run_id,node_key), UNIQUE(run_id,ordinal)
);
CREATE TABLE attempts(
 id TEXT PRIMARY KEY, node_run_id TEXT NOT NULL REFERENCES node_runs(id),
 attempt_number INTEGER NOT NULL CHECK(attempt_number>0),
 state TEXT NOT NULL CHECK(state IN ('starting','running','succeeded','failed','interrupted','cancelled')),
 revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0),
 output_json TEXT CHECK(output_json IS NULL OR json_valid(output_json)),
 error_code TEXT,
 UNIQUE(node_run_id,attempt_number)
);
CREATE UNIQUE INDEX one_active_attempt ON attempts(node_run_id) WHERE state IN ('starting','running');
CREATE TABLE events(
 run_id TEXT NOT NULL REFERENCES runs(id),
 event_seq INTEGER NOT NULL CHECK(event_seq>0),
 event_type TEXT NOT NULL,
 node_id TEXT REFERENCES node_runs(id),
 attempt_id TEXT REFERENCES attempts(id),
 schema_version INTEGER NOT NULL DEFAULT 1,
 payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 PRIMARY KEY(run_id,event_seq)
);
CREATE TABLE artifacts(
 id TEXT PRIMARY KEY, attempt_id TEXT NOT NULL REFERENCES attempts(id),
 content_hash TEXT NOT NULL CHECK(length(content_hash)=64),
 byte_length INTEGER NOT NULL CHECK(byte_length>=0),
 media_type TEXT NOT NULL
);
INSERT INTO schema_migrations(version) VALUES(1);
