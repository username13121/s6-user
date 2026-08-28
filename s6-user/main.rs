use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use serde::Deserialize;

const SYSTEM_CONFIG: &str = "/etc/s6-user/config.toml";
const S6: &str = "/usr/bin/s6";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("s6-user: {error}");
            ExitCode::from(70)
        }
    }
}

fn run() -> Result<(), String> {
    let environment = Environment::load()?;
    let paths = Paths::load(&environment)?;
    let arguments: Vec<OsString> = env::args_os().skip(1).collect();

    if arguments
        .first()
        .is_some_and(|argument| argument == "paths")
    {
        return paths_command(&arguments[1..], &environment, &paths);
    }

    paths.prepare()?;
    let error = frontend_command(&environment, &paths, &arguments).exec();
    Err(format!("cannot execute {S6}: {error}"))
}

fn paths_command(
    arguments: &[OsString],
    environment: &Environment,
    paths: &Paths,
) -> Result<(), String> {
    match arguments {
        [argument] if argument == "export" => {
            let resolved = ResolvedRuntime::query(environment, paths)?;
            println!("version=1");
            println!("scandir={}", resolved.scandir);
            println!("livedir={}", resolved.livedir);
            println!("repodir={}", paths.repository_dir.display());
            println!("bootdb={}", paths.boot_database.display());
            println!("stmpdir={}", resolved.stmpdir);
            println!("storelist={}", paths.storelist()?);
            println!("fdhuser=");
            println!("verbosity=1");
            Ok(())
        }
        [argument] if argument == "help" => {
            println!("Usage: s6-user paths export");
            Ok(())
        }
        _ => Err("usage: s6-user paths export".into()),
    }
}

fn frontend_command(environment: &Environment, paths: &Paths, arguments: &[OsString]) -> Command {
    let mut command = Command::new(S6);
    command
        .arg("--user")
        .arg("--verbosity=1")
        .arg(format!("--repodir={}", paths.repository_dir.display()))
        .arg(format!("--bootdb={}", paths.boot_database.display()))
        .arg(format!(
            "--storelist={}",
            paths.storelist().expect("validated paths")
        ))
        .arg("--fdholder-user=")
        .args(arguments);
    environment.apply(&mut command);
    command
}

#[derive(Debug)]
struct Environment {
    home: PathBuf,
    config_home: PathBuf,
    data_home: PathBuf,
    state_home: PathBuf,
    cache_home: PathBuf,
    runtime_dir: Option<PathBuf>,
}

impl Environment {
    fn load() -> Result<Self, String> {
        let home = absolute_environment_path("HOME")?
            .ok_or_else(|| "HOME must name an absolute directory".to_string())?;
        if !home.is_dir() {
            return Err(format!("HOME is not a directory: {}", home.display()));
        }

        let config_home = xdg_path("XDG_CONFIG_HOME", &home, ".config")?;
        let data_home = xdg_path("XDG_DATA_HOME", &home, ".local/share")?;
        let state_home = xdg_path("XDG_STATE_HOME", &home, ".local/state")?;
        let cache_home = xdg_path("XDG_CACHE_HOME", &home, ".cache")?;
        let runtime_dir = absolute_environment_path("XDG_RUNTIME_DIR")?;

        for (name, path) in [
            ("HOME", &home),
            ("XDG_CONFIG_HOME", &config_home),
            ("XDG_DATA_HOME", &data_home),
            ("XDG_STATE_HOME", &state_home),
            ("XDG_CACHE_HOME", &cache_home),
        ] {
            validate_line_path(name, path)?;
        }
        if let Some(path) = &runtime_dir {
            validate_line_path("XDG_RUNTIME_DIR", path)?;
        }

        Ok(Self {
            home,
            config_home,
            data_home,
            state_home,
            cache_home,
            runtime_dir,
        })
    }

    fn apply(&self, command: &mut Command) {
        command
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CACHE_HOME", &self.cache_home);
        if let Some(runtime_dir) = &self.runtime_dir {
            command.env("XDG_RUNTIME_DIR", runtime_dir);
        } else {
            command.env_remove("XDG_RUNTIME_DIR");
        }
    }
}

fn absolute_environment_path(name: &str) -> Result<Option<PathBuf>, String> {
    let Some(value) = env::var_os(name) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(format!("{name} must be an absolute path"));
    }
    Ok(Some(path))
}

