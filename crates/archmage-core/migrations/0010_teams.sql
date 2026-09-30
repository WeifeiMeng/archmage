CREATE TABLE teams(
 id TEXT PRIMARY KEY,
 name TEXT NOT NULL COLLATE NOCASE UNIQUE CHECK(length(trim(name)) BETWEEN 1 AND 80),
 revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0),
 graph_json TEXT NOT NULL CHECK(json_valid(graph_json)),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
INSERT INTO teams(id,name,graph_json) VALUES(
 'default','Default Team',
 '{"nodes":[{"capability":"designer","x":100,"y":120},{"capability":"coder","x":340,"y":120},{"capability":"tester","x":580,"y":120},{"capability":"reviewer","x":820,"y":120}],"edges":[{"from":"designer","to":"coder"},{"from":"coder","to":"tester"},{"from":"tester","to":"reviewer"},{"from":"designer","to":"tester"},{"from":"designer","to":"reviewer"},{"from":"coder","to":"reviewer"}]}'
);
INSERT INTO schema_migrations(version) VALUES(10);
