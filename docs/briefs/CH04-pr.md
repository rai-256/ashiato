# CH04 試験用 DB に溜まるデータで試験が遅くなるのを止める

change: `openspec/changes/ch04-test-db-growth/`。新しい capability `test-database`（ADDED 6）と、`data-durability` への ADDED 1。
依頼の逐語・価値・壊してはいけないもの・完了の判定は `docs/changes/CH04.md`。`requires: [CH02]`（書き出しの直しは CH02 の `excluded.rs` の上に積んだ）。

## 積んだ未決（A）

なし（`deep.md` に `(未回答)` は無い。Q1〜Q5 は本人の答え。Q1〜Q4 は 2026-10-24 の第 1 回、Q5 は 2026-10-25 の第 2 回。どれも推奨のまま）。

**本人に上げておくこと（A ではない。新しい change の候補）**: 件数の数え（`read_summary`。`GET /export/summary`）が行数に対して線形を大きく超えて伸びる。
120 万行・消した時間 100 で約 2000 秒（`export-scaling.sh` の summary_seconds: 15.6 → 537.2 → 2176.2 秒。直す前も 2643.1 秒）。Q2 の前提（120 万行で 3 秒超・25.5 秒）と桁が違う。
CH04 は Q2 の答え（時間は測って残し合否にしない）と Global Constraints（件数の数え方 ST33 D13 は替えない）で、ここを直していない（`review/code.md` R2）。

## 何が変わるか

- **試験は `ashiato` database に書かない。** 過程ごとに回の database（`ashiato_run_<12 桁>`。テンプレート `_tpl` の写し）を作り、次の回の始まりに、持ち主のいない回（助言ロックを取れたもの）と印の無い `*_tmp_*` を消す（D1・D2・D6）
- 書き出しの試験は試験ごとの database（`testdb::isolated()`）、移行の当て方を見る試験は空の使い捨て（`testdb::scratch()`）。どちらも Drop で消す（D4・D5）
- 試験の足場の全部の接続に `statement_timeout`（既定 60 秒・`ASHIATO_TEST_STATEMENT_TIMEOUT`。`0` で上限なし）（D7）
- `tools/check-test-db-growth.sh`: `--lint`（足場の外の `CREATE DATABASE`・書き出しの試験が回の database に繋ぐ口）と、2 回走らせて database が積もらないことを見る検査。CI の rust job に 3 行（D5・D8）
- 製品: 書き出しの除く範囲の重なりの判定を、消した時間を multirange にまとめて 1 回で引く形に（D9）。除く記録は CH02 と同じ（`ch04_overlap_matches_ch02` と security review の総当たり 116,480 通りで漏れ 0・余り 0）
- `tools/check-tests-kept.sh`: base から Scenario の印・`fn` の名前・`assert` の行が減っていないか（Q5）

## 仮決め（B）と Ruling

全 Task の ledger に `Ruling:` 行は無い（`.superpowers/sdd/ch04-task-*/` に `progress.md` が無く、`Ruling:` を grep して 0 件）。

design の（仮）と反転条件:
- **D1（仮）** 回の id を頭に持つ 4 種の database。走っている印は `postgres` database で持つ助言ロック。反転条件: 専用の接続が途中で切れる例が出たら、その回の試験を落とす形に
- **D2（仮）** 片付けと作成は 1 本ずつ、試験ごとの写しは過程の中で 1 本ずつ。反転条件: 写す 1 本が 1 秒を超えたら `STRATEGY FILE_COPY`
- **D4（仮）** 書き出しの試験は `isolated()`。反転条件: 書き出し以外にも同じ回の干渉で遅い群があれば広げる
- **D5（仮）** 使い捨ては `scratch()`、`--lint` が足場の外を落とす（`db-ok: <理由>` で通す）
- **D7（仮）** 上限の既定 60 秒。反転条件: まっさらな回で正当な試験が落ちたら、その最長の 2 倍に
- **D8（仮）** 台本は一覧を 3 回撮って比べる。CI は `ch04_` の絞り込みで 2 回（片付けの試験 `ch04_sweep_` は 2 回の走りから外す。R12）
- **D9（仮）** 重なりは multirange の `&&`。反転条件: 読んだ行の数が 8 倍の線を越えたら直す
- **D12（仮）** `CH04.md` の `capability` は空のまま / **D13（仮）** capability は `test-database` / **D14（仮）** 時間は `export-scaling.sh` で測って残す（CI では走らせない）
- **D16（仮）**（finish で足した）台本の説明の行に Scenario の印の字を書かない（`check_scenarios.py` が拾う。R16）

