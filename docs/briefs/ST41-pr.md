# ST41 幅のある出来事に終わりの時刻を持たせる

change: `openspec/changes/st41-event-end-time/`。満たす要件は FR-91（扉 #28）。触る capability は `record-envelope`・`data-durability`・`collection-coverage`。
記録に終わりの時刻（`end_time` / `end_tz_offset_min` / `end_tz_id` の 3 列）を足し、取り込み口・錠・期間の読み出し（`GET /events?from&to&source`）・稼働記録・書き出しをそれに合わせた。
収集アプリ（`collector-android` / `crates/collector-windows`）と滞在の作り直し（`stay_store.rs` の書き込み）は変えていない。

## deep.md に積んだ未決（A）

なし。第 1 回の Q1〜Q7 は本人が答え済み（2026-10-23。7 問とも推奨と同じ）。final・code_verify・security・/code-review の指摘で A に上げたものは無い。

## 仮決め（B）と Ruling

design の（仮）と反転条件（詳しくは `design.md`）:

- **D4（仮）** 終わりが始まりより前の記録は送られたまま入れ、読む側（期間・書き出し・消した滞在の時間）では使わない（Q1）
- **D8（仮）** 期間も記録も半開区間 `[始まり, 終わり)`。長さ 0・逆順は点（Q2）
- **D9（仮）** 期間で引く口は `GET /events` に足す（Q3）
- **D12（仮）** 日の区切りは Asia/Tokyo のまま、件数は始まりの日に 1 件（Q4）。1 日の並びは ST39 へ申し送り
- **D13（仮）・D14（仮）** 滞在・収集済みの記録の内容にある終わりは列に写さない（Q5・Q6）
- **D16（仮）** 期間の求めに件数の上限を置かない（ST45 が衛星向けに使うなら足す）
- **D17（仮）** 書き出しの「列の終わりを先に読む」は ADDED のまま（CH02 が同じ要件を MODIFIED しているため）
- **D18（仮）** 内容が同じで終わりを持たない到着は保存済みの終わりを消さない
- **D19（仮）** 主張・主観・場所の錠も終わりの 3 列を凍結する（final review R2。台帳 `provisional.jsonl` に載せた。merge gate の前に deep-review が確かめる）

全 Task の ledger の `Ruling:`（`.superpowers/sdd/st41-task-*/progress.md`。Ruling があるのは Task 5 だけ）:

- Task 5: `Ruling: st25 の鍵の検査の絞り込みは受け入れ待ち — 元のアサートは共有 DB で成立しない — 受け入れられなければ premise の A`
- Task 5（parked F1）: `Ruling: 人間が絞り込みを受け入れた（2026-10-24）。期待値（鍵 6 つ）は不変で、元の assert は D9 のもとで共有 DB では成立しない。5.2 の grep は原理的に空にならない計画の欠陥。final review が見る`
  —— final review R12 で `rejected`（裁定のまま park。`rebuild_all` の `s01-stay` の行の鍵を見なくなった点は残る）

final review の scoped re-review で park したもの（指摘にはしない）:

- `Ruling:` 878d10a0 の全体の `cargo test` の 2 本の落ち（`archive_scan_interval_change_takes_effect_without_restart`・`stay_rebuild_chain_of_short_moves_settles`。時間に依る試験で単独では通る）は fix と無関係の負荷依存の揺れとみる。code_verify が全体の走りで rc=0（1452 passed）を取り直した
- 取り込み口の更新の経路（`lib.rs` の外部の識別子の更新）で、本人が書いた行に終わりだけ違う版が届くと錠が断り 500。始まりの列でも既にある振る舞いで、新しい穴ではない

## Task

7 Task・18 項目をすべて埋めた（`- [ ]` の残り 0。`ci` / `human` の残りも無い）。画面を持たないので e2e と人間の確認待ちは無い。

## 計画外の変更（final review R13）

