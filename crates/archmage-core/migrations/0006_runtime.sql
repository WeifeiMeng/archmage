ALTER TABLE runs ADD COLUMN outcome TEXT CHECK(outcome IS NULL OR outcome IN ('approved','changes_requested'));
ALTER TABLE attempts ADD COLUMN thread_id TEXT;
ALTER TABLE attempts ADD COLUMN result_artifact_id TEXT REFERENCES artifacts(id);
ALTER TABLE attempts ADD COLUMN changeset_artifact_id TEXT REFERENCES artifacts(id);
INSERT INTO schema_migrations(version) VALUES(6);
