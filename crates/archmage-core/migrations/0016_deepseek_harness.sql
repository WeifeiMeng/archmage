CREATE TABLE agent_instances_v16(
 id TEXT PRIMARY KEY,
 provider_kind TEXT NOT NULL CHECK(provider_kind IN ('codex','deepseek_harness','claude','openclaw','hermes')),
 display_name TEXT NOT NULL CHECK(length(trim(display_name))>0),
 executable_path TEXT,
 provider_route TEXT,
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

INSERT INTO agent_instances_v16(
 id,provider_kind,display_name,executable_path,provider_route,model,enabled,
 connection_state,discovered_version,last_verified_at,last_error,
 verification_thread_id,created_at,updated_at
)
SELECT id,provider_kind,display_name,executable_path,NULL,model,enabled,
 connection_state,discovered_version,last_verified_at,last_error,
 verification_thread_id,created_at,updated_at
FROM agent_instances;

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

INSERT INTO schema_migrations(version) VALUES(16);
