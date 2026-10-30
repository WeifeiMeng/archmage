CREATE TABLE agent_instances_v16(
 id TEXT PRIMARY KEY,
 provider_kind TEXT NOT NULL CHECK(provider_kind IN ('codex','deepseek_harness','claude','openclaw','hermes')),
 display_name TEXT NOT NULL CHECK(length(trim(display_name))>0),
 executable_path TEXT,
 model TEXT,
 enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0,1)),
 connection_state TEXT NOT NULL DEFAULT 'unverified' CHECK(connection_state IN ('unverified','ready','failed')),
 discovered_version TEXT,
 last_verified_at TEXT,
 last_error TEXT,
 verification_thread_id TEXT,
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

INSERT INTO agent_instances_v16
SELECT * FROM agent_instances;

CREATE TABLE capability_bindings_v16 AS
SELECT capability,agent_instance_id,updated_at FROM capability_bindings;

DROP TABLE capability_bindings;
DROP TABLE agent_instances;
ALTER TABLE agent_instances_v16 RENAME TO agent_instances;

CREATE TABLE capability_bindings(
 capability TEXT PRIMARY KEY CHECK(capability IN ('designer','coder','tester','reviewer')),
 agent_instance_id TEXT NOT NULL REFERENCES agent_instances(id),
 updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

INSERT INTO capability_bindings
SELECT capability,agent_instance_id,updated_at FROM capability_bindings_v16;
DROP TABLE capability_bindings_v16;

INSERT INTO agent_instances(id,provider_kind,display_name,model)
VALUES('deepseek-harness-default','deepseek_harness','DeepSeek Harness',NULL);

CREATE TABLE provider_sessions(
 run_id TEXT NOT NULL REFERENCES runs(id),
 capability TEXT NOT NULL CHECK(capability IN ('designer','coder','tester','reviewer')),
 provider_kind TEXT NOT NULL CHECK(provider_kind='deepseek_harness'),
 session_id TEXT NOT NULL UNIQUE,
 state TEXT NOT NULL CHECK(state IN ('reserved','ready','uncertain')),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 PRIMARY KEY(run_id,capability)
);

INSERT INTO schema_migrations(version) VALUES(16);
