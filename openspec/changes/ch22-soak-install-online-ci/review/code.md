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
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

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
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R3. システムの記録を読めなくても「読めた・0 件」と出て、警告が出ない
- 成果物: tools/release/soak-install.sh
- 根拠: `wevtutil.exe qe System /f:text 2>/dev/null | awk … | while … done >"$dir/events"`（`:266-290`）は、命令が落ちても空の `events` を作る。判定は `[[ -r "$events" ]]` で「読めた」とみなす（`:52-53`）。
  複製の台本で偽の `wevtutil.exe` を `exit 1` にした実地の口 → rc=0、出力は `system-events: readable count=0`、`::warning::` は無い、`events` は 0 バイト。
  spec の Scenario「システムの記録を読めなければ…」の GIVEN は「終了コード 1 で何も返さない偽の命令」だが、自己試験は `events` を消しているだけ（`:566`）。P11 の観測（読めたか）が CI のログで嘘になる
- kind: technical
- 提案: `wevtutil` の rc（と 1 件も読めないこと）で読めたかを決め、読めないときは `events` を作らない。自己試験は実地の口で偽の `wevtutil` を `exit 1` にして警告の行を見る。spec の AND（`ashiato-db` の番号を変えると落ちる）も足す
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R4. PC の収集アプリが、台本の起こし直しより前に終わっても落ちない
- 成果物: tools/release/soak-install.sh
- 根拠: `soak_collector_pid` は `[[ "$ts" > "$restart" && "$ts" < "$end" ]] || continue`（`:145`）で、起こし直しより前の読みを全部捨てる。spec は「置く間に、台本が止めた 1 回のほかに…終わる」なら落とす。
  起こし直しより前に PID が 100 → 空 → 101 と変わる読みを入れた `--evaluate` → rc=0（`collector pid changes=0`）。実地では最初の 60 秒（`SOAK_COLLECTOR_RESTART_SECONDS`）が見られない。1 回目の起動（案内を出す）で落ちる不具合は、台本の起こし直しで隠れる
- kind: technical
- 提案: 置く間の始まりから起こし直しまでの読みも数え、台本の `taskkill` の前後の 1 回だけを除く。自己試験に「起こし直しより前に消える」入力を足す
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R5. 「数えない」の 2 つの Scenario は 1 行だけで試していて、除外を消しても緑のまま
- 成果物: tools/release/soak-install.sh
- 根拠: 「警告の段の行と置く段の始まりより前の行は数えない」は `WARN` の 1 行（`:574`）、「置く段の始まりと置く間の始まりの間に書かれた誤りの行は数えない」は `ERROR` の 1 行（`:576`）。spec はどちらも 5 行（PC の収集アプリのログの警告、DB のログの `FATAL`）。
  複製の台本で、窓の比べ（`:88`）と `ERROR` の絞り（`:98`）を両方 `:` にした → `--self-test` rc=0（`OK: soak-install self-test`）。tasks 1.1 の「1 か所だけ直した入力では通ることを並べて見る」の逆（除外が効かなければ落ちる側）も無い。spec の AND（同じ 5 行を置く間へ移すと落ちる）も無い
- kind: technical
- 提案: spec のとおり 5 行を、spec の名指しのログ（PC の収集アプリ・DB）に置き、置く間へ移した写しで `expect_fail` を並べる。手 3 の「見る場所のずれ」4 件も、spec の GIVEN・THEN に合わせる
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R6. 台本の合言葉を伏せる値に入れていない
- 成果物: tools/release/soak-install.sh
- 根拠: `ng_secret_values` は `SOAK_RELEASE_PASSWORD`・`SOAK_TEST_PASSWORD`（どこでも決めていない）を出し、インストーラに渡す `SOAK_INSTALL_PASSWORD='soak-install-password-0123'`（`:16`・`:226`）を出さない（`:22-26`）。
  `ERROR login failed password=soak-install-password-0123` をサーバのログに入れた `--evaluate` → 出力は `error[server.log] ERROR login failed password=soak-install-password-0123 (count=1)`（伏せていない）。
  spec の Scenario「置く段の出力に秘密の値が出ない」は「画面の合言葉」も挙げるが、自己試験の入力（`:599`）に合言葉が無い。値は repo に書いてある固定の字面なので漏れの害は小さいが、spec の主張は満たしていない
