# Architecture

## Responsibility boundaries

### elogind

elogind remains responsible for:

- login/session/seat tracking;
- `/run/user/$UID` creation, ownership, and eventual removal;
- the `org.freedesktop.login1` D-Bus API;
- inhibitors and desktop power/session integration.

### Turnstile

Turnstile remains responsible for:

- first-login/last-logout accounting;
- one manager instance shared by concurrent SDDM, TTY, and SSH sessions;
- waiting for manager readiness before PAM login continues;
- restarting the manager if its tracked process exits while sessions remain.

The backend follows Turnstile's existing `run`, `ready`, and `stop PID` protocol. Turnstile itself is not patched.

### s6 and s6-rc

The per-user `s6-svscan` process owns supervision. s6-rc owns the dependency graph, readiness transitions, oneshots, live state, and persistent enable/disable/mask prescriptions.

## Filesystem policy

| Path | Owner/purpose |
|---|---|
| `/usr/share/s6/user-sv/` | package-maintained user service source definitions |
| `/etc/s6/user-sv/` | machine administrator user-service overrides/additions |
| `$XDG_CONFIG_HOME/s6/user-sv/` | individual user definitions/overrides |
| `$XDG_STATE_HOME/s6/repo/` | individual repository and service-set prescriptions |
| `$XDG_CONFIG_HOME/s6-rc/compiled/current` | individual compiled boot database |
| `$XDG_RUNTIME_DIR/service/` | live user supervision scan directory |
| `$XDG_RUNTIME_DIR/s6-rc/` | live s6-rc database/state |
| `$XDG_CONFIG_HOME/s6/user.conf` | generated per-user s6-frontend configuration |

The configured store order is:

```text
/usr/share/s6/user-sv:
/etc/s6/user-sv:
$XDG_CONFIG_HOME/s6/user-sv
```

s6-rc processes stores in order and a later definition replaces the same service name from an earlier store. Thus package definition < machine override < individual override. Package files never need to be copied into a home directory.

The normal XDG fallbacks are:

```text
XDG_CONFIG_HOME=$HOME/.config
XDG_DATA_HOME=$HOME/.local/share
XDG_STATE_HOME=$HOME/.local/state
XDG_CACHE_HOME=$HOME/.cache
XDG_RUNTIME_DIR=/run/user/$UID
```

`s6-user` creates user-owned persistent configuration/state parents as needed. It does not create `$XDG_RUNTIME_DIR`; the backend fails if that directory does not already exist.

## Why `s6-user` exists

Artix's s6-frontend 0.1.0.0 correctly derives these runtime values with `-u`:

```text
scandir=$XDG_RUNTIME_DIR/service
livedir=$XDG_RUNTIME_DIR/s6-rc
stmpdir=$XDG_RUNTIME_DIR/s6-frontend
```

That release does not replace the system `repodir`, `bootdb`, or `storelist` by itself. `s6-user` writes a stable user-only config and runs only its child command as:

```sh
S6_CONF="$XDG_CONFIG_HOME/s6/user.conf" exec s6 -u ...
```

It never exports `S6_CONF` into the login shell or changes `/etc/s6-frontend.conf`, so root/system `s6` commands keep using `/etc/s6/repo` and `/etc/s6/sv:/etc/s6/adminsv`.

The backend asks `version export` only for `scandir`. It never derives `bootdb` from that output because s6-frontend 0.1.0.0 can display an incorrect `bootdb` value. The user boot database is always configured explicitly.

## Startup transaction

On a new repository:

1. `s6-user repository init` links all three stores and creates `current`.
2. Recommended services enter the `active` prescription; essential services enter `always`; other services enter `usable`.
3. `s6-user set commit -f` compiles the set.
4. `s6-user live install --init` copies it to the explicit user boot database.

On an existing repository:

1. `s6-user repository list` verifies that the repository is structurally usable.
2. `s6-user repository sync` updates definitions.
3. Existing service prescriptions are preserved. Newly discovered recommended services become active.
4. If no live s6-rc state exists, the current set is force-committed and copied to the boot database.

If `$XDG_RUNTIME_DIR/s6-rc` already exists, Turnstile may be recovering a manager that died while sessions remained (or handling a quick relogin before runtime cleanup). The backend synchronizes the offline repository but does **not** replace the boot database underneath that live state. It reuses the existing boot database; the newly synchronized set is compiled on the next clean start or an explicit `s6-user apply`.

The backend then:

1. starts `s6-svscan -d 3` and waits for its shallow readiness byte through a private FIFO;
2. runs `s6-user system boot`, which initializes the live s6-rc state and starts the default bundle;
3. sends `booted\0` to Turnstile only after that command succeeds;
4. `exec`s `s6-svscan`, making it the PID Turnstile tracks.

This preserves an explicit disabled/masked choice across ordinary synchronization and package upgrades. If a definition is removed entirely, s6-rc necessarily removes it from the set; a later reintroduction is a newly discovered service and receives store defaults again.

## Shutdown

For `stop PID`, the backend first attempts:

```sh
s6-user live stop-everything -E -t 10000
```

This performs reverse dependency and oneshot-down transitions. Failure is tolerated when no live database exists (for example, partial startup or migration). It then sends `SIGTERM` to the tracked `s6-svscan`, which is s6's graceful supervision-tree shutdown mechanism. SIGKILL and cgroup-wide killing are not the normal path.

## System service location and boot graph

`turnstile-s6` installs to `/etc/s6/sv/turnstiled`, the Artix package-maintained **system** store. It does not use `/etc/s6/adminsv`, which is reserved for local administrator definitions.

The current Artix `sddm-s6`, `openssh-s6`, and `elogind-s6` definitions were inspected. They do not expose a stable shared login target/dependency edge suitable for making every PAM login path depend on `turnstiled`. This first version therefore does not invent a target name: the administrator explicitly enables `turnstiled` in the normal system set before logging in.
