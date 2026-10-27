# CH06 コードのレビュー（ブランチ全体）

## final review（2b924da..63bd736）

席: `superpowers:requesting-code-review` の `code-reviewer.md`（review package `.superpowers/sdd/tasks/review-2b924da4..63bd7367.diff`）。
判定: **With fixes**（Critical なし・Important 2・Minor 11）。reviewer が手元で走らせたもの: `tools/check-verify-env-reset.sh --self-test` rc=0（CI に似せた環境でも rc=0）/
`openspec validate ch06-reset-verify-db --strict` valid / `python3 scripts/check_scenarios.py . ch06-reset-verify-db` OK / `tools/check-private.sh` rc=0。

Task ごとの ⚠️ の振り分け: T1・T2・T3①・T4 は解消（コード・証跡・P12 で確かめた）。T3②・T5・T6 は CI の結果待ち（R1）。T7① は R9、T7② は R2。

## R1. CI での本体の結果が 1 つも無い（CI だけで通る Scenario と 90 秒の反転条件が未確認）
- 成果物: .github/workflows/ci.yml:87-88 / tools/check-verify-env-reset.sh:95-100
- 根拠: 完了の判定 1 は「CI の rust job で走る」、Global Constraints は「CI と verify-run の証跡を合わせて 24 本が通る」。Scenario「CI でない main の checkout では、印つきで呼んでも行が残る」は worktree では SKIP。branch は未 push で CI の run が無い。reviewer はコードで CI の流れ（`ci-db-env.sh` → env-init 規則 1 → `CI=true` で reset → `env -u CI` で keep → `db-roles.sh` → `as_role`）を追い、破綻は見つけていない
- kind: technical
- 処置: rejected: コードの不具合ではない。reviewer が tools/check-verify-env-reset.sh:95-100 と .github/workflows/ci.yml:87-88 を追って破綻なし、`tools/check-verify-env-reset.sh --self-test` を CI に似せた環境で rc=0。CI の rust job の緑はグラフの publish → gate が見る関門で、本体の時間（D8 の 90 秒）とあわせて PR 本文に挙げる

## R2. 7.1 が 5 回中 1 回しか通っていない
- 成果物: openspec/changes/ch06-reset-verify-db/design.md:187-192（計測の記録）
- 根拠: 落ちたのは `stay_rebuild_chain_of_short_moves_settles`（3 秒の上限）ほかの時間・負荷に依る試験と、`stay_tests::fresh_db` の `DROP DATABASE … WITH (FORCE)` の権限。`crates/` は変えていない（7.2）。CH06 が入るとどの verify-run も空の DB から始まり、下流がこの揺れを踏み続ける
- kind: technical
- 処置: rejected: CH06 の不具合ではない。完了の判定 2 の証跡は evidence.jsonl の 7.1 PASS（木 4c1f3ac6、rc=0、754 秒）。落ちた試験は design.md:189-191 のとおり単独で通り、`crates/` は 7.2 の `git diff origin/main...HEAD -- crates …` が空。揺れる試験は CH06 の外の不具合として、PR 本文の「未処置の申し送り」に別の change の起票を挙げる

## R3. `cut_line` の `psql -F ' '` は application_name の空白・カンマで出所の表示がずれる
- 成果物: tools/verify-env.sh（`cut_line`）
- 根拠: 空白区切りなので `PostgreSQL JDBC Driver` などで `$1`/`$2` がずれる（合計は正しい）。T4 F3
- kind: technical
- 処置: fixed 4.1 —— tools/verify-env.sh:41,48-49 を psql `-F $'\t'` と awk `-F'\t'` にし、`-F', '` の 2 段目をやめた

## R4. `grep '切った接続: 1'` が「10」にも当たる
- 成果物: tools/check-verify-env-reset.sh:147 / :175 / :189
- 根拠: 境界が無い。`切った接続: 0` も同じ。T5 F3
- kind: technical
- 処置: fixed 5.1 —— tools/check-verify-env-reset.sh:153 を `grep -E '切った接続: 1([^0-9]|$)'`、0 の側は helper `cut0`（:56-58）。表示をわざと `切った接続: 10）` にすると本体 rc=1 を確かめて戻した

## R5. cleanup が `pg_sleep(600)` の backend を残す
- 成果物: tools/check-verify-env-reset.sh:35
- 根拠: kill は `docker compose exec` の CLI だけを止める。次の作り直しで消えるので害は小さい。T5 F4
- kind: technical
- 処置: fixed 5.1 —— tools/check-verify-env-reset.sh:36-38 の cleanup で `application_name='ch06-holder'` の backend を `pg_terminate_backend`（`|| true`）

## R6. self-test の `init` が env-init の stderr を捨て、落ちたとき場合の名前を出さない
- 成果物: tools/check-verify-env-reset.sh:265
- 根拠: `set -e` で NG の行も理由も無く終わる。各失敗が場合を名指しする要求に反する。T2 F3
- kind: technical
- 処置: fixed 2.2 —— tools/check-verify-env-reset.sh:271-277 で env-init の stderr を取り、直前の Scenario の名前つきで `self-test NG` を出す。env-init に `exit 3` を差して rc=1 と場合の名前が出るのを確かめて戻した

## R7. body が `return 1` すると ERR trap が誤解を招く 1 行を出す
- 成果物: tools/check-verify-env-reset.sh:232
- 根拠: `"") body; exit $?` の位置で ERR trap が鳴り「NG [終わりの DB]: 232 行目のコマンドが失敗した」が余分に出る
- kind: technical
- 処置: fixed 5.1 —— fix round 1 の `body || exit $?` は `||` の左で body の errexit と ERR trap を殺していた（re-review の Important。`bash -c 'set -Eeuo pipefail; f(){ trap … ERR; false; echo continued; }; f || exit $?'` で `continued` を再現）。呼び出しを `body; exit $?` に戻し、tools/check-verify-env-reset.sh:238 の `return 1` の前で `trap - ERR` にした。本体 rc=0（52 秒）

