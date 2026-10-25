# CH05 — 試験の DB が打ち切った run の残りで膨れないようにする

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違ったのは Q7 だけで（2026-10-25）、`spec --answers Q7` が片付けを書き足した。Q1〜Q6 は推奨と同じ。
> **第 2 回の Q8（2026-10-25。本人は推奨を選んだ）で範囲を狭めた**: 膨れた `ashiato` の作り直しと、使っている途中の見送り（第 1 回の Q2〜Q5）は外し、検証のたびに DB を空から作り直す CH06 に任せる。
> CH05 は CH06 の上に積む（`requires: [CH04, CH06]`）。CH06 はまだ gate を通っていないので、この版は **proposal まで**（`spec --until proposal --answers Q8`）。
> specs・design・tasks は CH06 の gate の後に、CH06 の gate 済みの spec を読んで書き直す。Q8 の前の版（Requirement 4 つ・D1〜D9・Task 0〜6）は `837e79be` にある ——
> 残る 2 つ（殺した後の検査 D6・片付け D9 と、その Task 0・3・4・5）の設計はそこから引き継ぐ。
> CH04 の design・spec は `feat/ch04-test-db-growth` の `9fe8c5e`（PR #156 で gate 済み。`deps.json`）で読んだ。CH04 の決定は D 番号で引き、本文を写さない。

## Why

worktree ごとのテスト用 DB の `ashiato` database が試験の残りで膨れ（ch03 214 MB・ch02 206 MB・st23 110 MB。作ったばかりは 10 MB 前後）、後の試験が数百倍遅くなって下流が止まった（`docs/changes/CH05.md`）。
残りの大半は殺された回の分ではなく、**正常に終わった回も**共有の `ashiato` に残した行で（`premises.md` の 1 行目）、その止め方は CH04 が実装している（回ごとの database・次の回での片付け）。
各 worktree の `ashiato` に**いま積もっている**分は、CH06 が検証の準備（`HARNESS_VERIFY=1 tools/verify-env.sh`）のたびに DB を空から作り直すので消える（CH06 の本人の答え Q2。CH05 の Q8）。
CH04 と CH06 のどちらも持たないのは 2 つ —— 殺された回の後に残りが無いことを確かめること（CH04 の台本は 2 回とも最後まで走る回を見る。CH04 Q4）と、
作業ツリーが消えた後に残る docker のコンテナとネットワークの片付け（CH06 の Non-Goals。Q7）。CH05 はその 2 つだけを持つ。

## What Changes

- **殺した後の次の回を確かめる台本 `tools/check-test-db-killed.sh`（`--self-test` 付き）**（Q6 の推奨）。体積の行を入れる探りの試験（`#[ignore]`）を、文の途中と、確定した後・試験の終わりの前の 2 時点で SIGKILL し、
  次の回の後に殺した回の database が残らず、`ashiato` の行数と大きさが変わらないことを見る。秒数は出すだけで合否に使わない。CI の rust job で走らせる。
  Q8 の前の版にあった「探りが眠っている間の作り直しは見送る」の確かめは外す（作り直しを作らないので）
- **消えた作業ツリーのコンテナとネットワークを片付ける**（Q7 の本人の答え。推奨と違う）。検証の準備が DB を起こす前に新しい台本 `tools/docker-tidy.sh` を呼び、作業ツリーが消えたことを機械で確かめられるもの
  （compose の印の path が無く `git worktree list` にも無い、`<main>-` の project のコンテナと、コンテナの無いその project のネットワーク）だけを外す。動いているコンテナも外す。main の checkout・確認バッチ・印の無いもの・別の repo のものには触らない。
  **volume は 1 つも消さない**（片付けたコンテナの名前の無い volume も残る）。片付けが落ちても検証の準備は続く。`--dry-run` と `--self-test` を持ち、CI の rust job で self-test を走らせる。
  呼ぶ 1 行は CH06 が書き換えた `tools/verify-env.sh` の上に積む（Q8）
