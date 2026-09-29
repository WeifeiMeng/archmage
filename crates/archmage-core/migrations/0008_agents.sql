CREATE TABLE agent_instances(
 id TEXT PRIMARY KEY,
 provider_kind TEXT NOT NULL CHECK(provider_kind IN ('codex','claude','openclaw','hermes')),
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

CREATE TABLE capability_bindings(
 capability TEXT PRIMARY KEY CHECK(capability IN ('designer','coder','tester','reviewer')),
 agent_instance_id TEXT NOT NULL REFERENCES agent_instances(id),
 updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

INSERT INTO agent_instances(id,provider_kind,display_name)
VALUES('codex-default','codex','Codex');

INSERT INTO capability_bindings(capability,agent_instance_id) VALUES
 ('designer','codex-default'),
 ('coder','codex-default'),
 ('tester','codex-default'),
 ('reviewer','codex-default');

INSERT INTO schema_migrations(version) VALUES(8);
