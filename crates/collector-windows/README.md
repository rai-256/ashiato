<!-- SPDX-License-Identifier: AGPL-3.0-only -->
# C-02 — PC のウィンドウを集める（ST07）

前景のアプリ・ウィンドウ題名・URL の変化を 1 件ずつ、離席・PC が止まっていた期間・
除外した件数と併せて取り込み口（S-01）へ送る。契約は `docs/collector-contract.md`、
決めたことは `openspec/changes/st07-active-window/deep.md`（**本人が決めた 9 件**）。

## 動かす

```powershell
$env:ASHIATO_BASE_URL  = "http://127.0.0.1:8787"
$env:ASHIATO_API_TOKEN = "<合言葉>"
$env:ASHIATO_USER_ID   = "<uuid>"
$env:ASHIATO_DEVICE_ID = "pc-01"
$env:ASHIATO_STATE_DIR = "$env:APPDATA\ashiato"      # **絶対パス。既定は無い**
.\ashiato-collector-windows.exe
```

**5 つとも必須**（置き場も含む）。1 つでも欠けると起動しない —— 既定で埋めると
送り先や置き場を間違えたまま動き、前の置き場の未送信・印・除外の登録が置き去りになる。

**Windows の上でだけ動く**（前景・最後の入力・UI Automation を読む）。
他の OS では起動時に落ちる。ビルドも Windows の上で行う
（`cargo build --release -p ashiato-collector-windows`）。

置き場（`ASHIATO_STATE_DIR`）に作られるもの:

| | |
|---|---|
| `outbox.jsonl` | まだ送れていない記録。**上限なし**（捨てない。design D3） |
| `heartbeat.jsonl` | まだ送れていない生存信号 |
| `*.broken.*` | 読めなかった行・印・数えの退避先（**消さずに残す**） |
| `last-seen.txt` | 「ここまで動いていた」の印（FR-82 の材料。1 分ごとに更新） |
| `clean-stop.txt` | 自分で止まったときだけ書く印（次の起動で読んで消す。design D23） |
| `engine.json` | 開いている離席の区間と除外の数え（**本文は含まない**。design D21） |
| `counters.json` | 取得の試行と成功の数え（起動をまたいで残す） |
| `exclusions.json` | **除外の登録**（下記） |
| `collector.log` | ログ。**件数・種別だけで、題名も URL も出ない** |

**二重に起動しない。** 印が 2 分半より新しければ「もう動いている」として終わる（design D24）。

## 自動起動（design D7）

```powershell
.\ashiato-collector-windows.exe --install-autostart
```

スタートアップフォルダへ `ashiato-collector.cmd` を 1 つ置く（**消せば止まる**）。
**いまの 5 つの変数を全部書き込む**ので、上の「動かす」の変数を設定したシェルで実行する。
**合言葉が平文で入る** —— このファイルを他人に渡さない。
NFR-12 が「収集に手作業を要さない」と定めているので既定でこれを仕込む ——
**起動を忘れた期間の記録は後から作れない。**

> **トレイの常駐表示は無い**（design D7・仮）。止まっていることに気づく手段は
> 稼働状況の画面（生存信号が 6 時間ごとに届く。FR-78 / FR-80）と `collector.log`。
> 落ちても次の起動の `powered-off` に `boot_at` / `clean_stop` が載るので、
> 「PC を閉じていた」か「収集だけが止まっていた」かは後から分かる（design D23）。

## 除外の登録（FR-83 / 深掘り Q5）

**既定は空**（何も除外しない）。`exclusions.json` に書く:

```json
{
  "rules": [
    { "match": "process-name",   "value": "1password.exe" },
    { "match": "exe-path",       "value": "C:\\Program Files\\KeePassXC\\KeePassXC.exe" },
    { "match": "title-contains", "value": "シークレット" },
    { "match": "url-contains", "value": "accounts.example.test" },
    { "match": "browser-profile", "browser": "chrome", "profile": "Work" }
  ]
}
```

| `match` | 当たり方 |
|---|---|
| `exe-path` | 実行ファイルのパスの完全一致（大文字小文字を無視） |
| `process-name` | プロセス名の完全一致（同上） |
| `title-contains` | ウィンドウ題名の部分一致（同じソフトの中の一部の窓だけ落とす） |
| `url-contains` | URL の部分一致。前景では題名なども含めてその変化全体を除外し、履歴にも効く |
| `browser-profile` | ブラウザ履歴の指定ブラウザ・プロファイルだけを除外（前景には当てない） |

除外された間は**アプリ名も題名も URL も記録されず、取り込み口へも送られない**。
残るのは「除外が起きたこと」と**その件数**だけ（`kind: "excluded"`）——
残さないと、その時間帯の「記録が無い」が「PC を触っていなかった」のか
「除外された」のかを永久に区別できない（扉 #14）。

### **最初に登録しておくもの**（design Risks）

深掘り Q3 で**ウィンドウ題名と URL の既定の感度は「外部 AI に出してよい」**（PERM-3）に
置いた。そのため **URL に載った鍵（再設定リンク・共有リンク）や検索語も既定で外部 AI へ出る**。
守りは **この除外**と **PERM-9（緩める操作の確認）**の 2 つだけなので、
**使い始める前に少なくとも次を登録する**:

- パスワード管理ソフト（`1password.exe` / `KeePassXC.exe` / `Bitwarden.exe` …）
- 銀行・医療・保険の窓（`title-contains` で題名の一部を指す）
- シークレット / プライベートウィンドウ（`title-contains`: `シークレット` / `プライベート` / `InPrivate`）

書き間違えた登録は**空に倒さずエラーで止まる**（理由は `collector.log`）——
壊れた JSON だけでなく、**知らない欄（`rule` の打ち間違い）・`rules` の欠落・空の `value`** も断る。
空に倒すと、除外が黙って外れて残したくなかった題名と URL が入る。
何も除外しないときは `{"rules": []}` と書く。

## 実行時テスト（Windows の上でだけ）

`tests/runtime_windows.rs` は**テストが自分で窓を作り**、本物の前景・入力・アドレスバーを読ませて
記録を数える（design D25）。単体（`cargo test` が ubuntu で走らせる 86 本）が見ない `platform.rs` を
ここで確かめる。CI は `windows-latest` の job が同じものを走らせる。

```powershell
cargo test -p ashiato-collector-windows            # 単体 + 実行時テスト（Windows の上で）
cargo test -p ashiato-collector-windows --test runtime_windows -- --test-threads=1
```

**走らせている間はマウスとキーボードに触らない**（前景と最後の入力を本物から読む）。
相手役の窓は `tests/support/helper_window.ps1`（WinForms）と Edge で、どちらも自動で閉じる。

## 開発

```bash
cargo test -p ashiato-collector-windows          # 規則の部分（OS を触らない）
cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings   # 実機の部分
```

**OS を触る部分（`platform.rs`）と規則の部分（`engine.rs` ほか）を分けてある。**
規則の部分は Linux でも走るので、実機を待たずに確かめられる。実機の部分は
Linux からは**型検査と lint だけ**確かめる（CI の `collector-windows` job が同じことをする）。
読んだ値の解釈（ロック画面の名前・空の URL など）は `winrules.rs` に出して Linux で確かめる。
`unsafe` は 1 行も書かない（作業場の lint が `forbid`。design D15）。
