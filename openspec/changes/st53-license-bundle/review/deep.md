# ST53 深掘りの問いの独立検査（deep-review）

対象: `openspec/changes/st53-license-bundle/deep-questions.json`（Q1〜Q7）と `premises.md`（P1〜P9）、呼び出し元が問いの外に C として置く 6 項目。
手順 1〜8 は問いの一覧を見る前に行い、終えてから突き合わせた。指摘は R1 以降。問いの JSON は触っていない。

## 手順ごとに確かめた範囲

- 手順 1（要件どうしの衝突）: FR-127・NFR-26・§5「商用化の方針」「配布の方針」・扉 35・扉 34、Story の「壊してはいけないもの」を突き合わせた。
  - 「既存の許可一覧を壊さない」と §5「他人の GPL / AGPL は NFR-26 の検査が止める」は、web の lock を読んだ結果、実際にはぶつからない（R8）。
  - §5 の「取り込んだコードも検査が止める」は依存の検査では成り立たない（R8）。
  - 同じ「本物の公開」について、申し送りと ST52 の完了の判定が「ST53 の確認で見る」と「ST53 の確認の後」の両方を言っている（R1）。
- 手順 2（扉の幅）: 扉 35 の「Google Play 開発者サービスのライブラリ」を Google の maven の `com.google.android.gms` 群と突き合わせた。この群には広告と Firebase Analytics の本体が入っている（R2）。
  FR-127 の「同じ印」は、動く印 `nightly` のときに幅が残る（R6）。
- 手順 3（一方通行）: 次の 6 つを洗い出した。
  - 許可の文の中身（R3）
  - 鍵（R4）
  - 最初の印（R1）
  - 公開のリポジトリの CI の成果物（R5）
  - 一覧の中身（R7・R10）
  - SPDX（Q3。open で妥当）
- 手順 4（日常）: 該当は 1 件だけで、公開ごとの承認の手間（Q2 の選択肢が書いている）。常時通知・電池・容量に効くものは無い（この Story は画面を持たず、携帯の振る舞いを変えない）。
- 手順 5（既存コード）:
  - `tools/check-licenses.sh` の Android の節は echo だけで、検査をしていない（NFR-26 を満たしていない）。
  - `tools/release/build.sh` の tar / zip には実行ファイルが 1 つ入るだけ（FR-127 を満たしていない）。
  - `check-artifacts.sh`・`release.yml`・`nightly.yml` に許諾の語は 0 件（`grep -n -i "licen\|許諾\|NOTICE\|third"` の結果）。
  - `cargo metadata` に std は無い（R7）。
  - `gradlew :app:dependencies --configuration releaseRuntimeClasspath --offline` を走らせた。gms は 4 つで P2 のとおり。
  - gms の aar には `third_party_licenses.txt` があるが、aar の根に置かれていて res / assets ではない（R7）。
  - `tools/verify-prep.sh:58-61` が最初の端末に `adb install -r` で入れることを確かめた（Q6 の前提のとおり）。
- 手順 6（過去の取りこぼし）: `scripts/upstream_misses.py report` の観点を当てた。
  - 「申し送りの全項目を見出し単位で突き合わせる」: `docs/handoff/ST53.md` の 3 見出しのうち IMP-01 は Q2、K1 は Q6 が受けている。「ST52 Task 11 — 最初の印と開発版の公開」は、どの問いも受けていない（R1）。
  - 「ST52 の完了の判定が、まだ起きていない本物の操作に乗る」: 同じ族（R1）。
  - 「外に出る文言を決めるときは、意味を決めている決定を挙げる」: R2・R3。
  - 「後のために残す → 後でどう見つけるか」: R9。
  - 「git の履歴を読む検査は checkout の深さを前提に書く」: 該当なし。Q2 の推奨は承認を GitHub の API で読み、git の履歴は読まない。
  - 時計・保存場所・初回の大量の族: 該当なし。時刻・端末の状態を入力にする決定が無い。