- kind: technical
- 提案: `ng_secret_values` で `SOAK_INSTALL_PASSWORD` を出し、自己試験の誤りの行に合言葉を混ぜる
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R7. 「塞ぎを外す手順は置く段の 1 つだけ」の検査が、別の書き方の 2 つ目を見逃す
- 成果物: tools/release/soak-install.sh（`soak_ci_definition_self_test`）
- 根拠: 置く段の台本を走らせる手順を `'soak-install.sh package/'` の字面で数え（`:499`・`:518`）、規則を消す命令を `'delete rule'` で数える。
  複製の `ci.yml` で、`rust` の job に `./tools/release/soak-install.sh dist/ashiato-server.zip dist/ashiato-collector-windows.zip` を足した → `--self-test` rc=0。
  `shell: pwsh` の `Remove-NetFirewallRule -Name ashiato-update-check-ci-outbound` を足した → rc=0。spec の Scenario「更新係の外向きの塞ぎを外す手順は、置く段の 1 つだけである」は「置く段の台本を走らせる・塞ぎの規則を消す」手順の全部を数える主張
- kind: technical
- 提案: `soak-install.sh` の実地の口（`--self-test`・`--list` 以外の引数）を走らせる手順をどの引数でも数え、規則の名前 `ashiato-update-check-ci-outbound` を持つ手順を `--block-outbound` のほかに置く段だけに限る
- 処置: fixed 2.2
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R8. 自己試験がまれに落ちる（「更新係の確かめた記録が無い」）
- 成果物: tools/release/soak-install.sh
- 根拠: 作業ツリーの台本のまま `--self-test` を連続 20 回 → 1 回 rc=1（`NG: updater の置く段以後の確かめた記録が無い` → `NG: self-test: 実地の口が通らない`）。それとは別に、複製の台本の 1 回でも同じ NG が出て、同じ複製を走らせ直すと 4 回とも通った。
  実地の口の自己試験（`soak_install_self_test`）は、偽のインストーラが `date` で書く `at` と台本が取る `start` をミリ秒の字面の `>` で比べ、インストーラの上限は `SOAK_SETUP_TIMEOUT=1` 秒。原因はまだ特定していない（出力にインストーラの失敗の NG は無い）。揺れを見たのはこの WSL の上だけ（CI の runner では走らせていない）
- kind: technical
- 提案: 落ちた回の `checks.jsonl` と `start` を観測に出し、原因を決めてから直す（偽の記録の時刻を `start` から作るなど、比べの両側を台本が決める形）
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R9. Task 2.2 の終了の条件（`install-windows` が置く段を含めて緑）が未確認のまま `[x]`
- 成果物: openspec/changes/ch22-soak-install-online-ci/tasks.md / .github/workflows/ci.yml
- 根拠: `gh pr list --head feat/ch22-soak-install-online-ci` → `[]`、`origin/feat/ch22-soak-install-online-ci` は無い。置く段の実地の口（`sc.exe queryex` の読み・`tasklist` の CSV・`wevtutil` の Date・Git Bash での `/FI` などの引数・DB のログの時刻）は、まだどこでも走っていない。
  自己試験は全部偽の命令で、R2・R3 のように偽の形と本物の形がずれたところは緑になる。tasks の本文も「下流の手元では走らない」と書いている
- kind: premise
- 提案: settle と final で、PR の CI の `install-windows` の置く段の出力（起こし直しの回数・誤りの行・`system-events`・`outage_notice_unavailable`・確かめた記録）を読んでから完了とする。R1〜R4 を直す前の走りが緑でも、判定が働いた証にはならない
- 処置: escalated

## R10. 置く時間の 5 分（`SOAK_WAIT_SECONDS=300`）を変えても全部通る
- 成果物: tools/release/soak-install.sh
- 根拠: 複製の台本で `SOAK_WAIT_SECONDS=30` にした → `--self-test` を 4 回走らせて 4 回とも rc=0。`--self-test-run` が `SOAK_WAIT_SECONDS=1` に上書きし（`:619`）、頭の値を読むテストが無い。
  5 分は依頼の本人の決定（「5 分ほど」）で、spec の Requirement の本文と Scenario の名前にある。`SOAK_SAME_ERROR_LIMIT` には `== 3` の assert があるが、こちらには無い
