# ST53 コードレビュー

## final review（ac244280..1ba70a3f）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）がブランチ全体の差分（review package `.superpowers/sdd/tasks/review-ac244280..1ba70a3f.diff`、41 commit）と全 Task の指摘の記録（Minor と ⚠️）を見た。判定は「No（直せば可）」。

## R1. HEAD で `tools/check-spdx.sh` が rc=1 で落ちる（空白を含むパスを `awk` で切っている）。自己試験は緑のまま
- 成果物: tools/check-spdx.sh:43
- 根拠: `git ls-files -s | awk '{print $1 "\t" $4}'` は空白で区切るので、Task 5 が足した `tools/licenses/texts/Android Software Development Kit License.txt` が `.../texts/Android` で切れ、`head` が落ちる。pipefail と set -e の下で NG の文を出さずに rc=1。再現: `tools/check-spdx.sh` → 例外 5 行の後に `head: cannot open '.../tools/licenses/texts/Android'`、rc=1（2026-11-04、この final で確かめた）。自己試験は `output="$(check_headers "$root")"` の中で errexit が引き継がれないので失敗を飲み込み、「今の木で rc=0」の Scenario が rc を見ていない。最後の PASS の証跡はこのファイルができる前（eaba9a0c）。T2 F4 の未処置
- kind: technical
- 提案: `git ls-files -s -z` と `read -r -d ''`（またはタブで割る）でパスを読む。自己試験の今の木の検査は `$(…)` で包まずに rc を確かめる（`check_headers "$root" >out || return 1`）
- 処置: fixed 2.2 （80729cd5: `git ls-files -s -z` で NUL 区切りに読み、今の木の検査は別プロセスで rc を見る。HEAD で `tools/check-spdx.sh` rc=0 を確かめた）

## R2. APK の THIRD-PARTY-NOTICES に雛形・自作の要約の許諾の文が入る
- 成果物: tools/licenses/texts/BSD-3-Clause.txt / tools/licenses/texts/Android Software Development Kit License.txt / tools/release/notices.sh:198-212
- 根拠: `androidx.health.connect:connect-client-external-protobuf` の項が `Copyright (c) [copyright holder]` のまま入る（BSD-3 の条件の著作権の表示が欠ける）。spec（distribution）の「部品ごとに許諾の全文と著作権の表示を書く」「IF 全文も著作権の表示も見つからない THEN 組み立てを落とす」と、design D8 の「無ければ SPDX の全文に作者を添える。どちらも無ければ落とす」を携帯の側で満たしていない。gms の項の `Android Software Development Kit License.txt` は 6 行の自作の要約で、許諾の本文でも出所のある表示でもない（T5 F12 の未処置）。aar は最上位の `META-INF/` だけを読み、`classes.jar` の中の LICENSE / NOTICE を拾わない。最初の印の後では配った文を直せない
- kind: technical
- 提案: aar の `classes.jar` の中の `META-INF/LICENSE*|NOTICE*` も読む。それでも無いときの著作権の表示は POM（developers / organization）から機械的に取り、取れなければ落とす（雛形の `[copyright holder]` が残る本文は落とす）。PROPRIETARY の項は自作の要約をやめ、許諾の名前と POM の許諾の URL を出所つきで書く（法的な文を作文しない）
- 処置: fixed 5.2 （f94f27cd: classes.jar の中まで許諾のファイルを読み、無ければ SPDX の全文に POM（親も）の作者を出所つきで添え、雛形の残る本文は落とす。gms の自作の要約を消し、許諾の URL を出所つきで書く）

## R3. Windows の組み立て（`package-windows`）が一度も走っておらず、notices.sh の Windows の経路が確かめられていない
- 成果物: tools/release/notices.sh / tools/release/build.sh（`archive_windows`）/ tools/release/check-artifacts.sh
- 根拠: 証跡は Linux のサーバと APK だけで、このブランチの CI の run も無い。Windows の job で `python3` を使うのは notices.sh が初めて。Windows の Python の既定の符号化（cp1252）で UTF-8 の許諾の本文を読み書きする箇所、バックスラッシュの `manifest_path` で `dirname` が `.` に化けて全部の部品に repo の LICENSE が入る経路が確かめられていない。`check-artifacts.sh` は Windows の zip の THIRD-PARTY-NOTICES について `== rust-std` の有無しか見ないので、この化け方を捕まえない
- kind: technical
- 提案: Python の読み書きに `encoding='utf-8'` を明示（または `PYTHONUTF8=1`）。`manifest_path` を正規化する。`check-artifacts.sh` で、THIRD-PARTY-NOTICES の部品の本文がすべて repo の `LICENSE` と同じでないことを見る。PR の CI の `package-windows` が緑になることを merge の条件として PR 本文に書く
- 処置: fixed 5.1 （f94f27cd が Python の入出力を UTF-8 にしバックスラッシュの置き場を正規化、e1d7803f が「部品の本文がすべて木の LICENSE」の一覧を check-artifacts で落とす）。Windows の 2 つの組み立てはこの手元で走らせられないので、PR の CI の `package-windows` と `package-artifacts` が緑になることを merge の条件として PR 本文に書く

