ALTER TABLE runs ADD COLUMN owner_generation INTEGER NOT NULL DEFAULT 0 CHECK(owner_generation>=0);
ALTER TABLE attempts ADD COLUMN owner_generation INTEGER CHECK(owner_generation IS NULL OR owner_generation>0);
ALTER TABLE attempts ADD COLUMN process_pid INTEGER CHECK(process_pid IS NULL OR process_pid>0);
ALTER TABLE attempts ADD COLUMN process_pgid INTEGER CHECK(process_pgid IS NULL OR process_pgid>0);
ALTER TABLE attempts ADD COLUMN process_started TEXT;
INSERT INTO schema_migrations(version) VALUES(3);