- kind: daily
- 提案: 自己試験に `[[ "$SOAK_WAIT_SECONDS" == 300 ]]` を足す（同じ誤りの閾値と同じ形）
- 処置: fixed D17 仮
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## security review（7817dc5..d5e974b）

見た経路:
- `install-windows` の置く段 → `soak_remove_outbound_rule`（`tools/release/soak-install.sh:214`）→ 更新係と PC の収集アプリが本物の配布元へ出る。塞ぎを外すのは job の最後の試験の step（`ci.yml` の「配る形を外につながる状態で置く」）で、後ろは `check_scenarios`（差分の外・変わらず）だけ（3）
- 新しい listen・平文の外への送信先・証明書の検証を切る設定・依存の追加: 差分に無い（`git diff --stat` は ci.yml・docs/testing.md・soak-install.sh・openspec だけ）（2）
- サーバ・DB・更新係・PC の収集アプリのログの ERROR 行 → `soak_out`（`:28`、`ng_redact`）→ CI の標準出力。設定の秘密（API_TOKEN・WEB_PASSWORD_VERIFIER・DB の URL）は伏せる。CI の runner の上の記録は runner の窓だけで、本人の記録は届かない（4: `--self-test` rc=0。`:599` の「秘密の値が出ない」の場合を含む）
- 実地の口（引数 2 つ）→ `soak_reset_machine`（`:184`）→ サービスの停止・削除、`C:/ProgramData/ashiato`（DB）と `%LOCALAPPDATA%/ashiato/collector`（PC の収集アプリの未送信の outbox）を `rm -rf`。CI の上かを見る関門は無く、zip の有無より前に走る（4。下の R11）
- 偽の命令で台本を走らせる口 `--self-test-run`（`:617`）: 置き場を `C:/…` の相対パスに固定し、作業の箱の中で走らせる。本番の組み立てからは呼ばれない（3）

安全の決め手: 「外向きの塞ぎを外すのは、配布元へ出ることを主張する置く段 1 つだけで、その後ろに本物の配布元に問い合わせる別の step が無い」—— 確かさ 3（`ci.yml` の `install-windows` の step の並びを辿った。別の書き方の 2 つ目は R7 が既に指摘）。秘密の伏せは確かさ 4（`tools/release/soak-install.sh --self-test` → `OK: soak-install self-test`）

## R11. 実地の口は CI の上かを確かめずに、本人の PC の DB と PC の収集アプリの未送信の記録を消す
- 成果物: tools/release/soak-install.sh
- 根拠: `soak_run`（`:294`）は zip を開く（`:223`・`:310`）より前に `soak_reset_machine`（`:304`）を呼び、`:202`〜`:209` で `C:/ProgramData/ashiato`（DB）と `SOAK_COLLECTOR_DIR` = `%LOCALAPPDATA%/ashiato/collector`（`:299`。`crates/collector-windows/src/config.rs:146`〜`148` の置き場と同じで、`runtime.rs:305` の `outbox.jsonl` を持つ）を `rm -rf` する。`GITHUB_ACTIONS` や確認の引数を見る分岐は `main`（`:615`）に無い。
  台本で確かめた: 一時の箱の中に `C:/ProgramData/ashiato/db/PG_VERSION` と `LOCALAPPDATA/ashiato/collector/outbox.jsonl` を置き、`sc.exe` などを rc=1 の偽にして、`GITHUB_ACTIONS`・`CI` を外し、**存在しない** zip 2 つで `soak-install.sh` を走らせた → `NG: 配るサーバの zip を開けない` rc=1 で終わったが、終わったあとに 2 つの置き場は空。`taskkill /F /IM ashiato-collector-windows.exe` も呼ばれていた（確かさ 4）。
  DB を消すこと自体は既存の `update-check-install.sh:140`・`update-apply-install.sh` の reset と同じだが、**PC の収集アプリの置き場（まだサーバへ届いていない記録）を消すのはこの差分から**で、本人が自分の Windows で置く段を試せば（`docs/testing.md` は「Windows の実地の確かめ」の台本として並べている）、まだ送っていない記録が戻らない形で消える
