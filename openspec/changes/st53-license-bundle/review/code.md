# ST53 コードレビュー

## final review（ac244280..1ba70a3f）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）がブランチ全体の差分（review package `.superpowers/sdd/tasks/review-ac244280..1ba70a3f.diff`、41 commit）と全 Task の指摘の記録（Minor と ⚠️）を見た。判定は「No（直せば可）」。

## R1. HEAD で `tools/check-spdx.sh` が rc=1 で落ちる（空白を含むパスを `awk` で切っている）。自己試験は緑のまま
- 成果物: tools/check-spdx.sh:43
- 根拠: `git ls-files -s | awk '{print $1 "\t" $4}'` は空白で区切るので、Task 5 が足した `tools/licenses/texts/Android Software Development Kit License.txt` が `.../texts/Android` で切れ、`head` が落ちる。pipefail と set -e の下で NG の文を出さずに rc=1。再現: `tools/check-spdx.sh` → 例外 5 行の後に `head: cannot open '.../tools/licenses/texts/Android'`、rc=1（2026-11-04、この final で確かめた）。自己試験は `output="$(check_headers "$root")"` の中で errexit が引き継がれないので失敗を飲み込み、「今の木で rc=0」の Scenario が rc を見ていない。最後の PASS の証跡はこのファイルができる前（eaba9a0c）。T2 F4 の未処置
- kind: technical
- 提案: `git ls-files -s -z` と `read -r -d ''`（またはタブで割る）でパスを読む。自己試験の今の木の検査は `$(…)` で包まずに rc を確かめる（`check_headers "$root" >out || return 1`）

## R2. APK の THIRD-PARTY-NOTICES に雛形・自作の要約の許諾の文が入る
- 成果物: tools/licenses/texts/BSD-3-Clause.txt / tools/licenses/texts/Android Software Development Kit License.txt / tools/release/notices.sh:198-212
- 根拠: `androidx.health.connect:connect-client-external-protobuf` の項が `Copyright (c) [copyright holder]` のまま入る（BSD-3 の条件の著作権の表示が欠ける）。spec（distribution）の「部品ごとに許諾の全文と著作権の表示を書く」「IF 全文も著作権の表示も見つからない THEN 組み立てを落とす」と、design D8 の「無ければ SPDX の全文に作者を添える。どちらも無ければ落とす」を携帯の側で満たしていない。gms の項の `Android Software Development Kit License.txt` は 6 行の自作の要約で、許諾の本文でも出所のある表示でもない（T5 F12 の未処置）。aar は最上位の `META-INF/` だけを読み、`classes.jar` の中の LICENSE / NOTICE を拾わない。最初の印の後では配った文を直せない
- kind: technical
- 提案: aar の `classes.jar` の中の `META-INF/LICENSE*|NOTICE*` も読む。それでも無いときの著作権の表示は POM（developers / organization）から機械的に取り、取れなければ落とす（雛形の `[copyright holder]` が残る本文は落とす）。PROPRIETARY の項は自作の要約をやめ、許諾の名前と POM の許諾の URL を出所つきで書く（法的な文を作文しない）

## R3. Windows の組み立て（`package-windows`）が一度も走っておらず、notices.sh の Windows の経路が確かめられていない
- 成果物: tools/release/notices.sh / tools/release/build.sh（`archive_windows`）/ tools/release/check-artifacts.sh
- 根拠: 証跡は Linux のサーバと APK だけで、このブランチの CI の run も無い。Windows の job で `python3` を使うのは notices.sh が初めて。Windows の Python の既定の符号化（cp1252）で UTF-8 の許諾の本文を読み書きする箇所、バックスラッシュの `manifest_path` で `dirname` が `.` に化けて全部の部品に repo の LICENSE が入る経路が確かめられていない。`check-artifacts.sh` は Windows の zip の THIRD-PARTY-NOTICES について `== rust-std` の有無しか見ないので、この化け方を捕まえない
- kind: technical
- 提案: Python の読み書きに `encoding='utf-8'` を明示（または `PYTHONUTF8=1`）。`manifest_path` を正規化する。`check-artifacts.sh` で、THIRD-PARTY-NOTICES の部品の本文がすべて repo の `LICENSE` と同じでないことを見る。PR の CI の `package-windows` が緑になることを merge の条件として PR 本文に書く

