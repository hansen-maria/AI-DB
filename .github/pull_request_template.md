## Summary

<!-- What does this PR change, and why? -->

Closes #

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Refactoring (no functional change)
- [ ] Documentation
- [ ] Build / CI / deployment
- [ ] Breaking change (API contract, database schema, CLI behavior)

## Affected components

- [ ] Backend (`backend/`)
- [ ] Shared types (`crates/aidb-types`)
- [ ] Client / CLI (`crates/aidb-client`, `crates/aidb-cli`)
- [ ] Frontend (`frontend/`)
- [ ] Docker / deployment

## How has this been tested?

<!-- Describe the tests you ran: unit tests, manual tests in the UI, API calls, test FASTA files, ... -->

## Screenshots

<!-- For UI changes: before / after. Delete this section otherwise. -->

## Checklist

- [ ] `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass
- [ ] `cargo test --workspace` passes
- [ ] `npm run type-check` and `npm run build` pass (frontend changes)
- [ ] Documentation updated (READMEs, OpenAPI annotations, About/FAQ page)
- [ ] API changes are reflected in `aidb-types` and `api_version`
- [ ] Database changes are migrated on startup without data loss
- [ ] No secrets, tokens, real user data, or database files committed
- [ ] I have read the [contributing guidelines](../CONTRIBUTING.md) and agree to the [Code of Conduct](../CODE_OF_CONDUCT.md)