- **完了の条件の書き方を量に直す**（Q6 の推奨）。依頼の「次の `cargo test` の速さが変わらない」を、速さを決める量（殺した回の残り・`ashiato` の行数と大きさ）で判定する形に `docs/changes/CH05.md` で書き直す（依頼の逐語は消さない）
- **Q8 で外したもの**（第 1 回の Q2〜Q5 の答え。本人の答えは `deep.md` に残る）: 膨れた `ashiato` の閾値（64 MiB）での作り直し・走っている回と接続での見送り・作り直してよい作業ツリーの範囲。
  新しい台本 `tools/test-db-rebuild.sh` と、`tools/ports.sh` に作業ツリーの種類を足すことも作らない。依頼の 2（膨れたら作り直す）は CH06 が満たす（`docs/changes/CH05.md` の完了の判定）
- **振る舞いを変えないもの**: CH04 が決めた試験の足場（`testdb` の形・回の database・片付け・`tools/check-test-db-growth.sh`）、CH06 が決めた検証の準備（印での DB の作り直し・`tools/ports.sh`・`tools/env-init.sh`・subnet）、
  消せない表のトリガ、試験の期待値と `// Scenario:` の印、`tests/server_startup.rs`・縦串（`tools/smoke.sh`）・開発用の起動（`tools/dev.sh`）が `ashiato` を使うこと、既存の 4 つの台本（smoke・stack・check-immutable・check-panic-log）の `down -v`
- やらないこと（Q7・Q8）: docker の volume の片付け（浮いた volume は印が無く、どの作業ツリーのものか区別できない）。膨れた DB の作り直しと見送り（CH06）。ST23 で出た lock timeout・権限のエラーの原因調べ

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `test-database`（CH04 が立てる。まだ正典に無い）: Requirement を 2 つ足す（ADDED）—— 殺した後の次の回で残りが無いことを台本と CI が確かめる・消えた作業ツリーのコンテナとネットワークを検証の準備が片付ける（Q7。置き場は Q8 の前の版と同じ）。
  CH05 は `## Purpose` を持たず、CH04 の Requirement を MODIFIED しない（別の名前で足すだけ）
- `verify-environment`（CH06 が立てる。まだ正典に無い）: CH06 の Requirement「印があれば、検証の準備がこの worktree の DB を空から作り直す」の「ほかの compose project のコンテナ・volume と、compose の外のコンテナ・volume には触らない」を、
  **作業ツリーが消えたことを機械で確かめられる `<main>-` の project のコンテナ・ネットワークを除く**形に MODIFIED で書き直す（Q8。review/spec.md R21。volume に触らないことは変えない）。
  本文と Scenario は CH06 の gate 済みの spec（`story_deps.py sync-specs` が読む SHA）から写す —— いま main にある CH06 の上流の spec は gate の fix で動きうるので、この版では写さない
- archive は CH04 と CH06 の後（`requires`）。schema の「tooling は `skip_specs`」は採らない（CH03・CH04 と同じ理由: 残りが無い・片付ける・volume を消さない、は外から見える振る舞いで、Scenario を `check_scenarios.py` が印に縛る）

## Impact