## R4. docs/release.md の組み替えで ST52 の手順が消え、申し送り C2（FR-108 が指す §7）が満たされていない
- 成果物: docs/release.md §6・§7・§8
- 根拠: 旧版（`git show ac244280:docs/release.md`）と比べると、最初の印の手順の列（`git tag v<版> <commit>` → `version.sh plan` → `git push origin v<版>` → workflow の順の確認）、修正の枝のコードの塊と「修正の枝からベータ版は出さない」「`migrations/ORDER` と `envelope.json` を変えない」、`sign-apk.sh … dev` と `ASHIATO_EXPECT_DEV_CERT_SHA256` の注記、nightly の公開の順、§7 の「起動後に確認し、異常なら旧版へ戻す」が消えた。「本人の環境を移す」は `###` に格下げされ、`docs/requirements.md:1004`（FR-108）は今も「`docs/release.md` §7」を指す（`docs/handoff/ST53.md` の C2）。§6 の `"$dir"` は定義されていない（T10 F7）。§8 末尾の「Q2 の判断へ戻す」は tasks.md 11.2 の Q3 と食い違う
- kind: technical
- 提案: 消えた手順を戻す。「本人の環境を移す」を `## 7.` に戻し、冒頭の ★ 2026-11-03 の段落を残す（feat の枝から要件は書き換えない）。`"$dir"` を定義し、Q 番号を合わせる

## R5. 確認バッチの準備がエミュレータ 1 台への導入の失敗で全体ごと止まり、手順書から APK の場所が消える
- 成果物: tools/verify-prep.sh:11・48・62
- 根拠: `android_note="$(install_on_emulators …)"` が set -e の下にあり、導入の失敗 1 つで準備全体が止まる（以前は手順書に注記して続けていた）。注記がエミュレータ名だけになり APK の場所が消える（T9 ⚠️）。冒頭の説明「実機が繋がっていれば APK も入れる」（:11）と、読まれない `device=""`（:48）が残る
- kind: technical
- 提案: 失敗は注記にして続ける。注記に APK のパスを入れる。古い説明と使われない変数を消す

## R6. `tools/verify-prep-device.sh` の自己試験の `[ "$output" != *emulator-5554* ]` が常に真
- 成果物: tools/verify-prep-device.sh:66
- 根拠: `[ ]` の中ではグロブが効かない（T9 F6 の未処置）。自己試験が `tools/verify-prep.sh` を相対パスで読む（T9 F5）
- kind: technical
- 提案: `[[ ]]` に直し、パスはスクリプトの場所から組む

## R7. `tools/release/check-guards.sh` は approvals の行が無いと NG の文を出さずに終わる
- 成果物: tools/release/check-guards.sh:133
- 根拠: `grep | head | cut` が pipefail で落ち、理由を示さずに非 0 で終わる（T8 F3）
- kind: technical
- 提案: 見つからないときに NG の文を出して落とす

## R8. README の参考訳が追加の許可の条件と主語を落としている
- 成果物: README.md
- 根拠: 参考訳から「Android SDK のライセンスの条項の対象となる部分を含む」と、許可を与える主語（本プログラムのライセンサー）が抜けている（T1 F6）。CONTRIBUTING の `## ライセンス` の「本体は AGPL-3.0-only」は、1.1 の検証が見出しの後ろの書き換えを禁じた計画の側の食い違い
- kind: technical
- 提案: 参考訳を正本（`LICENSES/AdditionRef-ashiato-play-services.txt`）どおりに直す。CONTRIBUTING は 1.1 の検証を壊さない範囲で扱う

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
