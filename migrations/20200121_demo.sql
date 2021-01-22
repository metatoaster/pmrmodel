CREATE TABLE IF NOT EXISTS workspace (
    id INTEGER PRIMARY KEY NOT NULL,
    url TEXT NOT NULL,  -- should be immutable
    superceded_by_id INTEGER,  -- if superceded?
    description TEXT,
    long_description TEXT,
    created DATETIME NOT NULL
);
