CREATE TABLE IF NOT EXISTS workspace (
    id INTEGER PRIMARY KEY NOT NULL,
    url TEXT NOT NULL,  -- should be immutable
    superceded_by_id INTEGER,  -- if superceded?
    description TEXT,
    long_description TEXT,
    created DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS workspace_sync (
    id INTEGER PRIMARY KEY NOT NULL,
    workspace_id INTEGER NOT NULL,
    start INTEGER NOT NULL,
    end INTEGER,
    status INTEGER NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspace(id)
);
CREATE INDEX workspace_id_idx ON workspace_sync(workspace_id);