- kind: conflict
- 提案: 実地の口は `GITHUB_ACTIONS=true`（か、明示の `--i-am-ci` のような引数）でないと reset の前に `die` する。少なくとも `SOAK_COLLECTOR_DIR` の既定を本人の `%LOCALAPPDATA%` にせず、`ASHIATO_STATE_DIR` で作業の箱の置き場を渡して収集アプリを起こす。引数の zip の確かめは reset より前に回す
- 処置: fixed D16 仮

範囲外: `update-check-install.sh`・`update-apply-install.sh` の reset も CI の上かを見ずに `C:/ProgramData/ashiato` を消す（既存）。
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## /code-review（7817dc5..721d2e3）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R12. The soak step never reads the updater's error log, so updater errors during the soak can never fail it. soak_collect_observations reads `$SOAK_UPDATER_DIR/updater.log`, but the updater writes its errors to `dir/logs/updater.log` (crates/updater/src/lib.rs `log_service_error`). Fix: change the path to `$SOAK_UPDATER_DIR/logs/updater.log`.
- 成果物: tools/release/soak-install.sh:259
- 根拠: During the soak, the updater service writes the same error 3 or more times to C:/ProgramData/ashiato-updater/logs/updater.log. The script reads a file that does not exist, so logs/updater.log ends up empty. The 'same error repeated' check does not fire and the step passes.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R13. The collector's PID reading returns the session number, not the PID, so the 'PID changed' check can never fire. In the tasklist CSV line `"exe","PID","Console","Session#","Mem"`, the greedy `.*,"\([0-9][0-9]*\)",` in `sed` matches the last `,"digits",`, which is the Session# field. Fix: take the second column with a non-greedy pattern such as `sed -n 's/^"[^"]*","\([0-9]*\)".*/\1/p'`, and make the self-test stub check the value too.
- 成果物: tools/release/soak-install.sh:248
- 根拠: The collector crashes during the soak and something restarts it right away (PID 100→200). collector.pid records the same session number ('1' or '0') each time, so changes=0 and the step passes. The self-test stub's `"42","Console","1"` also yields '1', and the self-test does not catch it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R14. On failure, the NG output shows only a file path, not the file's contents, so the cause can't be traced. `die_seen <reason> <value>` prints its second argument as the observed value; file contents only come out when the file is passed as ng_report's fourth argument. This script passes `"$work/install.out"`, `"$dir/services/$svc"` (l.64), `"$SOAK_TMP/errors"` (l.108) and `"$file"` (l.154) as the second argument. On top of that, NG_STATE_DIR, NG_HEALTHZ_URL and NG_SETTINGS_FILES are never set, so the service, healthz and server-log observations are empty too. Fix: pass the file contents as the value (e.g. the output of `tail -n 80`), or call ng_report with the file as the fourth argument, and set the NG_* variables in soak_run.
- 成果物: tools/release/soak-install.sh:228
- 根拠: The installer fails during the soak. The CI log shows only 'NG: 配る形のサーバのインストールに失敗' and the path '/tmp/ch22-soak-run.xxx/install.out', and $work is then deleted. Nothing remains to explain why it failed.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1
- 判定: NOT ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）: install.out は tail -n 80 の中身を渡し、NG_* も設定した。しかし die_seen の services/$svc・$SOAK_TMP/errors・collector.pid の 3 か所は、パスを渡すまま。中身が出ず、評価後に消える
- 判定: NOT ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）: soak-install.sh:74 ("$dir/services/$svc")・:125 ("$SOAK_TMP/errors")・:168 ("$file") が今もパスを die_seen の第 2 引数に渡す。die_seen は中身を出さない（check-ng-observations.sh:269）。:247 の install.out だけ tail の中身。今回の diff はこの 3 行に触れていない
- 判定: NOT ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）: soak-install.sh:74・:125・:168 が今もパスを die_seen の第 2 引数に渡す。die_seen は中身を出さない（check-ng-observations.sh:269）。今回の diff はこの 3 行に触れていない。install.out（:246）だけ tail の中身を渡している
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R15. Real secrets are written as plain text into the shared temp folder and never removed. NG_OBSERVED_DIR defaults to `${TMPDIR:-/tmp}`, so ng_secret_file/ng_collect_secrets inside soak_evaluate write `$TMPDIR/.ng-secrets`, `.ng-secrets.vals` and `.ng-read-API_TOKEN.val`. These hold the values read from server.json/owner.json: API_TOKEN, WEB_PASSWORD_VERIFIER and the DB passwords. Only the --self-test path removes them. Fix: point NG_OBSERVED_DIR at a dedicated folder created with mktemp and remove it in the EXIT trap.
- 成果物: tools/release/soak-install.sh:13
- 根拠: After a real soak run on the Windows runner, C:\Users\runneradmin\AppData\Local\Temp\.ng-secrets keeps the server's API token and DB owner password. Later steps can read them, and they leak if the folder is uploaded as an artifact or the machine is shared.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1
- 判定: NOT ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）: soak_evaluate の trap は RETURN のみ。die は exit なので発火せず、失敗経路で ch22-soak-observed.*（.ng-secrets）が残る。実測で 258 個
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R16. The real run's work folder is never cleaned up. soak_evaluate's `trap '...' RETURN EXIT` replaces main's `trap 'rm -rf "$work"' EXIT` (l.629). Even if it did not, `work` is local to main, so under `set -u` it is unbound when the EXIT trap runs after main returns. Fix: register the cleanup of $work and SOAK_TMP together in one global trap. Do not override EXIT inside soak_evaluate (use RETURN only, or clean up explicitly).
- 成果物: tools/release/soak-install.sh:160
- 根拠: After every real run, /tmp/ch22-soak-run.XXXXXX is left behind, holding copied logs, the event log, the collector executable and install.out. This also trips the harness's leak check (HARNESS_TMP_LEAK=fail).（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R17. The baseline line count is always written as 0, so a log line whose timestamp can't be read is counted even if it was written before the soak window. The updater log uses the format `Utc::now().to_rfc3339()`, which ends in `+00:00`, and the timestamp regex (which only accepts `...Z`) does not match it. Every updater log line therefore falls through to the 'unreadable timestamp' path and the window check does not apply. Fix: record each log's line count at soak_start as the baseline, and accept the `+00:00` form in the regex as well.
- 成果物: tools/release/soak-install.sh:264
- 根拠: Right after install, before the soak starts, the updater writes the same error 3 times (e.g. at startup). Once the path in the first finding is fixed, those lines are counted as errors 'inside the window', and the soak fails even though it ran normally.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R18. The collector the script started is never stopped when the step ends. It is started in the background with `"$collector" &` and never killed after the soak, so it keeps running from $work. Fix: kill the collector with taskkill before evaluating or before exiting.
- 成果物: tools/release/soak-install.sh:320
- 根拠: Once the soak step ends, the collector keeps running for the rest of the job. On Windows it locks the executable under $work, so `rm -rf "$work"` fails. Later steps also see a collector that keeps sending and writing its log.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R19. The 'system-events: readable count' shown in the output is three times the real number of events. `((total++))` sits inside the per-service loop and runs for every 7031/7034 event, whichever service it belongs to. Fix: count total in a separate loop that runs once, or only on lines that match a service.
- 成果物: tools/release/soak-install.sh:58
- 根拠: With a single 7034 event, the output reads 'readable count=3', which is misleading when someone reads the evidence.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R20. The secrets list includes two variables that are never set, while the password actually used for install is left out. ng_secret_values outputs `SOAK_RELEASE_PASSWORD` and `SOAK_TEST_PASSWORD`, which nothing sets. `SOAK_INSTALL_PASSWORD`, the one fed to the installer, is not listed. Fix: add `$SOAK_INSTALL_PASSWORD` to the output and remove the unused variable names.
- 成果物: tools/release/soak-install.sh:25
- 根拠: If the installer echoes the entered password into install.out or a log, the password is printed to the CI log without being masked.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R21. An error message containing `|` is never counted, so its repeats go undetected. Keys are saved as `name|key` and counted with `awk -F'|'` comparing only `$2`. If the key itself contains `|`, it never matches and count=0. Comparing timestamps as strings also gets the order wrong when the fractional-second precision differs (chrono omits the fraction for whole seconds, and 'Z' sorts after '.'). Fix: separate the fields with a tab or similar and compare with `substr`, and normalize timestamps before comparing.
- 成果物: tools/release/soak-install.sh:106
- 根拠: The server logs 3 lines of `ERROR ... a|b`. Each count comes out as 0, the 'same error repeated' check does not fire, and the step passes.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @e43c6b3c87976157edc7fdaa0cd1d7cf2b184c2c）
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## settle_review（re-review 1 @e43c6b3）: 処置の直しが新しく壊したもの

