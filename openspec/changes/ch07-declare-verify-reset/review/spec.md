# CH07 spec の段の検査（spec-review）

検査の対象: `openspec/changes/ch07-declare-verify-reset/` の proposal.md / specs/verify-environment/spec.md / design.md / tasks.md / premises.md / deep.md / deps.json と `docs/changes/CH07.md`。
突き合わせた先: CH06 の gate 済みの spec・design（`docs/briefs/deps/CH06@c666673/…`。`origin/main` の CH06 の delta と同じ —— `diff` で差 0）、
CH06 の実装（`/home/yosis/dev/ashiato2-ch06` の `c666673`: `tools/verify-env.sh`・`tools/check-verify-env-reset.sh`・`tools/db-roles.sh`・`tools/env-init.sh`・`docker-compose.yml`・`.github/workflows/ci.yml`）、
`.harness2`（`aaecd2c`）の `scripts/evidence.py`（`run_one`・`RESET_MARK`・`declared`・`CMD_LIKE`・`TMP_LEAK`）と `scripts/init.py`（`check_verify_env`・`Report.line`）。

## 機械の検査

```
$ openspec validate ch07-declare-verify-reset --strict
Change 'ch07-declare-verify-reset' is valid                                   rc=0

$ python3 scripts/check_chain.py .
chain: OK (0 件 / 未回収 0 件 / warn 0 件)                                      rc=0

$ python3 scripts/check_scenarios.py . ch07-declare-verify-reset
  [FAIL] 担保の無い Scenario: 溜まった DB を作り直したら、宣言を 1 行出す
  [FAIL] 担保の無い Scenario: 2 回続けて作り直しても、どちらも宣言を 1 行出す
  [FAIL] 担保の無い Scenario: DB のコンテナが無くても、立てたら宣言を 1 行出す
  [FAIL] 担保の無い Scenario: DB のコンテナが止まっていても、作り直したら宣言を 1 行出す
  [FAIL] 担保の無い Scenario: ハーネスの点検が、作り直して宣言する準備と認める
  [FAIL] 担保の無い Scenario: 印が無ければ、宣言を出さない
  [FAIL] 担保の無い Scenario: CI でない main の checkout では、印つきでも宣言を出さない
  [FAIL] 担保の無い Scenario: 作り直せずに非 0 で終わるときは、宣言を出さない
  [FAIL] 担保の無い Scenario: volume を持たない DB のコンテナでは、作り直しても宣言を出さない
  [FAIL] 担保の無い Scenario: 判定だけを出すときは、宣言を出さない
scenarios: FAIL (担保なし 10 件 / 名無しの確認待ち 0 件)                           rc=1
（ほかの [warn] は既存の Story の印。この change に関わらない）

$ python3 scripts/review_triage.py . ch07-declare-verify-reset
  [FAIL] deep.md R3: design.md の D1 に「仮」の印が無い（仮決めは design 側にも印を置く。PR 本文の「仮決め」一覧はここから引く）
triage: FAIL (1 件)                                                            rc=1

$ python3 scripts/spec_overlap.py candidates . ch07-declare-verify-reset
[{"change": "ch06-reset-verify-db", "cap": "verify-environment",
  "name": "作り直すのは、印があり、worktree か CI のときだけ", "kind": "名指し", "ops": "相手 ADDED"}]   rc=0

$ python3 .harness2/scripts/evidence.py lint openspec/changes/ch07-declare-verify-reset
[OK] 検証コマンド（ch07-declare-verify-reset）                                    rc=0
```

- check_scenarios の 10 件は、印の置き場（`tools/check-verify-env-reset.sh`）が下流で書かれる前なので、この段では予定どおり。10 本とも tasks のどこかが名指ししている（Global Constraints と 2.1・2.2・3.1・3.2）。
- triage の FAIL は R1。

---

