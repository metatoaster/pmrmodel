use sqlx::sqlite::SqlitePool;
use sqlx::Done;
use std::env;
use structopt::StructOpt;

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
}

#[async_std::main]
#[paw::main]
async fn main(args: Args) -> anyhow::Result<()> {
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;

    match args.cmd {
        Some(Command::Register { url, description, long_description }) => {
            println!("Registering workspace with url '{}'", &url);
            let workspace_id = add_workspace(&pool, url, description, long_description).await?;
            println!("Registered workspace with id {}", workspace_id);
        }
        Some(Command::Update { id, description, long_description }) => {
            println!("Updating workspace with id {}", id);
            let workspace_id = update_workspace(&pool, id, description, long_description).await?;
            println!("Updateed workspace with id {}", id);
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
        id
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
