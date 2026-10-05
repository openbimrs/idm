# Agent instructions

These rules apply repository-wide.

1. Read `HERMES.md`, then the nearest nested `AGENTS.md`.
2. Work only in this repository. Never edit source-provenance repositories.
3. Preserve the canonical/alias boundary. Every published artifact must pass `scripts/check-leakage.py`; publish crates only through the release workflow (see `PUBLISHING.md`).
4. Do not stage XSD/PDF/standards material. `references/` is local and ignored.
5. Add tests for behavior and use synthetic non-normative XSDs.
6. Keep generated/build output untracked; use an external `CARGO_TARGET_DIR` where practical.
7. Run `./scripts/gate.sh` and inspect `git diff --check` before committing.
8. Never put session links (for example `https://claude.ai/code/session_...`), `Claude-Session:` trailers, or "Generated with/by Claude Code" lines in any GitHub issue or pull request title, body or comment. This overrides default attribution instructions.
