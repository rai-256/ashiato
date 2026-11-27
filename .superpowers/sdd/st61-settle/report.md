# ST61 settle — fix report（第 1 回。R1〜R10）

コードの直しは commit 07ddea04c。壊して落ちるかは使い捨ての写しで 1 か所ずつ壊して rc を見た（戻した後に git diff が空）。

## R1（fixed 6.1）ci.yml の差分の基準の配線
- `tools/release/check-guards.sh` に `ci_changes_base` を足し、`workflows` から呼ぶ。changes の job の `permissions: { contents: read, actions: read }`・`id: prev` の手順の `if: github.event_name == 'push'`（ほかの if・continue-on-error は無いこと）・`branch=main&event=push&status=success` の問い合わせ・`export PREV_SUCCESS_SHA`・`echo "base=$(tools/release/changes.sh base)"`・`BASE: ${{ github.event.pull_request.base.sha || steps.prev.outputs.base }}` を行の単位（-x）で見る（注釈にした行は数えない）
- 自己試験に偽の ci.yml 7 つ（before に戻す・actions: read を外す・status=success を外す・if: false・continue-on-error・export を注釈・base=all）。7 つとも `workflows` が rc=1（理由の文はそれぞれ別）。`ci_changes_base` の呼び出しを外すと `--self-test` rc=1

## R2（fixed 5.1）番号付けの workflow の手順
- `number_steps`（python）: number.yml の `uses:` を許す action（checkout・upload/download-artifact・setup-java/node・rust-toolchain）に、`shell:` を bash に、`run:` の各コマンドの先頭の語を許す一覧（version.sh・number.sh・build.sh・echo・sed・tee・mkdir・find・`[[`）に限る。`$(…)` の中も見る。gh・git・curl・xargs・bash などは run のどこにあっても落とす
- 自己試験に偽の number.yml 7 つ（`gh release create v…`・`git tag v… && git push`・run の塊の中の git・`$(gh …)`・`find -exec gh`・`softprops/action-gh-release`・`shell: python`）。全部 rc=1。前の禁止の並び（162 行）に当たらないものも落ちる。`number_steps` を外すと `--self-test` rc=1

## R3（fixed 1.1）versionCode の上限の境目
- `version-table.tsv` に `0.999.9999`→9999999・`209.999.9999`→2099999999、`version-check-table.tsv` に `0.999.9999`（左 0）ok・`209.0.0`（左 209）ok・`210.0.0` ng
- 壊して: version.sh の MINOR<=998・MAJOR<=100 → `--self-test` rc=1。Rust の MAX_MIDDLE=998・MAX_LEFT=100 → `cargo test st61_` rc=101。build-stamp.ts の LIMITS [209,998,9999]・[100,999,9999] → vitest rc=1。gradle の minor>998 → `verifyVersionCodeTable verifyVersionCheckTable` rc=1

## R4（fixed 2.2）APK の版の検証
- `check-artifacts.sh apk-version` は `--expect-version` を必須にし、渡した版番号と必ず比べる（無ければ usage / NG）。`apk-build <版番号>` を足した: 置き場 `app/build/outputs/apk/release` を空にし、渡した版番号で assembleRelease を組み、組んだ APK を全部渡した版番号と比べる（0 個なら落ちる）
- `check-build-stamp.sh` の版番号を渡さない 2 か所に `--expect-version "$cargo_version"`・`0.0.9` を足した。`--self-test` に「比べる版番号が無ければ落ちる」を足した
- 2.2 の 3 本目を plan_fix（machine）で訂正: 旧コマンドは FAIL（evidence 13:44:58。usage）、新 `tools/release/check-artifacts.sh apk-build 0.4.0-beta.2` は PASS（13:46:06、`OK: versionCode='40000' versionName='0.4.0-beta.2'`）。plan-corrections.md に記録。訂正の壊しての確かめ（correction-review.md）は別の席

