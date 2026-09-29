CREATE TABLE capability_configurations(
 key TEXT PRIMARY KEY CHECK(key IN ('designer','coder','tester','reviewer')),
 display_name TEXT NOT NULL CHECK(length(trim(display_name))>0),
 description TEXT NOT NULL DEFAULT '',
 extra_instructions TEXT NOT NULL DEFAULT '',
 revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0),
 updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

INSERT INTO capability_configurations(key,display_name,description) VALUES
 ('designer','Designer','分析目标，设计实现方案与验收标准'),
 ('coder','Coder','根据设计完成代码实现'),
 ('tester','Tester','自主选择测试方式并验证结果'),
 ('reviewer','Reviewer','审查实现与测试结果');

INSERT INTO schema_migrations(version) VALUES(9);