## R4. docs/release.md の組み替えで ST52 の手順が消え、申し送り C2（FR-108 が指す §7）が満たされていない
- 成果物: docs/release.md §6・§7・§8
- 根拠: 旧版（`git show ac244280:docs/release.md`）と比べると、最初の印の手順の列（`git tag v<版> <commit>` → `version.sh plan` → `git push origin v<版>` → workflow の順の確認）、修正の枝のコードの塊と「修正の枝からベータ版は出さない」「`migrations/ORDER` と `envelope.json` を変えない」、`sign-apk.sh … dev` と `ASHIATO_EXPECT_DEV_CERT_SHA256` の注記、nightly の公開の順、§7 の「起動後に確認し、異常なら旧版へ戻す」が消えた。「本人の環境を移す」は `###` に格下げされ、`docs/requirements.md:1004`（FR-108）は今も「`docs/release.md` §7」を指す（`docs/handoff/ST53.md` の C2）。§6 の `"$dir"` は定義されていない（T10 F7）。§8 末尾の「Q2 の判断へ戻す」は tasks.md 11.2 の Q3 と食い違う
- kind: technical
- 提案: 消えた手順を戻す。「本人の環境を移す」を `## 7.` に戻し、冒頭の ★ 2026-11-03 の段落を残す（feat の枝から要件は書き換えない）。`"$dir"` を定義し、Q 番号を合わせる
- 処置: fixed 10.2 （6e0a62ad: 消えた ST52 の手順を戻し、「本人の環境を移す」を §7 に戻し、`"$dir"` を定義し、Q 番号を 11.2 の Q3 に合わせた）

## R5. 確認バッチの準備がエミュレータ 1 台への導入の失敗で全体ごと止まり、手順書から APK の場所が消える
- 成果物: tools/verify-prep.sh:11・48・62
- 根拠: `android_note="$(install_on_emulators …)"` が set -e の下にあり、導入の失敗 1 つで準備全体が止まる（以前は手順書に注記して続けていた）。注記がエミュレータ名だけになり APK の場所が消える（T9 ⚠️）。冒頭の説明「実機が繋がっていれば APK も入れる」（:11）と、読まれない `device=""`（:48）が残る
- kind: technical
- 提案: 失敗は注記にして続ける。注記に APK のパスを入れる。古い説明と使われない変数を消す
- 処置: fixed 9.1 （d6b95dad: 導入の失敗は注記にして続け、注記に APK のパスを入れ、古い説明と使われない `device` を消した）

## R6. `tools/verify-prep-device.sh` の自己試験の `[ "$output" != *emulator-5554* ]` が常に真
- 成果物: tools/verify-prep-device.sh:66
- 根拠: `[ ]` の中ではグロブが効かない（T9 F6 の未処置）。自己試験が `tools/verify-prep.sh` を相対パスで読む（T9 F5）
- kind: technical
- 提案: `[[ ]]` に直し、パスはスクリプトの場所から組む
- 処置: fixed 9.1 （d6b95dad: `[[ ]]` に直し、verify-prep.sh をスクリプトの場所から読む）

## R7. `tools/release/check-guards.sh` は approvals の行が無いと NG の文を出さずに終わる
- 成果物: tools/release/check-guards.sh:133
- 根拠: `grep | head | cut` が pipefail で落ち、理由を示さずに非 0 で終わる（T8 F3）
- kind: technical
- 提案: 見つからないときに NG の文を出して落とす
- 処置: fixed 8.2 （5ab79383: 行が無いときも既存の NG の文で落ち、自己試験がその文を見る）

## R8. README の参考訳が追加の許可の条件と主語を落としている
- 成果物: README.md
- 根拠: 参考訳から「Android SDK のライセンスの条項の対象となる部分を含む」と、許可を与える主語（本プログラムのライセンサー）が抜けている（T1 F6）。CONTRIBUTING の `## ライセンス` の「本体は AGPL-3.0-only」は、1.1 の検証が見出しの後ろの書き換えを禁じた計画の側の食い違い
- kind: technical
- 提案: 参考訳を正本（`LICENSES/AdditionRef-ashiato-play-services.txt`）どおりに直す。CONTRIBUTING は 1.1 の検証を壊さない範囲で扱う
- 処置: fixed 1.1 （829c94fb: 参考訳を正本の条件と主語どおりにした。CONTRIBUTING の `## ライセンス` は 1.1 の検証の範囲なので触らない）

## R9. tasks.md の 8.1 が `- [ ]` のまま
- 成果物: openspec/changes/st53-license-bundle/tasks.md
- 根拠: 実装は済み、8.1 の証跡は PASS（d430b0a8）、HEAD の自己試験も PASS
- kind: technical
- 処置: rejected: 8.1 は preflight が「PR・gate」の工程に分けた行で、`[ ]` で残るのが正常（`docs/briefs/ST53-preflight.md` の残りタスク）。`[x]` はグラフが付け、実装の側は tasks.md を編集しない

## R10. 10.1〜10.3 の証跡が a820435d より前（6139fc84）のもの
- 成果物: openspec/changes/st53-license-bundle/evidence.jsonl
- 根拠: a820435d が ci.yml に `check-licenses.sh --self-test` を足した後に取り直していない。final reviewer が HEAD で `check-guards.sh workflows` を走らせ PASS を確かめた
- kind: technical
- 処置: rejected: 申告と証跡のずれを見るのは code-verify の席（グラフの `code_verify`）。HEAD での PASS は final reviewer が確かめた

## R11. Task の古い Minor が残る（どれも merge は止めない）
- 成果物: 全 Task（`.superpowers/sdd/st53-task-*/task-*-findings.md`）
- 根拠: T3 F1〜F4、T4 F6〜F8、T5 F9〜F11・F13、T6 F7〜F9（`$TMPDIR/ashiato-kotlin-daemon` を消さない）、T7 F7（使われない `license_text`）、T10 F5・F6
- kind: technical
- 処置: fixed 3.1 （720285b0 が T3 F1・F2、T4 F6、T5 F9・F11、T6 F9、T10 F5 を、e1d7803f が T7 F7 を直した。T5 F13 は R24 の処置で直った）。残りの T3 F3・F4、T4 F7・F8、T5 F10、T6 F7・F8、T10 F6 は merge を止めない品質の Minor で、fix は 1 回だけの決まりにより park（PR 本文の「park した Minor」に Ruling つきで写す）

