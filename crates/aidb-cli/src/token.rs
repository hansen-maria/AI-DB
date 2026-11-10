//! Access token handling: validation, parsing pasted input, masking and
//! finding the token to use.
//!
//! Precedence: `AIDB_TOKEN` environment variable, then the token saved for
//! the server in `credentials.toml`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::credentials::Credentials;
use crate::error::{CliError, Result};

/// Where the token in use comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenSource {
    /// `AIDB_TOKEN` environment variable
    Env,
    /// Saved in this credentials file
    File(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedToken {
    pub token: String,
    pub source: TokenSource,
}

/// Canonical form (lowercase, hyphenated) of a valid token, `None` otherwise.
///
/// Tokens are random UUIDv4 values; the server applies the same normalisation,
/// so `3F2B…` and `3f2b…` identify the same owner.
pub fn parse_token(raw: &str) -> Option<String> {
    match Uuid::parse_str(raw.trim()) {
        Ok(uuid) if uuid.get_version_num() == 4 => Some(uuid.to_string()),
        _ => None,
    }
}

/// Finds the token in pasted text or a file, accepting what the web
/// interface hands out: the bare token, `AIDB_TOKEN=…`, `export AIDB_TOKEN=…`,
/// the downloaded `aidb-token.env` (with comment lines) or the copied curl
/// example (`Authorization: Bearer …`).
///
/// Order: an `AIDB_TOKEN=` entry, then a token right after `Bearer`, then an
/// input consisting of nothing but the token.
pub fn extract_token(input: &str) -> Result<String> {
    let lines: Vec<&str> = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    if lines.is_empty() {
        return Err(CliError::Usage(
            "no access token found in the input (is the clipboard empty?)".to_string(),
        ));
    }

    let assigned = lines.iter().find_map(|&line| {
        let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
        line.strip_prefix("AIDB_TOKEN=")
    });
    if let Some(value) = assigned {
        let value = value.trim().trim_matches(|c: char| c == '"' || c == '\'');
        return parse_token(value).ok_or_else(|| {
            CliError::Usage(format!(
                "AIDB_TOKEN in the input is not a valid access token. {HOW_TO_COPY}"
            ))
        });
    }

    // `Authorization: Bearer <token>`, e.g. the copied curl example
    let bearer: BTreeSet<String> = bearer_tokens(input).collect();
    let mut bearer = bearer.into_iter();
    match (bearer.next(), bearer.next()) {
        (Some(token), None) => return Ok(token),
        (Some(_), Some(_)) => {
            return Err(CliError::Usage(
                "the input contains several different access tokens; paste only one".to_string(),
            ))
        }
        (None, _) => {}
    }

    // Only the token itself. Deliberately no search inside arbitrary text:
    // job IDs are UUIDv4 values too, and a pasted job link must not be
    // mistaken for a token.
    let single = match lines.as_slice() {
        [line] => parse_token(line.trim_matches(|c: char| c == '"' || c == '\'')),
        _ => None,
    };
    single.ok_or_else(|| {
        CliError::Usage(format!(
            "no valid AI-DB access token found in the input (expected a UUIDv4 such as \
             3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c). {HOW_TO_COPY}"
        ))
    })
}

/// Valid tokens directly following the word `Bearer` (any case).
fn bearer_tokens(input: &str) -> impl Iterator<Item = String> + '_ {
    let mut words = input.split(|c: char| c.is_whitespace() || c == '"' || c == '\'');
    std::iter::from_fn(move || {
        while let Some(word) = words.next() {
            if word.eq_ignore_ascii_case("bearer") {
                // The next non-empty word is the token candidate
                if let Some(next) = words.by_ref().find(|w| !w.is_empty()) {
                    if let Some(token) = parse_token(next) {
                        return Some(token);
                    }
                }
            }
        }
        None
    })
}

/// Hint appended to import errors.
const HOW_TO_COPY: &str =
    "In the web interface, open Jobs → Access token and use the Copy button next to the token.";

/// Masks all but the last 4 characters, like the web interface.
pub fn mask(token: &str) -> String {
    let tail: String = {
        let chars: Vec<char> = token.chars().collect();
        chars[chars.len().saturating_sub(4)..].iter().collect()
    };
    format!("••••••••-••••-••••-••••-••••••••{tail}")
}