## R1. review/deep.md R3 の処置は `fixed D1 仮` なのに、spec の段が design D1 の（仮）を外したので、triage が FAIL する
- 成果物: openspec/changes/ch07-declare-verify-reset/design.md
- 根拠: `python3 scripts/review_triage.py . ch07-declare-verify-reset` → `[FAIL] deep.md R3: design.md の D1 に「仮」の印が無い` / review/deep.md:59・61（R3 は `kind: daily`、`処置: fixed D1 仮`）/ design.md:32（見出しに（仮）が無い）・:36（「（仮）を外した」）/ `scripts/review_triage.py:167-180`（kind=conflict / daily は `fixed D<n> 仮` でしか閉じられず、design の D<n> に「仮」の印が要る）/ `.harness2/docs/flow-gates.md:77`
- kind: technical
- 提案: D1 の見出しを `### D1（仮）. …` に戻す。D1 の「起きた側で決まった」は本文に残してよく、反転条件は既にある :37（CH06 が後で宣言の行を足したら 1.1 が結び直す）をそのまま使う。triage を rc=0 にしてから merge gate へ
- 処置: fixed D1 仮（見出しに（仮）を戻し、反転条件を本文に明示）

## R2. Requirement「作り直していなければ…」の本文にある 2 つの場合（git の木でない・作り直しの途中で落ちた）を、どの Scenario も言っていない。後者は D2 の置き場の決め手なのに、印の無い段（worktree だけの 8.）でしか確かめない
- 成果物: openspec/changes/ch07-declare-verify-reset/specs/verify-environment/spec.md
- 根拠: spec.md:38（「git の木でないとき」「作り直しの途中で落ちたときを含む」）に対し、Scenario は :43〜:64 の 5 本で、非 0 は :53（volume を別のコンテナが使っている = `reset_db` が何も消さずに `exit 1`。CH06 `tools/verify-env.sh:63-66` —— 印の変数が立つ前に落ちる）だけ /
  design.md:47・:49（`exit 0` の直前で出す理由は「消した後の `up`・`db-roles.sh` が落ちても宣言が残らない」こと）/ design.md:68（8. の rc≠0 の 2 回 = DB のコンテナが無い状態で `reset_db` が印を立てた後に `clear_blocking_network` が `exit 1` —— これが「途中で落ちた」の唯一の確かめ。「印は付けない」）/
  CH06 `tools/check-verify-env-reset.sh:207`・`:252`（8. は subnet のファイルがある木だけ。CI では SKIP）/ git の木でない場合: CH06 `tools/verify-env.sh:18-23`（`dirs` が空なら CI でなければ main の checkout と同じ `keep`）
