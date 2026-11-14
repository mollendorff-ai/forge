#!/usr/bin/env bash
# The checksums.txt of a release, from the archives the build jobs upload.
#
#   scripts/release-checksums.sh artifacts > checksums.txt
#
# Each build job uploads its archive in a folder of its own
# (artifacts/forge-<target>/forge-<target>.tar.gz), and the release publishes
# the archive without the folder. The names here are the published ones, so
# `sha256sum -c checksums.txt` works next to downloaded archives. rc.24 and
# earlier wrote the folder in front of each name, and that check failed with
# "No such file or directory". `forge update` and scripts/homebrew-formula.sh
# read either form.
set -euo pipefail
dir="${1:?usage: release-checksums.sh ARTIFACTS_DIR}"
find "$dir" -type f \( -name '*.tar.gz' -o -name '*.zip' \) -print0 |
  sort -z |
  while IFS= read -r -d '' file; do
    (cd "$(dirname "$file")" && sha256sum -- "$(basename "$file")")
  done
