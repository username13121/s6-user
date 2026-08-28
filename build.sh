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

# packages/ is generated output. Recreate it so the pacman wildcard below can
# never select stale releases from an earlier build.
rm -rf "$output"
mkdir -p "$output" "$builddir" "$sourcedir"

makepkg_args=(--cleanbuild --clean --force --nodeps)
if [[ ${SIGN_PACKAGES:-0} == 1 ]]; then
    makepkg_args+=(--sign)
    if [[ -n ${GPGKEY:-} ]]; then
        makepkg_args+=(--key "$GPGKEY")
    fi
fi

for package_dir in \
    turnstile \
    s6-user \
    turnstile-s6 \
    turnstile-backend-s6 \
    pipewire-s6 \
    wireplumber-s6
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

echo "==> Packages are in $output"
echo "==> Install with: sudo pacman -U $output/*.pkg.tar.zst"
