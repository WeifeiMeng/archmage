#!/bin/bash
set -euo pipefail

usage() {
  echo "usage: scripts/verify-macos-release.sh <dmg-path> <version>" >&2
}

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "verify-macos-release: this verifier requires macOS" >&2
  exit 1
fi
if [[ $# -ne 2 ]]; then
  usage
  exit 2
fi

dmg_input=$1
expected_version=$2
if [[ ! -f "$dmg_input" ]]; then
  echo "verify-macos-release: DMG does not exist: $dmg_input" >&2
  exit 1
fi
if [[ ! "$expected_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z-]+)*$ ]]; then
  echo "verify-macos-release: invalid version: $expected_version" >&2
  exit 1
fi

dmg_dir=$(cd "$(dirname "$dmg_input")" && pwd -P)
dmg="$dmg_dir/$(basename "$dmg_input")"
temp_root=$(mktemp -d "${TMPDIR:-/tmp}/archmage-release.XXXXXX")
mount_dir="$temp_root/mount"
mounted=0
mkdir "$mount_dir"

cleanup() {
  if [[ $mounted -eq 1 ]]; then
    hdiutil detach "$mount_dir" >/dev/null 2>&1 || hdiutil detach -force "$mount_dir" >/dev/null 2>&1 || true
  fi
  rm -rf "$temp_root"
}
trap cleanup EXIT INT TERM

echo "Verifying DMG container: $dmg"
hdiutil verify "$dmg"
hdiutil attach -readonly -nobrowse -mountpoint "$mount_dir" "$dmg" >/dev/null
mounted=1

mapfile_support=0
if [[ -n "${BASH_VERSION:-}" ]] && (( BASH_VERSINFO[0] >= 4 )); then
  mapfile_support=1
fi
if [[ $mapfile_support -eq 1 ]]; then
  mapfile -t apps < <(find "$mount_dir" -maxdepth 2 -type d -name 'Archmage.app' -print)
else
  apps=()
  while IFS= read -r app; do apps+=("$app"); done < <(find "$mount_dir" -maxdepth 2 -type d -name 'Archmage.app' -print)
fi
if [[ ${#apps[@]} -ne 1 ]]; then
  echo "verify-macos-release: expected exactly one Archmage.app, found ${#apps[@]}" >&2
  exit 1
fi

app=${apps[0]}
plist="$app/Contents/Info.plist"
if [[ ! -f "$plist" ]]; then
  echo "verify-macos-release: missing Info.plist" >&2
  exit 1
fi

plist_value() {
  plutil -extract "$1" raw -o - "$plist"
}

identifier=$(plist_value CFBundleIdentifier)
version=$(plist_value CFBundleShortVersionString)
executable_name=$(plist_value CFBundleExecutable)
minimum_system=$(plist_value LSMinimumSystemVersion)

[[ "$identifier" == "dev.archmage.workbench" ]] || {
  echo "verify-macos-release: unexpected CFBundleIdentifier: $identifier" >&2
  exit 1
}
[[ "$version" == "$expected_version" ]] || {
  echo "verify-macos-release: unexpected CFBundleShortVersionString: $version" >&2
  exit 1
}
[[ "$minimum_system" == "14.0" ]] || {
  echo "verify-macos-release: unexpected LSMinimumSystemVersion: $minimum_system" >&2
  exit 1
}

executable="$app/Contents/MacOS/$executable_name"
if [[ ! -x "$executable" ]]; then
  echo "verify-macos-release: main executable is missing or not executable: $executable" >&2
  exit 1
fi
architectures=$(lipo -archs "$executable" | xargs)
[[ "$architectures" == "arm64" ]] || {
  echo "verify-macos-release: expected only arm64, found: $architectures" >&2
  exit 1
}

forbidden=$(find "$app" \
  \( -name '.env' -o -name '.env.*' -o -name '.git' -o -name '*.db' -o -name '*.sqlite' \
     -o -name 'node_modules' -o -name 'target' -o -iname '*fixture*' \
     -o -name '.credentials*' -o -name 'credentials.json' -o -name '*.pem' \
     -o -name '*.p12' -o -name '*.key' \) -print -quit)
if [[ -n "$forbidden" ]]; then
  echo "verify-macos-release: forbidden development or sensitive payload: $forbidden" >&2
  exit 1
fi

echo "Code-signing metadata (Developer ID/notarization are not claimed for this Preview):"
codesign -dvv "$app" 2>&1 || true
echo "Verified Archmage $version: identifier=$identifier architecture=$architectures minimum_macos=$minimum_system"