## R8. env-init の awk が末尾に改行の無い `.env` に改行を 1 つ足す
- 成果物: tools/env-init.sh:41-45
- 根拠: 「ほかの行は 1 文字も変えない」の字面から 1 byte ずれる。2 回目以降は冪等。T1 F2
- kind: technical
- 処置: fixed 2.1 —— tools/env-init.sh:41-46 の awk を、元の末尾が改行のときだけ末尾に改行を付ける形にした（`COMPOSE_FILE` を末尾に足すときは区切りの改行が付く）

## R9. 7.3 の `openspec validate --strict` の証跡が evidence.jsonl に無い
- 成果物: openspec/changes/ch06-reset-verify-db/evidence.jsonl
- 根拠: reviewer の手元では valid。記録だけが抜けている。T7 F2
- kind: technical
- 処置: rejected: 記録の道具の制約。`scripts/verify-run 7.3` は rc=0 で、check_scenarios・check-private・check-ports・check-verify-env-reset の self-test の 4 本が evidence.jsonl に PASS で入った。`openspec validate` は harness2 の scripts/evidence.py:63 の `CMD_LIKE` に openspec が無く、本文の検証コマンドとして拾われない（`--command` でも rc=2）。手で `openspec validate ch06-reset-verify-db --strict` は rc=0

## R10. `clear_blocking_network` は自分の network が無いときだけ動く（移行の時期の端の場合）
- 成果物: tools/verify-env.sh（`clear_blocking_network`）
- 根拠: 古い既定の subnet の自分の network がある木で、移る先の subnet を消えた木の空の network が塞いでいると、up が自分の network を消した後 `Pool overlaps` で 1 回 BLOCKED_INFRA。次の呼び出しで直る
- kind: technical
- 処置: rejected: 挙動は直さず、注記で受け入れる。移行の時期に一度だけ起きる端の場合で、次の呼び出しでは自分の network が無いので確かめが動いて直る（tools/verify-env.sh:74-76 に注記を足した）。直すには up の前に自分の network を消す経路が要り、D5 の「塞いでいる空の network だけを消す」の範囲を広げる

## R11. `cut_line` は psql がどんな理由で落ちても「DB は止まっていた」と出す
- 成果物: tools/verify-env.sh（`cut_line`）
- 根拠: 役割が無い場合なども同じ表示になり、実際と違うことがある
- kind: technical
- 処置: rejected: 表示は design.md:68 と tasks.md:76 の書式 `切った接続: 0（DB は止まっていた）`（D3。「止まっていた・問えなかった」の両方をこの文言で出すと決めている）のとおり。fix round 1 で文言を変えたが、計画との食い違い（re-review の Minor）になるので戻した。tools/verify-env.sh:45 に「見分けない」旨の注記だけを残した

## R12. docs/flow-gates.md の走る場所の欄が「手元では self-test しか走らない」と読める
- 成果物: docs/flow-gates.md:15
- 根拠: 「ローカル（`--self-test` は本体を除く）」。本体は手元でも走る。T6 F1
- kind: technical
- 処置: fixed 6.2 —— docs/flow-gates.md:15 を「CI の `rust`、手元（本体と `--self-test`。`--self-test` は docker の daemon に触らない）」にした

## R13. check-ports.sh の比べる木の並びが生きている木の全部でない
- 成果物: tools/check-ports.sh:49
- 根拠: demo-ch01 の代わりに demo-ch09 を並べ、生きている demo-st06-t7 が漏れる。subnet は port の単射なので実害は無い。T1 F1
- kind: technical
- 処置: rejected: specs/verify-environment/spec.md:107-109 の Scenario が 7 つの木を名指ししていて、demo-st06-t7 を足すと Scenario とずれる。subnet は port の単射なので被らない。tools/check-ports.sh:48-49 に demo-ch09 が demo-ch01 の番号を継ぐ旨の注記を足した

## final review の fix（67477ca..）と scoped re-review

fix は 1 回（77534268、R3〜R13 を 1 本の dispatch で）。scoped re-review（`.superpowers/sdd/tasks/review-67477cae..77534268.diff`）の判定:
R3・R4・R5・R6・R8・R11・R12 は ADDRESSED、R9・R10・R13 は注記・記録の制約止まり（上の rejected）。
新しい壊れ: Important 1（R7 の直し方が body の `set -e` と ERR trap を殺す）・Minor 1（R11 の文言が design D3 / tasks 4.1 と食い違う）。
第 2 の fix wave は出さず、controller が R7 を `trap - ERR` の形に直し、R11 の文言を計画の書式に戻した（上の処置）。
その後: `tools/check-verify-env-reset.sh --self-test` rc=0 / `scripts/quiet-run reset-final2 -- tools/check-verify-env-reset.sh` rc=0（52 秒）。

## code_verify（独立検証。木 cbc36bba）

席: `code_verify`（手 1〜7）。subnet は `10.<第 2 組>.<第 3 組>.0` を `(第 2 組, 第 3 組)/接頭長` と書く（点区切りの 4 つ組は `tools/check-private.sh` が落とす。R19）。コードは触っていない。壊した複製はどれも作業ツリーで一時的に作り、`git checkout -- tools/` で戻した後に
`git status --porcelain` と `git diff` が空（壊す前と同じ）で、`ch06-sentinel-` のコンテナ・volume・network が 0 であることを毎回確かめた。
検証で残した名前の無い volume（下の R14）は消し、dangling の数を始める前の 103 に戻した。

### 申告と実測

申告: tasks の `[x]` 16/16。evidence.jsonl の最後の記録では検証コマンド 35 本が PASS（7.1 の候補 `tools/test-server.sh` は FAIL で、このファイルは無い。tasks.md には載っていない）。

