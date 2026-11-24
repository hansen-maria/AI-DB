# Security Policy

## Supported versions

Security fixes are applied to the latest state of the `main` branch, which is
what runs on the public instance at
[ai-db.computational.bio](https://ai-db.computational.bio), and to the latest
release of the `aidb` command-line client.

| Component                              | Supported          |
|----------------------------------------|--------------------|
| Public instance (ai-db.computational.bio) | :white_check_mark: |
| `main` branch                          | :white_check_mark: |
| Latest `aidb-cli` release              | :white_check_mark: |
| Older releases / self-hosted forks     | :x:                |

## Reporting a vulnerability

**Please do not report security vulnerabilities through public GitHub issues,
discussions, or pull requests.**

Instead, report them privately via one of these channels:

1. **GitHub private vulnerability reporting** (preferred):
   [Report a vulnerability](https://github.com/hansen-maria/AI-DB/security/advisories/new)
2. **Email**: ai-db@computational.bio
   (subject line starting with `[SECURITY]`)

Please include as much of the following as you can:

- Type of issue (e.g. authorization bypass, injection, information disclosure)
- Affected component (backend API, frontend, CLI, deployment) and endpoint or
  file path
- Steps to reproduce, ideally with a minimal proof of concept
- Impact: what an attacker could achieve
- Any suggested mitigation

## What to expect

- We aim to acknowledge your report within **5 working days**.
- We will keep you informed about the assessment and the fix.
- Once the issue is resolved, we will publish a security advisory and credit
  you, unless you prefer to remain anonymous.

AI-DB is maintained by a small academic team, so please allow reasonable time
for a fix before any public disclosure.

## Scope

Of particular interest are issues affecting:

- Job ownership and read-only sharing (owner tokens, `ai_db_user` cookie)
- The admin endpoints (`/api/admin/*`, `X-Admin-Secret`)
- Integrity of the AI-DB annotations DB (ingest verification)
- Unintended disclosure of submitted sequences or job data

Out of scope:

- Denial of service through high request volume
- Vulnerabilities in third-party services (Bakta Web API, Psos, UniProt, NCBI,
  QuickGO) — please report those to their maintainers
- Findings from automated scanners without a demonstrated impact

## Safe harbor

We will not pursue legal action against researchers who act in good faith,
avoid privacy violations and service disruption, do not access or modify other
users' jobs beyond what is needed to demonstrate the issue, and give us
reasonable time to respond before disclosure.
