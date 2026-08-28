# Artix per-user s6 services

This repository packages the `s6-user` policy wrapper and reusable per-user
service definitions. Lifecycle management is provided separately by
[`elogind-usersv`](https://github.com/username13121/elogind-usersv).

```text
system s6 -> elogind + elogind-usersvd
                         |
                         v
              elogind-usersv backend: s6-user
                         |
                         v
                 per-user s6-svscan
                         |
                         v
             pipewire / wireplumber / pipewire-pulse
```

## Packages

- **s6-user** — a small Rust wrapper around `s6 --user`. It configures only
  persistent repository, boot-database, and service-store paths.
- **pipewire-s6-user** / **pipewire-pulse-s6-user** — ready-notifying per-user
  service definitions.
- **wireplumber-s6-user** — a per-user WirePlumber service definition.

Package names use `-s6-user` to distinguish this policy from system s6 service
packages and any future official s6 user-service implementation. Service names
remain `pipewire`, `pipewire-pulse`, and `wireplumber`.

## Install release packages without compiling

Download all `.pkg.tar.zst` files and `SHA256SUMS` from this repository's
official GitHub Release. Verify and install them:

```sh
sha256sum -c SHA256SUMS
sudo pacman -U ./*.pkg.tar.zst
```

The packages are currently unsigned. Checksums detect download corruption but
do not replace package signatures; download only from the official repository
release page.

Install this package set before installing the `elogind-usersv` package set.

## Build and install from source

Install `base-devel`, `git`, and Rust/Cargo first. The build script never runs
pacman or installs dependencies.

```sh
git clone https://github.com/username13121/s6-user.git
cd s6-user
./build.sh && sudo pacman -U ./packages/*.pkg.tar.zst
```

`build.sh` performs clean `makepkg --nodeps` builds and writes package checksums
to `packages/SHA256SUMS`.

## User commands

```sh
s6-user live status
s6-user process status pipewire

s6-user start wireplumber
s6-user stop pipewire
s6-user enable pipewire
s6-user disable pipewire-pulse
s6-user repository sync
s6-user apply
```

Use `s6-user` instead of raw `s6` for user-tree operations. It prevents the
system `/etc/s6/repo` and system stores from being selected by
`/etc/s6-frontend.conf`.

## Configuration

System defaults and per-user overrides are read in this order:

```text
built-in defaults
/etc/s6-user/config.toml
$XDG_CONFIG_HOME/s6-user/config.toml
```

Only persistent paths are configurable. Runtime paths are selected by
`s6 --user` from `XDG_RUNTIME_DIR` and cannot be overridden through s6-user.
Inspect the complete resolved policy with:

```sh
s6-user paths export
```

See [Architecture](docs/architecture.md), [Installation](docs/installation.md),
[Testing](docs/testing.md), and [Releasing](docs/releasing.md).

License: [0BSD](LICENSE).
