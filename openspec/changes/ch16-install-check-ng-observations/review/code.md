<!-- 処置の判定: re-review -->
# CH16 code verify — ch16-install-check-ng-observations

検証の日: 2026-11-17。HEAD 7699e746e（base 7109e94aa）。作業ツリーは変えていない（壊す試しは `/tmp/ch16rv/r` の写しで行った）。

## 申告と実測

申告: 3/3 の Task が `[x]`。検証コマンドは 13 本。

| Task | 検証コマンド | 申告 | 実測 |
|---|---|---|---|
| 1.1 | `tools/release/check-ng-observations.sh --self-test` | rc=0 | 一致（rc=0、11 秒、`check-ng-observations: self-test OK`） |
| 1.1 | `bash -c '…bad.sh…"f() { echo \"NG: x\" >&2; }"…'` | rc=0 | 一致（rc=0、`bad.sh:3: 決まり 1`）。注: `mktemp`＋`trap rm -rf` の形が道具の安全検査に止められたので、置き場を固定の `/tmp/ch16rv` にした同じ中身で走らせた |
| 1.1 | `bash -c '…bad.sh…"f() { exit 3; }"…'` | rc=0 | 一致（rc=0、`bad.sh:3: 決まり 2`）。上と同じ注 |
| 2.1 | `tools/release/install-check.sh --self-test` | rc=0 | 一致（rc=0、12 秒） |
| 2.1 | `tools/release/check-ng-observations.sh tools/release/install-check.sh` | rc=0 | 一致（rc=0） |
| 2.1 | `--list` を merge-base と diff（install-check） | rc=0 | 一致（rc=0、80 行） |
| 2.1 | 今の自己試験の `\|\| die`・`expect_fail` が残る（install-check） | rc=0 | 一致（rc=0） |
| 3.1 | `tools/release/update-check-install.sh --self-test` | rc=0 | 一致（rc=0、11 秒） |
| 3.1 | `check-ng-observations.sh install-check.sh update-check-install.sh` | rc=0 | 一致（rc=0） |
| 3.1 | `--list` を merge-base と diff（update-check-install） | rc=0 | 一致（rc=0） |
| 3.1 | 今の自己試験の確かめが残る（update-check-install） | rc=0 | 一致（rc=0） |
| 3.1 | `git diff --quiet <base> -- .github/workflows/ci.yml` | rc=0 | 一致（rc=0） |
| 3.1 | `grep -q check-ng-observations.sh docs/testing.md` | rc=0 | 一致（rc=0） |
| 3.1 終了の条件 | PR の CI の `chain` と `install-windows` が緑 | — | **未実測**。`gh pr list --head feat/ch16-install-check-ng-observations` は `[]`、`git ls-remote origin` も何も返さない（push も無い）。D7 の確かめ（本物の `sc.exe`・`timeout`・`curl` を Git Bash で呼ぶ部分）と、Windows でだけ起きることは、まだ誰も観測していない |

`python3 scripts/check_scenarios.py . ch16-install-check-ng-observations` は rc=0（`scenarios: OK`）。この change の 21 件の Scenario には全部印がある
（check-ng-observations.sh に 14 件、install-check.sh に 6 件、update-check-install.sh に 2 件。「2 つの台本は検査を通る」は 2 か所にある）。

## 手 1: 該当なし

確かめた範囲: 3 つの台本の自己試験に、実装の出力を写した期待値（ハッシュ・直書きのバイト列）は無い。期待値は全部、偽の命令に与えた入力の文字列だった。

## 手 2: ガードをわざと壊す（写しで、1 つずつ壊して戻した）

| 壊したもの | 走らせたもの | rc |
|---|---|---|
| 決まり 1 の判定を `if (0)` にする | check `--self-test` | 1（落ちた） |
| 決まり 3 の判定を `&& 0` にする | check `--self-test` | 1（落ちた） |
| `ng_redact` の sed（見出しの形）を外す | check / install-check `--self-test` | 1 / 1 |
| 同じ記録をまとめる分岐を `if false` にする | check `--self-test` | 1 |
| `ng_emit` から `ng_part_records` を外す | check `--self-test` | 1 |
| 一度集めた値を残す（`cat "$sf"`）を外す | check `--self-test` | 1 |
| `ng_try` の `timeout` を外す | check `--self-test` | 600 秒で終わらず PID で止めた。rc は観測していない |
| install-check に旧の `die() { echo "NG: …"; exit 1; }` を戻す | install-check `--self-test` | 1 |
| update-check の `windows_sc` を記録の口から外す | update-check `--self-test` | 1（検査の決まり 3） |
| install-check の `ng_secret_values` から `$PW1` / owner の読みを外す | install-check `--self-test` | 1 / 1 |
| **update-check の `ng_secret_values` から字面の合言葉 / 設定の読みを外す** | update-check `--self-test` | **0 / 0**（R1） |
| **`common_setup` から `observation_probe` の呼び出しを外す** | install-check `--self-test` | **0**（R2） |
| **`observation_probe` の中の `die_seen`（sc.exe の答えの行が無い）を外す** | install-check `--self-test` | **0**（R2） |
| **install-check の `common_setup` の `NG_*` の行を消す** | install-check `--self-test` | **0**（R3） |
| **update-check の `check_zip` の `NG_*` の行を消す** | update-check `--self-test` | **0**（R3） |
| **`install_with` の時間切れの `ng_report … 9` の fd を消す（標準エラーへ）** | install-check `--self-test` | **0**（R5 の添え） |