| 項目 | 検証コマンド（tasks.md のまま） | 申告 | 実測 |
|---|---|---|---|
| 1.1 | `. tools/ports.sh; [[ $(ashiato_db_subnet) == 10.20[01].*.0/24 ]]` / `ASHIATO_DB_PORT=1` で空 / `tools/check-private.sh` | PASS | rc=0 / rc=0 / rc=0（一致） |
| 1.2 | `tools/check-ports.sh --self-test` / `grep -c "Scenario: " ≥ 1` | PASS | rc=0 / rc=0（一致） |
| 1.3 | `.env` の `COMPOSE_FILE` と `git check-ignore` / network の subnet ＝ `ashiato_db_subnet` / `check-private.sh` | PASS | rc=0 / rc=0 / rc=0（一致） |
| 2.1 | 合言葉を環境で渡した一時の木で `.env` も subnet のファイルも無い / `verify-prep.sh` に `env-init.sh` | PASS | rc=0 / rc=0（一致） |
| 2.2 | `--self-test` / CI に似せた環境の `--self-test` / 印 ≥ 7 / `check-private.sh` | PASS | rc=0 ×4（一致） |
| 3.1 | `HARNESS_VERIFY=1 … --plan` が `reset` / 印なしが `keep:` | PASS | rc=0 / rc=0（一致） |
| 3.2 | `--self-test` / 印 ≥ 10 | PASS | rc=0 / rc=0（一致） |
| 4.1 | `HARNESS_VERIFY=1 tools/verify-env.sh 2>&1 \| grep -q "切った接続: "` | PASS | rc=0（一致） |
| 4.2 | 2 回続けて作り直す / 禁じた旗の grep | PASS | rc=0 / rc=0（一致。ただし grep は R18 の書き方を見逃す） |
| 4.3 | 3 回の時間 | PASS | rc=0（5 秒・4 秒・6 秒） |
| 5.1 | 本体 / 印 ≥ 23 / sentinel の残り 0 | PASS | rc=0（40 秒）/ rc=0 / rc=0（一致。ただし sentinel の残りの検査は R14 を見ない） |
| 6.1 | ci.yml の 2 行 / ほかの行に触っていない | PASS | rc=0 / rc=0（一致） |
| 6.2 | docs の 2 か所 | PASS | rc=0（一致） |
| 7.1 | `set -a; . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server` / `--list` ≥ 1000 | PASS（5 回中 1 回） | rc=101（1385 通過・1 落ち、854 秒。R2 の揺れ。下の「7.1 の実測」）/ rc=0 |
| 7.2 | 変えないものの `git diff origin/main...HEAD` が空 | PASS | rc=0（一致） |
| 7.3 | `openspec validate --strict` / `check_scenarios.py` / `check-private.sh` / `check-ports.sh --self-test` / `--self-test` | PASS（validate は記録なし。R9） | rc=0 ×5（一致） |

CI だけで通る段の実測（R1 の補い）: worktree でない clone（`git clone -s`、`.env` なし）で、`CI=true`・3 つの合言葉を環境で・`ASHIATO_DB_PORT=55990`（main の 55432 を避けた）・`COMPOSE_FILE` を外して
`tools/check-verify-env-reset.sh --self-test` rc=0、本体 rc=0（33 秒）。Scenario「CI でない main の checkout では、印つきで呼んでも行が残る」が `ok` 3 本で通り、8. は `SKIP`、`.env` は作られなかった。
同じ clone で `plan()` の main の `keep` を `reset` に替えると本体 rc=1（`NG [3. 印なし]: 行は 3 件のまま` / `標準エラーに keep: の行がある`）。後片付け済み。D8 の 90 秒の反転条件にも当たらない（手元 33 秒。GitHub の runner の値ではない）。

### 7.1 の実測

`HARNESS_VERIFY=1 tools/verify-env.sh`（`切った接続: 0`）で作り直した直後に、tasks.md 7.1 のコマンドを `scripts/quiet-run` で走らせた（`verify-run` は使っていない。evidence.jsonl は変えていない）:
rc=101、`test result: FAILED. 1385 passed; 1 failed`、854 秒。落ちたのは `stay_tests::stay_rebuild_chain_of_short_moves_settles`（stay_tests.rs:2371 の `elapsed() < 3 秒`。「2026-05-01 の作り直しが隣の滞在をたどっている」）だけで、
design.md の計測の記録が挙げる揺れる試験（R2）と同じ。終わった時の load average は 17.79 / 31.08 / 20.73。同じ試験を単独で走らせると rc=0（2.17 秒）。
空の DB から始めたことで新しく落ちた試験は無い。申告の PASS（5 回中 1 回）は、この 1 回の実測では再現しなかった —— 新しい指摘にはせず R2 の揺れとして扱う。

### 手 1: 固定値を独立に再計算する

python の `ipaddress` で `off = port − 55400` から組み直した値は、台本の出力と一致した: 55432→(200, 32)/24（`check-ports.sh` の期待値 `printf … 200 32` と同じ）、
55499→10.200.99、55505→10.200.105、55600→10.200.200、55655→10.200.255、55656→10.201.0、55911→10.201.255、55912→空。この木（55607）は (200, 207)/24 で、`docker-compose.subnet.yml` と network の実際の値も同じ。不一致なし。

### 手 2: ガードをわざと壊す

| 壊したもの | 走らせた検査 | rc | 要点 |
|---|---|---|---|
| verify-env.sh:63-66 の D4（volume を別のコンテナが使っていれば止める）を消す | 本体 | 1 | `NG [4.]: DB のコンテナは同じ id` / `sentinel のコンテナの名前が出る` |
| verify-env.sh:88-91 の D5（network にコンテナが繋がっていれば止める）を `if false` にする | 本体 | 1 | `NG [8.]: rc≠0（rc=0）` / `network は同じ id` |
| verify-env.sh:69 を `rm -s -f -v` にする | 本体 | 1 | `NG [1. 空にする]: rc=0（1。期待は 0）` |
| `plan()` の main の keep を reset に（23 行目） | `--self-test` | 1 | `worktree でない木は keep:（reset…）` |
| `plan()` の印の確かめを消す（17 行目） | `--self-test` | 1 | 印なしの 4 本が NG |
| `plan()` の `CI=true` の枝を消す（22 行目） | `--self-test` | 1 | `worktree でない木の CI=true` が NG |
| `plan()` の worktree の枝を `if false` に（21 行目） | `--self-test` | 1 | `demo-st05`・`demo-verify`・`demo-ch01` の worktree が keep |
| env-init.sh:14（CI では作らない）を消す | `--self-test` | 1 | `.env も docker-compose.subnet.yml も無い（2…）` |
| env-init.sh:15（録画の `.env` に触らない）を消す | `--self-test` | 1 | `.env は呼ぶ前と同じ` が NG |
| env-init.sh:19 を `${ASHIATO_DB_SUBNET:-…}` に | `--self-test` | 1 | subnet が写し元の木の値になる |
| env-init.sh:47 を `mv` に（権限 644） | `--self-test` | 1 | 権限と inode が NG |
| env-init.sh:45 で合言葉の行を書き換える | `--self-test` | 1 | `ほかの行は呼ぶ前と同じ` が NG |
| env-init.sh:23 の override を並べない | `--self-test` | 1 | `COMPOSE_FILE は 3 つを並べる` が NG |
| env-init.sh:34（範囲の外の port で subnet のファイルを消す）を `:` に | `--self-test` | **0** | R17 |
| env-init.sh:44 の `!done` を外す（`COMPOSE_FILE` の行が 2 つあれば 2 つとも残る） | `--self-test` | **0** | R17 |
| verify-env.sh:49 の出所の並べ方（10 種で切る・`×<数>`・`…`）を壊す | 本体 | **0** | R16 |
| ports.sh の 512 の上限と 256 での繰り上がりを壊す（55700 → `(200, 300)/24`） | `check-ports.sh --self-test` / `--self-test` | **0** / **0** | R17 |
| verify-env.sh:69 を `rm -s -f -v` / `rm --volumes` / `down --volumes` にする | 4.2 の禁じた旗の grep | **0** ×3 | R18 |

