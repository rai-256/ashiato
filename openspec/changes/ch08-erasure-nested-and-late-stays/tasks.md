# CH08 実装タスク — 本文ごと消した時間に入る消した滞在の行・作り直しで隠れた重なる滞在の本文の消去・戻す口の断り・起動時の手当て

読む順: `deep.md`（**最優先。本人が決めたこと**。Q1〜Q5 はどれも本人の選択）→ このファイル → `specs/record-deletion/spec.md` → `specs/browsing-views/spec.md` → `design.md` →
`defaults.md`（C1〜C7）→ `premises.md`（探り A1〜B2）→ `docs/changes/CH08.md`（壊してはいけないもの）→ `CLAUDE.md`。
ST23 の正典と設計は `openspec/specs/record-deletion/spec.md`・`openspec/specs/browsing-views/spec.md`・`openspec/changes/archive/2026-10-30-st23-record-erasure/design.md`。

触るのは `crates/server/src/`（`stay_store.rs`・`deletion.rs`・`lib.rs`・新しい試験 2 本）・`docs/openapi.json`（生成物）・`web/src/`（`DayView.tsx`・`stays.ts`・`messages.ts`）・
`web/e2e/`（新しい `ch08-nested.spec.ts` と、文面の変わる `st23-purge.spec.ts` の 1 本）。**移行は足さない**（design D5）。
検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` と `tools/db-roles.sh` が前提（`docs/testing.md` §8）。

## Global Constraints（規律。**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript はコメント（`// Scenario: 作り直しで吸収された重なる滞在の本文も消える`）。
  `scripts/check_scenarios.py . ch08-erasure-nested-and-late-stays` が spec の全 Scenario と突き合わせ、**印の無い Scenario を FAIL にする**。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change が足す Scenario は 48 本**（`record-deletion` の MODIFIED に足した 28 本 + ADDED の 8 本 + `browsing-views` の MODIFIED に足した 12 本）。人間の確認待ちは 0 本。
  MODIFIED で写した既存の Scenario は既存の印を生かす（名前を変えない）。THEN を書き換えたのは「後から届いて本文を消した件数が行に出る」の 1 本だけで、その e2e を Task 6 で直す。
  **書き直す ST23 の既存の試験は 3 本だけ**（design D1 の表）: `st23-purge.spec.ts` の文面（6.1）・`stay_tests.rs` の `DayEntry` のリテラル（5.1）・`purge_late_tests.rs` の入れ子の B を戻す試験（3.1）
  THEN に主張が 2 つ以上ある Scenario は、試験が主張を 1 つずつ assert する
- **件数つき検証**: 各項目の「検証:」は、`cargo test` を repo の根から `scripts/quiet-run` で走らせるもの（rc=0）と、`-- --list` で一致する試験の本数の下限を見るもの（rc=0）の 2 つを並べる。
  `cargo test <絞り込み>` は一致するテストが 0 本でも rc=0 になるので、下限の方を必ず走らせる。絞り込みは 1 つずつ書く。一時ファイルは `"$TMPDIR"` の下に書く（`/tmp` の固定名に書かない）
- **試験の関数名の前置きは design D9 のとおり**: `purge_hidden_`（Task 1）/ `purge_remedy_`（Task 2）/ `purge_restore_`（Task 3）/ `purge_visible_`（Task 4）/ `purge_nested_`（Task 5）。
  ST23 の既存の前置き（`purge_scope_`・`purge_api_`・`purge_late_`・`purge_day_` など）と重ならない
- **深掘りの答え（`deep.md` が正本）**: 変えるのは Q7・Q8・手当てと、それが必ず変える ST23 の振る舞いだけ（Q1）/ 戻しても一覧に戻らない B は画面も口も戻さない（Q2）/
  「後から N 件も本文ごと消した」の 1 つの数（Q3）/ 一覧に出ている重なる滞在があるときだけ確認に添える（Q4）/ 起動のたびに Q8 と同じ処理で手当て（Q5）
- **聞かずに決めた既定（`defaults.md`）と D の対応**: C1 → D2・D5・D7 / C2 → D2・D5 / C3 → D3 / C4 → D2 / C5 → D6・D8 / C6 → D1・D9 / C7 → D3。
  **D2〜D8 は（仮）決め。** 反転条件は `design.md`。変えたらその D 番号を書き直す
