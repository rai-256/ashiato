# CH06 — 検証のたびにテスト用 DB を作り直す

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違えば `spec --answers` が写し直す。
> `docs/changes/CH06.md` の「価値」「壊してはいけないもの」「完了の判定」も、この段で書いた。
> spec の段で、subnet を全部の呼び出し口に届ける手段（P13）・既にある worktree の DB の中身が subnet を入れても残ること（P12）・
> 追跡ファイルに点区切りの私設 IP を書けないこと（P14）を確かめ、`premises.md` に足した。spec-review（`review/spec.md`）を受けて、subnet の届け方を生成したファイルに替え（R5）、
> 番号を継いだ木（R3）・確認バッチの作業ツリー（R4）・手で置いた override（R10）を足した。

## Why

ハーネスの `verify-run` は、検証のたびに準備（`tools/verify-env.sh`）と検証コマンドへ `HARNESS_VERIFY=1` を渡し、「repo は溜まった状態を作り直す」と約束している（`scripts/evidence.py:28-31`）。
ashiato2 の `verify-env.sh` はこの印を見ず、worktree のテスト用 DB は作り直されないまま溜まる（ch03 の DB は `core.event` 22 万行）。書き出しの試験は DB 全体を読むので、
同じ試験が作った直後の 43 秒から 2745 秒・7048 秒へ伸び、10/17〜10/24 の server の試験の証跡は合計 53.5 時間になった（`docs/changes/CH06.md`）。

もう 1 つ、いまのこの機械では、新しい worktree の DB が起動できない。Docker の既定のアドレスの空きが尽きていて（P4）、消えた worktree の network が 15 個残っている（P10）。
CH06 の下流の検証そのものが BLOCKED_INFRA になる。

## What Changes

- **印（`HARNESS_VERIFY=1`）があれば、`tools/verify-env.sh` がこの worktree の DB を空から作り直してから役割を整える**（Q1 の推奨・C5）。
  DB のコンテナの volume の名前を控え、コンテナを消し、その volume を名指しで消して立て直す。network は手放さない（C2）。移行は当てない（`testdb.rs` の初回が当てる。C3）。
  作り直すのは compose project がこの worktree の DB だけで、ほかの worktree の DB には触らない
- **作り直しのとき繋いでいたものは構わず切り、切った接続の数と出所（`application_name` と接続元）を 1 行出す**（Q2 の推奨）
- **作り直せなければ非 0 で終わる**（C6）。ハーネスは BLOCKED_INFRA として記録し、溜まった DB の上で証跡を取らない
- **作り直すかの判定**（C1）: 印があり、かつ（worktree である か `CI=true`）のときだけ作り直す。CI でない main の checkout（開発用の起動の DB）は、印があっても作り直さず、そう 1 行出す。
  判定は `tools/verify-env.sh --plan` で docker に触らずに出せる
- **worktree ごとの DB の network に、被らない subnet を割り当てる**（Q3 の推奨）。subnet はその木の DB の port（`tools/ports.sh` の規則で生きている木どうしで被らない。台帳の木は台帳の番号から port が決まる）から決める。
  `tools/env-init.sh` が subnet を書いた `docker-compose.subnet.yml`（生成物。追跡しない）を毎回作り直し、`.env` に `COMPOSE_FILE` を書く。どの呼び出し口の `docker compose` にも効く（P13）。
  既にある `.env` には、合言葉を変えずに `COMPOSE_FILE` の行だけを足す・直す（新しい規則）。subnet の値は環境変数にも `.env` にも載せない（ハーネスが渡す古い `.env` の値が勝つため。review/spec.md R5）。
  既定のアドレスの空きに依らずに network を作れるので、`stack.sh`・`smoke.sh` などが `down -v` で network を手放しても、次の `up` で同じ subnet を取り直せる
- **番号を継いだ木の DB も立つ**: 消えた worktree の network が同じ subnet を塞いでいれば、コンテナが繋がっていないときだけその network を消して立てる。繋がっていれば何も消さずに非 0（design D5。「残りは片付けない」の唯一の例外）
- **確認バッチの作業ツリーの準備（`tools/verify-prep.sh`）も `env-init.sh` を呼ぶ**（main の `.env` を写した木の subnet をその木の値にする）
- **CI では `.env` を作らない**: `.env` が無く、合言葉が環境変数で渡っている（CI の `$GITHUB_ENV`）とき、`env-init.sh` は何もしない。CI の DB は今までどおり base の compose で立つ（subnet を明示しない）
- **完了の判定 3 の検査 `tools/check-verify-env-reset.sh`（`--self-test` 付き）を足し、CI の rust job で走らせる**（C4）。subnet が木どうしで被らないことは、いまある `tools/check-ports.sh --self-test` に足す
- **振る舞いを変えないもの**: 印が無いときの `verify-env.sh`（起動して待ち、役割を整えるだけ）、CI でない main の checkout の DB の中身、役割と合言葉（`db-roles.sh`・既にある `.env` の合言葉）、
  worktree ごとの port の規則、`stack.sh`・`smoke.sh`・`check-immutable.sh`・`check-panic-log.sh` の `down -v`、`tools/dev.sh`、試験のコード（`crates/`）
