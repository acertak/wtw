wtw (Worktree Pro for Windows)
==============================

Windows‑native worktree helper compatible with **wtp** (Worktree Plus), written in Rust. “wtw” stands for **Worktree Pro for Windows**.

Git worktree を Windows で快適に扱うための CLI ツールです。`wtp` の `.wtp.yml` 設定と挙動にできるだけ追従しつつ、Windows 11 / PowerShell 前提で使いやすくすることを目指しています。  
日本語の README は `README.ja.md` を参照してください。

This project is **based on `wtp` version 2.3.4**.  
Going forward, we do **not** aim for strict compatibility with upstream `wtp`; instead, this repository will evolve with its own extensions and design choices.  
Because of that, **all files in this repository should be treated as potentially modified from the original `wtp` sources.**

> Status: early 0.1.x. The CLI is already useful for daily work, but full wtp compatibility is still in progress. See `docs/wtp-wtw-feature-map.md` for the latest status.


Features
--------

- **Windows‑first worktree helper**
  - Uses `git.exe` under the hood.
  - Supports Windows‑style paths and drive letters.
- **Almost drop‑in compatible with wtp**
  - Reads the same `.wtp.yml` format (version, `defaults.base_dir`, `hooks.post_create`, …).
  - `add`, `list`, `remove`, `cd` behave very close to wtp.
- **Automatic worktree layout**
  - Branch names like `feature/auth` are mapped to `../worktree/feature/auth` by default.
  - Windows‑forbidden characters in branch names are sanitized (e.g. `feat:bad*name` → `feat_bad_name`).
- **Post‑create hooks**
  - `copy` hooks to copy files (even gitignored ones like `.env`) from the main worktree.
  - `command` hooks to run bootstrap commands (install deps, run migrations, etc.).
- **Rich `list` output with JSON**
  - Human‑friendly table with `PATH`, `BRANCH`, `HEAD`, `STATUS`, `UPSTREAM`, `ABS_PATH`.
  - `wtw list --json` for tooling and PowerShell completion.
- **PowerShell integration**
  - `wtw init` appends a small function to your PowerShell profile so that `wtw cd` actually changes the current directory.
  - Tab completion for subcommands and `wtw cd` worktree names.


Requirements
------------

- **OS**: Windows 11 (other modern Windows versions may work, but are not officially tested).
- **Git**: Git for Windows (with `git.exe` on `PATH`).
- **Shell**:
  - PowerShell 7+ (recommended and currently the only shell with first‑class integration).
  - Cmd / Git Bash are planned, but `shell-init` for them is not implemented yet.
- **Rust toolchain** (only if you build from source):
  - Rust stable
  - `cargo`


Installation
------------

### Download prebuilt binary (recommended for most users)

Once you publish a release, the typical distribution looks like:

- `wtw-<version>-x86_64-pc-windows-msvc.zip`

Each archive should contain:

- `wtw.exe`
- `README.md` (this file)
- `LICENSE`

Install steps:

```powershell
# 1. Download the ZIP from this repository's “Releases” page
# 2. Extract it somewhere, for example:
Expand-Archive -Path .\wtw-0.2.0-x86_64-pc-windows-msvc.zip -DestinationPath C:\tools\wtw

# 3. Add that directory to your PATH (once)
[System.Environment]::SetEnvironmentVariable(
  "Path",
  $env:Path + ";C:\tools\wtw",
  "User"
)

# 4. Open a new PowerShell and verify
wtw --help
```

> NOTE: The exact archive name and destination path are just examples. Adjust them according to your release/tag naming.


### Build and install from source

Clone this repository and build inside the `wtw` crate:

```powershell
git clone <this repository>
cd wtw

# Build a release binary
cargo build --release

# Option 1: use the built binary directly
.\target\release\wtw.exe --help

# Option 2: install to ~/.cargo/bin
cargo install --path .
wtw --help
```


Quick Start
-----------

### 1. Prepare a Git repository

Inside a Git repository (or with `--repo` pointing to one), `wtw` auto‑detects the repo root:

```powershell
# In your existing Git repo
cd C:\src\my-project
wtw list --json

# Or from outside the repo
wtw --repo C:\src\my-project list --json
```


### 2. Enable PowerShell integration (optional but recommended)

If `wtw.exe` is on `PATH`, you can add the `wtw` function and completion to your PowerShell profile with a single command:

```powershell
# Use the default PowerShell profile
wtw init

# Or specify the shell/profile explicitly
wtw init --shell pwsh
wtw init --shell pwsh C:\Users\<you>\Documents\PowerShell\Microsoft.PowerShell_profile.ps1
```

What this does:

- Creates the profile directory/file if needed.
- Appends a section starting with `# wtw shell integration`.
- Defines a `wtw` function that:
  - Calls the real `wtw.exe`.
  - If the first argument is `cd` and the command succeeds, changes the current directory to the printed path.
- Registers a PowerShell `ArgumentCompleter`:
  - Completes subcommands (`add`, `list`, `remove`, `cd`, `shell-init`).
  - For `wtw cd`, fetches worktree names via `wtw list --json` and completes them.

After running `wtw init`, open a **new** PowerShell session and try:

```powershell
wtw cd @
wtw cd <TAB>  # completes worktree names
```

If you prefer to manage your profile manually, you can also emit the script and inspect it:

```powershell
wtw shell-init pwsh > wtw.ps1
# then dot-source it or copy-paste into your profile
```


Basic Usage
-----------

### Create a worktree (`add`)

```powershell
# Create a worktree from an existing local or remote branch
wtw add feature/auth

# Create a new branch and worktree
wtw add -b feature/new-feature

# Create a new branch tracking a specific remote branch
wtw add --track origin/feature/remote-only

# Use a specific commit as the base (branch name via -b)
wtw add -b hotfix/urgent abc1234
```

- By default, worktrees are placed under `../worktree` relative to the repo root.
- Branch names with `/` become nested directories (e.g. `feature/auth` → `../worktree/feature/auth`).


### List worktrees (`list`)

```powershell
# Human-friendly table
wtw list

# Example output:
# PATH                      BRANCH           HEAD     STATUS  UPSTREAM       ABS_PATH
# ----                      ------           ----     ------  --------       --------
# @*                        main             c72c7800 clean   origin/main    C:\src\my-project
# feature/auth              feature/auth     def45678 dirty   origin/feature/auth C:\src\my-project\..\worktree\feature\auth

# JSON for tooling or completion
wtw list --json
```

The JSON output roughly looks like this:

```json
[
  {
    "name": "@",
    "branch": "main",
    "head": "c72c7800",
    "status": "clean",
    "upstream": "origin/main",
    "path": "@",
    "abs_path": "C:\\src\\my-project",
    "is_main": true,
    "is_current": true
  }
]
```


### Remove a worktree (`remove`)

```powershell
# Remove a worktree (by display name/branch/directory)
wtw remove feature/auth

# Force removal even if the worktree is dirty
wtw remove --force feature/auth

# Remove worktree and its branch (only if merged)
wtw remove --with-branch feature/auth

# Remove worktree and force-delete the branch
wtw remove --with-branch --force-branch feature/auth
```

Only worktrees managed under `base_dir` are removed; others are left untouched.
You cannot remove the **current** worktree (an error is returned instead).


### Navigate between worktrees (`cd`)

With PowerShell integration enabled (`wtw init`), you can jump between worktrees:

```powershell
# Change to a worktree by its name or branch
wtw cd feature/auth

# Change back to the main worktree
wtw cd @
wtw cd my-project   # repo name also works
```

If `wtw` cannot find the requested worktree, it prints a helpful error with a list of available names and suggests running `wtw list`.


Configuration: .wtp.yml
-----------------------

`wtw` reads `.wtp.yml` at the repository root and is designed to be compatible with wtp’s configuration format.

### Base directory

```yaml
version: "1.0"
defaults:
  # Base directory for worktrees (relative to repo root, or absolute)
  base_dir: "../worktree"
```

- Relative `base_dir` is resolved from the Git repo root.
- Absolute paths are also supported, even on different drives.


### Hooks

```yaml
version: "1.0"
defaults:
  base_dir: "../worktree"

hooks:
  post_create:
    # Copy gitignored files from main worktree to the new worktree
    - type: copy
      from: ".env"     # relative to the main worktree
      to: ".env"       # relative to the new worktree

    # Run setup commands in the new worktree
    - type: command
      command: "npm ci"
      env:
        NODE_ENV: "development"

    - type: command
      command: "npm run db:setup"
      work_dir: "."
```

Behavior:

- `from` paths are always resolved relative to the **main** worktree.
- `to` paths are resolved relative to the newly created worktree.
- `command` hooks run inside the new worktree, with optional `env` and `work_dir`.
- If any hook fails, the whole `wtw add` command fails.

> **Security note**: `command` hooks execute arbitrary commands defined in `.wtp.yml`.  
> Only enable and run hooks for repositories you trust, and review the hook definitions before using `wtw add`.


Exit Codes
----------

`wtw` uses structured exit codes to distinguish error types:

- `0`: success
- `1`: user errors (invalid arguments, unknown worktree, etc.)
- `2`: configuration errors (invalid `.wtp.yml`)
- `3`: Git command failures
- `10`: unexpected internal errors


Compatibility with wtp
----------------------

While `wtw` starts from `wtp` 2.3.4 and keeps **a good level of compatibility** with that version,  
the long‑term direction is to allow `wtw` to grow its own features and behavior:

- `.wtp.yml` configuration is shared.
- Worktree layout, naming, and most of the `add/list/remove/cd` behavior match closely.
- PowerShell shell integration (`wtw init` / `wtw shell-init pwsh`) mirrors the wtp experience on macOS/Linux.

However, there are still known gaps:

- `shell-init` for `cmd` / `bash` is not implemented yet.
- Some detailed “helpful error” messages and remote branch resolution logic are less sophisticated than wtp.
- Additional flags specific to wtp (e.g. `list --quiet` / `--compact`) are not currently exposed.

For a detailed, up‑to‑date mapping, see:

- `docs/spec.md`
- `docs/wtp-wtw-feature-map.md`


License
-------

WTW は、MIT License のもとで公開されている [satococoa/wtp](https://github.com/satococoa/wtp) をベースにしたプロジェクトです。
このリポジトリ自体も MIT License で配布されており、詳細な条文は同梱の `LICENSE` を参照してください。  
上流プロジェクト wtp のライセンスについては、wtp リポジトリに含まれる `LICENSE` を参照してください。



