wtw (Worktree Pro for Windows)
==============================

**wtw** は、Rust 製の Windows 向け Git worktree 支援ツールです。  
オリジナルの **wtp (Worktree Plus)** との高い互換性を保ちつつ、Windows 11 と PowerShell で快適に使えることを目標としています。  
名前の「wtw」は **Worktree Pro for Windows** の略です。

本プロジェクトは **wtp バージョン 2.3.4** をベースにしたフォークです。  
今後は wtp 本体との完全な互換性維持を目標とはせず、このリポジトリ独自の方向性で機能追加や仕様変更を行っていく予定です。  
そのため、**このリポジトリに含まれるすべてのファイルは、元の wtp から何らかの変更が加えられているものとみなしてください。**

> ステータス: 0.1 系の早期版です。日常的な利用には十分な機能がありますが、wtp との互換性はまだ移植途中の部分があります。  
> 最新の互換状況は `docs/wtp-wtw-feature-map.md` を参照してください。


主な特徴
--------

- **Windows 向けに最適化された worktree ヘルパー**
  - 内部的に `git.exe` を呼び出して動作します。
  - Windows のパス表現やドライブレターを考慮した実装になっています。
- **wtp とほぼドロップイン互換**
  - `.wtp.yml` のフォーマット（`version`, `defaults.base_dir`, `hooks.post_create` など）をそのまま読み込みます。
  - `add`, `list`, `remove`, `cd` の挙動は wtp に極力合わせています。
- **自動的な worktree パスレイアウト**
  - 例えば `feature/auth` というブランチ名は、既定では `../worktrees/feature/auth` にマップされます。
  - Windows で使えない文字を含むブランチ名はサニタイズされます（例: `feat:bad*name` → `feat_bad_name`）。
- **post_create hooks による自動セットアップ**
  - `copy` フックで、メイン worktree から `.env` のような gitignore されたファイルをコピーできます。
  - `command` フックで、依存関係のインストールや DB マイグレーションなどのコマンドを自動実行できます。
- **`list` のリッチな出力と JSON 対応**
  - `PATH`, `BRANCH`, `HEAD`, `STATUS`, `UPSTREAM`, `ABS_PATH` を含む表形式で一覧表示します。
  - `wtw list --json` で JSON 形式の一覧を出力でき、スクリプトや PowerShell 補完から利用できます。
- **PowerShell 連携**
  - `wtw init` で PowerShell プロファイルに関数を追記し、`wtw cd` で実際にカレントディレクトリが移動するようになります。
  - `wtw` のサブコマンドや `wtw cd` の worktree 名に対するタブ補完を提供します。


動作環境
--------

- **OS**: Windows 11（その他の Windows でも動作する可能性はありますが、公式には 11 を前提としています）
- **Git**: Git for Windows（`git.exe` が `PATH` に通っていること）
- **シェル**:
  - PowerShell 7+（推奨。現時点でフルサポートしている唯一のシェル）
  - Cmd / Git Bash も将来的に対応予定ですが、`shell-init` はまだ未実装です。
- **Rust ツールチェーン**（ソースからビルドする場合のみ）
  - Rust stable
  - `cargo`


インストール
------------

### 1. 配布バイナリからインストール（推奨）

GitHub Releases などで配布する想定のアーカイブは次のような名前です:

- `wtw-<version>-x86_64-pc-windows-msvc.zip`

アーカイブには最低限、次のファイルを含めます:

- `wtw.exe`
- `README.md`（英語または日本語のどちらか / 本リポジトリでは英語版を想定）
- `LICENSE`

インストール例（PowerShell）:

```powershell
# 1. Releases ページから ZIP をダウンロード
# 2. 任意のディレクトリに展開（例）
Expand-Archive -Path .\wtw-0.1.0-x86_64-pc-windows-msvc.zip -DestinationPath C:\tools\wtw

# 3. 展開したディレクトリを PATH に追加（ユーザー環境変数）
[System.Environment]::SetEnvironmentVariable(
  "Path",
  $env:Path + ";C:\tools\wtw",
  "User"
)

# 4. 新しい PowerShell を開いて動作確認
wtw --help
```

