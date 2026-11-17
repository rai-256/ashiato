# CH16 spec review — ch16-install-check-ng-observations

## 機械の検査（2026-11-17）

| 検査 | 結果 |
|---|---|
| `openspec validate ch16-install-check-ng-observations --strict` | `Change 'ch16-install-check-ng-observations' is valid` |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)` |
| `python3 scripts/check_scenarios.py . ch16-install-check-ng-observations` | `scenarios: FAIL (担保なし 10 件 / 名無しの確認待ち 0 件)`。10 件はこの change の Scenario の全部で、実装の前なので印がまだ無い。上流では指摘にしない。warn 8 件はほかの台本のもので、この change とは関係しない |
| `python3 scripts/review_triage.py . ch16-install-check-ng-observations` | `指摘 0 件 / 仮決め 0 件 / 要件へ戻すもの 0 件`、`triage: OK` |
| `python3 scripts/spec_overlap.py candidates . ch16-install-check-ng-observations` | `[]`（0 件） |

追加で実測したもの:
- tasks 2.1 と 3.1 の `--list` の diff の命令は、書いたままで rc=0 になった（いまは同じファイル同士を比べている）。tasks 1.1 の `bash -c '…bad.sh…'` は、検査の台本がまだ無いので rc=1 になった。つまり何もしなくても通ってしまう検証ではない
- `.github/workflows/ci.yml` の 609〜610 行は `chain` の job（567 行から）の中にある。`rust` の job（94 行から）ではない（R2）
- `set -euo pipefail; f(){ false; echo inside-after-false; }; [[ x == x ]] && { f; …; }` を走らせると rc=1 で、`inside-after-false` は出なかった。`$(false; echo …)` の中では set -e が効かず、続きが走った（R1）

A（失う側がある問い）は見つからなかった。escalated は無い。

---

## R1. 「`self_test` の中では set -e が効かない」は事実と違う。効かないのは `$(…)`・`if ( … )`・`||` / `&&` の左の中
- 成果物: openspec/changes/ch16-install-check-ng-observations/design.md / openspec/changes/ch16-install-check-ng-observations/tasks.md
- 根拠: design.md:13-14 と tasks.md:19-20 は、`[[ … ]] && { self_test; exit 0; }` の `&&` の左にあるので効かない、と書いている。しかし `self_test` は最後の `&&` の**右**にあり、bash では errexit が効く。同じ形の実験は rc=1 で止まり、`false` の次の行は走らなかった。`update-check-install.sh` は末尾が `case`（:380-386）で、`&&` ですらない。一方、`bash -c 'set -e; x="$(false; echo after)"; echo "x=$x"'` は `x=after-false-in-cmdsubst` を出した（`$(…)` の中では効かない）。`expect_fail` の `if ( "$@" )`（install-check.sh:1532、update-check-install.sh:289）の中も効かない
- kind: technical
- 提案: Context と Global Constraint を、事実に合う形に直す:「`self_test` の本体では set -e が効く。だから、落ちるはずの判定を `( … ) 2>err; rc=$?` と書くと、自己試験がそこで黙って rc=1 で終わる。落ちるはずの判定は `if` か `|| rc=$?` で受ける。`$(…)`・`if ( … )`・`||` の左では set -e が効かない（`die` の部品も、そこで呼ばれたときは失敗で止まらない）」。`|| die` で書く規則は残してよい。
- 処置: fixed design.md — Context に set -e の効く所・効かない所を事実の形で書き、tasks の Global Constraints を「落ちるはずの判定は if か || rc=$? で受ける。確かめは || die で書く」に直した

## R2. CI で 2 つの `--self-test` を走らせる job は `chain` で、`rust` ではない
- 成果物: openspec/changes/ch16-install-check-ng-observations/specs/install-check-diagnostics/spec.md
- 根拠: spec.md:47、design.md:10、proposal.md:23、tasks.md:11・:81、docs/changes/CH16.md:86 が `rust` と書いている。ci.yml では `rust:` は 94 行、`chain:` は 567 行から始まり、609〜610 行は `chain` に属する（job の見出しを awk で数えた）。ci.yml:498-499 のコメントも「Linux の自己試験（chain の job）」と言っている。`rust` は `if: needs.changes.outputs.code == 'true'` を持つ別の job なので、`rust` の緑を見ても自己試験が通った証にはならない。呼び出し元の依頼文（「rust の job の 609〜610 行」）にも同じ誤りがある
- kind: technical
- 提案: 6 か所の `rust` を `chain` に直す。Scenario「いまの判定の部品と Scenario の一覧は変わらない」の WHEN と、CH16.md の「完了の判定」の CI の行も直す。
- 処置: fixed specs/install-check-diagnostics/spec.md — spec・design・proposal・tasks・CH16.md の `rust` を `chain` に直した（Scenario の WHEN は「2 つの台本は検査を通る」へ寄せた）

## R3. spec は 2 つの台本の両方が「共通の様子」を出すと言うが、tasks の条件では `update-check-install.sh` が出さない。しかもその台本の `URL` は healthz の先ではない
- 成果物: openspec/changes/ch16-install-check-ng-observations/specs/install-check-diagnostics/spec.md
- 根拠: spec.md:11-15 は、2 つの台本について、実地の確かめの中では 3 つのサービスの状態・healthz・設定のキーの名前・記録の末尾を出すと書いている。ところが tasks.md:32-33 は、共通の様子を「台本が `STATE`・`WORK`・`URL`・`SETTINGS`・`OWNER_SETTINGS` を持つときだけ」出すとしている。`update-check-install.sh` には `STATE`・`SETTINGS`・`OWNER_SETTINGS` が無い（:5-11。設定は :255・:264・:275 で `'C:/ProgramData/ashiato/settings/server.json'` を直に書いている）ので、この条件では出ない。さらに、その台本の `URL='http://127.0.0.1:18701'`（:9）は偽の配布元の受け手で（:147・:171）、サーバの healthz ではない。条件を「どれかを持つとき」と読む実装をすると、`$URL/healthz` は偽の配布元へ飛ぶ。Scenario「実地の確かめの中の NG は…」の担保は `install-check.sh` にしかない（tasks.md:57）
- kind: technical
- 提案: 共通の様子に使う入力は、台本が明示的に渡す名前（例: `NG_HEALTHZ_URL`・`NG_SETTINGS_FILES`）で受ける。台本の変数名を推測して読まない。`update-check-install.sh` も共通の様子を出すかは、どちらかに決めて spec と tasks をそろえる。出すなら tasks 3.1 に渡す値と Scenario の担保を足す。出さないなら spec.md:14-15 を `install-check.sh` に限る。
- 処置: fixed D3 — 共通の様子の入力を台本が名前で渡す `NG_STATE_DIR`・`NG_SETTINGS_FILES`・`NG_HEALTHZ_URL`・`NG_OBSERVED_DIR` にし、update-check-install.sh も渡す（healthz はサーバの 18700。偽の配布元の URL は渡さない）と D3 と tasks 3.1 に書いた

## R4. Requirement が言う「命令が落ちる・戻らないとき」と「取れなかったことを 1 行書く」を、どの Scenario も確かめていない
- 成果物: openspec/changes/ch16-install-check-ng-observations/specs/install-check-diagnostics/spec.md
- 根拠: spec.md:16 は「観測を取る命令が無い・落ちる・戻らないときも、NG の行と rc 1 は変わらず、取れなかったことを観測の塊に 1 行で書く」と言う。これに当たる Scenario は spec.md:38-40 だけで、確かめるのは「無い」場合の rc と 1 行目だけ。「戻らない」場合（design D3 の 30 秒で打ち切る）を試すものが無い。`die` の中で観測の命令が戻らないと、CI の job はその上限まで止まる。本人の「なぜ」（CI の 1 周＝約 1 時間を無駄にしない）が、そのまま壊れる形になる。また WHEN の「`sc.exe` も `curl` も無い（Linux の）環境」は、CI の Linux に `/usr/bin/curl` がある以上、PATH から curl を抜く手段を書かないと作れない。PATH ごと抜くと grep や sed なども消える。spec.md:15 の「サーバのログ（の末尾）」も、Scenario「実地の確かめの中の NG は…」（spec.md:29-31）の THEN に無い。Requirement の本文だけが言っている
- kind: technical
- 提案: Scenario を 2 つ足す。①偽の `sc.exe` が戻らない（`sleep` する）とき、打ち切りの秒数を上書きできる環境変数（例: 1 秒）のもとで、`die` は rc=1 で終わり、`（取れない: sc.exe rc=124）` の 1 行を出す。②命令が無い・落ちるときも、その 1 行が出る。spec.md:39 の作り方は、要る道具だけを並べた PATH の置き場を作る形で、tasks 1.1 に書く。spec.md:31 の THEN には、偽のサーバのログの末尾の行を足す。
- 処置: fixed specs/install-check-diagnostics/spec.md — Scenario「観測を取る命令が無い・落ちても…取れなかったことを書く」「観測を取る命令が戻らなくても、時間の上限で打ち切って NG で終わる」を足し（`NG_OBSERVE_TIMEOUT`）、命令の無い環境の作り方を tasks 1.1 に、サーバのログの末尾を実地の Scenario の THEN に足した

## R5. 観測できる振る舞いが design にだけある。archive で正典から落ちる
- 成果物: openspec/changes/ch16-install-check-ng-observations/design.md
- 根拠: design にしか無い振る舞いは次のとおり:
  - 見出しの `== 観測（ここから）` / `== 観測（ここまで）`（design.md:27）。CH16.md:87 の「完了の判定」は、実地の確かめをこの見出しが出ることに頼っている。spec には見出しが無いので、実装が文字を変えても spec は落ちない
  - 伏せる値を読めないときに、ファイル・ログの末尾・命令の記録を出さない厳しい側（design.md:55-56）
  - 直近 12 件・1 件 40 行・各 30 秒の上限（design.md:35・:47-48）
  - 印 `# ng-observations: <理由>` を付けた行を除くことと、理由が空なら外れにすること（design.md:70）
  - 検査の決まり 4（部品を `source` していない台本や、`die` を定義し直した台本を落とす。design.md:68）