## code_verify（HEAD 720285b0。独立検証 2026-11-04）

申告: tasks.md 18/19 `[x]`（1.1〜10.3。8.1 は `[ ]` のままで、preflight の工程分けによる正常（R9 の処置のとおり））。検証コマンドは各 Task の本文。evidence.jsonl の最後の記録は 6139fc84（HEAD の 11 commit 前。final の後の fix 8 本の後に取り直していない。R10）。ここで全部を取り直した。

**作業ツリーは HEAD と同じではない。** `git status --porcelain` = ` M tools/release/notices.sh`（`git diff --stat`: 17 行足し・8 行消し。mtime 01:34 で、HEAD 720285b0 の commit は 01:28）。この席はこの差分に触らず、commit もしていない。検証の前と後で `git status --porcelain` は同じ。下の表は作業ツリーで走らせた結果で、HEAD と食い違うものは別の列に書いた。

| 項目 | 検証コマンド（tasks.md の本文どおり） | 実測（作業ツリー） | 実測（HEAD だけ。違うもの） | 一致 |
|---|---|---|---|---|
| 1.1 | `head -n N LICENSE \| cmp` / README・CONTRIBUTING の grep / `git diff --quiet origin/main -- docs/CLA.md` / `diff <(origin/main:CONTRIBUTING.md の最初の見出しから後ろ) <(今の同じ範囲)` | rc=0 / rc=0 / rc=0 / **rc=1**（38 行目 `../tools/check-licenses.sh` → `… rust node`） | — | **不一致**（R14） |
| 1.2 | `check-spdx.sh --self-test` / `check-spdx.sh license` | rc=0 / rc=0 | — | 一致 |
| 2.1 | `check-spdx.sh headers` / 古い表示 0 件 / `check-openapi.sh` / `cargo metadata` の license の集合 | rc=0（例外 5 行）/ rc=0 / rc=0「API の契約 OK」/ rc=0 | — | 一致（`npm ci && npm run build` は単独では走らせていない。6.1a の `build.sh server-linux` が `npm ci --ignore-scripts` と `npm run build` を呼び（build.sh:26）、rc=0） |
| 2.2 | `check-spdx.sh --self-test` / `check-spdx.sh` | rc=0 / rc=0 | — | 一致 |
| 3.1 | `check-licenses.sh --self-test` / `cd web && ../tools/check-licenses.sh rust node` / `ALLOW` に AGPL が無い | rc=0「self-test OK」/ rc=0（Node 257 件・不許可 0）/ rc=0 | — | 一致（`npm ci` は省いた。node_modules は既にある） |
| 4.1 | 取り寄せ → `--offline` → `cmp` → location あり → 部品 ≥50 | rc=0 | — | 一致 |
| 4.2 | `--self-test` / 2 つの Scenario の印がちょうど 2 | rc=0 / rc=0 | — | 一致 |
| 4.3 | `check-licenses.sh android` で `通した PROPRIETARY: com.google.android.gms:` がちょうど 4 / self-test の 2 つの印 | rc=0（55 件・不許可 0）/ rc=0（54 秒） | — | 一致 |
| 5.1 | `notices.sh collector-windows … --target x86_64-pc-windows-gnu` と rust-std・≥20 項 | rc=0 | — | 一致 |
| 5.2 | `notices.sh apk "$o"` と location・`(gms の中の第三者)` | rc=0（55 項） | **rc=1**「許諾の全文も著作権の表示も無い部品: com.google.code.findbugs:jsr305:3.0.2」 | **HEAD で不一致**（R12） |
| 5.3 | `notices.sh --self-test` / 3 つの印がちょうど 3 | rc=0 / rc=0 | `--self-test` は HEAD でも rc=0 | 一致 |
| 6.1 | `build.sh server-linux` の tar の根 / `build.sh apk` の `assets/licenses/` が 3 / release.yml の `ASHIATO_SOURCE_REF` が 4 / `ASHIATO_SOURCE_COMMIT` 無し / `check-guards.sh workflows` / Scenario の印 1 | rc=0（57 秒）/ rc=0（79 秒）/ rc=0 / rc=0 / rc=0 / rc=0 | `build.sh apk` は `set -e` の下で 5.2 の `notices.sh apk` を呼ぶ（build.sh:54）ので HEAD では落ちる | **HEAD で不一致**（R12） |
| 7.1 | `check-artifacts.sh --self-test` / 9 つの印がちょうど 9 | rc=0（510 秒。本物の server-linux と APK を組む）/ rc=0 | 同上（自己試験が `build.sh apk` を呼ぶ。check-artifacts.sh:314） | **HEAD で不一致**（R12） |
| 8.1（`[ ]`） | `check-guards.sh --self-test` | rc=0 | — | （申告外。PASS） |
| 8.2 | `--self-test` / `workflows` / actionlint | rc=0 / rc=0 / rc=0（`actionlint` が無く、docker の `rhysd/actionlint:1.7.7` の側で通った） | — | 一致 |
| 9.1 | `verify-prep-device.sh --self-test` / `bash -n verify-prep.sh` / `adb`・`install` の字が無い | rc=0 / rc=0 / rc=0 | — | 一致 |
| 10.1 | actionlint（ci.yml）/ 5 つの呼び出しの grep / `check-guards.sh workflows` | rc=0（docker の側）/ rc=0 / rc=0 | — | 一致 |
| 10.2 | 見出しの grep 8 つ / 資格情報 → 鍵 / 承認者 → 鍵 | rc=0 / rc=0 / rc=0 | — | 一致 |
| 10.3 | 4 つの grep / `check_scenarios.py . st53-license-bundle` | rc=0 / rc=0「scenarios: OK」 | — | 一致 |