## R22. 失敗の経路（die → exit 1）では RETURN の trap が発火せず、SOAK_TMP と observed（.ng-secrets を持つ）が消えない。自己試験の失敗ケースごとに残り、1 回で約 46 個。実測で ch22-soak-eval.* と ch22-soak-observed.* が計 548 個。HARNESS_TMP_LEAK=fail に当たり、実際の失敗でも秘密が平文で残る（R15 が直りきっていない理由）。EXIT の掃除に両方を足す。
- 成果物: tools/release/soak-install.sh
- 根拠: tools/release/soak-install.sh:soak_evaluate (trap ... RETURN)（re-review 1 の New Breakage。Important）
- kind: technical
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R23. 起こした直後の最初の読みで PID が空だと、起こし直し前の空 PID は許されず「変化」と数えられる。tasklist が間に合わないときだけ起きる。
- 成果物: tools/release/soak-install.sh
- 根拠: tools/release/soak-install.sh:soak_collector_pid（re-review 1 の New Breakage。Minor）
- kind: technical
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R24. 収集アプリのスタブの at が date の現在時刻のまま。R1 で return 0 を外したため、起こし直しの時刻との取り合いに負けると失敗になる。12 回では再現しなかった。
- 成果物: tools/release/soak-install.sh
- 根拠: tools/release/soak-install.sh:soak_install_self_test (collector スタブ)（re-review 1 の New Breakage。Minor）
- kind: technical
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R25. must_fail に ValueError を足したため、broken/ 差し替えは数えの assert でなく index の例外で通る。その must_fail の効きが弱い。
- 成果物: tools/release/soak-install.sh
- 根拠: tools/release/soak-install.sh:soak_ci_definition_self_test（re-review 1 の New Breakage。Minor）
- kind: technical
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 2 @14194630e25a9959cef2a0be2940dc8455ce94a8）
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## settle_review（re-review 2 @1419463）: 処置の直しが新しく壊したもの

