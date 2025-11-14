WTW Specification
=================

1. Overview
-----------

WTW (Worktree Pro for Windows) is a Windows‑native helper CLI for managing Git
worktrees. It is implemented in Rust and is designed to be highly compatible
with the original **wtp (Worktree Plus)** while providing a first‑class
experience on Windows 11 and PowerShell.

This document specifies:

- The command‑line interface (global options and subcommands).
- The configuration file format (`.wtp.yml`) as interpreted by WTW.
- How WTW integrates with Git and `git worktree`.
- The post‑create hook mechanism.
- PowerShell shell integration.
- Logging behavior and exit codes.
- Behavioral guarantees captured by the automated test suite.

All information in this document is derived directly from the source code and
tests in this repository. No speculative or planned behavior is described.


2. Terminology and Concepts
---------------------------

- **Main worktree**  
  The primary worktree of a Git repository (the one corresponding to the
  `.git` directory). In `git worktree list --porcelain`, it is the first
  `worktree` entry. WTW marks this entry as `is_main = true`.

- **Additional worktree**  
  Any worktree managed by `git worktree` other than the main worktree.

- **Base directory (`base_dir`)**  
  The root directory under which WTW manages worktrees by default. It is
  configured via `.wtp.yml` and defaults to `../worktrees` relative to the
  main repository root.

- **Managed worktree**  
  A worktree whose path is under the configured `base_dir` (or the main
  worktree itself). Functions such as `list`, `remove`, and `cd` treat these
  specially.

- **Display name**  
  The human‑friendly name shown in `wtw list` and used in places where a
  compact identifier is useful. For the main worktree it is `"@"`. For other
  worktrees under `base_dir`, the display name is the relative path from
  `base_dir` to the worktree directory (joined using the platform’s path
  separator). If that cannot be determined, the final path component or the
  full path string is used as a fallback.

- **Worktree name (for `cd` and `remove`)**  
  A user‑supplied token that can match:

  - `"@"` (main worktree).
  - `"root"` (case‑insensitive alias for the main worktree).
  - The repository name (the last path component of the main root), case‑insensitive.
  - A branch name (e.g. `feature/auth`).
  - A display name (e.g. `feature\auth` on Windows).
  - The worktree directory name (final path component).


3. Architecture Overview
------------------------

The crate is structured into modules corresponding to major responsibilities:

- `main`  
  OS entrypoint. Calls `wtw::run()` and maps errors to exit codes.

- `lib`  
  Exposes the `run()` function, parses CLI options via Clap, initializes
  logging, and dispatches to subcommand implementations.

- `cli`  
  Definitions of global options and subcommands using `clap::Parser` and
  `clap::Subcommand`. This is the single source of truth for the CLI surface.

- `config`  
  Loading and representation of `.wtp.yml`:

  - `config::loader`: file discovery and YAML parsing, error reporting.
  - `config::types`: strongly typed configuration (`Config`, `Defaults`,
    `Hooks`, `Hook`) and effective path resolution (`resolved_base_dir`).

- `git`  
  Integration with Git:

  - `git::rev`: `RepoContext` that discovers the main and worktree roots.
  - `git::runner`: `GitRunner` wrapper around `git.exe` with logging and
    error types.
  - `git::worktree`: parsing of `git worktree list --porcelain` output into
    structured `WorktreeInfo` values.

- `worktree`  
  Implementation of subcommands operating on worktrees:

  - `worktree::add`: `wtw add` behavior (worktree creation, path mapping,
    conflict detection, post‑create hooks).
  - `worktree::list`: `wtw list` behavior (table and JSON output).
  - `worktree::remove`: `wtw remove` behavior (worktree and optional branch
    removal).
  - `worktree::resolve`: `wtw cd` behavior (name resolution).
  - `worktree::common`: cross‑cutting helpers for path normalization,
    display names, and “managed” checks.

- `hooks`  
  The post‑create hooks executor (`HookExecutor`) which runs `copy` and
  `command` hooks defined in `.wtp.yml`.

- `shell`  
  Shell integration:

  - `shell::init`: initialization of the PowerShell profile (`wtw init`).
  - `shell::pwsh`: PowerShell function and argument completer script.
  - `shell::cmd`, `shell::bash`: placeholders (currently unused).

- `logging`  
  Initialization of the `tracing` subscriber based on global verbosity flags.

- `error`  
  Core application error type `AppError` with categories and exit code
  mapping.

- `tests`  
  Integration tests (`tests/*.rs`) which invoke the compiled binary and verify
  CLI behavior. These are treated as executable specification for critical
  flows (add/list/remove/cd/config/shell‑init).


4. CLI Specification
--------------------

4.1 Global Options
~~~~~~~~~~~~~~~~~~

The top‑level CLI is defined in `cli::Cli` and parsed via `clap::Parser`.

Global options (available before any subcommand):

