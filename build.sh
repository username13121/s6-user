#!/usr/bin/env bash

set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
output="$root/packages"
builddir="$root/.build"
sourcedir="$root/.sources"

if [[ $EUID -eq 0 ]]; then
    echo 'build.sh: makepkg must not be run as root' >&2
    exit 1
fi

# packages/ is generated output. Recreate it so it contains only this build.
rm -rf "$output"
mkdir -p "$output" "$builddir" "$sourcedir"

makepkg_args=(--cleanbuild --clean --force --nodeps)

for package_dir in \
    s6-user \
    pipewire-s6-user \
    wireplumber-s6-user
do
    echo "==> Building $package_dir"
    mkdir -p "$sourcedir/$package_dir"
    (
        cd "$root/$package_dir"
        PKGDEST="$output" \
        BUILDDIR="$builddir" \
        SRCDEST="$sourcedir/$package_dir" \
            makepkg "${makepkg_args[@]}"
    )
done

(
    cd "$output"
    sha256sum -- *.pkg.tar.zst >SHA256SUMS
)
echo "==> Packages and SHA256SUMS are in $output"
