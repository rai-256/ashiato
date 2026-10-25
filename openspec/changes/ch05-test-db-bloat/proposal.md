# CH05 — 試験の DB が打ち切った run の残りで膨れないようにする

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違ったのは Q7 だけで（2026-10-25）、`spec --answers Q7` が片付けを書き足した。Q1〜Q6 は推奨と同じ。
> **第 2 回の Q8（2026-10-25。本人は推奨を選んだ）で範囲を狭めた**: 膨れた `ashiato` の作り直しと、使っている途中の見送り（第 1 回の Q2〜Q5）は外し、検証のたびに DB を空から作り直す CH06 に任せる。
> CH05 は CH06 の上に積む（`requires: [CH04, CH06]`）。Q8 の直後の版は CH06 の gate を待って proposal までにした。CH06 が gate を通った後（2026-10-25）、CH06 の gate 済みの spec を読んで specs・design・tasks を書き直した。
> Q8 の前の版（Requirement 4 つ・D1〜D9・Task 0〜6）は `837e79be` にある。残る 2 つ（殺した後の検査 旧 D6・片付け 旧 D9）の設計はそこから引き継ぎ、D 番号は振り直した（design の冒頭に対応）。
> CH04 の design・spec は `feat/ch04-test-db-growth` の `9fe8c5e`（PR #156）、CH06 は `feat/ch06-reset-verify-db` の `c666673`（PR #164）で読んだ（どちらも gate 済み。`deps.json`）。CH04・CH06 の決定は D 番号で引き、本文を写さない。

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
- **消えた作業ツリーのコンテナとネットワークを片付ける**（Q7 の本人の答え。推奨と違う）。検証の準備が DB を作り直す（`--plan` が `reset`）とき、DB を消す前に新しい台本 `tools/docker-tidy.sh` を呼び、作業ツリーが消えたことを機械で確かめられるもの
  （compose の印の path が無く `git worktree list` にも無い、`<main>-` の project のコンテナと、コンテナの無いその project のネットワーク）だけを外す。動いているコンテナも外す。main の checkout・確認バッチ・印の無いもの・別の repo のものには触らない。
  **volume は 1 つも消さない**（片付けたコンテナの名前の無い volume も残る）。片付けが落ちても検証の準備は続く。`--dry-run` と `--self-test` を持ち、CI の rust job で self-test を走らせる。
  呼ぶ 1 行は CH06 が書き換えた `tools/verify-env.sh` の上に積む（Q8）。印の無い呼び出しと `--plan` では呼ばない（CH06 の「印が無ければ今までどおり…だけ」を書き直さないため。design D5）
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

- `test-database`（CH04 が立てる。まだ正典に無い）: Requirement を 1 つ足す（ADDED）—— 殺した後の次の回で残りが無いことを台本と CI が確かめる。
  CH05 は `## Purpose` を持たず、CH04 の Requirement を MODIFIED しない（別の名前で足すだけ）
- `verify-environment`（CH06 が立てる。まだ正典に無い）: Requirement を 1 つ足す（ADDED）—— 作り直す検証の準備は、消えた作業ツリーのコンテナとネットワークを片付け、volume は消さない（Q7）。
  あわせて CH06 の Requirement「印があれば、検証の準備がこの worktree の DB を空から作り直す」の「ほかの compose project のコンテナ・volume と、compose の外のコンテナ・volume には触らない」に、
  **作業ツリーが消えたことを機械で確かめられる `<main>-` の project のコンテナ・ネットワークは片付けが外す**という例外の 1 文と、それを確かめる Scenario 1 本を足す（MODIFIED。Q8。review/spec.md R21。volume に触らないことは変えない）。
  本文と Scenario 5 本は CH06 の gate 済みの spec（`c666673`）から逐語で写した。片付けの置き場を Q8 の直後の版（`test-database`）から `verify-environment` に移したのは、例外の 1 文と片付けの Requirement を同じ capability で読めるようにするため（design D1）
- archive は CH04 と CH06 の後（`requires`）。schema の「tooling は `skip_specs`」は採らない（CH03・CH04 と同じ理由: 残りが無い・片付ける・volume を消さない、は外から見える振る舞いで、Scenario を `check_scenarios.py` が印に縛る）

## Impact

