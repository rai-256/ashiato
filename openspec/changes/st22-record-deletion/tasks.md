# ST22 実装タスク — 記録を消したことにできる

読む順: `deep.md`（**最優先。本人が決めた 4 件と、聞かずに決めた C1〜C10**）→ このファイル →
`specs/record-deletion/spec.md` → `specs/browsing-views/spec.md` → `design.md` → `docs/stories/ST22.md` →
`docs/handoff/ST22.md`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D10）。**名前は作成時刻 `YYYYMMDDHHMM_deletion_ledger.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。
**`record-envelope` / `derived-records` / `collection-coverage` の要件と、取り込みの口の応答・稼働状況の数え方・
`core.event` の凍結のトリガには触らない**（design D11。ST05 / ST12 / ST04 が並走している）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d db` が前提。

## 0. 規律（**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript はコメント（`// Scenario: 消した滞在を戻すと一覧に戻る`）、
  bash は `echo`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、**印の無い Scenario を FAIL にする**
- 印の名前は spec の `#### Scenario:` と**一字一句合わせる**（空白は無視される）
- **この change が持つ Scenario は 65 本**（`record-deletion` 39 本 / `browsing-views` 26 本）。
  そのうち **14 本は ST16 が既に印を持っている**（`browsing-views` の据え置き分。移動と UI の下限の Requirement を MODIFIED に足したぶん増えた）。
  **2 本は意味が変わった**（`1 日の並びは種類と時刻を持つ` に「消した」の種類が増え、`消した滞在と吸収された滞在は一覧に出ない` が
  「消した」の行を要求する）—— 既存のテストを**直す**（印を移すだけにしない）
