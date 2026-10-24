# ST23 実装タスク — 本文を本当に消せる（消した滞在を本文ごと消す口と確認・台帳・範囲・後着・派生の門・夜の書き直し・画面）

読む順: `deep.md`（**最優先。本人が決めたこと**。第 1 回は推奨で先に書いてある —— 答えが推奨と違えば、その問いだけ spec と design が直っている）→ このファイル →
`specs/record-deletion/spec.md` → `specs/browsing-views/spec.md` → `design.md` → `defaults.md`（C1〜C16）→ `premises.md` → `docs/stories/ST23.md` → `docs/handoff/ST23.md`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

触るのは `migrations/`（1 本）・`crates/server/src/`（`deletion.rs`・`stay_store.rs`・`lib.rs`・`export.rs`・`export/excluded.rs`・新しい `purge_compaction.rs`・`bin/openapi.rs`・試験）・`docs/openapi.json`（生成物）・
`web/src/`（`DayView.tsx`・`stays.ts`）・`web/e2e/`（新しい spec）・`tools/`（新しい `check-immutable.d/` と `smoke.d/` と `seed.d/` のファイル）・`docs/handoff/ST30.md`。
**`crates/server/src/archive/worker.rs` は触らない**（design D9。ST13 が直している）。
検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` と `tools/db-roles.sh` が前提（`docs/testing.md` §8）。

## Global Constraints（規律。**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript はコメント（`// Scenario: 一緒に消えた位置の本文も消える`）、bash は `echo`。
  `scripts/check_scenarios.py . st23-record-erasure` が spec の全 Scenario と突き合わせ、**印の無い Scenario を FAIL にする**。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change が足す Scenario は 91 本**（`record-deletion` の ADDED 82 本 + MODIFIED に足した 4 本 + `browsing-views` の MODIFIED に足した 5 本）。人間の確認待ちは 0 本。
  MODIFIED で写した既存の 22 本は既存の印を生かす（名前を変えない。ST22 の試験を緑のまま）。THEN に主張が 2 つ以上ある Scenario は、試験が主張を 1 つずつ assert する
- **件数つき検証**: `cargo test <絞り込み>` は一致するテストが 0 本でも rc=0 になる。このファイルで **`CT <絞り込み> (≥N)`** と書いたものは、
  `bash -o pipefail -c 'cargo test -p ashiato-server <絞り込み> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [0-9]+ passed' /tmp/ct.log && test "$(cargo test -p ashiato-server <絞り込み> -- --list 2>/dev/null | grep -c ': test$')" -ge N`
  が rc=0 になることを指す（N 本以上あり、すべて通る）。走らせるのは repo の根から `scripts/quiet-run`（AGENTS.md の「長いコマンド」）。
  **試験の関数名は各 Task が指定する前置きで始める**（絞り込みがその前置きで効くように）
- **`E2E <ファイル> (≥N)`** は `cd web && npm run test:e2e -- <ファイル>` が rc=0 で、`grep -c '// Scenario:' web/e2e/<ファイル>` が N 以上であることを指す
- **深掘りの答え（`deep.md` が正本）**: 口は「消した」滞在だけ（Q1）/ 範囲は「消した」にしたときの記録＋重なる滞在＋前の版すべて（Q2）/ 後着は入れた後に本文も消す（Q3）/ 書庫の写しは消さない（Q4）/
  「消した」行に「戻す」と並べて「本文ごと消す」・確認はソースごと＋前の版＋残るもの・「本文ごと消した」行（Q5）/ 毎晩その日に本文を消した表だけ書き直す（Q6）。
  **第 1 回の答えが推奨と違えば、上流が spec と design を直してからここに来る** —— 推奨のつもりで書かない。deep.md の答えを先に読む
- **聞かずに決めた既定（`defaults.md`）と D の対応**: C1 → D3 / C2 → D6・D17 / C3 → D4 / C4 → D3 / C5 → D7 / C6・C15 → D18 / C7 → D1 / C8 → D2 / C9 → Task 10 の申し送り / C10 → Non-Goals / C11・C12 → Non-Goals / C13 → D5・D14 / C14 → D2 / C16 → D16。
  **D2・D6・D8・D9・D10〜D17 は（仮）決め。** 反転条件は `design.md`。変えたらその D 番号を書き直す
