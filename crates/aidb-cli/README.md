# AI-DB command-line client

Command-line client for [AI-DB](https://ai-db.computational.bio), the hash-based,
alignment-free annotation service for bacterial proteins.

> Work in progress (v0.1). 

## Access token

Your jobs belong to a random access token. Use the same token as in the web
interface, so web and CLI jobs belong together:

```bash
# Web interface: Jobs → Access token → Copy, then paste when asked
aidb auth import

# or the downloaded file / a pipe
aidb auth import --file aidb-token.env
pbpaste | aidb auth import

# no web jobs yet? create a new token
aidb auth init

aidb auth show            # masked
aidb auth show --reveal   # full token, e.g. for a CI secret
```

Tokens are saved per server in `~/.config/aidb/credentials.toml` (Windows:
`%APPDATA%\aidb\`, override with `AIDB_CONFIG_DIR`), readable only by you.
`AIDB_TOKEN` takes precedence, which suits CI and HPC jobs.

## Server

```bash
aidb status               # health, version, compatibility with this CLI
aidb db info              # Bakta DB release, AI-DB annotations DB status
```

| Option            | Environment     | Default                            |
|-------------------|-----------------|------------------------------------|
| `--server <URL>`  | `AIDB_SERVER`   | `https://ai-db.computational.bio`  |
| `-o human\|json`  | `AIDB_OUTPUT`   | `human`                            |
| `--timeout <S>`   |                 | `30`                               |

Data goes to stdout, hints and errors to stderr. Only the JSON output is meant
for parsing.

## Exit codes

| Code | Meaning                                       |
|------|-----------------------------------------------|
| 0    | success                                       |
| 1    | general error                                 |
| 2    | wrong usage                                   |
| 3    | not authorized / no access token              |
| 4    | not found                                     |
| 5    | job failed                                    |
| 6    | timeout while waiting for a job               |
| 7    | server not reachable or temporarily failing   |
| 8    | input rejected                                |
| 9    | partially successful batch                    |