- `-v`, `--verbose` (counting flag)  
  Increases log verbosity. Each occurrence increments an internal counter:

  - `0` (default): log level `WARN`.
  - `1`: log level `DEBUG`.
  - `>= 2`: log level `TRACE`.

- `--quiet`  
  Suppresses most diagnostic output and sets the log level to `ERROR`. This
  conflicts with `--verbose`.

- `--repo <PATH>`  
  Treats the given path as the starting directory for discovering the Git
  worktree root. If the path is relative, it is resolved against the current
  working directory. If the path points to a file, its parent directory is
  used. If the resulting directory does not exist, an error is returned.

  The `--repo` flag is validated in integration tests by running `wtw` from
  outside the repository and checking that `list --json` still succeeds.


4.2 Subcommands
~~~~~~~~~~~~~~~

The `Command` enum defines the available subcommands:

- `add` (`AddCommand`)
- `list` (`ListCommand`)
- `remove` (`RemoveCommand`)
- `cd` (`CdCommand`)
- `init` (`InitCommand`)
- `shell-init` (`ShellInitCommand`)

Each subcommand is documented below.


4.2.1 `wtw add`
^^^^^^^^^^^^^^^

**Purpose**  
Create a new Git worktree under the configured base directory, optionally
creating or tracking a branch, and then run post‑create hooks.

**Synopsis**

```text
wtw add [OPTIONS] [BRANCH_OR_COMMIT]
```

**Options (AddCommand)**

- `BRANCH_OR_COMMIT` (positional, optional)  
  - If `--branch` is specified, this is treated as the starting point
    (commitish) for the new branch and worktree.  
  - Otherwise, this is required and is used directly as the commitish for the
    worktree.

- `-b, --branch <BRANCH>`  
  Name of the new branch to create for the worktree. When provided:

  - The new worktree is created at the path derived from the branch name.
  - The positional `BRANCH_OR_COMMIT` argument, if present, is used as the
    starting commit.

- `--track <REMOTE/BRANCH>`  
  Remote tracking branch to use when creating the worktree. This value is
  passed as the commitish to `git worktree add`. The local branch name is
  inferred from the remote/branch string unless `--branch` is explicitly
  supplied.

**Argument validation**

Behavior is determined from the combination of `--branch`, `--track`, and
`BRANCH_OR_COMMIT`:

1. If `--track` is supplied:

   - The branch name is either:
     - Provided explicitly via `--branch`, or
     - Inferred from the part after the first `/` in `REMOTE/BRANCH`
       (e.g. `origin/feature/auth` → `feature/auth`).
   - If no branch name can be determined, the command fails with a user error:
     `"--track requires a branch name (use --branch or specify remote/branch)"`.

2. Else if `--branch` is supplied:

   - The branch name is taken from `--branch`.
   - The commitish for `git worktree add` is taken from `BRANCH_OR_COMMIT`
     if present; otherwise it is `None`, and Git will use its own defaults
     for branch creation from the worktree root.

3. Else (no `--track` and no `--branch`):

   - `BRANCH_OR_COMMIT` is required. If it is missing or blank, the command
     fails with a user error: `"branch or commit is required"`.
   - The commitish is set to the provided value.
   - No new branch is created by WTW; `git worktree add` is invoked with only
     the path and commitish.

These error messages and exit code 1 (user error) are verified in tests.

**Worktree path derivation**

The effective base directory is `config.resolved_base_dir(main_root)`, where
`main_root` is the main repository root discovered by `RepoContext`. This
uses:

- The configured `defaults.base_dir` when present.
- The default `../worktrees` when `defaults.base_dir` is absent.
- Relative `base_dir` resolved against `main_root`; absolute `base_dir`
  left as‑is.

Within the base directory, WTW derives a relative path from the branch or
commit identifier:

- The primary identifier is the branch name if known; otherwise the commitish.
- The identifier is split on `/` and `\`.
- Each segment is sanitized:
  - Empty segments, `"."`, or `".."` become `"_"`.
  - Windows‑forbidden characters `<`, `>`, `:`, `"`, `|`, `?`, `*`, and `\`
    are replaced with `_`.
- Non‑empty sanitized segments are joined as path components.
- If sanitization results in an empty relative path, a fallback is used by
  sanitizing the entire identifier as a single segment.

The final worktree path is `base_dir.join(relative_path)`.

**Conflict detection**

Before creating the worktree, WTW checks for conflicts using:

1. Existing worktrees from `git worktree list --porcelain`.
2. The filesystem.

Checks:

- If a branch name is known and any existing `WorktreeInfo` has the same
  branch, the command fails with a user error:

  ```text
  worktree for branch '<branch>' already exists: <existing_path>
  ```

- If any existing worktree’s path (normalized) matches the target path
  (normalized), the command fails with a user error:

  ```text
  worktree path already exists in git metadata: <path>
  ```

- If the target path already exists in the filesystem, the command fails with:

  ```text
  destination path already exists: <path>
  ```