## R26. soak_evaluate が張る EXIT trap（SOAK_TMP と SOAK_OBSERVED だけを消す）が main の soak_cleanup を置き換える。実地の口で評価の中が die すると $work が消えず taskkill も呼ばれない。RETURN の trap だけが soak_cleanup に戻す。落ちた回に /tmp/ch22-soak-run.* が残り、R16 の HARNESS_TMP_LEAK が失敗の回で再発する。直し方: EXIT を置き換えず、soak_cleanup（:39-42）が SOAK_TMP と SOAK_OBSERVED も消す（既にそうなっている）。
- 成果物: tools/release/soak-install.sh
- 根拠: tools/release/soak-install.sh:178-179（re-review 2 の New Breakage。Minor）
- kind: technical
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 3 @3b6e9e86edacdd3bdfe1d6b3844eb288c2f3589c）
- 判定: ADDRESSED（re-review 4 @139ebe133dd382fb8823a3d06fdd1a921e13b0de）
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## final review（7817dc5..c2f6468）

## R27. Git Bash が引数を場所の名前に書き換えるのを止めていない。`/F` `/IM` `/FI` `/FO` `/NH` `/f` `/T` `/grant` が化けるおそれが強い。化ければ PC の収集アプリの PID がいつも空になり、「動き続ける」の判定は何を観測しても通る（Critical。台本を読んでの見込みで、本物の runner では未観測。R9）
- 成果物: tools/release/soak-install.sh
- 根拠: `soak-install.sh` と、読み込む唯一の台本 `check-ng-observations.sh`（`:7` の source）には `MSYS_NO_PATHCONV` も `MSYS2_ARG_CONV_EXCL` も無い。`.github/workflows/ci.yml` の install-windows の置く段（`:652-654`）にも、job にも env は無い（`grep -n MSYS` で 0 件）。兄弟の台本は止めている（`install-check.sh:705` の `export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'`、`update-apply-install.sh:200-202`、`update-check-install.sh:37-39,213`）。
  化ければ `tasklist.exe /FI … /FO CSV /NH`（`soak-install.sh:216-224`）は失敗し、PID は空になる。`soak_collector_pid` は空の行を飛ばすので `changes=0` となり、判定は必ず通る。
  `taskkill /F /IM`（`:40,266,357,365`）は `|| true` に隠れて効かない。そのため 60 秒での起こし直しも、終わりの停止（R18）も実際には起きない。`reg delete /f` と `icacls /T` も効かない。
  自己試験の偽の命令は引数を見ないので、これを捕まえられない