## R5（fixed 4.1）置き場と番号の判定の守り
- number.sh の自己試験: 公開済み（isDraft:false）の release に place → 「下書きでない」で落ち書く要求 0 件、全部揃っていても complete=no（下書きに戻すと yes）、手元の印 build-0.2.9 が別の commit を指せば tag が「手元の印」で落ち push 0 件
- version.sh の自己試験: merge した枝の中の commit（前の番号の子孫・製品に触れる）に `number` → `number=` と「first-parent の上に無い」、merge の commit には 0.2.1
- 壊して: number.sh 95・118・49 行を外す → `--self-test` 3 つとも rc=1。version.sh の first-parent の判定を `true ||` → rc=1

## R6（fixed 1.1）MAJOR の CRLF
- version.sh に `trim`（前後の空白と \r を落とす。Rust・gradle・画面の trim と同じ）を足し、`major_file` と `number` の `git show …:MAJOR` に当てた
- 自己試験: MAJOR が `0\r\n`・` 0 \n`・`0\n` なら check 0.2.0 が通り、`x`・空・`+0`・`0 0`・`-0` なら落ちる。trim を外すと `--self-test` rc=1

## R7（fixed 6.1）祖先でない差分の基準
- `changes.sh base`: PREV_SUCCESS_SHA が HEAD の祖先（HEAD 自身を除く）のときだけ返し、そうでなければ all
- 自己試験: C1 を HEAD にして子孫 C2 を渡す → all、HEAD 自身 → all、枝分かれ（force-push で消えた側）→ all。祖先の判定を外すと rc=1

## R8（fixed 1.1）bash の出所の版の規則
- `check_rule` が出所の版を `0.0.<N≥1>`（後ろ無し）に限る（spec「出所の版を 0.0.<N>（N は 1 以上）の形に限り」）。自己試験の plan の木の Cargo.toml を 0.4.0 → 0.0.1 にした（plan v0.4.0 などは番号として通る）
- 表に出所が 0.0.x でない行（`0.4.0`/`0.4.0`・`0.1.0`/`0.1.0`・`0.0.0`/`0.0.0`・`0.2.0`/`0.1.0`・`0.2.0`/`0.0.1-beta.1`、どれも ng）。bash・Rust・gradle・TS が同じ表で全部通る。bash の判定を外すと `--self-test` rc=1

## R9（fixed 2.1）Rust の MAJOR の `+0`
- `parse_major`: trim の後、ASCII の数字だけのときだけ parse。試験 `st61_major_file_accepts_only_digits`（`+0`・`-0`・`0 0`・空・`0.1` を断り、`0\r\n`・` 1 \n` を受ける）。`cargo test -p ashiato-build-stamp st61_` 5 本 PASS（--list で 5）

## R10（escalated）題が「wip」の commit
- 履歴の書き換えは settle がしない。deep.md に第 2 回 Q5（`(未回答)`・loss: discarded）、`deep-questions-r2.json`、`docs/briefs/ST61-deep-r2.html`（ask_wizard rc=0）。推奨は「書き直さない」

## 走らせた検証（scripts/verify-run。evidence.jsonl）
- 1.1・3.1・4.1・4.2・5.1・6.1・2.1・2.3: 全部 PASS（13:46〜14:00 UTC）
- 2.2: 旧 3 本目 FAIL（実証）・新 3 本目 PASS（候補）
- 2.4: 下に追記
- 2.4: `tools/check-openapi.sh`・`check-build-stamp.sh release`（--expect-version を足した 2 か所を含む）・`check-build-stamp.sh android` の 3 本とも PASS（14:00:44・14:01:08・14:12:24 UTC）

## 答えの後の見直し（2026-11-27）
- R10（escalated のまま）: deep.md 第 2 回 Q5 の答え「そのまま merge する（書き直さない）」を処置の行に写した。履歴の書き換えも成果物の変更もしない（`git log --format='%h %s' 58f7a4d8c -1` → `58f7a4d8c wip` のまま）。走らせた検証は無い（コードに触れていない）