検査の外側: 本体の後片付けと 5.1 の「sentinel の残り」の検査は、sentinel が作る名前の無い volume を見ていない（R14）。

### 手 3: Scenario と test を突き合わせる

`python3 scripts/check_scenarios.py . ch06-reset-verify-db` rc=0（`scenarios: OK`）。この change の 24 本はどれも印がある。印の先を 1 本ずつ見た:
- 「CI でない main の checkout では、印つきで呼んでも行が残る」: worktree では `SKIP` で、CI の run も無かった（R1）。上の CI に似せた clone で実際に通り、壊すと落ちることを確かめた
- 「溜まった DB は…空になる」: `core.event` は移行の表でなく 1 列の代わりの表（design D8 のとおり）。主張は「表が無いか 0 行」なので階層は合う
- 「作り直した後も、所有者とアプリの役割で繋がる」: `as_role` はコンテナの中から非 loopback のアドレスで繋ぐ（違う合言葉で繋がらないことも見ている）。試験が使う口（ホストの 127.0.0.1:<port>）は verify-env.sh:101-102 の `pg_isready` と `/dev/tcp` が見る。合わせれば主張に届く
- 「ほかの compose project と compose の外のものには触らない」: 別の compose project はコンテナの id だけを見て、その project の volume（postgres の image の匿名 volume）は見ていない。compose の外のほうは volume も見ている
- 「どの 2 つの木も、DB の subnet が被らない」: 7 つの木はどれも off < 256 で、10.201 の側を通る木が無い（R17）
- Requirement「作り直せなければ…」の「docker が失敗したときも非 0」には Scenario が無い（spec のとおり。手 2 の D4 を消した複製では `volume rm` の失敗が rc=1 になった）

### 手 4: 本人の決定が test で固定されているか

| 決定（deep.md） | 値・する/しない | 書き換えると落ちる試験 |
|---|---|---|
| Q1 毎回 volume ごと作り直す（CH04 と重ねる） | 前の volume が無く volume がちょうど 1 つ | 本体 5. と 6.（`rm -v` の複製で rc=1） |
| Q2 構わず作り直し、切った数と出所を 1 行 | `切った接続: <数>`・`application_name@接続元` | 数と 1 つの出所は本体 5. / 6. / 7. が固定する。**`×<数>`・10 種で切る・`…` は書き換えても全部通る**（R16） |
| Q3 subnet を port（台帳の番号）から被らずに | `off = port − 55400`・`10.<200+⌊off/256⌋>.<off mod 256>`・`off < 512` | 55400 と 200 は `check-ports.sh` の main の期待値（off = 32）が固定する。**256 での繰り上がりと 512 の上限は書き換えても全部通る**（R17） |
| Q3 消えた worktree の残りは片付けない（塞ぐ空の network だけ例外） | コンテナの繋がった network は消さない | 本体 8.（D5 を消した複製で rc=1） |
| D2 の判定の表（4 行） | — | `--self-test` の `--plan` の 3 本が全部の行を固定する（4 行それぞれを壊した 4 つの複製で rc=1。worktree の枝を `if false` にすると `demo-st05` など 3 本が NG） |

### 手 5: tasks の `[x]` と実体

16 項目の検証コマンドを tasks.md のとおりに全部走らせ、7.1 の 1 本目（rc=101。R2 の揺れる試験）を除いて全部 rc=0（上の表）。挙がった台本・ファイルはどれも実在する
（`tools/check-verify-env-reset.sh`・`tools/check-ports.sh`・`tools/env-init.sh`・`tools/verify-prep.sh`・`.github/workflows/ci.yml`・`docs/testing.md`・`docs/flow-gates.md`）。
`cargo test --test <名前>` の形は無い。7.1 の `--list` は rc=0（1000 本以上）。

### 手 6: 隙間

- 名前の無い volume が検査のたびに増える（R14。捨てたものではないが、Non-Goals の「宙に浮いた volume 66 個」を検査が自分で増やし、5.1 の残りの検査には見えない）
- agent の環境に入った `COMPOSE_FILE` が、録画の木（`COMPOSE_PROJECT_NAME` を持つ `.env`）で compose を落とす（R15）
- `.env` の書き換え（env-init.sh:47 の `cat "$tmp" >"$root/.env"`）は切り詰めてから書くので、その間に止まると合言葉の行が消える。窓はごく短く、design D7 が inode と権限を保つために選んだ形。実際に止めて確かめてはいないので、指摘にはしない

### 手 7: 該当なし（確かめた範囲: `plan_fix.py pending` の出力）

`python3 scripts/plan_fix.py pending openspec/changes/ch06-reset-verify-db` → `[]`（rc=0）。