- **消去の形は `raw = ''` と `payload = '{}'` だけ**（design D3）。`event_time`・`content_hash`・`device_id`・取り込みの時刻・削除の印と時刻は書き換えない。本文を消す SQL は `deletion.rs` の 1 つの関数だけが持つ（ほかに書かない）
- **消した時間帯の終わりを `payload->>'end'` から直に読まない**（design D5）。`core.stay_erased`（か同じ式の `core.stay_span`）だけを通す。Task 3 の許可の一覧の試験（`purge_range_reader_allowlist`）が見る
- **ST22 の口と確認を壊さない**（`docs/stories/ST23.md` の「壊してはいけないもの」）: `POST /stays/erase`・`POST /stays/restore` の成功の形、「この滞在を消す」の確認の文面、`web/e2e/day-erase.spec.ts` を緑のまま。
  **ST03 の重複判定**（`dedup_tests.rs`）も緑のまま
- **写真・画像・PC・ブラウザ・気分の本文を消さない**（design D2）。主張・場所・気分の錠（ST19 / ST21 / ST17）に触らない
- **走っている Story と同じファイル**（proposal の Impact。spec review R17）: `archive/worker.rs` は触らない（ST13。design D9）。
  ST09 も `export.rs` の `CATALOG` と `export/excluded.rs` に足し、ST13 も `lib.rs` の `run()`（取り込み器の隣の spawn）と `bin/openapi.rs` に足す。
  **ST09 / ST13 が先に main に入ったら rebase で解き、`docs/openapi.json` は手で混ぜずに生成し直す**（`cargo run -q -p ashiato-server --bin openapi > docs/openapi.json`）
- **ログに出すのは件数・論理ソースの名前・失敗の種別・SQLSTATE だけ**（記録の値・座標・原文を出さない。製造準備 A-2）
- **移行の名前は作成時刻 `YYYYMMDDHHMM_<slug>.sql`**（と `.down.sql`）。`migrations/ORDER` の末尾へ 1 行。連番にしない
- **縦串・書き換え禁止の台本は Story ごとの新しいファイル**（`tools/smoke.d/<時刻>_st23-purge.sh`・`tools/check-immutable.d/<時刻>_st23-derived-erasure.sh`）。共有の台本の末尾に書き足さない（`tools/check-shared-appends.sh`）
- **画面の Scenario は `web/e2e` で担保する**（`docs/testing.md` §4.5）。アサートは数値と経路（実寸・可視・要素の数・文面の数字）。スクリーンショット比較は使わない
- **壁時計を直に読まない**（夜の書き直しの時刻。design D11）。時刻は差し込める形にする

## Task 1: 移行 —— 消去の操作の台帳・範囲・派生の門・ビュー・書き直しの記録（design D4 / D5 / D7 / D11）

