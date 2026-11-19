# CH24 zip の部品を 2 から 9 に上げ、例外の表の理由を事実に合わせる

change: `openspec/changes/ch24-zip-upgrade/`（capability `data-durability`。ADDED の Requirement 1 つ・Scenario 4 本）。
依頼と完了の判定は `docs/changes/CH24.md`（受け皿の依頼 #338 から）。

Rust の zip を 2.6.1 から 9（9.0.5 以上）に上げ、例外の表（`osv-scanner.toml`）から zip の行を消す。zip を読む・書く振る舞いは変えない。
上げる前の版で書いた書き出しの zip と、上げる前の版の名前の読み方・道の判定の結果を、先に試験に固定してから上げる。

軽い路（`lane.json` の `reason: same`）。深掘りと合意は省いた。正典の既存の Requirement は変えず、ADDED だけ。

## 本人が決めたこと（議論 2026-11-19。聞き直していない）

- 何を: zip を直った版（9.0.5 以上）に上げ、GHSA-m665-v7pm-7grj・GHSA-q7v9-x93c-xqv3・GHSA-8hxf-qr8p-62x4 の例外を外す
- なぜ: 例外の理由「上げる予定: 互換範囲外の修正版待ち」は事実と違う（直った版は出ている。`premises.md`）
- どこまで: zip を使う 4 か所を新しい版に合わせ、振る舞いは変えない。書き出しの AES の zip が今と同じに読めることを確かめる。
  上げきれなければ、例外の理由を「上げる予定: 2 → 9 の上げの作業が残る」に直して期限を 90 日以内に置く（design D4: その場合は計画の直しとして人間に返す）

## 消す例外の行は 7 行

本人が名指ししたのは GHSA 3 件。同じ zip 2.6.1 の RUSTSEC-2026-0367・0368・0369・0429 の 4 行も 9.0.0 で直っていて、残すと行の検査が「勧告がもう出ない」で落ちるので一緒に消す（design D3）。
`lane.json` と `deep.md` は「3 件」と書いているが、触るファイルは同じで、軽い路の裏取りは変わらない（review R10）。消した行は検査の `== 例外の表で足した・変えた行` に出ない（足した・変えた行は `なし`）。

## 仮決め（B）と、振る舞いを残すための決定

- D1（仮）zip の features は既定のまま。反転条件: 既定の features が引く依存の許諾が許す一覧に無い・勧告が当たる・組み立てが通らない（そのときは人間へ返す）
- D2（仮）前の版で書いた書き出しは試験のファイルに囲みの行つきで埋め込む。反転条件: 64 KiB を超える
- D6 PC の更新の展開は、zip 9 の判定（`enclosed_name`）を使わず、2.6.1 と同じ名前の確かめを自前で当てる。9 は絶対パス・ドライブの前置きを相対に直して受け付け、
  `NUL`・末尾の `.`・Linux の `..\x` を新しく断るので、そのままだと受け付ける名前が変わる
- D7（仮）Takeout の項目の名前は 2.6.1 と同じ規則（UTF-8 の印があれば UTF-8、無ければ CP437）で読む。zip 9 は印を見ずに読むので、そのままだと印の無い書庫の名前（記録の `inner_path`）が変わる。CRC の合わない Info-ZIP Unicode Path（`0x7075`）を持つ書庫だけは、2.6.1 では開けず `broken_zip` だったが、9 では読める側に変わる（失うものが無いので受け入れた。review R12）。反転条件: 実物の Takeout が印を持たず名前が化けていると分かったら、別の change で決める
- D8 前の版で書いた書き出しも、台帳の照合で「書いたまま」と判定されて次の書き出しで消える
- 仮の台帳（`provisional.jsonl`）: D1・D2・D7（D6 は一度載せたが、9 の判定を使わない形にして仮でなくなり外した）

## 見つけたが直さないもの

- `update_apply.rs` の展開は zip の `extract` を使わず自前で Unix の権限を写す。zip 9 の `unix_mode()` は SUID などの bit を落として返すので（RUSTSEC-2026-0367 の直し）、
  `#[cfg(unix)]` の枝が写す権限は変わる。PC の収集アプリは Windows で動き、この枝は配る物では走らない。自前のコードは変えない（design D5）
- base の全体の脆弱性の検査は zip と無関係な 3 件（chrono・libsqlite3-sys・rustix）で落ちる。この change の範囲の外

## capability

前倒しなし。`data-durability` に ADDED の 1 Requirement。`dependency-vulnerability-scan` は例外の表（データ）を変えるだけで delta を持たない。

## tasks

14 項目（Task 1 上げる前の振る舞いの固定 5・Task 2 上げと直し 5・Task 3 例外の表 2・Task 4 予定のパスと埋め込みの確かめ 2）。人間の確認待ちは無い。

## レビュー

spec-review の指摘 16 件（`review/spec.md`。再確認で R11〜R16 が出た）: fixed 15（R1・R11・R16 → D6・R3・R12 → D7 仮・R2・R4〜R9・R13〜R15 → spec / tasks / design）、rejected 1（R10。PR 本文で名指し）。escalated 0。
R3 は再確認の席が kind を premise から technical に直した（D7 で実物の Takeout の印の有無に依る決定が無くなった）。

🤖 Generated with [Claude Code](https://claude.com/claude-code)