## R14. 本体を 1 回走らせるたびに、名前の無い volume が 4 つ（CI の形では 3 つ）残る。5.1 の「sentinel の残り」の検査はこれを見ない
- 成果物: tools/check-verify-env-reset.sh:40 / :115 / :136 / :138 / :218 / tasks.md 5.1 の 3 本目の検証
- 根拠: `docker volume ls -q -f dangling=true | wc -l` は始める前 103 → 本体 1 回 + 壊した複製 1 回の後 109。本体の回で増えた 4 つ（作られた時刻 11:55:15・16・17・39）は 4.（`$sp-vol`）・5.（sentinel の compose project・`$sp-ext`）・8.（`$sp-ctr`）の作られた時刻と合う。
  ほかの本体の回でも同じく 4 つ増え、CI に似せた clone では 3 つ（8. は SKIP）。どれも postgres の image の `VOLUME` が作る匿名 volume で、後片付け（:40 の `docker rm -f`、`-v` なし）が残す。
  名前が `ch06-sentinel-` で始まらないので、5.1 の `docker volume ls -q --filter name=ch06-sentinel-` は rc=0（空）のまま。手元で `verify-run 5.1` を回すたびに積もる（CI は job ごとに消えるので害は無い）
- kind: technical
- 提案: sentinel を `true` で作るなら postgres でなく `VOLUME` を持たない image を使うか、後片付けを `docker rm -f -v` と `docker compose -p "$sp" … down -v` にする（`-v` で DB の volume を名指しで付けた `$sp-vol` から DB の volume が消えないことを確かめてから）。5.1 の残りの検査は、走らせる前と後の dangling の数の差で見る
- 処置: fixed 5.1 —— tools/check-verify-env-reset.sh:41・:123・:238 の sentinel の後片付けを `docker rm -f -v`（名指しで付けた DB の volume は `-v` でも消えない）。design D8 の後片付けの注記も直した。本体の前後で dangling の volume の数が 103 → 103（本体 rc=0、74 秒）

## R15. agent の環境に渡った `COMPOSE_FILE` は、録画の木で compose を落とし、override のある木では override を黙って外す（D6 の「木によらず同じ形」が成り立たない）
- 成果物: tools/env-init.sh:15 / :65 / tools/agent-env.sh:47-50 / tools/record-env.sh:130・:200（`stack.sh up`）/ design.md D6
- 根拠: このセッションの環境に `COMPOSE_FILE=docker-compose.yml:docker-compose.subnet.yml` が入っている（`tools/agent-env.sh` が `.env` を `set -a` で読む）。
  一時の木に `docker-compose.yml`・`docker-compose.override.yml`・`COMPOSE_PROJECT_NAME=ashiato2rec99` の `.env` を置き（`docker-compose.subnet.yml` は無い。env-init.sh:15 は録画の `.env` に触らないので作らない）、
  同じ値の `COMPOSE_FILE` を環境に入れて `docker compose config` → rc=1・`stat …/docker-compose.subnet.yml: no such file or directory`。`env -u COMPOSE_FILE` では rc=0 で、override の port（55991）も入る。
  環境の値は `.env` に勝つので、`.env` に 3 つ並べた木（override あり）でも、2 つの値を持つ環境から呼ぶと override が外れる。agent が worktree から `tools/record-env.sh` を呼ぶと、録画の木の `tools/stack.sh up` がこの環境を継ぐ（グラフの `record-run` は `script()` で agent の環境を渡さないので、グラフの経路は当たらない）。
  hx の server（pid 461437）の環境と `systemctl --user show hx-dev -p Environment` に `COMPOSE_FILE` は 0 件。CH06 の「壊してはいけない」に `tools/record-env.sh` の振る舞いがある。
  D6 は（仮）で本人の答えではないので、崩れたのは既決の前提でなく仮の判断
- kind: technical
- 提案: `tools/agent-env.sh` の `.env` を読む段の後で `unset COMPOSE_FILE` する（compose は `.env` を自分で読むので、環境に写す要が無い。`--self-test` も既に外して呼んでいる）。
  `record-env.sh` の中で外す手もあるが、そちらは CH06 の「変えないもの」に当たる。design D6 の「木によらず同じ形なので古い値にならない」を直す
- 処置: fixed D6 仮 —— tools/agent-env.sh:53 で `.env` を読んだ後に `unset COMPOSE_FILE`（compose は project の `.env` を自分で読む）。`. tools/agent-env.sh` の後に `COMPOSE_FILE` が無く、合言葉は入ることを確かめた。design D6 の「木によらず同じ形なので古い値にならない」を直した

## R16. 本人の決定 Q2 の「出所」のうち、同じ出所の `×<数>`・10 種で切る・`…` は、書き換えても全部通る
- 成果物: tools/verify-env.sh:49 / tools/check-verify-env-reset.sh:143-156
- 根拠: verify-env.sh:49 の `++c <= 10` を `<= 1` に、`($2 > 1 ? " ×" $2 : "")` を `""` に、`END{ if (c > 10) printf ", …" }` を `END{}` にした複製で `tools/check-verify-env-reset.sh` rc=0（56 秒、`OK`）。
  本体が作る接続は `ch06-holder` の 1 本だけで、同じ出所が 2 本ある場合も 11 種ある場合も無い。design D3 の書式は spec ではなく design が持つ（spec が持つのは `切った接続: <数>` と 0 の 1 行）
- kind: technical
- 提案: 本体 5. に同じ `application_name` の接続を 2 本足して `ch06-holder ×2` と `切った接続: 2` を見るか、出所の並べ方を関数に切り出して `--self-test` で 11 種の入力を与える
- 処置: fixed 5.1 —— 本体 5. の接続を 12 本（同じ出所 2 本と別の出所 10 本）にし、tools/check-verify-env-reset.sh:160 で `×2`・10 種で切る・`…` を含む 1 行の全体を比べる。本体 rc=0