- 手順 7（族の網羅）:
  - Q2（印から鍵を読む）の兄弟は 4 つ。承認の API、Environment・ルールセットの管理の API、main に入った workflow の変更、開発版の鍵。このうち管理の API が問いに無い（R4）。
  - Q4（配布物）の兄弟は 4 つ。正式版・ベータ版、開発版、PR の CI の成果物、下書き。PR の CI の成果物（R5）と、開発版の「同じ印」（R6）が無い。
  - Q5（検査で止める）の兄弟は 4 つ。依存、取り込んだコード、ツールチェーンが焼き込む std、aar の中の第三者。R7・R8。
- 手順 8（前提の確かさ）:
  - Q1（exported）の文の中身は、確かさ 1 の P9 に載っている（R3）。
  - Q2（exported）の推奨は、確かさ 2 の P8 に載っている。その未確認点は、この席の資格情報で否と分かった（R4）。
  - Q2 の文脈にある「印の push で走る workflow は印の commit の定義」は premises に無い（R4）。
  - P3 の Node（確かさ 1）は、この席で lock を読んで確かめた（R8）。
- 構成の記号（S-01 など）: `grep -nE '\b(S|C|V|D|A)-0[0-9]\b' deep-questions.json` は 0 件で、該当なし。ST52 の決定の番号（D21・D22・Q13・IMP-01・K1・R30）は参照としてだけ出ている。
- 「著作者は本人 1 人」（§5・Q1 の context）: `git log --format='%an'` の人は「ライ」だけで、ほかは AI の Co-Authored-By。repo の中の第三者のコードは `collector-android/gradlew`・`gradlew.bat`（Apache-2.0。配布物に入らない）だけ。前提は成り立つ。

## C（聞かずに既定で決める）6 項目の判定

| C | 判定 |
|---|---|
| 許可の範囲は `com.google.android.gms` の群（Firebase は含めない） | **否**。群には広告（`play-services-ads`）と Firebase Analytics の本体（`play-services-measurement*`）が入る。そのため「群」と「Firebase を含めない」は両立しない。広い側は exported → R2 |
| 許可は本体全体の `LICENSE` 1 枚 | 可。§5「本体（S-01 / C-01 / C-02 / V-01 / D-01）の AGPL-3.0 に…書き足す」が決めていて、幅が無い |
| 携帯の依存の検査の道具は design で選ぶ | 可。失うものは無い（道具は替えられる）。Google の `oss-licenses-plugin` を選んだ場合でも、画面の部品 `play-services-oss-licenses` は gms 群なので、R2 の範囲の決め方に効く |
| 一覧は配布物ごとに組み立て時に生成し、`check-artifacts.sh` が確かめる | 条件付きで可。生成の入力を `cargo metadata` と gradle の依存に限ると、std と aar の中の第三者が漏れる → R7 |
| CONTRIBUTING・CLA は受け入れ条件を変えず、許可の下で受けることを書き足す | 可。CLA の 2-2（AGPL 以外の条件でもライセンスできる）が、許可を付ける権利を与えている。外部の貢献者は 0 人（git log） |
| gms の aar の `third_party_licenses.txt` を携帯の一覧に含める | 可。ただし AGP は aar の根のファイルを APK に入れないので、取り出しの手順が要る → R7 |

---

