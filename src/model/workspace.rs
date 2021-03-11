use sqlx::sqlite::SqlitePool;
use sqlx::Done;
use std::fmt;

pub struct WorkspaceRecord {
    pub id: i64,
    pub url: String,
    pub description: Option<String>,
}

impl std::fmt::Display for WorkspaceRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} - {} - {}",
            self.id,
            &self.url,
            match &self.description {
                Some(v) => v,
                None => "<empty>",
            },
        )
    }
}

pub async fn add_workspace(pool: &SqlitePool, url: String, description: String, long_description: String) -> anyhow::Result<i64> {
    let mut conn = pool.acquire().await?;

    let id = sqlx::query!(
        r#"
INSERT INTO workspace ( url, description, long_description, created )
VALUES ( ?1, ?2, ?3, strftime('%Y-%m-%d %H:%M:%f','now') )
        "#,
        url,
        description,
        long_description,
    )
    .execute(&mut conn)
    .await?
    .last_insert_rowid();

    Ok(id)
}

pub async fn update_workspace(pool: &SqlitePool, id: i64, description: String, long_description: String) -> anyhow::Result<bool> {
    let rows_affected = sqlx::query!(
        r#"
UPDATE workspace
SET description = ?1, long_description = ?2
WHERE id = ?3
        "#,
        description,
        long_description,
        id,
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected > 0)
}

pub async fn list_workspaces(pool: &SqlitePool) -> anyhow::Result<Vec<WorkspaceRecord>> {
    let recs = sqlx::query_as!(WorkspaceRecord,
        r#"
SELECT id, url, description
FROM workspace
ORDER BY id
        "#
    )
    .fetch_all(pool)
    .await?;
    Ok(recs)
}

pub async fn get_workspaces_by_id(pool: &SqlitePool, id: i64) -> anyhow::Result<WorkspaceRecord> {
    // ignoring superceded_by_id for now?
    let rec = sqlx::query_as!(WorkspaceRecord,
        r#"
SELECT id, url, description
FROM workspace
WHERE id = ?1
        "#,
        id,
    )
    .fetch_one(pool)
    .await?;
    // TODO custom match statement for Err/Ok for custom message
    Ok(rec)
}
