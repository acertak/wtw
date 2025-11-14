use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::shell::pwsh;

/// 既定の PowerShell プロファイルパス（ユーザー／ホスト単位）を推定する
///
/// 現状は PowerShell 7 以降の既定値に合わせて:
///   %USERPROFILE%\Documents\PowerShell\Microsoft.PowerShell_profile.ps1
pub fn default_pwsh_profile() -> Result<PathBuf> {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .context("failed to determine user home directory for PowerShell profile")?;

    Ok(PathBuf::from(home)
        .join("Documents")
        .join("PowerShell")
        .join("Microsoft.PowerShell_profile.ps1"))
}

/// PowerShell プロファイルに wtw 用シェル統合スクリプトを追記する
pub fn init_pwsh(profile_path: &Path) -> Result<()> {
    let profile_display = profile_path.display().to_string();

    if let Some(parent) = profile_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create profile directory: {}",
                parent.display()
            )
        })?;
    }

    let existing = fs::read_to_string(profile_path).unwrap_or_default();

    // すでに設定済みなら何もしない（冪等）
    if existing.contains("# wtw shell integration") {
        return Ok(());
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(profile_path)
        .with_context(|| format!("failed to open profile file: {}", profile_display))?;

    if !existing.is_empty() && !existing.ends_with('\n') {
        writeln!(file)?;
    }

    writeln!(file, "# wtw shell integration")?;
    writeln!(file, "{}", pwsh::script())?;

    Ok(())
}


