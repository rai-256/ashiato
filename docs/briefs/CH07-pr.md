# CH07 検証の準備が作り直した DB を宣言する

change: `openspec/changes/ch07-declare-verify-reset/`。CH06 が立てた capability `verify-environment` に Requirement を 2 つ ADDED（Scenario 12 本）。
依頼の逐語・価値・壊してはいけないもの・完了の判定は `docs/changes/CH07.md`。`requires: [CH06]`（`deps.json`）。

`HARNESS_VERIFY=1` の準備（`tools/verify-env.sh`）が DB を作り直したときだけ、役割まで整えた後に標準出力へ `HARNESS_RESET: testdb` を 1 行出す。
出すのは「DB のコンテナが無くて立てた」か「控えた volume を 1 つ以上消した」ときだけ。印なし・CI でない main の checkout・git の木でない・非 0・volume が 0 個のときは出さない。
ハーネスの `evidence.py` がこの行を拾い、`evidence.jsonl` の `reset` に残す。

## 深掘りに積んだ未決（A）

なし。問いは 0 件。聞かずに決めたものは `premises.md` の C1〜C9。

## 仮決め（B）と Ruling

design の（仮）（反転条件は design の各見出しの下）:

- **D1（仮）** harness2 #112 が先に入っても、CH07 の順を変えず、CH06 へも差し戻さない
- **D3（仮）** 検査の本体では、既にある段 1・3〜8 に宣言の数の確かめを足し、新しい段を 3 つ足す（9. 止まっていた DB・10. volume を持たない DB（tmpfs の compose を `COMPOSE_FILE` で並べる）・11. 途中で落ちる（`ENV_FILE` に無いファイル））

Ruling: 全 Task の ledger に `Ruling:` の行は無い（`.superpowers/sdd/ch07-task-*/` に `progress.md` が無い。final の fix report にも無い）。
finish で park したもの（fix は 1 回だけの決まり）: scoped re-review の残り 2 件。下の申し送りに書いた。

## Task

4 Task・8 項目を全部埋めた（`[x]` 8/8）。残した Task は無い。

## 通した検証（`evidence.jsonl`）

| 項目 | コマンド | rc |
|---|---|---|
| 1.1 | 印つきの準備で `HARNESS_RESET: testdb` が 1 行 / 印なしで 0 行 / `--plan` = `reset` / 注記でない echo が 1 つ | 0 |
| 2.1 / 2.2 | `tools/check-verify-env-reset.sh --self-test`（CI と同じ環境変数でも）と印の grep | 0 |
| 3.1 / 3.2 | `echo "Scenario: ` の本数（35）と印の grep | 0 |
| 3.1 / 3.2 / 4.3 | `tools/check-verify-env-reset.sh`（本体。79 秒） | 0 |
| 4.1 | flow-gates.md と冒頭の注記の grep | 0 |
| 4.2 | `init.py --check`（`.harness2` HEAD `aaecd2c6`）の `[人間]` 0 行・`[ok]` 1 行 | 0 |
| 4.3 | 変えないものの `git diff origin/main...HEAD` が空 / `openspec validate --strict` / `check_scenarios.py` / `check-private.sh` / `check-ports.sh --self-test` | 0 |

finish の直しの後に 1.1・2.1・2.2・4.3 を `verify-run` で走らせ直し、すべて PASS（`reset: ["testdb"]`）。`review_triage.py` rc=0（review/code.md の 14 件と上流のレビューの分、すべてに処置）。

## レビュー

- final（R1〜R3）: fixed 2（`--self-test` のラベルの展開・本体の注記の段の範囲）、rejected 1（R3。10. の途中の abort で残るコンテナは次の準備が作り直す）
- finish で code_verify（R4・R5）・security-review（指摘なし）・`/code-review`（R6〜R14）を処置した:
  - fixed 4: R4・R6・R7（`tools/verify-env.sh` の注記が宣言の字を持っていて、`init.py --check` が echo の無い準備も `[ok]` にしていた。注記から字を外し、`--self-test` で注記の字 0 を守る）・
    R12（宣言の行の守りを「echo の後ろが空・`$(` とバッククォートの中でない」に）。echo を消す・注記に字を足す・`>>"$log"` を付ける の変異で `--self-test` が rc=1 になることを確かめた
  - followup CH06 2: R5・R10（コンテナだけが外で消された木で、前の DB の匿名 volume が残る。`docs/handoff/CH06.md`）
  - rejected 5: R8（判定 3 は C7 で `init.py --check`。1.8 秒）・R9（R3 と同じ。次の準備で収束）・R11（volume 0 個で宣言しないのは C2 の決定と Scenario）・
    R13（tasks 3.1 が名指しした標準エラー 0 行の確かめ）・R14（最初の NG が 11. の段名を出す）

## 申し送り

- `docs/handoff/CH06.md`: コンテナが無く volume が残った木の宙に浮いた volume の片付け（R5・R10）。CH06 の後続の change で決める
- harness2: `init.py` の `check_verify_env` は `HARNESS_RESET:` の部分一致で、注記も数える。注記を除いて読む形にすると、ashiato2 の側の注記の守りが要らなくなる（R4）
- park（scoped re-review の残り。確かさは低い）: echo の**前**に置いたリダイレクト（`|| >/dev/null echo …`）と行末の注記（`exit 0  # HARNESS_RESET: x`）は `--self-test` の守りを抜ける
  （`tools/check-verify-env-reset.sh:486`・`:489`）。前者は本体の段が落とす。後者は上の harness2 の直しと同じ根
- 本体の CI での結果は、この PR の CI の rust job で初めて出る。「CI でない main の checkout」の段は CI だけで走る（code_verify は clone した木で模擬して ok）