- If the derived worktree name resolves to an empty path, the command fails
  with:

  ```text
  worktree name resolves to an empty path: <identifier>
  ```

**Git invocation**

WTW constructs arguments to `git worktree add` as follows:

- Always: `["worktree", "add"]`.
- If tracking: appends `"--track"`.
- If a branch name is set: appends `"-b"` and the branch name.
- Appends the worktree path.
- If a commitish is set: appends the commitish.

The command is executed via `GitRunner::run`. If Git exits with a non‑success
status, WTW:

- Extracts `stderr`, trims it, and:
  - If non‑empty, surfaces it as a Git error message.
  - If empty, surfaces `"git worktree add failed without error output"`.

These failures are treated as Git errors and mapped to exit code 3.

**User‑visible output**

On success, `wtw add` prints a single line to standard output:

```text
Created worktree '<display_name>' at <absolute_path>
```

After that, it executes post‑create hooks (see section 6). The hooks executor
prints progress messages and hook‑specific output.

If any hook fails, `wtw add` fails (the error is propagated and printed by
`main`). Hook failures are treated as internal errors and mapped to exit code
10.


4.2.2 `wtw list`
^^^^^^^^^^^^^^^^

**Purpose**  
List worktrees associated with the current repository, either in a
human‑readable table or as JSON suitable for tooling and completion.

**Synopsis**

```text
wtw list [--json]
```

**Options (ListCommand)**

- `--json`  
  Output JSON instead of a formatted table.

**Data collection**

`wtw list` performs the following steps:

1. Calls `git worktree list --porcelain` via `GitRunner`.
2. Parses the output into `WorktreeInfo` entries. Each entry contains:

   - `path` (absolute, canonicalized path).
   - `head` (full commit hash as reported by Git).
   - `branch` (optional branch name; omitted in detached HEAD).
   - `is_main` (the first parsed entry is flagged as main).
   - `is_detached` (true if the `detached` line appears).
   - `locked` (optional reason from a `locked` line).
   - `prunable` (optional reason from a `prunable` line).

3. Determines the effective base directory: `config.resolved_base_dir(main_root)`.
4. Determines the current worktree path from `RepoContext::worktree_root()`.
5. For each worktree, builds a `DisplayRow` with:

   - `name` (display name; section 2).
   - `branch_display`:
     - Branch name if present.
     - `"detached"` otherwise.
   - `branch` (the raw branch name, if any).
   - `head` (shortened commit hash, first 8 characters when longer).
   - `status`: `"clean"` or `"dirty"`, determined by:

     - Running `git status --short` in the worktree.
     - Treating an empty output as `"clean"`, otherwise `"dirty"`.

   - `upstream`: optional upstream reference, determined by:

     - Running `git rev-parse --abbrev-ref --symbolic-full-name @{u}` in the
       worktree.
     - If the command succeeds, using its trimmed stdout (if non‑empty).
     - If the command fails due to a Git command error (e.g. no upstream
       configured), treating it as `None`.

   - `abs_path`: normalized absolute path string.
   - `is_main`: as above.
   - `is_current`: `true` if the worktree path matches the current worktree
     path (after normalization).

These behaviors are validated by the integration tests in `tests/list_spec.rs`.

**Table output (default)**

The table is printed with dynamically sized columns. The headers are:

```text
PATH  BRANCH  HEAD  STATUS  UPSTREAM  ABS_PATH
```

For each row:

- `PATH` contains the display name.
- If the worktree is the current worktree, an asterisk `*` is appended to the
  display name (e.g. `feature\current*`).
- `BRANCH` contains `branch_display`.
- `HEAD` contains the shortened commit hash.
- `STATUS` contains `"clean"` or `"dirty"`.
- `UPSTREAM` contains the upstream string or `"-"` if none.
- `ABS_PATH` contains the normalized absolute path string.

**JSON output**

When `--json` is provided, `wtw list` emits pretty‑printed JSON:

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

Fields:

- `name`: display name (e.g. `"@"`, `"feature\\auth"`).
- `branch`: optional branch name.
- `head`: short commit hash (up to 8 characters).
- `status`: `"clean"` or `"dirty"`.
- `upstream`: optional upstream reference string.
- `path`: same as `name` (logical path).
- `abs_path`: absolute filesystem path.
- `is_main`: whether this is the main worktree.
- `is_current`: whether this is the current worktree.


4.2.3 `wtw remove`
^^^^^^^^^^^^^^^^^^

**Purpose**  
Remove a managed worktree, and optionally remove its corresponding branch.

**Synopsis**

```text
wtw remove [OPTIONS] <WORKTREE>
```

**Options (RemoveCommand)**

- `WORKTREE` (positional, required)  
  Target worktree identifier. Resolution follows the same rules as for
  `wtw cd` (see section 4.2.4), except that the main worktree is never
  removable.

- `-f, --force`  
  Passes `--force` to `git worktree remove`, allowing removal of dirty
  worktrees.