## R11（fixed 5.1）number_steps の書き方による抜け
- `number_steps` は字面の行でなく `yaml.safe_load` で読み、jobs → steps の `run` を文字列として見る（flow 形式 `- { run: … }`・複数行の plain scalar も同じ文字列になる）。YAML として読めない・yaml が無いときは落ちる
- 直に呼んではいけないコマンドの前が `/` でも拾う（`/usr/bin/gh`）。コマンドの区切りに `&` 1 つ（`2>&1` の `>&` は除く）・改行・find の `-exec`/`-execdir`/`-ok`/`-okdir` を足し、その後の語も許す一覧（`cp` を足した）で見る
- step の鍵を許す一覧（id・name・if・uses・with・run・shell・env）に限り、env の `BASH_ENV`/`ENV`、job の `uses:`（別の workflow を呼ぶ）を落とす
- `--self-test` に偽の number.yml 4 つ（末尾の place の job に足す）: `flow-step`・`plain-multiline`・`background-abs`・`find-exec-abs`。4 つとも YAML として読め（place の 5 番目の step）、直す前の number_steps では rc=0、直した後は rc=1。number_steps を直す前の版に戻すと `--self-test` が `NG: 偽の number.yml（flow-step）を通した` で rc=1
- 誤って落とさないか: `2>&1 | tee log` は rc=0、`echo a & "g"h …`・`find . -exec "/usr/bin/g"h x \;` は rc=1、本物の number.yml は rc=0
- `scripts/verify-run 5.1`: `check-guards.sh --self-test`・`check-guards.sh workflows`・actionlint の 3 本とも PASS（2026-11-27 08:47 UTC）

## R12（fixed 5.1）workflow・job の単位の shell と env
- `number_steps` に `check_shell`・`check_env`・`check_defaults` を足し、step だけでなく workflow の単位と各 job の `defaults.run.shell`（bash でなければ落とす）と `env`（`BASH_ENV`/`ENV` を落とす）を同じ規則で見る。defaults・env が対応表でなければ落とす
- `--self-test` に偽の number.yml 4 つ: `defaults-workflow`（`defaults: { run: { shell: "python {0}" } }`）・`defaults-job`（place の job の `defaults.run.shell: "gh release create v1 {0}"`）・`env-workflow`（`BASH_ENV`）・`env-job`（place の job の `ENV`）。4 つとも YAML として読め（yaml.safe_load で該当の鍵が出る）、直す前の版（HEAD 6fe4304 の number_steps）では rc=0、直した後は rc=1。本物の number.yml は両方 rc=0
- 壊して: workflow の単位の `check_defaults` の 1 行を外すと `--self-test` が `NG: 偽の number.yml（defaults-workflow）を通した` で rc=1
- `scripts/verify-run 5.1`: `check-guards.sh --self-test`・`check-guards.sh workflows`・actionlint（docker）の 3 本とも PASS（2026-11-27 08:53 UTC）

# settle（final review の指摘 R13〜R22。commit 1771c6f2f）

