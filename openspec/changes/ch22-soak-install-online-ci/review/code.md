<!-- 処置の判定: re-review -->

# CH22 code-verify（`ch22-soak-install-online-ci`）

確かめた木: `feat/ch22-soak-install-online-ci` @ `63a170e4e`（未 push・PR 無し。`gh pr list --head feat/ch22-soak-install-online-ci` → `[]`）。
手元は Linux（WSL）。実地の口（Windows の runner）は手元では走らない。実物の形の入力を `soak-install.sh --evaluate` に渡すか、複製の台本を壊して自己試験を走らせて確かめた。
探りの置き場は `/tmp/ch22probe`（作業ツリーの外）。終わった後の `git status --porcelain` は空のまま。

## 申告と実測

申告: 5/5 の `[x]`（1.1・2.1・2.2・3.1・4.1）。検証コマンドの一覧と実測:

| Task | 検証コマンド | 実測 |
|---|---|---|
| 1.1 | `tools/release/soak-install.sh --self-test` | 一致（rc=0）。ただし揺れる: 連続 20 回のうち 1 回 rc=1（R8） |
| 1.1 | `tools/release/check-ng-observations.sh tools/release/soak-install.sh` | 一致（rc=0） |
| 1.1 | `# Scenario:` が 20 以上・`SOAK_SAME_ERROR_LIMIT=3` | 一致（rc=0。印は 30） |
| 2.1 | `--list` が 2 行 | 一致（rc=0） |
| 2.2 | actionlint（`-ignore SC2155`） | 一致（rc=0。手元に `actionlint` が無く、docker の `rhysd/actionlint:1.7.7` で通った） |
| 2.2 | `grep`（`--self-test`・`--list`・`--block-outbound`） | 一致（rc=0） |
| 2.2 | 終了の条件「PR の CI の `install-windows` が置く段を含めて緑」 | **不一致（まだ走っていない）**: push も PR も無い（R9） |
| 3.1 | `grep`（`docs/testing.md`） | 一致（rc=0） |
| 4.1 | `python3 scripts/check_scenarios.py . ch22-soak-install-online-ci` | 一致（rc=0、`scenarios: OK`） |
| 4.1 | `install-check.sh` / `update-check-install.sh` / `update-apply-install.sh --self-test` | 一致（3 つとも rc=0） |
| 4.1 | 予定のパスの外に触れていない（`git diff --name-only origin/main...HEAD`） | 一致（rc=0） |

CI の配線（`ci.yml` の差分）: `install-windows` の置く段は `shell: bash`（`-eo pipefail` なので `| tee` が rc を隠さない）、`timeout-minutes: 15`・job に `100`、`--list` と `comm -23` の突き合わせあり、`rust` の job に `--self-test` あり。差分を読んで確かめた。

## 手 1: 固定値を独立に再計算する

期待値を直書きしたものは 1 件: 自己試験は偽の `wevtutil` の `Date: 11/20/2026 12:00:11 AM` が `2026-11-20T00:00:11.000Z` になることを見ている（`soak-install.sh:412`）。
python の `datetime.strptime('11/20/2026 12:00:11 AM','%m/%d/%Y %I:%M:%S %p')` で独立に計算して `2026-11-20T00:00:11.000Z`。一致した。
ISO の形（`2026-11-20T00:00:11.1230000Z`・`2026-11-20T00:00:11.123`）も GNU `date -u -d` で同じ値になる。ほかの直書きは閾値の定数だけ（手 4 で見る）。

## 手 2: ガードをわざと壊す