まとめ: 作業ツリーでは 1.1 の 1 本を除いて全部 rc=0。HEAD 720285b0 のままだと 5.2・6.1（APK）・7.1 が落ちる。CI の `package-linux` の job が落ちる状態で、通るのは commit されていない差分があるときだけ。

- 手 1（固定値の再計算）: 不一致なし。`check-spdx.sh` の `expected_agpl_sha256`（0d96a4ff…abcb0）は、python の `hashlib.sha256` で `git show origin/main:LICENSE`（spec の「今の `LICENSE` の全文」）を計算した値と、今の `LICENSE` の `tail -n +$((n+3))` を計算した値の両方と一致した。台本の `expected_text` と正本の `Listed libraries:` より前は、python で spec の Requirement のコードの塊と突き合わせて、どちらも 1 字も違わなかった（`True` / `True`）。gnu.org の原本との突き合わせは、取り寄せが時間切れ（curl rc=28）でできなかった
- 手 2（ガードを壊す）: 使い捨ての写し（`git archive HEAD` を展開した木）でだけ壊し、作業ツリーには触れていない。次は意図どおり落ちた —— `LICENSE` の AGPL の 31 行目を 1 語変える → `check-spdx.sh license` rc=1「AGPL の全文が違う」/ `LICENSE` の末尾に 1 行足す → rc=1 / `// SPDX-License-Identifier: MIT` の `crates/x.rs` を add する → `headers` rc=1 / 根の `Cargo.toml` の `license` を `AGPL-3.0-only` にする → `check-licenses.sh rust` rc=1（`NG ashiato-server: AGPL-3.0-only`、不許可 3 件）/ `verify-prep-device.sh` の `emulator-` の条件を外す → `--self-test` rc=1。落ちなかったものは R15〜R17
- 手 3（Scenario と test）: `check_scenarios.py` は rc=0（51 本すべて担保あり）。確かめた範囲: `check-spdx.sh` と `verify-prep-device.sh` の自己試験は全部、`check-artifacts.sh`・`check-guards.sh`・`notices.sh` は該当の段を読んだ。`check-licenses.sh` の自己試験の本文は、許可の一覧の段（:245）のほかは読んでいない（rc=0 と印が出ることだけを見た）。主張の階層がずれていたのは「4 つの配布物は…」（R16）と「公開の job は、署名の前に承認者を確かめる」（R15）の 2 本
- 手 4（本人の決定の固定）: 自己試験のコードを読んで、値を変えると落ちる段があることを確かめた —— Q2 の名指しの 4 つ（check-spdx.sh の `check_license`、4.3 の「ちょうど 4」）・Q5 の式・Q10/Q11 の例外 5 つ（`grep -c '^  例外:'` が 5）・Q16/Q17 の `ref` と `built/`（check-guards の 5 つの偽の ci.yml）・許可の 17 の値（check-licenses.sh:245 が文字列ごと比べる）。Q7 の 40 桁・Q12 の AND は自己試験の印が出て rc=0 であることだけを見た。実際に値を変えて走らせたのは Q11 の「先頭 25 行」だけで、これは下にしか固定されていない —— `head -n 25` を 1・5 にすると `--self-test` は rc=1、1000 にすると `--self-test` も `headers` も rc=0（R18）
- 手 5（`[x]` と実体）: 表のとおり。`cargo test --test` 型の検証はこの change に無い。不一致は 1.1（R14）と、HEAD だけで見たときの 5.2・6.1・7.1（R12）
- 手 6（隙間）: 取り消せないものは、配った版に入る許諾の文と依存の一覧（R12・R13・R16）。`tools/verify-prep.sh:62-66` は、導入の失敗を注記にして準備を続ける形になっていた（R5 は直っている）。Windows の 2 つの組み立ては、この枝の PR がまだ無く（`gh pr list --head feat/st53-license-bundle --state all` → `[]`）、一度も走っていない（R3 のまま）
- 手 7: 該当なし（`plan_fix.py pending openspec/changes/st53-license-bundle` → `[]`）

## R12. HEAD 720285b0 では携帯の APK の依存の一覧が組めない（jsr305 で落ちる）。通るのは commit されていない notices.sh の差分があるときだけ
- 成果物: tools/release/notices.sh（f94f27cd が足した著作権の表示の必須）/ tools/release/build.sh:54 / tools/release/check-artifacts.sh:314
- 根拠: 4.1 の出力（`$TMPDIR/poms.tsv`。55 部品）を `ASHIATO_LICENSES_POM_LIST` で渡すと、`git archive HEAD` の写しの `tools/release/notices.sh apk` は rc=1「許諾の全文も著作権の表示も無い部品: com.google.code.findbugs:jsr305:3.0.2（部品の中に許諾のファイルが無く、POM と親の POM に developers / organization が無い）」、作業ツリーの版は rc=0（55 項）。HEAD の `notices.sh --self-test` は rc=0 —— 自己試験は偽の部品だけを見ていて、しかも HEAD の段「許諾のファイルも POM の作者も無い携帯の部品があれば、組み立てが落ちる」は、jsr305 と同じ形（Apache-2.0・許諾のファイル無し・作者無し）の部品で落ちることを正として固定している。本物の依存では、その振る舞いで組み立てが落ちる。f94f27cd の後に 5.2・6.1・7.1 の証跡は取られていない（最後は 3b2d983e・6234c1a2・fe60a778）。作業ツリーで組んだ配布物の `SOURCE.txt` と組み立ての印は `720285b0…` で `.dirty` が無い（`tar -xOzf … SOURCE.txt`、`strings`。印の `.dirty` は `source_paths` しか見ない。build-stamp/lib.rs:86）。その commit の notices.sh では同じ一覧を作れない
- kind: technical
- 提案: 作業ツリーの差分を、出所を確かめたうえで commit するか捨てるかを決める。決めたら、その commit で 5.2・6.1（apk）・7.1 を取り直す。`notices.sh --self-test` に、`$TMPDIR/poms.tsv` と同じ形の本物の一覧（今の 55 部品）で `apk` が rc=0 になる段を足すと、この型（自己試験は緑・本物で落ちる）を捕まえられる
- 処置: fixed D19 仮 （notices.sh の著作権の表示の規則を D19（仮）として commit した。HEAD 相当の作業ツリーで `tools/release/notices.sh apk <出力>` が rc=0・55 項、jsr305 の項は Apache-2.0 の全文だけで通る。自己試験に jsr305 と同じ形の部品（Apache-2.0・許諾のファイル無し・作者無し）が通る段を置いた。5.2・6.1・7.1 の証跡はこの commit で取り直す）

