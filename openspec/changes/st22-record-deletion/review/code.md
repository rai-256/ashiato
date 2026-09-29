## final review（e99c457..5630e31）

### Strengths

- 削除・復元を利用者単位の advisory lock と単一トランザクションに結び、台帳への追記と状態変更を一体で扱っている。
- day view は削除済み区間を専用ビュー経由の `erased` 行として表し、復元 UI へ識別子を渡している。
- サーバ・Web・e2e の Scenario テスト、OpenAPI、immutable 検査を更新している。

## R1. 重なる削除操作の片方を戻すと、なお削除中の位置が復元される
- 成果物: `crates/server/src/deletion.rs`
- 根拠: scoped re-review（`5630e31..9faee62`）で `crates/server/src/deletion.rs:129-170,280-334` を確認。重複済み位置への B 原因の追記は直ったが、A 削除→B 削除→B 復元では A が削除中でも位置を戻す。追加テスト `crates/server/src/deletion_tests.rs:436-456` は逆順だけを検証する。
- kind: technical
- 処置: 未解決（scoped re-review: R1 NOT ADDRESSED）。1 回だけの final 修正波を使い切ったため、次段 `finish` で処置を決める。

## R2. 不正な `payload.end` を持つ削除済み滞在で一覧・作り直しが失敗し得る
- 成果物: `migrations/202609271716_deletion_ledger.sql`, `crates/server/src/stay_store.rs`
- 根拠: `migrations/202609271716_deletion_ledger.sql:46` と `crates/server/src/stay_store.rs:651` が `payload->>'end'` を直接 `timestamptz` にキャストする。一方 `span_of` は壊れた end を安全に扱う。
- kind: technical
- 処置: fixed Task 1・Task 3 / design D12。`core.try_timestamptz` で不正な end を NULL に畳み、ビューと rebuild の範囲判定で開始時刻へフォールバックするよう統一。malformed end の一覧・rebuild 回帰テストを追加。

## R3. ST22 の移行が計画上限の 1 本を超えている
- 成果物: `migrations/202609291151_detail_counts_index.sql`, `crates/server/src/lib.rs`
- 根拠: `migrations/202609291151_detail_counts_index.sql:1-5` と `crates/server/src/lib.rs:127-134`。Task 1 / design D10 の「移行は 1 本だけ」に対し、deletion ledger と detail counts index の 2 本を足している。
- kind: technical
- 処置: fixed Task 1 / design D10。detail counts のライブ索引を deletion ledger 移行へ統合し、2 本目の移行と MIGRATIONS 登録を除去。

## R4. 破壊操作のボタンがライトテーマでもダーク配色に固定される
- 成果物: `web/src/DayView.tsx`
- 根拠: `web/src/DayView.tsx:326-333` が `SCHEMES.dark` を固定で渡すため、OS の明暗設定に追従しない。
- kind: technical
- 処置: fixed Task 8 / design D9。DayView の破壊操作へ現在の scheme を渡し、ライトテーマでライトの操作面を使う UI 回帰テストを追加。

---

# code-verify（独立検証。2026-09-29）

対象: worktree `/home/yosis/dev/ashiato2-st22`、ブランチ `feat/st22-record-deletion`、HEAD `98dcade`（merge-base `e99c457`、25 ファイル）。
PR はまだ無い（`gh pr list --head feat/st22-record-deletion` → `[]`）。
**作業ツリーのコードは触っていない**（終了時 `git status --short` は空）。わざと壊す検査と使い捨ての観測テストは、
`git archive HEAD` で作った複製 `/tmp/st22v/mut` の中だけで行った。DB は専用の compose project（`st22verify` 55422 /
`st22e2e` 55424・API 18722・画面 5192 / `st22smoke` 55425・API 18723）を立てて使い、最後に `down -v` した。共有の 55432 / 18787 には触れていない。
番号は上の final review（R1〜R4）の続き（R5〜）。

## 申告と実測

申告: tasks 34 件すべて `[x]`（1.1〜10.4）。`evidence.jsonl` 114 行、項目ごとの最新はすべて PASS。
**ただし最新の証跡はどれも `57f88b4` 以前（最も新しいもので 04:36:50Z）で、final 修正 `9faee62`（04:50Z）の後に取られた証跡は 1 本も無い。**

