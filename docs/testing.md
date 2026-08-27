# Test checklist

Run lifecycle tests on a non-production machine. Keep a root console available while changing PAM/session infrastructure.

## Package and configuration sanity

- [ ] All packages build with `./build.sh`.
- [ ] `pacman -Qlp packages/*.pkg.tar.zst` shows:
  - `/usr/bin/s6-user`;
  - `/usr/lib/turnstile/s6`;
  - `/etc/s6/sv/turnstiled/{type,run}` (not `/etc/s6/adminsv`);
  - audio services only under `/usr/share/s6/user-sv`.
- [ ] `s6-user version export` reports user `scandir`, `livedir`, and `repodir` paths.
- [ ] Running `s6-user` does not export `S6_CONF` back into the calling shell.
- [ ] A normal root/system `s6 version export` still reports `/etc/s6/repo` and the system stores.
- [ ] `/run/user/$UID` is created by elogind before backend startup, not by these packages.

## First login

Start with no repository for a disposable test user (or a newly created user).

- [ ] Log in once through SDDM, TTY, or SSH.
- [ ] `/run/user/$UID` exists.
- [ ] Exactly one user-owned `s6-svscan` starts.
- [ ] `$XDG_STATE_HOME/s6/repo` is initialized automatically.
- [ ] `$XDG_CONFIG_HOME/s6-rc/compiled/current` exists.
- [ ] `s6-user live status` shows `pipewire`, `wireplumber`, and `pipewire-pulse` up when all three packages are installed and recommended.
- [ ] `${PIPEWIRE_RUNTIME_DIR:-$XDG_RUNTIME_DIR}/pipewire-0` is a socket before dependents are considered up.
- [ ] `$XDG_RUNTIME_DIR/pulse/native` is a socket before `pipewire-pulse` is considered up.

## Concurrent sessions

Record the manager and PipeWire PIDs:

```sh
pgrep -u "$USER" -x s6-svscan
pgrep -u "$USER" -x pipewire
```

- [ ] Open a second simultaneous session for the same user (include SSH in this test).
- [ ] The same `s6-svscan` PID remains.
- [ ] The same PipeWire PID remains; no duplicate PipeWire process appears.
- [ ] Log out of one session.
- [ ] The user manager and audio services remain while the other session is active.
- [ ] Log out of the last session.
- [ ] s6-rc stops user services dependency-aware and `s6-svscan` exits.
- [ ] `/run/user/$UID` eventually disappears through elogind cleanup.

## Supervision

With a login still active:

```sh
kill "$(pgrep -n -u "$USER" -x wireplumber)"
```

- [ ] s6 restarts WirePlumber with a new PID.

Kill the tracked user manager unexpectedly:

```sh
kill -KILL "$(pgrep -n -u "$USER" -x s6-svscan)"
```

- [ ] Turnstile starts a replacement `s6-svscan` while the login remains.
- [ ] The user service graph becomes operational again.
- [ ] No second persistent manager or duplicate PipeWire remains.

## Dependency graph

With all audio services up:

```sh
s6-user stop pipewire
```

- [ ] `wireplumber` goes down.
- [ ] `pipewire-pulse` goes down.
- [ ] `pipewire` goes down.

Then:

```sh
s6-user start wireplumber
```

- [ ] PipeWire starts first.
- [ ] PipeWire reaches socket readiness.
- [ ] WirePlumber starts after readiness.
- [ ] `pipewire-pulse` stays down unless explicitly requested or restored by policy.

## Persistent user policy

```sh
s6-user disable pipewire-pulse
s6-user apply
s6-user set status pipewire-pulse
```

- [ ] Log out of the last session and log back in; `pipewire-pulse` stays disabled.
- [ ] Run `s6-user repository sync`, then `s6-user apply`; it stays disabled.
- [ ] Rebuild/reinstall the same service package to simulate an update, synchronize/apply again, and verify it stays disabled.
- [ ] A genuinely new service with `flag-recommended` is active when first discovered.

## Overrides

- [ ] Add a controlled override with the same service name under `/etc/s6/user-sv`, synchronize, and verify it replaces `/usr/share/s6/user-sv`.
- [ ] Add an individual override under `$XDG_CONFIG_HOME/s6/user-sv`, synchronize, and verify it replaces both global definitions.
- [ ] Remove test overrides and synchronize/apply again.

## Multiple users

For two simultaneously logged-in users:

- [ ] Both use definitions from `/usr/share/s6/user-sv`.
- [ ] Each has a distinct `$XDG_STATE_HOME/s6/repo`.
- [ ] Each has a distinct `$XDG_RUNTIME_DIR/service` and `$XDG_RUNTIME_DIR/s6-rc`.
- [ ] Disabling `pipewire-pulse` for Alice does not change Bob's set.
- [ ] Logging Alice out of her last session does not stop Bob's manager.

## Desktop integration

- [ ] No XDG desktop entry invokes `artix-pipewire-launcher` for the migrated user.
- [ ] Starting a desktop does not change the s6-managed PipeWire/WirePlumber PIDs unexpectedly.
- [ ] elogind login1, seat, inhibitor, suspend/power, and `/run/user/$UID` behavior still works normally.