- 過去の移行 `migrations/202609120943_version_and_ledger.sql` の `event_version_live` を、無ければ作るだけにした（`to_regclass` で囲む）。`migrate()` は起動のたびに全版を当て直すので、ST41 が列を足した後のビューを古い形で置き換えようとして落ちるため
- `tools/check-openapi.sh` の収集アプリの欄名の照合から、終わりの 3 欄を名指しで外した（収集アプリは送らない —— D3。Task 2 F2）
- `export_ledger_tests.rs`（`export_ledger_migration_applies_twice`）と `media_migration_tests.rs`（`st09_mig_applies_twice_and_is_last` → `…_is_in_order`）の `.last()` → `.iter().any()`。ST41 の移行が ORDER の末尾に付き、「MIGRATIONS の末尾」を見る形が落ちたため。期待（その版が MIGRATIONS にある・2 回当てても壊れない）は同じ（`agreed-test-changes.md`）
- `tools/check-immutable.sh` の本体: 戻し手順の節の頭に ST41 の版の戻しを足し、凍結の点呼に終わりの 3 列を足した（D19）。`.d/` の節は `tools/check-immutable.d/202610231200_st41-event-end-time.sh`
- `day_tests.rs` の `st25_reasons_before_collection_started` の読む日を 2001-01-05 → 1990-01-05（無いときに仮に置く開始日を 2002 → 1995）。書き出しの試験が `testdb::unique_day()`（2000〜2004 年）に位置を置くたびに `c01-location` の `collection_started_on` が下がり、手元の DB に積もると 2000-02-17 になって「開始前」が付かず毎回落ちた（main にもある。ST41 の試験の位置は 2026 年で関わらない）。期待（開始前の日の記録なしに「開始前」が添えられる）は同じ。積もった DB のまま、直す前は rc=101・直した後は rc=0
- finish での直し（下の「レビュー」）: `core.stay_erased` の作り直し・移行の頭の写し元の照合（R21）・`stay_store::event_end_sql`・`GET /events` の SQL の分け方

## 検証

`evidence.jsonl` の各項目の最後の記録（`scripts/verify-run`）:

| 項目 | コマンド | 終了コード |
|---|---|---|
| 1.1 | `./tools/check-migrations.sh`・`cargo build -p ashiato-server` | 0・0 |
| 1.2 | `cargo test -p ashiato-server st41_gate_` | 0 |
| 1.3 | `./tools/check-immutable.sh`・`./tools/check-shared-appends.sh` | 0・0 |
| 2.2 | `cargo test … st41_ingest_`・`… dedup_` | 0・0 |
| 2.3 | `./tools/check-openapi.sh` | 0 |
| 3.1 | `cargo test -p ashiato-server --lib st41_hash` | 0 |
| 3.3 | `cargo test … st41_update_`・`… st41_dedup_`・`… dedup_` | 0・0・0 |
| 4.1 | `cargo test … st41_coverage_` | 0 |
| 5.2 | `cargo test … st41_range_`・`… st25_api_unchanged` | 0・0 |
| 5.3 | `cargo test … st41_range_uses_partial_index`・`./tools/check-openapi.sh` | 0・0 |
| 6.2 | `cargo test … st41_export_`・`… export_` | 0・0 |
| 7.1b | `cargo test … st41_stays_and_collected` | 0 |
| 7.1 / 7.2 | （finish の後の取り直し。下の「finish の後」） | |

7.2 の 878d10a0 での記録は fmt・clippy・check 4 本・smoke・`check_scenarios.py` が 0、全体の `cargo test` が 101（上の Ruling の 2 本）。
code_verify（9935e831）が全体の `cargo test -p ashiato-server` rc=0（lib 1452 passed / server_startup 9 passed）・smoke rc=0（2 回目。1 回目は並走する ST23 の smoke と `/tmp/smoke.body` を取り合った）を実測した。

finish の後:

`scripts/verify-run`（HEAD bf6d88a4）:

| 項目 | コマンド | 終了コード |
|---|---|---|
| 7.1 | `python3 scripts/review_triage.py . st41-event-end-time` | 0 |
| 7.2 | `cargo fmt --all -- --check` | 0 |
| 7.2 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| 7.2 | `cargo test -p ashiato-server` | **101**（1451 passed / 4 failed。下）→ gate の後の取り直しで **0**（下） |
| 7.2 | `check-migrations && check-boundaries && check-openapi && check-immutable` | 0 |
| 7.2 | `./tools/smoke.sh` | 0 |
| 7.2 | `python3 scripts/check_scenarios.py . st41-event-end-time` | 0 |

全体の `cargo test` で落ちた 4 本は、どれも `--exact` で 1 本ずつ走らせ直すと通った（rc=0）。終わりの時刻に触れる試験は無い:
`archive_scan_interval_change_takes_effect_without_restart`（走査が 10 回に届かない: 9。時間に依る。878d10a0 の Ruling と同じ）・
`archive_erased_window_persistent_marking_failure_is_ledgered_and_throttled`（読み直しの回数 6 ≠ 3。走査の周期に依る）・
`st25_reasons_before_collection_started`（共有の DB の積もり。code_verify の注 3 と同じ）・
`export_excluded_sql_every_registered_source_is_classified_by_name`（`stay_tests.rs:1342` で別の接続を止める権限が無い `42501`）。
merge gate（PR #162）が 7.2 の全体の `cargo test` の FAIL で落ちた後の取り直し（d5eb0c93 の上）:
1 回目 101（`archive_scan_interval_change_takes_effect_without_restart` だけ。負荷平均 11〜13 で並走する木の試験と重なった）・
2 回目 101（`st25_reasons_before_collection_started` だけ。共有の DB の積もりで決まって落ちる。上の「計画外の変更」で直した）・
3 回目 **0**（st25 の直しを入れた木。lib 1455 passed / server_startup 9 passed）。
4 本はどれも ST41 を含まない木（ch03・st11・st23・st29・ch06・verify）の証跡でも落ちている。st25 以外の 3 本は時間と負荷に依る揺れで、ここでは直さない。

finish の直しが効くことは、壊した木で確かめた: R16/R19/R18 の直しを戻すと足した 3 本が落ちる（rc=101）・
錠から前の版の `end_time` の名指しと主張のずれ・地域の行を外すと check-immutable が rc=1・移行の写し元を変えると移行が止まる（使い捨ての DB）。
関わる試験（`st41_` `export_` `mark_late` `stay_erased` `ingest::`）は 260 本 rc=0。

## レビュー

- final review（`review/code.md` R1〜R13）: fixed 11（うち仮 3: D19・D12・D18）・deferred ST45 1（R4 期間の点の側の索引）・rejected 1（R12）
- code_verify（R14〜R16）: fixed 3 —— Q6 を取り込み口で観測する試験・前の版の錠の名指しを check-immutable で見る・消した滞在の逆順の列の終わり
- security review: 指摘なし
- /code-review（R17〜R23）: fixed 7 —— 節の作り直しで消える付与・削除の連鎖の終わりの読み方・凍結の点呼の終わりのずれと地域・移行の写しが前の版の変更を黙って戻すのを止める照合・`?source=` の汎用の計画（20 万行で Seq Scan を再現）・重複した固定値の試験
- `python3 scripts/review_triage.py . st41-event-end-time` rc=0

## 申し送り

- `docs/handoff/ST39.md`: 外部の識別子を持たない送り手は終わってから送る・始まりの側のずれと TZ の検査が無い既存の穴・1 日の並びに衛星のソースを足すときの読み方
- ST45: 期間で引くときの点の側の索引（または利用者で絞る）—— `review/code.md` R4 の `deferred ST45`（`story_brief.py` が ST45 のブリーフへ拾う）
- CH02: CH02 が先に main に入ったので `docs/handoff/CH02.md` は書いていない（tasks 7.1）