> アーカイブ名や展開先はプロジェクト運用に合わせて適宜変更してください。


### 2. ソースコードからビルドしてインストール

このリポジトリをクローンし、`wtw` crate ディレクトリでビルドします:

```powershell
git clone <このリポジトリ>
cd wtw

# リリースビルド
cargo build --release

# そのままバイナリを実行
.\target\release\wtw.exe --help

# あるいは cargo install でインストール
cargo install --path .
wtw --help
```


クイックスタート
----------------

### 1. Git リポジトリを用意する

Git リポジトリ直下、または `--repo` でリポジトリを指し示した状態で `wtw` を実行します。  
`wtw` は `git rev-parse --show-toplevel` 相当の処理でリポジトリ root を自動検出します。

```powershell
# 既存の Git リポジトリ内で
cd C:\src\my-project
wtw list --json

# リポジトリの外から --repo で指定
wtw --repo C:\src\my-project list --json
```


### 2. PowerShell 連携を有効化する（推奨）

`wtw.exe` が `PATH` に通っていれば、1 コマンドで PowerShell 連携を有効化できます:

```powershell
# 既定の PowerShell プロファイルを使う
wtw init

# シェル種別やプロファイルパスを明示的に指定する場合
wtw init --shell pwsh
wtw init --shell pwsh C:\Users\<you>\Documents\PowerShell\Microsoft.PowerShell_profile.ps1
```

この処理により、次のような設定が行われます:

- プロファイルファイル（例: `Microsoft.PowerShell_profile.ps1`）が存在しない場合は作成。
- `# wtw shell integration` から始まるセクションを追記。
- 実際の `wtw.exe` を呼び出す `wtw` 関数が定義され、  
  最初の引数が `cd` でコマンドが成功した場合、出力されたパスに `Set-Location` されます。
- `Register-ArgumentCompleter` により:
  - サブコマンド（`add`, `list`, `remove`, `cd`, `shell-init`）の補完
  - `wtw cd` で `wtw list --json` の結果に基づく worktree 名の補完
  が有効になります。

設定後、新しい PowerShell セッションを開き、次のように試せます:

```powershell
wtw cd "@"
wtw cd <TAB>  # worktree 名が補完される
```

プロファイルを自分で管理したい場合は、スクリプトだけ出力して確認することもできます:

```powershell
wtw shell-init pwsh > wtw.ps1
# .\wtw.ps1 を dot-source するか、内容をプロファイルにコピーしてください
```


基本的な使い方
--------------

### worktree を追加する (`add`)

```powershell
# 既存のローカル / リモートブランチから worktree を作成
wtw add feature/auth

# 新規ブランチと worktree を同時に作成
wtw add -b feature/new-feature

# 特定のリモートブランチを追跡する新規ブランチ + worktree
wtw add --track origin/feature/remote-only

# 特定コミットから新規ブランチを切って worktree を作成
wtw add -b hotfix/urgent abc1234
```

- 既定では、worktree はリポジトリ root から見た `../worktrees` 配下に作成されます。
- ブランチ名に `/` が含まれる場合、その区切りごとにディレクトリが切られます  
  （例: `feature/auth` → `../worktrees/feature/auth`）。


### worktree を一覧表示する (`list`)

```powershell
# 表形式の一覧
wtw list

# 出力例（簡略化）
# PATH                      BRANCH           HEAD     STATUS  UPSTREAM            ABS_PATH
# ----                      ------           ----     ------  --------            --------
# @*                        main             c72c7800 clean   origin/main         C:\src\my-project
# feature/auth              feature/auth     def45678 dirty   origin/feature/auth C:\src\my-project\..\worktrees\feature\auth

# ツールや補完から使いやすい JSON 形式
wtw list --json
```

`wtw list --json` の出力イメージ:

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


### worktree を削除する (`remove`)

```powershell
# 表示名 / ブランチ名 / ディレクトリ名で worktree を削除
wtw remove feature/auth

# dirty な worktree も強制削除
wtw remove --force feature/auth

# 対応するブランチも削除（マージ済みが前提）
wtw remove --with-branch feature/auth

# worktree 削除 + ブランチを強制削除
wtw remove --with-branch --force-branch feature/auth
```