## R13. 未 commit の差分は「著作権者を求める式」を正規表現で新しく決め、Apache-2.0 の部品は著作権の表示が無くても通す
- 成果物: tools/release/notices.sh（作業ツリー。`HOLDER_REQUIRED=re.compile(r'^(BSD-.*|MIT.*|ISC|0BSD|Zlib)$')`）
- 根拠: `git diff tools/release/notices.sh`。作業ツリーの一覧では `== com.google.code.findbugs:jsr305 3.0.2` の項は Apache-2.0 の全文だけで、`Copyright:` の行が無い。spec（distribution）は「部品ごとに … 許諾の全文と著作権の表示を書く」、落とすのは「全文も著作権の表示も見つからない」とき。全文はあり著作権の表示は無い、という間の場合をどう扱うかを決めた文は、deep・design（D8）・tasks のどこにも無い。これを固定しているのは、差分が足した自己試験の 2 本（Apache-2.0 は通す・BSD-2-Clause は落とす）だけ。配った版の一覧は後から直せない
- kind: conflict
- 提案: 「全文はあるが著作権の表示が無い部品」を通すか落とすかを、design D8 に 1 行で書く（式の一覧の決め方も含める）。書いた決定にそって差分を commit する
- 処置: fixed D19 仮 （「全文はあるが著作権の表示が無い部品」は既定で落とし、本文が著作権者の行を条件に持たない `Apache-2.0` だけを免除する、と design D19（仮）に書き、台帳に載せた。差分の「著作権者を求める式」の許可の一覧は、R21 のとおり免除の一覧に反転した）

## R14. Task 1.1 の検証（CONTRIBUTING の最初の見出しから後ろを変えない）が HEAD で rc=1。原因は 10.3 が同じ範囲の手元の検査の行を書き換えたこと
- 成果物: CONTRIBUTING.md:38 / openspec/changes/st53-license-bundle/tasks.md（1.1 と 10.3）
- 根拠: `diff <(git show origin/main:CONTRIBUTING.md | sed -n "/^## /,\$p") <(sed -n "/^## /,\$p" CONTRIBUTING.md)` → rc=1。違うのは `## 送る前に` の 38 行目だけ（2c33b5f4 が `check-licenses.sh` を `check-licenses.sh rust node` にした）。CLA の節だけを切った diff（`/^## CLA/,/^## 送る前に/`）は rc=0 で、守るべき中身は変わっていない。1.1 は `[x]` のまま、最後の PASS の証跡は 7b0e5eca（2c33b5f4 より前）
- kind: premise
- 読み替え: finish が conflict から premise に読み替えた（守る CLA の節は変わっておらず、成立しないのは検証コマンドの範囲だけ）
- 提案: 1.1 の検証の範囲を、本文が言う CLA の節（`/^## CLA/,/^## 送る前に/`）に狭める。計画を直すなら上流（tasks.md）で行う
- 処置: fixed 1.1 （検証が成立しないだけ —— 守るべき CLA の節は変わっていない。旧コマンドの FAIL を `verify-run 1.1` で記録し、CLA の節だけを比べる台本 `tools/check-cla-section.sh` を `plan_fix.py … --approved machine` で入口にした）

## R15. `check-guards.sh workflows` の「署名の前に承認者を確かめる」は、行をコメントにしても `|| true` を付けても `if: false` にしても通る
- 成果物: tools/release/check-guards.sh:134-136
- 根拠: `.github/workflows` を写した `CHECK_GUARDS_ROOT` で、release.yml の 84 行目を (a) `# - run: tools/release/check-guards.sh approvals`、(b) `… approvals || true`、(c) `- if: false` を前に置いた形、の 3 通りに書き換えた。どれも `check-guards.sh workflows` は rc=0。`grep -nF 'tools/release/check-guards.sh approvals'` は字面しか見ず、注釈の行・失敗を飲む行・走らない手順にも当たる。spec 279 行が言うとおり守りの本体は GitHub の承認だが、Scenario「公開の job は、署名の前に承認者を確かめる」が確かめているのは「その文字列が前にある」ことだけ。`ci.yml` の `package-linux` の `ref:` の行（357 行目）を `# ref: …` にした写しでも `workflows` は rc=0 だった（`grep -q` が注釈の行に当たる）
- kind: technical
- 提案: `publish` の手順を YAML として読み（python の `yaml` か、`- run:` で始まり `#` で始まらない行に絞る）、`run` の値がちょうど `tools/release/check-guards.sh approvals` で、`if:` と `continue-on-error` を持たないことを見る。自己試験に (a)〜(c) の 3 つの偽の release.yml を足す
- 処置: fixed 8.2 （check-guards.sh の `publish_steps` が publish の手順を割り、注釈の行を数えず、承認の手順は `run` がちょうど `tools/release/check-guards.sh approvals` で `if:` と `continue-on-error` を持たないものだけを数える。ci の `ref:` の照合も行頭から見る（注釈の行に当たらない）。自己試験に注釈・`|| true`・`if: false`・`continue-on-error` の 4 つの偽の release.yml と、`ref:` を注釈にした偽の ci.yml を足した）

