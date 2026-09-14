#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH='' cd -- "$script_dir/.." && pwd)
cd "$repository_root"
bun "$script_dir/release-metadata.ts" >/dev/null

package_target_dir=${QUICKGUI_PACKAGE_TARGET_DIR:-"$repository_root/target/package-gate"}
if [[ $package_target_dir != /* ]]; then
  package_target_dir="$repository_root/$package_target_dir"
fi

allow_dirty_arg=""
if [[ ${QUICKGUI_PACKAGE_ALLOW_DIRTY:-0} == 1 ]]; then
  allow_dirty_arg=--allow-dirty
fi

cargo_command=(cargo)
if [[ -n ${QUICKGUI_PACKAGE_TOOLCHAIN:-} ]]; then
  cargo_command=(cargo "+$QUICKGUI_PACKAGE_TOOLCHAIN")
fi

package_version() {
  awk -F '"' '/^version = "/ { print $2; exit }' "$1"
}

winit_version=$(package_version vendor/winit/Cargo.toml)
accesskit_version=$(package_version vendor/accesskit_winit/Cargo.toml)
cosmic_text_version=$(package_version vendor/cosmic_text/Cargo.toml)
glyphon_version=$(package_version vendor/glyphon/Cargo.toml)
system_version=$(package_version crates/quickgui-system/Cargo.toml)
extension_sdk_version=$(package_version crates/quickgui-extension-sdk/Cargo.toml)
quickgui_version=$(package_version Cargo.toml)

winit_patch="patch.crates-io.quickgui-winit.path=\"$repository_root/vendor/winit\""
accesskit_patch="patch.crates-io.quickgui-accesskit-winit.path=\"$repository_root/vendor/accesskit_winit\""
cosmic_text_patch="patch.crates-io.quickgui-cosmic-text.path=\"$repository_root/vendor/cosmic_text\""
glyphon_patch="patch.crates-io.quickgui-glyphon.path=\"$repository_root/vendor/glyphon\""
system_patch="patch.crates-io.quickgui-system.path=\"$repository_root/crates/quickgui-system\""
extension_sdk_patch="patch.crates-io.quickgui-extension-sdk.path=\"$repository_root/crates/quickgui-extension-sdk\""

CARGO_TARGET_DIR="$package_target_dir" "${cargo_command[@]}" package \
  --manifest-path vendor/winit/Cargo.toml \
  ${allow_dirty_arg:+"$allow_dirty_arg"}
CARGO_TARGET_DIR="$package_target_dir" "${cargo_command[@]}" package \
  --manifest-path vendor/accesskit_winit/Cargo.toml \
  --no-verify \
  ${allow_dirty_arg:+"$allow_dirty_arg"} \
  --config "$winit_patch"
CARGO_TARGET_DIR="$package_target_dir" "${cargo_command[@]}" package \
  --manifest-path vendor/cosmic_text/Cargo.toml \
  ${allow_dirty_arg:+"$allow_dirty_arg"}
CARGO_TARGET_DIR="$package_target_dir" "${cargo_command[@]}" package \
  --manifest-path vendor/glyphon/Cargo.toml \
  ${allow_dirty_arg:+"$allow_dirty_arg"} \
  --config "$cosmic_text_patch"
CARGO_TARGET_DIR="$package_target_dir" "${cargo_command[@]}" package \
  --manifest-path crates/quickgui-system/Cargo.toml \
  --locked \
  ${allow_dirty_arg:+"$allow_dirty_arg"}
CARGO_TARGET_DIR="$package_target_dir" "${cargo_command[@]}" package \
  --manifest-path crates/quickgui-extension-sdk/Cargo.toml \
  --locked \
  ${allow_dirty_arg:+"$allow_dirty_arg"}
CARGO_TARGET_DIR="$package_target_dir" "${cargo_command[@]}" package \
  --locked \
  --no-verify \
  ${allow_dirty_arg:+"$allow_dirty_arg"} \
  --config "$winit_patch" \
  --config "$accesskit_patch" \
  --config "$cosmic_text_patch" \
  --config "$glyphon_patch" \
  --config "$system_patch" \
  --config "$extension_sdk_patch"

package_dir="$package_target_dir/package"
winit_archive="$package_dir/quickgui-winit-${winit_version}.crate"
accesskit_archive="$package_dir/quickgui-accesskit-winit-${accesskit_version}.crate"
cosmic_text_archive="$package_dir/quickgui-cosmic-text-${cosmic_text_version}.crate"
glyphon_archive="$package_dir/quickgui-glyphon-${glyphon_version}.crate"
system_archive="$package_dir/quickgui-system-${system_version}.crate"
extension_sdk_archive="$package_dir/quickgui-extension-sdk-${extension_sdk_version}.crate"
quickgui_archive="$package_dir/quickgui-${quickgui_version}.crate"

for archive in "$winit_archive" "$accesskit_archive" "$cosmic_text_archive" "$glyphon_archive" "$system_archive" "$extension_sdk_archive" "$quickgui_archive"; do
  if [[ ! -f $archive ]]; then
    echo "package-release-gate: missing archive $archive" >&2
    exit 1
  fi
done

scratch_dir=$(mktemp -d -t quickgui-package-gate.XXXXXX)
trap 'rm -rf "$scratch_dir"' EXIT

tar -xzf "$winit_archive" -C "$scratch_dir"
tar -xzf "$accesskit_archive" -C "$scratch_dir"
tar -xzf "$cosmic_text_archive" -C "$scratch_dir"
tar -xzf "$glyphon_archive" -C "$scratch_dir"
tar -xzf "$system_archive" -C "$scratch_dir"
tar -xzf "$extension_sdk_archive" -C "$scratch_dir"
tar -xzf "$quickgui_archive" -C "$scratch_dir"
cp -R tests/downstream_smoke "$scratch_dir/consumer"

required_files=(
  "quickgui-winit-${winit_version}/LICENSE"
  "quickgui-winit-${winit_version}/README.md"
  "quickgui-accesskit-winit-${accesskit_version}/LICENSE-APACHE"
  "quickgui-accesskit-winit-${accesskit_version}/README.md"
  "quickgui-cosmic-text-${cosmic_text_version}/LICENSE-APACHE"
  "quickgui-cosmic-text-${cosmic_text_version}/LICENSE-MIT"
  "quickgui-cosmic-text-${cosmic_text_version}/README.md"
  "quickgui-glyphon-${glyphon_version}/LICENSE-APACHE"
  "quickgui-glyphon-${glyphon_version}/LICENSE-MIT"
  "quickgui-glyphon-${glyphon_version}/LICENSE-ZLIB"
  "quickgui-glyphon-${glyphon_version}/README.md"
  "quickgui-system-${system_version}/Cargo.toml"
  "quickgui-system-${system_version}/src/lib.rs"
  "quickgui-extension-sdk-${extension_sdk_version}/Cargo.toml"
  "quickgui-extension-sdk-${extension_sdk_version}/src/lib.rs"
  "quickgui-${quickgui_version}/src/lib.rs"
  "quickgui-${quickgui_version}/LICENSE-MIT"
  "quickgui-${quickgui_version}/LICENSE-APACHE"
  "quickgui-${quickgui_version}/CHANGELOG.md"
  "quickgui-${quickgui_version}/THIRD_PARTY_NOTICES.md"
  "quickgui-${quickgui_version}/README.md"
)
for relative_path in "${required_files[@]}"; do
  if [[ ! -f "$scratch_dir/$relative_path" ]]; then
    echo "package-release-gate: packaged file is missing: $relative_path" >&2
    exit 1
  fi
done

assert_only_top_level_entries() {
  local package_directory=$1
  shift
  local entry entry_name allowed_name allowed

  while IFS= read -r entry; do
    entry_name=${entry##*/}
    allowed=0
    for allowed_name in "$@"; do
      if [[ $entry_name == "$allowed_name" ]]; then
        allowed=1
        break
      fi
    done
    if (( allowed == 0 )); then
      echo "package-release-gate: unexpected packaged entry: $package_directory/$entry_name" >&2
      exit 1
    fi
  done < <(find "$scratch_dir/$package_directory" -mindepth 1 -maxdepth 1 -print | LC_ALL=C sort)
}

