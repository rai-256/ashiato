# CH05 — 試験の DB が打ち切った run の残りで膨れないようにする

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違ったのは Q7 だけで（2026-10-25）、`spec --answers Q7` が片付け（What Changes の 5 つ目）を書き足した。Q1〜Q6 は推奨と同じ。
> CH04 の design・spec は `feat/ch04-test-db-growth` の `e27a521a`（Task 1〜8 済み・9〜10 が残り）で読んだ。CH04 の決定は D 番号で引き、本文を写さない。

## Why

worktree ごとのテスト用 DB の `ashiato` database が試験の残りで膨れ（ch03 214 MB・ch02 206 MB・st23 110 MB。作ったばかりは 10 MB 前後）、後の試験が数百倍遅くなって下流が止まった（`docs/changes/CH05.md`）。
残りの大半は殺された回の分ではなく、**正常に終わった回も**共有の `ashiato` に残した行で（`premises.md` の 1 行目）、その止め方は走っている CH04 が実装している（回ごとの database・次の回での片付け）。
CH04 が持たないのは、各 worktree の `ashiato` に**いま積もっている**分と、殺された回の後を確かめること（CH04 の台本は 2 回とも最後まで走る回を見る。CH04 Q4）。CH05 はその 2 つだけを持つ。

## What Changes

- **検証の準備で、膨れた `ashiato` を作り直す**（Q2・Q4 の推奨）。`tools/verify-env.sh` が新しい台本 `tools/test-db-rebuild.sh` を呼び、`ashiato` database の大きさが閾値（64 MiB。仮）を超えていれば、
  database だけを消して空で作り直す（コンテナ・ネットワーク・volume には触らない。移行は使う側が当てる）。作り直したことは標準エラーに 1 行出す。CH04 の後は試験が `ashiato` に書かないので、作り直しは各 worktree でほぼ 1 回になる
- **使っている途中は作り直さない**（Q3 の推奨）。走っている回の助言ロック（CH04 D1）・回の作成と片付けの直列化のロック（CH04 D2）・`ashiato` への接続のどれかがあれば、作り直さずに見送り、検証はそのまま続ける。
  判定から作り直しの終わりまで直列化のロックを持ち、消すときは接続を切らない（`WITH (FORCE)` を付けない）ので、隙間に繋いだ接続があれば PostgreSQL が断って見送りになる
- **作り直してよいのは Story・change の作業ツリーだけ**（Q5 の推奨）。main の checkout・確認バッチ（`-verify`）・それ以外では大きさを出すだけ。作業ツリーの種類は `tools/ports.sh` の規則から取る（規則を 2 か所に書かない）
- **殺した後の次の回を確かめる台本 `tools/check-test-db-killed.sh`（`--self-test` 付き）**（Q6 の推奨）。体積の行を入れる探りの試験（`#[ignore]`）を、文の途中と、確定した後・試験の終わりの前の 2 時点で SIGKILL し、
  次の回の後に殺した回の database が残らず、`ashiato` の行数と大きさが変わらないことを見る。秒数は出すだけで合否に使わない。CI の rust job で走らせる
- **消えた作業ツリーのコンテナとネットワークを片付ける**（Q7 の本人の答え。推奨と違う）。検証の準備が DB を起こす前に新しい台本 `tools/docker-tidy.sh` を呼び、作業ツリーが消えたことを機械で確かめられるもの
  （compose の印の path が無く `git worktree list` にも無い、`<main>-` の project のコンテナと、コンテナの無いその project のネットワーク）だけを外す。動いているコンテナも外す。main の checkout・確認バッチ・印の無いもの・別の repo のものには触らない。
  **volume は 1 つも消さない**（片付けたコンテナの名前の無い volume も残る）。片付けが落ちても検証の準備は続く。`--dry-run` と `--self-test` を持ち、CI の rust job で self-test を走らせる
- **完了の条件の書き方を量に直す**（Q6 の推奨）。依頼の「次の `cargo test` の速さが変わらない」を、速さを決める量（殺した回の残り・`ashiato` の行数と大きさ）で判定する形に `docs/changes/CH05.md` で書き直す（依頼の逐語は消さない）
- **振る舞いを変えないもの**: CH04 が決めた試験の足場（`testdb` の形・回の database・片付け・`tools/check-test-db-growth.sh`）、消せない表のトリガ、試験の期待値と `// Scenario:` の印、
  `tests/server_startup.rs`・縦串（`tools/smoke.sh`）・開発用の起動（`tools/dev.sh`）が `ashiato` を使うこと、`tools/ports.sh` の port の規則、既存の 4 つの台本（smoke・stack・check-immutable・check-panic-log）の `down -v`
- やらないこと（Q7）: docker の volume の片付け（浮いた volume は印が無く、どの作業ツリーのものか区別できない）。既存の `down -v` の台本に見送りを当てること。ST23 で出た lock timeout・権限のエラーの原因調べ。design の Non-Goals

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `test-database`: Requirement を 4 つ足す（ADDED）—— 膨れた database を検証の準備が作り直す・使っている途中は作り直さない・殺した後の次の回で残りが無いことを台本と CI が確かめる・消えた作業ツリーのコンテナとネットワークを検証の準備が片付ける（Q7）。
  片付けは試験用 DB を立てる足場の振る舞い（作業ツリーの DB が立つこと）なので、新しい capability を立てずに同じ `test-database` に置く（change は capability を持たない —— `docs/changes/CH05.md` の `capability: []`）。
  `test-database` は CH04 が立てる capability で、まだ正典に無い（CH04 の delta にだけある）。CH05 は `## Purpose` を持たず、CH04 の Requirement を MODIFIED しない（別の名前で足すだけ。CH04 の本文を写さずに済み、CH04 の gate・fix で本文が動いても追従が要らない）。
  archive は CH04 の後（`requires`）。schema の「tooling は `skip_specs`」は採らない（CH03・CH04 と同じ理由: 作り直す・見送る・残りが無い、は外から見える振る舞いで、Scenario を `check_scenarios.py` が印に縛る）

