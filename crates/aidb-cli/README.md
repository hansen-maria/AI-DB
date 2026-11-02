# aidb – AI-DB command-line client

Command-line client for [AI-DB](https://ai-db.computational.bio), the hash-based,
alignment-free annotation service for bacterial proteins.

> Work in progress (currently: v0.1). 

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

## In one step

```bash
# Submit, wait, download: results/proteins_annotations.tsv and .gff3
aidb annotate proteins.faa -f tsv,gff3 -d results/

# A whole folder, only the unmatched sequences as FASTA (for Bakta or Psos)
aidb annotate data/*.faa -f fasta --only unmatched -d unmatched/
```

Inputs and output paths are checked before the first upload; existing result
files are only replaced with `--force`. Exit code 0 means every file was
annotated and saved, 5 that all jobs failed, 9 that some did.

## Jobs

```bash
# Upload protein FASTA (plain or gzipped, max. 100 MB each); one job per file
aidb submit proteins.faa
aidb submit sample_*.faa.gz
aidb submit genome.faa --name "E. coli K-12"

# Status and match counts (job ID or web link)
aidb job show 3f2b8c1e-5d4a-4e7b-9a6c-1d2e3f4a5b6c

# Wait with progress bars; exit 0 = completed, 5 = failed, 6 = timeout
aidb job wait 3f2b8c1e-… 7c1d9a20-… --max-wait 30m

# In scripts: only the job ID, then wait quietly
JOB=$(aidb submit sample.faa -q)
aidb job wait "$JOB" -q && echo done

# Your jobs, newest first
aidb jobs
aidb jobs --status completed --search sample --all

# Results of a completed job (one file per format)
aidb download 3f2b8c1e-… -f tsv,json -d results/
aidb download 3f2b8c1e-… -f fasta --only unmatched --stdout > unmatched.faa
aidb download 3f2b8c1e-… --wait        # still running? wait first
```

Download filters (same as the web interface's sequence table): 
- `--only bakta-db|aidb-db|matched|unmatched`
- `--search TEXT`
- `--min-length` / `--max-length`
- `--cog CODE`
- `--ec-class 1–7`
- `--has-gene`
- `--has-product`

Filtered files are named `…_filtered_annotations.<ext>`. Downloads are owner
only and are written to a temporary file first, so an interrupted download
never leaves a truncated result behind.

Files are checked locally before the upload: FASTA format, size limit and
protein content. Nucleotide FASTA is rejected – translate the genes first
(e.g. with Bakta or Prodigal). `submit` needs an access token (see above), so
the jobs stay yours to manage.

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
