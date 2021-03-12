use anyhow::bail;
use sqlx::sqlite::SqlitePool;
use sqlx::Done;

use crate::utils::timestamp;

pub enum WorkspaceSyncStatus {
    Completed,
    Running,
    Error,
}

pub struct WorkspaceSyncRecord {
    pub id: i64,
    pub workspace_id: i64,
    pub start: i64,
    pub end: Option<i64>,
    pub status: i64,
}

pub async fn begin_sync(pool: &SqlitePool, workspace_id: i64) -> anyhow::Result<i64> {
    let ts = timestamp()? as i64;

    let id = sqlx::query!(
        r#"
INSERT INTO workspace_sync ( workspace_id, start, status )
VALUES ( ?1, ?2, ?3 )
        "#,
        workspace_id,
        ts,
        WorkspaceSyncStatus::Running as i32,
    )
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(id)
}

pub async fn complete_sync(pool: &SqlitePool, id: i64, status: WorkspaceSyncStatus) -> anyhow::Result<bool> {
    let ts = timestamp()? as i64;
    let status_ = status as i32;

    let rows_affected = sqlx::query!(
        r#"
UPDATE workspace_sync
SET end = ?1, status = ?2
WHERE id = ?3
        "#,
        ts,
        status_,
        id,
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected > 0)
}

pub async fn fail_sync(pool: &SqlitePool, id: i64, msg: String) -> anyhow::Result<()> {
    complete_sync(&pool, id, WorkspaceSyncStatus::Error).await?;
    bail!(msg);
}

pub async fn get_workspaces_sync_records(pool: &SqlitePool, workspace_id: i64) -> anyhow::Result<Vec<WorkspaceSyncRecord>> {
    let recs = sqlx::query_as!(WorkspaceSyncRecord,
        r#"
SELECT id, workspace_id, start, end, status
FROM workspace_sync
WHERE workspace_id = ?1
        "#,
        workspace_id,
    )
    .fetch_all(pool)
    .await?;
    // TODO custom match statement for Err/Ok for custom message
    Ok(recs)
}

