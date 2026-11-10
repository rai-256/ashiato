# CH11 —— 依存の脆弱性を CI で検査する

change: `openspec/changes/ch11-dependency-vulnerability-scan/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）:
- D2（仮）. 重大度を問わず落とす。知らせ（健全でない・保守されていない）も含める（Q5・C10）
- D3（仮）. 製品に入らない依存も区別せず落とす（Q4）
- D4（仮）. 例外の行の期限は「検査した日（UTC）から 90 日以内」で見る（Q7）
- D5（仮）. 公開の場に書くのは、依存・版・勧告の ID・重大度と、理由の短い文だけ（Q10）
- D9（仮）. 下流の AI は例外の行を足してよい。足した・変えた行は PR の検査が一覧で出す（Q9）
- D10（仮）. dependency-review の許可は、例外の表から CI が組む（Q6）
- D11（仮）. 携帯の依存の固定は、全部の構成とプラグインの classpath を STRICT で（Q12）
- D14（仮）. PostgreSQL の「最新」は postgresql.org の一覧。EDB にあるか、公開から 30 日で落とす（Q3）
- D15（仮）. この change では PostgreSQL を上げない（Q2）
- D16（仮）. 印 `v*` の組み立ては、全体の検査が通るまで始めない。開発版の公開は止めない（Q8）
- D17（仮）. GitHub の設定は下流の AI が `gh` で有効にし、API で読み返す（Q11）
- D18（仮）. いまある勧告は、互換の範囲で上げ、残りは例外の表へ（Q1）
- D20（仮）（C3）. PR の検査の条件は `tools/release/changes.sh deps` が正本。workflow だけの変更では走らせない

全 Task の ledger の `Ruling:`（逐語）:
- - Ruling: Task 6.1（`tools/check-vulns.sh github-settings`。本物の repo `rai-256/ashiato` の Dependabot alerts と依存グラフを `PUT` で有効にして読み返す）は下流で走らせていない。台本は入っている（`tools/check-vulns.sh:43` `github_settings`）が、本物の repo の設定を変える外向きの操作なので、処置の席（settle）は走らせない（review/code.md R10）。有効にする手と読み返しは「人間の確認待ち」の 1 つ目（tasks.md の「Task 6.1 を下流が済ませられなかったとき」）へ移した。Task 10 の dependency-review の手順は、人間が有効にした後の PR の CI（`vulns` の job が success）で確かめる。2026-11-09 の時点で `gh api repos/rai-256/ashiato --jq .permissions.admin` は true（権限が無いのではなく、走らせていないだけ）。
- - Ruling: **この PR の `vulns` の job は、依存グラフと Dependabot alerts を有効にするまで赤になる**（2026-11-09 の読むだけの API で `vulnerability-alerts` 404・`dependency-graph/sbom` 404。この PR は lock と例外の表を変えるので `deps=true` で dependency-review が走る。review/code.md R34）。merge の前に、設定画面か管理者の `gh` で有効にし `tools/check-vulns.sh github-settings` が rc=0 になってから、`vulns` の job を走らせ直す。順序は tasks.md の「merge の前の前提」。
- - Ruling: 例外の表で足した行（期限 2027-02-06。R35）: 理由 `届かない: 携帯の組み立ての道具専用` の 9 行 —— GHSA-2363-cqg2-863c・GHSA-3677-xxcr-wjqv・GHSA-4265-ccf5-phj5・GHSA-4g9r-vxhx-9pgx・GHSA-ccrj-gw8r-854m・GHSA-cj8j-37rh-8475・GHSA-r937-wjx7-w2jp・GHSA-rcgg-9c38-7xpx・GHSA-wg6q-6289-32hp
- - Ruling: 例外の表で足した行（期限 2027-02-06。R35）: 理由 `届かない: 携帯の組み立ての道具と計測テストの実行基盤の試験専用` の 37 行 —— GHSA-389x-839f-4rhx・GHSA-38f8-5428-x5cv・GHSA-3p8m-j85q-pgmj・GHSA-3qp7-7mw8-wx86・GHSA-45q3-82m4-75jr・GHSA-4g8c-wm8x-jfhw・GHSA-4mp9-239f-g9hg・GHSA-558v-64gr-wgg4・GHSA-563q-j3cm-6jxm・GHSA-57rv-r2g8-2cj3・GHSA-5x3r-wrvg-rp6q・GHSA-6cqp-g7gg-8hr5・GHSA-6jqx-86gh-f27w・GHSA-84h7-rjj3-6jx4・GHSA-8c42-7qj2-3j46・GHSA-93wv-jw9v-4972・GHSA-c2gf-v879-257j・GHSA-c4c3-7fpv-j4q5・GHSA-c653-97m9-rcg9・GHSA-c69g-56f8-xwqj・GHSA-f6hv-jmp6-3vwv・GHSA-fccg-mwvh-qqg4・GHSA-fghv-69vj-qj49・GHSA-gcjf-9mgh-3p7g・GHSA-hvcg-qmg6-jm4c・GHSA-jppx-w49h-x2qq・GHSA-m4cv-j2px-7723・GHSA-mj4r-2hfc-f8p6・GHSA-mvh2-crg5-v77c・GHSA-prj3-ccx8-p6x4・GHSA-pwqr-wmgm-9rr8・GHSA-q4f6-jm68-57ww・GHSA-v8h7-rr48-vmmv・GHSA-w9fj-cfpg-grvv・GHSA-x4gw-5cx5-pgmh・GHSA-xq3w-v528-46rv・GHSA-xxqh-mfjm-7mv9
- - Ruling: 例外の表で足した行（期限 2027-02-06。R35）: 理由 `届かない: 携帯の組み立ての道具と単体試験の試験専用` の 5 行 —— GHSA-574f-3g2m-x479・GHSA-9pwp-9qqc-pr26・GHSA-c3fc-8qff-9hwx・GHSA-p93r-85wp-75v3・GHSA-qp49-qgx5-5m26
- - Ruling: 例外の表で足した行（期限 2027-02-06。R35）: 理由 `届かない: 画面の開発用依存（試験と組み立て）専用` の 5 行 —— GHSA-5gmw-xhrv-c9v3・GHSA-82fw-gwwq-j7x9・GHSA-85c8-ppgw-ccpr・GHSA-g3hv-75wg-m249・GHSA-qxg6-89mg-rqv8
- - Ruling: 例外の表で足した行（期限 2027-02-06。R35）: 理由 `届かない: 携帯の計測テストの実行基盤の試験専用` の 4 行 —— GHSA-5jpm-x58v-624v・GHSA-6mjq-h674-j845・GHSA-735f-pc8j-v9w8・GHSA-xpw8-rcwv-8f8p
- - Ruling: 例外の表で足した行（期限 2027-02-06。R35）: 理由 `上げる予定: 推移的依存の修正版待ち` の 2 行 —— GHSA-5mg8-w23w-74h3・GHSA-7g45-4rm6-3mm3
- - Ruling: 例外の表で足した行（期限 2027-02-06。R35）: 理由 `上げる予定: 互換範囲外の修正版待ち` の 8 行 —— GHSA-8hxf-qr8p-62x4・GHSA-m665-v7pm-7grj・GHSA-q7v9-x93c-xqv3・GHSA-xxph-c9ww-hj94・RUSTSEC-2026-0367・RUSTSEC-2026-0368・RUSTSEC-2026-0369・RUSTSEC-2026-0429
- - Ruling: 例外の表で足した行（期限 2027-02-06。R35）: 理由 `上げる予定: 修正版公開後に互換範囲内へ更新` の 1 行 —— RUSTSEC-2023-0071
- 全 Task の ledger の `Ruling:`（逐語）:

## Task

11 Task のうち、埋めた Task は 10 本（Task 1, Task 2, Task 3, Task 4, Task 5, Task 7, Task 8, Task 9, Task 10, Task 11）。

残した項目:
- Task 6 の 6.1（PR ができてから走る検証（gate が持つ））: 6.1 `tools/check-vulns.sh github-settings` を design D17 のとおりに足す（`echo "== Scenario: 設定を API で読み返す"`。4 つの結果を 1 行ずつ出す。管理者の権限が無い `gh` では rc=2 で「管理者の権限が要る」と出す）。

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.1 `bash -n tools/check-vulns.sh` → rc=0（PASS・head 021e1ce）
- 1.2 `tools/check-vulns.sh --self-test` → rc=0（PASS・head 021e1ce）
- 2.1 `tools/check-vulns.sh --self-test` → rc=0（PASS・head b81ed5b）
- 3.1 `tools/check-vulns.sh --self-test` → rc=0（PASS・head 88a7303）
- 3.2 `tools/check-vulns.sh --self-test` → rc=0（PASS・head 88a7303）
- 4.1 `tools/check-vulns.sh --self-test` → rc=0（PASS・head 756d19b）
- 5.1 `bash -n tools/check-gradle-locks.sh` → rc=0（PASS・head 22020a7）
- 5.2 `scripts/quiet-run ch11-gradle-locks -- tools/check-gradle-locks.sh` → rc=0（PASS・head 22020a7）
- 5.2 `scripts/quiet-run ch11-gradle-locks-self -- tools/check-gradle-locks.sh --self-test` → rc=0（PASS・head 22020a7）
- 5.2 `scripts/quiet-run ch11-gradle-strict -- ./collector-android/gradlew -p collector-android detekt :app:assembleDebug :app:assembleDebugAndroidTest :app:testDebugUnitTest` → rc=0（PASS・head 22020a7）
- 5.3 `scripts/quiet-run ch11-same-release -- bash -c 'c="$(git log -1 --format=%H --diff-filter=A -- collector-android/app/gradle.lockfile)"; tools/check-gradle-locks.sh --same-release-as "$c~1" "$c"'` → rc=0（PASS・head 22020a7）
- 5.4 `scripts/quiet-run ch11-licenses-self -- tools/check-licenses.sh --self-test` → rc=0（PASS・head 22020a7）
- 5.4 `scripts/quiet-run ch11-licenses-android -- tools/check-licenses.sh android` → rc=0（PASS・head 22020a7）
- 7.1 `tools/check-vulns.sh` → rc=0（PASS・head 889cea2）
- 7.1 `bash -c 'tools/check-vulns.sh --base "$(git merge-base HEAD origin/main)"'` → rc=0（PASS・head 889cea2）
- 7.2 `scripts/quiet-run ch11-cargo-test -- bash -c './tools/db.sh up -d --wait db && ./tools/db-roles.sh && ./tools/tmp-leak-check.sh rust'` → rc=0（PASS・head 9511c8a）
- 7.2 `bash -c 'cd web && npm ci && npm test -- --run'` → rc=0（PASS・head 9511c8a）
- 7.2 `scripts/quiet-run ch11-android-unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest` → rc=0（PASS・head 9511c8a）
- 8.1 `tools/check-pg-minor.sh --self-test` → rc=0（PASS・head 8de9e83）
- 8.1 `bash -c 'tools/check-pg-minor.sh; test $? -eq 1'` → rc=0（PASS・head 8de9e83）
- 9.1 `tools/check-vulns.sh --self-test` → rc=0（PASS・head 530107f）
- 9.2 `bash -c 'git diff --quiet origin/main -- .github/workflows/nightly.yml'` → rc=0（PASS・head 530107f）
- 10.1 `bash -c 'grep -q "changes.sh deps" .github/workflows/ci.yml'` → rc=0（PASS・head cde4ee1）
- 10.2 `bash -c 'grep -q "tools/check-vulns.sh" .github/workflows/release.yml'` → rc=0（PASS・head cde4ee1）
- 10.3 `tools/check-vulns-wiring.sh` → rc=0（PASS・head cde4ee1）
- 10.3 `tools/check-vulns-wiring.sh --self-test` → rc=0（PASS・head cde4ee1）
- 11.1 `bash -c 'grep -q "check-vulns.sh" AGENTS.md && grep -q "check-vulns.sh" docs/testing.md'` → rc=0（PASS・head 4d07977）
- 11.2 `python3 scripts/check_scenarios.py . ch11-dependency-vulnerability-scan` → rc=0（PASS・head 4d07977）

## レビュー（`review/code.md`）

- 指摘 35 件（fixed 34・rejected 1）
- 処置の re-review: 3 回。最後の判定が NOT ADDRESSED のもの: なし

## 未処置の申し送り

- 例外の表の全行が 2027-02-06 に同時に期限切れになる。その 2〜3 週間前に見直しの change を立てる（`docs/handoff/CH11.md`。review/code.md で触れていない）