common_cargo_entries=(.cargo_vcs_info.json Cargo.lock Cargo.toml Cargo.toml.orig)
assert_only_top_level_entries "quickgui-winit-${winit_version}" \
  "${common_cargo_entries[@]}" LICENSE README.md build.rs src
assert_only_top_level_entries "quickgui-accesskit-winit-${accesskit_version}" \
  "${common_cargo_entries[@]}" LICENSE-APACHE README.md src
assert_only_top_level_entries "quickgui-cosmic-text-${cosmic_text_version}" \
  "${common_cargo_entries[@]}" LICENSE-APACHE LICENSE-MIT README.md src
assert_only_top_level_entries "quickgui-glyphon-${glyphon_version}" \
  "${common_cargo_entries[@]}" LICENSE-APACHE LICENSE-MIT LICENSE-ZLIB README.md src
assert_only_top_level_entries "quickgui-system-${system_version}" \
  "${common_cargo_entries[@]}" src
assert_only_top_level_entries "quickgui-extension-sdk-${extension_sdk_version}" \
  "${common_cargo_entries[@]}" src
assert_only_top_level_entries "quickgui-${quickgui_version}" \
  "${common_cargo_entries[@]}" build.rs CHANGELOG.md LICENSE-APACHE LICENSE-MIT README.md \
  THIRD_PARTY_NOTICES.md src