- kind: technical
- 提案: 見出しの 2 行、伏せる値を読めないときの振る舞い、印の扱い、決まり 4 を、Requirement の本文と Scenario に移す。数値は（仮）のままでよいが、spec に書いて Scenario が値を確かめる形にするか、spec に「上限がある」とだけ書いて上限の値の担保を tasks に置くかを決める。
- 処置: fixed specs/install-check-diagnostics/spec.md — 見出しの 2 行・伏せる値を読めないときの振る舞い・印の扱い・決まり 4 を Requirement と Scenario に移した。数値の上限は spec に「上限がある」とだけ書き、値は design D2・D3 の（仮）に残した

## R6. 検査の Scenario が決まりごとに分かれておらず、1 つの決まりが壊れても緑のままになる
- 成果物: openspec/changes/ch16-install-check-ng-observations/specs/install-check-diagnostics/spec.md
- 根拠: spec.md:60 の写し（`echo "NG: …" >&2; exit 1`）と、tasks.md:43 の検証の `bad.sh`（`f() { echo "NG: x" >&2; exit 1; }`）は、どちらも同じ 1 行で決まり 1（`die` の外の `NG:`）と決まり 2（`die` の外の 0 でない `exit`）に外れる。`bad.sh` には `source` も無いので、決まり 4 にも外れる。このため、決まり 1 の見分けが壊れていても、決まり 2 か 4 が同じ行で落とすので緑になる。spec.md:54 の `exit` の決まりと、design D5 の決まり 4 を単独で試す Scenario も無い
- kind: technical
- 提案: Scenario と tasks 1.1 の検証を、1 つの写しが 1 つの決まりにだけ外れる形にする。たとえば `echo "NG: x" >&2` だけの行、`exit 3` だけの行、`source` を欠く台本、`die() {` を定義し直す台本、理由が空の印。それぞれについて、出力に外れた決まりの名前と行の番号があることを見る。
- 処置: fixed specs/install-check-diagnostics/spec.md — 検査の Scenario を決まりごとに分け（1 つの写しが 1 つの決まりにだけ外れる）、印の Scenario を足した。tasks 1.1 の検証も決まり 1・2 を別々の写しで見る形にした