## R17. env-init の「範囲の外の port」の規則・`COMPOSE_FILE` の行が 2 つあるとき・subnet の 256 での繰り上がりと 512 の上限は、どの試験も固定していない
- 成果物: tools/env-init.sh:33-35 / :44 / tools/ports.sh（`ashiato_db_subnet`）/ tools/check-ports.sh:46-54 / tools/check-verify-env-reset.sh（`--self-test`）
- 根拠: env-init.sh:34 の `rm -f …subnet.yml` を `:` にした複製で `--self-test` rc=0（design D7 の 5. の後半「subnet が出なければファイルを作らず `COMPOSE_FILE` の行を消す」がどこからも走らない）。
  env-init.sh:44 の `!done` を外した複製（行が 2 つあれば 2 つとも書く）でも `--self-test` rc=0。
  `ashiato_db_subnet` の `off < 512` を外し、`200 + off / 256` と `off % 256` を `200` と `off` にした複製で `check-ports.sh --self-test` rc=0・`--self-test` rc=0。そのとき 55700 は `(200, 300)/24`、56000 は `(200, 600)/24`（第 3 の組が 255 を超える、IPv4 として成り立たない値）。
  比べる 7 つの木の off はどれも 256 未満で、台帳の番号 56 以上（10.201 の側）を通る木が無い
- kind: technical
- 提案: `--self-test` に `ASHIATO_DB_PORT` を範囲の外にした木（subnet のファイルが消え `COMPOSE_FILE` の行が無くなる）と、`COMPOSE_FILE` の行を 2 つ持つ `.env` を足す。`check-ports.sh --self-test` に port 55656 と 55911・55912 の値（`printf` で組んだ期待値）を足す
- 処置: fixed 1.2 / 2.2 —— tools/check-ports.sh:56-62 に off = 0・255・256・511・512・負の値の subnet（期待値は `printf` で組む）、`--self-test`（tools/check-verify-env-reset.sh:339-350）に `COMPOSE_FILE` の行が 2 つある `.env` と範囲の外の port の木を足した。`check-ports.sh --self-test` rc=0・`--self-test` rc=0

## R18. 4.2 の「禁じた旗」の grep は `rm -s -f -v`・`rm --volumes`・`down --volumes` を見逃す
- 成果物: tasks.md 4.2 の 2 本目の検証 / tools/verify-env.sh:69
- 根拠: verify-env.sh:69 を `docker compose rm -s -f -v db`・`docker compose rm --volumes -s -f db`・`docker compose down --volumes` にした 3 つの複製で、
  `bash -c '! grep -vE "^\s*#" tools/verify-env.sh | grep -nE "down -v|up .*-V|force-recreate|rm -[a-z]*v"'` は 3 つとも rc=0（`rm -sfv` だけ rc=1）。
  振る舞いの側は本体が見ていて、`rm -s -f -v` の複製は本体 rc=1（`NG [1. 空にする]`）。grep は 2 番目の網で、それ自体が穴を持つ
- kind: technical
- 提案: grep を `-v\b|--volumes|-V\b|--renew-anon-volumes|--force-recreate` を `docker compose (rm|down|up)` の行で探す形にするか、振る舞いは本体が持つとして grep の検証を外す
- 処置: fixed 4.2 —— tasks.md 4.2 の grep は今のコードで PASS するので plan_fix（FAIL の実証が要る）の対象でなく、`--self-test` の末尾（tools/check-verify-env-reset.sh:388-393）に、compose の rm / down / up の行の `-v` を含む旗の並び・`--volumes`・`--renew-anon-volumes`・`--force-recreate` を見る検査を足した。verify-env.sh:69 を `rm -s -f -v`・`rm --volumes -s -f`・`down --volumes`・`rm -sfv` にした 4 つの複製で `--self-test` rc=1（どれも名指しの NG）、戻して rc=0

## security review（2b924da..cbc36bb）

見た経路:
- 入口: この差分は API・画面・コマンドの入口を足さない。足したのは開発の道具（`tools/verify-env.sh` `tools/env-init.sh` `tools/ports.sh` の `ashiato_db_subnet`・`tools/check-verify-env-reset.sh`）と CI の 2 行だけ（`git diff --first-parent --stat`）（3）
- 待ち受け: subnet のファイルを重ねても DB の公開は `127.0.0.1` のまま（`docker-compose.yml:9`。この木で `docker compose config` → `host_ip: 127.0.0.1`）。新しい listen・外への送信先は無い（4）
- 消す操作: `HARNESS_VERIFY=1` → `plan()`（`tools/verify-env.sh:15-24`）→ `reset_db` は `docker compose ps -aq db` で引いた自分の project の DB のコンテナと、そのコンテナが mount した volume だけを消す。ほかのコンテナが同じ volume を使っていれば何も消さずに `exit 1`（`verify-env.sh:58-73`）（3）
- `.env`（合言葉）の書き換え: `mktemp "$root/.env.XXXXXX"`（0600）→ awk で `COMPOSE_FILE` の行だけ差し替え → `cat >"$root/.env"` で inode と権限を保つ。一時ファイルは `trap … EXIT` で消え、残っても `.gitignore:12` の `.env.*` に当たる（`git check-ignore -v .env.Ab12Cd` で確認）（4）
- 生成物 `docker-compose.subnet.yml` は `.gitignore:58` に当たる（`git check-ignore -v`）。合言葉は含まない（4）
- 出力: `cut_line` が出すのは `application_name@client_addr` と数だけで、記録の中身（URL・題名・位置・本文）は出さない。問い合わせは `pg_stat_activity` だけ（`verify-env.sh:39-50`）（3）
- 検査台本: `as_role` は合言葉を `docker compose exec -e PGPASSWORD=…` で渡し、`want` が出すのは問い合わせの結果（`ashiato_owner` など）だけ。合言葉そのものを標準出力にも NG の行にも出さない（`tools/check-verify-env-reset.sh:77-83,172-175`）。CI では合言葉が環境に無く、env-init が乱数で作る（`ci.yml` に `PASSWORD` の行は無い）（3）
- 追跡するファイルの私的な値: `evidence.jsonl` と `.superpowers/sdd/ch06-final-fix-report.md` には合言葉・接続 URL・ホームのパスが無い（grep）。`review/code.md` の未コミットの code_verify 節は `tools/check-private.sh` に当たる（→ R19）（4）