fn xdg_path(name: &str, home: &Path, fallback: &str) -> Result<PathBuf, String> {
    Ok(absolute_environment_path(name)?.unwrap_or_else(|| home.join(fallback)))
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct PartialPaths {
    repository_dir: Option<PathBuf>,
    boot_database: Option<PathBuf>,
    package_store: Option<PathBuf>,
    administrator_store: Option<PathBuf>,
    user_store: Option<PathBuf>,
}

impl PartialPaths {
    fn merge(&mut self, replacement: Self) {
        if replacement.repository_dir.is_some() {
            self.repository_dir = replacement.repository_dir;
        }
        if replacement.boot_database.is_some() {
            self.boot_database = replacement.boot_database;
        }
        if replacement.package_store.is_some() {
            self.package_store = replacement.package_store;
        }
        if replacement.administrator_store.is_some() {
            self.administrator_store = replacement.administrator_store;
        }
        if replacement.user_store.is_some() {
            self.user_store = replacement.user_store;
        }
    }
}

#[derive(Debug)]
struct Paths {
    repository_dir: PathBuf,
    boot_database: PathBuf,
    package_store: PathBuf,
    administrator_store: PathBuf,
    user_store: PathBuf,
}

impl Paths {
    fn load(environment: &Environment) -> Result<Self, String> {
        let mut configured = PartialPaths::default();
        configured.merge(load_config(Path::new(SYSTEM_CONFIG))?);
        configured.merge(load_config(
            &environment.config_home.join("s6-user/config.toml"),
        )?);
        Self::from_partial(environment, configured)
    }

    fn from_partial(environment: &Environment, configured: PartialPaths) -> Result<Self, String> {
        let paths = Self {
            repository_dir: configured
                .repository_dir
                .unwrap_or_else(|| environment.state_home.join("s6-rc/repository")),
            boot_database: configured
                .boot_database
                .unwrap_or_else(|| environment.config_home.join("s6-rc/compiled/current")),
            package_store: configured
                .package_store
                .unwrap_or_else(|| "/usr/share/s6-rc/user/sources".into()),
            administrator_store: configured
                .administrator_store
                .unwrap_or_else(|| "/etc/s6-rc/user/sources".into()),
            user_store: configured
                .user_store
                .unwrap_or_else(|| environment.config_home.join("s6-rc/sources")),
        };
        paths.validate()?;
        Ok(paths)
    }

    fn validate(&self) -> Result<(), String> {
        for (name, path) in [
            ("repository_dir", &self.repository_dir),
            ("boot_database", &self.boot_database),
            ("package_store", &self.package_store),
            ("administrator_store", &self.administrator_store),
            ("user_store", &self.user_store),
        ] {
            if !path.is_absolute() {
                return Err(format!(
                    "{name} must be an absolute path: {}",
                    path.display()
                ));
            }
            validate_line_path(name, path)?;
        }
        for (name, path) in [
            ("package_store", &self.package_store),
            ("administrator_store", &self.administrator_store),
            ("user_store", &self.user_store),
        ] {
            if os_str_contains(path.as_os_str(), ':') {
                return Err(format!("{name} must not contain ':'"));
            }
        }
        Ok(())
    }

    fn storelist(&self) -> Result<String, String> {
        let stores = [
            &self.package_store,
            &self.administrator_store,
            &self.user_store,
        ];
        stores
            .iter()
            .map(|path| {
                path.to_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("store path is not valid UTF-8: {}", path.display()))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|stores| stores.join(":"))
    }

    fn prepare(&self) -> Result<(), String> {
        // SAFETY: umask has no pointer arguments and affects only this process
        // immediately before directory creation and exec.
        unsafe { libc::umask(0o077) };
        create_directory(&self.user_store)?;
        create_parent(&self.repository_dir)?;
        create_parent(&self.boot_database)?;
        Ok(())
    }
}

fn load_config(path: &Path) -> Result<PartialPaths, String> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PartialPaths::default());
        }
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    toml::from_str(&source).map_err(|error| format!("cannot parse {}: {error}", path.display()))
}

fn create_parent(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("path has no parent: {}", path.display()))?;
    create_directory(parent)
}

fn create_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("cannot create directory {}: {error}", path.display()))
}

#[derive(Debug)]
struct ResolvedRuntime {
    scandir: String,
    livedir: String,
    stmpdir: String,
}