- [x] 1.1 移行 1 本（`migrations/YYYYMMDDHHMM_record_purge.sql` と `.down.sql`、`migrations/ORDER` の末尾へ 1 行）で次を入れる:
  `core.erasure_operation` と `core.erasure_span`（design D4 の列・CHECK・追記のみの行トリガと TRUNCATE の文トリガ。関数は表ごとに別）/ `core.erasure_ledger.operation_id uuid NULL` /
  `core.stay_erased` の作り直し（`start_at` / `end_at` は `core.erasure_span` の `scope = 'event'` を先に読み、無ければ本文。列 `purged`・`operation_id` を足す。design D5）/
  派生の門 `core.require_derived_erasure_ledger()` と constraint trigger `event_requires_derived_ledger`（design D7）/ `core.purge_compaction`（design D11）/
  `GRANT MAINTAIN ON core.event, core.event_version TO ashiato_app`（`.down.sql` で `REVOKE`）。`export.rs` の表の分類に新しい 3 表を `Exclude` で足す。
  `.down.sql` は先頭で `core.erasure_operation` に行があれば `RAISE EXCEPTION` で止める（design の Migration Plan。本文を消した後に down すると範囲が 1µs に縮む）。
  Scenario: `消去の操作の台帳は書き換えられない` / `消去の操作の台帳は消せず、表も切り詰められない` / `台帳を書かない滞在の本文の消去は拒まれる` / `前の版を残した滞在の本文の消去は拒まれる` /
  `本文を消した滞在へ本文を書き戻せない` / `解析済みを残して原文だけ空にする書き換えは拒まれる` / `作り直しの書き換えは今までどおり通る`。
  検証: `tools/check-migrations.sh` rc=0、`tools/check-immutable.sh` rc=0（新しい `tools/check-st23-derived-erasure.sh` が psql から門の 5 本と台帳の 2 本を確かめ、各 Scenario を `echo`）、
  `CT purge_schema_ (≥10)`（新しい `purge_tests.rs`。台帳の 2 本・門の 5 本・down の止まり方 1 本に加え、**行の形を固定の値で試験する**（R6: 2 表の列名・型・NULL 可否・CHECK を `information_schema` から読み、期待の表と一致）と、移行の前後で ST22 の消した滞在の `core.stay_erased` の範囲が変わらないこと）、
  `CT export_ (≥1)` で書き出しの表の分類の試験が通る、`CT app_role (≥1)` が `MAINTAIN` の 2 表だけを足した期待で通る（`app_role_tests.rs` を直す）

  （訂正 2026-10-24: 検証の入口を「tools/check-immutable.d/<時刻>_st23-derived-erasure.sh」から上の入口に。受け入れ条件は変えない。理由: <時刻> は名前の雛形で、そのまま走らせると bash の構文エラーになる。節は check-immutable.sh が . で読む形（psql・fail を共有）で単独では走らないため、check-immutable.sh を走らせ節の見出し・NG なし・節の OK 行 10 本以上を確かめる入口に直す 実証: evidence.jsonl 2026-10-23T23:45:14+00:00 FAIL。承認: machine: evidence.jsonl 2026-10-23T23:42:23+00:00 PASS。記録: plan-corrections.md）
## Task 2: 本文ごと消す範囲と消去の核（design D2 / D3 / D6 / D15 / D17）

- [x] 2.1 `deletion.rs` に `purge_scope(tx, stay_id)`（範囲を集めて組ごとに数える。確認と消すが同じ関数を使う）と `purge(pool, stay_id, requested_user, expected)`（design D3 の 1〜8 の順）を置く。
  範囲の組は design D15。重なる滞在は**作り直しの印のものだけ・端が触れるだけは入れない**（D2（仮））で、`user:purged` に付け替える（D6）。本人が別に消した滞在と印の無い滞在は入れない（D2 / D17（仮））。
  どの組にも入らない論理ソース（写真・画像など）が範囲に入ったら `unknown_source` で断る（D2 / D15）。
  Scenario: `一緒に消えた位置の本文も消える` / `位置を持つ書庫の項目は項目ごと本文が消える` / `前の版の本文も消える` / `吸収された滞在の本文も消える` / `同じ時間の写真の本文は残る` /
  `同じ時間の PC のウィンドウの本文は残る` / `消した滞在を指して書いた気分は残る` / `別の操作で消した位置の本文は消えない` / `別に消した滞在は重なっていても本文が残る` / `一覧に出ている滞在の本文は消えない` /
  `本文を消した記録は時刻と鍵を残す` / `本文ごと消すと操作の台帳に 1 行残る` / `記録ごとの消去の台帳の行が操作に結ばれる` / `本文を消した滞在の時間が範囲の行に残る` /
  `台帳に書けないと本文も消えない` / `本文ごと消す操作と戻す操作が同時に走るとどちらか一方だけが効く`。
  検証: `CT purge_scope_ (≥17)`（`purge_tests.rs`。本文が空であることは `raw` と `payload` の両方を DB から読んで見る。「台帳に書けないと」は ST22 の `erase_with_action` と同じく試験用の口で台帳の CHECK 違反を起こし、
  本文が 1 件も空になっていないことを見る。同時に走る試験は ST22 の `deletion_tests.rs` の同時実行の形を写す。端が触れるだけの滞在が入らないことを 1 本足す）、`CT deletion_ (≥1)` で ST22 の試験がすべて通る