- 複製の `ci.yml` で、置く段の手順を `--block-outbound` のすぐ後（インストールの試験より前）へ動かした → `--self-test` rc=1。この守りは効いている
- 複製の `ci.yml` で、`rust` の job に `- run: ./tools/release/soak-install.sh dist/ashiato-server.zip dist/ashiato-collector-windows.zip` を足した → `--self-test` **rc=0**（R7）
- 複製の `ci.yml` で、`shell: pwsh` の `Remove-NetFirewallRule -Name ashiato-update-check-ci-outbound` の手順を足した → `--self-test` **rc=0**（R7）
- 複製の台本で、偽の `wevtutil.exe` を `exit 1` にした → 実地の口は通り、出力は `system-events: readable count=0` で `::warning::` が出ない（R3）
- 複製の台本で、偽のインストーラに、更新係が実際に書く置き場 `$SOAK_UPDATER_DIR/logs/updater.log` へ同じ `ERROR same-updater` を 3 行書かせた → 実地の口は **rc=0**、`error[` の行は 0、集めた `logs/updater.log` は 0 バイト。
  対照に、台本が読む `$SOAK_UPDATER_DIR/updater.log` へ同じ 3 行を書かせると rc=1（`NG: updater.log の同じ誤りが 3 回`）。台本はどのログを見つけて何行読んだかを出さないので、置き場の違いは「誤りの無いログ」と見分けがつかない（R2）
- 複製の台本で、誤りの行を数える窓（`soak-install.sh:88`）と更新係・PC の収集アプリの `ERROR` の絞り（`:98`）を両方消した → `--self-test` **rc=0**（R5）

## 手 3: Scenario と test を突き合わせる

`check_scenarios.py` は rc=0（印は全部ある）。印の先が主張を観測しているかを 1 件ずつ見て、ずれていたもの:

- 「起こし直しより前の確かめた行だけなら、PC の収集アプリの確かめは無いとみなす」: spec の THEN は「判定は 0 でない終了コード」。自己試験は `expect_pass`（`:597`）。**逆**（R1）
- 「システムの記録を読めなければ、警告を出し…」: spec の GIVEN は「終了コード 1 で何も返さない偽のシステムの記録の命令」。自己試験はファイルの `events` を消しているだけ（`:566`）。実地の口では命令が落ちても空のファイルができるので、この分岐に来ない（R3）。AND（同じ命令のまま `ashiato-db` の番号を変えると落ちる）も見ていない
- 「警告の段の行と置く段の始まりより前の行は数えない」「置く段の始まりと置く間の始まりの間に書かれた誤りの行は数えない」: spec は 5 行。自己試験は 1 行だけ（`:574`・`:576`）。1 行では除外が無くても閾値の 3 に届かない（R5）
- 層は同じだが見る場所がずれているもの（指摘には入れない。R5 の提案で一緒に直せる）: 「1 回でも起こし直されたら」（spec は `ashiato-updater` と回数 1。自己試験は `ashiato-server` で、回数を見ない）、「2 回の同じ誤り」（spec はサーバのログで、ログの名前を出す。自己試験は更新係のログで、出力を見ない）、「届かない・読めない答えの行だけなら」（spec は 2 行。自己試験は `unreadable` の 1 行）、「確かめた行があれば通る」（spec は出力に 2 行。自己試験は出力を見ない）
- 実地の 2 つの `== Scenario:` は判定の前に echo するだけで、CI の `comm -23` は判定より前に出た印を数える。ただし判定の `die` で台本は rc≠0 になり、`pipefail` で手順が落ちるので、印だけ出て緑になる道は無い（確かめた範囲: `ci.yml` の置く段の手順と `soak_run` の終わり）

## 手 4: 本人の決定が test で固定されているか

