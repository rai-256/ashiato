# ST47 YouTube のマイアクティビティを視聴・検索・その他に分けて取り込む（下流・実装）

change: `openspec/changes/st47-youtube-myactivity-split/`。capability `external-ingestion`（書庫の取り込み）。

## 深掘りに積んだ未決（A）

**なし**（`deep.md` の未回答 0。第 1 回 6 問・第 2 回 4 問はすべて本人が答えた）。下流で人間に返した問いは無い。

## 仮決め（B）と Ruling

design の（仮）と反転条件:

- **D2（仮）** 製品の名前を持たないファイル（製品側の古い形と ST12 の合成の材料）は `YouTube` として読み、形の `products` は `[]`（`["YouTube"]` の形とは別に印を置く）。
  反転条件: 製品の名前を持たない YouTube の履歴が本物に無いと分かったとき（その枝を消し、材料に `products` を足す）。
  混ざったファイルの扱い（その他へ入れ、一覧に件数を出す）は第 2 回 Q10 で本人が決めた
- **D4（仮）** 評価と言語の決まり文句の表。日本語の表（評価は先頭の `高く評価しました ` / `低く評価しました `、視聴・検索は末尾）は第 2 回 Q9 で本人が決めた。**英語の表（`Liked ` / `Disliked ` / `Watched ` / `Searched for ` の先頭）は本物の書き出しで確かめていない**。
  反転条件: 本物の書庫で評価の題名の形が表と違うとき（表を直し、解析器の版を上げる。D9）。英語の書き出しは `languages: ["en"]` の新しい形として必ず確認待ちになるので、印を置く前に `destinations.rating` を見られる（0 なら表が外れている合図）。
  **日本語の側は本物の書庫で機械が確かめた**（code-verify R10）: 手元の本物の書庫を `shape_for_file` に通すと `products ["YouTube"]`・`languages ["ja"]`・視聴 89,368・検索 5,643・その他 23,884（うち広告 20,093・評価 196）

ledger の Ruling（全 Task の `progress.md` と report の Ruling の節の全部。`progress.md` は 1 つも無く、Ruling は Task 2 の report の 1 件だけ）:

- Task 2: 題名の言語は表の上から（ja → en）最初に当たった言語 1 つにだけ数える — 「Liked by the Sea を視聴しました」が ja と en の両方に当たると en が立ち、spec の Scenario（日本語の書き出しでは視聴に入る）が成り立たないため — 間違っていれば、両言語の混在ファイルで英語の題名の言語判定が ja に寄る（評価の取りこぼし。形の一覧の `rating` の件数で気づける）。
  Task 2 の review の指摘を受けて design D4 に書き込み済み（反転条件: 1 つの題名が複数の言語の表に正当に当たる言語が表に加わったとき）

parked: なし。

## 確認バッチの DB の印を確かめる手順（design の Risks の 1 つ目）

確認バッチの作業用の DB に、旧い形（`{"kind":"YouTubeWatch","products":[]}` = `f3e578ff…`）で入った YouTube の行があるかもしれない（Q5。本人は覚えていない）。旧い形の印は D5 で効かなくなるので、この Story の後に入る行は新しい規則でしか入らない。
確かめる手順（共有の資源を変えるので、本人か確認バッチの準備の段でやる）:

1. docker volume `1dd5c7e5…`（更新 2026-10-12 23:31。確認バッチ 2 回目の日）を別の名前の volume に写して PostgreSQL を起こす
2. `select count(*) from core.archive_shape_confirmation where shape_hash like 'f3e578ff%';` を引く
3. 1 以上なら、その DB の `c03-youtube-watch` には検索・広告・その他が混ざっている（作業用なので捨ててよい）。本人の運用の DB に同じ印が見つかったときだけ、扱いを深掘りで問う

## Task

7 Task・14 項目のうち 13 項目が `[x]`（グラフの `mark`）。残りは正常な残り:

- 8.1 —— 人間の確認（realdata）。**件数の照合は code-verify R10 で機械が済ませた**（上の D4 の実測。tasks.md は凍結なので 8.1 の本文は古いまま）。確認バッチで人間に見てもらうのは、稼働状況の書庫のソースの格子に視聴・検索・「マイアクティビティ: YouTube」の 3 本の最終日が出るかと、違和感だけ
- tasks.md 2.2 の本文（「日本語は末尾」）は final review R1 の訂正（日本語の評価は先頭一致）の前のまま。凍結のため直していない。正しいのは design D4
- tasks.md 1.1 の本文（「MIGRATIONS の末尾に載せる」）は main #97 の前のまま。#97 の後の正しい置き場は `migrations/ORDER` の末尾（89d57287 で移した）。凍結のため直していない

