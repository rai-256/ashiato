# 整合の確認（2026-11-03・ST53 の深掘り第 2 回の答えを FR-127 と §5「配布の方針」へ戻した改訂）

- 基準: `da90c517e4b6bef6e01d5e6db334e90f097c646b`..`67b7211bb44bc4619bf67a53418c64bef4c2645a`

対象は commit 67b7211b が足した 2 つの ★（`docs/requirements.md:1073` の FR-127 ★ 2026-11-03 補足、`:1548` の §5「配布の方針」の ★ 2026-11-03 補足）と、
`make_story.py` で作り直した `docs/stories/ST53.md:22`。答えの記録（`openspec/changes/st53-license-bundle/deep-answers-2.txt`・`deep.md` の第 2 回の節・`deep-questions-r2.json`・`provisional-review.md`）と突き合わせた。文書は head（`67b7211b`）の内容で読んだ。

機械の入力:
- `req_impact.py . --base da90c517…`: 改訂した要件 1 件（FR-127）・扉 0 項。名指しは `docs/stories/INDEX.md:76,371`・`stories.json:1260` と st53-license-bundle の中だけ
- `check_chain.py .`: 要件 171 / Story 58 / 扉 35・拾われた要件 169/171・未回収 0・warn 0。FAIL 1 件「整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-127」（この監査が受けるもの）
- `check_scenarios.py .`: 担保の無い Scenario 45 件。すべて st53-license-bundle の change のもの（tasks は `- [x]` が 0 件）。正典の Scenario で印の無いものは 0

走っている change は `st53-license-bundle`（出どころ）と `st44-collector-credential-scope`（許諾・`SOURCE.txt`・表示の検査に触れない。交差しない）。
st53 の tasks は 1 件も済んでいないので、失われるもの（uncaptured / discarded / exported / rewrite-all）を持つ印は無い。
呼び出し元の指示どおり、st53 の design・spec・tasks・go.json が第 2 回の答えをまだ写していないことは、次の spec --answers 段が直すものとして「直すところ」を `走っている change（st53。spec --answers で写す）` にし、ここで直すもの（requirements・ほかの文書）と分けた。

## C1. FR-127 ★ は PR の head の commit を「取れる側」として選んだが、head も閉じた後に取れるかは確かめておらず、枝を rebase して push し直す運用では前の組み立ての head を指す ref が残らない
- 行き先: requirements（FR-127 ★ 2026-11-03 に、PR の head も閉じた後・push し直した後に取れるかは確認バッチで確かめると足した。11.1 と P14 は走っている change の spec --answers が直す —— `deep.md` の「spec の段へ渡すもの」に書いた）
- 重大度: 中
- 文書: docs/requirements.md:1073
- 要件: FR-127
- 根拠: FR-127 ★ 2026-11-03「**PR の組み立ての `SOURCE.txt` には PR の head の commit を書く**（GitHub が作る merge の commit は PR を閉じた後に取れるかが確かめられておらず、取れなければ落とされた写しのソースの場所は直せない）」。
  理由は merge の commit の側の未確認だけを挙げる。問いの本文（`deep-questions-r2.json` Q14 の context）は「head の commit も、PR を閉じた後に `refs/pull/<n>/head` で取れるかは確かさ 4 に上げていません（一般には PR の head は閉じた後も残る）」と書き、
  推奨（`deep.md:91`）は「確認バッチ 11.1 で閉じた PR の head を取れるかを確かめる」を含む。ところが答えの写し（`deep.md:128`「D9 ← Q14（PR は head の commit `github.event.pull_request.head.sha`。組み立ての印との比べはその場合だけ外す）」）にも ★ にも、この確かめが無い。
  この repo の PR の枝は rebase して push し直される（commit `66b014c3`「merge gate（PR #230）が origin/main（#228。ST44）への rebase で…」、`docs/audit/2026-11-02-st53-consistency.md:4`）。`refs/pull/<n>/head` は最新の head だけを指すので、push し直す前の組み立て（成果物は `retention-days` 無しで置かれる。`.github/workflows/ci.yml:364-365`・`:385-386`）の head は、どの ref からも辿れなくなる。
  `tasks.md:218-220` の 11.1 は「この Story の PR」の `Commit:` を merge の後に `git fetch` するだけで、main は merge commit で取り込む（`git log origin/main --merges` が 176 件）ので、merge された PR の最後の head は main から辿れて必ず取れる。push し直す前の head は 11.1 では試されない
