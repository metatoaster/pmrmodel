CREATE TABLE IF NOT EXISTS workspace (
    id INTEGER PRIMARY KEY NOT NULL,
    url TEXT NOT NULL,  -- should be immutable
    superceded_by_id INTEGER,  -- if superceded?
    description TEXT,
    long_description TEXT,
    created INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS workspace_sync (
    id INTEGER PRIMARY KEY NOT NULL,
    workspace_id INTEGER NOT NULL,
    start INTEGER NOT NULL,
    end INTEGER,
    status INTEGER NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspace(id)
);
CREATE INDEX workspace_sync_idx_workspace_id ON workspace_sync(workspace_id);

CREATE TABLE IF NOT EXISTS workspace_tag (
    id INTEGER PRIMARY KEY NOT NULL,
    workspace_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    commit_id TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspace(id)
);
CREATE INDEX workspace_tag_idx_workspace_id ON workspace_tag(workspace_id);
CREATE UNIQUE INDEX workspace_tag_idx_workspace_id_name ON workspace_tag(workspace_id, name);
CREATE UNIQUE INDEX workspace_tag_idx_workspace_id_name_commit_id ON workspace_tag(workspace_id, name, commit_id);