- kind: technical
- 処置: fixed 2.1
- 提案: Windows の口の頭で `install-check.sh` と同じく `export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'` を置く。あわせて、起こし直しの前と後のそれぞれに空でない PID が 1 つ以上あることを求め、すべて空なら落とす。注意: `MSYS_NO_PATHCONV=1` を置くと、Windows の命令（インストーラ・PC の収集アプリ・Python）に渡す `/tmp/…` や mktemp の場所も読み替えられなくなる（`install-check.sh:752` の注記）。`$work` などは `cygpath -m` で `C:/…` の形にして渡す
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R28. R22 が戻っている。自己試験で落ちる場合ごとに、一時の置き場 `ch22-soak-eval.*` と `ch22-soak-observed.*`（偽の秘密の `.ng-read-*.val` 入り）が 1 組ずつ残る
- 成果物: tools/release/soak-install.sh
- 根拠: `soak-install.sh:656-657`（`expect_fail` / `expect_pass` が部分殻の中で `soak_evaluate` を呼ぶ）と `:178`。3b6e9e86e（R26 の直し）で `soak_evaluate` の中の EXIT trap を外した。部分殻の中で `die` すると RETURN の trap は発火しない。親の `soak_cleanup` は部分殻の変数を見られない。
  隔離した `TMPDIR` で `--self-test` を 1 回走らせると、それぞれ 17 個残った（落ちる場合の数と一致）。HARNESS_TMP_LEAK=fail に当たる
- kind: technical
- 処置: fixed 1.1
- 提案: 部分殻の中で `trap 'rm -rf "${SOAK_TMP:-}" "${SOAK_OBSERVED:-}"' EXIT` を張ってから `soak_evaluate` を呼ぶ。または `--self-test` の間は `TMPDIR` を自己試験の作業場所の下に向け、終わりにまとめて消す
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R29. サーバと DB のログの基準の行数がいつも 0 になる（R17 の直しがディレクトリを見ていない）。日時を読めない行は、インストールのときの誤りまで数える
- 成果物: tools/release/soak-install.sh
- 根拠: `soak-install.sh:278` の `[[ -f "$path" ]]` は、`logs/server` と `logs/db`（`:275-276`。ディレクトリ）ではいつも偽になり、`0` が書かれる。一方、集める側（`:293`）は `cat "$path"/*` でつないでいる。
  design D6 の「置く間の始まりのログの大きさ」と食い違う。Scenario「置く段の始まりと置く間の始まりの間に書かれた誤りの行は数えない」は、日時が読める前提でしか守られない