## R7. 「観測の口を通しても判定が見る出力は変わらない」が標準出力と rc の全量しか比べていない。標準エラーと、途中で閉じるパイプを含めていない
- 成果物: openspec/changes/ch16-install-check-ng-observations/specs/install-check-diagnostics/spec.md
- 根拠: spec.md:42-44。判定の多くは命令の標準エラーを使っている。`sc.exe query x >/dev/null 2>&1`（install-check.sh:1273-1275）、`sc.exe queryex … 2>&1`（`service_diagnostics`）、`icacls.exe … 2>/dev/null` がその例。包む関数が標準エラーを記録に取り込んで呼び手へ流さないと、`2>&1` で受ける判定の入力が変わる。また `set -o pipefail` のもとで `… | grep -q` と組むとき、出力がパイプの大きさを超えると、包む前と後で終了の値が変わる。実測（Linux）: 200 KB を出す命令を直に `| grep -q MATCH` へ流すと rc=1（書き手が BrokenPipe で 1 で終わる）、出力をためてから `printf` する関数で包むと rc=141 で、3 回とも同じだった。Windows の命令が閉じたパイプへの書き込みの失敗を無視して 0 で終わる場合、包む前は rc=0 で通り、包んだ後は 141 で `|| die` に落ちる。判定の条件が変わる形になる。これは「壊してはいけないもの」の 1（CH16.md）に直に効く
- kind: technical
- 提案: Scenario の WHEN と THEN に、①標準エラーも包む前と同じであること、②`set -o pipefail` のもとでパイプの大きさ（64 KiB）を超える偽の出力を `| grep -q <先頭の行>` で受けたとき、パイプ全体の終了コードが `command <命令>` のときと同じであること、を足す。
- 処置: fixed D2 — 記録する口は命令の 2 つの出力を別々のファイルに受け、そのまま返し、閉じたパイプへの書きを失敗にせず命令の終了コードを返すと D2 に書き、Scenario に標準エラーの比べとパイプが途中で閉じる場合を足した