検査の外側: `exit "$rc"`・`f || exit $?`・`cat <<<"NG: x" >&2`・`f() { false; }; f` だけを持つ写しは、どれも `check-ng-observations.sh` が rc=0 で通す（R7）。
`tools/release/` の install 系の台本は名指しの 2 本だけだった（`ls tools/release | grep -i install`）。

## 手 3: Scenario と test

印の先は全部、主張を観測していた。ただし次の 2 つは階層がずれる。
- 「CI の Windows で、観測の塊を本物の命令で一度出してから実地の確かめに入る」: 印は関数の定義の上にあり、試験は関数を単体で、しかも通る側だけで走らせている（R2）
- 「伏せる値を読めなければ…」: 試験は `ng_secret_values() { return 1; }` と書いて、設定のファイルを読めなくしていない。実際の読み手でも確かめた。install-check.sh の `ng_secret_values` を、壊れた JSON の設定に向けて走らせると `（伏せる値を読めないので出さない）` になり、記録は出なかった。だから install-check では主張は成り立つ。update-check の読み手はどの試験も通らない（R1）

## 手 4: 本人の決定

`deep.md` は軽い路で、「本人の答え」が無い。本人の決定は `docs/changes/CH16.md` にあり、数値は持たない。
- 列挙（サービスの状態・設定・healthz・trace）: `ng_emit` から `ng_part_services` / `ng_part_health` / `ng_part_settings` / `ng_part_files` / `ng_part_serverlog` を 1 つずつ外すと、install-check `--self-test` はどれも rc=1 になった（固定されている）
- 「出さない判定を検査で落とす」: 旧の `die` を戻す・`windows_sc` を記録から外す、のどちらでも rc=1（固定されている）
- 「CI の他の job と製品のコードは変えない」: `ci.yml` の diff は rc=0。diff は `tools/release/` の 3 本・`docs/testing.md`・`openspec/changes/ch16-…` だけ
- 仮の上限の値（design D2・D3）は固定されていない（R6）

## 手 5: tasks の `[x]` と実体

13 本は全部実在し、rc=0 だった（上の表）。`cargo test --test` 型のコマンドは無い。
3.1 の終了の条件（PR の CI が緑）は、PR も push も無いので未実測。

## 手 6: 隙間

- 字面の合言葉が記録から素で出る（R4）。伏せる値を読めないときでも、`install_with` の時間切れはファイルの末尾を出す（R5）
- 記録のたびに伏せる値を集め直すので、遅くなる（R8）
- ST58 の `update-apply-install.sh` は対象の外（design D5 で決めた持ち越し）なので、`defer` にしない

## 手 7: 該当なし

確かめた範囲: `python3 scripts/plan_fix.py pending openspec/changes/ch16-install-check-ng-observations` は `[]`（rc=0）。`correction-review.md` は作っていない。

---

## R1. update-check-install.sh の伏せる値（`ng_secret_values` / `ng_read_secrets`）は、どの試験も通らない。消しても全部緑のまま
- 成果物: tools/release/update-check-install.sh / tools/release/check-ng-observations.sh
- 根拠: 写しで、update-check-install.sh:41 の `printf '%s\n' 'update-check-test-password-0123' 'update-check-release-password-0123'` を `:` にしても、:42-43 の `server.json` の読みを `:` にしても、`update-check-install.sh --self-test` は rc=0 だった。Scenario「観測に秘密の値が出ない」の担保は install-check.sh:1835 だけにある。update-check の読み手は `C:/ProgramData/ashiato/settings/*.json` を直に書き、`python -` を heredoc で起こす。この形は install-check の `PY` とは別の作りで、どこでも走っていない。ここが壊れても、伏せる値を集められずに厳しい側（記録を出さない）へ倒れるだけで、漏れは起きない。ただし、合言葉の行を落とす変更は誰も気づけない
- kind: technical
- 提案: 設定のファイルの場所を変数（既定は今の `C:/ProgramData/…`）に出す。`self_test` で偽の `server.json` と 2 つの字面の合言葉を偽の `sc.exe` の出力に混ぜ、`die` の標準エラーに出ないことを見る（install-check.sh:1835 と同じ形）

