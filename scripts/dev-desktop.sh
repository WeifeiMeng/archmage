#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"
desktop_dir="$repo_root/apps/archmage-desktop"

for executable in git cargo node npm codex; do
  if ! command -v "$executable" >/dev/null 2>&1; then
    printf 'Archmage desktop requires %s on PATH.\n' "$executable" >&2
    exit 1
  fi
done

if [[ ! -d "$desktop_dir/node_modules" ]]; then
  printf 'Installing Archmage desktop dependencies...\n'
  npm install --prefix "$desktop_dir"
fi

cd "$repo_root"
exec npm run tauri dev --prefix "$desktop_dir"