## Task 3: 消した時間帯の読み手を台帳へ寄せる（design D5 / D6 / D14。spec「本文ごと消した時間帯は、台帳の範囲から読む」）

- [x] 3.1 `stay_store.rs` の `mark_late_arrivals`・`mark_late_for_user`（`user_erased_stay_sql` をビュー越しに）・`load_existing`（`raw = ''` の滞在を候補から外し、その時間を `erased` に足す）、`export/excluded.rs`（前の版は `core.erasure_span` の `scope = 'version'` か本文）を `core.stay_erased` へ寄せる。
  ビューは `user:purged` を出さない（design D5 / D13（仮））。span の無い本文を消した滞在は design D14（その日の全体。後着は印だけ）で読む。
  **Task 2 の消す操作の上に積む**（この Task の Scenario は本文を消した滞在を作ってから見る）。
  Scenario: `本文ごと消した後も消した時間帯の長さは変わらない` / `本文ごと消した時間帯の撮影位置は書き出しから除かれたまま` / `本文ごと消した時間帯に作り直しは滞在を作らない` /
  `作り直しの後も本文を消した滞在の本文は空のまま` / `吸収されていた滞在の本文は作り直しで戻らない` / `本文を消した記録は 1 日の記録の読み出しに出ない` /
  `範囲の行が無い本文を消した滞在はその日全体を書き出しから除く` / `本文ごと消す操作と作り直しが同時に走っても本文は戻らない`（spec review R13。`load_existing` の候補外しが要るのでこの Task）。
  検証: `CT purge_range_ (≥9)`（Scenario の 8 本と `purge_range_reader_allowlist`）、`CT export_excluded (≥1)` と `CT stay_ (≥1)` で既存の試験がすべて通る。
  `purge_range_reader_allowlist` は `crates/server/src` の `_tests.rs` 以外の `payload->>'end'` の出現を `file:関数` で集め、design D5 の許可の一覧（`stay_store.rs` の `load_existing` と `day_view` の生きた滞在の絞り・
  `lib.rs` の `stays_detail_get`・`export/excluded.rs` の前の版の本文の補い・コメント行）と一致することを見る（一覧の外に 1 つでも出たら落ちる。spec review R6）

## Task 4: 確認の口と消す口・戻す口の断り（design D1 / D6。spec「消した滞在を本文ごと消す操作がある」「本文ごと消す前に…」「確認で見せた件数と…」「消したものを戻せる」）

- [x] 4.1 `lib.rs` に `GET /stays/purge/preview` と `POST /stays/purge` を足し（型は `PurgeCounts` / `PurgePreview` / `PurgeOutcome`。`bin/openapi.rs` の契約にも載せる）、`POST /stays/restore` が本文を消した記録を含む求めを 409 で断る（D6）。
  すでに本文ごと消した滞在は件数を比べずに 200・0 件（design D1 の順）。戻す口の 409 は、別の原因で消えたまま残る記録を数えない（design D6。spec review R3）。
  Scenario: `消した滞在を本文ごと消すと 200 で件数が返る` / `消えていない滞在は本文ごと消せない` / `作り直しが隠しただけの滞在は本文ごと消せない` / `滞在でない記録の識別子では本文ごと消せない` /
  `他の利用者を指定して本文ごと消すことはできない` / `資格情報の無い本文ごと消す求めは断られる` / `知らない識別子を本文ごと消そうとすると断られる` / `本文ごと消した滞在をもう一度指定しても何も起きない` /
  `組に入らない記録が範囲にあると断られる` /
  `確認は消える件数をソースの組ごとに返す` / `確認を求めても何も消えない` / `本文ごと消した滞在の確認は 0 件を返す` / `滞在でない記録の識別子の確認は断られる` / `知らない識別子の確認は断られる` /
  `消えていない滞在の確認は断られる` / `資格情報の無い確認の求めは断られる` /
  `確認の後に位置が届くと消さずに断られる` / `件数が一致すれば本文ごと消える` / `本文を消した滞在は戻せない` / `本文を消した記録を含む戻す求めは断られ、何も戻らない` /
  `本文ごと消した時間と境界で位置を共有しても隣の消した滞在は戻せる` / `消去の操作の台帳は記録の読み出しと書き出しに出ない`。
  検証: `CT purge_api_ (≥22)`（`api_tests.rs` の流儀で HTTP の口を叩く。「確認を求めても何も消えない」は確認の前後で範囲の全記録の `raw` を比べる。「組に入らない記録」は試験の中で写真の記録に連鎖の台帳の行を直に積んで作る）、
  `CT api_ (≥1)` で既存の口の試験がすべて通る。
  契約（spec review R14）: `cargo run -q -p ashiato-server --bin openapi > docs/openapi.json` の後に `tools/check-openapi.sh` rc=0、
  `jq -e '.paths["/stays/purge"] and .paths["/stays/purge/preview"] and .paths["/stays/restore"].post.responses["409"]' docs/openapi.json` rc=0