- kind: technical
- 提案: Scenario を 2 本足す。(1)「作り直しの途中で落ちたら、宣言を出さない」—— 本体の段で `ENV_FILE=<無いパス> HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶと volume を消した後に `tools/db-roles.sh:24` で `exit 1` になる（`ENV_FILE` を読むのは `db-roles.sh` だけ。`grep -n ENV_FILE tools/ports.sh tools/env-init.sh tools/verify-env.sh` は 0 件）ので、CI でも worktree でも rc≠0・宣言 0 行・前の volume が無いことを確かめられる（次の段の前に印つきで立て直す）。
  (2)「git の木でなければ、印つきでも宣言を出さない」—— `--self-test` で git でない一時の場所に `tools/verify-env.sh` を写し `--plan` が `keep:` を返すことまで（本体は docker が要るので判定まで）でよい。足したら tasks の印の数（Global Constraints の 10 本・3.1 / 3.2 の下限）を直す
- 処置: fixed specs/verify-environment/spec.md —— Scenario「作り直しの途中で落ちたら、宣言を出さない」「git の木でなければ、印つきでも作り直さない判定になる」を足した。design D3 の 11.・D4・tasks 2.1・3.2

## R3. tasks 1.1 の 1 本目は `$TMPDIR/err` を残す。verify-run は検証の後に一時フォルダの残りを数え、`HARNESS_TMP_LEAK=fail` の repo では rc=0 でも FAIL にする
- 成果物: openspec/changes/ch07-declare-verify-reset/tasks.md
- 根拠: tasks.md:29（`2>"$TMPDIR/err"` を作り、消さない）/ `.harness2/scripts/evidence.py:399`（検証コマンドに検証ごとの `TMPDIR` を渡す）・`:401-406`（`os.listdir(tmp)` を `tmp_left` に記録）・`:415-416`（`HARNESS_TMP_LEAK=fail` なら `FAIL`）/ 走っている CH03（`openspec/changes/ch03-test-tmp-cleanup`）の後で ashiato2 が `fail` に切り替える予定
- kind: technical
- 提案: 標準エラーを変数で受ける形にする（例: `bash -c 'e=$(mktemp); trap "rm -f $e" EXIT; out=$(HARNESS_VERIFY=1 tools/verify-env.sh 2>"$e") && … && ! grep -q "^HARNESS_RESET:" "$e"'` の `mktemp` を `$TMPDIR` の下に作り trap で消す）か、`{ out=$(…); } 2>&1` で分けて取る。標準エラーに出ないことは本体の 5. でも確かめるので、1.1 から外してもよい
- 処置: fixed 1.1 —— 標準エラーは捨て、標準エラーに出ないことは 3.1 の 5. に任せる。一時ファイルを作らない

## R4. tasks 4.3 の `openspec validate ch07-declare-verify-reset --strict` は verify-run が拾わず、走らない
- 成果物: openspec/changes/ch07-declare-verify-reset/tasks.md
- 根拠: tasks.md:75 / `evidence.declared` で 4.3 から取れたコマンドは 6 本で `openspec validate` が無い（`python3 -I -c 'import evidence; … e.declared(it)'` で列挙）/ `.harness2/scripts/evidence.py:67`（`CMD_LIKE` は `./`・`tools/`・`scripts/`・`python3`・`bash` などで始まるものだけ。`openspec` で始まる backtick は捨てる）
- kind: technical
- 提案: `bash -c 'openspec validate ch07-declare-verify-reset --strict'` と書く
- 処置: fixed 4.3 —— `bash -c '…'` で包んだ

## R5. tasks 4.3 の「変えないもの」の確かめは Global Constraints の一部しか見ず、基準の `origin/main` は CH06 を取り込んだ下流のブランチでは CH06 の差分を含む
- 成果物: openspec/changes/ch07-declare-verify-reset/tasks.md
- 根拠: tasks.md:20（変えないものに `tools/env-init.sh`・`tools/ports.sh`・`.github/workflows/` を挙げる）と :74（diff の対象にこの 3 つが無い）/ `git -C /home/yosis/dev/ashiato2-ch06 diff --stat origin/main...HEAD -- tools .github docker-compose.yml crates migrations` → CH06 は `tools/env-init.sh`・`tools/ports.sh`・`.github/workflows/ci.yml`・`tools/agent-env.sh`・`tools/verify-prep.sh` を変える / `.harness2/docs/flow-gates.md:252-256`（下流は依存の実装を main への merge を待たずに取り込む。依存を取り込んだブランチは main を merge する）
- kind: technical
- 提案: 基準を CH06 の取り込んだ SHA（下流の `deps.json` に記録される）にし、diff の対象を Global Constraints の全部（`tools/env-init.sh tools/ports.sh .github/workflows` を含む）にそろえる。基準を `origin/main` のままにするなら、3 つを外した理由（CH06 の差分が混ざる）を 4.3 に 1 行書く
- 処置: fixed 4.3 —— 基準は origin/main のまま、3 つを外した理由と、CH06 の検査が CH06 の振る舞いを固定することを書いた

## R6. tasks 3.2 の最後の検証（DB の volume がちょうど 1 つ）は、verify-run がその直前に準備で DB を作り直すので、検査の 10. の戻しを見ていない
- 成果物: openspec/changes/ch07-declare-verify-reset/tasks.md
- 根拠: tasks.md:65（「検査の後の DB は volume を 1 つ持つ形に戻っている」）/ `.harness2/scripts/evidence.py:383-388`（検証コマンドの 1 本ごとに `tools/verify-env.sh` を `HARNESS_VERIFY=1` で先に呼ぶ）/ 10. で tmpfs のコンテナが残っていても、その準備が `rm -s -f db` → `up`（tmpfs のファイルを含まない `COMPOSE_FILE`）で volume を持つ形に立て直す（CH06 `tools/verify-env.sh:55-71`・`:123`）
- kind: technical
- 提案: この 1 本を外し、戻しの確かめは tasks.md:60 のとおり検査の本体の中（10. の戻しで `db_vols` がちょうど 1 つ）に任せる。残すなら「準備の後の状態を見ている」と書く
- 処置: fixed 3.2 —— その 1 本を外し、戻しは検査の本体の中で確かめると書いた

## R7. premises の P5・P7 と proposal の Impact、C5 の反転条件が、spec の段の前提（CH06 は gate 済みの `c666673`）と合わない古い記述のまま
- 成果物: openspec/changes/ch07-declare-verify-reset/premises.md
- 根拠: premises.md P5（「7 Task のうち Task 1 が完了、Task 2 が進行中」「本体（D8 の 1.〜8.）は Task 5 でまだ無い」「`exit 0` が待ちのループの中…`tools/verify-env.sh:13`・`:21`・`:22`」、確かさ 2）/ premises.md C5 末尾・proposal.md Impact（「CH06（下流が走っている。7 Task のうち Task 1 が完了）」「CH06 の Task 5 が検査の本体の段を変えたら」）/
  一方 deps.json（`spec_sha: c666673…`）と design.md:9-14（`c666673` の本体 1.〜8. と `verify-env.sh` の判定・手順を読んで D2・D3 を書いた）/ `/home/yosis/dev/ashiato2-ch06/tools/check-verify-env-reset.sh` は本体 1.〜8. を持つ（`:86`〜`:253`）/
  P7（「印つきで作り直さない木は CI でない main の checkout だけ」）と C3・spec.md:38（git の木でない木も作り直さない側）
- kind: technical
- 分類の直し（spec の段）: premise から technical に直した。前提そのもの（CH06 の判定・手順・検査の本体があること）は gate 済みの `c666673` の実物を読んで確かめ直し、決定は変わらない。古かったのは questions の段の記述だけ
- 提案: P5 の結果を「`c666673`（gate 済み）で判定・手順・切った接続の 1 行・非 0・検査の本体 1.〜8. が揃っている」に直し、確かさを上げる（実物を読んだ）。C5 と proposal の反転条件は design の Risks（:107。CH06 の gate 後の SHA が進んだら merge gate の「依存の spec:」で落ちる）に合わせる。P7 に git の木でない木を足す
- 処置: fixed premises.md —— P5・P7・C5 の反転条件・proposal.md Impact

## R8. spec に harness2 の関数名 `check_verify_env` が入り、Requirement の本文が SYSTEM でなくハーネスの振る舞い（「認める」）を言う
- 成果物: openspec/changes/ch07-declare-verify-reset/specs/verify-environment/spec.md
- 根拠: spec.md:32（Scenario の WHEN「`.harness2` の `init.py` が `check_verify_env` を持つとき」）/ spec.md:9（「ハーネスの点検（`init.py --check`）がこの準備を…認める」—— 決めるのは `.harness2/scripts/init.py:361-374` で、この repo の SYSTEM ではない。前提は premises.md P4）
- kind: technical
- 提案: WHEN を「ハーネスの点検（`init.py --check`）が準備の宣言を確かめる版のとき」のように関数名を外した言い方にする。Requirement の本文は「`tools/verify-env.sh` の本文に字 `HARNESS_RESET: testdb` を置く（ハーネスの点検は本文の字で読む —— premises P4）」と SYSTEM の側の約束に寄せる
- 処置: fixed specs/verify-environment/spec.md —— Requirement の本文を準備の側の約束に寄せ、Scenario の WHEN から関数名を外した

## R9. CH07.md の完了の判定は「DB のコンテナが volume を持たない」を「作り直さない場合」に数えるが、spec と design は「作り直しても宣言を出さない」（作り直しの経路は通る）と書く
- 成果物: docs/changes/CH07.md
- 根拠: docs/changes/CH07.md の「完了の判定」1 つ目（「作り直さない場合（…・DB のコンテナが volume を持たない）」）/ spec.md:57-60（Scenario 名「作り直しても宣言を出さない」、THEN に `切った接続: ` の行 = 作り直しの経路を通った）/ design.md:43-44（`rm -s -f db` まで CH06 のとおり進み、tmpfs なら中身は消えるが宣言しない）
- kind: technical
- 提案: CH07.md を「作り直さない場合（印が無い・CI でない main の checkout・git の木でない・作り直せずに非 0）と、消した volume が 0 個のとき（中身が残りうるので宣言しない）と `--plan` は出さない」に分けて書く。確認バッチと archive の判定がこの文を読むので、spec と同じ区切りにする
- 処置: fixed ../../../docs/changes/CH07.md —— 作り直さない場合と、消した volume が 0 個のときを分けた

## R10. `--self-test` にハーネスの点検（`init.py --check`）を入れると、検査の冒頭の約束「CI の rust job と手元で同じ結果」が崩れるが、D5 は冒頭の注記の直しにそれを含めていない
- 成果物: openspec/changes/ch07-declare-verify-reset/design.md
- 根拠: CH06 `tools/check-verify-env-reset.sh:4-5`（「`--self-test` … CI の rust job と手元で同じ結果になる」）/ design.md:88-89（`.harness2` があれば走らせ、無ければ SKIP —— CI は SKIP、worktree は実行）/ `.harness2/scripts/init.py:355-358`（`--check` は `gh repo view` を呼ぶ。`gh` の認証・ネットワークに出力が依る）/ design.md:95（D5 は冒頭の注記に「ch07 の段（9.・10.）と宣言の確かめ」だけを足す）/ CH06 の木の `docs/flow-gates.md:15`（「`--self-test` は docker の daemon に触らない」）
- kind: technical
- 提案: D5 と tasks 4.1 に、冒頭の注記の「同じ結果」に例外（ハーネスの点検は `.harness2` のある木だけ。CI は SKIP）を書き足すことを入れる。`init.py --check` は rc=1 でも 1.5 秒ほどで終わり（2026-10-25 にこの木で実測）、`[ok]`/`[人間]` の行の判定は `gh` の結果に依らないので、判定そのものは変えなくてよい

---

## 観点 1: deep の決定が正典に写っているか

`deep.md`・`premises.md` の C1〜C9 と、依頼の完了の判定（CH07.md の「依頼（逐語）」）を 1 件ずつ当てた。

| 決定 | 効く先 | 写り |
|---|---|---|
| C1 行は `HARNESS_RESET: testdb` そのもの・行頭から | spec.md:7-8、Scenario は「`HARNESS_RESET: testdb` だけの行」（tasks・D3 は `grep -cx` で数える） | 写った |
| C2 自身の echo・役割の後・rc=0 の直前・条件（volume ≥1 を消した / コンテナが無くて立てた）・止まっていても出す・volume 0 個は出さない・途中で落ちたら出さない | spec.md:5-9・:37-39、Scenario :13・:22・:26・:57 / design D2 / tasks 1.1 | 「途中で落ちたら出さない」だけ Scenario に無い → R2 |
| C3 印なし・CI でない main・git の木でない・非 0 は出さない。none も出さない | spec.md:37-39、Scenario :43・:48・:53。none は否定の Scenario が `^HARNESS_RESET:` の 0 行で覆う | 「git の木でない」が Scenario に無い → R2 |
| C4 CH06 の振る舞いを変えない・検証コマンドの側に出さない | spec.md:10、:40（`--plan`）、Scenario :62 / tasks Global Constraints・4.3（CH06 の検査をそのまま走らせる）| 写った（4.3 の確かめの範囲は R5） |
| C5 検査は CH06 の検査に足す | design D3・D4、tasks 2.1〜3.2 | 写った（反転条件の古さは R7） |
| C6 `requires: [CH06]` | CH07.md の front matter `requires: [CH06]` | 写った |
| C7 判定 3 は `[人間]` が無く `[ok]` がちょうど 1 つ・rc は見ない・SHA を残す | Scenario :31-33、tasks 2.2・4.2（`git -C .harness2 rev-parse HEAD`・`|| exit 1`）、CH07.md 完了の判定 2 つ目 | 写った（関数名の混入は R8） |
| C8 #112 が先でも順を変えない | design D1 | 写った（（仮）の印が外れた → R1） |
| C9 「作り直したときだけ」は判定と C2 の両方 | spec.md:5-6（Requirement「作り直すのは、印があり…」を名指し） | 写った |
| 依頼の完了の判定 1〜4 | CH07.md の完了の判定・spec の 2 Requirement | 写った（判定 3 は C7 の読み替え。review/deep.md R1） |

- 当初案を覆したもの（「判定が `reset` なら出す」→「中身を実際に消したときだけ」）: 古い方は spec に残っていない。spec.md:57 の Scenario が新しい側を固定する
- 要件へ戻すもの: 無し（`satisfies: []`）
- 数値: 決定に含まれる数は「1 行」「volume 1 つ以上 / 0 個」だけで、どれも Scenario にある

## 観点 2: Scenario が検証可能か

R2・R8。ほかに確かめたこと:
- 10 本の WHEN / THEN はどれも観測できる言葉（rc・標準出力 / 標準エラーの行の数・`切った接続:` の行・`core.event` の件数・`docker inspect` の volume の数）。「正しく」「適切に」の類は無い
- THEN に `rc=0` と行の数を束ねた Scenario が多いが、検査の段は `want "rc=0"` と数の確かめを別の行で NG にする形（CH06 の `want`/`ok`）なので、片方だけで緑にはならない
- CI だけ・worktree だけで通る Scenario: 「CI でない main の checkout では…」は CI の rust job（CH06 `.github/workflows/ci.yml:88` が本体を走らせ、CI の checkout は worktree でないので 3. の CI の段に入る）、「ハーネスの点検が…」は worktree の verify-run（tasks 2.2 の `! grep -q "^SKIP"`・4.2）。`check_scenarios.py` は印の在りかを静的に見るので両方とも担保は立つ。分担は CH07.md の完了の判定と design D3・D4 に書かれ、CH06 の 3.・8. の SKIP と同じ型。R にはしない

## 観点 3: 置き場の誤り

R8（spec の関数名）。ほかに確かめたこと:
- design の D1〜D6 のうち観測できる振る舞いを述べるもの（D2 の出す条件・volume 0 個で出さない・止まっていても出す）は spec の Requirement と Scenario に写っている。D3・D4 は検査の手順、D5・D6 は文書と delta の形で、正典に要らない
- proposal の Capabilities（Modified: `verify-environment`）と `specs/verify-environment/` が一致。capability は CH06 が立てるもので正典 `openspec/specs/` にはまだ無く、delta は `ADDED` だけ（D6）。`docs/stories/INDEX.md` は change を載せない（`grep -n "verify-environment\|CH06\|CH07" docs/stories/INDEX.md` は 0 件。CH04・CH06 と同じ）

## 観点 4: tasks が検証を持つか

R3・R4・R5・R6。ほかに確かめたこと:
- 全 9 項目に検証コマンドと rc の条件がある（`evidence.py lint` は OK。`declared` で各項目から 2〜6 本が取れる）
- `grep -c 'echo "Scenario: '` の下限: CH06 の検査（`c666673`）でいま 23 本。Task 2 の 2 本で 25、3.1 の 6 本で 31、3.2 の 2 本で 33 —— tasks.md:55・:63 の下限と合う。`-ge` なので CH06 側の後の印の追加でも落ちない。名前の一致は 4.3 の `check_scenarios.py` が見る
- 引用: 1.1 の 4 本目 `grep -v "^\s*#" … | grep -c "echo \"HARNESS_RESET: testdb\""` は `bash -c '…'` の中の二重引用で `echo "HARNESS_RESET: testdb"` を探す形になり、書いたまま走る。3.2 の `--format "{{… \"volume\" …{{\"\n\"}}…}}"` も同じ（R6 は中身の空振りで、書き方は走る）
- 依存順: 1.1（宣言の行）→ 2.1（本文の字の確かめ・`--plan`）→ 2.2（`init.py --check` は 1.1 の後でないと `[人間]`）→ 3.1・3.2 → 4.x。逆順は無い
- 4.1 の確かめの前提（CH06 の木の `docs/flow-gates.md` に `tools/check-verify-env-reset.sh` の行がある）: `docs/flow-gates.md:15` にある
- 2.1 の 2 本目の合言葉の字面（`POSTGRES_PASSWORD=a`）は、`tools/check-db-secret.sh` が見る範囲（`docker-compose.yml`・`crates/*.rs`・`tools/*.sh`・`.github/workflows/*.yml`。`:22-25`）の外なので止まらない
- 人間の確認待ち: 無し。画面も実機も無く、CH07.md も置かないと書く
- **design D3（仮）の tmpfs の段は成り立つ**（2026-10-25、この検査で実測。一時の compose project を作り、終わりに消した）:
  CH06 の `docker-compose.yml` と `tools/db-roles.sh` を一時の木に写し、`.env` に `COMPOSE_FILE=docker-compose.yml` と合言葉を置き、別の場所に `services.db.tmpfs: [/var/lib/postgresql/data]` のファイルを置いた。
  環境変数 `COMPOSE_FILE=docker-compose.yml:<tmpfs のファイル>` で `docker compose config` に tmpfs が入り（環境変数が `.env` に勝つ）、`up -d --wait db` は Healthy、`docker inspect` の `Mounts` は `[]`・`HostConfig.Tmpfs` に `/var/lib/postgresql/data`。
  同じ環境変数を渡して `tools/db-roles.sh` を呼ぶと、中で `.env` を `set -a` で読み直して `COMPOSE_FILE` が tmpfs を含まない値に戻っても `docker compose exec` は同じコンテナを見つけ rc=0（`== 役割を整えた`）。`.env` だけの `docker compose ps -aq db` も同じコンテナを返した。同じ環境変数での `rm -s -f db` も rc=0。
  したがって D3 の反転条件（`db-roles.sh` が落ちたら override の形へ）は、いまの compose では当たらない見込み。P9 にこの結果を足せる。ただし D3 の表の 1.「前の状態に依らず 1 行」は、10. の途中で落ちて tmpfs のコンテナが残った木で本体を**直接**呼ぶと 1 回だけ 0 行で NG になる（verify-run 経由なら準備が先に立て直すので起きない）。D3 :76 の記述と合わせて読めば足りるので R にはしない
- D3 の表の 8.: 2 回の rc≠0 は、`drop_db` の後で DB のコンテナが無く `reset_db` が印を立ててから `clear_blocking_network` が `exit 1` で落ちるので、`exit 0` に届かず 0 行。最後の立て直しは 1 行。D2 と整合する（ただし Scenario の印が無い → R2）

## 観点 5: change の文書（`docs/changes/CH07.md`）

R9。ほかに確かめたこと:
- `check_chain.py` は OK（change は要件から来ないので観点 8 は無い）
- 「完了の判定」は空でなく、4 つ（宣言の出る・出ない、`init.py --check`、CH06 の振る舞い、確認待ちを置かない）が deep の C1〜C9 と spec に合う（ずれは R9 の 1 か所）
- 「壊してはいけないもの」: CH06 の振る舞いは CH06 の 24 本の Scenario（検査の 23 本と `tools/check-ports.sh --self-test` の 1 本）を tasks 4.3 がそのまま走らせて固定する。none を出さない・中身が残ったまま宣言しないは CH07 の Scenario（:43〜:60 の否定の 4 本・:57）が固定する。台本・試験・移行に触らないは 4.3 の diff（範囲は R5）
- 「依頼（逐語）」の範囲: spec が依頼を越えるのは「volume 0 個は出さない」「止まっていても出す」「git の木でない」で、どれも deep の C2・C3（review/deep.md R8）に決定がある。欠けは無い
- この段は CH07.md から `hx change` の雛形の「## 依頼 / ## なぜ / ## 完了の判定（雛形）」の重複を消し、「価値」に整えた。逐語の引用（「依頼（逐語）」）は残っている

## 観点 6: 答えが写ったか

該当なし（`spec --answers` の呼び出しでない。深掘りは問い 0 件）。

## 観点 7: 走っている change との前提関係
- ch06-reset-verify-db / 作り直すのは、印があり、worktree か CI のときだけ: 前提（CH07 の Requirement「作り直したら…宣言する」の WHEN が「作り直す木」をこの Requirement の名前で名指しし、Scenario「印が無ければ…」「CI でない main の checkout では…」も CH06 の判定の表（`keep` の経路）が在ることを当てにしている。CH06 の判定が変われば CH07 の Scenario も書き直しになる）

CH06 は `docs/changes/CH07.md` の `requires: [CH06]` にあるので、requires を足す指摘は立てない。
- 処置: fixed D5 —— 4.1 も同じく。冒頭の注記に --self-test の例外を足す