| 本人の答え | 定数・分岐 | 書き換えて落ちるか |
|---|---|---|
| Q2 1 回でも起こし直されたら落とす | `(( stopped + changed + event_count == 0 ))` | `<= 1` にした複製 → rc=1。固定されている |
| Q3 同じ誤り 3 回で落とす | `SOAK_SAME_ERROR_LIMIT=3`・`count < LIMIT` | 自己試験に `== 3` の assert があり、`<=` にした複製 → rc=1。固定されている |
| Q4 確かめた行が 1 行は要る・上限だけ警告 | `soak_check_records` | 更新係は固定されている。**PC の収集アプリは確かめた行が無くても通る**（R1） |
| Q5 消す → 外す → 入れる | 命令の記録の順 | 自己試験が順を見る |
| Q7 段を緩めない（design D12: 外れを許す一覧を持たない・答えの形を見本と比べない・PC の収集アプリに送り先を設定しない） | 台本を読む | 実地の口が読む環境変数は `reset_wait_seconds`（`:185`）・`SOAK_SETUP_TIMEOUT`（`:227`）・`NG_OBSERVED_DIR`/`TMPDIR`（`:13`）・`LOCALAPPDATA`（`:298-301`）・`SOAK_RELEASE_PASSWORD`/`SOAK_TEST_PASSWORD`（`:25`）だけで、どれも外れを許す一覧ではない（頭の `SOAK_*` の定数は環境から上書きできない形で置かれる）。確かめた記録は `ok` と `reason` だけを読み、`latest` や答えの形を比べない。PC の収集アプリは引数なしで起こす（`:313`・`:320`）。**ただし R1 の `collector` → `return 0` は Q7 の「緩めない」に反する** |
| Q1 1 回起こし直す | `taskkill` | 自己試験は `taskkill` が呼ばれたことだけを見る |
| 依頼「5 分ほど」 | `SOAK_WAIT_SECONDS=300` | `30` にした複製 → 4 回とも rc=0。**値を変えても全部通る**（R10） |

本人の答えでない閾値（`SOAK_COLLECTOR_RESTART_SECONDS=60`・`SOAK_READ_INTERVAL_SECONDS=5`・`soak_wait_healthz 150`）も、`--self-test-run`（`:617-620`）が上書きするので、どの値でも自己試験は通る。D11 で仮の定数と決めているので、指摘には入れない。`timeout-minutes: 15`・`100` は CI の定義の検査が 1 つ減らした写しで落とす（`must_fail`）。

## 手 5: tasks の `[x]` と実体

コマンドは全部実在して、rc=0（上の表）。Task 2.2 の「終了の条件」は満たしていない（R9）。`cargo test --test` の型の検証は無い。

## 手 6: 隙間

最優先で探したのは、0.1.0 型の壊れ方が「数えずに緑」になる道。実物のログ・記録の形を製品のコードから取って、`--evaluate` に渡した:

- 更新係のログの置き場: 台本は `ashiato-updater\updater.log` を読み、更新係は `ashiato-updater\logs\updater.log` に書く。実地の口で確かめて、読まれていない（R2）
- 更新係の `updater.log` の実際の形（`crates/updater/src/lib.rs:319` の `Utc::now().to_rfc3339()` は `+00:00` で終わる。chrono 0.4.45 の `to_rfc3339` は `use_z=false`）で同じ誤りを 5 行 → rc=0、行が数えられない（R2）
- PC の収集アプリの確かめた記録が空 → rc=0（R1）
- PC の収集アプリが、台本の起こし直しより前（最初の 60 秒）に終わる → rc=0（R4）
- `soak-install.sh` の固定の合言葉が誤りの行に出る → 伏せずに出る（R6）
- サーバ（`crates/server/src/service.rs:695-699` の tracing の fmt。`…Z ERROR …`）・PC の収集アプリ（`crates/collector-windows/src/main.rs:106-110`）・確かめた記録（serde の `DateTime<Utc>` は `Z`、`reason` は snake_case で `rate_limited`）は、台本の読み方と合っている
- DB のログ（`log_line_prefix` は既定の `%m [%p] `、`log_timezone` を installer が決めていない）・`wevtutil /f:text` の本物の Date の形は、Windows で走らせないと確かめられない。premises P8・P11 の範囲で、最初の CI の走りの観測に任せる（指摘には入れない）

## 手 7: 該当なし（`python3 scripts/plan_fix.py pending openspec/changes/ch22-soak-install-online-ci` → `[]`）

---