## R1. 本物の公開（最初の印・正式な鍵での署名・本人のサーバの切り替え）が ST53 の確認の「中」か「後」かが決まっていない。ST53 の完了の判定にも無い
- 成果物: openspec/changes/st53-license-bundle/deep-questions.json
- 根拠:
  - docs/handoff/ST53.md:18 は「最初の印と開発版の公開は、ST53 の確認が終わった後に本人が始める」と言う。同じ節の docs/handoff/ST53.md:20 は「本物の印の公開、正式な鍵での署名、本人のサーバの切り替えは ST53 の確認で見る（Q13 / D22）」と言う。
  - docs/stories/ST52.md:56 は「本物の印による公開は、ST53 の確認の後」と言う。docs/stories/ST52.md:58 は「正式な鍵での署名は、本人が鍵を Environment に置いた後の ST53 の確認で見る」と言う。
  - openspec/changes/archive/2026-11-02-st52-release-channels/design.md:191 は「本物の公開・正式な鍵・本人の環境の切り替えは ST53 の確認で見る」と言う。
  - docs/stories/ST53.md の「完了の判定」3 項目には、どれも無い。
  - 正式な鍵での署名は `release.yml` の `publish`（印 `v*` の push でだけ走る。.github/workflows/release.yml:2-4）でしか起きない。したがって「ST53 の確認で正式な鍵の署名を見る」には、ST53 の確認の中で本物の印を打つ必要がある。打てばその印は消せず動かせない（ST52 の D2）。
  - Q2 の context（deep-questions.json:42）は「ST53 の確認の後」と「ST53 の確認で見る」を並べて書くだけで、どちらかに決めていない。
- kind: conflict
- loss: exported
- 提案: A の問いを 1 つ足す。問いは「ST53 の確認バッチで、本物の印（`v0.1.0-beta.1`）を打って正式な鍵での署名と公開を見るか」。選択肢と帰結は次のとおり。
  - 見る場合: ST53 の完了の判定に足す。Q2 の守りと許諾の同梱は、確認の前に入っていることになる。
  - 見ない場合: ST52 の完了の判定の「正式な鍵での署名」を、どの Story の、いつの確認に移すか（stories.json の書き直し先）を決める。
  - どちらでも、本人のサーバの切り替えを誰の確認で見るかを同じ問いで閉じる。
- 処置: escalated —— A の問い Q4（conflict・loss: exported）を足した。確認バッチの間は ST53 が main に入っておらず、公開の判定が main の祖先を求める（P13）ので、確認の中で打つ印には許可も一覧も入らない。推奨は「main に入った後に本人が行う」。ST52 の完了の判定の書き直しは record の段で答えに合わせて行う


## R2. C の「`com.google.android.gms` の群（Firebase は含めない）」は内部で食い違っている。群には広告の SDK と Firebase Analytics の本体が入る。許可の範囲の決め方は扉 35 が決めておらず、広い側は取り消せない
- 成果物: openspec/changes/st53-license-bundle/deep-questions.json
- 根拠:
  - `curl https://dl.google.com/android/maven2/com/google/android/gms/group-index.xml` を取った（2026-11-02）。群には `play-services-ads`・`play-services-ads-lite`・`play-services-measurement`・`-measurement-api`・`-measurement-sdk`・`play-services-oss-licenses` が入っている。
  - `firebase-analytics-24.0.0.pom` の依存は `com.google.android.gms:play-services-measurement`・`-measurement-api`・`-measurement-sdk` で、Firebase Analytics の実装は gms 群にある。
  - `play-services-ads-24.0.0.pom` の `<license>` は「Android Software Development Kit License」（P1 と同じ）。
  - 扉 35（docs/requirements.md の §5 扉 35）が決めているのは「Google Play 開発者サービスのライブラリに限る」までで、何で数えるか（maven の群・名前の型・位置の取得に要るもの）は決めていない。
  - Q1 の context（deep-questions.json:13）は「範囲の書き方は扉 35 のとおり `com.google.android.gms` の群…Firebase は含めません」としていて、問いにしていない。