- 直すところ: requirements（★ の括弧に「head も閉じた後・push し直した後に取れるかは確認バッチで確かめる」を足す）と、走っている change（st53。spec --answers で写す。11.1 に push し直した前の head の `git fetch` を足し、P14 を head の前提に書き直す）
- 機械: 安く捕まえられる（11.1 で使い捨ての PR を開き、組み立て → force-push → 閉じる → 前の head を `git fetch origin <sha>` する。取れなければ rc≠0）

## C2. 走っている change の design D7・spec・tasks が、Q12 の答え（AND）と逆の「どれか 1 つが許可されていれば通す」を Scenario で固定している（印のみ）
- 行き先: leave: 走っている change（st53）の spec --answers Q12 が写す。`deep.md` の「spec の段へ渡すもの」に書いた
- 重大度: 中
- 文書: openspec/changes/st53-license-bundle/specs/distribution/spec.md:114
- 要件: NFR-26
- 根拠: 答え `deep-answers-2.txt`「Q12 […] -> AND で読む（全部が許可の一覧にあるか、名指しに当たるときだけ通す）」。
  `specs/distribution/spec.md:86`「1 つの部品が許諾の名前を 2 つ以上持てば、どれか 1 つが許可されていれば通す」、`:114` Scenario「許諾の名前を 2 つ持つ部品は、どれか 1 つが許可されていれば通る」（THEN「その部品は Apache-2.0 として通る」）、`design.md:116`「どれか 1 つが許可の一覧にあれば通す（`OR` と読む）」、`tasks.md:105` が同じ Scenario 名を検証に挙げる。
  `design.md:112`「POM は依存の解決のときに gradle の cache に入っているので、`--offline` で動く」も Q12 の推奨（`deep.md` の Q12「あわせて…`--offline` で取れなければ組み立てを落とす」に直す）の前のまま。
  NFR-26 ★「許可の外のプロプライエタリな依存が入ったら検査が止める」は AND と向きが合い、要件の改訂は要らない（Q12・Q13 は要件を変えない判断、という呼び出し元の説明と合う）。
  このまま下流が Scenario を担保すると、名指しの外のプロプライエタリを通す分岐をテストが守ることになる
- 直すところ: 走っている change（st53。spec --answers で写す。`:86` の文、`:114` の Scenario を「2 つ持つ部品は、片方でも許可の外なら落ちる」に反転、`tasks.md:105` の名前、D7・D8 の見出しの「未決」）
- 機械: 捕まえられた（`deep.md:128` の「D7 ← Q12（AND。…）」が spec --answers の入力になる）。写し漏れは、`spec --answers` の後に Scenario 名「どれか 1 つが許可されていれば通る」を grep して残れば落とす形で安く捕まえられる

## C3. 走っている change の D9・D16・spec・go.json・premises が、PR の `SOURCE.txt` を「組み立ての印と同じ commit（合成の merge commit）」のまま持つ（印のみ）
- 行き先: leave: 走っている change（st53）の spec --answers Q14・Q15 が写す。`deep.md` の「spec の段へ渡すもの」に書いた
- 重大度: 中
- 文書: openspec/changes/st53-license-bundle/specs/distribution/spec.md:163
- 要件: FR-127
- 根拠: FR-127 ★ 2026-11-03「PR の組み立ての `SOURCE.txt` には PR の head の commit を書く」、答え Q14「組み立ての印との比べはその場合だけ外す」。
  `spec.md:163`「それ以外の組み立て（開発版・PR の組み立て）では、組み立ての印の 40 桁の commit を書く」、`:169`「PR の組み立てと下書きの配布物の検査（debug の鍵を許す検査）と、開発版の検査: 組み立ての印の commit」、
  `:215-218` Scenario「開発版と PR の組み立ての配布物のソースの場所は、組み立ての commit」（THEN「組み立ての印と同じ 40 桁の commit」）、`:220-223` Scenario「PR の組み立てと下書きの配布物の検査は、commit のソースの場所で通る」（GIVEN「組み立ての印と同じ commit を持つ」）。
  `design.md:142` D9 の本文「渡されないとき（PR・開発版・下書き）は、`git rev-parse HEAD` の 40 桁を書く」、`tasks.md:151`「`--allow-debug-key` の PR と下書き…なら組み立ての印の commit と同じ `Commit:`」、`:220`「取れなければ D9（仮）の反転条件で上流へ返す」、
  `go.json:921`「Commit: <GitHub が PR のために作る merge commit>」・`:2086` IMP-06、`premises.md:23` P14（merge commit の前提）。
  コード: `tools/release/draft.sh:32` は PR の成果物に `check-artifacts.sh --allow-debug-key` を当てるので、比べを外す「その場合」は PR の `package` と下書きの両方になる。開発版（`nightly.yml:71` の `--channel nightly`）と main への push の組み立ては比べを残す。
  `ci.yml:355` の checkout は `ref` を渡さず、組み立ての印は合成の merge commit を持つ（`crates/server/build.rs:34-46`・`collector-android/app/build.gradle.kts:258-259` は `ASHIATO_BUILD_COMMIT` か `git rev-parse HEAD`）