- やらないこと: 消えた worktree の compose project（network 15 個・動いたままの DB 4 個）と宙に浮いた volume（66 個）の片付け（Q3 の推奨は「触らない」）。design の Non-Goals

## Capabilities

### New Capabilities

- `verify-environment`: 検証の準備（`tools/verify-env.sh`）が、ハーネスの印を受けて worktree の DB を空から作り直すこと・作り直さない木・切った接続の知らせ・作り直せないときの止まり方・
  worktree ごとの DB の network が被らない subnet で立つこと。製品の振る舞いではなく、並走する Story と change が同じ機械の上で証跡を取り続けるための、開発の道具の約束。
  CH04 が立てる `test-database`（試験の中の database の持ち方）とは別にする: CH04 はまだ archive されておらず、同じ新しい capability に 2 つの change が `## Purpose` を持つ ADDED を積むと、
  archive の順で片方の Purpose が宙に浮く（CH04 の design D13 と同じ理由。design D10（仮））
- schema の「tooling は `skip_specs`」は採らない（CH03・CH04 と同じ理由）: 行が 0 になる・ほかの worktree に触らない・作り直せなければ非 0、は外から見える振る舞いで、
  Scenario を `check_scenarios.py` が印に縛り、正典に残る

### Modified Capabilities

（無し）

## Impact

- 道具: `tools/verify-env.sh`（判定・作り直し・切った接続の 1 行・塞いでいる network・`--plan`）、`tools/env-init.sh`（`COMPOSE_FILE` と subnet のファイルの保守・CI で作らない・`COMPOSE_PROJECT_NAME` を持つ `.env` に触らない・override を並べる）、
  `tools/ports.sh`（subnet の関数）、`tools/verify-prep.sh`（env-init を呼ぶ 1 行）、新しいファイル `tools/check-verify-env-reset.sh`、`tools/check-ports.sh` の自己検査に subnet を足す、`.gitignore` に生成物 `docker-compose.subnet.yml`
- CI: `.github/workflows/ci.yml` の rust job に 2 行（`check-verify-env-reset.sh --self-test` と本体）。`check-ports.sh --self-test` の行の後に置き、`cargo test` の周りには触らない
- 文書: `docs/testing.md` の CI の表の rust 行、`docs/flow-gates.md` の検査の表に 1 行
- 試験のコード・移行・製品のコード: 変えない
- 依存: 足さない
- **既にある worktree への効き方**: rebase した worktree で最初に `verify-env.sh` が走ると、`.env` に `COMPOSE_FILE` が入って subnet のファイルができ、次の `up` で network が明示の subnet で作り直される。
  DB のコンテナは作り直されず、中身は残る（P12）。main の checkout の `.env` にも同じ行が入る（中身は残る。作り直しはしない）。手で置いた `docker-compose.override.yml`（`ashiato2-integ` など）は `COMPOSE_FILE` に並べる
- **走っている Story・change との重なり**:
  - CH04（下流の Task 10）: `testdb.rs` は触らない。CH04 の回ごとの database は volume ごと消えるので、同じ worktree で並んで走っている回は切られる（Q2 の推奨。design の Risks）。
    CH04 の片付けの助言ロックは作り直しを止めない。どちらが先に merge されても互いの前提は崩れない（CH04 は空の DB からテンプレートを作る）
  - CH03・CH04 の `ci.yml` の rust job: どちらも `cargo test` の行とその直後に手を入れる。CH06 は job の末尾（`check-ports.sh --self-test` の後）に足すので、同じ行を取り合わない。
    後から merge する側が rebase で追従する。merge の順は決めない
  - st23・st29・st41: `tools/` の上の 4 ファイルにも `docker-compose.yml` にも触らない（tasks.md の grep）。走っている Story へ差し戻さない
- `docs/changes/CH06.md` の `requires: []`・`capability: []` はそのままにする（CH04 の D12 と同じ扱い。design D10（仮））
