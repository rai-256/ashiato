# CH04 — 試験用 DB に溜まるデータで試験が遅くなるのを止める

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違えば `spec --answers` が写し直す。
> spec の段で、Q2 の前提が CH02（同じ `excluded.rs` の「次の記録の引き」を直した change）を見落としていたと分かり、本人に渡す前に Q2 を書き直した（`premises.md` の P14）。
> `docs/changes/CH04.md` の「価値」「壊してはいけないもの」「完了の判定」と front matter の `requires: [CH02]` も、この段で書いた。

## Why

server の試験は、入れたデータを worktree の DB の `ashiato` という 1 つの database に残し続ける（st23 で 17.4 万行・利用者 1,461 人ぶん。`premises.md` P1）。
書き出しの除く範囲は利用者を問わず全部の行を判定する（ST33 Q7）ので、書き出しの試験が回を追うごとに遅くなり、1 本の問い合わせが 8〜23 分走った（P10）。
並走する Story の 1 段が 3 時間かかり、待ちきれない agent が範囲を絞らない pkill を打った（`docs/changes/CH04.md`）。
消せない表は DELETE も TRUNCATE もトリガが断るので、「試験の終わりに行を消す」はできない（P2。トリガは外さない —— 依頼）。

製品の側も同じ形で遅い。CH02 が「次の記録の引き」の二乗を直した後も、候補の行ごとに消した時間の一覧を舐める入れ子が残り、
記録の数 N と消した滞在の数 M がどちらも使った年数に比例して増えるので、年数の二乗で伸びる（P14）。

## What Changes

- **試験は回ごとの database に書く**（Q1 の推奨）。`testdb` が回の初めに移行を当てたテンプレートの database を 1 つ作り、それを写した回の database を、その回のほとんどの試験が共有する。
  `pool()`・`app_pool()`・`url()` の形は変えず、繋ぎ先だけが回の database になる。共有の `ashiato` には試験の記録を積まない
- **書き出しの試験だけは試験ごとの database**（Q1 の推奨）。`export_*tests.rs` の試験は、テンプレートを写した自分だけの database を使い、同じ回の他の試験の記録と消した滞在を判定しない。
  試験ごとの database は、試験が通っても panic しても、その試験の終わりに消える
- **片付けは次の回の始まりに、走っていない回のものだけを `DROP DATABASE` で消す**。走っているかは接続の数でなく、回の間ずっと持つ助言ロックで判定する（C6）。
  database ごと消すので、消せない表のトリガは撃たず、定義にも触らない（C4・P3）。試験が作る使い捨ての database（いまの `*_tmp_*` の 5 か所）も回の印を持つ名前にし、同じ片付けに載せる（C7）
- **試験の接続に問い合わせの時間の上限（`statement_timeout`）を置く**（Q3 の推奨）。詰まった問い合わせは数十分待たずに、どの試験かが分かる形で落ちる。量を測るときは環境変数で上限を外せる
- **再発を機械で止める台本 `tools/check-test-db-growth.sh`（`--self-test` 付き）を足す**（Q3 の推奨）。試験を 2 回続けて走らせ、前の回の database が残らず、
  残るのは最後の回の分だけで、試験ごとと使い捨ての database が残っていないことを見る。CI の rust job で走らせる。database を作る書き方を `testdb` の外に置かせない検査も足す
- **書き出しの除く範囲の判定から、候補の行 × 消した時間の入れ子を取る**（Q2 の推奨。CH02 の作り替えた式の上に積む）。除く記録と前の版は 1 件も変えない（C2。CH02 の式を試験に残して突き合わせる）。
  判定は「N と M を一緒に 4 倍にしても、消した時間を読む量が 8 倍未満（直す前の式では 8 倍以上）」。時間は今の量では入れ子が線形の部分に埋もれて判定にならないので（review/spec.md R1）、
  120 万行まで直す前と後を測って design に残すだけにする。件数の数え方（ST33 D13）は替えない
- **振る舞いを変えないもの**: 試験の期待値・`// Scenario:` の印・試験の名前と本数（C1）、本物の PostgreSQL・worktree ごとの port（C5）、消せない表のトリガ（C4）、
  書き出しの除く記録（C2）、除く範囲を利用者で絞らないこと（C3）、`tests/server_startup.rs` と縦串（smoke）・開発用の起動（`tools/dev.sh`）が `ashiato` を使うこと（C7）
- やらないこと: いま各 worktree の `ashiato` に積もっているデータと、使われていない Docker の volume（Q4 の推奨。この change では何もしない）。design の Non-Goals

## Capabilities

### New Capabilities

- `test-database`: 試験が DB に入れたデータを、後の試験（同じ回の他の試験も、次の回も）が読まないこと・前の回の database を次の回が片付けること・詰まった問い合わせで試験が落ちること・
  それを CI が確かめること。製品の振る舞いではなく、並走する Story と change が同じ機械の上で試験を回し続けるための、開発の道具の約束。
  CH03 が立てる `test-hygiene`（一時フォルダ）とは別にする: CH03 はまだ archive されておらず、同じ新しい capability に 2 つの change が `## Purpose` を持つ ADDED を積むと、
  archive の順で片方の Purpose が宙に浮く。試験の DB と一時フォルダは、壊れ方も直し方も別（design D13（仮））