- `--with-branch`  
  After removing the worktree, also remove its local branch if one is
  associated.

- `--force-branch`  
  When used with `--with-branch`, pass `-D` instead of `-d` to `git branch`,
  allowing the branch to be deleted even when not merged. If supplied without
  `--with-branch`, the command fails with a user error:

  ```text
  --force-branch requires --with-branch
  ```

**Target resolution**

`wtw remove`:

1. Enumerates `WorktreeInfo` entries from `git worktree list --porcelain`.
2. Computes the effective `base_dir`.
3. Skips:
   - The main worktree (`is_main == true`).
   - Any worktree that is not “managed” (its path is not under `base_dir`).
4. Attempts to match the target string against each remaining worktree in
   this order:

   - The branch name (`info.branch`).
   - The worktree directory name (final path component).
   - The display name (relative path under `base_dir`).

If a match is found, that worktree is the removal target.  
If no match is found, a user error is returned with a message of the form:

```text
worktree '<target>' not found
Available worktrees: <name1>, <name2>, ...
Run 'wtw list' to see available worktrees.
```

The list of available names includes display names of managed worktrees.

**Current worktree protection**

Before removal, WTW compares:

- The normalized path of the current worktree (from `RepoContext`), and
- The normalized path of the target worktree.

If they are equal, removal is rejected with a user error:

```text
cannot remove the current worktree '<target>': <path>
```

This behavior, and the guarantee that the current worktree remains intact, is
verified in tests.

**Git invocation**

Worktree removal:

- Arguments: `["worktree", "remove", (optional "--force"), <path>]`.
- On success: the worktree directory is removed by Git.
- On failure:

  - If stderr is non‑empty, it is surfaced as a Git error message.
  - If stderr is empty, an error of the form
    `"git worktree remove failed for <path> without error output"` is used.

Branch removal (when `--with-branch` and a branch is available):

- Uses `git branch -d <branch>` or `git branch -D <branch>` depending on
  `--force-branch`.
- On failure:

  - If stderr is non‑empty, it is used directly as the error message.
  - Otherwise, a generic error like `"failed to remove branch '<branch>'"`
    is used.

All such failures are treated as Git errors and mapped to exit code 3.

**User‑visible output**

On successful worktree removal, WTW prints:

```text
Removed worktree '<target>' at <absolute_path>
```

If branch removal is requested and succeeds, it also prints:

```text
Removed branch '<branch>'
```


4.2.4 `wtw cd`
^^^^^^^^^^^^^^

**Purpose**  
Resolve a worktree identifier to an absolute path. In conjunction with the
PowerShell integration, this enables shell‑level directory changes.

**Synopsis**

```text
wtw cd <WORKTREE>
```

**Options (CdCommand)**

- `WORKTREE` (positional, required)  
  Target worktree identifier. If missing or resolves to an empty name, the
  command fails with a user error:

  ```text
  worktree name is required
  ```

**Name sanitization**

The input is first sanitized by:

- Trimming leading/trailing whitespace.
- Removing any trailing `*` (e.g. a copied value from `wtw list` where the
  current worktree is marked with an asterisk).

If the sanitized value is empty, the command fails with the error above.

**Resolution algorithm**

`wtw cd`:

1. Enumerates `WorktreeInfo` entries from `git worktree list --porcelain`.
2. Computes:
   - `base_dir` as `config.resolved_base_dir(main_root)`, and
   - `repo_name` from the main root directory name.

3. For each worktree, in order:

   - If it is the main worktree (`is_main == true`):

     - Matches if the target string is:
       - `"@"`
       - `"root"` (case‑insensitive)
       - Equal to `repo_name` (case‑insensitive)
       - Equal to the main branch name (if any)

     - If matched, returns this worktree path immediately.

   - For non‑main worktrees:

     - Skips any worktree that is not managed under `base_dir`.
     - Matches if the target string equals:
       - The branch name (`info.branch`), or
       - The display name, or
       - The worktree directory name (final path component).

4. If no match is found, a user error is returned with a message of the form:

```text
worktree '<target>' not found
Available worktrees: <names...>
Run 'wtw list' to see available worktrees.
```

The list of available names includes `"@"`, the main branch name (if any),
the repository name, and the display names of managed worktrees.

**Output**

On success, `wtw cd` prints the normalized absolute path of the resolved
worktree to standard output, followed by a newline. No other output is
emitted in the success path.

Integration tests assert that:

- `wtw cd @` resolves to the repository root.
- `wtw cd <display_name>` resolves correctly after `wtw add`.
- Errors for unknown worktrees include both the “Available worktrees” list
  and the `Run 'wtw list'` hint.


4.2.5 `wtw init`
^^^^^^^^^^^^^^^^

**Purpose**  
Install shell integration into a PowerShell profile by appending a function
and argument completer for `wtw`.

**Synopsis**