/// The token to use for `server`, or `None` if there is none.
///
/// `env_token` is the value of `AIDB_TOKEN` (passed in, so tests do not have
/// to change the process environment).
pub fn resolve(
    server: &str,
    env_token: Option<String>,
    credentials_path: &Path,
) -> Result<Option<ResolvedToken>> {
    if let Some(raw) = env_token.filter(|t| !t.trim().is_empty()) {
        let token = parse_token(&raw).ok_or_else(|| {
            CliError::Usage(
                "AIDB_TOKEN is set but is not a valid access token (UUIDv4)".to_string(),
            )
        })?;
        return Ok(Some(ResolvedToken {
            token,
            source: TokenSource::Env,
        }));
    }

    let credentials = Credentials::load(credentials_path)?;
    let Some(raw) = credentials.get(server) else {
        return Ok(None);
    };
    let token = parse_token(raw).ok_or_else(|| {
        CliError::Config(format!(
            "the token saved for {server} in {} is invalid; replace it with `aidb auth import --force`",
            credentials_path.display()
        ))
    })?;
    Ok(Some(ResolvedToken {
        token,
        source: TokenSource::File(credentials_path.to_path_buf()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c";
    const SERVER: &str = "https://ai-db.example.org/";

    #[test]
    fn parse_token_normalises_and_rejects_non_v4() {
        assert_eq!(parse_token(&TOKEN.to_uppercase()).as_deref(), Some(TOKEN));
        assert_eq!(parse_token(&format!("  {TOKEN}\n")).as_deref(), Some(TOKEN));
        assert_eq!(parse_token("00000000-0000-0000-0000-000000000000"), None);
        assert_eq!(parse_token("3f2b8c1e-5d4a-1e7b-9a6c-1d2e3f4a5b6c"), None); // v1
        assert_eq!(parse_token("admin"), None);
    }

    #[test]
    fn extract_token_accepts_all_web_formats() {
        let env_file = format!(
            "# AI-DB access token – keep this file private\n\
             # Created 2026-11-02T10:00:00Z for https://ai-db.example.org\n\
             AIDB_TOKEN={TOKEN}\n"
        );
        for input in [
            TOKEN.to_string(),
            format!("{TOKEN}\n"),
            format!("AIDB_TOKEN={TOKEN}"),
            format!("export AIDB_TOKEN={TOKEN}"),
            format!("AIDB_TOKEN=\"{TOKEN}\""),
            format!("OTHER=1\nAIDB_TOKEN={TOKEN}\n"),
            env_file,
            // Copied curl example from the access token dialog
            format!(
                "curl -H \"Authorization: Bearer {TOKEN}\" https://ai-db.example.org/api/jobs/"
            ),
            format!("Authorization: Bearer {}", TOKEN.to_uppercase()),
            format!("'{TOKEN}'"),
        ] {
            assert_eq!(extract_token(&input).unwrap(), TOKEN, "{input:?}");
        }
    }

    #[test]
    fn extract_token_explains_bad_input() {
        for input in [
            "",
            "# only a comment\n",
            "a\nb\n",
            "AIDB_TOKEN=admin",
            "not-a-token",
            // Two different tokens: ambiguous
            "Bearer 3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c\nBearer c0ffee00-0000-4000-8000-000000000000",
            // A job link must not be taken for a token (job IDs are UUIDv4 too)
            "https://ai-db.example.org/jobs/3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c",
            "token: 3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c",
        ] {
            assert!(
                matches!(extract_token(input), Err(CliError::Usage(_))),
                "{input:?}"
            );
        }
    }

    #[test]
    fn mask_keeps_only_the_last_four_characters() {
        assert_eq!(mask(TOKEN), "••••••••-••••-••••-••••-••••••••5b6c");
        assert!(!mask(TOKEN).contains("3f2b"));
    }

    #[test]
    fn environment_wins_over_saved_token() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(crate::credentials::FILE_NAME);
        let mut creds = Credentials::default();
        creds.set(SERVER, "c0ffee00-0000-4000-8000-000000000000".to_string());
        creds.save(&path).unwrap();

        let resolved = resolve(SERVER, Some(TOKEN.to_string()), &path)
            .unwrap()
            .unwrap();
        assert_eq!(resolved.token, TOKEN);
        assert_eq!(resolved.source, TokenSource::Env);

        let resolved = resolve(SERVER, None, &path).unwrap().unwrap();
        assert_eq!(resolved.token, "c0ffee00-0000-4000-8000-000000000000");
        assert_eq!(resolved.source, TokenSource::File(path.clone()));
    }

    #[test]
    fn tokens_are_per_server() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(crate::credentials::FILE_NAME);
        let mut creds = Credentials::default();
        creds.set(SERVER, TOKEN.to_string());
        creds.save(&path).unwrap();
        assert_eq!(
            resolve("http://localhost:8000/", None, &path).unwrap(),
            None
        );
    }

    #[test]
    fn invalid_environment_token_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(crate::credentials::FILE_NAME);
        assert!(matches!(
            resolve(SERVER, Some("nope".into()), &path),
            Err(CliError::Usage(_))
        ));
        // An empty variable is treated as unset
        assert_eq!(resolve(SERVER, Some(String::new()), &path).unwrap(), None);
    }
}
