//! `aidb status` – server health, version and compatibility with this CLI.

use aidb_client::compat::{check_api_version, Compatibility};
use aidb_client::types::{BakrepDbHealth, BaktaDbHealth, API_VERSION};
use aidb_client::{Client, Result};
use serde::Serialize;

use crate::cli::OutputFormat;
use crate::output::{print_json, print_rows, warn};

/// JSON shape of `aidb status -o json`.
#[derive(Serialize)]
struct StatusReport<'a> {
    server: &'a str,
    status: &'a str,
    server_version: &'a str,
    api_version: u32,
    cli_version: &'a str,
    cli_api_version: u32,
    /// "compatible" | "unknown" | "server_older" | "server_newer"
    compatibility: &'static str,
    bakta_db: &'a BaktaDbHealth,
    /// Absent for servers without BakRep support
    #[serde(skip_serializing_if = "Option::is_none")]
    bakrep_db: Option<&'a BakrepDbHealth>,
}

fn compatibility_key(c: Compatibility) -> &'static str {
    match c {
        Compatibility::Compatible => "compatible",
        Compatibility::Unknown => "unknown",
        Compatibility::ServerOlder { .. } => "server_older",
        Compatibility::ServerNewer { .. } => "server_newer",
    }
}

pub async fn run(client: &Client, output: OutputFormat) -> Result<()> {
    let health = client.health().await?;
    let compatibility = check_api_version(health.api_version);
    let cli_version = env!("CARGO_PKG_VERSION");

    match output {
        OutputFormat::Json => print_json(&StatusReport {
            server: client.server().as_str(),
            status: &health.status,
            server_version: &health.version,
            api_version: health.api_version,
            cli_version,
            cli_api_version: API_VERSION,
            compatibility: compatibility_key(compatibility),
            bakta_db: &health.bakta_db,
            bakrep_db: health.bakrep_db.as_ref(),
        }),
        OutputFormat::Human => {
            let server_version = if health.version.is_empty() {
                "unknown (server predates version reporting)".to_string()
            } else {
                format!("{} (API v{})", health.version, health.api_version)
            };
            let compat_text = match compatibility {
                Compatibility::Compatible => "compatible",
                Compatibility::Unknown => "unknown",
                Compatibility::ServerOlder { .. } => "server is older",
                Compatibility::ServerNewer { .. } => "server is newer",
            };
            print_rows(&[
                ("Server", client.server().to_string()),
                ("Status", health.status.clone()),
                ("Version", server_version),
                ("Bakta DB", health.bakta_db.status.clone()),
                (
                    "BakRep DB",
                    health
                        .bakrep_db
                        .as_ref()
                        .map(|b| b.status.clone())
                        .unwrap_or_else(|| "unknown (server predates BakRep support)".to_string()),
                ),
                (
                    "CLI",
                    format!("{cli_version} (API v{API_VERSION}) – {compat_text}"),
                ),
            ]);
        }
    }

    // Hints go to stderr in both output modes, so scripts still get clean JSON
    match compatibility {
        Compatibility::Compatible => {}
        Compatibility::Unknown => {
            warn("the server does not report its API version; some commands may not work")
        }
        Compatibility::ServerOlder {
            server,
            client: cli_api,
        } => warn(&format!(
            "the server speaks API v{server}, this CLI expects v{cli_api}; \
             some commands may not work until the server is updated"
        )),
        Compatibility::ServerNewer {
            server,
            client: cli_api,
        } => warn(&format!(
            "the server speaks API v{server}, this CLI only v{cli_api}; please update the AI-DB CLI"
        )),
    }
    if health.bakta_db.status != "connected" {
        warn(&format!(
            "the server's Bakta DB is '{}': hash lookups will find no Bakta matches",
            health.bakta_db.status
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_keys_are_stable() {
        assert_eq!(compatibility_key(Compatibility::Compatible), "compatible");
        assert_eq!(compatibility_key(Compatibility::Unknown), "unknown");
        assert_eq!(
            compatibility_key(Compatibility::ServerOlder {
                server: 1,
                client: 2
            }),
            "server_older"
        );
        assert_eq!(
            compatibility_key(Compatibility::ServerNewer {
                server: 3,
                client: 2
            }),
            "server_newer"
        );
    }
}
