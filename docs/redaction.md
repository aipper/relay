# Input Redaction (MVP)

Inputs sent from PWA/CLI to a run are **recorded** for replay/debugging, but stored as:

- `text_redacted` (safe for UI)
- `text_sha256` (fingerprint of raw text)

Raw inputs are **not** stored by default.

## Default Rules

1. Key-value masking (case-insensitive keys): `api_key`, `token`, `password`, `secret`, `authorization`
2. Header masking: `Authorization: Bearer <...>` → `Authorization: Bearer ***REDACTED***`
3. Known token patterns (examples): `sk-...`, `ghp_...`, AWS `AKIA...` (extend as needed)
4. Long high-entropy substrings: `[A-Za-z0-9+/=_-]{32,}` → `***REDACTED***` (best-effort)

## Configuration

Planned envs:

- `STORE_RAW_INPUT=false` (default)
- `REDACTION_EXTRA_REGEX=<comma-separated regex>`

