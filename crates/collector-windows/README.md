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
$env:ASHIATO_STATE_DIR = "$env:APPDATA\ashiato"
.\ashiato-collector-windows.exe
```

**Windows の上でだけ動く**（前景・最後の入力・UI Automation を読む）。
他の OS では起動時に落ちる —— 動いているつもりで 1 件も入らない状態が
いちばん見つかりにくいため。

置き場（`ASHIATO_STATE_DIR`）に作られるもの:

| | |
|---|---|
| `outbox.jsonl` | まだ送れていない記録。**上限なし**（捨てない。design D3） |
| `heartbeat.jsonl` | まだ送れていない生存信号 |
| `last-seen.txt` | 「ここまで動いていた」の印（FR-82 の材料。1 分ごとに更新） |
| `counters.json` | 取得の試行と成功の数え（起動をまたいで残す） |
| `exclusions.json` | **除外の登録**（下記） |

## 自動起動（design D7）

```powershell
.\ashiato-collector-windows.exe --install-autostart
```

スタートアップフォルダへ `ashiato-collector.cmd` を 1 つ置く（**消せば止まる**）。
NFR-12 が「収集に手作業を要さない」と定めているので既定でこれを仕込む ——
**起動を忘れた期間の記録は後から作れない。**

> **トレイの常駐表示はまだ無い**（design D7 の後半）。止まっていることに気づく手段は
> いまは稼働状況の画面（生存信号が 6 時間ごとに届く。FR-78 / FR-80）だけ。

## 除外の登録（FR-83 / 深掘り Q5）

**既定は空**（何も除外しない）。`exclusions.json` に書く:

```json
{
  "rules": [
    { "match": "process-name",   "value": "1password.exe" },
    { "match": "exe-path",       "value": "C:\\Program Files\\KeePassXC\\KeePassXC.exe" },
    { "match": "title-contains", "value": "シークレット" }
  ]
}
```

| `match` | 当たり方 |
|---|---|
| `exe-path` | 実行ファイルのパスの完全一致（大文字小文字を無視） |
| `process-name` | プロセス名の完全一致（同上） |
| `title-contains` | ウィンドウ題名の部分一致（同じソフトの中の一部の窓だけ落とす） |

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

書き間違えた JSON は**空に倒さずエラーで止まる** —— 空に倒すと、
除外が黙って外れて残したくなかった題名と URL が入る。

## 開発

```bash
cargo test -p ashiato-collector-windows          # 規則の部分（OS を触らない）
cargo check -p ashiato-collector-windows --target x86_64-pc-windows-gnu   # 実機の部分
```

**OS を触る部分（`platform.rs`）と規則の部分（`engine.rs` ほか）を分けてある。**
規則の部分は Linux でも走るので、実機を待たずに確かめられる。実機の部分は
Linux からは**コンパイルだけ**確かめる（CI の `collector-windows` job が同じことをする）。
`unsafe` は 1 行も書かない（作業場の lint が `forbid`。design D15）。