## R1. PC の収集アプリが置く間に一度も確かめなくても、置く段は通る（spec は落とす）
- 成果物: tools/release/soak-install.sh / openspec/changes/ch22-soak-install-online-ci/specs/distribution/spec.md
- 根拠: `soak_check_records` は、label が `collector` で起こし直しより後の行が 0 なら `soak_out "check[collector] no post-restart record"; return 0`（`soak-install.sh:126-129`）。
  空の `update-checks.jsonl` を入れた `--evaluate` → rc=0（出力 `check[collector] no post-restart record`）。ファイルが無いときだけ rc=1。
  自己試験の Scenario「起こし直しより前の確かめた行だけなら、PC の収集アプリの確かめは無いとみなす」は `expect_pass`（`:597`）だが、spec の THEN は「判定は 0 でない終了コードで終わり、標準エラーに PC の収集アプリの確かめた行が無いことがある」。
  Requirement「…確かめた記録が無ければ、置く段を落とす」と、実地の Scenario「配る形の PC の収集アプリは…本物の配布元に確かめ」の判定がどちらも働かない。
  深掘り Q7・design D12 の「段の判定を緩めない」にも反する（落とす条件を黙って外している）。D7 の反転（runner で通知を登録できない）を先取りしたようにも読めるが、design・ledger に `Ruling:` は無い（`git log -S'no post-restart record'` は `0ce817585` だけ）
- kind: technical
- 提案: 起こし直しより後の行が 0 なら `die` にして、自己試験を `expect_fail`（needle は PC の収集アプリ）に直す。runner で登録できないと分かったら、D7・D15 のとおり spec ごと直す

## R2. 置く段は更新係のログを読んでいない（置き場が違う）。読めても、本物の形では同じ誤りを数えない
- 成果物: tools/release/soak-install.sh
- 根拠: **置き場**: 更新係は `dir.join("logs/updater.log")`（`crates/updater/src/lib.rs:318`。`dir` は `default_dir` = `C:\ProgramData\ashiato-updater`、`crates/update-check/src/lib.rs:53`）に書き、design D6 も `…\ashiato-updater\logs\updater.log`。
  台本は `path="$SOAK_UPDATER_DIR/updater.log"`（`soak-install.sh:259`）を読み、無ければ `cat "$path"/*` が落ちて空のファイルになる。
  実測: 偽のインストーラに `$SOAK_UPDATER_DIR/logs/updater.log` へ同じ 3 行を書かせた実地の口 → rc=0、集めた `logs/updater.log` は 0 バイト。台本が読む置き場へ書かせた対照 → rc=1。
  0.1.0 型（更新係が落ちるたびに同じ行を書く）は、誤りの行の判定からは見えない（起こし直しの判定だけが残る）。どのログを何行読んだかを出さないので、緑の回のログからも気づけない。
  **形**: 更新係は `"{} {error}"`、時刻は `Utc::now().to_rfc3339()`（`crates/updater/src/lib.rs:319`。chrono 0.4.45 では `+00:00` で終わり、`Z` にならない）で、`ERROR` という語を書かない。
  台本は (a) 時刻を `…Z` の形でしか読まない（`:86`）、(b) 更新係の行を `*ERROR*` に絞る（`:97-98`。spec は「更新係の誤りのログは全部の行」）、(c) 同じ誤りのキーから `+00:00` の時刻を外さない、(d) 時刻の読めない行の基準の行数を、実地ではいつも `0` と書く（`:264`。置く間の始まりのログの大きさを取らない）。
  実測: `2026-11-20T00:01:0N.123456789+00:00 配布元の答えを読めない: 接続を拒否された` を 5 行入れた `--evaluate` → rc=0（`error[updater.log]` の行も出ない）。`ERROR` を足した 3 行でも rc=0（`count=1` が 3 行。時刻がキーに残る）。
  自己試験の更新係のログの行は全部 `…Z ERROR same`（`:571-580`）で、製品が書かない形
- kind: technical
- 提案: 更新係のログを `$SOAK_UPDATER_DIR/logs/updater.log` から読み、ログごとに見つけたか・読んだ行数を落ちない回にも 1 行出す。更新係の時刻の形（`+00:00`）を読み、キーから外す。更新係は全部の行を数える。基準の行数は置く間の始まりに取る。自己試験の入力を `crates/updater` の書く形に合わせる