- kind: irreversible
- loss: exported
- 提案: A の問いを足す（または Q1 に範囲の選択肢として入れる）。
  - 選択肢: ①maven の群 `com.google.android.gms` の全部（広告と Firebase Analytics の実装も入る） ②`play-services-location` とそれが引く gms の部品に限る ③Google が「Google Play services」と呼ぶ一覧に限り、広告と measurement を名指しで外す。
  - 狭い側は、本人と CLA の下で次の版から広げられる。広い側は、配った版について取り消せない（フォークが広告を足して配れる）。
  - 検査の許可（NFR-26）と `LICENSE` の文が同じ範囲を指すことを、design で機械の照合にする。
- 処置: escalated —— A の問い Q2（許可が受ける範囲を何で数えるか）を足した。C から外した。P10 を premises に足した（group-index.xml を取って確かさ 5）


## R3. Q1 は許可の文の「言葉」だけを問い、外に出る文の中身を本人に見せていない。中身の型は確かさ 1 の P9（FSF の FAQ を取れていない）に載っている
- 成果物: openspec/changes/st53-license-bundle/deep-questions.json / openspec/changes/st53-license-bundle/premises.md
- 根拠:
  - deep-questions.json:7-33（選択肢は英語・日本語・両方）。
  - premises.md の P9 は「確かさ 1」「何の決定も載せない」としている。一方で Q1 の context（deep-questions.json:13）は「文の型に倣います」と書いていて、配る文の中身が P9 に依っている。
  - FSF の型には、本人が決めるべき分岐がある。
    - 「(or a modified version of that library)」を入れるか。改変した gms と組み合わせてよいかが変わる。
    - 「{Corresponding Source … shall include the source code for the parts of [library] used}」の括弧の節を外すか。gms はソースが無いので、入れると許可が空になる。
    - 版を限るか（gms の将来の版を含むか）。
  - Q1 の推奨「`LICENSE` の先頭に置く」は、`LICENSE` の全文の照合で許諾を判定する道具（GitHub の表示など）の結果を変えうる。確かめていない。
- kind: premise
- loss: exported
- 提案:
  - premises の P9 を取りに行き直し、確かさを 3 以上にする。
  - Q1 の context に配る英文の案そのもの（範囲の名前は R2 の答えに依る）を載せ、上の 3 つの分岐を選択肢の detail か別の問いで本人に見せる。
  - 置き場所（`LICENSE` の先頭か、別のファイルか）は B に分けてよい。
- 処置: escalated Q1 —— 英文の案そのものと 3 つの分岐（改変した版を含めない・ソースを求めない・将来の版を含む）を context に載せ、問いを「この案で正本にするか」にした。P9 は取りに行けず 1 のまま、決定を載せないと premises に書いた。置き場所（`LICENSE` の先頭）は案の一部として示し、問いに分けない


## R4. Q2 の推奨の守り（必須の承認者 ＋ エージェントの設定で承認の API を拒む）は、同じ資格情報で Environment とルールセットを書き換えられる経路を数えていない。この席の資格情報は repo の admin で、P8 の未確認点は否と分かる
- 成果物: openspec/changes/st53-license-bundle/deep-questions.json / openspec/changes/st53-license-bundle/premises.md
- 根拠:
  - この席（本人の資格情報で動くエージェント）で次の 2 つを確かめた。`gh auth status` → Token scopes: `repo`, `workflow`。`gh api repos/rai-256/ashiato --jq .permissions` → `{"admin":true,…}`。
    admin の資格情報は、Environment の保護の規則（必須の承認者）の書き換え・外し、ルールセットの削除、保留中の配備の承認を、どれも API で行える。
  - Q2 の推奨（deep-questions.json:47-51）が数える経路は「承認の API」だけで、選択肢の `irreversible` も同じ。
  - P8 は確かさ 2 で、「エージェントが承認の API を呼べるかは確かめていない」と書く。資格情報の権限から、呼べる側だと読める。
  - Q2 の context の「エージェントは本人の資格情報で動くので、本人と分けられません」は事実ではなく選択。エージェントに admin の無い別の資格情報（admin を持たない fine-grained token か GitHub App）を持たせる選択肢が、一覧に無い。
  - IMP-01 の unknowns の 3 つ目「印の push で走る workflow は印の commit の定義」（archive の go.json IMP-01）は、Q2 が問いになる根拠なのに premises に P が無い。