- 道具（新しいファイル）: `tools/check-test-db-killed.sh`（`--self-test` 付き）・`tools/docker-tidy.sh`（`--dry-run`・`--self-test`・`--verify-env-check` 付き。Q7）
- 道具（既存）: `tools/verify-env.sh`（作り直すとき、DB を消す前に `tools/docker-tidy.sh` を呼ぶ 1 行と注記。CH06 の版の上に積み、既存の行は変えない）。`tools/ports.sh` は触らない（Q8）
- コード（試験の側）: 新しいファイル `crates/server/src/killed_probe_tests.rs`（探りの試験。`#[ignore]` の 1 本と、次の回の 1 本。名前の頭は `ch05_`）と `lib.rs` の試験のモジュールの並びに 1 行。`testdb.rs` は変えない
- CI: `.github/workflows/ci.yml` の rust job の後ろに、殺した後の検査と片付けの self-test の 3 行（CH04・CH06 が同じ job に行を足している。既存の行は変えず、後ろに足す）
- 文書: `docs/testing.md` に 1 節、`docs/flow-gates.md` の検査の表に 2 行、`docs/changes/CH05.md`（価値・壊してはいけないもの・完了の判定）
- 依存: 足さない。移行は足さない
- **CH06 に依る**（Q8）: `requires: [CH04, CH06]`（`docs/changes/CH05.md` の front matter。record が書いた）。片付けの 1 行は CH06 の `tools/verify-env.sh` の流れ（印での作り直し・subnet の割り当て）の中に置く。
  CH06 の下流 `feat/ch06-reset-verify-db` は `c666673`（PR #164）で gate を通った。下流の入口はその SHA を取り込む（`story_deps.py`。archive は待たない）。
  **CH06 の gate 済みの spec と突き合わせた結果**: CH06 の Requirement「worktree ごとの DB の network は、木ごとに被らない subnet で立つ」も、同じ subnet を持つコンテナの無い network（消えた worktree の残り）を消す。
  片付けはその確かめより前に走り、`<main>-` の project に絞るので、CH06 の Scenario（`ch06-sentinel-` の network とコンテナ）はどれもそのまま成り立つ。書き直すのは Requirement 1 の 1 文だけ（design D5・D6）。
  台帳の番号を継いだ木で、消えた worktree の DB が同じ subnet の network に繋がったまま CH06 が止まる場合は、片付けが先に外すので止まらなくなる。Q7 の範囲は狭めない
- **CH04 に依る**（Q1 の推奨）: 回の database の名前（`ashiato_run_<12 桁>`）・回の助言ロック（`4820253`）・直列化のロック（`4820252`）・次の回の初めの片付け（CH04 D1・D2・D6）の上に殺した後の検査を積む。
  下流の入口は CH04 の gate を通った SHA（`9fe8c5e`。PR #156）を取り込む
- **下流の前提 —— この作業ツリーの DB が立つこと**: 2026-10-24 の時点で docker の既定のネットワークの割り当てが尽きていて、新しい compose の project は DB を立てられなかった（`premises.md:15`）。
  CH06 が作業ツリーごとに subnet を明示するので、割り当ての枯渇には頼らなくなる（CH06 design D5・D6）。消えた作業ツリーのコンテナ（2026-10-25 に 15 個。動いたままのものが 4 つ。`premises.md:16`）は CH06 の後も残り、作業ツリーが消えるたびに溜まる
- **片付けが外すもの（Q7）**: この機械では、作業ツリーの path が無い `ashiato2-*` のコンテナ 15 個（`premises.md:16`）。片付けたコンテナの volume は残る（`premises.md:17`）。外すものの一覧は下流が `--dry-run` で design の「計測の記録」に写してから片付ける
- **走っている Story・change との重なり**（`origin/main` = 934d18dc の時点）:
  - `tools/verify-env.sh`・`tools/ports.sh`・`tools/env-init.sh`: CH06（上流は main にある。下流 `feat/ch06-reset-verify-db` は `c666673` で gate を通った）が書き換える。Q8 で解いた ——
    CH05 は CH06 の上に積み、作り直しの規則・`tools/ports.sh`・`tools/env-init.sh` に触らず、`tools/verify-env.sh` に片付けの 1 行を足すだけ。
    片付け（Q7）は CH06 の self-test の compose の project（`ch06-sentinel-<乱数>`）を、project の名前を `<main>-` に絞って外さない（Q8 の前の版の design D9）
  - `.github/workflows/ci.yml` の rust job: CH04（`tools/check-test-db-growth.sh` の 3 行）・CH06（`tools/check-verify-env-reset.sh` の 2 行）・ST29（`check-satellite-boundary.sh` の 2 行）。どれも rust job の行の追加。CH05 は CH04 と CH06 の行の後ろに足す。後から入る側が rebase で追従する
  - `docs/testing.md`: CH04（1 節）・CH06（CI の表の行）・ST29（2 行）。CH05 は 1 節を足す。`docs/flow-gates.md` の検査の表: CH06 が 1 行。CH05 は 2 行足す
  - `crates/server/src/lib.rs` の試験のモジュールの並び: CH04（`testdb_tests`）・ST06・ST22・ST23・ST29・ST41 が行を足す。CH05 も 1 行足すだけ
  - 走っている Story へ差し戻すものは無い（CH06 の Requirement の書き直しは、CH05 の delta の MODIFIED で CH06 の archive の後に当てる）