## R8. Scenario「いまの判定の部品と Scenario の一覧は変わらない」は、その WHEN（`--self-test`）では確かめられない主張を AND で束ねている
- 成果物: openspec/changes/ch16-install-check-ng-observations/specs/install-check-diagnostics/spec.md
- 根拠: spec.md:46-49。AND の「`--list` の出力がこの変更の前と 1 行も変わらない」は、変更前の版と比べないと決まらない。`self_test` の中に置く印（tasks.md:61）では比べられず、比べているのは tasks 2.1 と 3.1 の `git merge-base` との diff の命令だけになっている。THEN の「いまの合う出力と外れた出力の試しがそのまま通る」は、今の試しを消しても rc=0 のままなので、真偽が決まらない
- kind: technical
- 提案: 2 つに分ける。①WHEN「merge-base の台本と変更後の台本の `--list` を比べる」→ THEN「差が無い」。担保は tasks の diff の命令に `# Scenario:` を付けた検査の台本に置く。②今の試しが残っていることは、変更前の `self_test` にある `|| die` と `expect_fail` の行が全部変更後にも在ることで測る形にするか、Scenario から外して tasks の制約に置く。
- 処置: fixed specs/install-check-diagnostics/spec.md — Scenario「いまの判定の部品と Scenario の一覧は変わらない」を外し、`--list` の不変は Requirement の本文と tasks 2.1・3.1 の diff に、今の確かめが残ることは Global Constraints と tasks 2.1・3.1 の検証（変更前の `|| die`・`expect_fail` の行が残る）に置いた

## R9. 観測が標準出力に出ないこと（壊してはいけないもの 3）を確かめる Scenario が無い
- 成果物: openspec/changes/ch16-install-check-ng-observations/specs/install-check-diagnostics/spec.md
- 根拠: CH16.md の「壊してはいけないもの」3 と、design D6（design.md:81-82）は、観測を標準エラーだけに出すと言う。spec.md:17 も標準出力を変えないと言う。しかし、どの Scenario の THEN も標準エラーに出ることしか見ていない（spec.md:23・:27・:31・:35）。観測を標準出力にも出す実装でも全部が緑になり、CI の `tee` と `== Scenario:` の数え（ci.yml:506-511）に観測の塊が混ざる
- kind: technical
- 提案: spec.md:20 の Scenario に「**AND** 標準出力は空である（観測の塊の行が無い）」を足す。
- 処置: fixed specs/install-check-diagnostics/spec.md — 最初の Scenario に「標準出力に観測の塊の行は無い」を足し、Requirement の本文にも標準エラーにだけ出すと書いた