- kind: irreversible
- loss: exported
- 提案:
  - Q2 の推奨の `irreversible` と context に「同じ資格情報で保護の規則そのものを外せる」を書く。
  - 選択肢「エージェントの資格情報を本人から分ける（admin と配備の承認を持たない token。印 `v*` の作成と承認は本人の資格情報だけ）」を足す。
  - premises に「印の push の workflow は印の commit の定義」と「admin の token で Environment の保護を外せる」を P として足し、使い捨ての Environment で走らせて 4 にする。
- 処置: escalated Q3 —— 管理者の資格情報で承認・規則の書き換え・ルールセットの削除ができることを context に書き、推奨を「承認者＋エージェントの資格情報を管理と承認の権限の無いものに替える」に直した。旧推奨は 2 つ目の選択肢にし、irreversible に規則を外せる経路を書いた。premises に P11（印の commit の workflow）と P12（admin・権限を絞った token）を足した


## R5. 公開のリポジトリで、PR ごとの CI の成果物（gms 入りの APK を含む）がもう 90 日間置かれている。許諾も一覧も入っていない。Q4 の「4 つの配布物」は、この経路を数えていない
- 成果物: openspec/changes/st53-license-bundle/deep-questions.json / openspec/changes/st53-license-bundle/premises.md
- 根拠:
  - .github/workflows/ci.yml:364-365・:385-386・:401-402 が `package-linux`（APK を含む）・`package-windows`・`release-package` を `upload-artifact` に上げる。retention の指定は無い。
  - `gh api "repos/rai-256/ashiato/actions/artifacts?per_page=100"` を走らせた。期限内の `package-linux`・`release-package` は 14 件で、最新は 2026-11-02T09:00:15Z 作成・2027-01-31 期限・10.6 MB。
  - P7 で、リポジトリは PUBLIC。
  - Q4 の context（deep-questions.json:88-108）は、組み立てたリリースの 4 つだけを数えている。
  - 公開のリポジトリの成果物を誰が落とせるか（GitHub にログインした人なら誰でも、と読む）は確かめていない。
- kind: premise
- loss: exported
- 提案:
  - premises に「公開のリポジトリの CI の成果物を、第三者が落とせるか」を P として足して確かめる。
  - 落とせるなら、Q4 の context に「同梱は `build.sh` に置くので、PR の成果物・下書き・開発版にも同じものが入る」を書く。
  - ST53 が merge されるまでの間に成果物の retention を縮めるかを、C か B で決める。
- 処置: escalated Q6 —— 同梱は `build.sh` に置くので PR の成果物・下書き・開発版にも入ること、既存の許諾なしの成果物 14 件は ST53 の merge 後に消すことを context に書いた（C。design に渡す）。第三者が落とせるかは確かめずに、落とせる側として扱う（消す処置はどちらでも安い）


## R6. 開発版は印 `nightly` が毎日動く。FR-127 の「対応するソースは同じ印のリポジトリで示す」を開発版でどう示すかの問いが無い
- 成果物: openspec/changes/st53-license-bundle/deep-questions.json
- 根拠:
  - docs/requirements.md の FR-127（「同じ印（タグ）のリポジトリで示す」）。
  - .github/workflows/nightly.yml:75-83 は、印 `nightly` を動かし、前日の配布物を消す。docs/release.md:45-46 も同じ手順を書いている。
  - 開発版の配布物の版の印は `<版>+<40 桁の commit>`（tools/release/check-artifacts.sh:31 の `artifact_stamp`）。
  - 既に落とされた開発版の配布物は、配布物に書いた「印 `nightly`」が翌日には別の commit を指す。
  - Q4 の context（deep-questions.json:93）は「同じ 1 枚に書く」とするだけで、動く印を扱っていない。