## R16. `check-artifacts.sh` は、依存の一覧が `== rust-std` の 1 項だけでも通す。自己試験の Windows の 2 つはまさにその形の偽物
- 成果物: tools/release/check-artifacts.sh:62-104・:320-324
- 根拠: 6.1a で組んだ本物の `ashiato-server-0.1.0-linux-x86_64.tar.gz`（`== ` の見出し 201 個）の `THIRD-PARTY-NOTICES.txt` を `== rust-std 1` / `License: MIT OR Apache-2.0` の 2 行に置き換え、6.1b の本物の APK と、印つきの偽の Windows の zip 2 つ（同じ 2 行の一覧）を並べて `check-artifacts.sh <置き場> --allow-debug-key --expect-version 0.1.0` → rc=0「OK: release の配布物を検査した（版 0.1.0）」。spec の IF（179 行）が字面で求めるのは rust-std の項があることだけなので、spec には反していない。それでも、Rust・Node の部品 200 項が抜けた配布物が公開の検査を通る。「4 つの配布物は、許諾・依存の一覧・ソースの場所を持つ」の自己試験は、Windows の 2 つをこの 2 行の一覧の偽の zip で置いている（:322-324）。Windows の本物の一覧（notices.sh を Windows の Python で走らせたもの）を確かめる検査は、rust-std の有無と「全部が木の LICENSE」の 2 つしかない。配った一覧は後から直せない
- kind: technical
- 提案: 検査に下限を足す —— サーバと PC の収集アプリは `== ` の項が N 以上（今は Linux で 201）、サーバは `== react ` を持つ、APK は `== com.google.android.gms:play-services-location ` を持つ。Windows の 2 つは、PR の CI の `package-artifacts` が緑になったことを merge の条件にする（R3）
- 処置: fixed 7.1 （check-artifacts.sh が、Rust の配布物では rust-std のほかの部品があること、サーバでは `== react ` があること、APK では `== com.google.android.gms:play-services-location ` があることを求める。自己試験の偽の一覧は rust-std・anyhow・react の 3 項にし、部品を失った 3 つの形が落ちる段を足した。項の数の下限は数が動くので置かない。Windows の本物の一覧は R3 のとおり PR の CI で確かめる）

## R17. `verify-prep-device.sh` の `$2 == "device"`（spec の「`device` の状態で返す端末のうち」）を外しても自己試験が通る
- 成果物: tools/verify-prep-device.sh:9・:34-46
- 根拠: 写しで awk の条件から `$2 == "device" &&` を消す → `--self-test` rc=0。`$1 ~ /^emulator-/` を消すと rc=1。偽の `adb` が返すのは `device` の状態の行だけで、`offline` / `unauthorized` のエミュレータの場合が無い
- kind: technical
- 提案: 偽の `adb devices` に `emulator-5556\toffline` を足し、呼ばれないことを見る段を自己試験に足す
- 処置: fixed 9.1 （自己試験の偽の `adb devices` に `offline` と `unauthorized` のエミュレータを足し、`device` のものにだけ入れることを見る。写しで `$2 == "device" &&` を消すと `--self-test` rc=1 を確かめた）

## R18. 本人の決定「先頭 25 行」（Q11）は上へ変えても試験が通る
- 成果物: tools/check-spdx.sh（`head -n 25`）
- 根拠: 写しで `head -n 25` を 1 / 5 / 1000 に変えて走らせた。1・5 では `--self-test` が rc=1（今の木の例外が 5 つにならない）、1000 では `--self-test` も `headers` も rc=0。窓を広げても狭めても検査の範囲は変わるのに、広げる側は捕まえない
- kind: technical
- 提案: 自己試験に、26 行目に `// SPDX-License-Identifier: GPL-3.0-only` を持つファイルが rc=0 で通る（見ない）段を足す
- 処置: fixed 2.2 （自己試験に、26 行目に `GPL-3.0-only` の表示を持つファイルが通る（見ない）段を足した。写しで `head -n 25` を 1000 にすると `--self-test` rc=1 を確かめた）

## security review（ac24428..720285b）