| 検証コマンド（申告） | 実測（HEAD `98dcade`） | 一致 |
|---|---|---|
| `cargo test --workspace`（10.3） | **FAILED。server 354 passed / 3 failed**（`restore_is_scoped` / `restoring_one_of_overlapping_erases_keeps_the_other_cause_hidden` / `malformed_erased_stay_end_is_safe_for_listing_and_rebuild`）。2 回走らせて同じ。**`5630e31`（final 修正の前）の複製では 355 passed / 0 failed** | **不一致**（R5 / R6） |
| `CT` 23 本（1.1〜7.1） | 22 本は rc=0 かつ件数 ≥ 1。**4.3 `CT restore_is_scoped` は `0 passed; 1 failed`** | **不一致**（R5） |
| `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` | rc=0 / rc=0 | 一致 |
| `cd web && npm run lint && npm run test && npm run build` | rc=0 / 22 files・152 passed / rc=0 | 一致 |
| `npm run test -- stays / DayView / erase / erased-row / keyboard`（8.1〜8.5・5.3） | 2 / 37 / 15 / 15 / 15 passed（erase・erased-row・keyboard は同じ 1 ファイルに当たる） | 一致 |
| `npm run test:e2e`（9.1 / 9.2・10.3） | rc=0。6 passed（`day-erase.spec.ts` の 2 本を含む） | 一致 |
| `tools/check-immutable.sh`（1.3・10.3） | rc=0。`OK deletion_ledger は UPDATE / DELETE / TRUNCATE を拒む`、出力に `deletion_ledger` あり | 一致 |
| `tools/smoke.sh`（10.3） | rc=0（`縦串 OK`）。ST22 の口を叩く段は無い（口は e2e が本物のサーバ越しに叩いている） | 一致 |
| `tools/check-migrations.sh` / `check-openapi.sh` / `check-boundaries.sh` | rc=0 / rc=0 / rc=0 | 一致 |
| `python3 scripts/check_scenarios.py . st22-record-deletion` | rc=0（Scenario 484 / 担保あり 484 / 人間の確認待ち 0） | 一致（中身は手 3） |
| `python3 scripts/check_chain.py .` / `openspec validate st22-record-deletion --strict` | rc=0 / valid | 一致 |
| `python3 scripts/review_triage.py . st22-record-deletion`（10.2） | **rc≠0。`triage: FAIL (4 件)`**（上の R1 が処置なし、R2〜R4 の `fixed Task` が形に合わない） | **不一致**（R5。ただし finish の段で直る種類） |
| 10.4 `grep -c 'st22-record-deletion' docs/handoff/ST23.md docs/handoff/ST33.md` | 6 / 3 | 一致 |

## 手 1: 固定値を独立に再計算する —— 一致

| 固定値 | 独立の計算 | 実装・テスト |
|---|---|---|
| 消す・位置 61 件（`erase_endpoint`） | fixture は 10:00 から `0..=60` 分の 61 点。端を含む（D3）ので 61 | `{"erased":{"stays":1,"locations":61}}` |
| 詳細の位置 8 件（`stays_detail_counts`） | 12 点を置き、`put_deleted_event` は**同じ時刻の生きた行もまとめて**印を付ける（`testdb.rs:183-196`）ので 0〜3 分の 4 点が消え 12 − 4 = 8 | `"count": 8` |
| ライトの破壊操作の背景 | python `colorsys.hls_to_rgb(132/360, 0.86, 0.30)` → `(209, 230, 213)` | `rgb(209, 230, 213)` |
| 消した行の文字（ライト） | muted 30 / ground 96 の WCAG 比 6.47（4.5 以上） | —— |

## 手 2: ガードをわざと壊す

複製で 1 か所ずつ置き換え（置き換え前に出現回数 1 を assert）、テストを走らせて戻した。基準（無変更）は 354 passed / 3 failed なので、下の「落ちた」はその 3 本以外が落ちたことを指す。

