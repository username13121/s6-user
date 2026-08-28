# Test checklist

Use a non-production Artix machine and keep an independent root console open
while testing PAM/session changes.

## Automated checks

In `s6-user-projects`:

```sh
cargo test --locked --manifest-path s6-user/Cargo.toml
cargo clippy --locked --manifest-path s6-user/Cargo.toml --all-targets -- -D warnings
./build.sh
sha256sum -c packages/SHA256SUMS
```

In `elogind-usersv`:

```sh
make test
make test-live
./build.sh --clean
sha256sum -c packages/SHA256SUMS
```

`make test-live` requires a running elogind/login1 system service.

## Package contents

- [ ] `s6-user` contains `/usr/bin/s6-user` and
      `/etc/s6-user/config.toml`.
- [ ] User service definitions exist only below
      `/usr/share/s6-rc/user/sources/{pipewire,pipewire-pulse,wireplumber}`.
- [ ] No per-user package belongs to `s6-world`.
- [ ] No package contains Turnstile binaries, backends, or system services.
- [ ] `elogind-usersv` contains daemon, supervisor, PAM module, PAM editor, and
      required internal PAM profile.
- [ ] `elogind-usersv-backend-s6-user` contains only the `s6-user` backend and
      appropriate documentation/license files.
- [ ] `elogind-usersv-s6` contains `/etc/s6/sv/elogind-usersvd` with an elogind
      service dependency; it does not depend on a user backend package.

## Path policy

- [ ] `s6-user paths export` reports runtime `scandir`, `livedir`, and
      `stmpdir` beneath `$XDG_RUNTIME_DIR`.
- [ ] The repository, boot database, and stores use s6-user persistent policy,
      not `/etc/s6/repo` or `/etc/s6/sv`.
- [ ] System configuration is overridden field-by-field by per-user
      configuration.
- [ ] Relative paths, unknown TOML fields, line breaks, and `:` in store paths
      are rejected.
- [ ] Runtime path fields are rejected as unknown configuration.
- [ ] s6-frontend's incorrect exported `bootdb` does not leak through
      `s6-user paths export`.

## Backend selection

- [ ] Missing `backend` causes a clear configuration failure.
- [ ] Invalid names containing uppercase characters, slashes, leading dots, or
      traversal are rejected.
- [ ] `backend = "s6-user"` resolves only to
      `/usr/libexec/elogind-usersv/backends/s6-user`.
- [ ] The daemon runs before PAM integration is enabled.

## PAM integration

- [ ] `elogind-usersv-pam enable` inserts exactly one required entry after the
      common `pam_elogind.so` line in `/etc/pam.d/system-login`.
- [ ] A second enable is a no-op.
- [ ] `status` reports enabled.
- [ ] `disable` removes only the managed line; a second disable is a no-op.
- [ ] Ownership and mode of `system-login` are preserved.
- [ ] Ambiguous or manually altered usersv entries fail closed.
- [ ] Package removal cannot leave the required usersv line active.

## First login

- [ ] A real `Class=user` or `Class=user-early` login starts exactly one
      user-owned `s6-svscan`.
- [ ] login1 shows a separate `Service=elogind-usersv-manager`,
      `Class=background` lease.
- [ ] PAM returns after shallow `s6-svscan` readiness.
- [ ] The repository and compiled database are initialized automatically.
- [ ] PipeWire reaches socket readiness before dependents are marked up.
- [ ] WirePlumber and PipeWire Pulse start through their `pipewire` dependency.

## Concurrent sessions

- [ ] SDDM, TTY, and SSH sessions for one UID share one manager and one
      PipeWire process.
- [ ] Closing one session retains the manager while another eligible session
      remains.
- [ ] Final logout performs dependency-aware shutdown, exits `s6-svscan`, then
      closes the background elogind lease.
- [ ] elogind removes `$XDG_RUNTIME_DIR` only after the manager wrapper exits.

## Supervision and restart

- [ ] Killing WirePlumber causes s6 to restart it.
- [ ] Killing `s6-svscan` causes elogind-usersv to restart the manager while a
      login remains.
- [ ] A boot transaction pending during manager death is terminated and does
      not overlap the replacement manager.
- [ ] No duplicate audio process remains after recovery.

## Policy persistence and overrides

- [ ] Disabled services remain disabled across logout/login, repository sync,
      and package upgrade.
- [ ] A newly discovered recommended service becomes active.
- [ ] Administrator definitions override package definitions.
- [ ] Individual definitions override administrator and package definitions.
- [ ] Two logged-in users have independent repositories and runtime trees.

## Administrative termination

- [ ] `loginctl terminate-user USER` terminates sessions, manager, helper, and
      lease without leaving runtime state.
- [ ] Daemon shutdown waits for helpers and managers.
- [ ] Forced cgroup-wide SIGTERM/SIGKILL is not treated as dependency-ordered
      graceful shutdown.