- kind: conflict
- 提案: B の問いを足す（推奨は「開発版は印でなく commit（版の印に入っている 40 桁）でソースの場所を示す。正式版・ベータ版は印」）。配布物に書いたソースの場所は、配った後は直せない（その版の写しに残る）。
- 処置: escalated —— B の問い Q7（開発版は commit で示す、を推奨）を足した。開発版は本人の手元だけで毎日入れ替わるので B とした


## R7. 依存の一覧を `cargo metadata` と gradle の依存から作ると、全ての Rust の実行ファイルに焼き込まれる std（MIT / Apache-2.0）が漏れる。gms の aar の中の第三者の一覧は、APK に入らない場所にある
- 成果物: openspec/changes/st53-license-bundle/deep-questions.json（C「一覧は配布物ごとに組み立て時に生成」）
- 根拠:
  - `cargo metadata --format-version 1 --all-features --offline` の 357 部品に `std`・`core`・`alloc` は無い（この席で走らせた）。サーバと PC の収集アプリは std を静的に含む。
  - `unzip -l play-services-location-21.3.0.aar` の結果、`third_party_licenses.txt`（367,213 バイト）と `.json` は aar の根にあり、`res/`・`assets/` の下ではない。json の中身は Kotlin・Guava・Protocol Buffers・Tink・J2ObjC など 21 件。
  - Q4 の推奨（deep-questions.json:96）は「その配布物に入る依存の一覧」としていて、何を入力に作るかを書いていない。
- kind: technical
- loss: exported
- 提案:
  - 問いは足さない。C の文を「依存の検査が見ない、焼き込まれた部品（Rust の std、aar の中の第三者）も一覧に入れ、`check-artifacts.sh` が std の項目と gms の第三者の一覧の有無を確かめる」に直して design に渡す。
  - 抜けたまま配ると、その版の一覧の欠けは直せない（exported）。
- 処置: escalated Q6 —— 一覧の入力に Rust の標準ライブラリと aar の中の第三者の一覧を含めることを Q6 の context に書いた（C。design で `check-artifacts.sh` が両方の有無を確かめる）


## R8. Q5 は衝突ではない（不要）。web の lock の 306 部品に他人の GPL / AGPL は 0 件で、絞っても既存の許可は 1 つも落ちない。一方、§5 の「取り込んだコードも検査が止める」は、依存の検査では成り立たない
- 成果物: openspec/changes/st53-license-bundle/deep-questions.json / openspec/changes/st53-license-bundle/premises.md
- 根拠:
  - `web/package-lock.json` の `packages` を読んだ。license に `GPL` を含むのは根（`""`、自分の `AGPL-3.0-only`）だけ。この worktree の lock は `~/dev/ashiato2/web/package-lock.json` と md5 が同じ。
  - P3（Rust）と合わせると、自分の部品を名前で通して他人の GPL / AGPL を止めても、今の依存は 1 つも落ちない。Story の「既存の許可一覧を壊さない」と §5 は両立する。
  - P3 の Node は確かさ 1（deep-questions.json:116 の「確かめていません」）のままで、Q5 はその未確認を衝突の根拠にしている。
  - §5「他人の GPL / AGPL のコードを取り込むと…NFR-26 の検査が止める」（docs/requirements.md §5「配布の方針」）について。repo に写して置いたコードは依存の検査に現れない。いま repo の中にある第三者のコードは gradlew（Apache-2.0）だけ。
- kind: conflict
- 提案:
  - Q5 は問いから外し、C（自分の部品を名前で通し、他人の GPL / AGPL は止める）にする。P3 の Node を lock の読みで 3 に上げる。
  - §5 の「取り込んだコード」の部分を、依存の検査では止められないと premises か design に書く。検査を足すか（SPDX の見出しが本体のもの以外のファイルを落とす）は design で決める。