| # | 壊したもの | 走らせたもの | 結果 |
|---|---|---|---|
| M1 | 連鎖から `logical_source = ANY($2)` を外す（PC のウィンドウも消える） | deletion_tests | 落ちた（`erase_cascade`） |
| M2 | 連鎖の端を含まない形（`>` / `<`）に | deletion_tests | 落ちた（`erase_cascade` / `erase_endpoint` / `erase_writes_ledger`） |
| M3 | 滞在の印を `rebuild:user` に | deletion_tests | 落ちた（9 本） |
| M4 | 消す操作の錠（`pg_advisory_xact_lock`）を外す | `erase_locks_against_rebuild` × 3 回 | 3 回とも落ちた |
| M5 | 稼働状況（`facts`）を `core.event_live` に | `coverage_counts_deleted` | 落ちた |
| **M5b** | 途絶の判定に使う `active_days` だけを `core.event_live` に | `coverage_counts_deleted` | **通った**（R11） |
| M6 | 詳細の件数を `core.event`（削除済みを含む）に | `stays_detail_counts` | 落ちた |
| M7 | 作り直しの中の後着の印付けを呼ばない | `late_arrival` / `restore_includes_late_arrivals` | 落ちた |
| M8 | 消した区間を記録なしの計算から差し引かない | `day_view_erased` | 落ちた（`day_view_erased_not_no_record_or_move`）。`stay_day_view` 9 本は通る（据え置き分なので妥当） |
| M9 | 消した後に作り直さない | deletion_tests | 落ちた（`erase_rebuilds_day` / `erase_survives_rebuild_failure`） |
| M10 | 戻した後に作り直さない | deletion_tests | 落ちた（`restore_unhides_overlapping_stays`） |
| M11 | 台帳の行トリガを `BEFORE UPDATE` だけに | `deletion_ledger_is_append_only` / **`tools/check-immutable.sh`** | どちらも落ちた（check-immutable は rc=1、`NG deletion_ledger を変更できた: DELETE …`） |
| M12 | 台帳の TRUNCATE の文トリガを消す | `deletion_ledger_is_append_only` | 落ちた |
| M13 | `core.stay_erased` に `payload` を載せる | `stay_erased_view_hides_coordinates` | 落ちた |
| **M14** | **戻す操作の錠を外す** | deletion_tests | **通った**（R10） |
| M15 | 戻すで未知の識別子を断らない（`stays.is_empty()` だけ見る） | deletion_tests | 落ちた（`restore_endpoint`） |
| M16 / M17 | 消す・戻すの利用者の照合を外す | deletion_tests | 落ちた（`erase_endpoint` / `restore_endpoint`） |
| M18 | 「消した」の結合を `start < last.end`（隣り合いを繋がない）に | `day_view_erased` | 落ちた（`…merges_adjacent_ranges…`） |
| W1 | `DESTRUCTIVE_TARGET_PX` を 44 → 24 | vitest 152 本 | 落ちた（`削除操作は幅と高さが 44 px 以上`）。e2e の `boundingBox` も同じ値を測る |
| **W2** | 詳細を開く行のボタンで Enter / Space の既定動作を止める | vitest 152 本 | **通った**（R12） |
| **D1** | `.down.sql` から `DROP FUNCTION core.try_timestamptz` を消す | `tools/check-immutable.sh` | **rc=0**（R13） |

検査の外側: `check-immutable.sh` の down の確認は「`core.deletion_ledger` と `core.stay_erased` が無くなったか」だけを見る（`tools/check-immutable.sh:944`）。同じ移行が足した関数と `core.event` 上の索引は見ていない（R13）。

## 手 3: Scenario と test の突き合わせ

`check_scenarios.py` は rc=0（この change の 65 本すべてに印がある）。印の先を 1 本ずつ読み、主張の階層とテストの階層がずれていたもの:

