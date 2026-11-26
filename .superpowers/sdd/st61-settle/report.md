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