if [[ -d "$scratch_dir/quickgui-${quickgui_version}/vendor" ]]; then
  echo "package-release-gate: the main archive must not duplicate vendored support sources" >&2
  exit 1
fi

packaged_winit_patch="patch.crates-io.quickgui-winit.path=\"$scratch_dir/quickgui-winit-${winit_version}\""
packaged_accesskit_patch="patch.crates-io.quickgui-accesskit-winit.path=\"$scratch_dir/quickgui-accesskit-winit-${accesskit_version}\""
packaged_cosmic_text_patch="patch.crates-io.quickgui-cosmic-text.path=\"$scratch_dir/quickgui-cosmic-text-${cosmic_text_version}\""
packaged_glyphon_patch="patch.crates-io.quickgui-glyphon.path=\"$scratch_dir/quickgui-glyphon-${glyphon_version}\""
packaged_system_patch="patch.crates-io.quickgui-system.path=\"$scratch_dir/quickgui-system-${system_version}\""
packaged_extension_sdk_patch="patch.crates-io.quickgui-extension-sdk.path=\"$scratch_dir/quickgui-extension-sdk-${extension_sdk_version}\""
packaged_quickgui_patch="patch.crates-io.quickgui.path=\"$scratch_dir/quickgui-${quickgui_version}\""

CARGO_TARGET_DIR="$package_target_dir/downstream" "${cargo_command[@]}" check \
  --manifest-path "$scratch_dir/consumer/Cargo.toml" \
  --config "$packaged_winit_patch" \
  --config "$packaged_accesskit_patch" \
  --config "$packaged_cosmic_text_patch" \
  --config "$packaged_glyphon_patch" \
  --config "$packaged_system_patch" \
  --config "$packaged_extension_sdk_patch" \
  --config "$packaged_quickgui_patch"

read -r winit_hash _ < <(shasum -a 256 "$winit_archive")
read -r accesskit_hash _ < <(shasum -a 256 "$accesskit_archive")
read -r cosmic_text_hash _ < <(shasum -a 256 "$cosmic_text_archive")
read -r glyphon_hash _ < <(shasum -a 256 "$glyphon_archive")
read -r system_hash _ < <(shasum -a 256 "$system_archive")
read -r extension_sdk_hash _ < <(shasum -a 256 "$extension_sdk_archive")
read -r quickgui_hash _ < <(shasum -a 256 "$quickgui_archive")

checksum_file="$package_dir/SHA256SUMS"
printf '%s  %s\n' \
  "$winit_hash" "$(basename "$winit_archive")" \
  "$accesskit_hash" "$(basename "$accesskit_archive")" \
  "$cosmic_text_hash" "$(basename "$cosmic_text_archive")" \
  "$glyphon_hash" "$(basename "$glyphon_archive")" \
  "$system_hash" "$(basename "$system_archive")" \
  "$extension_sdk_hash" "$(basename "$extension_sdk_archive")" \
  "$quickgui_hash" "$(basename "$quickgui_archive")" \
  > "$checksum_file"

printf '%s\n' \
  "QUICKGUI_PACKAGE_RESULT {\"winit_sha256\":\"${winit_hash}\",\"accesskit_sha256\":\"${accesskit_hash}\",\"cosmic_text_sha256\":\"${cosmic_text_hash}\",\"glyphon_sha256\":\"${glyphon_hash}\",\"system_sha256\":\"${system_hash}\",\"extension_sdk_sha256\":\"${extension_sdk_hash}\",\"quickgui_sha256\":\"${quickgui_hash}\",\"checksums\":\"SHA256SUMS\",\"required_files\":true,\"minimal_top_level\":true,\"vendor_excluded\":true,\"downstream_check\":true,\"passed\":true}"
