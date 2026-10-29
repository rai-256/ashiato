# ST51 — 版の変わり目が、更新の履歴として残る

## Why

更新の間は収集アプリもサーバも止まり、稼働状況の格子にはその時間が途絶として出る。いまは部品の版が変わったことがどこにも残らないので、
後から見て「更新で止めた」のか「壊れていた」のかを区別できない（成功条件 1・扉 14・扉 33）。ST58 が更新を入れ始める前に、版の変化を追記のみで残し始める（FR-121）。

## What Changes

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q1・Q2 は A（失われるものがある）で、本人の答えが推奨と違えば `spec --answers` が書き直す。

- **S-01 の版の変化の行**（Q1 の推奨・C2・C5）: D-01 の記録の schema（`core`）に追記のみの表「更新の履歴」を足す。S-01 は **migrate の終わり**（当てた・当てる版が無かった・落ちた・退避が失敗した）と
  **起動のとき**（版の検査（ST48）を通った後・待ち受けを開く前）に、直前の通った行の版と自分の版が違えば 1 行を書く。1 行は、いつ・どの部品（S-01）・きっかけ（migrate / 起動）・前の版・今の版・
  移行の結果（この回に当てた版の名前 / 当てる版が無かった / 落ちた版の名前と SQLSTATE / 退避の失敗）を持つ。DB の誤りの本文は残さない。版は ST49 の組み立ての印（design D1〜D4・D7〜D10）
- **migrate が知らない版の当たった DB で止まった回（終了コード 3）と、起動が版の検査で断られた回は書かない**（正典 distribution。古い S-01 が新しい D-01 に書かない）。更新の履歴の表がまだ無い D-01 で落ちた回も書けない
- **通らなかった回も、退避から戻した後に 1 行残る**（Q6。FR-121 ★ 2026-10-29）: migrate は版の検査を通った後・退避を取る前に、前の版・今の版・きっかけ・移行の結果「試み」（`attempted`）の 1 行を書いて commit する（退避に入るので、戻しても残る）。
  通れば今どおり結果の行を足す。戻した D-01 で試みの後に結果の行が無ければ、通らなかった回と読める。試みの行は、結果の行と同じく版が変わる回だけに書く（design D16（仮））
- **落ちた回の結果の行は D-01 の中だけに置く**（Q2）。落ちた版の名前と SQLSTATE は退避から戻すと消えるので、戻した後に落ちた試みの結果と「戻した」を書き直すのは ST58（`docs/handoff/ST58.md`）
- **収集アプリの生存信号に版を載せる**（C1・C3）: C-01・C-02 は、すべての生存信号に**その信号を積んだ時に動いていた**組み立ての印を、任意の欄 `client_version` と原文の両方に載せる。
  ST49 が「生存信号には書き込まない」としていた 2 つの Requirement を MODIFIED する（ST49 design の「生存信号の版は ST51 が足す」）。S-01 は値の形を検査せず・断らず、版を持たない信号も今までどおり受ける
- **収集アプリの版の変化は生存信号から導く**（Q3・Q4 の推奨）: 端末ごと・部品（C-01 / C-02）ごとに発信の時刻の順に並べ、隣り合う 2 件の版が違う所を 1 行とする読み口（D-01 のビュー）を足す。
  更新の前に溜まった信号が後から届いても、古い位置に入るので偽の変化にならない（FR-121 ★ 2026-10-25）
- **全記録の書き出し**（Q5 の推奨・C5）: 更新の履歴と、収集アプリの版の変化のファイルを足し、生存信号のファイルに版の列を説明つきで足す
- **画面・API・格子・途絶の判定は変えない**（Q5 の推奨・C9）。更新の間の途絶は今までどおり途絶として出る

**BREAKING ではない**: 生存信号の欄は任意で、送らない収集アプリの要求は今と同じに受ける。今の S-01 は知らない欄を読み飛ばすので、新しい収集アプリが ST51 より前の S-01 に送っても信号は入り、版は原文に残る（premises 3）。

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `distribution`: S-01 の版の変化が更新の履歴に 1 行残る・更新の履歴の置き場と列の形・更新の履歴は書き換えられない（ADDED 3 つ）
- `collection-coverage`: 生存信号が積んだ時の収集アプリの版を持つ・収集アプリの版の変化を生存信号から導いて読める・版を足しても稼働状況と途切れの判定は変わらない（ADDED 3 つ）
- `device-collection`: ST49 の「携帯の収集アプリは、記録を未送信に積む時点で自分の版を焼き込み、溜めた行の欄を落とさない」を MODIFIED（生存信号にも積む時点の版を書き込む）
- `desktop-collection`: ST49 の「PC の収集アプリは、記録を未送信に積む時点で自分の版を焼き込み、溜めた行の欄を落とさない」を MODIFIED（同上）
- `data-durability`: 「書き出しは台帳・導出・内部の表と、消したことの痕跡を入れない」を MODIFIED（ファイルの一覧に 2 つ足す）・書き出しが更新の履歴と版の変化と生存信号の版を説明つきで持つ（ADDED 1 つ）