- `.wtp.yml` の `base_dir` 管理下にある worktree のみ削除対象です。
- **現在の worktree** は削除できず、エラーが返されます。


### worktree 間を移動する (`cd`)

PowerShell 連携（`wtw init`）を有効にしている場合、次のように移動できます:

```powershell
# 名前やブランチ名で worktree に移動
wtw cd feature/auth

# メイン worktree に戻る
wtw cd @
wtw cd my-project   # リポジトリ名でも指定可能
```

存在しない worktree 名を指定した場合は、候補一覧とともに  
`Run 'wtw list' to see available worktrees.` というヒント付きのエラーメッセージが表示されます。


設定ファイル: `.wtp.yml`
-------------------------

`wtw` はリポジトリ root の `.wtp.yml` を読み込み、**wtp と互換性のある形式**で解釈します。

### base_dir の設定

```yaml
version: "1.0"
defaults:
  # worktree のベースディレクトリ（リポジトリ root からの相対、または絶対パス）
  base_dir: "../worktrees"
```

- 相対パスの `base_dir` は Git リポジトリ root を基準に解決されます。
- 絶対パスもサポートしており、異なるドライブを指すことも可能です。


### フック設定

```yaml
version: "1.0"
defaults:
  base_dir: "../worktrees"

hooks:
  post_create:
    # メイン worktree から新規 worktree へ、gitignore されたファイルをコピー
    - type: copy
      from: ".env"     # メイン worktree からの相対パス
      to: ".env"       # 新規 worktree からの相対パス

    # 新規 worktree 上でセットアップコマンドを実行
    - type: command
      command: "npm ci"
      env:
        NODE_ENV: "development"

    - type: command
      command: "npm run db:setup"
      work_dir: "."
```

挙動のポイント:

- `from` は常に **メイン worktree** からの相対パスとして解釈されます。
- `to` は新規 worktree からの相対パスとして解釈されます。
- `command` フックは新規 worktree 内で実行され、`env` や `work_dir` で環境変数や作業ディレクトリを指定できます。
- いずれかのフックが失敗した場合、`wtw add` 全体が失敗として扱われます。

> **セキュリティ注意**: `command` フックは `.wtp.yml` に記述された任意のコマンドを実行します。  
> 信頼できるリポジトリでのみ有効化し、`wtw add` を実行する前にフック定義の内容を確認してください。


終了ステータス
--------------

`wtw` はエラーの種類ごとに終了コードを使い分けます:

- `0`: 正常終了
- `1`: ユーザーエラー（引数のミス、存在しない worktree など）
- `2`: 設定ファイルエラー（無効な `.wtp.yml` など）
- `3`: Git コマンドの失敗
- `10`: 想定外の内部エラー


wtp との互換性
--------------

`wtw` は、ベースとしている wtp 2.3.4 と **ある程度の互換性** を保ちつつも、  
今後は wtw 固有の拡張や仕様変更も行っていくことを想定しています:

- `.wtp.yml` の設定フォーマットを共有します。
- worktree のレイアウトや命名規則、`add/list/remove/cd` の基本挙動は wtp に近づけています。
- PowerShell の `wtw init` / `wtw shell-init pwsh` は、macOS/Linux 上の wtp の体験を Windows に持ち込むことを意図しています。

一方で、現時点では次のような差分・未対応もあります:

- `cmd` / `bash` 向けの `shell-init` は未実装です。
- 一部の「helpful error」（詳細なエラーメッセージ）やリモートブランチ解決ロジックは、wtp ほどリッチではありません。
- wtp 固有の追加フラグ（`list --quiet` / `--compact` など）はまだ Rust 版では露出していません。

詳細な対応状況・ギャップについては次を参照してください:

- `docs/spec.md`
- `docs/wtp-wtw-feature-map.md`


ライセンス
----------

WTW は、MIT License のもとで公開されている  
[satococoa/wtp](https://github.com/satococoa/wtp) をベースにしたプロジェクトです。

本リポジトリも同じく MIT License に従って配布されており、詳細な条文は同梱の `LICENSE` を参照してください。  
上流プロジェクト wtp のライセンスについては、wtp リポジトリに含まれる `LICENSE` を参照してください。