- **ST23 の振る舞いで変えてよいのは 4 つだけ**（`docs/changes/CH08.md`「壊してはいけないもの」）: 戻す口（Q2）・作り直しで隠した重なる滞在の本文の消去（Q8）・L の印の付け替え（C7）・1 日の一覧（Q7・Q3・Q4）。
  一覧に出ている重なる滞在は消さない（ST23 D17）。本文ごと消す範囲（ST23 D2）は広げない。端が触れるだけは重なりにしない（C1）
- **ST23 の Non-Goals を入れない**（Q1）: まとまった「消した」の行の「本文ごと消す」が先頭だけを指す（R10・R23）・夜の書き直しの範囲（R16）・一部重なる B の前の版の本文
- **本文を消す SQL は `deletion.rs` の `erase_bodies` だけが持つ**（ST23 D3）。作り直しの消去も起動時の手当ても `deletion::purge_hidden_overlap` を通す（規則を移行や別の SQL に書き写さない。Q5）
- **ログに出すのは件数・種別・SQLSTATE だけ**（記録の識別子・座標・原文・時刻を出さない。製造準備 A-2）
- **画面の Scenario は `web/e2e` で担保する**（`docs/testing.md` §4.5）。アサートは数値と経路（実寸・可視・要素の数・文面の数字・行の並び）。スクリーンショット比較は使わない
- **走っている change と同じファイル**（proposal の Impact）: ST49 が `deletion.rs`・`stay_store.rs` に版の列を足す。CH05 が `lib.rs` の試験のモジュールの並びに 1 行足す。CH03 が試験の一時フォルダの作り方を変える。
  **先に main に入ったら rebase で解き、`docs/openapi.json` は手で混ぜずに生成し直す**（`cargo run -q -p ashiato-server --bin openapi > docs/openapi.json`）

## Task 1: 作り直しが隠した重なる滞在の本文を同じまとまりで消す（design D2 / D3。spec「本文を消した記録は戻せず、作り直しも戻さない」「本文ごと消す操作は追記のみの台帳に 1 操作 1 行で残る」）

- [x] 1.1 `deletion.rs` に `purge_hidden_overlap(tx, …)` を置く（design D3 の 1〜5 の順。`kind = 'late'`・`parent_id` = 元の操作・`stays = 1`・前の版の数・記録ごとと前の版ごとの消去の台帳・L と前の版の範囲の行・`erase_bodies`・
  `user:purged` への付け替えと削除の台帳の `erase` の行）。`stay_store.rs` の `load_existing` が本文を消した滞在の操作の識別子を持ち、`rebuild_day` が `assign` の後に design D2 の 1〜3 で L を見分け、
  (a) 吸収される L は `absorb` の代わりに、(b) 引き継いで隠れる L は引き継がせずに新しい行を隠して作り、どちらも L を `purge_hidden_overlap` に渡す。元の操作は design D3 の順で選ぶ。
  Scenario: `作り直しで吸収された重なる滞在の本文も消える` / `作り直しで消した時間帯と重なるとして隠れた重なる滞在の本文も消える` / `作り直しで隠れた重なる滞在の前の版の本文も消える` /
  `端が触れるだけの滞在は作り直しで隠れても本文が残る` / `範囲の行が無い本文を消した滞在の日に隠れた滞在は本文が残る` / `作り直しの前から隠れていた滞在の本文は作り直しで消えない` / `一覧に出ている重なる滞在は隠れるまで本文が残る` /
  `重なる滞在の本文が消せないと作り直しは何も残さない` / `作り直しで本文を消した滞在は消した時間帯を広げない` /
  `作り直しで本文を消した滞在は後から本文を消した操作として台帳に残る` / `作り直しで本文を消した滞在の時間が範囲の行に残る` / `2 つの本文ごと消した時間帯と重なる滞在は重なりの大きい方の操作に結ばれる`。
  （「消せないと」は ST23 の `erase_with_action` と同じく試験用の差し込み口で消去の台帳の CHECK 違反を起こし、作り直しの後に L の印が `NULL`・本文が残ることを DB から読む。
  場面は新しい `crates/server/src/purge_hidden_tests.rs` に作る。一覧に出ている L は「作り直しが落ちて残った滞在」として、位置を消した後に印を付けずに置く —— ST23 D17 の場面）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server purge_hidden_'` rc=0、
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; test "$(cargo test -p ashiato-server purge_hidden_ -- --list 2>/dev/null | grep -c ": test$")" -ge 12'` rc=0
- [x] 1.2 ST23 の作り直し・本文ごと消す・後着の試験が変わらず通る（`rebuild_day` の書き換えで壊していない）。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server purge_'` rc=0、
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server stay_'` rc=0、
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; test "$(cargo test -p ashiato-server stay_ -- --list 2>/dev/null | grep -c ": test$")" -ge 1'` rc=0

