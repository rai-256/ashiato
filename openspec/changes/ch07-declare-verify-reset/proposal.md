# CH07 — 検証の準備が作り直した DB を宣言する

> spec の段（`--until proposal`、2026-10-25）。CH06（`requires`）の下流がまだ gate を通っていないので、proposal までを書いた。
> specs / design / tasks と `docs/changes/CH07.md` の「完了の判定」は、CH06 が gate を通った後の spec の段が書く。
> 深掘りは問い 0 件（`deep.md`）。決定は `premises.md` の C1〜C9 と `design.md` の D1（仮）。

## Why

harness2 #112（2026-10-25）で、検証の準備は作り直したものを `HARNESS_RESET: <何を>` の行で宣言する約束になった。`verify-run` はこの行を証跡の `reset` に残し、
宣言の無い準備には「注意」を出す。`init.py --check` も、宣言を持たない `tools/verify-env.sh` を人間の項目として出す（premises.md P2〜P4）。
CH06 は作り直しそのものを入れるが、この形が決まる前に合意したので宣言を出さない。CH07 が宣言の 1 行を足すと、作り直した証跡には何を作り直したかが残り、
溜まった DB の上で取った証跡にだけ注意が付く。

## What Changes

- **`HARNESS_VERIFY=1` で `tools/verify-env.sh` がこの worktree の DB を実際に作り直したら、標準出力に `HARNESS_RESET: testdb` を行頭から 1 行出す**（C1・C2）。
  出すのは `verify-env.sh` 自身の `echo` で、役割（`tools/db-roles.sh`）まで整えて rc=0 で終わる直前に 1 回だけ。字 `HARNESS_RESET:` は `verify-env.sh` の本文に置く（`init.py` はこのファイルの字を読む。P4）
- **出す条件は「CH06 の判定が `reset` で、かつ控えた volume を 1 つ以上消したか、DB のコンテナが無くて新しく立てた」**（C2・C9）。
  DB のコンテナが止まっていた場合も、この条件を満たせば出す。コンテナがあって控えた volume が 0 個（中身が残ったまま）なら出さない
- **作り直さない場合は出さない**（C3）: 印が無い・CI でない main の checkout・git の木でない・作り直せずに非 0（途中で落ちた場合を含む）。`HARNESS_RESET: none` も出さない
- **`tools/verify-env.sh --plan` は宣言を出さない**（C4。CH06 の `--plan` の標準出力は全体を比べる検証がある）
- **検査は CH06 の `tools/check-verify-env-reset.sh` に足す**（C5）: 本体（docker を使う方）は、作り直した段で準備の出力に宣言がちょうど 1 行、作り直さない段で 0 行。
  `--self-test`（docker に触らない）は、`--plan` が宣言を出さないことと、`verify-env.sh` の本文に字 `HARNESS_RESET: testdb` があること
- **完了の判定 3（`init.py --check`）は #112 の merge を待たずに、#112 自身の `check_verify_env` を走らせて確かめる**（C7）。
  判定は出力に `[人間] tools/verify-env.sh` で始まる行が無く、`[ok] tools/verify-env.sh` で始まる行がちょうど 1 つ（rc は見ない）。使った harness2 の SHA を証跡に残す
- **振る舞いを変えないもの**（C4。「壊してはいけないもの」）: CH06 の判定の表・`--plan` の出力・作り直しの手順と対象・切った接続の 1 行の形・作り直せないときの rc・`keep:` の標準エラーの 1 行。
  `tools/stack.sh` など検証コマンドの側の台本・試験のコード（`crates/`）・移行・CI の job の並びには触らない
- やらないこと: harness2 側（`verify-run`・`init.py`）の変更、#112 と CH07 の入る順の調整（design D1（仮））、CH06 の tasks への差し戻し（C6）

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `verify-environment`: CH06 が立てる capability（CH06 はまだ archive されておらず、正典の `openspec/specs/` には無い）。
  「作り直したら、作り直したものを宣言する・作り直さないときは宣言しない」の Requirement を 1 つ足す。CH06 の 6 つの Requirement の本文と Scenario は変えないので、specs の段は
  `MODIFIED` でなく `ADDED`（`## Purpose` は持たない。Purpose は CH06 の delta が持つ）で書く。archive は CH06 の archive の後にグラフが回す
- schema の「tooling は `skip_specs`」は採らない（CH03・CH04・CH06 と同じ理由）: 宣言が出る・出ない場合は `verify-run` の証跡と注意に外から見え、Scenario を検査の印に縛って正典に残す

## Impact

- 道具: `tools/verify-env.sh`（宣言の `echo` を 1 行と、出す条件の印）、`tools/check-verify-env-reset.sh`（本体と `--self-test` に段を足す。どちらも CH06 が作るファイル）
- CI: 変えない（CH06 が rust job に足す `check-verify-env-reset.sh` の 2 行がそのまま CH07 の段も走らせる）
- 文書: `docs/changes/CH07.md` の「完了の判定」（spec の段）。`docs/testing.md`・`docs/flow-gates.md` の CH06 の行に足すことは無い見込み（specs の段で確かめる）
- 試験のコード・移行・製品のコード・依存: 変えない
- harness2: 変えない。#112 が入る前も後も、宣言の行は無害（入る前はログに 1 行増えるだけ。C7）
- **走っている Story・change との重なり**:
  - CH06（下流が走っている。7 Task のうち Task 1 が完了）: CH07 が触る 2 ファイルはどちらも CH06 が書き換える・作る。CH07 は `requires: [CH06]` で CH06 の実装の上に積み（C6）、
    CH06 の tasks へ差し戻さない。CH06 の Task 5 が検査の本体の段（design D8 の番号）を変えたら、C5 の段の対応を specs の段で読み直す（C5 の反転条件）。
    #112 が先に `.harness2` に入って CH06 の下流に注意が出ても、順を変えない（D1（仮））
  - CH04・st48: tasks が `tools/verify-env.sh` を名指しするのは「`verify-run` が DB を整える」の前提の注記だけで、ファイルには触らない。宣言の行は検証の結果を変えない（P3。status は変えない）
  - CH03・st23・st29・st41: `tools/verify-env.sh` にも `tools/check-verify-env-reset.sh` にも触らない（`grep -rl 'verify-env\|check-verify-env-reset\|HARNESS_RESET' openspec/changes/<change>`）。走っている change へ差し戻さない
- `docs/changes/CH07.md` の `capability: []` はそのままにする（CH04・CH06 と同じ扱い。change の capability は proposal が持つ）