安全の決め手: **消す判定は main の checkout では `keep:` を返し（`CI=true` のときだけ例外）、worktree で印があるときだけ `reset` を返す。reset でも消すのは自分の compose project の DB のコンテナとその volume に限る** —— 確かさ 4。
一時の repo（main と `git worktree add` した木）に `verify-env.sh` を写して `--plan` を走らせた（docker に触らない）:
`m [] → keep: 印が無い` / `m [HARNESS_VERIFY=1] → keep: main の checkout（開発用の起動の DB）は作り直さない` / `m [HARNESS_VERIFY=1 CI=true] → reset` / `m [HARNESS_VERIFY=1 CI=1] → keep` / `w [] → keep: 印が無い` / `w [HARNESS_VERIFY=1] → reset`。
`CI=true` を手元の main の checkout で印と一緒に渡すと開発用の DB を消す（D2 で決めた形）。ただし、ハーネスが手元で `CI` を立てる経路は見つからなかった（`.harness2/scripts` を grep）ので、指摘にはしない。volume を共有するコンテナがあると止まることは、本体の段 4（`check-verify-env-reset.sh:105-118`）が確かめる形で、この review では走らせていない（3）。

## R19. 未コミットの code_verify 節が点区切りの 4 つ組を持ち、`tools/check-private.sh` が落ちる（このまま commit すると pre-commit が止め、通しても CI の `chain` が落ちる）
- 成果物: openspec/changes/ch06-reset-verify-db/review/code.md
- 根拠: `./tools/check-private.sh` → rc=1。`NG …/review/code.md:139`・`:140`・`:162`・`:234`（どれも `10.200.<n>.0/24` の形の subnet をそのまま書いた行）。`git status` でこの節は未コミット（M）。値は port の規則から決まる docker の subnet で、手元の網の実値ではないので外には何も漏れない。ただし、台本の側は「追跡するファイルに点区切りの 4 つ組を書かない」（`tools/ports.sh` の注記・`check-ports.sh` は `printf` で組む）を守っている。その規則をレビューの文書だけが破っている。この R19 自身も値を伏せて書いた（確かさ 4）
- kind: daily
- 提案: 該当の 4 行を `printf '10.%d.%d.0/24' 200 32` の形か「off=32 の /24」のような書き方にして、`tools/check-private.sh` が rc=0 になってから commit する。`HARNESS_ALLOW_PRIVATE_NAMES=1` では通さない
- 処置: fixed D12 仮 —— code_verify 節と /code-review 節の subnet を `(第 2 組, 第 3 組)/接頭長` の形に書き直し、evidence.jsonl の FAIL の記録 2 件の summary（check-private の NG の行の写し）も同じ形にした。`tools/check-private.sh` rc=0（verify-run 2.2 / 7.3 PASS）

## /code-review（2b924da..cbc36bb）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R20. clear_blocking_network refuses to proceed (exit 1) whenever a leftover network with the same subnet still has a stopped container attached. Because tools/ports.sh hands the slot number of a deleted worktree to the next one, a deleted worktree's stale compose network now blocks its successor for good.
- 成果物: tools/verify-env.sh:88
- 根拠: Worktree ashiato2-ch05 gets slot 3 (port 55603, subnet (200, 203)/24). It is removed with `git worktree remove` without `docker compose down`, so its stopped db container stays attached to `ashiato2-ch05_default`. A new `-ch09` worktree reuses slot 3. Its first verify-env sees its own network missing, finds ch05's network on the same subnet, `docker ps -a --filter network=` returns the stopped container, and it exits 1. Every later verification in ch09 is BLOCKED_INFRA until someone cleans up by hand. Before this change the stopped container held no port and ch09 just worked.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D5 —— 止まるのは変えない（本人の答え Q3「消えた worktree の残りは片付けない」。止まったコンテナにも volume が付いている）。tools/verify-env.sh:107-114 で、繋がったコンテナの compose の `working_dir` の印が無いディレクトリを指すなら「消えた worktree のもの」と出し、外し方（`docker rm -f … && docker network rm …`。volume は残る）を添える。本体 8. の段で印つきの sentinel のコンテナを繋ぎ、rc≠0・その文言が出る・network とコンテナが同じ id を確かめた（本体 rc=0）

## R21. env-init now always rewrites an existing `.env` with COMPOSE_FILE and creates docker-compose.subnet.yml, including in the main checkout (verify-env calls env-init before checking `keep`). The main dev DB's network was created from the default pool, so its config no longer matches.
- 成果物: tools/env-init.sh:38
- 根拠: Someone runs tools/verify-env.sh in the main checkout (plan says `keep`, but env-init still runs). Main's `.env` gains `COMPOSE_FILE=...subnet.yml` with (200, 32)/24. The next `docker compose up` from tools/stack.sh or dev.sh finds the existing `ashiato2_default` network with a different IPAM config. Compose either recreates the network, which disconnects or stops the running dev DB, or fails with a network-config error. design D7 lists this as a reversal condition but the code does not guard against it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: main の checkout でも subnet を書くのは design D5 / D7 の決めた形（`check-ports.sh` が main の off = 32 の値を固定）。既にある network の IPAM が変わった後の最初の `up` は network を作り直し、DB のコンテナは作り直されずに繋ぎ直して中身が残ることを design の P12 で実測している（「落ちる」側は起きない）。開発用の DB が繋ぎ直しの間だけ切れるのは、この 1 回きり

## R22. Writing COMPOSE_FILE into `.env` turns off compose's automatic loading of docker-compose.override.yml and makes every compose call depend on untracked files existing exactly as they did when env-init last ran.
- 成果物: tools/env-init.sh:23
- 根拠: If docker-compose.override.yml or docker-compose.subnet.yml is deleted after env-init (manual removal, `git clean -X` of ignored files), every `docker compose` call in smoke.sh, seed.sh, db.sh, stack.sh and dev.sh fails with 'no such file' until verify-env runs again. Those scripts never call env-init. In the other direction, an override added after env-init ran is silently ignored by those scripts, where before this change compose picked it up automatically.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: `COMPOSE_FILE` を置くと override を自動で読まなくなり、生成物が消えると次の env-init まで compose が落ちることは、design D7 が受け入れた代償として書いている（override のある木は並べる・消えたら次の verify-env / env-init が作り直す）。env-init の後に手で置いた override が次の env-init まで読まれない場合も design D6 に書き足した。反転条件は D6 の末尾（compose が `.env` の `COMPOSE_FILE` を読まなくなったら呼び出し口に `ports.sh` を足す）