## R10. 秘密の Scenario は「今の設定の値」しか試さない。型で伏せる値（前の API の合言葉、前のログインの印）と、厳しい側を試していない
- 成果物: openspec/changes/ch16-install-check-ng-observations/specs/install-check-diagnostics/spec.md
- 根拠: spec.md:33-36 は、設定のファイルの秘密と画面の合言葉と印の値を混ぜて見るだけで、`secret_values` が読む「今の値」に限られる。実地の確かめは、作り直した後の前の値も命令に渡している。`http_code -H "Authorization: Bearer $old_token"`（install-check.sh:1202）と `-H "Cookie: ashiato_session=$cookie_old"`（:1065）がそれで、D2 の記録は命令の引数も書くので、これらは記録に残る。こうした値を伏せるのは D4 の型（`Authorization:`・`ashiato_session=`）だけで、それを確かめる Scenario が無い。伏せる値を読めないときの厳しい側（design.md:55-56）にも Scenario が無い（R5）
- kind: technical
- 提案: 秘密の Scenario に 2 つの WHEN を足す。①今の設定に無い値を `Authorization: Bearer <値>` と `Cookie: ashiato_session=<値>` の形で命令の引数に混ぜる → 標準エラーにその値が無い。②設定のファイルを読めないようにして NG にする → 命令の記録・ファイルとログの末尾の行が無く、`（伏せる値を読めないので出さない）` がある。
- 処置: fixed specs/install-check-diagnostics/spec.md — Scenario「設定に無い秘密も、見出しの形で伏せる」「伏せる値を読めなければ、命令の記録とファイルの末尾を出さない」を足した

## R11. tasks 3.1 の `ng_secret_values` が、`update-check-install.sh` の持つ画面の合言葉を入れていない
- 成果物: openspec/changes/ch16-install-check-ng-observations/tasks.md
- 根拠: tasks.md:71 は「この台本は設定の秘密を読まないので、状態の置き場の設定が読めればその秘密を」とだけ言う。一方 `update-check-install.sh` は、画面の合言葉を字面で持ち、インストーラへ渡している（`'update-check-test-password-0123'`・`'update-check-release-password-0123'`。:173-190 と `repair_password`）。D4（design.md:54）は「試験の画面の合言葉（`PW1` など台本が持つもの）」も伏せると言っている。install-check.sh 側の tasks 2.1（:48）は合言葉を入れている
- kind: technical
- 提案: tasks 3.1 の `ng_secret_values` に、この台本の 2 つの合言葉を足す。読めない設定のときも、合言葉は出す（字面なので必ず読める）。
- 処置: fixed 3.1 — update-check-install.sh の `ng_secret_values` に字面の 2 つの合言葉を入れ、設定を読めなくても出すと tasks 3.1 と D4 に書いた

## R12. `common_teardown` と `install_with` の `NG:` を「部品を通す」と言うが、そのための口が design に無い
- 成果物: openspec/changes/ch16-install-check-ng-observations/design.md
- 根拠: design D1 の口は `die` と `die_seen` だけで、どちらも `exit 1` する（design.md:25-26）。ところが部品を通す先の 2 つは exit しない、または出し先が違う:
  - `common_teardown`（install-check.sh:717-727）は `NG:` を出した後も、利用者の片付けと `$WORK` の削除を続け、元の終了コードを返す
  - `install_with`（:675-681）は、呼び手が標準エラーを `$out` へ向けているので、fd 9 へ書く
  
  D5 の決まり 1（design.md:65）と tasks 2.1（tasks.md:51-52）は、どちらも部品を通すことを求めている。しかし、exit しない口も、出し先を選ぶ口も定めていない。そのまま `die` を呼ぶと、teardown の片付けが途中で切れるか、`install_with` の NG と観測が `$out` に入ってログに出なくなる。決まり 2 に当たる `console_probe` の `exit 2`（:1472・:1478）の扱いも、tasks に無い
