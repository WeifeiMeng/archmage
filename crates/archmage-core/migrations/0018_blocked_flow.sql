CREATE TABLE runs_v18(
 id TEXT PRIMARY KEY, goal TEXT NOT NULL CHECK(length(trim(goal))>0),
 state TEXT NOT NULL CHECK(state IN ('created','provisioning','running','interrupted','completed','failed','cancelled','blocked')),
 revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0),
 definition_json TEXT NOT NULL CHECK(json_valid(definition_json)),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 owner_generation INTEGER NOT NULL DEFAULT 0 CHECK(owner_generation>=0),
 outcome TEXT CHECK(outcome IS NULL OR outcome IN ('approved','changes_requested') OR outcome GLOB '*_blocked')
);
INSERT INTO runs_v18 SELECT id,goal,state,revision,definition_json,created_at,owner_generation,outcome FROM runs;
DROP TABLE runs;
ALTER TABLE runs_v18 RENAME TO runs;

CREATE TABLE node_runs_v18(
 id TEXT PRIMARY KEY, run_id TEXT NOT NULL REFERENCES runs(id),
 node_key TEXT NOT NULL, ordinal INTEGER NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('pending','ready','running','succeeded','failed','interrupted','cancelled','blocked')),
 revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0),
 UNIQUE(run_id,node_key), UNIQUE(run_id,ordinal)
);
INSERT INTO node_runs_v18 SELECT id,run_id,node_key,ordinal,state,revision FROM node_runs;
DROP TABLE node_runs;
ALTER TABLE node_runs_v18 RENAME TO node_runs;

CREATE TABLE flow_deliveries_v18(
 id TEXT PRIMARY KEY,
 run_id TEXT NOT NULL REFERENCES runs(id),
 sequence INTEGER NOT NULL CHECK(sequence>0),
 source_attempt_id TEXT REFERENCES attempts(id),
 target_key TEXT NOT NULL,
 payload_artifact_id TEXT REFERENCES artifacts(id),
 state TEXT NOT NULL CHECK(state IN (
  'queued','running','completed','blocked','skipped_due_terminal',
  'failed','cancelled','interrupted',
  'skipped_due_failure','skipped_due_cancel','skipped_due_interruption'
 )),
 attempt_id TEXT REFERENCES attempts(id),
 routing_json TEXT CHECK(routing_json IS NULL OR json_valid(routing_json)),
 UNIQUE(run_id,sequence)
);
INSERT INTO flow_deliveries_v18 SELECT id,run_id,sequence,source_attempt_id,target_key,payload_artifact_id,state,attempt_id,routing_json FROM flow_deliveries;
DROP TABLE flow_deliveries;
ALTER TABLE flow_deliveries_v18 RENAME TO flow_deliveries;
CREATE INDEX flow_deliveries_queue ON flow_deliveries(run_id,state,sequence);
INSERT INTO schema_migrations(version) VALUES(18);
