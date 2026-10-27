# CH06 検証のたびにテスト用 DB を作り直す

change: `openspec/changes/ch06-reset-verify-db/`。新しい capability `verify-environment`（ADDED 6・Scenario 24 本）。
依頼の逐語・価値・壊してはいけないもの・完了の判定は `docs/changes/CH06.md`。`requires: []`（CH04 と重なるが依らない。design D9）。

`HARNESS_VERIFY=1` の印がある worktree（と `CI=true`）では、`tools/verify-env.sh` が毎回 DB のコンテナと volume を作り直し、切った接続の数と出所を 1 行出す。
Docker の network の空きが尽きる問題（BLOCKED_INFRA）には、worktree ごとの subnet を DB の port から決めて `docker-compose.subnet.yml` と `.env` の `COMPOSE_FILE` で届ける。
検査は `tools/check-verify-env-reset.sh`（`--self-test` は docker に触らない・本体は溜まった DB から作り直して確かめる）を CI の rust job に置いた。

## 深掘りに積んだ未決（A）

なし。第 1 回の答え（2026-10-25、`deep-answers-1.txt`）は Q1〜Q3 とも推奨のまま、仕様への合意は「合意する」。

## 仮決め（B）と Ruling

design の（仮）（反転条件は design の各見出しの下）:

- **D1（仮）** 作り直しは volume の名前を控え、`rm -s -f db`（`-v` なし）→ 名指しの `volume rm` → `up`
- **D2（仮）** 判定は印・worktree か・`CI` の表。`--plan` で docker に触らずに出す
- **D5（仮）** subnet は DB の port から。塞いでいる network はコンテナが無ければ消し、あれば非 0（Q3 の「片付けない」の唯一の例外）。
  finish で足したもの: 繋がっているのが消えた worktree のコンテナなら、そう出して外し方を添える（R20。止まるのは変えない）/ 重なるだけの範囲の network でも名前を出して非 0（R23）/ network の範囲は 1 回の inspect で（R29）/ 名前は `compose config --format json` から（R25）
- **D6（仮）** subnet は生成した `docker-compose.subnet.yml` と `.env` の `COMPOSE_FILE` で届ける。finish で直した: `tools/agent-env.sh` は `COMPOSE_FILE` を環境に写さない（R15。写すと録画の木の compose が落ち、override のある木で override が外れる）
- **D7（仮）** env-init は `COMPOSE_FILE` と subnet のファイルだけを保つ・CI で作らない・録画の `.env` に触らない・override を並べる
- **D8（仮）** 検査の本体は 8 段。finish で足した: 接続 12 本で D3 の書式（`×<数>`・10 種で切る・`…`）を固定（R16）/ sentinel の後片付けを `rm -f -v`（R14）/ 重なる範囲・消えた worktree の段（R20・R23）
- **D10（仮）** capability は新しい `verify-environment`
- **D12（仮）** 追跡する文書の subnet は点区切りで書かない（`tools/check-private.sh` の例外を作らない。R19）

Ruling（全 Task の ledger。Task の ledger に `Ruling:` は無く、final の controller の 2 件だけ。`.superpowers/sdd/ch06-final-fix-report.md`）:

- Ruling: R2 —— 7.1 の 5 回中 1 回の PASS で完了の判定 2 を満たすとみなした（落ちた試験は時間・負荷と `fresh_db` の `FORCE` の権限で、`crates/` は不変・単独で通る）。違っていたら: 揺れる試験を直す change を CH06 の前に通す必要があり、CH06 の merge が遅れる。揺れる試験の起票を下の申し送りに挙げる
- Ruling: R7 —— re-review の Important（`body || exit $?` が errexit と ERR trap を殺す）を、第 2 の fix wave でなく controller が直した（`trap - ERR` の後に `return 1`、呼び出しは `body; exit $?`）。この直しはレビューの席を通っていない（bash の再現 2 本と本体 rc=0 で確かめた。code-verify が続けて見た）。違っていたら: 本体の失敗が trap の行を出さないか、通らない回が OK になる

## Task

7 Task・16 項目を全部埋めた（`[x]` 16/16）。残した Task は無い。

## 通した検証（`evidence.jsonl`、finish の後の木）

| 項目 | コマンド | rc |
|---|---|---|
| 1.2 | `tools/check-ports.sh --self-test`（subnet の繰り上がり・512 の上限を足した） | 0 |
| 2.2 / 3.2 / 7.3 | `tools/check-verify-env-reset.sh --self-test` | 0 |
| 2.2 / 7.3 | `tools/check-private.sh` | 0 |
| 4.1 | `HARNESS_VERIFY=1 tools/verify-env.sh` が `切った接続: ` を出す | 0 |
| 4.2 | 2 回続けての作り直し / 禁じた旗の grep | 0 / 0 |
| 5.1 | `tools/check-verify-env-reset.sh`（本体。74 秒）/ sentinel の残りが 0 | 0 / 0 |
| 7.1 | 完了の判定 2（木 4c1f3ac6、754 秒。5 回中 1 回。Ruling R2） | 0 |
| 7.3 | `python3 scripts/check_scenarios.py . ch06-reset-verify-db` | 0 |

`review_triage.py` rc=0（指摘 52 件すべてに処置）。本体の前後で dangling の volume の数は 103 → 103（R14 の直しの確かめ）。
本体の CI での結果と時間（D8 の反転条件 90 秒）は、この PR の CI の rust job で初めて出る（R1）。

## レビュー（finish の分）

final（R1〜R13）は final の段で処置済み。finish で code_verify・security-review・`/code-review` の 16 件を処置した:

- fixed 10: R14（sentinel の匿名 volume）・R15（`COMPOSE_FILE` を agent の環境に写さない）・R16（D3 の書式を試験で固定）・R17（env-init と subnet の端を試験で固定）・
  R18（`--self-test` に volume を消す旗の検査。4 つの複製で落ちる）・R19（文書の点区切りの subnet。D12 仮）・R20・R23・R25・R29（D5）
- rejected 6: R21（main の network の作り直しは P12 で実測済み・中身は残る）・R22（design D7 が受け入れた代償）・R24（環境変数から subnet を取る形への戻りを止める網。複製で落ちる）・
  R26（`--plan` は ports.sh の台帳を書けない）・R27（再現しない。ログに NG 0 件）・R28（design D7 の inode を保つ形・同じ木で並んで読む経路が無い）

## 未処置の申し送り

- 7.1 の `cargo test` が揺れる（5 回中 4 回 FAIL。時間・負荷と `fresh_db` の `FORCE` の権限）。CH06 の外の不具合として、別の change で起票する（Ruling R2）
- `tools/archive.sh` は `git worktree remove` の前に DB を止めない。止まった DB を残して消えた木の番号を継ぐと、次の木の verify-env が外し方を出して止まる（R20。片付けるのは人 —— Q3）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