- kind: technical
- 提案: design D1 に、観測の塊だけを出して戻る口（例: `ng_report <理由> [fd]`）を足し、`die` はその口を呼んでから exit する形にする。tasks 2.1 には、`common_teardown` と `install_with` をその口で書くこと、`console_probe` の `exit 2` に印 `# ng-observations: <理由>` を付けることを書く。
- 処置: fixed D1 — 戻る口 `ng_report <理由> [観測] [fd]` を D1 に足し、die はそれを呼んで exit する形にした。common_teardown・install_with（fd 9 と印）・console_probe の exit 2（印）の扱いを D1 と tasks 2.1 に書いた

## R13. 検査が「観測を出さずに NG で終わる道が無いこと」を見ると言うが、set -e で止まる道は静的に見えない
- 成果物: openspec/changes/ch16-install-check-ng-observations/specs/install-check-diagnostics/spec.md
- 根拠: spec.md:53 と design の Goals（design.md:18「NG で終わる道の全部が観測を出す」）。実地の確かめには、set -e で黙って止まる道がある。`unzip -q "$archive" -d "$WORK/unpacked"`（install-check.sh:697）が落ちると、`NG:` も観測も無しに 0 でない値で終わる。設定を単独の代入で読む行（例: :1079 の `api="$(setting API_TOKEN)"`）で `PY` が落ちるときも同じ形になる（単独の代入は `$(…)` の失敗で set -e が効く。R1 の `$(…)` の中とは別）。いまの台本に `trap … ERR` は無い（grep で 0 件）。検査の決まり 1〜4 はこれを捉えないので、Requirement の本文が担保より強いことを言っている
- kind: technical
- 提案: どちらかに決めて spec を合わせる。①spec.md:53 を「`NG:` の行と 0 でない `exit` を、観測を出す口の外に持たない」に狭め、set -e で止まる道は対象の外と明記する。②`trap '<観測の塊だけを出す口>' ERR` を部品に置き、set -e で止まるときにも観測を出す。この場合は Scenario と、終了コードを変えないことの担保を足す。本人の決定の「判定が NG になったとき」は①で満たされるので、②は本人に聞かずに決めてよい範囲。
- 処置: fixed D8 — ①を採った。Requirement の本文を「NG: を出して終わる道」に狭め、set -e で判定の外の命令が落ちて止まる道は対象の外と spec と D8 に書いた（反転条件つき）

## R14. `die` の観測の塊（共通の様子）は、本物の Windows で一度も走らないまま merge される
- 成果物: openspec/changes/ch16-install-check-ng-observations/tasks.md
- 根拠: tasks.md:81 の終了の条件は `install-windows` の緑だが、緑の回は OK の道しか通らず、`die` の D3（実物の `sc.exe queryex`・`timeout`・`curl` を Git Bash で呼ぶ部分）は走らない。CH16.md:87 も「次に落ちたときが実地の確かめ」としている。ただしその「次」は、まさに観測が要る回になる。そこで塊が壊れていれば（`timeout` が Git Bash で sc.exe を止められない、など）、本人が避けたかった周回がもう 1 回起きる
- kind: technical
- 提案: ci.yml は変えずにできる。`install-windows` が既に走らせている `install-check.sh --probe-console`（ci.yml:501）か実地の確かめの冒頭で、部分殻の中で本物の命令に向けて観測の塊を 1 回出させ、見出しの 2 行と `ashiato-db` の行（サービスが無くても `sc.exe` の答えの行）があることを見る。あるいは、この確かめは置かないと決めて CH16.md に理由を書く。
- 処置: fixed D7 — install-check.sh の実地の確かめの始めに、観測の塊を本物の命令で一度出して見出しと 3 つのサービスの行を確かめる D7（仮）と Scenario を足した（ci.yml は変えない）