## Impact

- 道具（新しいファイル）: `tools/test-db-rebuild.sh`（`--self-test` 付き）・`tools/check-test-db-killed.sh`（`--self-test` 付き）・`tools/docker-tidy.sh`（`--dry-run`・`--self-test` 付き。Q7）
- 道具（既存）: `tools/verify-env.sh`（DB を起こす前に `tools/docker-tidy.sh` を呼ぶ 1 行、役割を整える前に `tools/test-db-rebuild.sh` を呼ぶ 1 行と注記）、`tools/ports.sh`（作業ツリーの種類を 1 つの変数で出す。port の値と規則は変えない）
- コード（試験の側）: 新しいファイル `crates/server/src/killed_probe_tests.rs`（探りの試験。`#[ignore]` の 1 本と、次の回の 1 本。名前の頭は `ch05_`）と `lib.rs` の試験のモジュールの並びに 1 行。`testdb.rs` は変えない
- CI: `.github/workflows/ci.yml` の rust job の後ろに 4 行（片付けの self-test を含む）（CH03・CH04 が同じ job に行を足している。既存の行は変えず、後ろに足す）
- 文書: `docs/testing.md` に 1 節、`docs/flow-gates.md` の検査の表に 3 行、`docs/changes/CH05.md`（価値・壊してはいけないもの・完了の判定）
- 依存: 足さない。移行は足さない
- **下流の前提 —— この作業ツリーの DB が立つこと**: 2026-10-24 の時点で docker の既定のネットワークの割り当てが尽きていて、新しい compose の project は DB を立てられなかった（`premises.md:15`。この上流の作業ツリーでも立たなかった）。
  2026-10-25 にはコンテナの無いネットワークが外されていて尽きていない（`premises.md:18`）が、消えた作業ツリーのコンテナ（15 個。動いたままのものが 4 つ）は残り、作業ツリーが消えるたびにまた溜まる。
  Q7 の答えで片付けが CH05 に入ったので、下流は片付け（tasks Task 0）を DB を使う Task より先に行う。片付けの後も立たなければ BLOCKED_INFRA として返す
- **片付けが外すもの（Q7）**: この機械では、作業ツリーの path が無い `ashiato2-*` のコンテナ 15 個（`premises.md:16`）。片付けたコンテナの volume は残る（`premises.md:17`）。外すものの一覧は下流が `--dry-run` で design の「計測の記録」に写してから片付ける（tasks 0.2）
- **CH04 に依る**（Q1 の推奨）: 回の database の名前（`ashiato_run_<12 桁>`）・回の助言ロック（`4820253`）・直列化のロック（`4820252`）・次の回の初めの片付け（CH04 D1・D2・D6）の上に積む。
  `requires: [CH04]` は、Q1 の答えの後に record が `docs/changes/CH05.md` の front matter へ書く（questions の段の `review/deep.md` R1 の処置）。CH04 の下流は Task 9〜10 が残り、gate を通っていない。
  下流の入口は CH04 の gate を通った SHA を取り込む（`story_deps.py`。archive は待たない）
- **走っている Story・change との重なり**（`origin/main` = 81b353d4 との差分）:
  - `tools/verify-env.sh`・`tools/ports.sh`: 走っている feat/* で触るものは無い（`git diff --name-only origin/main...<branch>` で確かめた）。
    **2026-10-25 の追記**: CH06（検証のたびにテスト用 DB を作り直す。上流は main の 2b924da4 に入り、`feat/ch06-reset-verify-db` が走っている）が `tools/verify-env.sh`・`tools/ports.sh`・`tools/env-init.sh` を触る。重なりと CH05 の範囲は第 2 回の問い Q8 で本人に問う（`deep.md` R20）。
    片付け（Q7）は CH06 の self-test の compose の project（`ch06-sentinel-<乱数>`）を、project の名前を `<main>-` に絞って外さない（design D9）。
    `tools/ports.sh` は port の分岐を種類で分ける形に書き換えるが、port の値は変えない（`tools/check-ports.sh --self-test`）
  - `.github/workflows/ci.yml` の rust job: CH04（`tools/check-test-db-growth.sh` の 3 行）・ST29（`check-satellite-boundary.sh` の 2 行）。どちらも rust job の行の追加。CH05 は CH04 の行の後ろに足す。後から入る側が rebase で追従する
  - `docs/testing.md`: CH04（1 節）・ST29（2 行）。CH05 は 1 節を足す
  - `crates/server/src/lib.rs` の試験のモジュールの並び: CH04（`testdb_tests`）・ST06・ST22・ST23・ST29・ST41 が行を足す。CH05 も 1 行足すだけ
  - 走っている Story へ差し戻すものは無い
