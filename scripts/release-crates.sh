#!/usr/bin/env bash

set -euo pipefail

case ${1:-} in
  --check)
    publish=0
    ;;
  --publish)
    publish=1
    ;;
  *)
    echo "usage: release-crates.sh --check|--publish" >&2
    exit 2
    ;;
esac

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH='' cd -- "$script_dir/.." && pwd)
cd "$repository_root"
bun "$script_dir/release-metadata.ts" >/dev/null

package_version() {
  awk -F '"' '/^version = "/ { print $2; exit }' "$1"
}

scratch_dir=$(mktemp -d -t quickgui-crates-release.XXXXXX)
trap 'rm -rf "$scratch_dir"' EXIT

crate_is_published() {
  local crate_name=$1
  local crate_version=$2
  local response_file="$scratch_dir/${crate_name}.json"
  local http_status

  if ! http_status=$(curl \
    --silent \
    --show-error \
    --location \
    --retry 5 \
    --retry-all-errors \
    --output "$response_file" \
    --write-out '%{http_code}' \
    --header 'User-Agent: QuickGUI release workflow' \
    "https://crates.io/api/v1/crates/$crate_name/$crate_version")
  then
    echo "release-crates: crates.io request failed for $crate_name $crate_version" >&2
    return 2
  fi

  case $http_status in
    200)
      if [[ $(jq -r '.version.num // empty' "$response_file") != "$crate_version" ]]; then
        echo "release-crates: crates.io returned unexpected metadata for $crate_name $crate_version" >&2
        return 2
      fi
      if [[ $(jq -r '.version.yanked' "$response_file") == true ]]; then
        echo "release-crates: $crate_name $crate_version exists but is yanked" >&2
        return 2
      fi
      return 0
      ;;
    404)
      return 1
      ;;
    *)
      echo "release-crates: crates.io returned HTTP $http_status for $crate_name $crate_version" >&2
      return 2
      ;;
  esac
}

wait_for_crate() {
  local crate_name=$1
  local crate_version=$2
  local status

  for _attempt in {1..60}; do
    if crate_is_published "$crate_name" "$crate_version"; then
      return 0
    else
      status=$?
      if (( status != 1 )); then
        return "$status"
      fi
    fi
    sleep 5
  done

  echo "release-crates: timed out waiting for $crate_name $crate_version" >&2
  return 1
}

crate_names=(
  quickgui-extension-sdk
  quickgui-winit
  quickgui-accesskit-winit
  quickgui-cosmic-text
  quickgui-glyphon
  quickgui-system
  quickgui
)
crate_manifests=(
  crates/quickgui-extension-sdk/Cargo.toml
  vendor/winit/Cargo.toml
  vendor/accesskit_winit/Cargo.toml
  vendor/cosmic_text/Cargo.toml
  vendor/glyphon/Cargo.toml
  crates/quickgui-system/Cargo.toml
  Cargo.toml
)

for index in "${!crate_names[@]}"; do
  crate_name=${crate_names[$index]}
  manifest=${crate_manifests[$index]}
  crate_version=$(package_version "$manifest")
  if [[ -z $crate_version ]]; then
    echo "release-crates: could not read the version from $manifest" >&2
    exit 1
  fi

  if crate_is_published "$crate_name" "$crate_version"; then
    echo "release-crates: $crate_name $crate_version is already public; skipping"
    continue
  else
    status=$?
    if (( status != 1 )); then
      exit "$status"
    fi
  fi

  if (( publish == 0 )); then
    echo "release-crates: $crate_name $crate_version is ready to publish"
    continue
  fi
  if [[ -z ${CARGO_REGISTRY_TOKEN:-} ]]; then
    echo "release-crates: CARGO_REGISTRY_TOKEN is required for publication" >&2
    exit 1
  fi

  publish_args=(--manifest-path "$manifest")
  if [[ ${QUICKGUI_RELEASE_ALLOW_DIRTY:-0} == 1 ]]; then
    publish_args+=(--allow-dirty)
  fi
  if [[ $crate_name == quickgui-system || $crate_name == quickgui ]]; then
    publish_args+=(--locked)
  fi
  cargo publish "${publish_args[@]}"
  wait_for_crate "$crate_name" "$crate_version"
  echo "release-crates: published $crate_name $crate_version"
done