## R23. The check for a blocking network only matches an identical subnet string (`grep -qx`). An existing network whose range merely overlaps the /24 is not detected, and `up` fails with no explanation.
- 成果物: tools/verify-env.sh:86
- 根拠: Another tool or a VPN has a docker network at (200, 0)/16 or (200, 200)/23. clear_blocking_network finds no exact match, removes nothing and reports nothing. `docker compose up` then fails with 'Pool overlaps with other one on this address space', and the user gets no hint about which network is in the way.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D5 —— tools/verify-env.sh:78-85 の `cidr_overlap` で、同じでなく重なるだけの範囲の network を見つけたら、消さずに network と範囲を出して rc=1。本体 8. で subnet を含む /23 の sentinel の network を作り、rc≠0・名前が出る・同じ id を確かめた（本体 rc=0）

## R24. The self-test passes ASHIATO_DB_SUBNET to prove that a stale environment value cannot override the subnet, but no code reads ASHIATO_DB_SUBNET, so the case is vacuous and gives false assurance.
- 成果物: tools/check-verify-env-reset.sh:303
- 根拠: Suppose a future change makes the subnet come from an inherited ASHIATO_DB_PORT or a COMPOSE_FILE environment value. This 'stale value' scenario still passes, because the variable it injects has no effect anywhere. The `-u ASHIATO_DB_SUBNET` in clean() (line 269) is dead for the same reason.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 空ではない。この場合は、subnet を環境変数から取る形（review/spec.md R5 で採らなかった形）に戻す変更を止める網。tools/env-init.sh:19 を `subnet="${ASHIATO_DB_SUBNET:-$(ashiato_db_subnet)}"` にした複製で `--self-test` rc=1（`subnet のファイルの subnet はその木の値` が NG。demo-ch01 の値になる）、戻して rc=0

## R25. The compose network name is pulled out by awk-scraping YAML from `docker compose config` (the first indented `name:` after `^networks:`), and the same fragile pattern is copied in check-verify-env-reset.sh.
- 成果物: tools/verify-env.sh:81
- 根拠: If compose changes its key order, or another top-level network or a networks-level key with `name:` appears first, `own` names the wrong network. clear_blocking_network then skips its check or misreads its own network, and the check script compares the wrong id. Reading `docker compose config --format json` (already used in check-exposure.sh and the self-test) would be robust.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D5 —— tools/verify-env.sh:90 と tools/check-verify-env-reset.sh:69 の network の名前を `docker compose config --format json` の `networks.default.name` から取る。本体 1. の「DB の network の名前が compose config から出る」と 8. が通る（本体 rc=0）

## R26. The rule for telling a worktree from the main checkout (git-dir vs git-common-dir) is now written out a third time, in verify-env.sh plan(), check-verify-env-reset.sh body() and tools/ports.sh, instead of being shared.
- 成果物: tools/verify-env.sh:15
- 根拠: If ports.sh's rule changes (for example to handle non-git trees or nested checkouts), plan() and body() keep the old rule. The DB could then be reset in a tree that ports.sh treats as main (port 55432, the dev DB), or kept where it should be reset. ports.sh could export its result (for example `_ports_is_worktree`) or provide a helper function, as it already does with ashiato_db_url and ashiato_db_subnet.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: `plan()` は `--plan` で docker にも `.env` にも触らない約束で（tools/verify-env.sh:4）、`tools/ports.sh` は読み込むだけで worktree の台帳を書く（flock と `mv`）ので、plan から読めない。見分け方の結果は `--self-test` の Scenario（main・worktree・`-verify`・`-st05` の木で plan の判定）が ports.sh の木の型と合わせて固定している。共有の helper に切り出すのは CH06 の範囲の外

## R27. Because of `set -E`, the ERR trap is inherited by functions and command substitutions, so expected failures inside `$(rows_after_reset)` and `$(as_role ... x)` print spurious 'NG [stage]: N 行目のコマンドが失敗した' lines even though the check passes.
- 成果物: tools/check-verify-env-reset.sh:9
- 根拠: Stage 5 deliberately runs `as_role ashiato_owner wrong-password`, and the post-reset `to_regclass` queries can fail. Each failing psql inside a substitution can fire the ERR trap and print an NG line to stderr. The output then shows NG lines next to a final OK, which misleads anyone reading the CI log.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 再現しない。`$(…)` の中の失敗で ERR trap が鳴る筋は bash で確かめたが、この台本の該当の呼び出しは `2>/dev/null || true` の中（`||` の左では ERR trap は鳴らない）か、失敗しない問い合わせ（`to_regclass` は表が無くても null を返す）。本体の通しのログ（.superpowers/sdd/logs/20261025-115506-cv-body.log、73 行）と今回の本体の出力に `NG` は 0 件で `OK`

## R28. `cat tmp > .env` truncates `.env` and then rewrites it in place, so a concurrent reader (compose or db-roles.sh in another process of the same tree) can read an empty or partial `.env`.
- 成果物: tools/env-init.sh:50
- 根拠: The harness runs verify-env (and so env-init) while another command in the same worktree, such as a seed or smoke step, starts `docker compose`. Compose reads `.env` during the truncate window, POSTGRES_PASSWORD is missing, and `${POSTGRES_PASSWORD:?}` aborts that compose call.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: design D7 が権限と inode を保つために選んだ形で、`--self-test` が inode を固定している（code_verify の手 2 で `mv` にした複製が落ちる）。書き換えは `cmp -s` で中身が違うときだけ（木ごとに `COMPOSE_FILE` が入る最初の 1 回と override の出入り）で、ハーネスは verify-env を検証コマンドの前に順に呼ぶので、同じ木で compose が並んで `.env` を読む経路は無い

## R29. clear_blocking_network runs a separate `docker network inspect` for every network on the host, one after another, instead of a single `docker network inspect $(docker network ls -q)`.
- 成果物: tools/verify-env.sh:77
- 根拠: On a host with dozens of leftover compose networks (the network-pool exhaustion this change is meant to address), every verify-env in a tree whose own network is missing, including right after smoke.sh's `down -v`, spends N sequential docker CLI round-trips before `up`.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D5 —— tools/verify-env.sh:118 で `docker network ls -q | xargs -r docker network inspect` の 1 回で全部の network の名前と範囲を引く。本体 8. が通る（本体 rc=0）