- kind: technical
- 処置: fixed 1.1
- 提案: ディレクトリなら `cat "$path"/* | wc -l` を基準にする。自己試験に、基準が 0 でない場合（日時を読めない古い行がある場合）を足す
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R30. `--self-test-run` は `GITHUB_ACTIONS` の確かめを通らずに、`soak_reset_machine`（本人の PC の DB と未送信の記録を消す）まで行ける（R11 の穴）
- 成果物: tools/release/soak-install.sh
- 根拠: `soak-install.sh:669-672`。Windows では `C:/ProgramData/ashiato` は絶対の場所なので、偽の PATH が無いまま呼ぶと本物を消す（design D16）
- kind: technical
- 処置: fixed 2.1
- 提案: `--self-test-run` は、偽の命令の印（例: `ST22_CALLS` が設定済み）があるときだけ受け付ける
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R31. 自己試験の偽の `rm` が見る `$SOAK_STATE_DIR` は export されていないので空になる。`ST22_KEEP_STATE=1` の間はどの rm も何もせず、`ng-obs.*` が 1 つ残る
- 成果物: tools/release/soak-install.sh
- 根拠: `soak-install.sh:420-424`。`*""*` がすべてに当たる。判定は settings だけが残る形で区別できているので、試験の結論は変わらない
- kind: technical
- 処置: fixed 2.1
- 提案: 偽の命令に場所を export して渡すか、偽の `rm` の条件を空の値で当たらない形にする
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R32. 設定の秘密を読めないと、台本の合言葉も伏せる値に入らず、失敗も握りつぶされて、値が伏せずに出る
- 成果物: tools/release/soak-install.sh
- 根拠: `soak-install.sh:23-27`（`ng_settings_secrets` が失敗すると `|| return 1` で、その後の `SOAK_INSTALL_PASSWORD` を出さない）と `:179`（`ng_collect_secrets || true`）。その後の `soak_out` と `die_seen` が出す値は伏せられない
- kind: technical
- 処置: fixed 1.1
- 提案: 台本の合言葉を先に出す。秘密を読めなかったときは、誤りの行の中身を出さずに落とす
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R33. `NG_OBSERVED_DIR` の既定が `$TMPDIR` のまま。評価より前で `die` すると（インストールの失敗・healthz）、本物の秘密の入った `.ng-secrets` と `.ng-read-*.val` が `$TMPDIR` に残る（R15 は評価の中しか直っていない）
- 成果物: tools/release/soak-install.sh
- 根拠: `soak-install.sh:13`。runner は捨てられるので影響は小さい
- kind: technical
- 処置: fixed 2.1
- 提案: `soak_run` の頭で `NG_OBSERVED_DIR` を mktemp の専用の置き場に向け、`soak_cleanup` で消す
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R34. 「秘密の値が出ない」の Scenario は、GIVEN の「サーバのログと DB のログ」のうちサーバのログしか試していない
- 成果物: tools/release/soak-install.sh
- 根拠: `soak-install.sh:650`。`openspec/changes/ch22-soak-install-online-ci/specs/distribution/spec.md` の該当の Scenario
- kind: technical
- 処置: fixed 1.1
- 提案: DB のログに秘密を入れる場合を自己試験に足す
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R35. 文書に古い記述が残っている。「2 つの台本の `--self-test` がこれを呼ぶ（CI の `chain`）」とあり、`rust` の行に `soak-install.sh --self-test` が無い（task-3 F1・F2 の残り）
- 成果物: docs/testing.md
- 根拠: `docs/testing.md:189` と `:32`。`.superpowers/sdd/ch22-task-3/task-3-findings.md` の F1・F2
- kind: technical
- 処置: fixed 3.1
- 提案: 呼ぶ台本と job の実際に合わせて 2 か所を直す
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R36. 1 行ごとに `sed` と `date` を起こし、段の絞り込みはその後にしている。Windows の fork は遅い（Minor）
- 成果物: tools/release/soak-install.sh
- 根拠: `soak-install.sh:100-105`
- kind: technical
- 処置: fixed 1.1
- 提案: 段で先に絞ってから日時を読む
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## R37. `soak_install_server` が `install-check.sh` と同じ口で入れていない（`--profile` / `--downloads` と、別の管理者で走らせる口を使わない）。tasks の「同じ口で入れる」と違うが、意図的かは確かめていない
- 成果物: tools/release/soak-install.sh
- 根拠: `soak-install.sh:239`（`soak_install_server`）と `install-check.sh:1225` の比べ。`openspec/changes/ch22-soak-install-online-ci/tasks.md:67` の「配る zip を `install-check.sh` と同じ口で入れる」
- kind: technical
- 処置: fixed 2.1
- 提案: 意図的でなければ `install-check.sh` の口に合わせる。意図的なら design に理由を書く
- 判定: ADDRESSED（re-review 5 @3db1f675eb080b537fdaa70db5e28616ab2eb318）

## settle_review（re-review 5 @3db1f67）: 処置の直しが新しく壊したもの

## R38. --self-test-run のガード（ST22_CALLS が設定済みでそのファイルがある）は環境変数 1 つで通せる。PATH に偽の命令があることまでは見ないので、本物の soak_reset_machine を呼ばない保証ではなく、誤操作の防ぎにとどまる。
- 成果物: tools/release/soak-install.sh
- 根拠: tools/release/soak-install.sh:728（re-review 5 の New Breakage。Minor）
- kind: technical
- 処置: fixed 1.1