## R15. ST58 の `update-apply-install.sh` を検査の外に残し、申し送りも置かない
- 成果物: openspec/changes/ch16-install-check-ng-observations/proposal.md
- 根拠: 本人の決定は「tools/release の install 系の台本の判定が NG になったとき…出さない判定を検査で落とす」（CH16.md の「議論で決めたこと」）。proposal.md:47-49 と design.md:76-77 は、ST58 が足す `update-apply-install.sh`（install 系の台本）を対象にしない。理由は、ST58 が main に無く、lane の予定のパスの外にあるからで、ここは妥当。ただし proposal は「このときの申し送りは置かない」としている。そのため、ST58 の merge の後にこの台本を名指しへ足す仕事が、どこにも記録されない。走っている Story へ差し戻さない理由は、記録を置かない理由にならない（受け皿の issue か後続の change の候補として置けば、ST58 に触れない）
- kind: defer
- 提案: ST58 の merge の後に、`check-ng-observations.sh` の名指しへ `update-apply-install.sh` を足すかを決める持ち越しを、受け皿（#307 の続き、または新しい issue）に 1 行置く。proposal.md:49 の「申し送りは置かない」はその記録へ差し替える。
- 処置: fixed proposal.md — proposal に「持ち越し」の節を足し、ST58 の merge の後に update-apply-install.sh を名指しに足すかを人間が決める持ち越しとして書いた（PR 本文にも載せる。issue は上流の段では作らない）

## R16. ST58 との重なりの記述が、ST58 の枝の今と合わない
- 成果物: openspec/changes/ch16-install-check-ng-observations/proposal.md
- 根拠: proposal.md:44-46 と tasks.md:24 は「ST58 は `install-check.sh` の Scenario の印の文と 1 つの Scenario を直す」と書いている。しかし `git diff --stat $(git merge-base origin/main origin/feat/st58-update-apply-rollback) origin/feat/st58-update-apply-rollback -- tools/release .github` に出るのは、`ci.yml`・`check-artifacts.sh`・`package.sh`・`update-apply-install.sh` の 4 つだけ。`install-check.sh` と `update-check-install.sh` は出ない。`移行が失敗すると、サーバのサービスを開始せずに止まる` も、ST58 の tip（33c2fed45）で main と同じ字面（:44・:1421）のまま。ST58 の tasks 6.1 は `[x]` だが、その直しはしていない
- kind: technical
- 提案: 「今の ST58 の枝は 2 つの台本に触れていない（2026-11-17、33c2fed45）。ST58 が後から触れたら、後に merge する側が rebase で合わせる」と、事実と手順に分けて書き直す。tasks.md:24 の「判定の本体を書き換えない」は、衝突を避けるためでなく、判定の条件を変えないためとして残す。
- 処置: fixed proposal.md — 今の ST58 の枝（33c2fed45）は 2 つの台本に触れていないと事実の形に直し、tasks の Global Constraints も同じにした。判定の本体を書き換えない理由は判定の条件を変えないためとした

## R17. spec に台本の中の関数の名前が入っている
- 成果物: openspec/changes/ch16-install-check-ng-observations/specs/install-check-diagnostics/spec.md
- 根拠: spec.md:26 に `assert_acl_only`、spec.md:54 に `die`、spec.md:64 に `"$(windows_dir)/System32/sc.exe"` がある。これらは台本の中の関数の名前で、実装で名前を変えると spec が嘘になる。外から呼ぶ名前（台本のパス、`--self-test`・`--list`、`sc.exe` などの Windows の命令）は契約なので、残してよい
- kind: technical
- 提案: spec.md:26 は「受けた値を見る判定（icacls の行を受けるもの）」、spec.md:54 は「観測を出す口」、spec.md:64 は「`System32` の下の命令の場所を直に書いた呼び出し」と言い換える。関数の名前は tasks と design に残す。
- 処置: fixed specs/install-check-diagnostics/spec.md — 関数の名前を「受けた icacls の出力の行を見る判定」「観測を出す口」「System32 の下の sc.exe の場所を直に書いて呼ぶ」に言い換えた

## R18. 「ci.yml を変えない」と `docs/testing.md` の追記に検証の命令が無い
- 成果物: openspec/changes/ch16-install-check-ng-observations/tasks.md
- 根拠: tasks.md:11 は「`.github/workflows/ci.yml` は変えない」と言うが、ci.yml は lane.json の予定のパスに入っている（lane.json `paths`）。そのため、後で lanes が diff を確かめても、ci.yml を変えたことは落ちない。tasks 3.1 の `docs/testing.md` の追記（tasks.md:75-76）にも検証が無い
- kind: technical
- 提案: tasks 3.1 の検証に 2 つ足す。`bash -c 'git diff --quiet "$(git merge-base HEAD origin/main)" -- .github/workflows/ci.yml'` rc=0 と、`grep -q 'check-ng-observations.sh' docs/testing.md` rc=0。
- 処置: fixed 3.1 — `git diff --quiet <merge-base> -- .github/workflows/ci.yml` と `grep -q check-ng-observations.sh docs/testing.md` を 3.1 の検証に足した

