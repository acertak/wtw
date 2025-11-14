use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "wtw",
    version,
    about = "Windows-native worktree helper compatible with wtp"
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOptions,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Debug, Clone)]
pub struct GlobalOptions {
    /// 詳細ログ（stderr に出力）
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count, conflicts_with = "quiet")]
    pub verbose: u8,
    /// 標準出力を最小限に（エラーのみ）
    #[arg(long = "quiet", action = ArgAction::SetTrue)]
    pub quiet: bool,
    /// 任意のディレクトリを Git リポジトリ root として扱う
    #[arg(long = "repo", value_name = "PATH")]
    pub repo: Option<PathBuf>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// worktree を追加
    Add(AddCommand),
    /// 登録済み worktree を一覧表示
    List(ListCommand),
    /// worktree を削除
    Remove(RemoveCommand),
    /// 指定 worktree の絶対パスを出力
    Cd(CdCommand),
    /// シェル統合をプロファイルにインストール
    Init(InitCommand),
    /// シェル初期化スクリプトを出力
    #[command(name = "shell-init")]
    ShellInit(ShellInitCommand),
}

#[derive(Args, Debug, Clone)]
pub struct AddCommand {
    /// ブランチまたはコミット
    #[arg(value_name = "BRANCH_OR_COMMIT")]
    pub target: Option<String>,
    /// 新規ブランチ名
    #[arg(short = 'b', long = "branch", value_name = "BRANCH")]
    pub branch: Option<String>,
    /// 追跡する remote/branch
    #[arg(long = "track", value_name = "REMOTE/BRANCH")]
    pub track: Option<String>,
}

#[derive(Args, Debug, Clone, Copy)]
pub struct ListCommand {
    /// JSON 形式で出力
    #[arg(long = "json")]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RemoveCommand {
    /// 削除対象の worktree
    #[arg(value_name = "WORKTREE")]
    pub target: Option<String>,
    /// 強制削除
    #[arg(short = 'f', long = "force")]
    pub force: bool,
    /// 対応ブランチも削除
    #[arg(long = "with-branch")]
    pub with_branch: bool,
    /// ブランチが別の worktree にチェックアウトされていても削除
    #[arg(long = "force-branch")]
    pub force_branch: bool,
}

#[derive(Args, Debug, Clone)]
pub struct CdCommand {
    /// 対象 worktree
    #[arg(value_name = "WORKTREE")]
    pub target: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct ShellInitCommand {
    /// シェル種別（pwsh/cmd/bash）
    #[arg(value_enum)]
    pub shell: ShellKind,
}

#[derive(Args, Debug, Clone)]
pub struct InitCommand {
    /// シェル種別（省略時は pwsh）
    #[arg(value_enum, default_value_t = ShellKind::Pwsh)]
    pub shell: ShellKind,
    /// シェルのプロファイルファイルパス（例: $PROFILE）。省略時は既定の PowerShell プロファイルを使用
    #[arg(value_name = "PROFILE_PATH")]
    pub profile: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum ShellKind {
    Pwsh,
    Cmd,
    Bash,
}

impl ShellKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ShellKind::Pwsh => "pwsh",
            ShellKind::Cmd => "cmd",
            ShellKind::Bash => "bash",
        }
    }
}
