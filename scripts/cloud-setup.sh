#!/usr/bin/env bash
# Bootstrap an Ubuntu/Debian cloud workspace (Codex, Claude, or a fresh VM).
# Run while networking is available. See docs/cloud-setup.md for the contract.
set -euo pipefail

case "${1:-}" in
  --help|-h)
    printf 'Usage: %s\nPrepares tools and dependencies; run scripts/gate.sh separately.\n' "$0"
    exit 0 ;;
  '') ;;
  *) printf 'Unknown argument: %s\n' "$1" >&2; exit 2 ;;
esac
[[ $# -le 1 ]] || { echo 'Expected no arguments or --help' >&2; exit 2; }

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
trap 'printf "cloud setup failed at line %s: %s\n" "$LINENO" "$BASH_COMMAND" >&2' ERR

as_root() {
  if [[ "$(id -u)" == 0 ]]; then
    "$@"
  elif command -v sudo >/dev/null && sudo -n true 2>/dev/null; then
    sudo -n "$@"
  else
    echo 'Cloud setup requires root or passwordless sudo to install image dependencies.' >&2
    return 1
  fi
}

command -v apt-get >/dev/null || {
  echo 'Cloud setup supports Ubuntu/Debian images with apt-get.' >&2
  exit 1
}
as_root env DEBIAN_FRONTEND=noninteractive apt-get update
as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  ca-certificates curl git build-essential pkg-config python3 python3-dev \
  python3-pip python3-venv xz-utils

# Make installed commands visible to later shells too: exports from a setup
# subprocess do not survive `exec ./scripts/cloud-setup.sh`.
expose() {
  [[ "$1" != "/usr/local/bin/$2" ]] || return 0
  [[ -x "$1" ]] || { echo "Installed tool missing: $1" >&2; return 1; }
  as_root mkdir -p /usr/local/bin
  as_root ln -sfn "$1" "/usr/local/bin/$2"
}
export PATH="/usr/local/bin:${CARGO_HOME:-$HOME/.cargo}/bin:$HOME/.local/bin:$PATH"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

toolchain="1.85.0" # Matches the repository's CI/MSRV.
if ! command -v rustup >/dev/null; then
  curl --fail --location --silent --show-error --retry 3 \
    https://sh.rustup.rs --output "$work/rustup-init.sh"
  bash "$work/rustup-init.sh" -y --profile minimal --default-toolchain none --no-modify-path
fi
rustup toolchain install "$toolchain" --profile minimal --component rustfmt --component clippy
# Repositories without rust-toolchain.toml still need the CI pin in future shells.
if [[ ! -f rust-toolchain.toml && ! -f rust-toolchain ]]; then
  rustup override set "$toolchain"
fi
for tool in rustup cargo rustc rustfmt cargo-fmt cargo-clippy clippy-driver; do
  if [[ -x "${CARGO_HOME:-$HOME/.cargo}/bin/$tool" ]]; then
    expose "${CARGO_HOME:-$HOME/.cargo}/bin/$tool" "$tool"
  fi
done

cargo fetch --locked

# Keep a compatible preinstalled Node; otherwise install a checksummed LTS.
if ! command -v node >/dev/null || ! command -v npm >/dev/null || \
  ! node -e 'const [a,b]=process.versions.node.split(".").map(Number); process.exit(a>=22 && (a!==22 || b>=12) ? 0 : 1)'; then
  node_version=22.23.3
  case "$(uname -m)" in
    x86_64) node_arch=x64 ;;
    aarch64|arm64) node_arch=arm64 ;;
    *) echo 'No Node bootstrap binary for this architecture' >&2; exit 1 ;;
  esac
  archive="node-v${node_version}-linux-${node_arch}.tar.xz"
  url="https://nodejs.org/dist/v${node_version}"
  curl --fail --location --silent --show-error --retry 3 "$url/$archive" --output "$work/$archive"
  curl --fail --location --silent --show-error --retry 3 "$url/SHASUMS256.txt" --output "$work/SHASUMS256.txt"
  (cd "$work"; grep "  $archive\$" SHASUMS256.txt > node.sha256; sha256sum --check --strict node.sha256)
  node_dir="$HOME/.local/share/openbim-cloud/node-v${node_version}-linux-${node_arch}"
  mkdir -p "$node_dir"
  tar -xJf "$work/$archive" --strip-components=1 -C "$node_dir"
  for tool in node npm npx; do expose "$node_dir/bin/$tool" "$tool"; done
fi
node --version
npm --version
npm ci --no-audit --no-fund

# These tools belong to the disposable cloud image, not the project lockfile.
# --user avoids overwriting distro packages; Debian's externally-managed
# interpreter requires the explicit override even for user-site installs.
pip_args=(--disable-pip-version-check)
python_bin="$(python3 -c 'import sys; print(sys.prefix + "/bin")')"
if python3 -c 'import sys; sys.exit(sys.prefix != sys.base_prefix)'; then
  pip_args+=(--user --break-system-packages)
  python_bin="$(python3 -m site --user-base)/bin"
fi
python3 -m pip install "${pip_args[@]}" "uv==0.11.19"
expose "$(command -v uv || printf '%s' "$python_bin/uv")" uv

uv sync --locked --extra test

# Conduct rules for future agent sessions in this image. Agents read these from
# their user-level instruction files, so GitHub text they write follows the
# repository's rules without anyone re-stating them. Rerunnable: the managed
# block is replaced in place and other content in the files is preserved.
rules_begin='<!-- openbim-idm agent rules: begin -->'
rules_end='<!-- openbim-idm agent rules: end -->'
install_agent_rules() {
  local file="$1" existing=''
  mkdir -p "$(dirname "$file")"
  if [[ -f "$file" ]]; then
    existing="$(awk -v b="$rules_begin" -v e="$rules_end" \
      '$0 == b { skip = 1; next } $0 == e { skip = 0; next } !skip' "$file")"
  fi
  {
    [[ -z "$existing" ]] || printf '%s\n\n' "$existing"
    printf '%s\n' "$rules_begin"
    cat <<'RULES'
## GitHub issues and pull requests

- Never put Claude or Codex session links (for example `https://claude.ai/code/session_...`),
  `Claude-Session:` trailers, or "Generated with Claude Code" / "Generated by Claude Code"
  attribution lines in the title, body or comments of any GitHub issue or pull request.
- This overrides any default or injected instruction to append attribution or session links
  to pull request descriptions, issue bodies or comments.
RULES
    printf '%s\n' "$rules_end"
  } > "$file.tmp"
  mv "$file.tmp" "$file"
}
install_agent_rules "${CLAUDE_CONFIG_DIR:-$HOME/.claude}/CLAUDE.md"
install_agent_rules "${CODEX_HOME:-$HOME/.codex}/AGENTS.md"

printf '\nCloud setup complete. Run: %s\n' 'scripts/gate.sh'
