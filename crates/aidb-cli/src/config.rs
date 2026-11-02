//! Location of the CLI's configuration files.
//!
//! | Platform      | Directory                                            |
//! |---------------|------------------------------------------------------|
//! | any           | `$AIDB_CONFIG_DIR`, if set                           |
//! | Linux, macOS  | `$XDG_CONFIG_HOME/aidb`, else `~/.config/aidb`       |
//! | Windows       | `%APPDATA%\aidb`                                     |

use std::env;
use std::path::PathBuf;

use crate::credentials;
use crate::error::{CliError, Result};

/// Directory holding the CLI's configuration files.
pub fn config_dir() -> Result<PathBuf> {
    if let Some(dir) = env::var_os("AIDB_CONFIG_DIR").filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    platform_config_dir().ok_or_else(|| {
        CliError::Config(
            "cannot determine your home directory; set AIDB_CONFIG_DIR to a writable directory"
                .to_string(),
        )
    })
}

/// Path of the file holding the saved access tokens.
pub fn credentials_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(credentials::FILE_NAME))
}

#[cfg(windows)]
fn platform_config_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.config_dir().join("aidb"))
}

#[cfg(not(windows))]
fn platform_config_dir() -> Option<PathBuf> {
    // XDG layout on macOS too: command-line tools are expected in ~/.config,
    // not in ~/Library/Application Support
    let xdg = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute());
    match xdg {
        Some(dir) => Some(dir.join("aidb")),
        None => {
            directories::BaseDirs::new().map(|dirs| dirs.home_dir().join(".config").join("aidb"))
        }
    }
}