## Task 5: 取り込みの直後に後着の印を付け、本文を消した時間帯なら同じまとまりで消す（design D8。申し送り R8）

- [x] 5.1 `stay_store::settle_arrival(tx, &stored)` を置き、`lib.rs` の `ingest_request` に `Settle::InTx` / `Settle::Later` の引数を足して、`ingest_one`（HTTP）から入った記録だけトランザクションの中（commit の前）で呼ぶ
  （`store_with`・書庫の取り込み器は `Later`。design D8（仮）。spec review R1）。位置（と位置を持つ書庫の項目）のときだけ錠を取り、消した時間帯なら `user:late`、本文を消した時間帯なら加えて本文を消し、
  `kind = 'late'` の操作の行と記録ごとの台帳の行を積む（Task 2 の消去の関数を使う）。範囲の行が無い時間帯（design D14）は印だけ。`rebuild_day` の `mark_late_arrivals` は残す。応答は変えない。
  Scenario: `本文ごと消した時間に端末から届いた位置は本文が消える` / `端末から届いた位置は本文を持ったまま確定しない` / `後から届いて本文を消した記録は元の操作に結ばれる` / `本文ごと消した時間の外に届いた位置は本文が残る` /
  `一緒に本文を消した滞在のはみ出しは時間帯に入らない` / `範囲の行が無い時間帯に後から届いた記録は本文を消さない` /
  `本文を消した位置を同じ内容で送り直しても戻らない` / `作り直しが失敗しても後から届いた位置に印が付く` / `本文ごと消す操作と同じ時間帯の取り込みが同時に走っても位置の本文は残らない`。
  検証: `CT purge_late_ (≥9)`（新しい `purge_late_tests.rs`。「本文を持ったまま確定しない」は、消去の台帳の行の `txid` が記録の行の `xmin` の取り込みのトランザクションと同じことを見る（spec の THEN）。
  「作り直しが失敗しても」は `App::stays` を常に失敗する偽物に差し替え、印と書き出しの除外を見る。「同時に走っても」は 2 つの接続で本文ごと消す操作と `/ingest` を同時に始め、両方の終わりの後に 10:30 の行の `raw` が空であることを見る）、
  `CT deletion_ (≥1)`・`CT dedup_ (≥1)` で ST22 の後着の試験と ST03 の重複判定がすべて通る

## Task 6: 書庫の後着の本文を消し、見回りごとに消し直す（design D9）

- [x] 6.1 `stay_store::mark_archive_arrivals` が本文を消した時間帯の行の本文も消す（Task 5 と同じ関数）。`archive/worker.rs` は変えない —— 落ちたら既存の読み直し（`read` の台帳の行を書かず、次の見回りが書庫を読み直して印付けを呼び直す）が消し直す（design D9（仮））。
  Scenario: `本文ごと消した時間に書庫から入った位置は本文が消える` / `書庫の後着の消去に失敗しても次の見回りで消える` / `同じ書庫を読み直しても本文は戻らない`。
  検証: `CT purge_archive_ (≥3)`（`archive_flow_tests.rs` の書庫の組み立てを使う。「失敗しても」は `stay_store` 側の試験用の差し込み口で 1 回目の消去を落とし、次の見回りの後に空であることを見る。
  これが落ちたら design D9 の反転条件 —— 上流へ返す）、`CT archive_ (≥1)` で ST12 / ST47 の書庫の試験がすべて通る、`git diff --quiet origin/main -- crates/server/src/archive/worker.rs` rc=0