- **`別の操作で消した記録は戻らない`**: 印の先 `restore_is_scoped` は HEAD で**落ちている**。落ちている assert は THEN そのもの（`最新の台帳行が別の滞在を原因とする位置を戻した`）で、テストの誤りではなく実装が Scenario を満たさない（R5）
- **`キーボードで詳細を開ける`**: 印の先 `キーボード決定操作で詳細を開閉できる`（`DayView-erase-erased-row-keyboard.test.tsx:233-241`）は `fireEvent.click` を 2 回呼ぶだけで、キーを 1 度も押していない。e2e にもこの操作は無い（`grep keyboard web/e2e` は日の移動の Tab だけ）（R12）
- `消した行から戻すと滞在の行が戻る`: 印の先の vitest は「`POST /api/stays/restore` を呼んだ」までしか見ない（モックは戻した後も一覧を変えない）。THEN の「滞在の行が戻り、消した行は消える」は e2e `確認して消すと…戻すと滞在行に戻る`（別の Scenario の印）が本物のブラウザで観測している。**担保はあるが印の位置がずれている**ので指摘にはしない
- `消した行は文字で区別される`: THEN の「記録なしの行と読み分けられる」に対し、vitest の fixture は「消した」の行 1 本だけで記録なしの行を並べていない。文字「消した」の有無は見ているので指摘にはしない
- `台帳に書けないと滞在の印も付かない` / `作り直しが失敗しても消したことは残る`: THEN を観測している（後者はログの `kind="stay.rebuild"`・利用者・日・`failure=other` と、座標が出ないことまで assert）

## 手 4: 本人の決定が test で固定されているか

| 決定 | 書き換えたら落ちるテスト |
|---|---|
| Q1 滞在と、その時間の位置を消す / 位置以外は消さない | あり（M1 / M2） |
| Q2 後から届いた位置は印を付けて入れる / 応答は変えない | あり（M7、`ingest_response_unchanged_after_erase`） |
| Q3 稼働状況は消した記録も数える | 日の状態（`facts`）はあり（M5）。**途絶の判定に使う前後の活動（`active_days`）は無い**（M5b → R11） |
| Q4 44 px | あり（W1、e2e の実寸） |
| Q4 閉じている行に消す操作は出ない / その場で確認・やめると消えない / 件数を文面に出す | あり（vitest 3 本） |
| Q4 「消した」の行を残す・戻せる | あり（M8 / M18、e2e 9.2） |
| C1 `deleted_by` は `rebuild:` で始めない | あり（M3） |
| C4 二度消しで時刻を動かさない | あり（`erase_is_idempotent`） |
| C6 / R44 消す・戻すは作り直しと同じ錠 | **消す側だけ**（M4）。**戻す側は外しても全部通る**（M14 → R10） |
| C7 位置を消したら作り直す | あり（M9） |
| C10 利用者は行から取る・違えば 404 | あり（M16 / M17） |
| D7（仮）隣り合う消した時間は 1 行 | あり（M18） |
| D9（仮）戻すに確認を置かない | あり（`消した行の戻すで復元口を呼ぶ` は確認なしで POST を期待する） |

## 手 5: tasks の `[x]` と実体

名指しされたテスト名・ファイル・コマンドはすべて実在する（`CT` 23 本の絞り込みはどれも 1 本以上に当たる）。HEAD で rc≠0 になる `[x]`:

- **4.3** `CT restore_is_scoped` → `0 passed; 1 failed`
- **10.3** `cargo test --workspace` → 3 failed
- **10.2** `review_triage.py` → `triage: FAIL (4 件)`（final review の処置の形。finish の段で書き直される種類）

3 件とも、`[x]` の根拠になった証跡は final 修正 `9faee62` より前の HEAD（`adfc7758` / `57f88b49`）で取られている（R5）。

## 手 6: 隙間

複製に使い捨てのテスト（`zz_probe_*`。観測値を出す）を置いて、本物の作り直し（`App::for_test` の既定）を通して走らせた。