- **件数つき検証**: `cargo test <絞り込み>` は一致するテストが 0 本でも rc=0 になる。このファイルで
  **`CT <絞り込み>`** と書いたものは、次のコマンドが rc=0 になることを指す:
  `bash -o pipefail -c 'cargo test -p ashiato-server <絞り込み> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log`
  （`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く）
- **テストは利用者で隔離する**（ST16 と同じ。`testdb::user()` で毎回新しい利用者を作る）。基準の台帳は追記のみで消せない
- **`deleted_by` に書いてよい値は 3 つだけ**（design D4。`user` / `user:cascade` / `user:late`）。
  **`rebuild:` で始まる値を書かない**（申し送り R14。書くと次の作り直しで消した滞在が戻る）
- **印を書くトランザクションは先頭で `pg_advisory_xact_lock(4816016, hashtext(<user>::text))`**（`stay_store::LOCK_KEY`。申し送り R44）
- **D7 / D8 / D9 は（仮）決め。** 反転条件は `design.md` にある。変えたらその D 番号の（仮）を外すか反転条件を書き直す
- **Q1〜Q4 は本人の決定。下流は変えない**（消す範囲・後から届いた位置・稼働状況・画面の形）
- 記録の値（座標・時刻・URL）を**ログに出さない**（製造準備 A-2）。出すのは件数・日付・種別・利用者だけ

## 1. 移行 —— 削除の台帳（design D1 / D10）

- [ ] 1.1 移行 `migrations/YYYYMMDDHHMM_deletion_ledger.sql` と `.down.sql` を足す —— `core.deletion_ledger`（D1 の列）、
  索引 2 本、UPDATE / DELETE を拒む行トリガと TRUNCATE を拒む文トリガ（**この移行専用の関数** `core.reject_deletion_ledger_change()`）。
  当て直せる形（`IF NOT EXISTS`）。`MIGRATIONS` 配列の末尾に足す。`.down.sql` の先頭に「先に戻す操作を済ませること」をコメントで書く（design の Migration Plan 4）。
  同じ移行に**削除済みの滞在を読む専用のビュー** `core.stay_erased`（design D12。識別子と時刻の範囲と印だけ。緯度経度と `raw` を載せない）も入れる。
  検証: `tools/check-migrations.sh` rc=0、`CT deletion_ledger_migration_applies_twice`（全版を 2 回当てて落ちないテスト）、
  `CT stay_erased_view_hides_coordinates`（ビューの列に緯度経度と `raw` が無いことをテストで固定する）
- [ ] 1.2 台帳が追記のみであることのテスト。
  Scenario: `台帳の行は書き換えられない` / `台帳の行は消せず、表も切り詰められない` / `台帳の行は記録の読み出しに出ない`。
  検証: `CT deletion_ledger_is_append_only`
- [ ] 1.3 `tools/check-immutable.sh` に足す（design D10。**手で維持している台本なので、足さないと新しい錠も down 移行も 1 度も当たらない**）——
  (a) `core.deletion_ledger` への UPDATE / DELETE / TRUNCATE が psql から拒まれること、(b) `…_deletion_ledger.down.sql` を戻しの逆順の先頭で当てて、
  当て直せること。検証: `tools/check-immutable.sh` rc=0 かつ出力に `deletion_ledger` の行がある
  （`bash -o pipefail -c 'tools/check-immutable.sh | tee /tmp/ci.log' && grep -q deletion_ledger /tmp/ci.log`）

## 2. 消す口（design D2 / D3 / D4 / D5。`crates/server/src/`）

削除の操作は新しいモジュール `crates/server/src/deletion.rs`（判定と SQL）と `lib.rs`（ハンドラ）に置く。
テストは `crates/server/src/deletion_tests.rs`。

- [ ] 2.1 `POST /stays/erase` を足す。滞在に `deleted_by = 'user'` の印。利用者は行から取り、添えられた利用者と違えば 404
  （`kind = "erase_user_mismatch"` のログ。値は載せない）。滞在でない記録・未知の識別子は 404、資格情報なしは 401。
  Scenario: `消した滞在は 1 日の並びの滞在として出なくなる` / `消した印は作り直しの印と区別される` /
  `滞在でない記録の識別子では消せない` / `知らない識別子を消そうとすると断られる` / `他の利用者を指定して消すことはできない` /
  `資格情報の無い消す求めは断られる`。検証: `CT erase_endpoint`
- [ ] 2.2 連鎖（design D3）—— 消す滞在の `start`〜`end`（端を含む）に入る、**その利用者の現在の基準の `sources`** の記録にも印（`user:cascade`）。
  位置以外のソースには付けない。
  Scenario: `滞在を消すとその時間の位置も読み出しから消える` / `滞在の時間の外の位置は消えない` / `同じ時間の PC のウィンドウの記録は消えない`。
  検証: `CT erase_cascade`
- [ ] 2.3 二度消し（C4）。すでに印のある行には書かない。応答は件数 0 で 200、台帳にも行を足さない。
  Scenario: `二度目の消す求めで削除時刻が動かない`。検証: `CT erase_is_idempotent`
- [ ] 2.4 まとまりと錠（design D5）。先頭で `LOCK_KEY` の錠。台帳の書き込みが失敗したら印も残らない
  （テストは台帳への INSERT を落とす形で起こす。**表の権限を剥がす・名前を変えるやり方は使わない** —— ST16 と同じ規律で、
  関数を差し替えるか、`action` の CHECK に当たらない値を通す経路で落とす）。
  Scenario: `台帳に書けないと滞在の印も付かない`。検証: `CT erase_is_atomic`
- [ ] 2.6 錠（design D5 / C6 / 申し送り R44）—— 消す操作と作り直しを同時に始めても、消したことが残り作り直しが巻き戻らないこと。
  `derived-records` の「同じ日の作り直しが同時に 2 回走っても滞在は二重にならない」と同じ形のテスト。
  Scenario: `消す操作と作り直しが同時に走っても消したことは残る`。検証: `CT erase_locks_against_rebuild`
- [ ] 2.5 台帳の行（design D1 / C2）。滞在 1 行＋位置 N 行、位置の行は原因の滞在を持つ。
  Scenario: `滞在と連鎖した位置の消去が台帳に残る`。検証: `CT erase_writes_ledger`

## 3. 作り直しと、後から届いた位置（design D5 / D6。申し送り R37）

- [ ] 3.1 消した後、触れた日（Asia/Tokyo。滞在が日をまたげば 2 日）ごとに 1 回 `stay_store::rebuild_day` を呼ぶ。**コミットの後**に呼ぶ。
  Scenario: `消した位置から作られていた滞在は一覧から外れる`。検証: `CT erase_rebuilds_day`
- [ ] 3.2 作り直しが失敗しても消したことは残す。失敗は `kind = "stay.rebuild"` に利用者・日・種別だけを出す（`/ingest` の後と同じ扱い）。
  テストは `App` の作り直しの差し替え口で失敗させる。
  Scenario: `作り直しが失敗しても消したことは残る`。検証: `CT erase_survives_rebuild_failure`
- [ ] 3.3 `rebuild_day` の中（錠を取った後・滞在を組み立てる前）で、本人が消した滞在の時間帯に入る**印の無い**基準のソースの記録に
  `user:late` の印と台帳の行（原因はその滞在）を付ける。印のある行は飛ばす（何度走ってもよい）。
  Scenario: `消した時間に後から届いた位置は行として残る` / `消した時間に後から届いた位置は読み出しに出ない` /
  `後から届いて印が付いた位置は台帳に消した行を持つ` / `消した時間の外に届いた位置には印が付かない`。
  検証: `CT late_arrival_is_marked`
- [ ] 3.4 取り込みの口の応答が変わらないこと（C9）。受理・重複の返り方を固定する。
  検証: `CT ingest_response_unchanged_after_erase`（消した時間帯へ送って `accepted=true` を確かめる。2.1 の Scenario には数えない）

## 4. 戻す口（design D2 / D5 / D6）

- [ ] 4.1 `POST /stays/restore` を足す。1 件以上の滞在の識別子を 1 つのまとまりで戻す。
  戻す対象は「**その記録について台帳に最後に書かれた行**が、この滞在を原因とする `erase`」の行（`seq` の最大で引く）。
  戻した後に触れた日ごとに `rebuild_day`。
  Scenario: `消した滞在を戻すと一覧に戻る` / `戻すと連鎖で消えた位置も戻る` / `資格情報の無い戻す求めは断られる` /
  `知らない識別子を戻そうとすると断られる`。検証: `CT restore_endpoint`
- [ ] 4.2 後から届いて `user:late` の印が付いた位置も、同じ原因なので一緒に戻る。
  Scenario: `後から届いて印が付いた位置も戻すと戻る`。検証: `CT restore_includes_late_arrivals`
- [ ] 4.3 別の操作で消えた記録は戻さない（原因で引く）。消えていない滞在を戻しても件数 0 で 200、台帳に行を足さない。
  Scenario: `別の操作で消した記録は戻らない` / `消えていない滞在を戻しても何も起きない`。検証: `CT restore_is_scoped`
- [ ] 4.4 戻した時間帯の `rebuild:erased-range` が作り直しで外れること（申し送り R19 の後段）。
  Scenario: `戻すと重なって隠れていた滞在も戻る`。検証: `CT restore_unhides_overlapping_stays`
- [ ] 4.5 戻すと台帳に `restore` の行が積まれ、`erase` の行が残ること。
  Scenario: `戻すと台帳に戻した行が積まれ、消した行は残る`。検証: `CT restore_writes_ledger`

## 5. 1 日の並びに「消した」を足す（design D7。`stay_store::day_view`）

- [ ] 5.1 `EntryKind` に `Erased` を足し、いま「移動で埋めない」ために引いている隠れた滞在の区間を**つないで 1 行**にする。
  行は `stay_ids`（その区間に重なる**本人が消した滞在**の識別子）を持つ。吸収された滞在（`rebuild:absorbed`）は行にしない。
  Scenario: `1 日の並びは種類と時刻を持つ`（**既存のテストを直す**。種類が 4 つになる） / `消した区間は戻すための識別子を持つ` /
  `消した滞在と吸収された滞在は一覧に出ない`（**既存のテストを直す**。「消した」の行が出ることを足す） /
  `隣り合う消した時間は 1 つの行にまとまる`。読みは `core.stay_erased`（design D12）越しにする。検証: `CT day_view_erased`
- [ ] 5.2 「消した」の区間を、記録なし・移動の計算から**先に差し引く**（レビュー R2 の実測を閉じる）。
  Scenario: `位置ごと消した時間は記録なしにならない` / `消した時間は消したと書かれた行で出る` /
  `消した時間の端に短い記録なしの行が生えない`（差し引いた残りを「記録が無いとみなす間隔」で測り直す） /
  `消した時間は移動の行にならない`。検証: `CT day_view_erased_not_no_record`
- [ ] 5.3 据え置きの Scenario が壊れていないこと（ST16 のテストがそのまま緑）。
  Scenario: `1 日歩き回った後、その日の滞在が一覧で出る` / `日付をまたぐ滞在は両方の日に出る` / `記録が欠けた時間は記録なしとして出る` /
  `位置の記録が無い日は丸ごと記録なしになる` / `前の日から途切れず続く記録は日の頭を記録なしにしない` / `今日の一覧はいまより後を記録なしにしない` /
  `解釈できない日付は断られる` / `資格情報の無い 1 日の並びの求めは断られる` / `滞在の間に移動の行が出る` /
  `一覧の文字はライトでもダークでも 4.5:1 を下回らない` / `日を移る操作は 24 px を下回らない` / `キーボードで移るとフォーカスの位置が見える`。
  検証: `CT stay_day_view` と `cd web && npm run test -- DayView`（ST16 のテストを走らせる。印は動かさない）

## 6. 詳細の件数の口（design D8）

- [ ] 6.1 `GET /stays/detail?stay_id=&user_id=` を足す。滞在の時間に重なる `core.event_live` の行を `logical_source` ごとに数え、
  登録簿の表示名を添える（滞在自身は除く）。未知の識別子は 404、資格情報なしは 401。
  Scenario: `詳細にその時間の記録の件数がソースごとに出る`（サーバ側の件数） / `削除済みの記録は件数に数えない`。検証: `CT stays_detail_counts`

## 7. 稼働状況を固定する（design D11。Q3）

- [ ] 7.1 消しても稼働状況と達成日数が変わらないことをテストで固定する（`coverage.rs` は変えない）。
  Scenario: `1 日の記録をすべて消しても稼働状況は記録ありのまま` / `記録を消しても達成日数は減らない`。検証: `CT coverage_counts_deleted`

## 8. 画面（design D9。`web/src/`。jsdom で測れるもの）

- [ ] 8.1 `stays.ts` —— `EntryKind` に `"erased"` を足し、`DayEntry` に `stay_ids?: string[]`。`isDayView` が新しい種類と識別子を通す
  （**形が違えば失敗として出す**を保つ）。検証: `cd web && npm run test -- stays`（新しい種類の応答を通し、壊れた応答を弾く単体テスト）
- [ ] 8.2 滞在の行を開ける行にする（`aria-expanded`。一度に 1 件）。開くと `GET /stays/detail` を 1 回叩き、件数をソースごとに出す。
  Scenario: `行を選ぶとその場で詳細が開く` / `別の行を開くと前の行は閉じる` / `詳細にその時間の記録の件数がソースごとに出る`。
  検証: `cd web && npm run test -- DayView`
- [ ] 8.3 詳細の末尾に「この滞在を消す」。閉じている行には出さない。押すと**同じ行の中**に確認（文面に一緒に消える位置の件数）。
  「やめる」で何も消さない、「消す」で `POST /stays/erase` を叩いて一覧を読み直す。`window.confirm` は使わない。
  Scenario: `閉じている行に消す操作は出ない` / `消す前に確認が出て、やめると消えない` / `確認の文面に一緒に消える位置の件数が出る`。
  検証: `cd web && npm run test -- erase`
- [ ] 8.4 「消した」の行（記録なしと同じ濃さの 1 行。詳細は開かない）と「戻す」。押すと `POST /stays/restore` を叩いて読み直す。
  Scenario: `消した行から戻すと滞在の行が戻る` / `消した行は文字で区別される`（色を外しても読み分けられる。`web/src/contrast.ts` と同じ形で固定する）。
  検証: `cd web && npm run test -- erased-row`
- [ ] 8.5 キーボードで開けること（`aria-expanded` を持つ操作対象に Enter / Space）。
  Scenario: `キーボードで詳細を開ける`。検証: `cd web && npm run test -- keyboard`

## 9. e2e（本物のブラウザ。`web/e2e/day-erase.spec.ts`）

**画面の Scenario を「人間の確認待ち」へ逃がさない。** アサートするのは**数値と経路**（実寸・可視・URL）。スクリーンショット比較は使わない。
偽データ（`SEED=normal`）は 2026-09-07 に滞在を作るので、`#/day/2026-09-07` を使う。
**消した後は同じテストの中で戻し、DB を元の状態に返す**（後続のテストが同じ DB を見る）。