## Task 2: 起動時の手当て（design D5。spec「本文ごと消した後に作り直しで隠れたのに本文が残る重なる滞在は、サーバの起動のたびに本文を消す」）

- [x] 2.1 `deletion.rs` に `purge_hidden_overlap_remedy(&pool) -> usize`（残った件数）を置く。design D5 の 4 つの条件（時刻を比べない）で対象を探し、利用者ごとに錠を取ったトランザクション（`lock_timeout` 5 秒）で
  Task 1 の `purge_hidden_overlap` に渡し、失敗を飲み込んで `warn`（`kind = "purge_remedy"`・残った件数・SQLSTATE だけ）を出すところまでこの関数が持つ。
  `lib.rs` の `run()` は、`purged_without_span` のログの隣・待ち受けの前でこの関数を 1 回呼ぶだけ（`?` で伝えない）。
  Scenario: `起動すると本文ごと消した後に隠れて本文が残った重なる滞在の本文が消える` / `起動を繰り返しても台帳の行は増えない` / `本文ごと消す前から隠れていた端の滞在は起動しても本文が残る` /
  `本文ごと消した後に初めから隠して作られた滞在は起動しても本文が残る` / `本文ごと消した後に戻されて作り直しで隠れた消した滞在は起動しても本文が残る` /
  `起動の手当てのログは件数だけを持つ` / `対象が無ければ起動の手当ては何も書かない` / `手当てが失敗しても起動は止まらず、次の起動で消える`。
  （「前の状態」は CH08 の前の作り直しを再現して作る: 本文ごと消した後、一覧に出ている L に `rebuild:absorbed` の印を直に付ける。「起動する」は `run()` が呼ぶのと同じ関数を呼ぶ。
  「失敗」は試験用の差し込み口で 1 回目の消去を落とし、関数が panic も `Err` も返さず残った件数 1 を返し、もう一度呼ぶと 0 になり本文が空であることを見る。
  ログは `tracing` の捕まえ口で行を読み、件数があり、滞在の識別子・時刻・緯度経度の文字列が無いことを見る）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server purge_remedy_'` rc=0、
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; test "$(cargo test -p ashiato-server purge_remedy_ -- --list 2>/dev/null | grep -c ": test$")" -ge 8'` rc=0、
  `bash -c 'grep -q "deletion::purge_hidden_overlap_remedy(&pool)" crates/server/src/lib.rs'` rc=0（`run()` から呼び出しの形で呼ばれている）

## Task 3: 戻す口の断り（design D4。spec「消したものを戻せる」）

- [x] 3.1 `stay_store.rs` の `rebuild_day` の本体を、呼び出し側のトランザクションで走る `rebuild_day_in(tx, …)` に切り出す（`rebuild_day` はそれを包むだけ。振る舞いは変えない）。
  `deletion.rs` の `restore` が、求めた滞在に本文ごと消した時間帯と重なる（端が触れるだけは除く）ものがあるときだけ、印を外した後に同じトランザクションで戻した日を作り直し、
  重なる滞在のどれかが `deleted_at IS NULL` で残っていなければ巻き戻して `RestoreError::NotRestorable` を返し、その作り直しが落ちたら巻き戻して `RestoreError::RebuildFailed` を返す（design D4 の 4）。
  `lib.rs` の `stays_restore` は 409 `not_restorable` / 503 `restore_rebuild_failed` を返し、作り直し済みの日を作り直さない。
  `purge_late_tests.rs` の `purge_late_archive_row_hidden_by_another_stay_keeps_that_stay_restorable` を design D1 の表のとおり置き直す（B を A からはみ出して滞在になる形に。主張は変えない）。
  重ならない求めはいまのとおり（commit の後に呼び出し側が作り直す）。`docs/openapi.json` を生成し直す。
  Scenario: `本文ごと消した時間にすっぽり入る消した滞在の戻す求めは断られる` / `はみ出しが滞在にならない消した滞在の戻す求めは断られる` / `はみ出しが滞在になる消した滞在は戻せる` /
  `戻せない滞在を含む戻す求めは何も戻さない` / `戻せない滞在の断りは本文を消した記録の断りと区別できる` / `戻せるかを確かめる作り直しが落ちると何も戻さない`。
  （場面は premises.md の探り A1〜A3 と同じ: P 10:00–11:00 の位置 5 分おき・B のはみ出しの位置。HTTP の口を叩き、DB から B と位置の印と本文を読む）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server purge_restore_'` rc=0、
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; test "$(cargo test -p ashiato-server purge_restore_ -- --list 2>/dev/null | grep -c ": test$")" -ge 6'` rc=0、
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server purge_late_'` rc=0（置き直した 1 本を含む）、
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server deletion_tests'` rc=0（ST22 の戻す口の試験が緑のまま）、
  `bash -c 'cargo run -q -p ashiato-server --bin openapi | diff -q - docs/openapi.json'` rc=0