- **本人が消した場面の位置が、印の無いまま読み出しに残る経路**（R8）: 取り込みの後の作り直しが 1 回落ちると、別の日の位置が届いても直らない
- **戻せない「戻す」**（R9）: 利用者が選べる基準（間隔 60 分）で本物の流れを通すと、翌日の「消した」の行が識別子 0 件になり「戻す」は 404 を返し続ける
- **消す操作に辿り着けない滞在**（R7）: 生きた滞在の `payload.end` が壊れていると `/stays/detail` が 500 を返し、画面は「この滞在を消す」を出さない
- 重なる 2 つの滞在（上の R1）: 本物の作り直しでは、1 つ目を消した後の作り直しで 2 つ目は `rebuild:absorbed` になり、2 つ目への消す求めは `{"stays":0,"locations":0}` を返す（`zz_probe_overlap_real_flow`）。**R1 のテスト（`restoring_one_of_overlapping_…`）の「A も B も本人が消した」状態は、このテストの fixture（位置 1 点）では本物の流れから作れない**。finish で R1 を処置するときの材料として書いておく
- 時計の戻り: 戻す対象の選び方は `at` ではなく `seq`（D1）で引いているので、`now()` が戻っても順序は崩れない（コードで確認。実行はしていない観測なので指摘にはしない）
- 応答が読めない: 画面の消す・戻すは失敗表示のまま再試行でき、二度消しは件数 0 の 200（C4）、戻すも件数 0 の 200 なので、再送で状態は壊れない（`削除の失敗を行内に表示し、再試行できる` と `erase_is_idempotent` が固定している）

---

## R5. final 修正（`9faee62`）で HEAD が赤になった。`別の操作で消した記録は戻らない` を固定していたテストが落ち、`[x]` の 4.3 / 10.3 は HEAD で成り立たない
- 成果物: `crates/server/src/deletion.rs` / `crates/server/src/deletion_tests.rs` / `openspec/changes/st22-record-deletion/tasks.md` / `evidence.jsonl`
- 根拠: 専用 DB（55422）で `cargo test --workspace` → `test result: FAILED. 354 passed; 3 failed`（2 回とも同じ 3 本）。同じ DB 構成で `git archive 5630e31` の複製は `355 passed; 0 failed`。落ちる 1 本 `restore_is_scoped`（tasks 4.3 の検証そのもの）は `deletion_tests.rs:572` の `assert_eq!(…, Some("user:cascade"), "最新の台帳行が別の滞在を原因とする位置を戻した")` が `left: None`。原因は final 修正で足した `already_deleted`（`deletion.rs:277-298, 324-335`）—— 滞在 1 を消して戻す → 滞在 2 を消す（位置は `user:cascade`、原因は滞在 2）→ 滞在 1 をもう一度消す、で滞在 1 を原因とする `erase` の台帳行が位置に積まれ、`restore`（`deletion.rs:129-145`）が「最後の台帳行の原因」で引くので、**滞在 2 がまだ消えているのに滞在 2 の連鎖で消えた位置を戻す**。R1 を「最後に消した側が持つ」に替えただけで、Scenario `別の操作で消した記録は戻らない`（`specs/record-deletion/spec.md`）の逆向きの破れを作った。`evidence.jsonl` の最新は 4.3 が `adfc7758`、10.3 が `57f88b49` で、`9faee62` 以後の証跡は 0 本
- kind: technical
- 提案: 位置を「最後の原因」1 つで持つのをやめ、**その位置を原因として消している滞在のうち、まだ消えているものが残っていれば戻さない**（台帳から原因ごとの最新行を引いて、生きた `erase` の原因が 0 件になったときだけ印を外す）。R1 と R5 の両方の順序をテストで固定し、`verify-run` で 4.3 / 10.3 の証跡を HEAD で取り直す

## R6. R1 の回帰テスト `restoring_one_of_overlapping_erases_keeps_the_other_cause_hidden` は HEAD で落ちたまま commit されている。fixture は本物の作り直しの下では前提の状態を作れない
- 成果物: `crates/server/src/deletion_tests.rs:409-456`
- 根拠: 同テストは `deletion_tests.rs:438` の `restore(first)` の `locations == 0` で `left: Number(1)` になり落ちる。同じ fixture を使い捨てのテスト `zz_probe_overlap_real_flow` で追うと、`erase(first)` の後の作り直しで `second` の印は `Some("rebuild:absorbed")` になり、`erase(second)` は `{"erased":{"locations":0,"stays":0}}`。つまり「A も B も本人が消した」状態に入っておらず、落ちる理由は実装だけでなく fixture にもある。上の R1 の処置（finish）はこのテストを根拠にしている
- kind: technical
- 提案: R1 を再現するなら、作り直しの差し替え口で何もしない rebuilder を使う（`restore_is_scoped` と同じ形）か、2 つの滞在が作り直しの後も生きて重なる fixture（別の基準の版）を置く。どちらにしても**赤のテストを commit しない**

