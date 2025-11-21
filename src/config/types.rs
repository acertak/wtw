use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub(crate) const DEFAULT_VERSION: &str = "1.0";
pub(crate) const DEFAULT_BASE_DIR: &str = "../worktree";

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub hooks: Hooks,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: default_version(),
            defaults: Defaults::default(),
            hooks: Hooks::default(),
        }
    }
}

impl Config {
    pub fn resolved_base_dir(&self, repo_root: &Path) -> PathBuf {
        self.defaults.resolve_base_dir(repo_root)
    }
}

fn default_version() -> String {
    DEFAULT_VERSION.to_owned()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Defaults {
    #[serde(default = "default_base_dir")]
    pub base_dir: PathBuf,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            base_dir: default_base_dir(),
        }
    }
}

impl Defaults {
    pub fn resolve_base_dir(&self, repo_root: &Path) -> PathBuf {
        if self.base_dir.is_absolute() {
            normalize_fs_path(&self.base_dir)
        } else {
            normalize_fs_path(repo_root).join(&self.base_dir)
        }
    }
}

fn default_base_dir() -> PathBuf {
    PathBuf::from(DEFAULT_BASE_DIR)
}

fn normalize_fs_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        const PREFIX: &str = r"\\?\";
        let display = path.to_string_lossy();
        if let Some(stripped) = display.strip_prefix(PREFIX) {
            PathBuf::from(stripped)
        } else {
            PathBuf::from(display.as_ref())
        }
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Hooks {
    #[serde(default)]
    pub post_create: Vec<Hook>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Hook {
    Copy(CopyHook),
    Command(CommandHook),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopyHook {
    pub from: PathBuf,
    pub to: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandHook {
    pub command: String,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default, rename = "work_dir")]
    pub work_dir: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn resolved_base_dir_uses_repo_root_for_relative_paths() {
        let repo = TempDir::new().expect("temp repo");
        let mut config = Config::default();
        config.defaults.base_dir = PathBuf::from("custom-worktrees");

        let resolved = config.resolved_base_dir(repo.path());
        let expected = normalize_for_assert(fs::canonicalize(repo.path()).expect("canonical root"))
            .join("custom-worktrees");

        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolved_base_dir_preserves_absolute_paths() {
        let repo = TempDir::new().expect("temp repo");
        let absolute = TempDir::new().expect("absolute base");
        let mut config = Config::default();
        config.defaults.base_dir = absolute.path().to_path_buf();

        let resolved = config.resolved_base_dir(repo.path());
        let expected = normalize_for_assert(fs::canonicalize(absolute.path()).expect("canonical abs"));

        assert_eq!(resolved, expected);
    }

    #[test]
    fn default_config_uses_expected_version_and_base_dir() {
        let repo = TempDir::new().expect("temp repo");
        let config = Config::default();
        assert_eq!(config.version, DEFAULT_VERSION);

        let resolved = config.resolved_base_dir(repo.path());
        let expected = normalize_for_assert(fs::canonicalize(repo.path()).expect("canonical root"))
            .join(DEFAULT_BASE_DIR);

        assert_eq!(resolved, expected);
    }

    fn normalize_for_assert(path: PathBuf) -> PathBuf {
        #[cfg(windows)]
        {
            let display = path.to_string_lossy();
            if let Some(stripped) = display.strip_prefix(r"\\?\") {
                PathBuf::from(stripped)
            } else {
                PathBuf::from(display.as_ref())
            }
        }
        #[cfg(not(windows))]
        {
            path
        }
    }
}