- [ ] 9.1 詳細の末尾の「この滞在を消す」を実寸で測る（`boundingBox` の幅と高さが 44 以上）。
  Scenario: `詳細の末尾の消す操作は 44 px を下回らない`。検証: `cd web && npm run test:e2e -- day-erase`
- [ ] 9.2 消す → 一覧からその滞在の行が消え、同じ時間に「消した」の行が出る → 戻す → 滞在の行が戻る（`#/day/2026-09-07` のまま。画面は移らない）。
  Scenario: `確認して消すとその滞在の行が一覧から消える`。検証: `cd web && npm run test:e2e -- day-erase`

## 10. 仕上げ

- [ ] 10.1 OpenAPI に 3 本の口を足す（`utoipa`）。検証: `tools/check-openapi.sh` rc=0
- [ ] 10.2 検査 3 本。検証: `python3 scripts/check_scenarios.py . st22-record-deletion` rc=0、
  `python3 scripts/review_triage.py . st22-record-deletion` rc=0、`python3 scripts/check_chain.py .` rc=0
- [ ] 10.3 まとめて緑。検証: `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` /
  `cargo test --workspace` / `cd web && npm run lint && npm run test && npm run build && npm run test:e2e` /
  `tools/check-immutable.sh` / `tools/smoke.sh` が全部 rc=0
- [ ] 10.4 `docs/handoff/` を PR の前にもう一度読む（開始時と合わせて 2 回）。ST22 宛ての 7 件のうち、
  ST22 が扱わないもの（st19 Q1 → ST23 / st19 R20 → ST23 / st08 R4 → ST23）が `design.md` の Non-Goals に残り、
  宛先のファイル（`docs/handoff/ST23.md` / `docs/handoff/ST33.md`）に置かれていることを確かめる。
  検証: `grep -c 'st22-record-deletion' docs/handoff/ST23.md docs/handoff/ST33.md` がどちらも 1 以上

## 人間の確認待ち

**無し。** 画面の Scenario は 9 章の e2e（本物のブラウザ）が担保し、サーバの Scenario は Rust のテストが担保する。
機械が再現できない物理（ロック・電池・本物の GPS・時間そのもの・実機）に当たるものが、この Story には無い
（消す・戻すはどちらもサーバと画面の中で完結する）。確認バッチの手順書が Story ごとに 1 問聞く
「触ってみて違和感は無かったか」だけが人間に渡る。