- schema の「tooling は `skip_specs`」は採らない（CH03 と同じ理由）: 前の回の database が消える・走っている回のものは消さない・詰まった問い合わせで落ちる、は外から見える振る舞いで、
  Scenario を `check_scenarios.py` が印に縛り、正典に残る

### Modified Capabilities

- `data-durability`: Requirement を 1 つ足す（ADDED）—— 「書き出しから除く範囲の判定は、記録の行数と消した滞在の数を一緒に伸ばしても、その積で伸びない」。
  CH02 が足す「記録の行数の 2 乗で伸びない」とは別の名前にし、CH02 の delta を MODIFIED しない（CH02 の spec の本文を写さずに済み、CH02 の gate と archive の順に依らない）

## Impact

- コード（試験の側）: `crates/server/src/testdb.rs`（回・テンプレート・試験ごとの database、片付け、助言ロック、`statement_timeout`）、
  `export_tests.rs`・`export_file_tests.rs`・`export_job_tests.rs`・`export_ledger_tests.rs`・`export_excluded_tests.rs`（と CH02 の `export_window_tests.rs`）が試験ごとの database を使う形、
  使い捨ての database を作る 5 か所（`clock_tests.rs:40`・`feelings_tests.rs:16`・`stay_tests.rs:1322`・`attributes_tests.rs:209`・`places_tests.rs:14`）、
  `unique_day()` の注記（隔離の理由が変わる）
- コード（製品）: `crates/server/src/export/excluded.rs` の `excluded_sql` / `overlap_sql`（重なりの判定）。公開の形（`excluded_sql(snapshot_at)`・`ExcludedSql`・`EXCLUDED_EVENT` / `EXCLUDED_VERSION`）と呼び出し側（`dump.rs`・`summary.rs`・`ledger.rs`）は変えない。移行は足さない（design D10）
- 書き出しの試験のファイル（`export_*tests.rs`）: CH03（`tmp_dir` の持ち方で `export_tests.rs`・`export_file_tests.rs`・`export_job_tests.rs` を 71〜108 行）・st09・st13・st23・st29・st41（新しい `export_end_time_tests.rs` を含む）が触る。
  tasks 5.1 は各試験の `testdb::pool()` の行を書き換えるので、後から入る側が rebase で衝突を解く。CH04 の後に入る側の新しい書き出しの試験が `testdb::pool()` を使えば、CI の `--lint` が止めて `testdb::isolated()` を示す（design D5）
- 道具と CI: 新しいファイル `tools/check-test-db-growth.sh`・`tools/export-scaling.sh`。`.github/workflows/ci.yml` の rust job に 2 行、`docs/testing.md` に 1 節、`docs/flow-gates.md` の検査の表に 1 行
- 依存: 足さない
- **requires: [CH02]**（`docs/changes/CH04.md` の front matter）: 書き出しの直しは CH02 が窓関数に作り替えた `overlap_sql` の上に積む。CH02 の下流は PR 本文まで済んでいる。
  下流の入口は CH02 の gate を通った SHA を取り込む（`story_deps.py`。archive は待たない）
- `docs/changes/CH04.md` の `capability: []` はそのままにする（design D12（仮））
- **走っている Story・change との重なり**（`origin/main` = 268fa882 との差分）:
  - `testdb.rs`: st09（`source_media` を足す）・st13（`archive_scan_row`・`fail_ledger_source_insert` を足す）。どちらも関数を足すだけで、CH04 が変える `pool()` / `app_pool()` / `url()` の形は変えない。
    rebase の後もコンパイルは通り、足した関数は回の database で動く（st13 の試験用のトリガは回ごとに消えるので、前の回の残りに当たらなくなる）
  - `excluded.rs`: CH02（`overlap_sql` の作り替え。CH04 はこれの上に積む）・st09（一覧の行を足す）・st41（幅の (a) の読み方）。CH04 が触るのは重なりの判定の 1 か所で、一覧と (a) の行に触らない。
    後から merge する側が rebase で追従する。merge の順は決めない。走っている Story へ差し戻さない
  - `.github/workflows/ci.yml` の rust job: CH03（`cargo test --workspace` を `tools/tmp-leak-check.sh` から走らせる）・st09。CH04 は `cargo test` の行を変えず、その後ろに行を足す
  - `tests/server_startup.rs`: st13 が触る。CH04 は変えない（`ashiato` に移行を当てるまま。C7）
  - st23・st29・st41 の新しい試験: 書き出し以外は `testdb::pool()` を使う限り、そのまま回の database に載る。書き出しの試験のファイルの `testdb::pool()`（st29 が `export_file_tests.rs` に 3 か所足している）と、
    新しく database を作る書き方は、検査（tasks 6.2 の `--lint`）が rebase の後に落とし、何を使えばよいかを示す。st41 の `export_end_time_tests.rs` は `export_excluded_tests::World` を使うので、`World` が `TestDb` を持てばそのまま載る