見た経路:
- 実行時のコード（`crates` `web/src` `web/e2e` `collector-android/app/src` `migrations`）の差分 → SPDX の行の書き換えだけ。新しい API・画面・listen・送信先・依存は無い（`git diff -U0` から SPDX の行を除くと 0 行。Cargo.lock / package-lock の変更も license の文字列 1 行だけ）（4）
- PR → `ci.yml` の `package-linux` / `package-windows` / `package-artifacts` が `github.event.pull_request.head.sha` を checkout → 組み立て → artifact。起動は `pull_request` だけで `pull_request_target` / `workflow_run` は無く、3 job とも secrets を使わず `persist-credentials: false`（`.github/workflows/ci.yml:1-10,350-417`）（3）
- 公開 → `release.yml` の publish が `check-guards.sh approvals`（`gh api repos/<repo>/environments/release` を読むだけ）→ 署名の秘密は次の `sign-apk.sh` の step の env にだけ渡る（`release.yml:83-92`）（2）
- `ASHIATO_SOURCE_REF: ${{ github.ref_name }}` → env 経由で `notices.sh source` → `v$version` との一致を確かめてから SOURCE.txt へ。`run:` の中に式として展開されない（`release.yml:40-67`、`notices.sh:21-25`）（3）
- 依存の許諾 → `notices.sh` が cargo metadata / package-lock / gradle の cache の aar・jar・POM を読む → THIRD-PARTY-NOTICES.txt。値は `printf '%s'` と python の文字列としてだけ使われ、シェル・eval に入らない。aar・配布物の中身は `unzip -p` / `tar -xO` / `zipfile.read` で標準出力・メモリに読むだけで、展開先のパスは作らない（ZIP slip の経路なし）（3）
- 配布物に入る許諾の束 → server / collector-windows の notices を実際に組み、`$HOME`・`/home/`・ユーザー名が入らないことを確かめた（server 53,515 行・collector-windows 40,060 行で該当 0）。APK の assets に足すのは `make_license_bundle` の一時の置き場（`licenses/` の 3 ファイル）だけ（`build.sh:41-59,154-156`）（4）
- `settings.gradle.kts` の `ashiato.extraRepo`（google / mavenCentral より先に引く取得元）→ 読むのは `gradle.startParameter.projectProperties`（コマンド行の `-P`）だけ。渡すのは `check-licenses.sh:195`（env `ASHIATO_LICENSES_EXTRA_REPO`）と自己試験だけで、`build.sh` の `assembleRelease` も CI も渡さない（3）
- 確認バッチの準備 → `verify-prep-device.sh` の `install_on_emulators` が `emulator-` の serial にだけ debug の APK を入れる。実機には入れない（`verify-prep-device.sh --self-test` → `== OK`）（4）

安全の決め手: この差分は、記録（ライフログ）に届く実行時の経路を 1 本も変えていない（実行時のコードの変更は SPDX の行だけ）。足した CI の経路は `pull_request` の秘密の無い job だけ —— 確かさ 4。`git diff -U0 ac24428..HEAD -- crates web/src web/e2e collector-android/app/src migrations | grep -E '^[+-]' | grep -vE '^(\+\+\+|---) ' | grep -v 'SPDX-License-Identifier: AGPL-3.0-only'` が 0 行（SPDX の行を含めると 928 行）。CI の側は `grep pull_request_target|workflow_run .github/workflows/*.yml` が 0 件（3）

指摘なし

範囲外: なし

## /code-review（ac24428..720285b）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R19. `approvals` looks for the reviewer's login at `u["login"]`, but the GitHub environments API puts it one level down, at `protection_rules[].reviewers[].reviewer.login`. The check can never pass against the real API.
- 成果物: tools/release/check-guards.sh:64
- 根拠: On a real tag push, the publish job runs `check-guards.sh approvals`. `gh api repos/<repo>/environments/release` returns `{"type":"User","reviewer":{"login":"rai-256"}}`. `u.get("login")` is always None, so the job dies with 'release Environment に承認者 … が無い' before signing, and no release can ever be published. The self-test fixtures use the same wrong flat shape (`{"type":"User","login":"fake"}`), so the self-test stays green.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D11 仮 （再現: GitHub の environments の API の `reviewers[]` は `{"type": "User", "reviewer": {"login": …}}` の形で、`u.get("login")` は常に None。`(u.get("reviewer") or {}).get("login")` を見るよう直し、自己試験の偽の JSON も API の形にした。D11 の本文も直した）

## R20. The uncommitted change removed the unconditional `need_holders=True`. A POM row with no license (name `-`, or a name that yields no SPDX parts) now produces empty `chunks` and an empty body, and nothing fails.
- 成果物: tools/release/notices.sh:257
- 根拠: A new Android dependency has no LICENSE in its aar/jar and no `<licenses>` in its POM, so the list row is `coord\t-\t\tself`. `spdx='-'` and `re.findall` finds no parts, so `body=''` and `need_holders` stays False. `build.sh apk` (which does not run check-licenses) ships an APK whose THIRD-PARTY-NOTICES block has a `License: -` line and no license text. Before the change this case at least required POM holders. There should be an explicit error when `chunks` is empty.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D19 仮 （再現: 一覧の行 `fake:nobody:1.0\t-\t\tself` を差分の版に渡すと rc=0 で本文の空の項ができた。部品を 1 つも読めない式と空の `chunks` は「許諾が無い部品」で落とす。自己試験に段を足した）

## R21. `HOLDER_REQUIRED` is an allowlist of license ids that require a copyright notice. Any id not listed (Unicode-3.0, BSL-1.0, Python-2.0, CC-BY-4.0, BlueOak-1.0.0, and others, several of them in check-licenses' ALLOW) silently skips the holder requirement.
- 成果物: tools/release/notices.sh:194
- 根拠: A dependency under Unicode-3.0 (allowed by check-licenses) has no license file and no POM developers. Once its text is added to texts/, notices.sh emits the full text with no Copyright line and passes, although the license requires the copyright notice to be reproduced. The default should be to require holders, with a short list of exempt ids such as Apache-2.0.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D19 仮 （再現: 差分の版では Unicode-3.0 の部品が作者無しで通った。既定を「作者が要る」にし、免除を `HOLDER_EXEMPT`（今は Apache-2.0 だけ）にした。自己試験に Unicode-3.0 が落ちる段を足した）

## R22. The publish job's token has only `permissions: contents: write`, yet `check-guards.sh approvals` calls `gh api repos/$repo/environments/release` with GH_TOKEN=github.token. That endpoint needs Actions read permission for an installation token.
- 成果物: .github/workflows/release.yml:84
- 根拠: On a tag push, GITHUB_TOKEN lacks `actions: read`, so the environments GET returns 403. `approvals` dies with 'release Environment の保護規則を読めない', and publishing is blocked even when the environment is configured correctly. No CI path exercises this against the real API.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D11 仮 （GET environments は installation token に actions の読みを求める（GitHub の文書）。`publish` の permissions に `actions: read` を足し、D11 に書いた。本物の API での確かめは D11 の反転条件のまま、人間の確認待ち 12.3）

