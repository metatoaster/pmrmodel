use anyhow::bail;
use git2::Repository;
use std::path::Path;

use crate::model::workspace::WorkspaceRecord;


// TODO replace git_root with the struct, or refactor this into a class?
pub async fn git_sync_workspace(git_root: &Path, workspace: &WorkspaceRecord) -> anyhow::Result<()> {
    let repo_dir = git_root.join(workspace.id.to_string());
    println!("Syncing local {:?} with remote <{}>...", repo_dir, &workspace.url);

    let repo = Repository::open_bare(&repo_dir);
    match repo {
        Ok(repo) => {
            println!("Found existing repo at {:?}, synchronizing...", repo_dir);
            let mut remote = repo.find_remote("origin")?;
            match remote.fetch(&[] as &[&str], None, None) {
                Ok(_) => println!("Repository synchronized"),
                Err(e) => bail!("Failed to synchronize: {}", e),
            };
        },
        Err(ref e) if e.class() == git2::ErrorClass::Repository => {
            bail!("Invalid data at local {:?} - expected bare repo", repo_dir);
        },
        Err(_) => {
            println!("Cloning new repository at {:?}...", repo_dir);
            let mut builder = git2::build::RepoBuilder::new();
            builder.bare(true);
            match builder.clone(&workspace.url, &repo_dir) {
                Ok(_) => println!("Repository cloned"),
                Err(e) => bail!("Failed to clone: {}", e),
            };
        }
    }

    Ok(())
}