## Task 7: 夜の書き直し（design D11）

- [x] 7.1 `crates/server/src/purge_compaction.rs` に `run_once(pool, now)`（対象の表を選び、書き出しが走っていれば待ち、`lock_timeout` を掛けて `VACUUM FULL`、`relfilenode` を前後で比べて `core.purge_compaction` に 1 行）と、毎日 03:30（Asia/Tokyo）に `run_once` を呼ぶ `spawn` を置き、`lib.rs` の `run()` で起こす。時刻は差し込める形にする。
  Scenario: `本文を消した晩に表が書き直される` / `本文を消していない晩は書き直さない` / `書き直せなかったら失敗として残り、次の晩にもう一度書き直す` / `書き出しの最中は書き直しを始めない`。
  検証: `CT purge_compaction_ (≥4)`（新しい `purge_compaction_tests.rs`。サーバと同じ `ashiato_app` の接続で走らせる。「書き直せなかったら」は試験の中で `REVOKE MAINTAIN` してから `run_once`、`GRANT` し直してもう一度。
  「書き出しの最中」は書き出しの job の走っている状態を作り、`run_once` が書き直さずに待つことを見る）。
- [x] 7.2 実物の量の時間（design D11 の反転条件。spec review R15）: `tools/seed.sh max` は 1 日分で量が足りないので使わない。新しい `tools/st23-compaction-volume.sh` が使い捨ての DB（`tools/stack.sh` の DB ではない）に
  `generate_series` で 1 年分（位置 60 秒ごと = 約 52.6 万行、滞在 1 日 15 件 × 前の版 20 件、`raw` は seed の 1 件と同じ大きさ）を `core.event` と `core.event_version` に入れ、1 日分の本文を消してから `run_once` を 1 回走らせ、
  表ごとの秒数と表の大きさを標準出力に出す。
  検証: `scripts/quiet-run volume -- tools/st23-compaction-volume.sh` rc=0 で、出力の `seconds_total` が 300 以下（超えたら rc=1。design D11 の反転条件として上流へ返す）。秒数と表の大きさは PR の本文に書く

## Task 8: 写しの数・残すもの・稼働状況・「後に消した記録」（design D10 / D18。spec「書庫の写し・本人のファイル…は消さない」「本文ごと消した記録も稼働状況と…」）

- [x] 8.1 確認の口の `copies` を design D10 で数え、`remains` に書庫の写し・本人のフォルダの書庫・書き出したファイル・今夜の書き直しを入れる。稼働状況と「後に消した記録」の数え方はコードを変えず試験で固定する（D18。申し送り R11 を含む）。
  Scenario: `確認は書庫の写しが残ることを返す` / `記録の入っていない写しも同じソースなら数える` / `本文ごと消しても書庫の写しは残る` / `本文ごと消しても書き出したファイルは残る` / `本文ごと消しても稼働状況は記録ありのまま` /
  `記録を消しても隣の記録の無い日の状態は変わらない` / `書き出しの後に本文ごと消した記録は後に消した数に入る`。
  検証: `CT purge_keep_ (≥7)`（写しと書き出したファイルは置き場のファイルの SHA-256 を消す前後で比べる。稼働状況は `/coverage` の応答で、隣の日は想定間隔が 1 日以上のソースで見る）

## Task 9: 画面 —— 「消した」行の「本文ごと消す」・確認・「本文ごと消した」行（design D12 / D16。spec「「消した」の行から本文ごと消せる」・`browsing-views` の MODIFIED）