- 直すところ: 走っている change（st53。spec --answers で写す。D9・D16 の本文と見出し、spec の `:163`・`:169` と 2 つの Scenario、tasks 5.x の `:151` と 11.1、go.json の walkthrough と IMP-06、premises P14）
- 機械: 安く捕まえられる（spec --answers の後に、spec・tasks に「PR」と「組み立ての印と同じ」が同じ文にあれば warn）

## C4. 走っている change の D4・D5・spec が、Q10 の答え（最初の印から後は直せないことを受け入れる）の前の「表示は後から書き換えられる」を失わない理由に持つ（印のみ）
- 行き先: leave: 走っている change（st53）の spec --answers Q10・Q11 が写す。`deep.md` の「spec の段へ渡すもの」に書いた
- 重大度: 低
- 文書: openspec/changes/st53-license-bundle/design.md:78
- 要件: FR-127
- 根拠: §5 ★ 2026-11-03「移行は APK に入らず追加の許可が要らないので書き換えず、最初の印から後はこの表示を直せないことを受け入れる」。
  `design.md:78` D4「失わない理由: 表示は後から書き換えられる。指紋の警告を受け入れるなら、いつでも 3 行を書き換えて例外から外せる」、`:76`「印 `v*` はまだ 0 個で、配った移行の凍結…には当たらない」。
  `spec.md:57`「D4（Q10 の答え待ち）・D5（Q11 の答え待ち）」、`:58`「D5（Q11 の答え待ち）」、`tasks.md:58`「D5（仮）」。
  例外の中身（wrapper の 2 つと移行の 3 つ。`spec.md:67`）は答えと同じで、変わるのは理由と見出しだけ
- 直すところ: 走っている change（st53。spec --answers で写す。D4 の失わない理由を Q10 の「受け入れる」に、見出しの「未決」を外す）
- 機械: できない（理由の文の中身）

## C5. Q12 の答え（`--offline` で取れなければ落とす）の下で、tasks 4.1 の検証は `--offline` で POM を取る。手元の gradle の cache には今の部品の POM が無い（印のみ）
- 行き先: leave: 走っている change（st53）の spec --answers Q12 が D7 と tasks 4.1 を直す。`deep.md` の「spec の段へ渡すもの」に書いた
- 重大度: 中
- 文書: openspec/changes/st53-license-bundle/tasks.md:93
- 要件: NFR-26
- 根拠: `deep.md:128`「D7 ← Q12（AND。`@pom` を解決して取り寄せ、`--offline` で取れなければ落とす）」。
  `tasks.md:93` の 4.1 の検証「`./collector-android/gradlew -p collector-android --offline -q :app:ashiatoDependencyPoms …` rc=0」。
  手元の cache（`~/.gradle/caches/modules-2/files-2.1/`）は `androidx.core/core/1.17.0` に `core-1.17.0.aar`、`com.google.android.gms/play-services-location` に `play-services-location-21.3.0.aar` だけを持ち、`.pom` も `.module` も無い（この席で `find` した）。Q12 の context も同じことを書いている。
  組み立ての前に一度 `@pom` を取り寄せる手順が tasks に無いので、`--offline` の検証と、CI の `package-linux`（新しい runner で cache が空。`ci.yml:350-365`）でどの順に取るかが決まっていない。検査が落ちる側に倒れるので配布物は漏れないが、PR ごとに CI が赤くなりうる