## R7. R2 は `fixed` とされたが、自分の回帰テストが落ち、`/stays/detail` は不正な `end` を直にキャストして 500 を返す。その滞在は画面から消せない
- 成果物: `crates/server/src/lib.rs:1517` / `crates/server/src/stay_store.rs:1059-1097` / `web/src/DayView.tsx:325`
- 根拠: (a) R2 の回帰テスト `malformed_erased_stay_end_is_safe_for_listing_and_rebuild` は `deletion_tests.rs:485` の「`Erased` の行がある」で落ちる。`core.stay_erased` は壊れた `end` を `start` に畳む（長さ 0）ので、`day_view` の `if start >= end { continue; }` で行が捨てられ、消した時間が一覧から見えなくなる（「消した」の行も「戻す」も出ない）。(b) `stays_detail_get` は `coalesce((payload->>'end')::timestamptz, event_time)`（`lib.rs:1517`）のままで、`try_timestamptz` も `span_of` も通していない。使い捨てのテスト `zz_probe_detail_malformed_end` で**生きた**滞在の `end` を `"not-a-time"` にすると `stays_detail_get` → `Err((500, "internal error"))`、同じ日の `day_view` は `["NoRecord", "Stay"]` でその滞在を一覧に出す。画面は詳細が `ok` のときだけ「この滞在を消す」を出す（`DayView.tsx:325`）ので、**一覧に出ている滞在を消す手段が無い**。design D3 は「読み方を 2 通りにすると一覧に出ている範囲と消える範囲がずれる」として `span_of` に揃えると決めている
- kind: technical
- 提案: 詳細の滞在の範囲も `span_of`（か `core.try_timestamptz`）で読む。長さ 0 の消した滞在でも「消した」の行と「戻す」を出すか、出さないならそれを D7 に書く。R2 の処置は、回帰テストが緑になってから `fixed` にする

## R8. 消した時間帯に後から届いた位置は、取り込みの後の作り直しが 1 回落ちると、印の無いまま読み出しに残り続ける。design の「次の位置の到着で直る」は過去の日には成り立たない
- 成果物: `crates/server/src/lib.rs:1104-1139`（`rebuild_stays_after_ingest`）/ `crates/server/src/stay_store.rs:568, 626-`（`mark_late_arrivals`）/ `design.md:137, 151, 240`
- 根拠: 使い捨てのテスト `zz_probe_late_arrival_after_failed_rebuild`: 2026-08-01 10:00–11:00 の滞在を消す → 作り直しが失敗する `App` で 10:30 の位置を取り込む（`accepted:true, duplicate:false`）→ 印は `None` → 作り直しが成功する `App` で**別の日**（2026-09-29）の位置を取り込む → 10:30 の位置の印はまだ `None`、`GET /events` に `true` で出る。同じ位置を再送したときだけ（`duplicate:true` でも作り直しが走る）`user:late` になる。印付けは「取り込んだ記録の日」の作り直しの中でしか走らず（`lib.rs:1137-1138`）、端末は受理された記録を再送しないので、**過去の日に後から届いた位置は、手の作り直し（`POST /stays/rebuild` はその日を指定しない）が無い限り本人が消した場面の座標を持ったまま生きる**。design は Risks でこの穴を認めているが、緩和の根拠「次の位置の到着か手の作り直しで印が付く」（`design.md:240`）は過去の日には当たらない。`docs/handoff/ST33.md` が ST33 に渡した担保は「削除済みを出さない」で、**印の無いこの位置はそこからも漏れる**（Q1 は `loss: exported`）
- kind: technical
- 提案: 取り込みの受理の後に、消した滞在の時間帯に入るかだけを `core.stay_erased` で引いて印を付ける（滞在の錠は要らない形にできる）か、失敗した作り直しの（利用者・日）を DB に残して起動時と次の取り込みで拾い直す。どちらも取らないなら、design D6 の Risks の緩和の文を「過去の日は手で作り直すまで残る」に直し、ST33 の申し送りに「印の無い後着」を足す
- 処置: escalated — Q6（loss: exported）として `deep.md` と `deep-questions-r3.json` に追加。人間の選択後に実装方針を確定する。

