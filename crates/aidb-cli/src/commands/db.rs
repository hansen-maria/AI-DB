//! `aidb db info` – annotation database status.

use aidb_client::{Client, Result};

use crate::cli::OutputFormat;
use crate::output::{print_json, print_rows, thousands};

pub async fn info(client: &Client, output: OutputFormat) -> Result<()> {
    let info = client.db_info().await?;

    if output == OutputFormat::Json {
        print_json(&info);
        return Ok(());
    }

    let entries = |n: Option<i64>| n.map(thousands).unwrap_or_else(|| "–".to_string());

    println!("Bakta DB");
    if info.available {
        print_rows(&[
            ("  Status", "available".to_string()),
            (
                "  Release",
                info.release
                    .clone()
                    .or(info.version.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
            ),
            ("  Entries", entries(info.ups_entries)),
        ]);
    } else {
        print_rows(&[
            ("  Status", "not available".to_string()),
            (
                "  Reason",
                info.error.clone().unwrap_or_else(|| "unknown".to_string()),
            ),
        ]);
    }

    println!();
    println!("BakRep DB");
    match &info.bakrep_db {
        Some(b) if b.available => print_rows(&[
            ("  Status", "available".to_string()),
            (
                "  Release",
                b.release.clone().unwrap_or_else(|| "unknown".to_string()),
            ),
            ("  Entries", entries(b.ups_entries)),
        ]),
        Some(_) => print_rows(&[("  Status", "not installed".to_string())]),
        None => print_rows(&[(
            "  Status",
            "unknown (server predates BakRep support)".to_string(),
        )]),
    }

    println!();
    println!("AI-DB annotations DB");
    if info.aidb_db.available {
        print_rows(&[
            ("  Status", "available".to_string()),
            ("  Entries", entries(info.aidb_db.ups_entries)),
            (
                "  Updated",
                info.aidb_db
                    .last_updated
                    .clone()
                    .unwrap_or_else(|| "never".to_string()),
            ),
        ]);
    } else {
        print_rows(&[("  Status", "not available".to_string())]);
    }
    Ok(())
}