- 直すところ: 走っている change（st53。spec --answers で写す。D7 に「取り寄せる段（online）と読む段（`--offline`）」を分けて書き、4.1 の検証を合わせる）
- 機械: 捕まえられる（4.1 の検証そのものが下流で落とす）

## C6. 製造準備の不可逆の台帳が、Q10 で受け入れた「移行の 3 つは最初の印から後は表示を直せない」を持たず、SPDX を全部 WITH の式と書く
- 行き先: docs（`docs/production-prep.md` の「本体のライセンス」の行に ★ 2026-11-03 で例外の 5 つと、前進側の 2 つは最初の印から後に直せないことを足した）
- 重大度: 中
- 文書: docs/production-prep.md:33
- 要件: FR-127（扉 35）
- 根拠: `docs/production-prep.md:33`「SPDX は `AGPL-3.0-only WITH AdditionRef-ashiato-play-services`）。付けて配った版はその許可付きで出回り続ける（取り消せない）」。例外の一覧（移行の 3 つ・wrapper の 2 つ）が無い。
  Q10 は `kind: irreversible`・`loss: exported`（`deep-questions-r2.json` Q10）で、選んだ選択肢の `irreversible`「最初の印の後は、この 2 つの表示を書き換えると凍結の検査に落ちる」。§5 ★ 2026-11-03 もそれを受け入れると書く。
  st53 の `tasks.md` 10.3 が `docs/production-prep.md` で直すのは「依存ライセンスの検査」の行だけ（前回の監査 C7 の根拠）。この台帳を読んで最初の印の前に表示をそろえようとする人は、移行も書き換えて凍結の検査（`tools/check-released-migrations.sh:52-54`）か指紋の警告に当たる
- 直すところ: その文書（`:33` に ★ 2026-11-03 で「移行の 3 つと gradle の wrapper の 2 つは元の表示のまま。移行の 2 つは最初の印から後は直せない（ST53 Q10）」を足す）
- 機械: できない

## C7. FR-127 ★ 2026-10-25 補足の「ソースの SPDX を一緒に直す」が、移行の 3 つを例外にする決定を名指さないまま ST53 の「満たす要件」へ写る
- 行き先: requirements（FR-127 ★ 2026-11-03 に「SPDX の例外は §5 の ★ 2026-11-03」と足した）
- 重大度: 低
- 文書: docs/requirements.md:1071
- 要件: FR-127
- 根拠: `:1071`「追加の許可を足す Story は、`LICENSE`・`README.md`・`CONTRIBUTING.md`・ソースの SPDX（`AGPL-3.0-only`）・`tools/check-licenses.sh` の許可一覧を一緒に直す」。
  例外の決定は §5 ★ 2026-11-03（`:1548`）にだけあり、FR-127 の ★ 2026-11-03（`:1073`）は Q14・Q15 だけを受ける。`ST53.md:19-22` は FR-127 を逐語で写し、§5 は写さない。
  `ST53.md:46` の完了の判定は「SPDX は表示の検査の例外の一覧 —— 移行の 3 つと gradle の wrapper の 2 つ —— を除く」を持ち、spec `:67` も同じなので、Story は誤らない
- 直すところ: leave（完了の判定と spec が例外を持つ）。直すなら requirements（FR-127 ★ 2026-11-03 に「SPDX の例外は §5 ★ 2026-11-03」と 1 句）
- 機械: できない