- [x] 9.1 サーバ: `stay_store::day_view` が本文を消した区間を `purged: true`・`purge_stay_id`・`late_purged` で出し、本文を消していない「消した」とまとめない（design D12）。
  区間の時間は指定した滞在の時間（design D12 / D13（仮））。
  Scenario: `本文ごと消した区間は印を持ち、戻すための識別子を持たない` / `本文ごと消した区間は指定した滞在と後から届いた件数を持つ` /
  `本文ごと消した時間と消した時間は隣り合ってもまとまらない`（読み出しの側）/ `本文ごと消した時間と重なる消した時間は差し引いて出る`（読み出しの側）。
  検証: `CT purge_day_ (≥4)`、`CT day_ (≥1)` で ST16 / ST22 / ST25 の 1 日の並びの試験がすべて通る
- [x] 9.2 画面: `web/src/DayView.tsx` と `stays.ts` に「本文ごと消す」・確認（`role="alertdialog"`。確認の口を読み、409 `counts_changed` で出し直す）・「本文ごと消した」行（後着の件数を添える）を足す。文面は design D16。
  Scenario: `消した行に本文ごと消すが出る` / `本文ごと消すの操作は 44 px を下回らない` / `確認に消える件数がソースの組ごとに出る` / `確認に消えずに残るものが出る` / `本文ごと消す確認をやめると何も消えない` /
  `確認して本文ごと消すと本文ごと消したの行になる` / `件数が変わって断られると数え直した確認が出る` / `後から届いて本文を消した件数が行に出る` / `この滞在を消すの確認は変わらない` /
  `本文ごと消した行には戻すが出ない` / `本文ごと消した時間は本文ごと消したと書かれた行で出る` / `本文ごと消した時間と消した時間は隣り合ってもまとまらない`（画面の側）/ `本文ごと消した時間と重なる消した時間は差し引いて出る`（画面の側）。
  偽データ（spec review R16）: **消す Scenario は試験の中で自分の日を作って消す**。実行ごとに使っていない日付（2031-01-01 から 1 日ずつ。`GET /day` で空の日を探す）に、`/ingest` で位置を送り
  （前の版 20 件は、終わりが延びる位置を 21 回に分けて送り、送るたびの作り直しで積む）、書庫の位置 3 件と位置を持つ書庫の項目 1 件も `/ingest` で送り（`tools/seed.sh` の書庫の項目の送り方を写す）、`POST /stays/erase` で消してから画面を開く。
  共有の偽データの滞在は消さない（読むだけ）。書庫の写し 2 個は新しい `tools/seed.d/<時刻>_st23-purge.sh` が置き場にロケーション履歴の写しのファイルを 2 個置く（読むだけの材料。消さないので何度走っても同じ。`tools/stack.sh` は `SEED` の既定 `normal` で seed を流すので seed.d が読まれる。`tools/stack.sh:102`）。
  検証: `E2E st23-purge.spec.ts (≥13)`（新しい `web/e2e/st23-purge.spec.ts`。44 px は `getBoundingClientRect` の幅と高さ。件数は文面の数字を読む。「件数が変わって」は確認を開いた後に API で位置を 1 件送る。
  `STACK_RESET` なしで 2 回続けて走らせても rc=0）、`E2E day-erase.spec.ts (≥1)` が緑のまま、`cd web && npm test` rc=0

## Task 10: 縦串・申し送り・仕上げ

- [ ] 10.1 `tools/smoke.d/<時刻>_st23-purge.sh`: 縦串で 滞在を作る → 消す → 確認を読む → 件数を添えて本文ごと消す → DB の本文が空・台帳に 1 行・`/day` に本文ごと消した区間 → 同じ内容を送り直して重複 を通す（各段を `echo`）。
  `docs/handoff/ST30.md` に C9（本文を消した記録と書庫の写しのバックアップ・拠点外の写しへの及び方。ST12 は写しをバックアップの対象にすると決めている）を 1 項足す。
  `docs/handoff/ST23.md` の st12 R23 の項に「ST12 の R56 で直っている」の済みの印を付ける（review/deep.md R13）。
  検証: `scripts/quiet-run smoke -- tools/smoke.sh` rc=0、`bash tools/check-shared-appends.sh` rc=0、`python3 scripts/check_scenarios.py . st23-record-erasure` rc=0、
  `scripts/quiet-run unit -- cargo test -p ashiato-server` rc=0、`cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0、`grep -c 'C9' docs/handoff/ST30.md` ≥ 1