- 処置: fixed D6 仮 —— Q5（旧）を問いから外し、自分の部品は名前で通して他人の GPL / AGPL は止める形を design の D6（仮）にした。P3 の Node は lock の読みで 4 にした。§5 の「取り込んだコード」は依存の検査で止まらないことを D5（仮）に書いた（表示を持つ写したコードは表示の検査が止め、表示の無いものは止められない。Risks にも書いた）


## R9. Q4・Q7 の推奨は「後で足せる」に倒しているが、足す先が書かれていない
- 成果物: openspec/changes/st53-license-bundle/deep-questions.json
- 根拠:
  - Q4 の推奨の detail（deep-questions.json:97）は「携帯の画面で見せるのは Play ストアへ出す前に足せる」と書く。
  - Q7 の推奨（deep-questions.json:157-158）は「一般向けの配布の前に、画面を持つ Story で足す」と書く。
  - どちらも、受ける Story・申し送りの置き場（docs/handoff/ST<NN>.md）が無い。
  - upstream_misses の観点「選択肢が『X を後のために残す』と言うときは、後で X をどう見つけるかを問う」に当たる。
- kind: defer
- 提案: 推奨の文に受け手を書く（例: 申し送りを docs/handoff/ の該当 Story に置く）。受け手の Story が無ければ「/requirements から足す」と明記する。問いの種類は B のままでよい。
- 処置: fixed deep-questions.json —— Q6・Q9: 推奨の受け手を「要件の未決として残す（/requirements から足す。受ける Story はまだ無い）」と書いた。record の段で要件に未決を足す


## R10. Q4 の選択肢 3（名前・版・許諾の名前だけ）は、選んで配ると取り消せない欠けが出る。問いは open で、その選択肢に `irreversible` が無い
- 成果物: openspec/changes/st53-license-bundle/deep-questions.json
- 根拠:
  - deep-questions.json:105-107。detail は「MIT・BSD の『全文と著作権の表示を入れる』を満たさない恐れ」と書くが、`irreversible` の欄は無い。
  - Q4 の why（deep-questions.json:92）は「次の版で形を変えれば戻ります」と書くが、配った版の欠けは次の版では直らない（exported）。
  - 推奨（選択肢 1）は失わない側で、安い。
- kind: irreversible
- loss: exported
- 提案:
  - 選択肢 3 に `irreversible`（「この形で配った版は、許諾が求める表示を欠いたまま出回る」）を書く。
  - 失わない側（選択肢 1）が安いので、Q4 は「全文を入れる」を C にし、問いを「携帯に許諾の画面を足すか」（B）だけに縮めてよい。
- 処置: escalated Q6 —— 全文と著作権の表示を入れることを C にし（問いにしない）、Q6 を「携帯に許諾の画面を足すか」だけに縮めた


## R11. Q3 の推奨の「全部を書き換え」は第三者の Apache-2.0 のファイルまで含む。件数も実物と合わない
- 成果物: openspec/changes/st53-license-bundle/deep-questions.json
- 根拠:
  - `git grep -h -o "SPDX-License-Identifier: [^ *]*" | sort | uniq -c` の結果は、`AGPL-3.0-only` が 506、`Apache-2.0` が 2。
  - Apache-2.0 の 2 つは collector-android/gradlew:4 と gradlew.bat:2 の「Copyright … the original authors」で、gradle の配布物。
  - deep-questions.json:68 は「約 490 ファイル」と書き、:73 は「全部を…に書き換え」としている。
- kind: technical
- 提案: 推奨の文を「`AGPL-3.0-only` を宣言している 506 ファイル（gradle の wrapper の 2 つは除く）」に直す。問いの種類（open）は妥当。
- 処置: fixed deep-questions.json —— Q5: 件数を 506 にし、gradle の wrapper の 2 つ（Apache-2.0）を書き換えないと書いた