---

## 観点 1: deep の決定が正典に写っているか

deep.md は軽い路の申告だけで、Q を持たない。本人の決定は CH16.md の「議論で決めたこと」にあるので、それを 1 件ずつ当てた。

- **何を**（NG のときに、サービスの状態・設定・healthz・trace の該当部分を出す。出さない判定を検査で落とす）:
  - spec.md:9-17 と :51-56 に写っている
  - `update-check-install.sh` の「共通の様子」が tasks では出ない食い違いは R3
  - set -e で止まる道は R13
  - install 系の 3 つ目の台本は R15
- **なぜ**（`sc query` の 4 行目）: spec.md:20-23 の Scenario がその形をそのまま使っている。写っている
- **どこまで**（台本の判定と自己試験。CI のほかの job と製品のコードは変えない）:
  - tasks.md:9-12 に写っている
  - 検証の命令が無いことは R18
- 本人の決定に数値は無い。design の数値（12 件・40 行・30 秒・400 行）はどれも（仮）で、置き場は R5 で挙げた

## 観点 2: Scenario が検証可能か
R4・R6・R7・R8・R9・R10 で挙げた。それ以外の Scenario（spec.md:20・:25・:29・:63・:67）は、WHEN と THEN が自己試験で観測できる言葉で書かれている。spec.md:29-31 の THEN は 4 つの項目を束ねているが、どれも同じ塊に出るかを見るもので、1 つの主張として読める。ただし Requirement の本文（spec.md:15）にある「サーバのログの末尾」は、この THEN に無い。これは R4 に含めた。

## 観点 3: 置き場の誤り
R5（design にだけある振る舞い）と R17（spec の中の関数の名前）。

- proposal の Capabilities（New: `install-check-diagnostics`、Modified: なし）と `specs/` のディレクトリは一致する
- MODIFIED・REMOVED・RENAMED は無い（軽い路の制約を守っている）
- delta の `## Purpose` は、過去の change と同じ形（例: ch04 の `test-database`）
- `docs/stories/INDEX.md` には、ほかの change の capability（`test-database`・`verify-environment`・`test-hygiene`）も載っていない（grep で 0 件）。change は INDEX の対象の外なので、指摘にしない

## 観点 4: tasks が検証を持つか
R1・R2・R14・R18 で挙げた。

- 1.1・2.1・3.1 は、どれもコマンドと rc の終了の条件を持つ。順は 1（部品と検査）→ 2・3（寄せる）で、依存の向きと合う
- `install-windows` が「下流の手元では走らない」ことは、tasks.md:81 に書いてある
- tasks 2.1 と 3.1 の `--list` の diff の命令は、書いたままで rc=0 になった。`bash <(git show …) --list` の形でも、旧版の `root` は `/` になるだけで、`--list` には効かない
- tasks 1.1 の `bad.sh` の検証が決まりを分けていない件は R6

## 観点 5: change の CH16.md
- 「完了の判定」は書かれている。ただし CI の job の名前の誤りが R2、見出しの文字が spec に無いことが R5 にある
- 「壊してはいけないもの」の 1〜6 のうち:
  - 2（1 行目と rc）は spec.md:20・:38 で Scenario になっている
  - 1（判定の条件）は R7、3（標準出力）は R9、4（秘密）は R10・R11、5（ci.yml）は R18、6（今の自己試験）は R8 が、それぞれ担保の不足として挙げる
- 「依頼（逐語）」の範囲を越えるものは無い。新しい台本 `check-ng-observations.sh` も、本人の「出さない判定を検査で落とす」の実体として範囲の中にある。欠けているものは R15
- `check_chain.py` は OK。change なので観点 8（再生成との一致）は無い

## 観点 6: 答えが写ったか
該当なし（`spec --answers` の呼び出しではない。軽い路で、deep の Q が無い）。

観点 7: 該当なし（spec_overlap.py candidates が 0 件）

（ST58 とのファイルの重なりの記述の食い違いは、前提関係でなく記述の誤りとして R16 で挙げた。）