## R3. システムの記録を読めなくても「読めた・0 件」と出て、警告が出ない
- 成果物: tools/release/soak-install.sh
- 根拠: `wevtutil.exe qe System /f:text 2>/dev/null | awk … | while … done >"$dir/events"`（`:266-290`）は、命令が落ちても空の `events` を作る。判定は `[[ -r "$events" ]]` で「読めた」とみなす（`:52-53`）。
  複製の台本で偽の `wevtutil.exe` を `exit 1` にした実地の口 → rc=0、出力は `system-events: readable count=0`、`::warning::` は無い、`events` は 0 バイト。
  spec の Scenario「システムの記録を読めなければ…」の GIVEN は「終了コード 1 で何も返さない偽の命令」だが、自己試験は `events` を消しているだけ（`:566`）。P11 の観測（読めたか）が CI のログで嘘になる
- kind: technical
- 提案: `wevtutil` の rc（と 1 件も読めないこと）で読めたかを決め、読めないときは `events` を作らない。自己試験は実地の口で偽の `wevtutil` を `exit 1` にして警告の行を見る。spec の AND（`ashiato-db` の番号を変えると落ちる）も足す

## R4. PC の収集アプリが、台本の起こし直しより前に終わっても落ちない
- 成果物: tools/release/soak-install.sh
- 根拠: `soak_collector_pid` は `[[ "$ts" > "$restart" && "$ts" < "$end" ]] || continue`（`:145`）で、起こし直しより前の読みを全部捨てる。spec は「置く間に、台本が止めた 1 回のほかに…終わる」なら落とす。
  起こし直しより前に PID が 100 → 空 → 101 と変わる読みを入れた `--evaluate` → rc=0（`collector pid changes=0`）。実地では最初の 60 秒（`SOAK_COLLECTOR_RESTART_SECONDS`）が見られない。1 回目の起動（案内を出す）で落ちる不具合は、台本の起こし直しで隠れる
- kind: technical
- 提案: 置く間の始まりから起こし直しまでの読みも数え、台本の `taskkill` の前後の 1 回だけを除く。自己試験に「起こし直しより前に消える」入力を足す

## R5. 「数えない」の 2 つの Scenario は 1 行だけで試していて、除外を消しても緑のまま
- 成果物: tools/release/soak-install.sh
- 根拠: 「警告の段の行と置く段の始まりより前の行は数えない」は `WARN` の 1 行（`:574`）、「置く段の始まりと置く間の始まりの間に書かれた誤りの行は数えない」は `ERROR` の 1 行（`:576`）。spec はどちらも 5 行（PC の収集アプリのログの警告、DB のログの `FATAL`）。
  複製の台本で、窓の比べ（`:88`）と `ERROR` の絞り（`:98`）を両方 `:` にした → `--self-test` rc=0（`OK: soak-install self-test`）。tasks 1.1 の「1 か所だけ直した入力では通ることを並べて見る」の逆（除外が効かなければ落ちる側）も無い。spec の AND（同じ 5 行を置く間へ移すと落ちる）も無い
- kind: technical
- 提案: spec のとおり 5 行を、spec の名指しのログ（PC の収集アプリ・DB）に置き、置く間へ移した写しで `expect_fail` を並べる。手 3 の「見る場所のずれ」4 件も、spec の GIVEN・THEN に合わせる

## R6. 台本の合言葉を伏せる値に入れていない
- 成果物: tools/release/soak-install.sh
- 根拠: `ng_secret_values` は `SOAK_RELEASE_PASSWORD`・`SOAK_TEST_PASSWORD`（どこでも決めていない）を出し、インストーラに渡す `SOAK_INSTALL_PASSWORD='soak-install-password-0123'`（`:16`・`:226`）を出さない（`:22-26`）。
  `ERROR login failed password=soak-install-password-0123` をサーバのログに入れた `--evaluate` → 出力は `error[server.log] ERROR login failed password=soak-install-password-0123 (count=1)`（伏せていない）。
  spec の Scenario「置く段の出力に秘密の値が出ない」は「画面の合言葉」も挙げるが、自己試験の入力（`:599`）に合言葉が無い。値は repo に書いてある固定の字面なので漏れの害は小さいが、spec の主張は満たしていない
- kind: technical
- 提案: `ng_secret_values` で `SOAK_INSTALL_PASSWORD` を出し、自己試験の誤りの行に合言葉を混ぜる