## Task 4: 確認の口の「一覧に出ている重なる滞在」（design D7。spec「本文ごと消す前に、消える件数と残るものを示す」）

- [x] 4.1 `deletion.rs` の `PurgeScope` と確認の口の応答（`lib.rs`）に `visible_overlaps` を足す（design D7 の数え方）。`counts`・消す口の `expected` の比べ・範囲は変えない。`docs/openapi.json` を生成し直す。
  消す口は `visible_overlaps` を比べない（design D7）。
  Scenario: `確認は一覧に出ている重なる滞在の件数を消える件数の外で返す` / `端が触れるだけの一覧に出ている滞在は重なる滞在に数えない` / `確認の後に一覧に出ている重なる滞在が増えても本文ごと消せる`
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server purge_visible_'` rc=0、
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; test "$(cargo test -p ashiato-server purge_visible_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0、
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server purge_api_'` rc=0、
  `bash -c 'cargo run -q -p ashiato-server --bin openapi | diff -q - docs/openapi.json'` rc=0

## Task 5: 1 日の並びの入れ子の項目と後からの件数（design D6。spec `browsing-views`「1 日の並びを読み出せる」）

- [x] 5.1 `stay_store.rs` の `DayEntry` に `nested`（`NestedEntry`: `start`・`end`・`stay_ids`・`purged`）を足し、`day_view` が design D6 の 1〜4 で入れ子の項目を作る
  （本文を消していない B は差し引いて空になる断片、本文ごと消した B はほかの本文ごと消した区間の和に覆われるもの。親は重なりの大きい区間 → 始まりの早い区間。入れ子にした本文ごと消した B の後からの件数は親に足す）。
  Task 1 の `kind = 'late'`・`stays = 1` の行は、いまの後からの件数の和にそのまま入る。`web/src/stays.ts` の型と検査に `nested` を足す。`docs/openapi.json` を生成し直す。
  `stay_tests.rs` の `stay_day_view_empty_day_is_all_no_record` の `DayEntry` のリテラルに `nested: None` を足す（design D1 の表。主張は変えない）。
  Scenario: `本文ごと消した区間はすっぽり入る消した滞在を入れ子の項目に持つ` / `本文ごと消した入れ子の滞在は別の区間にならない` / `作り直しで隠れて本文を消した滞在も後から本文を消した件数に入る` /
  `入れ子の滞在の後から本文を消した件数は親の区間に足される` / `時間が同じ 2 つの本文ごと消した滞在は先に消した方が区間になる`
  （新しい `crates/server/src/purge_nested_day_tests.rs`。`day_view` の応答を読む。日の境・2 つの区間に跨る・まとめの規則も同じ前置きで 1 本ずつ試験を足す —— 画面の Scenario の土台）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server purge_nested_'` rc=0、
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; test "$(cargo test -p ashiato-server purge_nested_ -- --list 2>/dev/null | grep -c ": test$")" -ge 8'` rc=0、
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server day_'` rc=0（ST16 / ST22 / ST23 の 1 日の並びの試験が緑のまま）、
  `bash -c 'cargo run -q -p ashiato-server --bin openapi | diff -q - docs/openapi.json'` rc=0、`bash -c 'cd web && npx vitest run'` rc=0

## Task 6: 画面 —— 入れ子の行・入れ子の行の口・戻す口の断り・後からの件数・確認の一文（design D8。spec の画面の Scenario）

