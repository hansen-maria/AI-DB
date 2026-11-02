//! Saved access tokens, one per AI-DB server.
//!
//! Stored in `credentials.toml` in the config directory, readable only by the
//! user (0600 on Unix). Keys are normalised server URLs, so `https://host`
//! and `https://host/` share one entry:
//!
//! ```toml
//! [tokens]
//! "https://ai-db.computational.bio/" = "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c"
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CliError, Result};

pub const FILE_NAME: &str = "credentials.toml";

const HEADER: &str = "# AI-DB CLI access tokens – keep this file private.\n\
                      # Each token is the only key to the jobs created with it.\n\n";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Credentials {
    #[serde(default)]
    tokens: BTreeMap<String, String>,
}

impl Credentials {
    /// Reads the file; a missing file means no saved tokens.
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|e| {
                CliError::Config(format!(
                    "{} is not a valid credentials file: {e}",
                    path.display()
                ))
            }),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(CliError::io(format!("cannot read {}", path.display()), e)),
        }
    }

    /// Saved token for a normalised server URL.
    pub fn get(&self, server: &str) -> Option<&str> {
        self.tokens.get(server).map(String::as_str)
    }

    pub fn set(&mut self, server: &str, token: String) {
        self.tokens.insert(server.to_string(), token);
    }

    /// Writes the file atomically (temp file + rename), private to the user.
    pub fn save(&self, path: &Path) -> Result<()> {
        let dir = path
            .parent()
            .filter(|d| !d.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        create_private_dir(dir)?;

        let body = toml::to_string_pretty(self)
            .map_err(|e| CliError::Config(format!("cannot encode the credentials: {e}")))?;
        let tmp = dir.join(format!(".{FILE_NAME}.tmp"));
        write_private(&tmp, format!("{HEADER}{body}").as_bytes())?;
        fs::rename(&tmp, path).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            CliError::io(format!("cannot write {}", path.display()), e)
        })
    }
}

fn create_private_dir(dir: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700); // only applies to directories created here
    }
    builder
        .create(dir)
        .map_err(|e| CliError::io(format!("cannot create {}", dir.display()), e))
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let error = |e| CliError::io(format!("cannot write {}", path.display()), e);
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(error)?;
    #[cfg(unix)]
    {
        // A leftover temp file may have been created with other permissions
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(error)?;
    }
    file.write_all(bytes).map_err(error)?;
    file.sync_all().map_err(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVER: &str = "https://ai-db.example.org/";
    const TOKEN: &str = "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c";

    #[test]
    fn missing_file_means_no_tokens() {
        let dir = tempfile::tempdir().unwrap();
        let creds = Credentials::load(&dir.path().join(FILE_NAME)).unwrap();
        assert_eq!(creds.get(SERVER), None);
    }

    #[test]
    fn tokens_round_trip_per_server_and_dirs_are_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join(FILE_NAME);

        let mut creds = Credentials::default();
        creds.set(SERVER, TOKEN.to_string());
        creds.set("http://localhost:8000/", "other".to_string());
        creds.save(&path).unwrap();

        let loaded = Credentials::load(&path).unwrap();
        assert_eq!(loaded.get(SERVER), Some(TOKEN));
        assert_eq!(loaded.get("http://localhost:8000/"), Some("other"));
        assert!(fs::read_to_string(&path)
            .unwrap()
            .starts_with("# AI-DB CLI access tokens"));
        assert!(!dir
            .path()
            .join("nested")
            .join(format!(".{FILE_NAME}.tmp"))
            .exists());
    }

    #[test]
    fn saving_replaces_the_previous_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let mut creds = Credentials::default();
        creds.set(SERVER, "old".to_string());
        creds.save(&path).unwrap();
        creds.set(SERVER, TOKEN.to_string());
        creds.save(&path).unwrap();
        assert_eq!(Credentials::load(&path).unwrap().get(SERVER), Some(TOKEN));
    }

    #[test]
    fn invalid_file_is_a_config_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        fs::write(&path, "tokens = 42").unwrap();
        assert!(matches!(Credentials::load(&path), Err(CliError::Config(_))));
    }

    #[cfg(unix)]
    #[test]
    fn file_is_private_to_the_user() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        Credentials::default().save(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
