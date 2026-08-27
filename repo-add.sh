#!/usr/bin/env bash

set -euo pipefail
shopt -s nullglob

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_name=${1:-artix-s6-user}
packages=("$root"/packages/*.pkg.tar.zst)

if (( ${#packages[@]} == 0 )); then
    echo 'repo-add.sh: no packages found; run ./build.sh first' >&2
    exit 1
fi

args=()
if [[ ${SIGN_REPO:-0} == 1 ]]; then
    args+=(-s)
    if [[ -n ${GPGKEY:-} ]]; then
        args+=(-k "$GPGKEY")
    fi
fi

repo-add "${args[@]}" "$root/packages/$repo_name.db.tar.gz" "${packages[@]}"
echo "Repository database: $root/packages/$repo_name.db.tar.gz"