```text
wtw init [--shell <SHELL>] [PROFILE_PATH]
```

**Options (InitCommand)**

- `--shell <SHELL>` (ValueEnum, default: `pwsh`)  
  Shell kind. Supported values:

  - `pwsh` (PowerShell; supported).
  - `cmd` (Windows Command Prompt; not supported).
  - `bash` (Bash; not supported).

- `PROFILE_PATH` (optional positional path)  
  Path to the profile file to be modified. If omitted and `--shell pwsh` is
  used, WTW computes a default profile path using the `USERPROFILE` or `HOME`
  environment variable:

  ```text
  <HOME>\Documents\PowerShell\Microsoft.PowerShell_profile.ps1
  ```

**Behavior**

- For `pwsh`:

  - Ensures the profile directory exists, creating it if necessary.
  - Reads the existing profile content (if any).
  - If the content already contains the marker line `# wtw shell integration`,
    it performs no changes (idempotent).
  - Otherwise, opens the profile file in append mode, optionally inserts a
    newline, and appends:
    - The marker line `# wtw shell integration`.
    - The PowerShell script from `shell::pwsh::script()`.

- For `cmd` or `bash`:

  - Returns an error with the message:
    - `"shell 'cmd' is not supported yet"` or
    - `"shell 'bash' is not supported yet"`.

  These errors are treated as user errors (exit code 1).


4.2.6 `wtw shell-init`
^^^^^^^^^^^^^^^^^^^^^^

**Purpose**  
Emit the shell integration script to standard output instead of writing it to
the profile file. This allows manual inspection or composition.

**Synopsis**

```text
wtw shell-init <SHELL>
```

**Options (ShellInitCommand)**

- `shell` (ValueEnum; required)  
  Shell kind (`pwsh`, `cmd`, `bash`).

**Behavior**

- For `pwsh`:

  - Writes the content of `shell::pwsh::script()` to standard output and
    flushes the output.

- For `cmd` or `bash`:

  - Returns an error with the same messages as `wtw init` for unsupported
    shells:
    - `"shell 'cmd' is not supported yet"`
    - `"shell 'bash' is not supported yet"`

Integration tests check that:

- `wtw shell-init pwsh` prints a script containing `function wtw` and
  `Register-ArgumentCompleter`.
- `wtw shell-init cmd` fails with the appropriate error message.

Note: The `shell::cmd` and `shell::bash` modules currently expose `script()`
functions that return empty strings, but these are not reachable from CLI
entrypoints because `cmd` and `bash` are explicitly rejected.


5. Configuration File Specification (`.wtp.yml`)
-----------------------------------------------

WTW reads configuration from a file named `.wtp.yml` located in the main
repository root (the root of the main worktree). The format is a YAML
document parsed into the following Rust types (using `serde`).

5.1 File Location and Loading
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

- The configuration file path is:

  ```text
  <main_root>/.wtp.yml
  ```

- If the file does not exist, WTW uses `Config::default()`, which corresponds
  to:

  ```yaml
  version: "1.0"
  defaults:
    base_dir: "../worktrees"
  hooks:
    post_create: []
  ```

  Integration tests verify that missing configuration produces the expected
  default values.

- If the path exists but is not a regular file (e.g., a directory),
  WTW returns a configuration error with a message containing:

  ```text
  configuration path is not a regular file: <path>
  ```

  This is mapped to exit code 2.

- If the file cannot be read, WTW returns a configuration error of the form:

  ```text
  failed to read config file <path>: <io_error>
  ```

- If the YAML cannot be parsed, WTW returns a configuration error:

  ```text
  failed to parse config file <path>: <serde_error>
  ```

  Integration tests assert that such failures produce exit code 2.

- After parsing, if `config.version` is blank or consists solely of
  whitespace, it is replaced with the default version `"1.0"`.


5.2 Top‑Level Structure
~~~~~~~~~~~~~~~~~~~~~~~

The top‑level structure corresponds to:

```rust
pub struct Config {
    pub version: String,
    pub defaults: Defaults,
    pub hooks: Hooks,
}
```

YAML mapping:

- `version` (string, optional)  
  - Default: `"1.0"` if missing or blank.

- `defaults` (mapping, optional)  
  - Default: see `Defaults` below.

- `hooks` (mapping, optional)  
  - Default: see `Hooks` below.

No `deny_unknown_fields` attribute is applied to `Config` or `Defaults`,
which means additional unknown keys at these levels are ignored by WTW
rather than causing a parse error.


5.3 Defaults
~~~~~~~~~~~~

```rust
pub struct Defaults {
    pub base_dir: PathBuf,
}
```

