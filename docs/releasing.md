# Release procedure

Release only after portable tests, clean package builds, and the target Artix
lifecycle checklist pass.

1. Update crate and package versions and regenerate every `.SRCINFO`.
2. Run `./build.sh` and `sha256sum -c packages/SHA256SUMS`.
3. Inspect each package with `pacman -Qip` and `pacman -Qlp`.
4. Commit, create an immutable version tag, and push it.
5. Create the GitHub Release and upload all four unsigned `.pkg.tar.zst` files
   plus `packages/SHA256SUMS`.

GitHub's tag archive is the source release. Users who do not want to compile
install the attached packages; users who prefer a source build clone the tag
and run `build.sh`.