## C8. §5 ★ 2026-11-02 訂正が「ST53 の design D5（仮）」を名指したまま。FR-127 と違い、★ 2026-11-03 がそれを決めたと書かない
- 行き先: requirements（§5 の ★ 2026-11-03 に「★ 2026-11-02 の D5（仮）はこれで決まった」と足した）
- 重大度: 低
- 文書: docs/requirements.md:1547
- 要件: NFR-26, FR-127
- 根拠: `:1547`「表示の検査。ST53 の design D5（仮）」。`:1548` の ★ 2026-11-03 は「仮の design D4・D5 が別の席で否とされ、本人が決めた」と出所を書くが、FR-127 ★ 2026-11-03（`:1073`）の「上の ★ 2026-11-02 の「D9（仮）が持ち…」はこれで決まった」に当たる 1 句が無い
- 直すところ: requirements（`:1548` の末尾に「上の ★ 2026-11-02 の D5（仮）はこれで決まった」）。leave でもよい（★ は経緯で、すぐ下に答えがある）
- 機械: 安く捕まえられる（check_chain: ★ の本文に `D<n>（仮）` を含み、その change の `provisional.jsonl` に D<n> が無ければ warn）

## C9. ST53 の完了の判定と spec が、`SOURCE.txt` を commit で示す組み立てを「開発版と PR」とだけ書き、FR-127 ★ が名指す main への push を落とす
- 行き先: stories（`stories.json` の ST53 の完了の判定を「それ以外の組み立て —— 開発版・main への push・PR」にして `make_story.py`。spec の文は走っている change の spec --answers が直す）
- 重大度: 低
- 文書: docs/stories/stories.json:1260
- 要件: FR-127
- 根拠: FR-127 ★ 2026-11-02「それ以外の組み立て（開発版・main への push・PR）は、40 桁の commit で示す」、★ 2026-11-03「開発版と main への push は main の commit」。
  `stories.json` ST53 done[4]（`ST53.md:50`）「正式版・ベータ版は印 `v*`、開発版と PR の組み立ては 40 桁の commit」、`spec.md:163`「それ以外の組み立て（開発版・PR の組み立て）」。
  コード: `ci.yml:3` の `push: { branches: [main] }` でも `package-linux`・`package-windows` が成果物を上げる（`:364-365`・`:385-386`）
- 直すところ: その文書（`stories.json` の done[4] を「それ以外の組み立て（開発版・main への push・PR）」にして `make_story.py`）。spec `:163` は走っている change（st53。spec --answers で写す）
- 機械: できない

## C10. 走っている change の `deep.md` が、第 2 回で要件へ戻したものを「要件へ戻すもの」に記さず、Q5・Q7 の行は D4・D16 を「（仮）」のまま呼ぶ（印のみ）
- 行き先: docs（`deep.md` に第 2 回の「要件へ戻すもの」「Story へ戻したもの」を足し、Q5・Q7 の行の（仮）を「第 1 回の時点では仮。第 2 回で本人が決めた」にした）
- 重大度: 低
- 文書: openspec/changes/st53-license-bundle/deep.md:115
- 要件: FR-127
- 根拠: `deep.md:115`「## 要件へ戻すもの（record の段で戻した。2026-11-02）」の下は第 1 回の 3 行だけで、2026-11-03 の FR-127 ★・§5 ★ の行が無い（`:57-58` の第 2 回の前書きは spec --answers へ写すことだけを書く）。
  `:130`「D4（仮）（移行の 3 ファイルは表示を書き換えない。書き換えるのは 503）は、答えの上に乗る例外として残す」、`:131`「D16（仮）は答えと一致した」。`:128` の第 2 回の行は「どれも本人の決定で、（仮）には戻さない」
- 直すところ: 走っている change（st53。spec --answers で写す。`:115` の節に 2026-11-03 の 2 行を足し、`:130-131` の（仮）を外す）
- 機械: できない

## K1. §5 ★ が「移行の 3 つ」は最初の印から後は表示を直せないと書くが、凍結の検査は戻し手順（`.down.sql`）を見ない
- 行き先: requirements（§5 の ★ 2026-11-03 を「最初の印から後は前進側の 2 つ（`.sql`）の表示を直せない。`.down.sql` は凍結の検査の外」に狭めた）
- 重大度: 低
- 文書: docs/requirements.md:1548
- 要件: FR-127
- 根拠: `:1548`「移行の 3 つ（表示 `AGPL-3.0-only` のまま）と gradle の wrapper の 2 つは例外の一覧に名前で載せる —— …最初の印から後はこの表示を直せないことを受け入れる」。
  コード: `tools/check-released-migrations.sh:5`「戻し手順（.down.sql）は見ない」、`:52` は `migrations/$name.sql` だけを `git diff` する。`crates/server/build.rs:73` も `.down.sql` を取り込まない。
  3 つのうち `migrations/202609211400_browser_history_record_id.down.sql:1` は凍結の外で、最初の印の後も書き換えられる。問い（`deep-questions-r2.json` Q10）は「前進側 2 つ」と「戻し手順。凍結の外」を分けて書き、選んだ選択肢の `irreversible` も「この 2 つ」
