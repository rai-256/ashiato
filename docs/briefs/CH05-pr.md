# CH05 試験の DB が打ち切った run の残りで膨れないようにする

change: `openspec/changes/ch05-test-db-bloat/`。依頼の逐語・価値・壊してはいけないもの・完了の判定は `docs/changes/CH05.md`。
CH04 と CH06 の上に積む（`requires: [CH04, CH06]`）。finish で origin/main（CH07・ST41 の archive の後）を取り込んだ（`4bc276fb`。review/code.md R22）。

- `tools/check-test-db-killed.sh`（新規）: 探りの試験 `ch05_killed_probe` を 2 つの時点（`insert` 行を書く前の走っている INSERT / `committed` 確定した後）で SIGKILL し、次の回の後に殺した回の database が残らず、`ashiato` の行数が同じで大きさが 1 MiB を超えて増えないことを見る。秒数は出すだけ
- `tools/docker-tidy.sh`（新規）: 作業ツリーが消えた `<main>-` の compose project のコンテナとネットワークを外す。volume は消さない
- `tools/verify-env.sh` に 1 行: 作り直す（`reset`）ときだけ、`reset_db`・`clear_blocking_network` の前に片付けを呼ぶ。落ちても `片付けに失敗した` の 1 行を出して続ける
- CI の rust job に 4 行（`check-test-db-killed.sh --self-test`・`docker-tidy.sh --self-test`・`--verify-env-check`・`check-test-db-killed.sh`）。`docs/testing.md`・`docs/flow-gates.md`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

全 Task の ledger（`.superpowers/sdd/ch05-task-1`〜`8`）に `progress.md` は無く、`Ruling:` 行は 0 件。仮決めは design の（仮）と台帳にある:

- **D1（仮）** 置き場: 殺した後の検査は `test-database`、片付けは `verify-environment`（CH06 の Requirement 1 を MODIFIED）。tasks 8.1 が正典との diff で見る（取り込んだ木で rc=0）
- **D2（仮）** 探りの試験を印で同期して SIGKILL する。`insert` の時点は行を 1 つも書く前の走っている文で、書きかけの行がある状態は見ない（finish で spec の本文と D2 に書き足した。R19）
- **D4（仮）** 片付けの条件（`<main>-` の project・path が無く `git worktree list` にも無い・main と `-verify` を除く。`rm -f` に `-v` を付けない）
- **D5（仮）** 片付けは `reset` のときだけ、`reset_db` の前。CH07 の後は `reset_done=` の後・`reset_db` の前（R22）。反転条件: CH07 の宣言が片付けも対象にすると決めたら `reset_done=` の前へ
- **D8（Q9 で確定）** 外したコンテナの volume と作業ツリーの結び付きは残さない（R20）。仮で足した inspect と行の末尾の `残す volume:` を、別の席（`provisional-review.md`）の否の後、Q9 (a) の答えどおり外した
- finish が退けたもの（Ruling 相当）: R15 開始時の探りの掃除を外した（並走するほかの木の self-test を消さない代わりに、SIGKILL された self-test の探りは印 `dev.ashiato.ch05-tidy-probe=1` で手で探す）

## Task

8 Task・15 項目すべて `[x]`。残した Task は無い。

## 検証（finish の後の木 `7e95030f` で `scripts/verify-run`、evidence.jsonl）

| 項目 | 結果 |
|---|---|
| 2.1 片付けの台本と self-test | PASS 4 本（rc=0） |
| 3.1 verify-env への組み込み（`EV quiet-run ch06 …` 70 秒を含む） | PASS 4 本 |
| 3.2 `docker-tidy.sh --verify-env-check`（23 秒） | PASS 4 本 |
| 5.1 `check-test-db-killed.sh --self-test` と印 | PASS 3 本 |
| 6.1 本物の試験で殺す（71 秒） | PASS 3 本 |
| 8.1 Scenario の印・正典との diff | PASS 2 本。`openspec validate --strict` は手で rc=0（INFO なし。証跡には載らない —— 下の申し送り） |
| 8.2 変えないものが変わっていない | PASS 2 本 |
| 8.3 verify-env と `cargo test -p ashiato-server`（187 秒） | PASS 2 本 |

ほかに: `cargo fmt --all --check` rc=0、`python3 scripts/review_triage.py . ch05-test-db-bloat` triage: OK、`check_scenarios.py` scenarios: OK。

## レビュー

- final review（R1〜R14）: 全部 fixed。scoped re-review で ADDRESSED。R15・R16 は rejected
- code-verify（R17〜R22）・security-review（指摘なし）・`/code-review`（R23〜R30）を finish で処置:
  - fixed: R17（self-test がネットワークの守りを観測していなかった）・R18（片付け → reset_db → clear_blocking_network の順を self-test が固定）・R19・R20（Q9 に返し、答え (a) で D8 を確定。仮の実装を外した）・R21・R22（取り込み）・R24（docker の警告を一覧に読まない `dq`）・R25（psql の失敗で 120 秒待たない）・R29（`timed_next`）
  - rejected（反証は review/code.md）: R23・R26・R27・R28・R30 —— どれも spec か tasks 8.2 が決めたとおり
  - 直したものは、守りを壊した複製で self-test が rc=1 になることを確かめた。scoped re-review: 全部 ADDRESSED／反証が成り立つ、新しい不具合なし

## 未処置の申し送り

- **harness2**: `scripts/evidence.py` の `CMD_LIKE` が `openspec` で始まるコマンドを拾わず、tasks 8.1 の `openspec validate … --strict` が証跡に載らない（R21）。harness2 側で `openspec\s` を足す

🤖 Generated with [Claude Code](https://claude.com/claude-code)