- 道具（新しいファイル）: `tools/check-test-db-killed.sh`（`--self-test` 付き）・`tools/docker-tidy.sh`（`--dry-run`・`--self-test` 付き。Q7）
- 道具（既存）: `tools/verify-env.sh`（DB を起こす前に `tools/docker-tidy.sh` を呼ぶ 1 行と注記。CH06 の版の上に積む）。`tools/ports.sh` は触らない（Q8）
- コード（試験の側）: 新しいファイル `crates/server/src/killed_probe_tests.rs`（探りの試験。`#[ignore]` の 1 本と、次の回の 1 本。名前の頭は `ch05_`）と `lib.rs` の試験のモジュールの並びに 1 行。`testdb.rs` は変えない
- CI: `.github/workflows/ci.yml` の rust job の後ろに、殺した後の検査と片付けの self-test の行（CH03・CH04 が同じ job に行を足している。既存の行は変えず、後ろに足す）
- 文書: `docs/testing.md` に 1 節、`docs/flow-gates.md` の検査の表に 2 行、`docs/changes/CH05.md`（価値・壊してはいけないもの・完了の判定）
- 依存: 足さない。移行は足さない
- **CH06 に依る**（Q8）: `requires: [CH04, CH06]`（`docs/changes/CH05.md` の front matter。record が書いた）。片付けの 1 行は CH06 の `tools/verify-env.sh` の流れ（印での作り直し・subnet の割り当て）の中に置く。
  CH06 の下流 `feat/ch06-reset-verify-db` は gate を通っていない。下流の入口は CH06 の gate を通った SHA を取り込む（`story_deps.py`。archive は待たない）。
  **specs を書く段で突き合わせるもの**: CH06 の Requirement「worktree ごとの DB の network は、木ごとに被らない subnet で立つ」も、同じ subnet を持つコンテナの無い network（消えた worktree の残り）を消す。
  CH05 の片付けのネットワークの側（コンテナの無い `<main>-` の project のネットワーク）と外すものが一部重なる。specs を書く段で CH06 の gate 済みの spec と突き合わせ、両方の Requirement が食い違わないように書く。
  Q7 の範囲（消えた作業ツリーのコンテナと、コンテナの無いその project のネットワーク）は狭めない
- **CH04 に依る**（Q1 の推奨）: 回の database の名前（`ashiato_run_<12 桁>`）・回の助言ロック（`4820253`）・直列化のロック（`4820252`）・次の回の初めの片付け（CH04 D1・D2・D6）の上に殺した後の検査を積む。
  下流の入口は CH04 の gate を通った SHA（`9fe8c5e`。PR #156）を取り込む
- **下流の前提 —— この作業ツリーの DB が立つこと**: 2026-10-24 の時点で docker の既定のネットワークの割り当てが尽きていて、新しい compose の project は DB を立てられなかった（`premises.md:15`）。
  CH06 が作業ツリーごとに subnet を明示するので、割り当ての枯渇には頼らなくなる（CH06 design D5・D6）。消えた作業ツリーのコンテナ（2026-10-25 に 15 個。動いたままのものが 4 つ。`premises.md:16`）は CH06 の後も残り、作業ツリーが消えるたびに溜まる
- **片付けが外すもの（Q7）**: この機械では、作業ツリーの path が無い `ashiato2-*` のコンテナ 15 個（`premises.md:16`）。片付けたコンテナの volume は残る（`premises.md:17`）。外すものの一覧は下流が `--dry-run` で design の「計測の記録」に写してから片付ける
- **走っている Story・change との重なり**（`origin/main` = 2b924da4 の時点）:
  - `tools/verify-env.sh`・`tools/ports.sh`・`tools/env-init.sh`: CH06（上流は main の 2b924da4。下流 `feat/ch06-reset-verify-db` が走っている）が書き換える。Q8 で解いた ——
    CH05 は CH06 の上に積み、作り直しの規則・`tools/ports.sh`・`tools/env-init.sh` に触らず、`tools/verify-env.sh` に片付けの 1 行を足すだけ。
    片付け（Q7）は CH06 の self-test の compose の project（`ch06-sentinel-<乱数>`）を、project の名前を `<main>-` に絞って外さない（Q8 の前の版の design D9）
  - `.github/workflows/ci.yml` の rust job: CH04（`tools/check-test-db-growth.sh` の 3 行）・ST29（`check-satellite-boundary.sh` の 2 行）。どちらも rust job の行の追加。CH05 は CH04 の行の後ろに足す。後から入る側が rebase で追従する
  - `docs/testing.md`: CH04（1 節）・ST29（2 行）。CH05 は 1 節を足す
  - `crates/server/src/lib.rs` の試験のモジュールの並び: CH04（`testdb_tests`）・ST06・ST22・ST23・ST29・ST41 が行を足す。CH05 も 1 行足すだけ
  - 走っている Story へ差し戻すものは無い（CH06 の Requirement の書き直しは、CH05 の delta の MODIFIED で CH06 の archive の後に当てる）