## 検証（`evidence.jsonl`。`scripts/verify-run` が走らせて記録した最新の行）

1.1（`tools/check-migrations.sh`）/ 3.2（`tools/smoke.sh`）/ 6.1 / 7.1（fmt・clippy `-D warnings`・`cargo test --workspace`）/ 7.2（`check_scenarios.py`）/
7.3（`check_chain.py`・`openspec validate --strict`・`check-migrations` / `check-openapi` / `check-boundaries` / `check-immutable` / `check-licenses`）—— どれも PASS（exit 0）。
**7.1 の最新の PASS は 72005b0c**（finish の fix の後。`cargo test --workspace` は server 840 本・collector-windows 133 本ほか全緑）。3.2・6.1 は cf8597e9、7.2・7.3 は d51755c。7.3 の FAIL の 1 行と 7.1 の FAIL の行は、同じ head で取り直して PASS。
3.2・6.1（smoke）は 72005b0c で取り直していない —— finish の fix は Rust の読み直しの経路と試験だけで、smoke が見る道具・移行・API の出力は変わっていない。

`CT` 型（2.1・2.2・3.1・4.1・5.1）は evidence に行が無い。code-verify 第 2 回が HEAD 6e3b0adf で全部走らせ、`-- --list` の本数が下限以上で通ることを確かめた（`review/code.md` の表）。
finish の fix の後: `cargo test -p ashiato-server archive` 188 passed、`cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0、`review_triage.py` OK。

## レビュー

- Task ごと: 7 Task の task reviewer（指摘は `.superpowers/sdd/st47-task-<N>/task-<N>-findings.md`）
- whole-branch review: R1〜R4。fixed 4（R1 日本語の評価は先頭一致 / R2 申し送りの D20・R17 を delta に / R3 言語の当たらない形は印があっても読まない / R4 design の決定の一覧）
- code-verify: R5〜R10。fixed 6（R5 走査の輪の上の取り直し / R6 広告を評価より先に見る順 / R7 行き先ごとの payload / R8 SQL 側の断り / R9 この本文の申し送りの見出し / R10 D4 仮の実測）。
  R5〜R8 は、生き残っていた変異（M06・M08・M17・M02）を作業ツリーで当てて新しい試験が落ちる（rc=101）ことを確かめた
- rebase 後の whole-branch review: R11〜R16（`review/code.md`）
- code-verify 第 2 回: R17〜R20。
  - **R17 fixed（コードの直し）**: 版の読み直しが確認待ちを含む書庫の途中で落ちると、輪の中で先に書いた確認待ちの台帳の行が書庫を次の周の対象から外し、二度と読み直されず失敗も数えられなかった。台帳の行を成功の経路の末尾で書くよう直し、再現を試験にした
  - R19・R20 fixed（試験の追加）: 版の読み直しの経路の言語の断り / 最初の製品の名前 / 評価は `watch?v=` を持つものだけ / 英語の `Searched for `。変異 N11・N18・N22・N23 と R17 を戻す変異で、新しい試験がどれも rc=101
  - R18 rejected: PR の head と本文が古いのはブランチの欠陥でなく、`publish` がこの本文と HEAD で書き直す
  - fix の後の scoped re-review: 全件 ADDRESSED、新しい破損なし

## 申し送り（`docs/handoff/ST47.md`。3 本とも処置済み）

- **2026-10-13-st33-merge-consistency.md C4** —— 分けた論理ソースが滞在の削除の連鎖（FR-50）から漏れない: `stay_store` の `c03-myactivity-` 前置き ＋ 見張り `every_archive_logical_source_is_classified_as_location_or_item`、試験 `archive_youtube_location_marks_only_the_located_other_item`
- **st14-outage-alert D20** —— 正典 `external-ingestion` の 60 日の文を通知の 75 日に合わせる: dc8ecc1e（MODIFIED で写して文だけ直した）
- **st14-outage-alert R17** —— 書庫のソースの格子の並びに、集計の格子が Must と書庫の間に入ることを書き足す: dc8ecc1e（`web/src/__tests__/archive-order.test.tsx` で固定）

未処置の申し送り: なし。範囲外の観察（re-review）: `archive_youtube_last_event_is_reported_per_logical_source` と `archive_erased_window_persistent_marking_failure_is_ledgered_and_throttled` が
共有の開発 DB で並走させたとき 1 回落ちた（単独では通る）。flaky の疑いとして記録に留める。`tools/verify-prep.sh` に本物の書庫の形の一覧を出させる直し（R10 の提案）は Story をまたぐ道具の変更なので入れていない。