**capability の照合**: INDEX.md の capability 表は ST51 を `collection-coverage` と `distribution` に割り当てていた。更新の履歴（S-01）は `distribution`、生存信号の版と導いた変化は `collection-coverage` で合っている。
ほかに、版を生存信号に焼き込むのは収集アプリの振る舞い（`device-collection` / `desktop-collection`。ST49 がそこに「生存信号には書き込まない」を置いた）、
書き出しは分類の無い表と列で止まる（`data-durability` の正典）ので、この 3 つに ST51 を足し、INDEX.md の訂正の節に理由を書いた。どれも後続の振る舞いの前倒しではない。

**requires の変更**: ST51 の requires に **ST49** を足した（layer 2 → 3。INDEX.md の訂正の節）。収集アプリと S-01 の組み立ての印は ST49 が作り、ST51 は読むだけ（C7）。
ST49 は下流が走っていて gate 前（`story_deps.py sync-specs ST51` は `waiting: ["ST49"]`）なので、MODIFIED は **ST49 の delta の今の本文**（`openspec/changes/st49-record-component-version/specs/`、
main の `19eb9e3c` の時点）から写した。ST49 の spec が gate までに変われば、merge gate の「依存の spec:」が差分を示し、ここを直す（design の Risks）。

## Impact

- コード（S-01）: `crates/server/src/heartbeat.rs`（`client_version`）、`crates/server/src/lib.rs`（`run_migrate` の終わりと `run` の版の検査の後に更新の履歴を書く・生存信号の INSERT）、
  `crates/server/src/migration_ledger.rs`（版の検査の後・退避の前に試みの行を書く口を呼ぶ・落ちた版の名前と SQLSTATE を呼び出し元へ返す）、新しい `crates/server/src/update_history.rs`、`crates/server/src/grants.sql`（更新の履歴に SELECT と INSERT だけ）、
  `crates/server/src/export.rs`（更新の履歴・版の変化のファイル・`HEARTBEAT_COLUMNS` に `client_version`）、`docs/openapi.json`（再生成）
- 移行: 新しい移行 1 つ（更新の履歴の表と書き換えの門・生存信号の `collector_version` 列・版の変化のビュー）と `.down.sql`、`migrations/ORDER` の末尾へ 1 行。名前の時刻は `202610250000` 以後で、ST48 の D10（仮）の規則に従う
- 収集アプリ: `crates/collector-windows/src/heartbeat.rs`（積む時点の印を欄と原文に）、`collector-android/app/src/main/kotlin/dev/ashiato/collector/Heartbeat.kt`（同）
- 文書: `docs/collector-contract.md`（生存信号の `client_version`）、`docs/handoff/ST58.md`（戻した後に書き直す・更新の後に C-01 を起き直させる）、`docs/handoff/ST54.md`（並べた C-02 の端末識別子）
- 検査: `tools/check-immutable.d/<時刻>_st51-update-history.sh`（更新の履歴の書き換えと消去を断る）
- **走っている change との重なり**（差し戻さない。後から main に入る側が rebase で解く）:
  - **ST49**（`st49-record-component-version`。下流が走っている）: requires に入れた。`device-collection` / `desktop-collection` の ST49 の Requirement を MODIFIED し、
    ST49 の試験「携帯の生存信号には版が書き込まれない」「PC の生存信号には版が書き込まれない」を反対向きに直す。`export.rs` の `HEARTBEAT_COLUMNS`・`grants.sql`・`migrations/ORDER`・`heartbeat.rs`（C-02）が同じファイル
  - **ST23**（`st23-record-erasure`）・**ST11**（`st11-health-history-permission`。`device-collection` に ADDED）・CH03・CH05: 触る Requirement と表が重ならない
- 申し送り: `docs/handoff/ST58.md`（R10・Q2・Q6）、`docs/handoff/ST54.md`（premises 9）