## R7. 「塞ぎを外す手順は置く段の 1 つだけ」の検査が、別の書き方の 2 つ目を見逃す
- 成果物: tools/release/soak-install.sh（`soak_ci_definition_self_test`）
- 根拠: 置く段の台本を走らせる手順を `'soak-install.sh package/'` の字面で数え（`:499`・`:518`）、規則を消す命令を `'delete rule'` で数える。
  複製の `ci.yml` で、`rust` の job に `./tools/release/soak-install.sh dist/ashiato-server.zip dist/ashiato-collector-windows.zip` を足した → `--self-test` rc=0。
  `shell: pwsh` の `Remove-NetFirewallRule -Name ashiato-update-check-ci-outbound` を足した → rc=0。spec の Scenario「更新係の外向きの塞ぎを外す手順は、置く段の 1 つだけである」は「置く段の台本を走らせる・塞ぎの規則を消す」手順の全部を数える主張
- kind: technical
- 提案: `soak-install.sh` の実地の口（`--self-test`・`--list` 以外の引数）を走らせる手順をどの引数でも数え、規則の名前 `ashiato-update-check-ci-outbound` を持つ手順を `--block-outbound` のほかに置く段だけに限る

## R8. 自己試験がまれに落ちる（「更新係の確かめた記録が無い」）
- 成果物: tools/release/soak-install.sh
- 根拠: 作業ツリーの台本のまま `--self-test` を連続 20 回 → 1 回 rc=1（`NG: updater の置く段以後の確かめた記録が無い` → `NG: self-test: 実地の口が通らない`）。それとは別に、複製の台本の 1 回でも同じ NG が出て、同じ複製を走らせ直すと 4 回とも通った。
  実地の口の自己試験（`soak_install_self_test`）は、偽のインストーラが `date` で書く `at` と台本が取る `start` をミリ秒の字面の `>` で比べ、インストーラの上限は `SOAK_SETUP_TIMEOUT=1` 秒。原因はまだ特定していない（出力にインストーラの失敗の NG は無い）。揺れを見たのはこの WSL の上だけ（CI の runner では走らせていない）
- kind: technical
- 提案: 落ちた回の `checks.jsonl` と `start` を観測に出し、原因を決めてから直す（偽の記録の時刻を `start` から作るなど、比べの両側を台本が決める形）

## R9. Task 2.2 の終了の条件（`install-windows` が置く段を含めて緑）が未確認のまま `[x]`
- 成果物: openspec/changes/ch22-soak-install-online-ci/tasks.md / .github/workflows/ci.yml
- 根拠: `gh pr list --head feat/ch22-soak-install-online-ci` → `[]`、`origin/feat/ch22-soak-install-online-ci` は無い。置く段の実地の口（`sc.exe queryex` の読み・`tasklist` の CSV・`wevtutil` の Date・Git Bash での `/FI` などの引数・DB のログの時刻）は、まだどこでも走っていない。
  自己試験は全部偽の命令で、R2・R3 のように偽の形と本物の形がずれたところは緑になる。tasks の本文も「下流の手元では走らない」と書いている
- kind: premise
- 提案: settle と final で、PR の CI の `install-windows` の置く段の出力（起こし直しの回数・誤りの行・`system-events`・`outage_notice_unavailable`・確かめた記録）を読んでから完了とする。R1〜R4 を直す前の走りが緑でも、判定が働いた証にはならない

## R10. 置く時間の 5 分（`SOAK_WAIT_SECONDS=300`）を変えても全部通る
- 成果物: tools/release/soak-install.sh
- 根拠: 複製の台本で `SOAK_WAIT_SECONDS=30` にした → `--self-test` を 4 回走らせて 4 回とも rc=0。`--self-test-run` が `SOAK_WAIT_SECONDS=1` に上書きし（`:619`）、頭の値を読むテストが無い。
  5 分は依頼の本人の決定（「5 分ほど」）で、spec の Requirement の本文と Scenario の名前にある。`SOAK_SAME_ERROR_LIMIT` には `== 3` の assert があるが、こちらには無い
- kind: daily
- 提案: 自己試験に `[[ "$SOAK_WAIT_SECONDS" == 300 ]]` を足す（同じ誤りの閾値と同じ形）
