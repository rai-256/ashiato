# CH24 design — zip の部品を 2 から 9 に上げる

## Context

- zip は 2 つの crate が `zip = "2"`（既定の features）で使い、`Cargo.lock` は 2.6.1 の 1 つだけ
- 使う所は 4 か所: サーバの `archive/open.rs`（Takeout の書庫を `ZipArchive` で読み、ディスクへ展開しない）・`export/dump.rs`（`ZipWriter` で
  deflate + AES-256 + 注記）・`export/ledger.rs`（置き場の書き出しの注記を読んで台帳と照合）・PC の収集アプリの `update_apply.rs`（指紋を照合した配布物を
  `by_index` と `enclosed_name` で自前に展開し、Unix では権限を写す）。試験は 7 つのファイルが `ZipWriter` / `ZipArchive` を直に使う
- 例外の表の zip の行は 7 つで、どれも理由は「上げる予定: 互換範囲外の修正版待ち」（`premises.md`）
- base の全体の脆弱性の検査は zip と無関係な 3 件（chrono・libsqlite3-sys・rustix）で既に落ちる。PR の検査（`--base`）は rc=0

## Goals / Non-Goals

- Goals: zip を 9.0.5 以上へ上げる・zip の行を例外の表から消す・振る舞いを変えない・前の版で書いた書き出しが読めることを試験で固定する
- Non-Goals: zip と無関係な勧告（全体の検査を落としている 3 件）を直す・例外の表のほかの行を変える・自前の展開を zip の `extract` に置き換える・
  `update_apply.rs` の権限の写し方を変える（下の D5）・features を絞って依存を減らす

## Decisions

### D1（仮）. zip の features は既定のまま

`zip = "9"`（既定の features）にする。`default-features = false` で aes-crypto と deflate だけに絞れば依存は減るが、Takeout の zip を読める
圧縮の方式が今より狭くなりうる（2.6.1 の既定は bzip2・lzma・zstd なども読む）。振る舞いを変えない約束なので、絞らない。
反転条件: 既定の features が引く依存の許諾が `tools/check-licenses.sh` の許す一覧に無い・その依存に勧告が当たる・組み立てが通らない。
そのときは features を絞る前に人間へ返す（読める方式が狭まるのは振る舞いの変化）。

### D2（仮）. 前の版で書いた書き出しは、試験のファイルにバイト列として埋め込む

上げる前の木（zip 2.6.1）で、試験の DB に決まった記録を置いて書き出しを 1 回走らせ、そのファイルのバイト列・項目ごとの SHA-256・書き出しの id・
合言葉を `crates/server/src/export_file_tests.rs` に定数（`CH24_PREVIOUS_EXPORT` など）として置く。別の材料のファイル（`crates/server/tests/fixtures/`）に置かないのは、
予定のパス（`lane.json`）の外だから。埋め込みを入れた commit の `Cargo.lock` が zip 2.6.1 であることを tasks 1.2 が git で確かめる
（上げた後に作り直すと互換を見なくなる）。
AES の形は zip の部品の読み方を通さず、中央ディレクトリと追加の欄 `0x9901`（vendor version・`AE` の印・強さ・実の圧縮方式）を試験の中でバイト列から読む。
手元に AES の zip を読める別の道具（7-Zip など）が無いため、別の実装で読む確かめは置かない。
反転条件: 埋め込みが大きすぎて試験のファイルが読めなくなる（目安 64 KiB を超える）なら、書き出す記録を減らす。

### D3. 例外の表から消すのは zip の 7 行

本人が名指ししたのは GHSA 3 件だが、同じ zip 2.6.1 の RUSTSEC-2026-0367・0368・0369・0429 の 4 行も 9.0.0 で直っていて、上げた後は当たらない。
残すと、行の検査が「勧告がもう出ない」で落ちる（`dependency-vulnerability-scan` の正典）。同じ部品の同じ上げの帰結で、範囲は広げていない。

### D4. 上げきれないときは、自分で代わりに切り替えない

本人は「上げきれないときは、例外の理由を事実（`上げる予定: 2 → 9 の上げの作業が残る`）に直して期限を 90 日以内に置く」と決めている。
ただしその場合は完了の判定（1〜4）が変わり、tasks の検証も成り立たないので、Task agent は切り替えずに、上げきれない理由（落ちた試験・組み立ての誤り・
許諾・新しい勧告）と本人の決めた代わりを添えて返す。グラフが計画の直しとして扱う。
上げきれないとは: 予定のパスの中の直し（4 か所と 7 つの試験の、API に合わせる直し）で Task 2 の検証が通らないこと。

### D5. `update_apply.rs` の展開は自前のまま

RUSTSEC-2026-0367・0368・0369 は zip の `extract` の振る舞い（SUID の保持・道の外への symlink・symlink 越しの権限）の勧告。
`update_apply.rs` は `extract` を使わず、`enclosed_name` で道の外を断り、`#[cfg(unix)]` で `unix_mode` を写す。上げても自前の展開の振る舞いは変わらず、
SUID の保持の直しも効かない。PC の収集アプリは Windows で動くので、`#[cfg(unix)]` の枝は配る物では走らない。振る舞いを変えない約束なので触らず、
PR 本文に挙げる（直すなら別の change）。

## Risks

- zip 9 の API の変化（例: `set_comment` が結果を返すようになった）は組み立てで分かる。読み方の意味の変化（項目の名前・ディレクトリの判定・`unix_mode`）は
  既存の試験（書き出し・Takeout・PC の更新）が見る
- PC の収集アプリは Windows で動く。Linux で走る試験に加え、PR の CI の Windows 向けの clippy と `runtime_windows` が見る（tasks 2.4）
