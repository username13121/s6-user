# Architecture

## Responsibility boundaries

### elogind

elogind remains the authority for login sessions, seats, inhibitors, and
`/run/user/$UID`. Neither s6-user nor its service packages create or retain the
runtime directory.

### elogind-usersv

elogind-usersv watches eligible elogind sessions, keeps one verified background
elogind lease per managed UID, and starts/stops one selected backend. Its
`s6-user` backend is a separate package in the elogind-usersv repository.

### s6-user backend

The backend prepares the user's offline repository through public `s6-user`
commands, obtains resolved paths from `s6-user paths export`, and execs the
actual `s6-svscan` manager. PAM readiness is shallow: login proceeds when
`s6-svscan` enters its event loop while the s6-rc boot transaction continues.

The backend is named `s6-user`, not `s6`. The latter remains available for a
future implementation maintained by the s6 project.

### s6-user

`s6-user` is a non-resident Rust wrapper around `/usr/bin/s6 --user`. It:

1. normalizes HOME and XDG persistent bases;
2. reads typed system and per-user TOML configuration;
3. validates all configured persistent paths;
4. creates the user-owned source store and repository/boot-database parents;
5. supplies explicit `repodir`, `bootdb`, `storelist`, and empty fdholder user;
6. replaces itself with `s6`.

It does not implement service supervision or session lifecycle.

## Filesystem policy

| Path | Purpose |
|---|---|
| `/usr/share/s6-rc/user/sources/` | package-maintained user service definitions |
| `/etc/s6-rc/user/sources/` | administrator definitions and overrides |
| `$XDG_CONFIG_HOME/s6-rc/sources/` | individual user definitions and overrides |
| `$XDG_STATE_HOME/s6-rc/repository/` | individual repository and prescriptions |
| `$XDG_CONFIG_HOME/s6-rc/compiled/current` | persistent compiled boot database |
| `$XDG_RUNTIME_DIR/service/` | live supervision scan directory |
| `$XDG_RUNTIME_DIR/s6-rc/` | live s6-rc state |
| `$XDG_RUNTIME_DIR/s6-frontend/` | frontend temporary state |

Store precedence is:

```text
package store < administrator store < individual user store
```

Later stores replace definitions with the same service name.

## Persistent configuration

Configuration is merged field-by-field:

```text
built-in defaults
  < /etc/s6-user/config.toml
  < $XDG_CONFIG_HOME/s6-user/config.toml
```

Supported fields are:

```toml
repository_dir = "/absolute/path/to/repository"
boot_database = "/absolute/path/to/compiled/current"
package_store = "/absolute/path/to/package/sources"
administrator_store = "/absolute/path/to/admin/sources"
user_store = "/absolute/path/to/user/sources"
```

All configured paths must be absolute and valid UTF-8. Store paths may not
contain `:` because s6-frontend uses a colon-separated store list. Unknown
fields are rejected.

Runtime paths are intentionally absent from this configuration. On the
supported s6-frontend, `s6 --user` derives `scandir`, `livedir`, and `stmpdir`
from `XDG_RUNTIME_DIR`. This prevents persistent policy changes from moving
live state outside the elogind-owned runtime directory.

`version export` in s6-frontend 0.1.0.0 misreports `bootdb`. The stable
`s6-user paths export` command substitutes the configured boot path while
retaining s6-frontend's resolved runtime paths.

## Repository startup

On first login the backend:

1. initializes the repository and links configured stores;
2. commits the initial recommended service set;
3. installs the boot database when no live state exists;
4. starts `s6-svscan` and reports its shallow readiness;
5. starts `s6-user system boot` asynchronously.

On later logins it validates and synchronizes the repository while preserving
existing enable/disable prescriptions. It does not replace boot state beneath
an existing live s6-rc database.

The asynchronous boot helper has a manager-lifetime watchdog. If the tracked
`s6-svscan` disappears, the pending boot transaction is terminated rather than
being left orphaned during manager restart.

## Shutdown

The backend first requests:

```sh
s6-user live stop-everything -E -t 10000
```

It then sends `SIGTERM` to the tracked `s6-svscan`. elogind-usersv supplies the
configured TERM/KILL fallback and retains the elogind background lease until
the manager has exited.
