use git2::Repository;
use sqlx::sqlite::SqlitePool;
use sqlx::Done;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use structopt::StructOpt;

#[derive(Debug)]
struct WorkspaceRecord {
    id: i64,
    url: String,
    description: Option<String>,
}

#[derive(StructOpt)]
struct Args {
    #[structopt(subcommand)]
    cmd: Option<Command>,
}

#[derive(StructOpt)]
enum Command {
    Register {
        url: String,
        description: String,
        #[structopt(short = "l", long = "longdesc", default_value = "")]
        long_description: String,
    },
    Update {
        id: i64,
        description: String,
        #[structopt(short = "l", long = "longdesc", default_value = "")]
        long_description: String,
    },
    Sync {
        id: i64,
    },
}

fn fetch_envvar(key: &str) -> anyhow::Result<String> {
    match env::var(&key) {
        Err(e) => {
            writeln!(&mut io::stderr(), "couldn't interpret {}: {}", key, e)?;
            process::exit(1);
        },
        Ok(val) => Ok(val),
    }
}

#[async_std::main]
#[paw::main]
async fn main(args: Args) -> anyhow::Result<()> {
    // TODO make this be sourced from a configuration file of sort...
    // extend lifetime to scope
    let temp_git_root = fetch_envvar("PMR_GIT_ROOT")?;
    let git_root = Path::new(temp_git_root.as_str());

    let pool = SqlitePool::connect(&fetch_envvar("DATABASE_URL")?).await?;

    match args.cmd {
        Some(Command::Register { url, description, long_description }) => {
            println!("Registering workspace with url '{}'", &url);
            let workspace_id = add_workspace(&pool, url, description, long_description).await?;
            println!("Registered workspace with id {}", workspace_id);
        }
        Some(Command::Update { id, description, long_description }) => {
            println!("Updating workspace with id {}", id);
            if update_workspace(&pool, id, description, long_description).await? {
                println!("Updated workspace id {}", id);
            }
            else {
                println!("Invalid workspace id {}", id);
            }
        }
        Some(Command::Sync { id }) => {
            println!("Syncing commits for workspace with id {}", id);
            let workspace = get_workspaces_by_id(&pool, id).await?;
            git_sync_workspace(&git_root, &workspace).await?;
        }
        None => {
            println!("Printing list of all workspaces");
            list_workspaces(&pool).await?;
        }
    }

    Ok(())
}

async fn add_workspace(pool: &SqlitePool, url: String, description: String, long_description: String) -> anyhow::Result<i64> {
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

async fn update_workspace(pool: &SqlitePool, id: i64, description: String, long_description: String) -> anyhow::Result<bool> {
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

async fn list_workspaces(pool: &SqlitePool) -> anyhow::Result<()> {
    let recs = sqlx::query!(
        r#"
SELECT id, url, description
FROM workspace
ORDER BY id
        "#
    )
    .fetch_all(pool)
    .await?;

    println!("id - url - description");
    for rec in recs {
        println!(
            "{} - {} - {}",
            rec.id,
            &rec.url,
            match &rec.description {
                Some(v) => v,
                None => "<empty>",
            },
        );
    }

    Ok(())
}

async fn get_workspaces_by_id(pool: &SqlitePool, id: i64) -> anyhow::Result<WorkspaceRecord> {
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

// TODO replace git_root with the struct, or refactor this into a class?
async fn git_sync_workspace(git_root: &Path, workspace: &WorkspaceRecord) -> anyhow::Result<()> {
    let repo_dir = git_root.join(workspace.id.to_string());
    println!("Syncing local {:?} with remote <{}>...", repo_dir, &workspace.url);

    match Repository::open_bare(&repo_dir) {
        Ok(repo) => {
            println!("Found existing repo at {:?}, synchronizing...", repo_dir);
            let mut remote = repo.find_remote("origin")?;
            match remote.fetch(&[] as &[&str], None, None) {
                Ok(_) => println!("Repository synchronized"),
                Err(e) => panic!("Failed to synchronize: {}", e),
            };
        },
        Err(e) => {
            if e.class() == git2::ErrorClass::Repository {
                panic!("Invalid data at local {:?} - expected bare repo", repo_dir);
            }
            println!("Cloning new repository at {:?}...", repo_dir);
            let mut builder = git2::build::RepoBuilder::new();
            builder.bare(true);
            match builder.clone(&workspace.url, &repo_dir) {
                Ok(_) => println!("Repository cloned"),
                Err(e) => panic!("Failed to clone: {}", e),
            };
        }
    }

    Ok(())
}
