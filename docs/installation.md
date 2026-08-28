# Installation and migration

Perform PAM and session-manager changes from a disposable machine with an
independent root console available.

## Package order

Install the repositories in this order:

1. `s6-user-projects` packages;
2. `elogind-usersv` packages.

The second set's `elogind-usersv-backend-s6-user` package depends on `s6-user`.

## Install GitHub Release packages

For each repository, download its `.pkg.tar.zst` files and `SHA256SUMS` from
the official GitHub Release, then run:

```sh
sha256sum -c SHA256SUMS
sudo pacman -U ./*.pkg.tar.zst
```

Release packages are unsigned. Download them only from the official repository
pages.

## Build from source

Install `base-devel`, `git`, and Rust/Cargo first. Build and install this
repository:

```sh
git clone https://github.com/username13121/s6-user.git
cd s6-user
./build.sh && sudo pacman -U ./packages/*.pkg.tar.zst
```

Then build/install elogind-usersv from its repository in the same way. Neither
build script runs pacman or resolves dependencies.

This repository produces:

```text
s6-user
pipewire-s6-user
pipewire-pulse-s6-user
wireplumber-s6-user
```

Package renames conflict with and replace this project's old `*-s6` package
names. The installed s6-rc service identifiers remain `pipewire`,
`pipewire-pulse`, and `wireplumber`.

## Select the backend

elogind-usersv deliberately has no default backend. Edit:

```text
/etc/elogind-usersv/config.toml
```

and set:

```toml
backend = "s6-user"
```

The name selects the root-installed executable at:

```text
/usr/libexec/elogind-usersv/backends/s6-user
```

## Enable the system daemon

The system definition is installed at `/etc/s6/sv/elogind-usersvd` and has a
service dependency on `elogind`.

```sh
sudo s6 enable elogind-usersvd
sudo s6 apply
sudo s6 live status elogind-usersvd
```

Do not activate the login PAM module until this status confirms that the
daemon is running with a valid backend configuration.

## Activate PAM

Artix login stacks share `/etc/pam.d/system-login`. Use the packaged,
idempotent editor rather than changing each SDDM, TTY, and SSH stack:

```sh
sudo elogind-usersv-pam enable
elogind-usersv-pam status
```

It inserts this required session entry immediately after `pam_elogind.so`:

```pam
session required pam_elogind_usersv.so
```

The tool refuses symlinks, unsafe ownership/modes, ambiguous anchors, duplicate
entries, and unexpected pre-existing usersv configuration. Disable it before
rollback:

```sh
sudo elogind-usersv-pam disable
```

Package removal also attempts to disable the managed line so a required PAM
entry cannot be left pointing at a removed module.

Artix pambase may retain an optional `pam_turnstile.so` line. It belongs to
pambase and is harmless when that module is absent; this project does not edit
or remove it.

## Disable duplicate PipeWire startup

Do not run both the desktop's PipeWire launcher and s6-user services. Locate
old XDG launchers:

```sh
grep -RIl 'artix-pipewire-launcher' \
    /etc/xdg/autostart \
    /usr/share/xdg/autostart \
    "$HOME/.config/autostart" 2>/dev/null || :
```

Disable package-owned desktop entries with a per-user override rather than
deleting them. For an entry named `pipewire.desktop`:

```sh
mkdir -p "$HOME/.config/autostart"
cat >"$HOME/.config/autostart/pipewire.desktop" <<'EOF'
[Desktop Entry]
Hidden=true
EOF
```

## First login and operation

Fully log out of all sessions, then log in again. The backend initializes the
repository automatically. Verify as the unprivileged user:

```sh
s6-user paths export
s6-user live status
s6-user set status
s6-user process status pipewire
```

Persistent service policy is changed with:

```sh
s6-user disable pipewire-pulse
s6-user apply

s6-user enable pipewire-pulse
s6-user apply
```

Package updates are imported with:

```sh
s6-user repository sync
s6-user apply
```

## Configure persistent paths

System policy belongs in `/etc/s6-user/config.toml`; per-user overrides belong
in `$XDG_CONFIG_HOME/s6-user/config.toml`. Only persistent paths are accepted.
See [Architecture](architecture.md) for fields and defaults.

Change persistent paths only while the user's manager is stopped. Migrate the
repository and compiled database before logging in with the new configuration.
Runtime paths remain fixed beneath `$XDG_RUNTIME_DIR`.

## Migrate away from Turnstile

Before enabling usersv PAM integration:

1. remove any manually added `pam_turnstile` activation;
2. disable and stop `turnstiled`;
3. install/configure/start `elogind-usersvd`;
4. run `sudo elogind-usersv-pam enable`;
5. log out of every session and log back in;
6. remove old Turnstile packages after verification.

Existing `$XDG_STATE_HOME/s6-rc/repository` and
`$XDG_CONFIG_HOME/s6-rc/compiled/current` data can be retained. The renamed
audio packages install the same service identifiers and source locations.

## Rollback

From a root console:

```sh
sudo elogind-usersv-pam disable
sudo s6 disable elogind-usersvd
sudo s6 apply
```

Restore the desktop PipeWire launcher only after the per-user manager no longer
starts PipeWire. Removing packages does not delete user repositories or custom
configuration.