## R2. D7 の確かめ（`observation_probe`）は、呼び出しを消しても、落とす分岐を消しても、自己試験が緑のまま
- 成果物: tools/release/install-check.sh
- 根拠: 写しで install-check.sh:738 の `observation_probe` の呼び出しを外しても、:755-756 の `die_seen "観測の塊に $service の sc.exe の答えの行が無い"` を `|| true` にしても、`install-check.sh --self-test` は rc=0 だった。自己試験（:1798-1801）は、正しい偽の `sc.exe` で通る側を 1 回走らせるだけ。落ちる側は手で確かめた。偽の `sc.exe` を `true` にすると rc=1 で、偽の `curl` を `true` にしても rc=1。だから分岐そのものは動く。Scenario の印（:744）は関数の上にあるので、`check_scenarios` は「実地の確かめが始まるときに走る」ことまで担保ありと数える。CI の `install-windows` も、呼び出しが消えれば何も見ずに緑になる
- kind: technical
- 提案: 自己試験に 2 つ足す。① `common_setup` の本文に `observation_probe` の行があることを `awk`/`grep` で見る。② 何も出さない偽の `sc.exe`、または healthz の行を出さない偽の `curl` で `ng_run observation_probe` を走らせ、rc=1 で `NG: 観測の塊に` の行があることを見る

## R3. 2 つの台本の実地の確かめで観測の入力を渡す行（`NG_*`）を消しても、自己試験は緑のまま
- 成果物: tools/release/install-check.sh / tools/release/update-check-install.sh
- 根拠: 写しで install-check.sh:719（`NG_STATE_DIR="$STATE" … NG_OBSERVED_DIR="$WORK"`）を消しても `install-check.sh --self-test` は rc=0、update-check-install.sh:228-229 を消しても `update-check-install.sh --self-test` は rc=0 だった。install-check の `ng_run`（:1782-1790）は `NG_*` を自分で入れ直してから判定を走らせるので、台本の配線を通らない。この行が消えると、実地の NG はサービス・healthz・設定・記録のどれも出さず、`NG:` の行と判定が渡した値だけに戻る。それでも CI は緑のまま（D7 の確かめは install-check の `common_setup` の中にあるが、R2 のとおり守られていない。update-check には D7 の確かめが無い）
- kind: technical
- 提案: 自己試験で、台本の本文に 4 つの名前を渡す行があることを `grep` で見る（2 つの台本それぞれ）。update-check にも D7 と同じ一度きりの確かめを `check_zip` の始めに置くかは、design D7 の範囲を広げるかどうかで決める

## R4. 書き出しの合言葉 `install-check-export-0123` が、命令の記録と観測の塊に素のまま出る
- 成果物: tools/release/install-check.sh / tools/release/check-ng-observations.sh
- 根拠: install-check.sh:1063-1065 は、包んだ `curl` に `--data '{"password":"install-check-export-0123"}'` を渡す。`ng_secret_values`（:399-404）の列挙はこの値を持たない。部品と install-check の `ng_secret_values` をそのまま使い、同じ `curl` を 1 回呼んでから `die` すると rc=1 になる。標準エラーに `[命令] curl … --data {"password":"install-check-export-0123"} http://127.0.0.1:18700/export (rc=0)` の行が出て、`observed.log` にも 1 件残った（`grep -c` で両方 1）。実地では、直後の :1069 の `die`（書き出しの zip ができない）がこの記録を直近 12 件に持ったまま出す。値は公開の repo に字面で書かれているので、新しく漏れるものは無い。ただし design D2 の「`curl --data` の合言葉などを素のまま残さない」とは合わない。spec が伏せる対象に挙げる「台本が使う画面の合言葉」と同じ種類の値でもある。`ng_secret_values` は手書きの列挙なので、台本に合言葉を足しても何も落ちない
- kind: technical
- 提案: `install-check-export-0123` を `ng_secret_values` に足す。加えて、自己試験で台本の本文から `password` を含む字面の値を抜き出し、`ng_secret_values` の出力に全部あることを見る（列挙の書き忘れを機械で落とす）

