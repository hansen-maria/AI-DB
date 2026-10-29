//! API version compatibility between this client and a server.
//!
//! The server reports its contract version in `GET /api/health`
//! (`api_version`); this client is compiled against
//! [`aidb_types::API_VERSION`].

use aidb_types::API_VERSION;

/// Result of comparing the server's API version with the client's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compatibility {
    /// Same contract version.
    Compatible,
    /// Server predates version reporting (`api_version` missing / 0).
    Unknown,
    /// Server speaks an older contract; some commands may fail.
    ServerOlder { server: u32, client: u32 },
    /// Server speaks a newer contract; the client should be updated.
    ServerNewer { server: u32, client: u32 },
}

impl Compatibility {
    /// `true` only for an exact contract match.
    pub fn is_compatible(self) -> bool {
        self == Compatibility::Compatible
    }
}

/// Compares the server's `api_version` with the client's compiled-in version.
pub fn check_api_version(server: u32) -> Compatibility {
    check(server, API_VERSION)
}

fn check(server: u32, client: u32) -> Compatibility {
    match server {
        0 => Compatibility::Unknown,
        s if s == client => Compatibility::Compatible,
        s if s < client => Compatibility::ServerOlder { server: s, client },
        s => Compatibility::ServerNewer { server: s, client },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_versions() {
        assert_eq!(check(0, 2), Compatibility::Unknown);
        assert_eq!(check(2, 2), Compatibility::Compatible);
        assert_eq!(
            check(1, 2),
            Compatibility::ServerOlder {
                server: 1,
                client: 2
            }
        );
        assert_eq!(
            check(3, 2),
            Compatibility::ServerNewer {
                server: 3,
                client: 2
            }
        );
    }

    #[test]
    fn current_version_is_compatible_with_itself() {
        assert!(check_api_version(API_VERSION).is_compatible());
    }
}
