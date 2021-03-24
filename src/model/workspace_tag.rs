use sqlx::sqlite::SqlitePool;
use std::fmt;

pub struct WorkspaceTagRecord {
    pub id: i64,
    pub workspace_id: i64,
    pub name: String,
    pub commit_id: String,
}

impl std::fmt::Display for WorkspaceTagRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} - {}",
            &self.commit_id,
            &self.name,
        )
    }
}

pub async fn index_workspace_tag(pool: &SqlitePool, workspace_id: i64, name: &str, commit_id: &str) -> anyhow::Result<i64> {
    let id = sqlx::query!(
        r#"
INSERT INTO workspace_tag ( workspace_id, name, commit_id )
VALUES ( ?1, ?2, ?3 )
ON CONFLICT (workspace_id, name, commit_id) DO NOTHING
        "#,
        workspace_id,
        name,
        commit_id,
    )
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(id)
}
// TODO create test so that the unique indexes are done correctly

pub async fn get_workspace_tags(pool: &SqlitePool, workspace_id: i64) -> anyhow::Result<Vec<WorkspaceTagRecord>> {
    let recs = sqlx::query_as!(WorkspaceTagRecord,
        r#"
SELECT id, workspace_id, name, commit_id
FROM workspace_tag
WHERE workspace_id = ?1
        "#,
        workspace_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(recs)
}