## R23. The self-test assertion `! grep -v install_on_emulators "$prep" | grep -Eq 'adb|install'` never runs as a check: bash ignores `set -e` for a pipeline negated with `!`.
- 成果物: tools/verify-prep-device.sh:80
- 根拠: Someone adds a direct `adb install -r "$apk"` back into verify-prep.sh, which would put the debug-key APK on a physical phone. The pipeline returns 1, `set -e` does not abort, and `--self-test` still prints `== OK` in CI. The fix is `if grep … | grep -Eq …; then exit 1; fi`.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 9.1 （再現: `! a | b` は set -e で止まらない。`if … ; then return 1; fi` に直し、写しの verify-prep.sh に `adb install -r x` を足すと `--self-test` rc=1 を確かめた）

## R24. The apk path splits the SPDX expression with `[A-Za-z0-9][A-Za-z0-9.+-]*`, so `WITH` and the exception id become separate parts. `spdx_text` (Rust) and check-licenses' `allowed()` treat `X WITH Y` as one unit.
- 成果物: tools/release/notices.sh:265
- 根拠: An Android dependency's POM name maps to `Apache-2.0 WITH LLVM-exception`, which is in check-licenses' ALLOW. notices.sh looks for `texts/WITH.txt`, dies with '許諾の全文が無い', and `build.sh apk` fails for a license the checker accepts.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D19 仮 （再現: `Apache-2.0 WITH LLVM-exception` の式で `texts/WITH.txt` を探して落ちた。apk の経路も `X WITH Y` を 1 つの部品として読む（bash の `spdx_parts` と同じ式）。自己試験に段を足した）

## R25. The new 'Apache-2.0 needs no copyright holder' rule exists only in the apk Python block. `emit_rust` and `emit_node` still require authors whenever the license file is missing, so the same component would be treated differently depending on the packaging path.
- 成果物: tools/release/notices.sh:105
- 根拠: An Apache-2.0 crate or npm package with no LICENSE file and no authors makes `notices.sh server` die, while the same situation in the APK passes. Two copies of the policy will keep drifting. The holder rule should live in one place that every emitter uses.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D19 仮 （`emit_rust` と `emit_node` も同じ `holder_exempt` で決める。規則の置き場は `notices.sh` の `HOLDER_EXEMPT` の 1 か所で、apk の Python には引数で渡す。自己試験に Rust の Apache-2.0 が通り BSD が落ちる段を足した）

## R26. The 'approval before signing' guard is a fixed-string grep on the publish block. It also matches a commented-out line, a line ending in `|| true`, or a step guarded by `if: false`.
- 成果物: tools/release/check-guards.sh:134
- 根拠: Someone changes release.yml to `# - run: tools/release/check-guards.sh approvals` or appends `|| true` (for example, to work around the 403 above). `check-guards.sh workflows` still returns rc=0, and the APK is signed and published with no approver check.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 8.2 （R15 と同じ直し）

## R27. For non-APK artifacts the notice check only requires a `== rust-std ` entry. A THIRD-PARTY-NOTICES.txt that has lost every crate and Node entry still passes the publish gate.
- 成果物: tools/release/check-artifacts.sh:80
- 根拠: A Windows notices.sh run that quietly yields only the std entry produces a two-line `== rust-std 1` / `License: …` file. `check-artifacts.sh --channel release` passes and the release ships without about 200 required dependency notices. The self-test's own Windows fakes have exactly this shape.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.1 （R16 と同じ直し）

## R28. On a successful install, the manual note becomes "`<apk>`。emulator-5554" and no longer says the APK was installed. The old text said 'を端末 … に入れた'.
- 成果物: tools/verify-prep.sh:63
- 根拠: A person following the verify manual sees only a path and a serial. They cannot tell whether the APK is already on the emulator or still needs a manual install, which defeats the point of the prep step.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 9.1 （`install_on_emulators` は入れたとき「エミュレータ <serial> に入れた」を出し、手順書の注記は「`<apk>` を作った。エミュレータ <serial> に入れた」になる）

## finish の scoped re-review（720285b0..HEAD の tools .github）

finish の fix の後に 1 回、別の席（pr-review-toolkit の code-reviewer、読むだけ）が直した差分だけを見た。

## R29. `publish_steps` は空行 1 つで手順の読み取りが切れ、後ろの `continue-on-error:` / `if:` を見落とす
- 成果物: tools/release/check-guards.sh（`publish_steps`）
- 根拠: 空行は手順の続きの行の正規表現に当たらず `cur=None` になる。`- run: …approvals` の後に空行と `continue-on-error: true` を置いた release.yml が承認の手順として数えられた（`publish_steps` の出力 `5 6`）
- kind: technical
- 処置: fixed 8.2 （空行と注釈の行は手順を切らずに読み飛ばす。直した後は同じ偽物で `0 6`。自己試験に空行を挟んだ変種を足した）

## R30. rust-std のほかの部品の確かめが pipe の `grep -q` で、pipefail の下で前段の SIGPIPE を失敗とみなしうる
- 成果物: tools/release/check-artifacts.sh（`check_artifact_files`）
- 根拠: `grep -E '^== ' | grep -vq …` は後段が 1 行目で抜ける。前段の出力が pipe の緩衝を越えると前段が SIGPIPE（rc=141）になり、正しい配布物を「部品が欠けている」で落とす
- kind: technical
- 処置: fixed 7.1 （pipe をやめ、awk 1 つで見る。rust-std だけの一覧で rc=1・部品のある一覧で rc=0 を確かめた）
