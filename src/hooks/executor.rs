use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow};

use crate::config::Config;
use crate::config::types::{CommandHook, CopyHook, Hook};

pub struct HookExecutor<'a> {
    config: &'a Config,
    repo_root: &'a Path,
}

impl<'a> HookExecutor<'a> {
    pub fn new(config: &'a Config, repo_root: &'a Path) -> Self {
        Self { config, repo_root }
    }

    pub fn execute_post_create_hooks<W: Write>(
        &self,
        writer: &mut W,
        worktree_path: &Path,
    ) -> Result<()> {
        let hooks = &self.config.hooks.post_create;
        if hooks.is_empty() {
            return Ok(());
        }

        writeln!(writer, "\nExecuting post-create hooks...")?;

        for (index, hook) in hooks.iter().enumerate() {
            writeln!(
                writer,
                "\n→ Running hook {} of {}...",
                index + 1,
                hooks.len()
            )?;

            match hook {
                Hook::Copy(copy_hook) => {
                    self.execute_copy_hook(writer, copy_hook, worktree_path)?
                }
                Hook::Command(command_hook) => {
                    self.execute_command_hook(writer, command_hook, worktree_path)?
                }
            }

            writeln!(writer, "✓ Hook {} completed", index + 1)?;
        }

        writeln!(writer, "✓ All hooks executed successfully")?;
        Ok(())
    }

    fn execute_copy_hook<W: Write>(
        &self,
        writer: &mut W,
        hook: &CopyHook,
        worktree_path: &Path,
    ) -> Result<()> {
        let src_path = self.resolve_repo_path(&hook.from);
        let dst_path = self.resolve_worktree_path(worktree_path, &hook.to);

        let metadata = fs::metadata(&src_path).with_context(|| {
            format!(
                "source path for copy hook does not exist: {}",
                src_path.display()
            )
        })?;

        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create destination directory {}",
                    parent.display()
                )
            })?;
        }

        writeln!(
            writer,
            "  Copying: {} → {}",
            self.relative_to_repo(&src_path),
            self.relative_to_worktree(worktree_path, &dst_path)
        )?;

        if metadata.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)
        } else {
            fs::copy(&src_path, &dst_path)
                .map(|_| ())
                .with_context(|| format!("failed to copy file to {}", dst_path.display()))
        }
    }

    fn execute_command_hook<W: Write>(
        &self,
        writer: &mut W,
        hook: &CommandHook,
        worktree_path: &Path,
    ) -> Result<()> {
        writeln!(writer, "  Running: {}", hook.command)?;

        let mut command = if cfg!(windows) {
            let mut cmd = Command::new("cmd");
            cmd.arg("/C").arg(&hook.command);
            cmd
        } else {
            let mut cmd = Command::new("sh");
            cmd.arg("-c").arg(&hook.command);
            cmd
        };

        let working_dir = hook
            .work_dir
            .as_ref()
            .map(|path| self.resolve_worktree_path(worktree_path, path))
            .unwrap_or_else(|| worktree_path.to_path_buf());
        command.current_dir(&working_dir);

        command.env_remove("WTP_SHELL_INTEGRATION");
        for (key, value) in &hook.env {
            command.env(key, value);
        }
        command.env(
            "GIT_WTP_WORKTREE_PATH",
            worktree_path.to_string_lossy().to_string(),
        );
        command.env(
            "GIT_WTP_REPO_ROOT",
            self.repo_root.to_string_lossy().to_string(),
        );

        let output = command
            .output()
            .with_context(|| format!("failed to execute hook command '{}'", hook.command))?;

        if !output.stdout.is_empty() {
            writer.write_all(&output.stdout)?;
        }
        if !output.stderr.is_empty() {
            writer.write_all(&output.stderr)?;
        }

        if !output.status.success() {
            let status_desc = output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| output.status.to_string());
            return Err(anyhow!("command exited with status {}", status_desc));
        }

        Ok(())
    }

    fn resolve_repo_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.repo_root.join(path)
        }
    }

    fn resolve_worktree_path(&self, worktree_path: &Path, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            worktree_path.join(path)
        }
    }

    fn relative_to_repo(&self, path: &Path) -> String {
        path.strip_prefix(self.repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    }

    fn relative_to_worktree(&self, worktree_path: &Path, path: &Path) -> String {
        path.strip_prefix(worktree_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)
        .with_context(|| format!("failed to create destination directory {}", dst.display()))?;

    for entry in fs::read_dir(src).with_context(|| format!("failed to read {}", src.display()))? {
        let entry = entry?;
        let entry_src = entry.path();
        let entry_dst = dst.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir_recursive(&entry_src, &entry_dst)?;
        } else {
            if let Some(parent) = entry_dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&entry_src, &entry_dst)
                .with_context(|| format!("failed to copy file to {}", entry_dst.display()))?;
        }
    }

    Ok(())
}