impl ResolvedRuntime {
    fn query(environment: &Environment, paths: &Paths) -> Result<Self, String> {
        let output = frontend_command(
            environment,
            paths,
            &[OsString::from("version"), OsString::from("export")],
        )
        .output()
        .map_err(|error| format!("cannot execute {S6}: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "s6 version export failed with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let output = String::from_utf8(output.stdout)
            .map_err(|_| "s6 version export returned invalid UTF-8".to_string())?;
        let value = |wanted: &str| -> Result<String, String> {
            output
                .lines()
                .filter_map(|line| line.split_once('='))
                .find_map(|(name, value)| (name == wanted).then(|| value.to_owned()))
                .filter(|value| Path::new(value).is_absolute())
                .ok_or_else(|| format!("s6 version export did not report an absolute {wanted}"))
        };
        Ok(Self {
            scandir: value("scandir")?,
            livedir: value("livedir")?,
            stmpdir: value("stmpdir")?,
        })
    }
}

fn validate_line_path(name: &str, path: &Path) -> Result<(), String> {
    let value = path
        .to_str()
        .ok_or_else(|| format!("{name} is not valid UTF-8: {}", path.display()))?;
    if value.contains(['\n', '\r']) {
        return Err(format!("{name} must not contain a line break"));
    }
    Ok(())
}

fn os_str_contains(value: &OsStr, character: char) -> bool {
    value.to_string_lossy().contains(character)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment(root: &Path) -> Environment {
        Environment {
            home: root.join("home"),
            config_home: root.join("config"),
            data_home: root.join("data"),
            state_home: root.join("state"),
            cache_home: root.join("cache"),
            runtime_dir: Some(root.join("runtime")),
        }
    }

    #[test]
    fn defaults_only_define_persistent_paths() {
        let root = Path::new("/tmp/s6-user-test");
        let paths = Paths::from_partial(&environment(root), PartialPaths::default()).unwrap();
        assert_eq!(paths.repository_dir, root.join("state/s6-rc/repository"));
        assert_eq!(
            paths.boot_database,
            root.join("config/s6-rc/compiled/current")
        );
        assert_eq!(paths.user_store, root.join("config/s6-rc/sources"));
        assert_eq!(
            paths.storelist().unwrap(),
            format!(
                "/usr/share/s6-rc/user/sources:/etc/s6-rc/user/sources:{}",
                root.join("config/s6-rc/sources").display()
            )
        );
    }

    #[test]
    fn later_configuration_replaces_individual_fields() {
        let mut base = PartialPaths {
            repository_dir: Some("/system/repository".into()),
            package_store: Some("/system/store".into()),
            ..PartialPaths::default()
        };
        base.merge(PartialPaths {
            repository_dir: Some("/user/repository".into()),
            ..PartialPaths::default()
        });
        let paths = Paths::from_partial(&environment(Path::new("/tmp/test")), base).unwrap();
        assert_eq!(paths.repository_dir, Path::new("/user/repository"));
        assert_eq!(paths.package_store, Path::new("/system/store"));
    }

    #[test]
    fn rejects_relative_paths_and_colons_in_stores() {
        let relative = PartialPaths {
            repository_dir: Some("relative".into()),
            ..PartialPaths::default()
        };
        assert!(Paths::from_partial(&environment(Path::new("/tmp/test")), relative).is_err());

        let colon = PartialPaths {
            user_store: Some("/tmp/invalid:store".into()),
            ..PartialPaths::default()
        };
        assert!(Paths::from_partial(&environment(Path::new("/tmp/test")), colon).is_err());
    }

    #[test]
    fn rejects_unknown_configuration_fields() {
        assert!(toml::from_str::<PartialPaths>("runtime_dir = '/tmp/run'").is_err());
    }

    #[test]
    fn frontend_receives_only_persistent_path_overrides() {
        let root = Path::new("/tmp/s6-user-test");
        let environment = environment(root);
        let paths = Paths::from_partial(&environment, PartialPaths::default()).unwrap();
        let command = frontend_command(&environment, &paths, &[]);
        let arguments: Vec<_> = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect();
        assert!(arguments.iter().any(|argument| argument == "--user"));
        assert!(
            arguments
                .iter()
                .any(|argument| argument.starts_with("--repodir="))
        );
        assert!(
            arguments
                .iter()
                .any(|argument| argument.starts_with("--bootdb="))
        );
        assert!(
            arguments
                .iter()
                .any(|argument| argument.starts_with("--storelist="))
        );
        assert!(!arguments.iter().any(|argument| {
            argument.starts_with("--scandir=")
                || argument.starts_with("--livedir=")
                || argument.starts_with("--stmpdir=")
        }));
    }
}