- [x] 6.1 `web/src/DayView.tsx` が `nested` を「本文ごと消した」の行のすぐ下に字下げした行で出し（design D8 の表）、入れ子の行には「本文ごと消す」（44×44 px 以上）と「本文ごと消した時間の中なので戻せません」だけを出す。
  本文ごと消した入れ子の行は「本文ごと消した 始まり – 終わり」で口を出さない。戻す口の 409 `not_restorable` にはその行に「戻しても一覧に戻らないため、戻せません」を出し、行を「消した」のまま残す。
  `messages.ts` の `purgedLate` を「後から N 件も本文ごと消した」にし、確認に `visible_overlaps > 0` のときだけ一文を添える。
  `web/e2e/st23-purge.spec.ts` の「後から届いた 2 件も本文ごと消した」を読む 1 本を、spec の THEN（「後から 2 件も本文ごと消した」）に合わせて直す（ほかは変えない）。
  Scenario（新しい `web/e2e/ch08-nested.spec.ts`）: `すっぽり入る消した滞在の行には戻すが出ない` / `戻しても一覧に戻らない消した行の戻すは断られた旨が出る` /
  `作り直しで隠れて本文を消した滞在も後からの件数に入る` / `すっぽり入る消した滞在の行から本文ごと消せる` / `すっぽり入る消した滞在の行の本文ごと消すは 44 px を下回らない` /
  `確認に一覧に出ている重なる滞在が添えられる` / `一覧に出ている重なる滞在が無ければ確認に添えない` /
  `すっぽり入る消した時間は本文ごと消した行の下に出る` / `すっぽり入る消した時間が 2 つあれば始まりの順に並ぶ` / `重なるすっぽり入る消した時間は 1 つの行にまとまる` /
  `2 つの本文ごと消した時間に跨る消した時間は重なりの大きい方の下に出る` / `日の境で切ると本文ごと消した時間に入る消した時間も下に出る` / `すっぽり入る消した滞在を本文ごと消した後も同じ行の下に残る` /
  `本文ごと消したすっぽり入る時間はまとめずに 1 行ずつ出る`。
  （場面は `st23-purge.spec.ts` と同じく API で位置を送り・消し・本文ごと消して作る。「すぐ下」は DOM の並びで親の行の次の兄弟であること、字下げは `getBoundingClientRect().left` が親より大きいこと、44 px は幅と高さ。
  「作り直しで隠れて」は、一覧に出ている重なる滞在を作るために位置を消した後の作り直しが走る前の状態を API の並びで作れなければ、試験用の DB の差し込み（`st23-purge.spec.ts` と同じ足場）で印の無い L を置き、日の作り直しを口から呼ぶ）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; cd web && npm run test:e2e -- ch08-nested.spec.ts'` rc=0、
  `bash -c 'test "$(grep -c "// Scenario:" web/e2e/ch08-nested.spec.ts)" -ge 14'` rc=0、
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; cd web && npm run test:e2e -- st23-purge.spec.ts day-erase.spec.ts'` rc=0（ST22 / ST23 の画面が緑のまま）

## Task 7: 仕上げ —— 全体の試験・Scenario の突き合わせ・共有の台本

- [ ] 7.1 server の試験の全体と、Scenario の印、共有の台本の末尾への書き足しが無いことを確かめる。書き直した ST23 の試験が design D1 の表の 3 本だけであることを PR の本文に書く。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server'` rc=0、
  `python3 scripts/check_scenarios.py . ch08-erasure-nested-and-late-stays` rc=0、`bash tools/check-shared-appends.sh` rc=0、
  `openspec validate ch08-erasure-nested-and-late-stays --strict` rc=0
- [ ] 7.2 既存の縦串が緑のまま（CH08 は縦串の節を足さない。CH08 の振る舞いは Task 1〜6 の試験が担保する）。
  検証: `scripts/quiet-run smoke -- tools/smoke.sh` rc=0

## 人間の確認待ち

**無し。** 画面の Scenario は Task 6 の e2e（本物のブラウザ）が担保し、サーバの Scenario は Rust の試験が担保する。
機械が再現できない物理（ロック・電池・本物の GPS・時間そのもの・実機）に当たるものが、この change には無い。
確認バッチの手順書が change ごとに聞く「触ってみて違和感は無かったか」で、本人は入れ子の行と戻す口の断りを触る（`docs/changes/CH08.md`「完了の判定」）。