## R9. 隠れた滞在だけが翌日にはみ出すと、翌日の「消した」の行は識別子 0 件になり、その「戻す」は 404 を返し続ける
- 成果物: `crates/server/src/stay_store.rs:1059-1097` / `crates/server/src/deletion.rs:104` / `web/src/DayView.tsx:297`
- 根拠: 使い捨てのテスト `zz_probe_erased_row_without_ids`（本物の作り直しを通す）: 2026-09-10 21:00〜翌 01:00 に 1 分ごとの位置（22:00–22:30 だけ別の場所）を置き、`POST /stays/rebuild` を `gap_minutes: 60`（1〜1,440 の範囲で利用者が選べる値）で呼ぶ → 滞在 3 件。22:00–22:29 の滞在を消す → 作り直しで前後がつながった 21:00–00:59 の滞在が `rebuild:erased-range` で隠れる。1 日の並びは 09-10 が `erased 12:00–15:00Z stay_ids=[<消した滞在>]`、**09-11 が `erased 15:00–15:59Z stay_ids=Some([])`**。画面はこの行にも「戻す」を出し、`{stay_ids: []}` を送る（`DayView.tsx:297`）→ `restore([])` → `Err(404)`（`deletion.rs:104`）。`day_view` は行の識別子を「その日の窓に重なる本人が消した滞在」からしか取らない（`stay_store.rs:1085-1097`）ので、窓の外の消した滞在が原因の行は戻す相手を持たない。spec「『戻す』を押したとき、その行が表す消した滞在をすべて戻し」を満たさない
- kind: technical
- 提案: 隠れた滞在（`rebuild:erased-range`）を行に入れるときは、その滞在に重なる本人が消した滞在を窓の外まで引いて `stay_ids` に入れる。少なくとも `stay_ids` が空の行に「戻す」を出さない

## R10. 戻す操作が作り直しと同じ錠を取ることを、どのテストも固定していない。錠を外しても全部通る
- 成果物: `crates/server/src/deletion.rs:108-121` / `crates/server/src/deletion_tests.rs`
- 根拠: 複製で `restore` の `pg_advisory_xact_lock($2, hashtext(user_id::text))` を `($2::bigint IS NULL)` に置き換え（M14）→ `cargo test deletion_tests` は基準と同じ 3 本だけが落ち、他の 17 本は通る。消す側は同じ変異（M4）で `erase_locks_against_rebuild` が 3 回とも落ちる。tasks の Global Constraints「印を書くトランザクションは先頭で `pg_advisory_xact_lock(...)`」と spec「印を書くまとまりを、派生の作り直しと同時に走らせない」は戻す側にも掛かるが、`erase_locks_against_rebuild` に当たる戻す側のテストが無い
- kind: technical
- 提案: `erase_locks_against_rebuild` と同じ形で、戻す操作と作り直しを同時に始めて、戻したことが残り作り直しが 1 回で終わることを見るテストを足す

## R11. 本人の決定 Q3「稼働状況は消した記録も数える」のうち、途絶の判定に使う前後の活動は固定されていない。`active_days` を `core.event_live` にしても通る
- 成果物: `crates/server/src/coverage.rs:902`（`active_days`）/ `crates/server/src/coverage/tests/achievement.rs`（`coverage_counts_deleted`）
- 根拠: 複製で `active_days` の `FROM core.event` だけを `FROM core.event_live` にする（M5b）→ `coverage_counts_deleted` は `1 passed`。この変異が観測できる差を生むことは、使い捨てのテスト（想定間隔 24 時間のソース、05-01 に 1 件、05-02 は空）で確かめた: 変異なしでは 05-01 の記録を消した前後で 05-02 は `AliveNoRecord` のまま（テスト緑）、変異ありでは同じテストが落ちる（05-02 の状態が変わる）。`coverage_counts_deleted` は消した日そのものの状態と達成日数しか見ておらず、**記録を消すと隣の空白日が別の状態に変わる**実装に変わっても気づけない（ST22 は `coverage.rs` を変えていないので、いまの実装は正しい）
- kind: daily
- 提案: `coverage_counts_deleted` に「消した日の隣の、記録の無い日の状態が変わらない」を 1 行足す（想定間隔を 1 日以上にしたソースで）

