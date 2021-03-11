use anyhow::bail;
use git2::Repository;
use sqlx::sqlite::SqlitePool;
use std::path::Path;

use crate::model::workspace::WorkspaceRecord;
use crate::model::workspace_sync::{
    WorkspaceSyncStatus,
    begin_sync,
    complete_sync,
};


// TODO replace git_root with the struct, or refactor this into a class?
pub async fn git_sync_workspace(pool: &SqlitePool, git_root: &Path, workspace: &WorkspaceRecord) -> anyhow::Result<()> {
    let repo_dir = git_root.join(workspace.id.to_string());
    println!("Syncing local {:?} with remote <{}>...", repo_dir, &workspace.url);
    let sync_id = begin_sync(&pool, workspace.id).await?;

    let repo = Repository::open_bare(&repo_dir);
    match repo {
        Ok(repo) => {
            println!("Found existing repo at {:?}, synchronizing...", repo_dir);
            let mut remote = repo.find_remote("origin")?;
            match remote.fetch(&[] as &[&str], None, None) {
                Ok(_) => println!("Repository synchronized"),
                Err(e) => {
                    complete_sync(&pool, sync_id, WorkspaceSyncStatus::Error).await?;
                    bail!("Failed to synchronize: {}", e)
                },
            };
        },
        Err(ref e) if e.class() == git2::ErrorClass::Repository => {
            complete_sync(&pool, sync_id, WorkspaceSyncStatus::Error).await?;
            bail!("Invalid data at local {:?} - expected bare repo", repo_dir)
        },
        Err(_) => {
            println!("Cloning new repository at {:?}...", repo_dir);
            let mut builder = git2::build::RepoBuilder::new();
            builder.bare(true);
            match builder.clone(&workspace.url, &repo_dir) {
                Ok(_) => println!("Repository cloned"),
                Err(e) => {
                    complete_sync(&pool, sync_id, WorkspaceSyncStatus::Error).await?;
                    bail!("Failed to clone: {}", e)
                },
            };
        }
    }

    complete_sync(&pool, sync_id, WorkspaceSyncStatus::Completed).await?;

    Ok(())
}
