ALTER TABLE capability_configurations ADD COLUMN skills_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE capability_configurations ADD COLUMN tools_json TEXT NOT NULL DEFAULT '[]';
INSERT INTO schema_migrations(version) VALUES(13);
