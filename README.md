# Artix per-user s6 services

Packages for running one `s6-svscan`/s6-rc service manager per logged-in user, with Turnstile handling first-login/last-logout lifecycle and elogind retaining normal login/session/seat and runtime-directory ownership.

```text
elogind + turnstiled + login PAM
                       |
                       v
             per-user s6-svscan
                       |
                       v
                    user s6-rc
                /       |        \
          pipewire  wireplumber  pipewire-pulse
```

## Packages

- **s6-user** — thin XDG-aware policy wrapper for per-user s6-frontend commands.
- **turnstile-s6** — system s6-rc source at `/etc/s6/sv/turnstiled`.
- **turnstile-backend-s6** — `/usr/lib/turnstile/s6` backend.
- **pipewire-s6** / **pipewire-pulse-s6** — split package with ready-notifying user services.
- **wireplumber-s6** — WirePlumber user service.

Package definitions are global under `/usr/share/s6-rc/user/sources`; service policy and compiled sets are private to each user under `$XDG_STATE_HOME` and `$XDG_CONFIG_HOME`.

## Build and install from Git

On Artix with the normal build tools installed:

```sh
git clone https://github.com/<you>/<repo>
cd <repo>

./build.sh
sudo pacman -U ./packages/*.pkg.tar.zst
```

`build.sh` uses `makepkg` and intentionally skips build-time dependency checks because these packages only install data/scripts. `pacman` still enforces every dependency from the package metadata during installation.

Then:

1. Edit `/etc/turnstile/turnstiled.conf` and select the `s6` backend.
2. Keep elogind in place and set `manage_rundir = no`.
3. Set `linger = no` for first-login/last-logout behavior.
4. Enable/start the system `turnstiled` s6 service.
5. Disable the old PipeWire XDG autostart launcher without deleting package files.
6. Fully log out, then log back in.

See **[Installation](docs/installation.md)** for exact settings and commands.

## User commands

```sh
s6-user live status

s6-user start wireplumber
s6-user stop wireplumber
s6-user start pipewire
s6-user stop pipewire

s6-user enable pipewire
s6-user disable pipewire
s6-user repository sync
s6-user apply
```

Use `s6-user` for all user-tree operations so they cannot accidentally target the system repository at `/etc/s6/repo`. The wrapper is stateless: it applies the fixed user path/store policy and `exec`s s6-frontend.

## Documentation

- [Architecture and filesystem policy](docs/architecture.md)
- [Installation, migration, and custom pacman repository](docs/installation.md)
- [Lifecycle and behavior test checklist](docs/testing.md)

## Scope

This project does **not** replace Turnstile, overwrite Turnstile policy, remove elogind, create `/run/user/$UID`, or delete Artix PipeWire package files. The administrator explicitly chooses the backend and migration policy.

License: [0BSD](LICENSE).