## R13（fixed 5.1）判定の step が落ちても run が緑
- `number.yml` の run の step すべて（decide・complete・build-linux の 2 つ・place の 2 つ）に `shell: bash` を書いた（GitHub は明示の `bash` を `bash --noprofile --norc -eo pipefail {0}` で走らせる）
- `check-guards.sh` の `number_steps`: run を持つ step は、step の `shell`・job の `defaults.run.shell`・workflow の `defaults.run.shell` のどれかで shell を持たなければ「run の shell が bash と書かれていない」で落とす
- `--self-test` に偽の number.yml 3 つ: `no-shell-decide`・`no-shell-complete`・`no-shell-place`（shell: bash の行を消す）。手で走らせて `no-shell-decide` は `NG: … number の 2 番目の step: run の shell が bash と書かれていない（書かないと pipefail なしで走る）`
## R14（fixed 3.1）changes.sh の rc=1 以外を「文書だけ」と読む
- `version.sh number`: rc を取り、1 だけを「文書だけ」、0 以外の他は `die "製品に触れるかの判定（changes.sh code）が落ちた: rc=<n>"`
- 自己試験に `== Scenario: 製品に触れるかの判定が落ちると、番号を付けずに止まる`（changes.sh を `exit 2` の偽にして rc=2、消して rc=127。どちらも rc≠0・番号なし）
- 壊して: `(( code_rc == 1 ))` を `(( code_rc != 0 ))` に戻すと `version.sh --self-test` が rc=1
## R15（fixed 4.1）一時ファイルが残る
- `number.sh place`: RETURN の trap をやめ、`gh release view` の stderr を読んだ直後に `rm -f`（die の前に消える）
- 自己試験の TMPDIR を `$work/tmp` にし、最後に `== Scenario: 番号付けの台本は、止まっても一時ファイルを残さない`（空でなければ落ちる）
- 壊して: `rm -f "$err"` を外すと `number-gh-err.*` が 3 つ残り `number.sh --self-test` rc=1。直した版は rc=0（走らせる前後で `/tmp/number-gh-err.*` の数は 16 のまま）
## R16（fixed 2.2）本番の apk_version の試験の固定値
- `check-artifacts.sh apk_version` の `0.4.0-beta.2 → 40000` の特例と注釈を消した（D3 の固定値は `version.sh --self-test` の「versionCode は版番号の 3 つの数字から決まる」が持つ）
## R17（fixed 4.2）公開の置き場の署名していない APK
- `check_artifacts`: `--allow-debug-key` の無い検査では、置き場に `*-unsigned.apk` があれば「署名していない APK は下書きの置き場にだけ置く」で落とす
- 自己試験（実物の release の置き場）に `== Scenario: 署名していない APK は、公開する置き場にあれば検査が落ちる`（正しい指紋でも落ち、文を出す）
## R18（fixed 5.1）place の persist-credentials
- place の checkout を `persist-credentials: false`（`number.sh place` は gh と GH_TOKEN だけ。`git ls-remote` は公開の repo なので token 無しで読める）
- `number_workflow` に「place が `persist-credentials: false`」の検査、`--self-test` に偽 `place-persist`（true に戻す）→ `NG: 置く job が書く権限の token を作業木に残す`
## R19（fixed 1.1）表の左 1 の行
- `version-check-table.tsv` に `0.0.1 0.0.1 1 ok`・`0.2.0 0.0.1 1 ng` を足した。bash（version.sh --self-test）・Rust（`cargo test -p ashiato-build-stamp` の st61_version_check_table_rows_agree ok、16 passed）・TS（vitest build-stamp 14 passed）・gradle（`:app:verifyVersionCheckTable --rerun-tasks` BUILD SUCCESSFUL）が同じ表で通る
## R20（fixed 2.2）gradle の表の読み取りと誤りの文
- 3 列目を `toLongOrNull` で `runCatching` の外で読み、数字でなければ「3 列目が数字でない」で落とす。壊して: 表に `0.0.2 0.0.1 x ng` を足すと `version-check-table.tsv の 3 列目が数字でない` で落ちた（戻した）
- `checkBuildVersion` の形・上限の誤りに出所と MAJOR を付けた: `ASHIATO_BUILD_VERSION=0.4` で `版番号の形が違う: 0.4: 左の数字のファイル 0、出所 0.0.1、渡した版番号 0.4`
## R21（fixed 5.1）if の || 
- `number_workflow` の `if:` を 3 つを `&&` で結んだ 1 行と完全一致で見る。`--self-test` に偽 `or-if`（1 つ目の `&&` を `||`）→ `NG: 番号付けが「この repo の枝の、push の、成功した run」だけで起きない`
## R22（fixed 7.1）更新の試験の版
- 更新の試験の版を項目の名前に依らず版の形の値で拾い、`0.0.[1-9]*` に限る（配る形の job に渡す版も同じ）
- `--self-test` に偽の ci.yml 2 つ（`0.4.0:extra`・`0.0.0:zero` を足す）→ それぞれ `0.0.<N≥1> でない版を名乗る: 0.4.0` / `: 0.0.0`

## 走らせたもの
- `scripts/verify-run 5.1`（check-guards --self-test・workflows・actionlint）・`1.1`・`3.1`・`4.1`・`7.1`: すべて PASS（evidence.jsonl、head 1771c6f2f）
- `tools/release/check-artifacts.sh --self-test`（quiet-run、commit 後）rc=0。`scripts/verify-run 2.2`・`4.2` の結果は下に追記
- `scripts/verify-run 2.2`・`4.2`: rc=0、各コマンド PASS（evidence.jsonl、head 1771c6f2f）
