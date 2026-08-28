# Installation and migration

## Prerequisites

Use an Artix installation booted with s6 and with elogind, s6-frontend 0.1.0.0, Turnstile, PipeWire, and WirePlumber available from configured repositories. Install the normal package build tools if building from Git:

```sh
sudo pacman -S --needed base-devel git
```

Do not install `turnstile-dinit` on an s6 system. `turnstile-s6` provides/conflicts with the common `init-turnstile` capability in the same way as other Artix init integrations.

## Build and install from Git

```sh
git clone https://github.com/username13121/s6-user.git
cd s6-user
./build.sh && sudo pacman -U ./packages/*.pkg.tar.zst
```

The transaction installs all six package names (the PipeWire pkgbase emits two packages):

```text
s6-user
turnstile-s6
turnstile-backend-s6
pipewire-s6
pipewire-pulse-s6
wireplumber-s6
```

The three audio service packages are usable reference definitions. They demonstrate readiness and dependency handling, but the general per-user s6 infrastructure is the project's primary purpose.

## Select the Turnstile backend

Edit `/etc/turnstile/turnstiled.conf`. Set the following policy explicitly (do not append duplicate keys if they already exist):

```ini
backend = s6
rundir_path = /run/user/%u
manage_rundir = no
linger = no
```

Meaning:

- `backend = s6` selects `/usr/lib/turnstile/s6`.
- `manage_rundir = no` leaves `/run/user/$UID` under elogind ownership.
- `linger = no` stops the user manager after the last logout.

These settings are deliberately not installed or changed by any package.

Turnstile still needs `pam_turnstile` in the applicable SDDM/login/sshd PAM session stacks, alongside `pam_elogind`. Use the Turnstile/Artix PAM setup appropriate to the machine and inspect the active files under `/etc/pam.d`; this project does not overwrite PAM policy.

## Enable the system daemon

The definition is installed at `/etc/s6/sv/turnstiled`. Add it to the system set and apply that set:

```sh
sudo s6 enable turnstiled
sudo s6 apply
```

Confirm it is running:

```sh
sudo s6 live status turnstiled
sudo s6 process status turnstiled
```

`turnstiled` must be running before a PAM login attempts to use it. This version intentionally does not attach a guessed `login.target`-style dependency to SDDM, sshd, or TTY services; current Artix s6 definitions do not provide one common stable edge.

## Disable duplicate PipeWire autostart

First verify that the s6-managed services work. Then find old XDG launchers:

```sh
command -v artix-pipewire-launcher || :
grep -RIl 'artix-pipewire-launcher' \
    /etc/xdg/autostart \
    /usr/share/xdg/autostart \
    "$HOME/.config/autostart" 2>/dev/null || :
```

A common Artix launcher is `/usr/bin/artix-pipewire-launcher`, invoked by a desktop entry named `pipewire.desktop`. Do **not** delete the executable or a package-owned desktop file. Disable the desktop entry through the desktop environment's autostart settings or with a per-user XDG override using the same desktop-file name. For example, when the system entry is `pipewire.desktop`:

```sh
mkdir -p "$HOME/.config/autostart"
cat >"$HOME/.config/autostart/pipewire.desktop" <<'EOF'
[Desktop Entry]
Hidden=true
EOF
```

Repeat per user, or deploy an administrator policy appropriate to the desktop. Do not leave both the launcher and s6 starting PipeWire: the launcher can kill/restart `pipewire` and `wireplumber` behind s6's back.

## Activate the user manager

Fully log out of **all** sessions for the user, then log back in. On first login the backend initializes the user's repository automatically. No manual `repository init`, `set commit`, service copying, or runtime-directory creation is needed.

Check:

```sh
s6-user live status
s6-user set status
s6-user process status pipewire
```

Expected persistent paths with default XDG locations:

```text
~/.config/s6-rc/sources
~/.config/s6-rc/compiled/current
~/.local/state/s6-rc/repository
```

`s6-user` has no configuration file. It applies the fixed path policy with explicit s6-frontend command-line options.

Expected runtime paths while logged in:

```text
/run/user/$UID/service
/run/user/$UID/s6-rc
/run/user/$UID/s6-frontend
```

## Service policy

The packaged audio services have `flag-recommended`, so they are enabled when first discovered. Change persistent policy with:

```sh
s6-user disable pipewire-pulse
s6-user apply
```

or:

```sh
s6-user enable pipewire-pulse
s6-user apply
```

`enable`/`disable` edit the user's working set. `apply` commits, installs, and resets live state. `repository sync` preserves prescriptions for existing services:

```sh
s6-user repository sync
s6-user apply
```

Machine-wide overrides go under `/etc/s6-rc/user/sources`; individual overrides go under `$XDG_CONFIG_HOME/s6-rc/sources`. Use the same service directory name to override an earlier global definition, then synchronize/apply each affected user's private repository.

## Migrate from the 0.1.0-1 filesystem layout

The revised paths replace the original `s6/user-sv` names. For an existing test user, migrate persistent user data while no user manager is running:

```sh
: "${XDG_CONFIG_HOME:=$HOME/.config}"
: "${XDG_STATE_HOME:=$HOME/.local/state}"

mkdir -p "$XDG_CONFIG_HOME/s6-rc" "$XDG_STATE_HOME/s6-rc"

if [ -d "$XDG_CONFIG_HOME/s6/user-sv" ] && \
   [ ! -e "$XDG_CONFIG_HOME/s6-rc/sources" ]; then
    mv "$XDG_CONFIG_HOME/s6/user-sv" \
       "$XDG_CONFIG_HOME/s6-rc/sources"
fi

if [ -d "$XDG_STATE_HOME/s6/repo" ] && \
   [ ! -e "$XDG_STATE_HOME/s6-rc/repository" ]; then
    mv "$XDG_STATE_HOME/s6/repo" \
       "$XDG_STATE_HOME/s6-rc/repository"
fi
```

After installing the revised packages, update the migrated repository's store links and compile it:

```sh
s6-user repository init --update-stores
s6-user set commit -f
```

The backend will install and boot that compiled set on the next clean login. The obsolete `$XDG_CONFIG_HOME/s6/user.conf` is no longer read and may be removed after verifying the migration. Machine overrides under `/etc/s6/user-sv` must be moved manually to `/etc/s6-rc/user/sources` by the administrator.

If preserving existing enable/disable prescriptions is unnecessary, omit the repository move; the backend creates a fresh repository using the recommended flags from the new stores.

## Rollback

Before removing packages, select another Turnstile backend (or disable Turnstile management) in administrator policy and stop/disable `turnstiled` as appropriate. Restore the prior PipeWire autostart only after the s6 user manager is no longer starting it. Removing these packages does not remove users' repositories or XDG policy automatically.