## R12. Scenario「キーボードで詳細を開ける」の印の先はキーを押していない。行のボタンで Enter / Space を止めても 152 本が緑
- 成果物: `web/src/__tests__/DayView-erase-erased-row-keyboard.test.tsx:233-241` / `web/src/DayView.tsx:276-283`
- 根拠: 印の先のテストは `fireEvent.click(button)` を 2 回呼ぶだけ。複製で行のボタンに `onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") e.preventDefault(); }}` を足しても（W2）、`npx vitest run` は `22 passed / 152 passed`。e2e（`web/e2e/`）でキーを押すのは日の移動の Tab（`day-stays.spec.ts:77`）だけで、滞在の行を開く検査は無い。spec の WHEN は「キーボードで滞在の行へフォーカスを移して決定する」。tasks の方針「画面の Scenario を人間の確認待ちへ逃がさない」に対し、本物のブラウザで測れる操作が jsdom の click で代わりに済まされている
- kind: technical
- 提案: `day-erase.spec.ts` に「Tab で滞在の行へ移り、Enter で `aria-expanded="true"` になり `stay-detail` が見える」を足す（`page.keyboard.press`）。印はそちらへ移す

## R13. `deletion_ledger` の down は同じ移行が足した索引を残し、`check-immutable.sh` の down の確認は表とビューしか見ない
- 成果物: `migrations/202609271716_deletion_ledger.down.sql` / `tools/check-immutable.sh:942-949`
- 根拠: R3 の処置で `event_by_user_time_live`（`core.event` 上の索引）を ledger の移行へ統合したが、`.down.sql` に `DROP INDEX` は無い。移行を当てた DB で `.down.sql` と同じ文をトランザクションの中で当てると `index_left=true`。また複製で `.down.sql` から `DROP FUNCTION IF EXISTS core.try_timestamptz(text);` を消しても `tools/check-immutable.sh` は rc=0（`書き換え禁止 OK`）。確認は `to_regclass('core.deletion_ledger') IS NULL AND to_regclass('core.stay_erased') IS NULL` だけ（`check-immutable.sh:944`）
- kind: technical
- 提案: `.down.sql` に `DROP INDEX IF EXISTS core.event_by_user_time_live;` を足し、`check-immutable.sh` の down の確認に索引と `core.try_timestamptz` の不在を足す

## 実行したコマンド一覧

- `DATABASE_URL=…55422 cargo test --workspace`（2 回）/ `cargo test -p ashiato-server deletion_tests`（2 回）/ 同じものを `git archive 5630e31` の複製で
- tasks の `CT` 23 本（`cargo test -q -p ashiato-server <絞り込み>`）
- `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings`
- `tools/check-migrations.sh` / `check-openapi.sh` / `check-boundaries.sh` / `check-immutable.sh`（`COMPOSE_FILE=/tmp/st22v/docker-compose.yml COMPOSE_PROJECT_NAME=st22verify`）
- `python3 scripts/check_scenarios.py . st22-record-deletion` / `check_chain.py .` / `review_triage.py . st22-record-deletion` / `openspec validate st22-record-deletion --strict`
- `cd web && npm run lint` / `npx vitest run` / `npm run build` / `npm run test -- stays|DayView|erase|erased-row|keyboard`
- `cd web && COMPOSE_PROJECT_NAME=st22e2e … BIND=127.0.0.1:18722 WEB_PORT=5192 STACK_RESET=1 npm run test:e2e`
- `COMPOSE_PROJECT_NAME=st22smoke … BIND=127.0.0.1:18723 tools/smoke.sh`
- 複製 `/tmp/st22v/mut` での変異 M1〜M18・W1・W2・D1（`/tmp/st22v/mutate.py`）と使い捨てのテスト `zz_probe_detail_malformed_end` / `zz_probe_overlap_real_flow` / `zz_probe_late_arrival_after_failed_rebuild` / `zz_probe_erased_row_without_ids` / `zz_verify_neighbor_of_erased_day`
- python `colorsys` による配色の再計算