## R5. 伏せる値を読めないときも、`install_with` の時間切れはインストーラの出力の末尾を出す
- 成果物: tools/release/install-check.sh / tools/release/check-ng-observations.sh
- 根拠: spec は「設定のファイルがあるのに読めないときは、命令の記録とファイルとログの末尾を出さず」と言う。`ng_emit`（check-ng-observations.sh:160）は判定が渡した値（`obs`）を `ok` に依らず出す。install-check.sh:699-701 の時間切れは、`tail -n 80 "$out"`（インストーラの出力のファイルの末尾）を `seen` に入れて渡す。そのため、この経路は「読めないときは末尾を出さない」を迂回する。再現は合成で行った。壊れた JSON の設定に向けた install-check の `ng_secret_values` のもとで、`ng_report "インストーラが戻らない" "installer said tok-unread-1" 2` を呼ぶ。すると `（伏せる値を読めないので出さない）` の行と一緒に `installer said tok-unread-1` がそのまま出た（`tok-unread-1` は設定の `API_TOKEN` の値）。base も fd 9 へ `tail -n 80 "$out"` を素で出していたので、退行ではない。この change が足した約束の穴である。あわせて、この時間切れの経路の fd 9 を消して標準エラーへ向けても、自己試験は rc=0 だった（手 2 の表）
- kind: technical
- 提案: `install_with` は `$out` の末尾を `seen` に入れず、`$out` を `NG_OBSERVED_DIR` の中に置いて `ng_part_files` に拾わせる（読めないときは部品が出さない）。または `ng_report` の第 4 引数で「渡す値はファイルの末尾」と印を付け、`ok=0` なら出さない

## R6. 観測の上限の値（記録 12 件・1 件 40 行・ファイル 2 つ・末尾 40 行・時間 30 秒）は、変えても全部通る
- 成果物: tools/release/check-ng-observations.sh
- 根拠: 写しで次の 5 つを 1 つずつ変えた。`max=12`→1000、`per=40`→4、`head -n 2`→50、サーバのログの `tail -n 40`→1000、`NG_OBSERVE_TIMEOUT:-30`→600。`check-ng-observations.sh --self-test` と `install-check.sh --self-test` は、どれでも rc=0 だった。spec は「観測の塊の量には上限があり」「1 つずつに時間の上限がある」と言うが、上限が効くこと自体を見る試験が無い（30 秒の試験は `NG_OBSERVE_TIMEOUT=1` で上書きしている）。値は design D2・D3 の（仮）で、本人の決定ではない
- kind: technical
- 提案: 13 件の別の記録と 41 行の出力を作って NG にし、最初の記録が出ないこと・`（以降 1 行省略）` があることを見る試験を 1 本足す。既定の秒数は `grep -q 'NG_OBSERVE_TIMEOUT:-30'` で固定するか、（仮）のまま固定しないと design に書く

## R7. 検査の決まり 2 は、字面の数字の `exit` しか見ない
- 成果物: tools/release/check-ng-observations.sh
- 根拠: check-ng-observations.sh:290 は `exit[[:space:]]+[1-9]` だけを見る。部品を読み込んだ写しに `f() { local rc=3; exit "$rc"; }`、`f || exit $?`、`cat <<<"NG: x" >&2; kill $$` のどれかだけを置くと、`check-ng-observations.sh <写し>` は rc=0 になる。今の 2 本で変数の `exit` は update-check-install.sh:191 の `cleanup`（受けた rc の素通し）だけで、これは正当。ただし spec の決まり 2 は「0 でない終了コードの `exit`」と言っていて、変数の形が外にあることは spec にも design にも書いていない
- kind: technical
- 提案: 変数の `exit`（`exit "$…"`・`exit $?`）も決まり 2 の対象にし、今の `cleanup` には `# ng-observations: 受けた終了コードの素通し` の印を付ける。または、字面の数字だけを見ることを spec の決まり 2 に書く

## R8. 包んだ命令は 1 回ごとに伏せる値を集め直すので、Linux で 1 回あたり約 20 倍遅くなる
- 成果物: tools/release/check-ng-observations.sh / tools/release/install-check.sh
- 根拠: `ng_record` は記録のたびに `ng_collect_secrets` → `ng_secret_values` を呼ぶ。install-check では、設定が 2 つあるとき Python を 2 回起こす。Linux で、設定の 2 つのファイルがある状態で偽の `sc.exe` を 30 回呼んだ。直に呼ぶと 172 ms、包んで呼ぶと 3403 ms だった（1 回あたり約 6 ms → 約 113 ms）。待ちのループ（`wait_state` など）は 1 秒ごとに `sc.exe`・`curl` を呼ぶので、1 回ごとの待ちが延びる。Windows（Git Bash の fork と Python の起動）では未実測
- kind: technical
- 提案: PR の `install-windows` の所要を base の直近の run と比べる。目に見えて延びたら、設定のファイルの更新時刻が変わったときだけ集め直す（design D3 の反転条件に当たる）
