use std::process;

use tokio_postgres::NoTls;

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./../../data-platform/db-schema/migrations");
}

async fn connect(database_url: &str) -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
    let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {e}");
        }
    });
    Ok(client)
}

async fn run_migrations(database_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = connect(database_url).await?;
    let report = embedded::migrations::runner()
        .run_async(&mut client)
        .await?;
    for migration in report.applied_migrations() {
        eprintln!(
            "  applied V{:03}__{}",
            migration.version(),
            migration.name()
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let database_url = if args.len() > 1 && !args[1].starts_with('-') {
        args[1].clone()
    } else {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            eprintln!("Usage: onelink-migration-runner [DATABASE_URL]");
            eprintln!("       Or set DATABASE_URL environment variable");
            process::exit(1);
        })
    };

    if args.iter().any(|a| a == "--check") {
        eprintln!("Checking migration files exist...");
        let mut client = connect(&database_url).await.unwrap_or_else(|e| {
            eprintln!("Failed to connect for --check: {e}");
            process::exit(1);
        });
        let report = embedded::migrations::runner().run_async(&mut client).await;
        match report {
            Ok(r) => {
                eprintln!(
                    "Migration check passed: {} migration(s) registered",
                    r.applied_migrations().len()
                );
            }
            Err(e) => {
                eprintln!("Migration check failed: {e}");
                process::exit(1);
            }
        }
        return;
    }

    eprintln!("Running migrations against {database_url}...");
    if let Err(e) = run_migrations(&database_url).await {
        eprintln!("Migration failed: {e}");
        process::exit(1);
    }
    eprintln!("Migrations complete.");
}