## Task

10 Task・30 項目すべて `[x]`。残した Task は無い（`ci` / `human` の残りも無い。画面を持たないので e2e も人間の確認待ちも無い）。

## 検証（`evidence.jsonl`。どれも exit 0・PASS）

| 項目 | コマンド（要旨） | 最後の PASS の head |
|---|---|---|
| 7.2 | `tools/check-test-db-growth.sh -p ashiato-server`（全体を 2 回。260 秒・2 回とも lib 1237 passed） | ba9b44f7 |
| 7.1・6.2・7.3 | `tools/check-test-db-growth.sh --self-test` / `--lint` / `-p ashiato-server ch04_` | ba9b44f7 |
| 10.2 | `tools/check-tests-kept.sh origin/main crates/server` | ba9b44f7 |
| 10.4 | `openspec validate --strict` / `check_scenarios.py` / `check-migrations.sh` / `cargo clippy -p ashiato-server --all-targets -- -D warnings` | ba9b44f7 |
| 3.2・5.3 | 片付けの試験の本数・`unique_day` の注記 | ba9b44f7 |
| 9.2 | `tools/export-scaling.sh`（120 万行まで。3382 秒） | 22f4f371（本体は e27a521a で計測。その後の差分は注記と self-test だけ） |
| 1.x・2.x・4.x・5.1/5.2・6.1・8.x・9.1・10.1・10.3 | 各 Task の `cargo test` の絞り込みと grep | Task ごとの commit（全体 2 回の 7.2 が上位集合） |

壊して落ちることを確かめたもの（finish の席）: 回の始まりの片付けを外す → 積もりの検査が rc=1 / `TestDb::pool` の上限を `0` に → `ch04_statement_timeout_default_is_set` が落ちる /
環境変数の名前を替える → `ch04_statement_timeout_env_is_read` が落ちる / 絞り込みが 0 本 → 積もりの検査が rc=1。

## レビュー

- final review（`review/code.md` R1〜R11）: Critical 0・Important 2・Minor 8。fix 1 回 + scoped re-review 1 回。R11 は re-review が挙げ、finish で直した
- code_verify（R12〜R16）・security review（指摘なし。安全の決め手は重なりの条件の総当たり）・`/code-review`（R17〜R26）
- finish の処置（`ba9b44f7` の 1 回の fix）: fixed 12（R11〜R17・R19・R20・R22〜R25。うち仮 1: R16 → D16）/ rejected 3（R18: DB は worktree ごと・D6 の反転条件 / R21: D1 の反転条件 / R26: D4 が Drop で待つと決めている）
- finish の scoped re-review（`ba9b44f7` だけ）: fixed の 13 件はどれも ADDRESSED、新しい Critical / Important なし。Minor 4 件は park（`review/code.md` の最後の節。`--list` が ignore の試験も数える・`--include-ignored` で子の試験が落ちる・行をまたぐ `use` を lint が拾わない・assert の比べ方が `assert` を含む行を全部見る）

## 申し送り

- `archive_flow_tests::archive_youtube_last_event_is_reported_per_logical_source`（`archive_flow_tests.rs:5485`。CH04 は触っていない。ST13 の試験）が時々落ちる: finish の 7.2 の 1 回目（ba9b44f7。`left: None right: Some("2026-09-12")`）と code_verify の席で 1 度ずつ。同じ木の 2 回目は PASS。取り込みを待たずに最終日を読んでいる疑い（確かめていない）

- 書き出しの試験が `$TMPDIR/ashiato-export-*` を残す（1 回で 1,000 個余り）。CH04 の持ち込みではなく origin/main の試験にも片付けが無い。CH03 の `HARNESS_TMP_LEAK=fail` に移ると引っかかる（code_verify の手 6・final の範囲外の気づき）
- CH04 の後に入る Story の書き出しの試験が `testdb::pool()` などを使うと、rebase の後に CI の `--lint` が止める（`db-ok: <理由>` か `testdb::isolated()` へ）
- 件数の数えの遅さ（上の「本人に上げておくこと」）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