- 直すところ: requirements（「最初の印から後は前進側の 2 つの表示を直せない」に狭める）
- 機械: できない

## 未確認

- GitHub が、閉じた PR の head と、force-push で置き換えられた前の head を、`git fetch origin <sha>` で返すか（C1。この席では試していない。`refs/pull/<n>/head` が最新の head だけを指すことも GitHub の文書の読みで、走らせていない）
- gradle の detached configuration で `<group>:<name>:<version>@pom` が `--offline` で解決できるか、CI の `package-linux` で POM をどの段で取り寄せるか（C5。走らせていない）
- いま公開リポジトリに置かれている PR の成果物の件数と、その中に rebase 前の head で組んだものがあるか（C1。`gh api` を走らせていない）
- `nightly.yml` を `workflow_dispatch` で main 以外から起こしたときの成果物（`nightly-linux`・`nightly-windows`。`upload-artifact` は Environment を通らない）の `SOURCE.txt` が main の commit にならないこと（構造から読んだだけで、走らせていない）

## 触らなくてよいと判断したもの（理由つき）

- **`check_chain.py` の FAIL（整合の確認の後に FR-127 が変わった）**: この監査が受ける。改訂した側が要件の ★ からこの監査を名指せば消える
- **`docs/stories/ST53.md:22`**: 要件 `:1073` と逐語で同じ（`make_story.py` の再生成）。題・価値・完了の判定は変わっていない
- **Q12・Q13 を要件へ戻していないこと**: NFR-26 ★「許可の外のプロプライエタリな依存が入ったら検査が止める」は AND の読みと向きが同じで、要件の字面を変えない。写し先は design D7・D8（C2）
- **表示の検査の窓（先頭 25 行）と例外の 5 つ**: この席で追跡しているファイル（symlink を除く）を全部見て、先頭 25 行に `AGPL-3.0-only` 以外の表示を持つのは `collector-android/gradlew:18`・`collector-android/gradlew.bat:16`（`Apache-2.0`）だけ、移行で表示を持つのは 3 つだけだった。文書が表示の語を引用している所（`docs/requirements.md` と st53 の change の文書）は、どれも 25 行より後ろにある。後から文書の先頭で引用すれば、作る検査そのものが落とす
- **§5 ★ の「表示の検査は…を見て」がいま無い道具（`tools/check-spdx.sh` は未作成）を現在形で書くこと**: ★ 2026-11-02 訂正（K1）と同じく ST53 の design が作るものとして書かれ、st53 の tasks 2.2 が作る
- **FR-127 ★ の「開発版は main の commit」**: 公開される開発版は `nightly` Environment を通り、その Environment は branch `main` だけに開く（`docs/release.md:35`、`tools/release/check-guards.sh:37-51` が確かめる）。main 以外から起こした成果物は「未確認」に分けた
- **「移行は APK に入らず」**: `collector-android/app/build.gradle.kts` は `migrations` を参照せず、移行はサーバの実行ファイルにだけ入る（`crates/server/build.rs:73`・`:88`）。事実と合う
- **`docs/handoff/ST53.md`**: `SOURCE.txt`・表示の検査・依存の読み方に触れず、第 2 回の答えと食い違わない
- **正典（`openspec/specs/`）**: この改訂は正典の Requirement も Scenario も変えない。正典の `distribution` の「手で起こした確かめの組み立ては、下書きのリリースにだけ置き、印を作らない」（`:836-840`）は、下書きの配布物が PR の head の commit を持つことと食い違わない（下書きは公開せず印も作らない）
- **走っている change `st44-collector-credential-scope`**: 許諾・`SOURCE.txt`・表示の検査に触れない（grep で 0 件）