- `base_dir` (string path)  
  Base directory for managed worktrees. YAML example:

  ```yaml
  defaults:
    base_dir: "../worktrees"
  ```

  Behavior:

  - If omitted, defaults to `"../worktrees"`.
  - When resolving the effective base directory:
    - If `base_dir` is absolute, it is returned normalized.
    - If `base_dir` is relative, it is joined to the canonicalized main
      repository root.
  - On Windows, extended path prefixes like `\\?\` are stripped in normalized
    paths used by WTW.


5.4 Hooks
~~~~~~~~~

```rust
pub struct Hooks {
    pub post_create: Vec<Hook>,
}
```

Currently, only `hooks.post_create` is supported. Other hook lists are not
read or used by WTW.

Each `Hook` is a tagged enum:

```rust
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Hook {
    Copy(CopyHook),
    Command(CommandHook),
}
```

This corresponds to YAML entries of the form:

```yaml
hooks:
  post_create:
    - type: copy
      from: "<path>"
      to: "<path>"
    - type: command
      command: "<shell_command>"
      env:
        KEY: "VALUE"
      work_dir: "<path>"
```

5.4.1 `copy` hook
^^^^^^^^^^^^^^^^^

```rust
#[serde(deny_unknown_fields)]
pub struct CopyHook {
    pub from: PathBuf,
    pub to: PathBuf,
}
```

Fields:

- `from` (string path, required)  
  Path relative to the main worktree root, or absolute.

- `to` (string path, required)  
  Path relative to the new worktree root, or absolute.

Behavior:

- At execution time, WTW:

  - Resolves `from` against the main repository root if relative.
  - Resolves `to` against the new worktree path if relative.

- If the source path does not exist or cannot be inspected, the hook fails
  with an error message indicating that the source path does not exist.

- If `to` refers to a directory:

  - WTW creates all necessary parent directories.
  - If the source is a directory, it recursively copies all contents.
  - If the source is a file, it copies the file to the target file path,
    creating parent directories as needed.

- All errors from filesystem operations are wrapped with contextual messages
  and cause the hook (and thus `wtw add`) to fail.

Validation:

- Because `CopyHook` uses `deny_unknown_fields`, any unknown keys in the YAML
  mapping for a `copy` hook cause the configuration to fail to parse.


5.4.2 `command` hook
^^^^^^^^^^^^^^^^^^^^

```rust
#[serde(deny_unknown_fields)]
pub struct CommandHook {
    pub command: String,
    pub env: BTreeMap<String, String>,
    pub work_dir: Option<PathBuf>,
}
```

Fields:

- `command` (string, required)  
  The shell command to execute.

- `env` (mapping, optional)  
  Environment variables to set for this command execution.

- `work_dir` (string path, optional, named `work_dir` in YAML)  
  Working directory relative to the new worktree when relative; absolute
  values are used as‑is.

Behavior:

- WTW logs the command being executed to the hook output.
- It then spawns a shell:

  - On Windows: `cmd /C <command>`.
  - On non‑Windows platforms: `sh -c <command>`.

- The working directory is:

  - `work_dir` resolved against the new worktree if specified, or
  - The new worktree path itself if `work_dir` is omitted.

- Environment:

  - The environment variable `WTP_SHELL_INTEGRATION` is removed.
  - All key/value pairs from `env` are set.
  - Two additional environment variables are always provided:

    - `GIT_WTP_WORKTREE_PATH`: the new worktree path as a string.
    - `GIT_WTP_REPO_ROOT`: the main repository root path as a string.

- The subprocess output:

  - `stdout` is written to the hook writer if non‑empty.
  - `stderr` is also written to the hook writer if non‑empty.

- If the subprocess exits with a non‑zero status, the hook fails with an
  error:

  ```text
  command exited with status <status>
  ```

  and `wtw add` fails accordingly (exit code 10).

Validation:

- Because `CommandHook` uses `deny_unknown_fields`, unknown keys in a
  `command` hook mapping also cause configuration parsing to fail.


6. Hook Execution
-----------------

Hook execution is performed by `HookExecutor`, which is created with a
reference to the loaded `Config` and the main repository root.

On `wtw add` success, WTW:

1. Constructs a `HookExecutor`.
2. Calls `execute_post_create_hooks` with:

   - A mutable writer wrapping standard output.
   - The path of the newly created worktree.

3. `execute_post_create_hooks`:

   - Returns immediately if `hooks.post_create` is empty.
   - Otherwise:
     - Prints an introductory message:

       ```text
       Executing post-create hooks...
       ```

     - For each hook (1‑based index):

       - Prints:

         ```text
         → Running hook <i> of <n>...
         ```

       - Executes the hook (`copy` or `command`).
       - If the hook succeeds, prints:

         ```text
         ✓ Hook <i> completed
         ```

     - After all hooks succeed, prints:

       ```text
       ✓ All hooks executed successfully
       ```

4. Any error during `copy` or `command` execution stops further hooks and
   causes `wtw add` to fail.

Integration tests verify that:

- Hooks are executed after worktree creation.
- A `copy` hook correctly copies a file from the main worktree into the new
  worktree.
- A `command` hook can create a file (`hook.log`) whose contents include
  the expected output.
- The success messages for hook execution are present in `stdout`.


7. Git and Worktree Integration
-------------------------------

7.1 Repository Context Discovery
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

`RepoContext::discover` determines the Git context as follows:

1. Determine the starting directory:

   - If `--repo <PATH>` was provided, resolve it:
     - Relative paths are resolved against the current directory.
     - If the path refers to a file, its parent directory is used.
     - If the path (or its parent) does not exist, an error is returned.
   - Otherwise, use `std::env::current_dir()`.

2. Run `git rev-parse --show-toplevel` in the starting directory to obtain
   the worktree root. This is canonicalized, and if the command fails, a
   contextual error is returned.

3. Run `git rev-parse --git-common-dir` in the worktree root to obtain the
   common Git directory. This path is resolved against the worktree root if
   it is relative, and canonicalized.

4. If the resolved common directory ends with `.git`, its parent directory is
   used as the main repository root. Otherwise, the canonical common
   directory is used directly as `main_root`.

5. The repository name (`repo_name`) is the final path component of
   `main_root`; if that cannot be determined, `main_root`’s display string
   is used.

`RepoContext` provides:

- `worktree_root()`: path to the current worktree.
- `main_root()`: path to the main repository root.
- `repo_name()`: the derived repository name.
- `is_main_worktree()`: whether current worktree equals main root (after
  canonicalization).


7.2 Git Command Execution
~~~~~~~~~~~~~~~~~~~~~~~~~

`GitRunner` encapsulates calls to `git.exe`:

- All commands are logged at `DEBUG` or higher with:

  - The working directory path.
  - The full command string as formatted by `format_command`.

- `GitRunner::run`:

  - Runs a command and returns a `GitOutput` on success.
  - If the command exits with a non‑zero status, returns `GitError::CommandFailed`
    with the exit status and both `stdout` and `stderr` captured.

- `GitRunner::run_in`:

  - Same as `run` but takes an explicit working directory.

- `GitRunner::run_with_status` / `run_with_status_in`:

  - Return `GitOutput` even when Git fails, allowing callers to inspect and
    interpret the exit code and output themselves.

`GitOutput` exposes:

- `command`: formatted command string.
- `status`: `ExitStatus`.
- `stdout`: captured standard output.
- `stderr`: captured standard error.

`GitError` variants:

- `Spawn`  
  Command could not be started (e.g. `git` not found). Includes the working
  directory, command string, and underlying `io::Error`.

- `CommandFailed`  
  `git` exited with a non‑zero status. Includes the status and captured
  `stdout`/`stderr`.

- `InvalidUtf8`  
  Output from `git` could not be decoded as UTF‑8.

Errors from `GitRunner` are generally wrapped as Git application errors and
mapped to exit code 3 by `main`.


7.3 Worktree Parsing
~~~~~~~~~~~~~~~~~~~~

`git::worktree::list_worktrees`:

- Executes `git worktree list --porcelain` via `GitRunner`.
- Parses lines into `WorktreeInfo` as follows:

  - `worktree <path>`: starts a new entry and sets `path` to the canonicalized
    version of `<path>`.
  - `HEAD <hash>`: sets the full commit hash.
  - `branch refs/heads/<branch>`: sets `branch` to `<branch>`.
  - `detached`: sets `is_detached = true` and prevents `branch` from being
    used.
  - `locked <reason>` or `locked`: sets `locked` accordingly.
  - `prunable <reason>` or `prunable`: sets `prunable` accordingly.
  - Blank lines delimit entries.

- After parsing, the first entry in the list is marked `is_main = true`.

This behavior is covered by unit tests in `git::worktree`.


8. Shell Integration (PowerShell)
---------------------------------

8.1 Generated Script
~~~~~~~~~~~~~~~~~~~~

The PowerShell script emitted by `wtw shell-init pwsh` and appended by
`wtw init` contains:

- A helper function `Get-WtwExePath` that:

  - First looks for `wtw.exe` using `Get-Command`.
  - Falls back to `Get-Command wtw -CommandType Application`.
  - Throws an error if no executable is found.

- A `wtw` function that:

  - Forwards arguments to the actual `wtw.exe`.
  - Captures `stdout` and the exit code.
  - If the exit code is zero and the first argument is `cd`:

    - Reads the last line of the output, trims it, and, if non‑empty,
      calls `Set-Location` to that path.

  - Otherwise, writes the output (if any) to the console.
  - Sets `$global:LASTEXITCODE` to the exit code from `wtw.exe`.

- An argument completer registered via `Register-ArgumentCompleter`:

  - When completing the first argument (the subcommand), suggests:
    `add`, `list`, `remove`, `cd`, `shell-init`.
  - When the subcommand is `cd`, it:
    - Invokes `wtw list --json`.
    - Parses the JSON into objects with a `.name` field.
    - Suggests each `name` as a completion candidate.
    - Special‑cases the `"@"` name by offering it quoted as `'@'` to avoid
      PowerShell parsing issues.

These behaviors are asserted by unit tests in `shell::pwsh` and integration
tests in `tests/shell_spec.rs`.


9. Logging and Error Handling
-----------------------------

9.1 Logging
~~~~~~~~~~~

`logging::init` configures a `tracing_subscriber` based on `GlobalOptions`:

- If `--quiet` is set:

  - Maximum log level is `ERROR`.

- Else if `--verbose` is:

  - `0`: maximum level is `WARN`.
  - `1`: maximum level is `DEBUG`.
  - `>= 2`: maximum level is `TRACE`.

Logs are written to standard error (`stderr`), without timestamps or target
information. Integration tests verify that using `--verbose` results in
debug lines such as `"Executing git command"` being emitted.


9.2 Error Types and Exit Codes
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

WTW uses a structured error type `AppError` with four variants:

- `User(String)`  
  For user mistakes such as missing arguments or invalid combinations.

- `Config(String)`  
  For problems related to configuration files, especially `.wtp.yml`.

- `Git(String)`  
  For failures of underlying Git commands.

- `Internal(String)`  
  For unexpected internal errors.

Each variant maps to an exit code:

- `User` → `1`
- `Config` → `2`
- `Git` → `3`
- `Internal` → `10`

Integration tests assert that:

- Invalid configuration results in exit code 2 and an error message starting
  with `"failed to parse config file"`.
- Git failures during `add` yield exit code 3.

The `main` function handles errors as follows:

1. Calls `wtw::run()`:

   - On `Ok(ExitCode)`: returns that exit code (currently always success).

2. On `Err(error)`:

   - First attempts to downcast directly to `AppError`.
   - If successful:
     - Prints the error message.
     - Exits with the corresponding `AppError::exit_code()`.

   - If not, iterates over the error cause chain and:

     - If an `AppError` cause is found:
       - Uses its message and exit code.

     - Else if a `GitError` cause is found:
       - Treats it as a Git error:
         - Exit code: 3.
         - Message: `GitError`’s `Display` output.

     - Else if a `serde_yaml::Error` cause is found:
       - Treats it as a configuration error:
         - Exit code: 2.
         - Message: the YAML error’s `Display` output.

   - If none of the above are found:

     - Uses the top‑level error’s `Display` output as the message.
     - Uses exit code 10.

In all failure cases, the selected message is printed to `stderr`.


10. Testing Strategy and Behavioral Guarantees
---------------------------------------------

The tests in `tests/` serve as an executable specification. Key guarantees
include:

- **Repository discovery and `--repo`**  
  `wtw list --json` works both inside and outside a repository when `--repo`
  is provided, returning at least the main worktree entry.

- **Configuration handling**  
  - Missing `.wtp.yml` yields default values (`version` and `base_dir`).
  - Invalid YAML produces a clear error message and exit code 2.
  - A directory at `.wtp.yml` is rejected as “not a regular file”.

- **`add` behavior**  
  - Creates new worktrees under the configured `base_dir` with paths derived
    from branch names.
  - Requires a branch or commit argument when no `--branch`/`--track` is used.
  - Detects branch conflicts and reports them with clear messages.
  - Enforces `--track` argument requirements.
  - Runs post‑create hooks and observes their effects (copied files and
    command‑generated files).

- **`cd` behavior**  
  - `wtw cd @` resolves to the repository root.
  - `wtw cd <display_name>` resolves to the appropriate worktree path.
  - Unknown worktrees produce “not found” errors including:
    - An “Available worktrees” list.
    - A “Run 'wtw list'” hint.

- **`list` behavior**  
  - `list --json` includes the main worktree with `name = "@"` and
    `branch = "main"`.
  - `list` marks dirty worktrees and shows upstream branches when configured.
  - `list` marks the current worktree with an asterisk in the `PATH` column.
  - `list --json` correctly reflects `is_main` and `is_current` flags.

- **`remove` behavior**  
  - `remove --with-branch --force-branch` deletes both the worktree directory
    and its branch.
  - `remove` only affects worktrees under the currently configured `base_dir`;
    changing `base_dir` can make existing worktrees unmanaged and thus
    protected from removal.
  - Attempting to remove the current worktree fails with a clear error and
    leaves the directory intact.
  - `--force-branch` without `--with-branch` is rejected.

- **Shell integration**  
  - `shell-init pwsh` emits a script containing both the wrapper function and
    the argument completer.
  - `shell-init cmd` is explicitly not supported and fails with the
    documented error message.

- **Help and version**  
  - `wtw --help` prints usage, descriptions, and lists subcommands including
    `shell-init`.
  - `wtw --version` prints the package version as defined by `CARGO_PKG_VERSION`.

- **Verbosity**  
  - Using `--verbose` results in debug logging that includes Git command
    execution messages.

Any future changes to WTW should preserve these behaviors unless the tests
are deliberately updated to reflect new specifications.


