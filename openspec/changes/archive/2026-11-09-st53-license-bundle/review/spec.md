# ST53 spec レビュー（第 1 回の先読み `spec --draft`。推奨の答えで書いた版）

対象: `openspec/changes/st53-license-bundle/`（proposal / specs/distribution / specs/verify-environment / design / tasks / premises / deep / deep-questions.json / review/deep.md / provisional.jsonl）、`docs/stories/ST53.md`。
確かめた木: `/home/yosis/dev/ashiato2-up-st53`（`docs/st53-upstream`。origin/main = `da14bd6b`）。

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate st53-license-bundle --strict` | `Change 'st53-license-bundle' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0。再生成との一致も通る） |
| `python3 scripts/check_scenarios.py . st53-license-bundle` | `scenarios: FAIL (担保なし 32 件 / 名無しの確認待ち 0 件)`（rc=1）。FAIL の 32 件はどれもこの change の ADDED の Scenario（distribution 29・verify-environment 3）。実装の前の下書きなので、**これは期待どおりの状態で、欠陥ではない**。32 本は全部 tasks.md のどこかの Task が名指ししている（全 Scenario 名を tasks.md に grep して欠け 0）。ほかの `[warn]` はこの change と関係の無い既存の印 |
| `python3 scripts/review_triage.py . st53-license-bundle` | `triage: OK`（指摘 11 件 / 仮決め 1 件 / 要件へ戻すもの 0 件） |
| `python3 scripts/spec_overlap.py candidates . st53-license-bundle` | `[]`（0 件） |

triage の「要件へ戻すもの 0 件」について: proposal（Impact の「record の段に渡すもの」）は、Q6・Q9 の要件の未決 2 つと ST52.md の完了の判定の書き直し（Q4）を record の段へ回している。triage の数には入っていないので、record の段がこの 3 つを落とさないように、ここに書いておく（R ではない）。

---

## R1. PR の組み立てと下書きの配布物の検査が、新しい SOURCE.txt の決まりで落ちる（`check-artifacts.sh` の既定のチャンネルは `release`）
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md / tasks.md
- 根拠: `tools/release/check-artifacts.sh:68` は `channel=release` が既定。`tools/release/draft.sh:32` は `check-artifacts.sh "$work" --allow-debug-key --expect-version "$src"`、`.github/workflows/ci.yml:400`（`package-artifacts`）は `check-artifacts.sh package/all --allow-debug-key --expect-version …` で、どちらも `--channel` を渡さない。spec の IF（spec.md:124）と Task 7.1（tasks.md:139）は「`--channel release|beta` なら `Ref: v<期待する版番号>`、それ以外なら `Commit:`」。一方で PR・下書きの組み立ては `ASHIATO_SOURCE_REF` を持たないので `Commit:` を書く（D9、design.md:131）。そのため Task 7.1 を文字どおり実装すると、PR ごとの `package-artifacts` と、確認バッチの Task 11.1（`draft.sh <run-id> 0.1.0-beta.1`）の両方が落ちる。`draft.sh` と `ci.yml:400` を直す Task はどこにも無い
- kind: technical
- 提案: spec の IF の「それ以外」を、検査が受け取る入力で言い直す（例: PR・下書きは `--channel pr` を渡す、または `--allow-debug-key` のときは `Commit:` を求める）。tasks には `draft.sh:32` と `ci.yml:400` の呼び出しの直しと、その検証を足す。Scenario「ソースの場所がチャンネルの決まりと合わない配布物は、公開しない」と対になる「PR の組み立ての検査は `Commit:` で通る」も足す
- 処置: fixed D9 仮 —— `--allow-debug-key` の検査（PR の package-artifacts・draft.sh）と nightly は `Commit:`、それ以外の release|beta は `Ref:` と spec の Requirement に書き、Scenario「PR の組み立てと下書きの配布物の検査は、commit のソースの場所で通る」を足した。既定のチャンネルと呼び出しは変えない（tasks 7.1）

## R2. 重い自己試験の Windows の zip は印 1 つだけの偽物で、「4 つの配布物は…」の Scenario を Linux の runner で満たせない
- 成果物: openspec/changes/st53-license-bundle/tasks.md（7.1）
- 根拠: `tools/release/check-artifacts.sh:176-178` は Windows の 2 つの zip を `stamp` のファイル 1 つから作っている（本物の組み立てではない）。Task 7.1（tasks.md:146）は、この重い自己試験に `4 つの配布物は、許諾・依存の一覧・ソースの場所を持つ` の印を置くと書いているが、偽の zip に 3 点を入れることは書いていない。spec の WHEN（spec.md:130）は「配る形の 4 つの配布物を組み立てて」。さらに、既存の失敗の Scenario（`.dirty`・`.debug`・資格情報。check-artifacts.sh:189-215）は出力に `.dirty` などがあるかを grep している。3 点の検査が先に落ちると、その grep が外れて自己試験が壊れる
- kind: technical
- 提案: 7.1 に「偽の Windows の zip にも 3 点を入れる」「既存の失敗の Scenario が、意図した理由で落ちることを保つ」を書く。本物の 4 つで確かめる印は、本物の 4 つが揃う `package-artifacts` の job（ci.yml:400）の手順に置くか、spec の WHEN を「Linux で組める 2 つ＋写しの 2 つ」に合わせる
- 処置: fixed 7.1 —— 偽の Windows の zip にも 3 点を入れ、既存の失敗の Scenario が意図した文で落ちることを保つと書いた。spec の WHEN を「4 つの配布物の置き場に検査を当てる」にし、本物の 4 つは package-artifacts の job が毎回見ると書いた

## R3. 先頭 5 行の決まり（D5）では、gradle の wrapper の例外にも、行 21 に表示を持つソースにも届かない
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:37・47-49 / design.md D5 / tasks.md 2.1・2.2
- 根拠: `git grep -n "SPDX-License-Identifier:"` で、`collector-android/gradlew:18`・`collector-android/gradlew.bat:16`・`crates/server/src/coverage/tests/mod.rs:21` の 3 つは 6 行目より後にある。
  - wrapper の 2 つは先頭 5 行の検査に読まれない。例外の一覧（design.md:83）の wrapper の行は効かず、Scenario「例外の一覧のファイルは…」の THEN（spec.md:49「`gradlew`・`gradlew.bat` は `Apache-2.0` のまま」）を検査は確かめていない
  - `coverage/tests/mod.rs` の `AGPL-3.0-only` は、Task 2.1 の書き換え（先頭 5 行）にも Task 2.2 の検査にも入らず、古い表示のまま残る。それでも検査は rc=0 になり、THEN「ほかの表示はどれも `AGPL-3.0-only WITH …`」と食い違う
  - D5 の「他所の表示を持ったまま写したファイルは、この検査が止める」（design.md:85）は、この木にある第三者のファイル（gradlew。表示が 18 行目）で成り立っていない
- kind: technical
- 提案: 2.1 で `coverage/tests/mod.rs` の表示を先頭へ移す（か書き換える）。窓を広げる（例: 先頭 25 行か全行の `SPDX-License-Identifier:`）か、wrapper を例外の一覧から外して Scenario の THEN から消すか、どちらかに決めて design・spec・tasks をそろえる。D5 の「止める」の主張は、窓の大きさに合わせて直す
- 処置: fixed D5 仮 —— 窓を先頭 25 行に広げ（gradlew:18・gradlew.bat:16 が入る）、`coverage/tests/mod.rs` の表示を 2.1 で先頭へ移す。spec・design・tasks を 25 行にそろえた

## R4. Task 2.1 の件数 503 は、tasks.md 自身が一致することで当たっている
- 成果物: openspec/changes/st53-license-bundle/tasks.md:51 / proposal.md（Impact「503 ファイル」）/ design.md:78
- 根拠: symlink でない追跡ファイルのうち、先頭 5 行に `SPDX-License-Identifier: AGPL-3.0-only` を持つものは 505。移行の 3 つを除くと、Task 2.1 で書き換えるのは 502。`git grep -l "SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services"` が 503 になるのは、tasks.md（:47・:51 にこの文字列がある）が commit されて数に入るから。design と proposal の「506 − 3 = 503」は、R3 の `coverage/tests/mod.rs`（行 21）を含んだ数なので、書き換える側の数と合わない。R3 に従って mod.rs も直せば 504 になり、検証が落ちる
- kind: technical
- 提案: 件数の検証を、`git ls-files` を起点にして `docs/`・`openspec/` を除いた先頭 N 行の数にするか、`tools/check-spdx.sh headers` の rc=0 と「古い式の表示が 0（例外の 3 つを除く）」の検証に替える。proposal・design の件数は、R3 の処置の後の実数に直す
- 処置: fixed 2.1 —— 件数の検証を `tools/check-spdx.sh headers` rc=0 と「例外の外に古い表示が 0（docs/・openspec/ を除く）」に替えた（今の木で 498 を数えることを走らせて確かめた）

## R5. R10 で取り消せないとした「全文と著作権の表示」に、Scenario も配布物の検査も無い
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:115・117-120・123
- 根拠: Requirement は部品ごとに「名前・版・許諾の式・許諾の全文と著作権の表示」を書くとし（:115）、サーバの一覧には画面の Node の部品も入れるとしている（:118）。しかし Scenario（:138-144）が確かめるのは `rust-std` の項と gms の中の第三者の見出しだけ。Task 7.1 の配布物の検査も、この 2 つの見出しだけを見る（tasks.md:137-138）。全文が無いときに組み立てを落とすこと（D8、design.md:122）は design にしか無い。review/deep.md R10 は、全文が欠けた版は配った後に直せない（irreversible）としていた。サーバの一覧に Node の部品が入ることも、どの Scenario も確かめていない
- kind: technical
  （spec の段で kind を `irreversible` から直した: 失うものを生むのは決定ではなく検査の欠けで、決定そのもの（Q1 / Q3 / Q6）は第 1 回の問いで人間に返してある。`loss` の欄も無い）
- 提案: Scenario を足す（「許諾の全文も著作権の表示も見つからない部品があれば、依存の一覧を組まずに組み立てが落ちる」「サーバの依存の一覧に、画面の dev でない Node の部品の項がある」）。tasks の 5.1 に、それぞれの自己試験を名指しで足す
- 処置: fixed 5.1 —— Scenario「許諾の全文も著作権の表示も見つからない部品があれば、組み立てが落ちる」「サーバの依存の一覧は、画面に入る Node の部品を含み、開発だけの部品を含まない」を足し、notices.sh の自己試験に名指しした

## R6. 「POM が許諾を持たない部品は、親の POM の許諾で判定する」は、書いてあるとおりには試されない
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:87-90 / tasks.md 4.2・4.3
- 根拠: Scenario の GIVEN は「許諾を持たない POM と、Apache-2.0 の許諾を持つその親の POM」。しかし Task 4.2 の自己試験は「4.1 が親から読んだ結果の形の行」を偽の一覧に置くだけ（tasks.md:96）で、親を辿る処理を通らない。親を辿る gradle の task（4.1）を確かめる 4.3 は「task が親から読んだ行に印を付けて出すなら、それを数える」（tasks.md:101）と条件付きで、コマンドも終了条件も無い。親を辿る処理が無くても、印の付いた試験は緑になる
- kind: technical
- 提案: 4.1 の task の出力に、親から読んだ行の印（例: 4 列目 `parent`）を必須にする。4.3 の検証に、その行が 1 本以上あることを数えるコマンドを書く（親を辿る部品が今の依存に無ければ、偽の maven の置き場で task を走らせる）。Scenario の印は、親を辿る処理を通る試験に置く
- 処置: fixed 4.3 —— task の出力に出所の列（self / parent）を必須にし、使い捨ての maven の置き場（`-Pashiato.extraRepo`）で親を辿る処理を通す自己試験に印を置いた。Scenario の GIVEN もそれに合わせた

## R7. Requirement の本文だけが言っていて、Scenario が言っていないこと
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md
- 根拠:
  - :11「AGPL-3.0 の全文が今の `LICENSE` の全文と違えば落とす」: 落ちる側の Scenario は許可の文を変えたもの（:21-24）だけで、AGPL の本体を変えた場合の Scenario が無い（Task 1.2 も試さない）
  - :12 の README（参考訳・正本は英文）と CONTRIBUTING（CLA の受け入れ条件は変えない）: Scenario が無い。Story の「壊してはいけないもの」の CONTRIBUTING の受け入れ条件は、Task 1.1 の検証（`docs/CLA.md` の差分 0 と grep）にも入っていない
  - :67「ほかの許可の一覧は減らさない」（Story の壊してはいけないもの）: Scenario が無い。Task 3.1 の検証は `AGPL-3.0-only` が無くなったことだけを grep する（tasks.md:75）
  - :68「`web/node_modules` が無い」節: Scenario「検査する対象が無い節は、通さずに落ちる」（:106-109）は携帯の節（gradle）だけ
  - :165「公開の承認の権限を持たない資格情報にする」・:171「job の中の確かめが守りの本体でないことを手順に書く」: Scenario が無い（前者は D11 のとおり API で読めず、人間の確認待ち 11.2 にだけある）
- kind: technical
- 提案: 少なくとも「AGPL の本体を変えると落ちる」「許可の一覧が `origin/main` の一覧から `AGPL-3.0-only` を除いたものと同じ」（`origin/main` の `tools/check-licenses.sh` の ALLOW と比べる）「`node_modules` が無ければ Node の節が落ちる」の 3 つを Scenario にして、tasks に名指しで足す。CONTRIBUTING の CLA の節は、`origin/main` との差分 0 を Task 1.1 の検証に足す
- 処置: fixed specs/distribution/spec.md —— 「AGPL の全文を変えると落ちる」「README のライセンスの節」「許可の一覧は AGPL-3.0-only を外したほかは前と同じ（17 の値を本文に列挙）」「node_modules が無ければ画面の節は落ちる」を Scenario にし、tasks 1.2・3.1 に名指しした。CONTRIBUTING の CLA の節は 1.1 の検証で origin/main との差 0 を見る。公開の承認の権限と『守りの本体でない』は API で読めないので、前者は人間の確認待ち 11.2、後者は 10.2 の手順書の検証に置いた

## R8. Q1 の決定（この英文の案を正本にする）を、どの Scenario も確かめていない
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:3-28
- 根拠: deep.md:8 の推奨は「この英文の案を正本にする」（案は deep-questions.json の Q1 の context）。spec の Scenario が確かめるのは、`LICENSE` の先頭と正本が一致すること・`but not a modified version` を消すと一致の検査が落ちること・名指しが 4 つであること、の 3 つ。正本の中身が案の英文であることは確かめていない。Requirement が挙げる 3 つの分岐（:7-9。範囲・どの版でもよいが改変は含めない・ライブラリのソースは求めない）も、本文に書いてあるだけ。実装が違う英文で正本を作っても、`LICENSE` の先頭にその文を写せば全部の Scenario が通る。扉 35 は、配った文を取り消せないとしている
- kind: technical
  （spec の段で kind を `irreversible` から直した: 失うものを生むのは決定ではなく検査の欠けで、決定そのもの（Q1 / Q3 / Q6）は第 1 回の問いで人間に返してある。`loss` の欄も無い）
- 提案: Scenario「追加の許可の正本は、深掘りで決めた英文と同じ」を足す（`Listed libraries:` より前の行が、決めた英文と 1 字も違わない。英文は spec の Requirement の本文に逐語で置く。tasks 1.1 は deep-questions.json を参照するのではなく、spec の逐語の文を写す）
- 処置: fixed specs/distribution/spec.md —— 英文を Requirement の本文に逐語で置き、Scenario「追加の許可の正本は、深掘りで決めた英文と同じ」を足した。tasks 1.1 は spec の英文を写す

## R9. Q5 の推奨（deep.md・deep-questions.json）は「506 ファイル」のままで、spec は D4（仮）で 503 にしている
- 成果物: openspec/changes/st53-license-bundle/deep.md:28 / deep-questions.json の Q5 / specs/distribution/spec.md:34-36
- 根拠: deep.md:28 の推奨と deep-questions.json の Q5 の context は「506 ファイル … を `AGPL-3.0-only WITH …` にする」と書き、移行は除いていない。spec と design D4（仮）は、移行の 3 つを書き換えない（design.md:78 も「推奨の文は 506 と書いていた。spec の段で 503 にした」と書いている）。本人が Q5 に「推奨のまま」と答えると、記録に残る答え（506）と仕様（503。移行は `AGPL-3.0-only` のまま）が食い違う。さらに ST53.md の完了の判定の 1 つ目「ソースの SPDX が同じ許諾を示す」も、例外の 3 つで字面どおりには成り立たない
- kind: conflict
- 提案: 問いを本人に出す前に、Q5 の推奨と context を「移行の 3 つと wrapper の 2 つを除く」（件数は R3・R4 の後の実数）に直す。D4（仮）を context で名指しする。完了の判定の文は、record の段で「例外の一覧を除き」と合わせる
- 処置: fixed D4 仮 —— D4 に、Q5 の推奨（506）の上に乗る例外であることと、Q5 で本人が全部の書き換えを補足したら D4 を外すことを書いた。問いの JSON は第 1 回の先読みの digest に結び付くので、この段では書き換えない。GO 資料で D4（仮）として見せる

## R10. 確認バッチの準備の直し（Q8）: 手で入れる案内の文が自分の検証に掛かり、D14 と Scenario は案内を出す場所が食い違う
- 成果物: openspec/changes/st53-license-bundle/tasks.md:171・177 / specs/verify-environment/spec.md:16-23 / design.md D14
- 根拠:
  - Task 9.1 は `android_note` を「エミュレータが無いので入れていない（手で入れる: `adb -s <エミュレータ> install -r <apk>`）」にすると書く（tasks.md:171）。一方、同じ Task の検証は `grep -Eq "adb[^|]*install" tools/verify-prep.sh && exit 1`（:177）で、自己試験の Scenario「準備の台本は…」も「`adb .*install` の行が無い」を見る（:175）。案内の文を verify-prep.sh に書くと、自分の検証が落ちる（今の :64 の案内「`adb install -r $apk`」も同じ型に当たる）
  - D14 は「1 台も無ければ何も出さずに rc=0」（design.md:183）。一方、Scenario「実機だけが繋がっていれば、何も入れない」は、WHEN が「端末の選び方を走らせる」で、THEN に「エミュレータが無いので入れていないことと、手で入れるときのコマンドが示される」（spec.md:17-19）。D14 の関数はそれを示さない
- kind: technical
- 提案: 案内の文（手で入れるコマンド）を `verify-prep-device.sh` の関数が出す形にそろえる（Scenario の THEN を満たす）。verify-prep.sh の検証は「`adb` を呼ぶ行（コマンドとして実行する行）が無い」に絞り、文字列の中の `adb` を除く書き方にする
- 処置: fixed D14 —— 案内の文は `verify-prep-device.sh` の関数が出し、verify-prep.sh は出力をそのまま使う。検証を「呼び出しの行を除いて adb と install の文字が無い」に替えた（tasks 9.1）

## R11. 受け付ける / 断る判定が design にだけある（D7 の「名前が 2 つなら OR」・D9 の「印と版が違えば組み立てを落とす」・D8 の見出しの形）
- 成果物: openspec/changes/st53-license-bundle/design.md:109・130・123 / specs/distribution/spec.md
- 根拠:
  - D7（design.md:109）「1 つの部品が名前を 2 つ以上持てば、どれか 1 つが許可の一覧にあれば通す（`OR` と読む）」: 依存の検査が通す側に倒れる判定。POM に `<license>` が 2 つあっても、どちらでもよい（OR）のか両方を守る（AND）のかは決まっていない。spec の携帯の節（spec.md:62-68）にも Scenario にも無い
  - D9（design.md:130）「`ASHIATO_SOURCE_REF` が `v<版>` と違えば組み立てを落とす」: 配布物を作らない振る舞い。spec に無い（Task 5.3 の自己試験にだけある）
  - D8（design.md:123）の項の形（`== <名前> <版>` → `License:`）と、配布物の検査がその見出し（`== rust-std `・`(gms の中の第三者)`）を字面で読むこと: 配る `THIRD-PARTY-NOTICES.txt` の観測できる形が、design と tasks にしか無い
  - `openspec archive` は main specs しか更新しないので、これらは正典から落ちる
- kind: technical
- 提案: 少なくとも D7 の OR と D9 の組み立ての拒否は、spec の Requirement と Scenario に上げる（「許諾の名前を 2 つ持つ部品は、どれか 1 つが許可されていれば通る」「印が版番号と違う公開の組み立ては、配布物を作らずに落ちる」）。OR の判定が通す側に倒れることを、仮の理由（keeps）に書く
- 処置: fixed specs/distribution/spec.md —— D7 の OR（Scenario「許諾の名前を 2 つ持つ部品は…」）・D9 の拒否（Scenario「印が版番号と違う公開の組み立ては、配布物を作らずに落ちる」）・D8 の見出しの形（Requirement の本文）を spec に上げた。D8 の design から判定の文を外した

## R12. Q3 の決定の要（資格情報 → 承認者 → 鍵の順）を、Scenario も検証も半分しか見ていない
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:166 / tasks.md:196
- 根拠: Requirement は手順の順を「エージェントの資格情報の切り替え → ルールセットと承認者 → 鍵」にするとしている（:166）。Scenario は無い。Task 10.2 の検証（tasks.md:196）は「エージェントの資格情報」が `ASHIATO_SIGNING_KEYSTORE_B64` より前にあることだけを見る。「必須の承認者」（承認者を付ける手順）が鍵より前にあることは見ていない。今の `docs/release.md` は、Environment と Secrets がルールセットより前にある（handoff ST53.md の IMP-01）。ここで順を取り違えると、IMP-01 の経路が開いたまま鍵が置かれる。読まれた鍵は戻せない（spec.md:174）
- kind: technical
  （spec の段で kind を `irreversible` から直した: 失うものを生むのは決定ではなく検査の欠けで、決定そのもの（Q1 / Q3 / Q6）は第 1 回の問いで人間に返してある。`loss` の欄も無い）
- 提案: Task 10.2 の検証に「`check-guards.sh approvals` を走らせる行と『必須の承認者』の節が、鍵（`ASHIATO_SIGNING_KEYSTORE_B64`）の節より前にある」を足す。できれば spec に Scenario「配布の手順書では、承認者の確かめが鍵を置く手順より前にある」を置く
- 処置: fixed 10.2 —— 検証に「`check-guards.sh approvals` と『必須の承認者』が鍵の Secret より前にある」を足した

## R13. 承認を経る公開（Requirement 5）と確認バッチの準備（Q8）が、Story に錨を持たない
- 成果物: docs/stories/ST53.md / openspec/changes/st53-license-bundle/specs/distribution/spec.md:173 / specs/verify-environment/spec.md:8
- 根拠: 2 つの Requirement は、導出元に扉 34 と ST52 の IMP-01・K1 を挙げる。ST53.md の `doors` は `[35]` だけで、`satisfies` は FR-127・NFR-26。完了の判定（ST53.md の 3 項）は、承認者の守りにも、`SOURCE.txt`（対応するソースの場所。FR-127 の後半）にも、確認バッチの準備にも触れていない。INDEX.md:76 にも理由が無い。完了の判定に無いものは、確認バッチにも archive の判定にも出ない。申し送り（docs/handoff/ST53.md）は引き受けを求めているので、引き受けること自体は正しい
- kind: defer
- 提案: record の段で、ST53 の完了の判定に「正式な鍵を読む公開は本人の承認を経る（`check-guards.sh approvals` が通る）」「配布物の `SOURCE.txt` がチャンネルの決まりどおり」「確認バッチの準備はエミュレータにだけ入れる」を足す。`doors` に 34 を足すか、INDEX に申し送りから引き受けた理由を書く
- 処置: fixed deep.md —— 「record の段へ渡すもの」に ST53 の完了の判定と doors（または INDEX の訂正）を書いた。Story は生成物で、record の段が答えに合わせて直す

## R14. Q7（開発版は commit で示す）は、FR-127 の「同じ印（タグ）のリポジトリで示す」の字面を変えるのに、要件へ戻すものに載っていない
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:121・127 / docs/requirements.md:1070
- 根拠: FR-127（requirements.md:1070）は「対応するソースは、同じ印（タグ）のリポジトリで示す」。spec は、正式版・ベータ版以外を 40 桁の commit で示す（:121）。deep-questions.json の Q7 の 2 つ目の選択肢は「FR-127 の文面どおり」で、推奨が字面から外れることを自分で書いている。triage は「要件へ戻すもの 0 件」で、proposal が record に渡すもの（Q6・Q9 の未決、ST52 の完了の判定）にも入っていない
- kind: conflict
- 提案: record の段で、FR-127 に ★（日付）で「開発版・PR の組み立ては組み立ての commit で示す（印 `nightly` は動くため）」を補う。proposal の「record の段に渡すもの」にこれを足す
- 処置: fixed D16 仮 —— design に D16（仮）（開発版・PR の組み立ては commit で示し、FR-127 の字面から逸れる。反転条件つき）を置き、deep.md の「record の段へ渡すもの」に FR-127 の ★ の補足を書いた（Q7 の答えに合わせて record が改訂する）

## R15. PR の組み立ての `SOURCE.txt` の commit は、GitHub が PR のために作る合成の merge commit になる（後から取れるかは未確認）
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:151-154 / design.md D9
- 根拠: `.github/workflows/ci.yml` の `package-linux`（:350-355）は `on: pull_request` で、`actions/checkout` に `ref` を渡していない。この既定では、`refs/pull/<n>/merge` の合成 commit を取り出す。D9 は `git rev-parse HEAD` を書く（design.md:131）。PR の成果物は公開リポジトリで 90 日置かれる（review/deep.md R5）。その commit が PR を閉じた後も clone や fetch で取れるかは、premises に無い。Scenario は「組み立ての印と同じ commit」を見るだけで、その commit を辿れるかは見ない
- kind: technical
  （spec の段で kind を `premise` から直した: 確かめていない事実は premises の P14（確かさ 2）に載せ、どの `loss` のある決定もそれに載らない。PR の成果物の書き方は D9（仮）の反転条件と人間の確認待ち 11.1 で閉じる。人間に問う決定は無い）
- 提案: premises に「PR の成果物の `SOURCE.txt` の commit は、PR を閉じた後も取れるか」を足し、確かさを上げる（人間の確認待ちの 11.1 で、閉じた PR の merge commit を fetch する）。取れなければ、PR の組み立てでは `github.event.pull_request.head.sha` を書く（D9 の反転条件に足す）
- 処置: fixed D9 仮 —— premises に P14（確かさ 2）を足し、人間の確認待ち 11.1 で閉じた PR の merge commit を fetch して 4 に上げる。取れなければ head の commit を書く反転条件を D9 に足した

## R16. 1 つの Scenario に主張を束ねたものがある
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:17-19・47-49・73-75・129-131
- 根拠:
  - :19 は「先頭が正本と同じ」と「AGPL の全文が続く」と「rc=0」を 1 つの THEN に束ねている（落ちる側は前者だけ。R7）
  - :49 は「wrapper は Apache-2.0 のまま」「移行は AGPL-3.0-only のまま」「ほかは全部 WITH の式」を束ねている（wrapper の主張は検査が見ない。R3）
  - :75 は「rc=0」と「通したものは名指しの 4 つだけ」を束ねている
  - :131 は「4 つの配布物 × 3 点」と「LICENSE が木の LICENSE と同じ」を束ねている（R2）
  - どれも、片方が成り立たなくても、もう片方で緑になりうる
- kind: technical
- 提案: 主張ごとに Scenario を分けるか、THEN の各句を、それぞれ別の検査の出力（行）で確かめる形に書き直す（R3・R7 の処置と一緒に）

---

## 観点ごとの該当なし

- 観点 3（置き場）のうち capability の照合: 該当なし（確かめた範囲: proposal の Modified Capabilities `distribution`・`verify-environment` と `specs/` のディレクトリが一致する。INDEX.md:105 の `distribution` 行が ST53 を持つ。`verify-environment` は change が立てた capability で INDEX に行が無い。proposal に理由がある）。specs の中の道具の名前（`tools/release/check-guards.sh` など）は、正典 `openspec/specs/distribution/spec.md` も `tools/stack.sh`・`STACK_RESET=1` などを書く家の流儀に合うので、指摘しない
- 観点 4 の依存順: 該当なし（確かめた範囲: 2.1 の式の書き換え → 3.1 の自分の部品の判定、1.1 の正本 → 4.2 の名指しの読み、4.1 → 5.2 → 6.1 → 7.1 の並び。鍵の計算に当たるものは無い）。人間の確認待ちは 11.1〜11.4 にまとめて印がある
- 観点 5 の観点 8（再生成との一致）: 該当なし（check_chain.py が OK）
- 観点 6: 該当なし（`spec --draft`。`--answers` ではない）
- 観点 7: 該当なし（spec_overlap.py candidates が 0 件。origin/main の `openspec/changes/` は `archive/` だけで、走っている change はこの change だけ）
- 処置: fixed specs/distribution/spec.md —— 束ねていた 4 つを分けた（LICENSE の先頭 / AGPL の全文、例外は「例外として通した 5 つ」の出力 1 つに、携帯の rc / 通した 4 つ、4 つの配布物の 3 点 / LICENSE の一致）


---

# 第 2 回の答えの写し（`spec --answers Q10,Q11,Q12,Q13,Q14,Q15`。2026-11-03）

対象: commit `d69bee48`（`git diff 58924f30 d69bee48 -- openspec/changes/st53-license-bundle/`）。答えの写しの判定は `review/answers.md`。ここには今回の差分の範囲の指摘だけを足す。

## 機械の検査（第 2 回）

| コマンド | 結果 |
|---|---|
| `openspec validate st53-license-bundle --strict` | `Change 'st53-license-bundle' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0） |
| `python3 scripts/check_scenarios.py . st53-license-bundle` | `scenarios: FAIL (担保なし 48 件 / 名無しの確認待ち 0 件)`（rc=1）。48 件はどれもこの change の ADDED の Scenario（実装の前なので期待どおり）。48 本の名前を全部 tasks.md に grep して欠け 0 |
| `python3 scripts/review_triage.py . st53-license-bundle` | `triage: OK`（指摘 27 件 / 仮決め 6 件 / 要件へ戻すもの 2 件） |
| `python3 scripts/spec_overlap.py candidates . st53-license-bundle` | `[]`（0 件） |

答え以外の変更の分け方（書いた側の申告 (a)〜(d) への判定。理由はここに書き、`review/answers.md` には決まった行だけを書く）:
- (a) D17（仮）: 答え以外の変更。中身は design.md:224-229 の D17 の節にある。仮なので合意は引き継がれる。ただし Q14 の答えの「その場合だけ外す」を広げている（R18）
- (b) Scenario「許諾の名前を 2 つ持つ部品は、両方が許可されていれば通る」は D7 の AND の定義（design.md:117「全部の名前が許可の一覧にあるか名指しに当たるときだけ通す」）の通る側、「PR の組み立ての head の commit が 40 桁の 16 進でなければ、配布物を作らずに落ちる」は D9 の本文（design.md:143「40 桁の 16 進でなければ落とす」）。どちらも答えの写しで、答え以外の変更に数えない
- (c) Risks の「PR の配布物の中身は head から組んだものと一字一句同じとは限らない」（design.md:240）: 振る舞いを変えない、Q14 の答えの帰結の記述。答え以外の変更には数えないが、答えの前提に関わるので R17 で人間に返す
- (d) Open Questions の P14 の行（design.md:250）: D9 の本文（design.md:154）と P14 の再掲。答え以外の変更に数えない

## R17. Q14 の推奨の選択肢は、選んだ後に分かった 2 つの帰結を見せずに選ばれている（組んだソースと head が違う・前の head を後から取れるか）
- 成果物: openspec/changes/st53-license-bundle/design.md（D9・Risks）/ premises.md（P14）/ tasks.md（11.1）
- 根拠:
  - 組んだソースと head が違う: `.github/workflows/ci.yml:355`・`:377` の `package-linux`・`package-windows` の checkout は `ref` を渡さないので、合成の merge commit（head とその時の main を合わせたもの）から組む。`SOURCE.txt` は head を指す（design.md:143）。design.md:240 の Risks がこの差を書いている。FR-127（docs/requirements.md:1070）は「対応するソース」を示すと言い、開発版の Scenario（spec.md:222）は「組み立ての印と同じ commit」を求めるのに、PR の組み立てだけ `SOURCE.txt` と組んだ木が一致しない。`deep-questions-r2.json` の Q14 の context は「配布物の組み立ての印は checkout した commit（PR では合成の merge commit）を持つ」と、組む木が merge commit であることは書くが、その結果 `SOURCE.txt` が組んだ木の対応するソースを指さなくなることは、context にも推奨の選択肢の detail にも書いていない
  - 前の head を後から取れるか: 推奨の detail は「閉じた後も `refs/pull/<n>/head` で残る見込みが高い」。record の段の整合の確認 C1 が、この repo の PR の枝は rebase して push し直すので、push し直す前の head は辿れなくなるかもしれないことを見つけた（design.md:154、premises.md:23 の P14 は確かさ 2）。確かめは 11.1（merge の後。tasks.md:225-230）で、その前に PR の成果物（90 日、誰でも落とせる）が出る。別の席が D9 を否とした理由（`provisional.jsonl` の D9 の reason「確かめは確認バッチ 11.1（merge の後）で、その前に成果物が出る」）と同じ形が、merge commit から head に替えた後も残っている
- kind: premise
- 処置: escalated —— 帰結 1（組んだ木と head の食い違い）を deep.md の Q16（`deep-questions-r3.json`、premise・exported）で人間に返した。帰結 2（前の head を後から取れるか）は、Q14 で本人が「確認バッチで確かめる」を選んでいるので問い直さず、探りを `package` に依らない 11.5 に分け、確認バッチの中・ST53 が main に入る前に置いた（R20 と同じ直し）
- 提案: 2 つの帰結を Q14 の答えの前提として本人に見せ、答えを保つか聞く。判断の材料として次を添える。
  (1) `package-linux`・`package-windows` の checkout に `ref: ${{ github.event.pull_request.head.sha || github.sha }}` を渡せば、組んだ木・組み立ての印・`SOURCE.txt` が head で一致し、比べを外す分岐も D17 も要らなくなる（代わりに PR の配布物は、その時の main と合わせた結果を映さない。ほかの job は merge の checkout のまま）。
  (2) 「push し直す前の head を、閉じた後に別の clone から fetch できるか」の探りは ST53 のコードを要しない（使い捨ての PR → force-push → 閉じる → fetch）ので、merge の前に走らせて P14 を確かさ 4 にできる（Q14 の第 3 の選択肢に近い）

## R18. D17（仮）の前提「引数だけでは PR と main への push を分けられない」は成り立たず、答えの「その場合だけ外す」を main への push に広げている
- 成果物: openspec/changes/st53-license-bundle/design.md:224-229（D17）/ specs/distribution/spec.md:176 / tasks.md（7.1）
- 根拠: 本人の答えは「PR の組み立てでは head の commit を書き、組み立ての印との比べは**その場合だけ**外す」（deep-answers-2.txt の Q14）。spec.md:176 は「debug の鍵を許す検査（PR・main への push の組み立てと下書き）: 組み立ての印とは比べない」で、main への push でも比べない。D17 は分けられない理由を `ci.yml:400` が同じ引数だからとするが、ci.yml は既に `${{ github.event.pull_request.base.sha || github.event.before }}`（`.github/workflows/ci.yml:34`）の形で PR と push を 1 つの式で分けている。D17 自身の反転条件（design.md:229）が、`--source-commit "${{ github.event.pull_request.head.sha || github.sha }}"` を渡して全部の組み立てで比べる形を書いており、費用は台本の引数 1 つと ci.yml の 1 行。`draft.sh` は 11.1 が既に使う `gh api …/actions/runs/<id> --jq .head_sha` で同じ値を取れる
- kind: technical
- 処置: escalated —— Q16 に乗る（どちらの選択肢でも D17 は外す。head から組むなら比べを外す分岐ごと要らず、合わせた木から組むなら `--source-commit` の案）。Q16 の答えを `spec --answers Q16` で写すときに D17 を `provisional.py resolve` で外す
- 提案: D17 を外し、`check-artifacts.sh` に `--source-commit <40 桁>` を足して、`--allow-debug-key` の検査でも `SOURCE.txt` の `Commit:` をその値と比べる（PR は head、main への push は `github.sha` = 組み立ての印）。spec.md:176 と tasks 7.1・6.1 の検証をそれに合わせる。R17 の (1) を採るなら、比べを外す分岐ごと要らない

## R19. 今回足した Requirement の文のうち、Scenario が言っていないもの
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:87・176
- 根拠:
  - spec.md:87「IF 依存の POM を取り寄せられない THEN その部品を飛ばさず、検査を落とす」（Q12 の推奨の detail「`--offline` で取れなければ組み立てを落とす」・C5）に Scenario が無い。近い Scenario「検査する対象が無い節は、通さずに落ちる」（spec.md:148。tasks 4.2 は `ASHIATO_GRADLEW=false`）は gradle 全体が落ちる場合で、task が解決できない部品を黙って飛ばす実装（例: lenient な解決）でも緑になる。tasks 4.1 の検証（tasks.md:96）は online → `--offline` の出力の `cmp` で、欠けた `.pom` で落ちることは試さない
  - spec.md:176「debug の鍵を許す検査: 40 桁の commit」には通る側の Scenario（spec.md:237）しか無い。`--allow-debug-key` で `Ref: v…` や 40 桁でない `Commit:` を持つ配布物が落ちる Scenario が無く、`SOURCE.txt` の中身を見ない実装でも緑になる（落ちる側の Scenario「ソースの場所がチャンネルの決まりと合わない配布物は、公開しない」は `--channel beta` だけ）
- kind: technical
- 処置: fixed 4.3 —— Scenario「POM を取り寄せられない部品があれば、許諾の検査が落ちる」（4.3 の使い捨ての置き場に `.pom` の無い部品を足して `--offline`）と「debug の鍵を許す検査でも、commit でないソースの場所の配布物は落ちる」（7.1）を足し、自己試験と検証の grep に名指した
- 提案: Scenario「取り寄せられない POM の部品があれば、許諾の検査が落ちる」（4.3 の `-Pashiato.extraRepo` の使い捨ての置き場に、`.pom` を置かない部品を足して `--offline` で走らせ、rc≠0 とその座標が出る）と、「debug の鍵を許す検査でも、40 桁の commit でないソースの場所は落ちる」を足し、tasks 4.2 / 4.3・7.1 の自己試験に名指す。R18 で比べを戻すなら、後者は「head の commit と違う `Commit:` は落ちる」にする

## R20. 11.1 の P14 の探りは、書いたままでは `package` が組まれない場合があり、merge の後に置く理由も無い
- 成果物: openspec/changes/st53-license-bundle/tasks.md:228-230（11.1）
- 根拠: 手順は「使い捨ての PR を開いて `package` を 1 回組ませ、その `SOURCE.txt` の commit を控える」。`package-linux` は `needs.changes.outputs.code == 'true'`（`.github/workflows/ci.yml:352`）のときだけ走り、`tools/release/changes.sh` の `code` は `docs/`・`openspec/`・`*.md` 以外のファイルが変わったときだけ真になる。使い捨ての PR が文書だけなら `package` は飛び、控える `SOURCE.txt` ができない。また、控えたいのは push し直す前の head の commit（`git rev-parse HEAD`）で、それは `SOURCE.txt` が無くても分かる
- kind: technical
- 処置: fixed 11.5 —— P14 の探りを 11.1 から外して 11.5 にし、`SOURCE.txt` も `package` も経ずに `git rev-parse HEAD` で head を控える形にした。確認バッチの中・ST53 が main に入る前に置き、取れなければ main に入れない。design D9・Open Questions・premises P14 の参照を 11.5 に直した
- 提案: 11.1 を 2 つに分ける。P14 の探り（PR を開く → head を控える → rebase して force-push → 閉じる → 別の clone で両方を `git fetch origin <commit>`）は `package` も ST53 のコードも要らないので、`SOURCE.txt` を経ずに head を控える形に書き、R17 の (2) のとおり merge の前に置けるかを決める。`package` を組ませる側を残すなら、使い捨ての PR が `docs/`・`openspec/`・`*.md` の外のファイルに触ることを手順に書く

## R21. 5.3 の自己試験が当てる「commit の選び方の関数」の置き場が書かれておらず、6.1 より前に検証できない
- 成果物: openspec/changes/st53-license-bundle/tasks.md:140-141（5.3）
- 根拠: 今回足した 5.3 の本文は「開発版・main への push と PR の 2 つの Scenario は、6.1 の `build.sh` の commit の選び方（`ASHIATO_SOURCE_COMMIT` があればそれ、無ければ `git rev-parse HEAD`）を同じ関数で当てる（関数を `build.sh` と自己試験で共有する）」で、関数をどのファイルに置くかを言わない。関数が 6.1 の `build.sh` に入るなら、5.3 の検証（`tools/release/notices.sh --self-test` の 3 本の grep）は 6.1 を済ませるまで通らず、tasks の並び（5.3 → 6.1）と依存の向きが逆になる
- kind: technical
- 処置: fixed 5.3 —— commit の選び方を `tools/release/notices.sh commit` の副コマンドに置き、6.1 の `build.sh` はそれを呼ぶと書いた。5.3 の自己試験はこの副コマンドを当てるので 6.1 を待たない
- 提案: 関数を `tools/release/notices.sh`（か、両方が読む小さな台本）に置き、6.1 の `build.sh` はそれを呼ぶ、と 5.3 に置き場を書く。または 2 本の Scenario の検証を 6.1 へ移す

## 第 2 回の観点ごとの該当なし

- 観点 1: 該当なし（確かめた範囲: Q10→D4・spec.md:53・tasks 2.1、Q11→D5・spec.md:55・tasks 2.2、Q12→D7・spec.md:86-87・Scenario 2 本・tasks 4.1/4.2、Q13→D8・tasks 5.2、Q14→D9・spec.md:169-176・Scenario 3 本・tasks 5.3/6.1/7.1/11.1、Q15→D16・spec.md:169。C1〜C5・C9 の仕様の側も同じ場所にある。go.json の W2・IMP-06 は reading の段が直すので見ていない。「答え待ち」「D<n>（仮）」の印は、第 2 回の 6 件には残っていない。「最初の印から後は前進側の 2 つを直せない」の Scenario は、正典 `openspec/specs/distribution/spec.md:203` の「配った版にある移行の中身を変えると、検査が落ちる」が持つ）
- 観点 3: 該当なし（確かめた範囲: 今回の差分の design D4・D5・D7・D8・D9・D16・D17 のうち、受け付ける / 断る判定は spec に上がっている（AND・取り寄せられなければ落とす・head の commit・40 桁でなければ落とす・debug の鍵の検査の決まり）。proposal の Capabilities は変わっていない）
- 観点 5: 該当なし（確かめた範囲: check_chain.py OK。docs/stories/ST53.md の完了の判定は record の段が C9 で「開発版・main への push・PR」に直していて、spec.md:169 と合う）
- 観点 7: 該当なし（spec_overlap.py candidates が 0 件）

## 第 2 回の処置の確かめ（R19〜R21。2026-11-03）

R20（11.5 に分け、`git rev-parse HEAD` で head を控え、ST53 が main に入る前に置く）と R21（`notices.sh commit` の副コマンドに置き、6.1 はそれを呼ぶ）は、指摘を満たす。R19 は 2 つとも Scenario が足されたが、次の R22・R23 が残る。

## R22. Scenario「POM を取り寄せられない部品があれば、許諾の検査が落ちる」の GIVEN では、部品を飛ばす実装でも別の理由で落ちて緑になる
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:148-151 / tasks.md:116
- 根拠: GIVEN は「使い捨ての maven の置き場に jar だけを置き `.pom` を置かない部品」。gradle の maven の置き場は、既定では POM か Gradle Module Metadata（`.module`）から部品を見つける（`artifact()` の metadata source は既定に入らない）ので、`.pom` も `.module` も無い部品は `releaseRuntimeClasspath` の解決の段で「見つからない」で落ちる。4.1 の task が `@pom` の detached configuration の失敗を黙って飛ばす実装（R19 が挙げた lenient な解決）でも、その前に依存の解決が落ちるので、この Scenario は rc≠0 で緑になる（tasks.md:116 の「lenient な解決で部品を黙って飛ばす実装はここで落ちる」は成り立たない）。実際に起きる形は、依存は解決できるのに `.pom` が手元に無い場合（`~/.gradle/caches/modules-2/files-2.1/androidx.core/core/*/*/` は `core-1.17.0.aar` だけ、`com.google.android.gms/play-services-location` も `.aar` だけ。2026-11-03 に ls で確かめた）
- kind: technical
- 処置: fixed 4.3 —— GIVEN を「`.module` と jar はあるが `.pom` が無い部品（依存の解決は通る）」にし、THEN に「POM を取れないことが示される（依存の解決の失敗ではない）」を足した。4.3 の手順も同じ形にし、`Could not find` でないことを見る
- 提案: GIVEN を「依存の解決はできるが `.pom` を取れない部品」に直す。例: 使い捨ての置き場に `.module`（Gradle Module Metadata）と jar を置き `.pom` を置かない部品を足す。依存は `.module` で解決でき、`@pom` の解決だけが落ちる。THEN に「出力にその部品の行が無いまま rc=0 にならない」ことと、示す文が依存の解決の失敗（`Could not find`）ではなく POM を取れないことであるのを足す

## R23. debug の鍵を許す検査の「40 桁の commit」のうち、40 桁でない `Commit:` を落とすことは Scenario が言っていない
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:247-250 / tasks.md（7.1）
- 根拠: 足した Scenario は `Ref: v0.4.0` が落ちることだけを見る。spec の決まり（spec.md:181「debug の鍵を許す検査: 40 桁の commit」）と D9（「`Commit:` が 40 桁の 16 進であることだけを見て」）の、形の検査そのもの（`Commit: main`・短い commit）は、`Commit:` で始まるかだけを見る実装でも緑になる。R19 が挙げた 2 つの例のうち、後の方が残っている
- kind: technical
- 処置: fixed 7.1 —— Scenario の GIVEN を `Ref: v0.4.0`・`Commit: main`・7 桁の `Commit:` の 3 つにし、3 つとも落ちることを求めた。7.1 の自己試験も同じ 3 つにした
- 提案: 同じ Scenario の GIVEN を `Commit: main`（または 7 桁の短い commit）に替えるか、もう 1 本足して 7.1 の自己試験に名指す（7.1 の検証の数も合わせる）。R18 / Q16 の答えで比べを戻すなら、この Scenario は「渡した commit と違う `Commit:` は落ちる」に置き換わる

## R24. 名前の統一の行が、5.3 で足した副コマンド `commit` を持たない
- 成果物: openspec/changes/st53-license-bundle/tasks.md:24
- 根拠: tasks.md:24 の「名前は統一する」は `tools/release/notices.sh <server|collector-windows|apk|source>` で、R21 の処置で 5.3 に足した `notices.sh commit`（6.1 の `build.sh` が呼ぶ）が無い。下流がこの行を道具の名前の正本として読むと、副コマンドの名前が揺れる
- kind: technical
- 処置: fixed tasks.md —— 「名前は統一する」の `notices.sh` の副コマンドに `commit` を足した
- 提案: tasks.md:24 を `<server|collector-windows|apk|source|commit>` にする

## 機械の検査（第 3 回。spec --answers Q16 の後）

| コマンド | 結果 |
|---|---|
| `openspec validate st53-license-bundle --strict` | `Change 'st53-license-bundle' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0） |
| `python3 scripts/check_scenarios.py . st53-license-bundle` | `scenarios: FAIL (担保なし 51 件 / 名無しの確認待ち 0 件)`（rc=1）。51 件はどれもこの change の ADDED の Scenario（`distribution` 48・`verify-environment` 3。実装の前なので期待どおり）。51 本の名前を全部 tasks.md に grep して欠け 0。tasks.md:13 の「51 本（48・3）」と合う |
| `python3 scripts/review_triage.py . st53-license-bundle` | `triage: OK`（指摘 35 件 / 仮決め 6 件 / 要件へ戻すもの 2 件） |
| `python3 scripts/spec_overlap.py candidates . st53-license-bundle` | `[]`（0 件） |
| `python3 scripts/provisional.py list openspec/changes/st53-license-bundle` | 開いている仮は D6・D10・D11・D18 の 4 つ。D17 は `resolved` で閉じている（`check` の FAIL は D18 の `provisional-review.md` が無いことだけ。グラフが後で書く） |

答え以外の変更の分け方（今回の差分。`git diff HEAD -- openspec/changes/st53-license-bundle`）:
- D17 を外したこと（design.md:228-230・provisional.jsonl:17・tasks.md:21）: Q16 の問いの context（deep-questions-r3.json の note・context「どちらの選択肢でも D17 は外します」）が答えの中に含めていた。答えの写しで、答え以外の変更に数えない
- D9 の書き直し（checkout の `ref`・`ASHIATO_SOURCE_COMMIT` を持たず組み立ての印と同じ出所の commit を書く・比べはすべての組み立てで残す・K2 の記述）、spec.md:174-175 の 2 文、Scenario の入れ替え（5 本）、tasks 5.3・6.1 の `package-linux`・`package-windows`・7.1・11.1: 選んだ選択肢の detail（「組んだソース・配布物の組み立ての印・`SOURCE.txt` が同じ head を指し、比べはすべての組み立てで残せる」「縦串や試験などほかの job は今どおり合わせた木で走る」）の写しで、答え以外の変更に数えない
- D18（仮）: `package-artifacts` も同じ `ref` で checkout し、版番号と `LICENSE` を head の木から取る・`draft.sh` は PR の head の手元で走らせる。選んだ選択肢は「`package-linux`・`package-windows` の checkout」「費用は workflow の 2 行」で、検査の job を名指していない。答え以外の変更。中身は design.md:232-238 の D18（仮）の節にあり、spec の Scenario「PR の配布物は、PR の head の commit から組まれる」の 3 つ目の job・tasks 6.1 の `package-artifacts`・10.2 の下書きの手順・proposal.md:71 に入った。仮なので合意は引き継がれる
- premises P17・P18: 前提の行で digest の外。D9 の本文（design.md:157）からの参照は確かめの置き場で、決定を変えない

## R25. Scenario「開発版と main への push の配布物のソースの場所は、組み立てた commit」の GIVEN が、Q16 で無くした「PR の head の commit を渡す」口を前提に残している
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:228
- 根拠: GIVEN「印も **PR の head の commit も渡されない**組み立て（開発版か main への push の組み立て）」。Q16 の後の D9（design.md:142「commit を別に渡す環境変数は持たない」）と tasks 6.1・名前の統一（tasks.md:26「commit を渡す環境変数は足さない」）では、PR の組み立ても head の commit を「渡されない」—— checkout した木の `git rev-parse HEAD` から取る。この GIVEN では、PR の組み立てもこの Scenario の前提に当たり、開発版・main への push と PR を分けるものが無い。5.3 の自己試験（tasks.md:143）は「main の commit を checkout した木」で当てると書いていて、Scenario の GIVEN と食い違う。`ASHIATO_SOURCE_COMMIT` の grep には掛からない言い方で残った（観点 1: 覆した後の内容だけが残っているか）
- kind: technical
- 提案: GIVEN を「main の commit を checkout した木で、印を渡さない組み立て（開発版か main への push の組み立て）」にし、隣の PR の Scenario（spec.md:233「main と合わせていない枝の commit（PR の head）を checkout した木で、印を渡さない組み立て」）と checkout した木で分ける。THEN は今のまま
- 処置: fixed specs/distribution/spec.md —— GIVEN を「main の commit を checkout した木で、印を渡さない組み立て（開発版か main への push の組み立て）」にし、隣の PR の Scenario と checkout した木で分けた（5.3 の自己試験と同じ形）

## R26. 6.1 の検証のうち 2 本は、正しい実装で落ちる
- 成果物: openspec/changes/st53-license-bundle/tasks.md:153（6.1 の検証）
- 根拠: (1) `! grep -rq "ASHIATO_SOURCE_COMMIT\|pull_request.head.sha" .github/workflows/release.yml .github/workflows/nightly.yml tools/release` は `tools/release` を再帰で見る。ところが同じ 6.1（tasks.md:152）は `tools/release/check-guards.sh` の `workflows` に「`ci.yml` の 3 つの job の checkout がその `ref` を持つこと」の判定と、`ref` を抜いた偽の `ci.yml` で落ちる自己試験を足させる。今の `workflows` は ci.yml を字面で `grep -q '<文字列>'` する型で（`tools/release/check-guards.sh:157`・`:160`・`:176`）、判定も偽の ci.yml を作る `sed` も `pull_request.head.sha` の文字列を持つのが自然。そうすると 6.1 の検証は `tools/release/check-guards.sh` に当たって rc=1 になる。
  (2) `grep -c "ref: \${{ github.event.pull_request.head.sha || github.sha }}" .github/workflows/ci.yml | grep -qx 3` は block 形式の `ref: ${{ … }}` だけに当たる。3 つの job の checkout は今 flow 形式（`ci.yml:356`・`:378`・`:394` の `with: { persist-credentials: false }`）で、そこへ足すと YAML の flow の中では `{` を含む値を引用符で囲む必要があり（`ci.yml:424` の `env: { RANGE: "${{ … }}" }` と同じ形）、`ref: "${{ … }}"` は 0 件で落ちる。tasks.md:151 は「`persist-credentials: false` は残す」と言うだけで形を決めていない
- kind: technical
- 提案: (1) の否定の grep は `ASHIATO_SOURCE_COMMIT` を `tools/release` と workflow の 3 つで、`pull_request.head.sha` を `release.yml`・`nightly.yml` だけで見る形に分ける。(2) は block 形式で書くと 6.1 に決めるか、grep を `ref: "\?\${{ github.event.pull_request.head.sha || github.sha }}"\?` のように引用符の有無を許す形にする（判定の正本は `check-guards.sh workflows` なので、そちらに寄せて YAML の grep を外してもよい）
- 処置: fixed 6.1 —— 否定の grep を `ASHIATO_SOURCE_COMMIT`（workflow と `tools/release`）と `pull_request.head.sha`（`release.yml`・`nightly.yml` だけ）に分けた。YAML の字面の本数の grep は外し、判定の正本を `check-guards.sh workflows` に寄せた。3 つの checkout の `with:` は block 形式に書き直すと 6.1 に決めた

## R27. Scenario「PR の配布物は、PR の head の commit から組まれる」が、どの Requirement の文も言っていない「検査する job も head を checkout する」を持つ
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:242-244 / :175
- 根拠: THEN「**3 つの job** のどれもが、PR では PR の head の commit を…checkout し」。3 つ目は「その配布物を検査する job」（`package-artifacts`。D18（仮））。Requirement の本文（spec.md:175）は「PR の組み立てを、PR の head の commit そのものから組む」で、組む側しか言わず、検査が何と比べるか（期待する版番号・`LICENSE` を head の木から取る。design.md:234）は design にだけある。archive すると、本文に錨の無い振る舞いが Scenario だけで正典に入る。名前も「組まれる」で、検査の job を含むことが読めない。あわせて THEN は「3 つが head を checkout する」と「どれかが違えば検査が落ちる」の 2 つを束ねる（片方だけを満たす実装を区別しない。R16 と同じ型）
- kind: technical
- 提案: Requirement の本文に「配布物の検査は、配布物を組んだのと同じ commit の木の版番号と `LICENSE` と比べる」の 1 文を足す（D18（仮）は導出元に既にある）か、Scenario を組む 2 つの job だけにして検査の job を D18 の反転条件と一緒に design に残す。THEN の否定の側は、偽の workflow で落ちる別の Scenario に分けるか、tasks 6.1 の自己試験の説明に下ろす
- 処置: fixed specs/distribution/spec.md —— Requirement の本文に「配布物の検査を、配布物を組んだのと同じ commit の木で走らせ、期待する版番号と比べる `LICENSE` をその木から取る」を足し（D18（仮））、Scenario の名前を「PR の配布物は、PR の head の commit から組まれ、同じ commit で検査される」にした。THEN は checkout の 1 つに絞り、偽の workflow で落ちる側は tasks 6.1 の自己試験の説明に下ろした

## R28. 11.1 は `draft.sh` を走らせる木を言っておらず、P17 の確かさを 4 に上げられない部分まで上げる
- 成果物: openspec/changes/st53-license-bundle/tasks.md:231-233（11.1） / premises.md:24（P17）
- 根拠: (1) D18（仮）（design.md:236）は「`draft.sh` は走らせた手元の木の版番号と `LICENSE` で比べる（`draft.sh:15`・`:32`）。`docs/release.md` の下書きの手順に、その PR の head を checkout した手元で走らせることを書く」とし、10.2（tasks.md:221）も同じことを手順書に書かせる。ところがこの Story で `draft.sh` を実際に走らせる唯一の場所の 11.1 は「この Story の PR の `package` の run で `tools/release/draft.sh <run-id> 0.1.0-beta.1` を走らせる」だけで、走らせる手元を言わない。`draft.sh` は `$root/tools/release/version.sh source`（`draft.sh:15`）と `$root` の `check-artifacts.sh`（`:32`。`LICENSE` も `$root` の木と比べる。`check-artifacts.sh:4`）を使うので、確認バッチの作業ツリーが PR の head でなければ K1 と同じ理由で比べが食い違う。
  (2) 11.1 は「その run の `package-linux`・`package-windows`・`package-artifacts` が head を checkout して通ったことで P17 を見る（P17・P18 の確かさを 4 にする）」。P17 は「fork の PR でも」取れることを含む（premises.md:24）が、この Story の PR は同じ repo の枝で、fork の PR の head は 11.1 では確かめられない。P17 自身の「確かめ方」も「fork の PR は配布元に無いので、外れたら反転する」と書いている
- kind: technical
- 提案: 11.1 に「ST53 の PR の head を checkout した手元（`git -C <作業ツリー> rev-parse HEAD` が run の `head_sha` と同じ）で走らせる」を足す。P17 は 11.1 で上げる範囲を「同じ repo の枝の PR」に限り、fork の側は確かさ 2 のまま残す（外れても落ちる側に倒れることは P17 に書いてある）
- 処置: fixed 11.1 —— （premises.md の P17 も同じ範囲に直した） 11.1 に `draft.sh` を PR の head を checkout した手元（`git rev-parse HEAD` が run の `head_sha` と同じ）で走らせることを足した。P17 を 11.1 で上げる範囲は同じ repo の枝の PR に限り、fork の側は確かさ 2 のまま残すと 11.1 と P17 に書いた

## R29. 7.1 の「次の 4 つを足す」が、並べた 7 項と検証の 8 本と合わない
- 成果物: openspec/changes/st53-license-bundle/tasks.md:164-173
- 根拠: tasks.md:164「`--self-test` に、手で組んだ小さな配布物…で次の 4 つを足す」の後に 7 項が並び、tasks.md:172 でもう 1 本（`4 つの配布物は、許諾・依存の一覧・ソースの場所を持つ`）を足し、検証（tasks.md:173）は `grep -qx 8`。第 1 回（c4a93a49）の「4 つ」のまま、R19・R23 と今回の処置で項が増えた。下流が数の方を読むと、足す本数を取り違える
- kind: technical
- 提案: 「次の 7 つを足す」に直す（tasks.md:172 の 1 本と合わせて検証の 8 本になることを 1 行添える）
- 処置: fixed 7.1 —— 「次の 7 つを足す（下の重い自己試験の 1 本と合わせて、検証の 8 本になる）」に直した

## 第 3 回の観点ごとの該当なし

- 観点 1（R25 のほか）: 該当なし（確かめた範囲: Q16 → design D9（design.md:137-160）・spec.md:174-175・Scenario「PR の組み立ての配布物のソースの場所は、PR の head の commit」「組み立ての commit が 40 桁の 16 進でなければ…」「PR の配布物は、PR の head の commit から組まれる」「PR の組み立てと下書きの配布物の検査は…」「debug の鍵を許す検査でも、組み立ての印の commit でない…」・tasks 5.3・6.1・7.1・11.1。`deep.md` の「spec の段へ渡すもの（第 3 回）」の各項 —— D9・D17 の resolve（台帳で閉じている）・spec と Scenario・checkout と比べ・premises P14（premises.md:23）、C1 の 11.1 の反転（tasks.md:233）・Risks の項の置き換え（design.md:249）・proposal.md:71、K1 の `package-artifacts`（D18（仮））、K2 の job の中の検査（design.md:154）、未確認 2 つ（P17・P18）—— は全部同じ場所にある。go.json は reading の段が直すので見ていない。change の文書に「合成の merge commit」「組み立ての印とは比べない」「`ASHIATO_SOURCE_COMMIT`」は、経緯・選ばなかった案・否定の検証の中にしか残っていない。Q10〜Q15 の写し先（D4・D5・D7・D8・D16、spec.md:53・55・86-87）は今回の差分で触られていない）
- 観点 3: 該当なし（確かめた範囲: 今回足した D18（仮）の観測できる振る舞いは spec の Scenario に上がっている（R27 の本文の錨は別）。spec の新しい Scenario の `ASHIATO_BUILD_COMMIT` は、正典 `openspec/specs/distribution/spec.md:40` が同じ名前を Scenario で使っている入力の口で、実装の名前として新たに持ち込んだものではない。proposal の Capabilities と specs のディレクトリは変わっていない）
- 観点 5: 該当なし（確かめた範囲: check_chain.py OK（再生成との一致を含む）。docs/stories/ST53.md の完了の判定「それ以外の組み立て —— 開発版・main への push・PR —— は 40 桁の commit」は Q16 と矛盾しない（組む木を言わないのは整合の確認 C2 が leave にした））
- 観点 7: 該当なし（spec_overlap.py candidates が 0 件）

## 第 3 回の処置の確かめ（R25〜R29。2026-11-03）

- R25: 閉じた。spec の GIVEN が「main の commit を checkout した木で、印を渡さない組み立て」になり、隣の PR の Scenario（「main と合わせていない枝の commit（PR の head）を checkout した木」）と checkout した木で分かれる。5.3 の自己試験（tasks.md:143）とも同じ形
- R26: 閉じた。6.1 の否定の grep は `ASHIATO_SOURCE_COMMIT` を `.github/workflows` と `tools/release` で、`pull_request.head.sha` を `release.yml`・`nightly.yml` だけで見る形に分かれ、`check-guards.sh` の判定の文字列に当たらない。YAML の字面の本数の grep は外れ、判定の正本は `check-guards.sh workflows`（rc=0）とその自己試験の Scenario 1 本（`grep -qx 1`）。`with:` を block 形式にすることも 6.1 に書いてある
- R27: 閉じた。Requirement の本文に「配布物の検査を、配布物を組んだのと同じ commit の木で走らせ、期待する版番号と比べる `LICENSE` をその木から取る」が入り、Scenario の名前が検査の job を含むことを言い、THEN は checkout の 1 つの主張になった。偽の workflow で落ちる側は 6.1 の自己試験の説明にある。新しい名前は 6.1 の本文と検証の grep で一字一句同じ。ただし足した 1 文は CI の外の検査にも掛かる（R30）
- R28: 閉じた。11.1 は `draft.sh` を PR の head を checkout した手元（`git rev-parse HEAD` が run の `head_sha` と同じ）で走らせると言い、P17 を上げる範囲を同じ repo の枝の PR に限った。premises.md:24 の P17 の確かめ方も同じ範囲になっている
- R29: 閉じた。7.1 は「次の 7 つを足す（下の重い自己試験の 1 本と合わせて、検証の 8 本になる）」で、並ぶ 7 項・tasks.md:172 の 1 本・検証の `grep -qx 8` と合う

## R30. R27 の処置で足した「配布物の検査を、配布物を組んだのと同じ commit の木で走らせる」は、最初の印の後に本人が手で走らせる検査（D12）にも掛かるが、D12 と 10.2 はその木を言っていない
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:176 / design.md（D12） / tasks.md:219（10.2）
- 根拠: 足した文は「THE SYSTEM SHALL 配布物の検査を、配布物を組んだのと同じ commit の木で走らせ、期待する版番号と比べる `LICENSE` をその木から取る」で、どの検査かを限っていない。CI の 3 つ（ci.yml の `package-artifacts`・`release.yml:85`・`nightly.yml:71`）は同じ event の commit を checkout するので当たる。下書きの `draft.sh` は 11.1・10.2 が PR の head の手元と書いた。残る 1 つが D12 の「`docs/release.md` §6『最初の印』の最後に `gh release download v<版> -D "$dir"` と `tools/release/check-artifacts.sh "$dir" --channel <beta|release> --expect-version <版>`」で、`check-artifacts.sh` は自分の置かれた木の `LICENSE` と比べる（`tools/release/check-artifacts.sh:4` の `root`）。D12 も 10.2（tasks.md:219「最初の印の最後: `gh release download` と `check-artifacts.sh --channel` の 2 行（D12）」）も、走らせる手元が印 `v<版>` の commit であることを言わない。印の後に main が `LICENSE` を変えた手元で走らせると、正しい配布物を落とす（落ちる側なので配布物は漏れない）。Requirement の文と、それを満たす手順の 1 つが合っていない
- kind: technical
- 提案: 10.2 の「最初の印の最後」の行と D12 に「印 `v<版>` を checkout した手元で走らせる」を足し、10.2 の検証の見出しの grep に 1 語足す。または Requirement の文を「CI と下書きの配布物の検査」に限り、D12 の手で走らせる検査はその外だと書く
- 処置: fixed D12 —— D12 と tasks 10.2 に「印 `v<版>` を checkout した手元で走らせる」を足した（Requirement の文は限らず、全部の検査が組んだ commit の木で走る形に揃えた）

## 第 4 回の写しの確かめ（spec --answers Q17。2026-11-03）

機械の検査（今の木。`git diff HEAD` の未 commit の写しを含む）:

| コマンド | 結果 |
|---|---|
| `openspec validate st53-license-bundle --strict` | `Change 'st53-license-bundle' is valid` |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)` |
| `python3 scripts/check_scenarios.py . st53-license-bundle` | FAIL（担保なし 52 件 / 名無しの確認待ち 0 件）。実装の前の上流なので、Scenario の担保が無いのは今回の差分に依らない |
| `python3 scripts/review_triage.py . st53-license-bundle` | `triage: OK`（指摘 41 件 / 仮決め 7 件 / 要件へ戻すもの 2 件） |
| `python3 scripts/spec_overlap.py candidates . st53-license-bundle` | `[]` |

確かめたこと:
- Q17 の答え（`deep-answers-7.txt`「合わせた木のまま走らせ、期待値だけを head の commit から取る（要件は変えない）」）は、design D18（design.md:232-244）・spec の Requirement の文（spec.md:176）と IF の文（spec.md:179）・Scenario（spec.md:243-251）・tasks 6.1（tasks.md:153-155）・7.1（tasks.md:166・:174・:176）・10.2・11.1・proposal.md:71・premises.md:24 に写っている。「head の木で checkout する」側（旧 D18（仮））は、D18 の経緯の引用（design.md:234）と deep.md の経緯の中にしか残っていない
- 要件 FR-127 ★ 2026-11-03（第 3 回）（docs/requirements.md:1096「ほかの job の縦串や試験は今どおり合わせた木で走る。組み立ての job の中の自己試験と許諾・表示の検査は head の木で走る」）と食い違わない。spec.md:176 は検査の job が合わせた木で**走る**と言い、head の commit からは期待値を**データとして読む**だけ（design.md:238「台本はその木から走らせない」）。要件は何がどの木で走るかを限っていて、期待値の読み先を限っていないので、要件の改訂は要らない（答えも「要件は変えない」）
- 実現の細部は答えの範囲の内: 選んだ選択肢の detail（deep-questions-r4.json）は「`package-artifacts` の checkout は今のまま。期待する版番号と比べる LICENSE は、配布物を組んだ head の commit の中身から取る。費用は `check-artifacts.sh` の期待値の取り方の変更」。2 つ目の checkout を `built/` に置くのは「head の commit の中身から取る」手段で、作業の checkout は変えない。`version.sh source built/Cargo.toml` と `--expect-license` は「期待値の取り方の変更」。旧 D18 の反転条件を外したのは、答えがその反転条件の形を選んだため。P17 の書き換えは前提の行で、確かめの置き場が `built/` の checkout に移っただけ
- 道具の実在: `tools/release/version.sh:14`・`:276` は `source [<Cargo.toml>]` で道を受ける（D18 の主張どおり）。`check-artifacts.sh` の自己試験の Scenario の出力は `== Scenario: <名前>` の型（check-artifacts.sh:21 ほか）。7.1 の grep の選択肢は 9 つで、並ぶ 8 項と重い自己試験の 1 本に当たり、既存の 10 本の名前（`配布物が 1 つでも欠ければ公開しない` ほか）とも、新しい `配布物の LICENSE は、渡した` と `配布物の LICENSE が` の間でも重ならない。6.1 の `check-guards.sh --self-test` の grep の名前は spec.md:243 の新しい名前と一字一句同じ（今の check-guards.sh には 0 件。実装の前なので当然）
- go.json は旧い Scenario 名「PR の配布物は、PR の head の commit から組まれ、同じ commit で検査される」を go.json:486・:731・:1638・:1716 に残している。この段では触らず（`reading --carry` が直す）、指摘に含めない。review/reflected.json:14 の旧い名前は第 3 回の写しの記録で、直す対象ではない

## R31. Scenario「PR の配布物は、PR の head の commit から組まれ、同じ commit の期待値で検査される」の AND が主張を 3 つ束ね、6.1 の偽の ci.yml はそのうち版番号の側を落とさない
- 成果物: openspec/changes/st53-license-bundle/specs/distribution/spec.md:243-246 / tasks.md:154（6.1）
- 根拠: AND（spec.md:246）は「検査する job の作業の checkout は `ref` を持たない」「同じ commit を別の置き場に checkout する」「期待する版番号と比べる `LICENSE` をその置き場から検査に渡す」を 1 つの文に束ねる。R27 の処置で THEN を checkout の 1 つの主張に絞った（第 3 回の処置の確かめ R27）ものが、Q17 の写しで再び広がった。6.1 の自己試験が落とす偽の `ci.yml` は 4 つ（組む 2 つの `ref` を抜く / 作業の checkout に `ref` を足す / `path: built` の checkout を抜く / `--expect-license built/LICENSE` を抜く）で、`version.sh source built/Cargo.toml` を `version.sh source` に戻した偽は無い。`workflows` の判定がこの字面を見なくても、自己試験も `check-guards.sh workflows` rc=0 も緑のまま通る。版番号は K1 の 2 つの食い違い（版番号か `LICENSE`）の片方で、Q17 が避けようとしたものそのもの（design.md:242）
- kind: technical
- 提案: 6.1 の偽の `ci.yml` に 5 つ目「`package-artifacts` の `version.sh source` の引数から `built/Cargo.toml` を抜いた」を足す。あわせて AND を「作業の checkout は `ref` を持たない」と「同じ commit を別の置き場に checkout し、期待する版番号と `LICENSE` をそこから渡す」に分けるか、R27 と同じく AND を落として 6.1 の自己試験の説明に下ろす
- 処置: fixed 6.1 —— spec の Scenario も直した。6.1 に 5 つ目の偽の `ci.yml`（`version.sh source built/Cargo.toml` を `version.sh source` に戻した）を足し、Scenario の AND を「作業の checkout は `ref` を持たない」と「期待値を同じ commit を別の置き場に checkout したものから渡す」の 2 つに分けた

## 第 4 回の観点ごとの該当なし

- 観点 1: 該当なし（確かめた範囲: deep.md の Q17（deep.md:121-127）と「答えと仮の決定の関係」の第 4 回（deep.md:262）→ design D18・spec.md:176・:179・:243-251・tasks 6.1・7.1・10.2・11.1・proposal.md:71。deep.md:119 が挙げる `docs/release.md` の手順は 10.2 の下書きの手順の行（tasks.md:224）で下流が書く。数値を含む決定は無い。要件へ戻すものは無い（答えが「要件は変えない」））
- 観点 2（R31 のほか）: 該当なし（確かめた範囲: 新しい Scenario「配布物の LICENSE は、渡した期待する LICENSE と比べる」の THEN は渡したとき rc=0・渡さないとき rc≠0 の対で、片方だけ満たす実装（渡したものを読まない・比べをしない）をどちらも落とす。spec.md:166 の「`LICENSE`: 木の `LICENSE` と同じ中身」と Scenario「配布物の LICENSE が木の LICENSE と違えば、公開しない」の「木」は組み立ての木（head）か、渡さないときの台本の木を指し、IF の文（spec.md:179）の「期待する `LICENSE`（配布物を組んだ commit の `LICENSE`）」と食い違わない）
- 観点 3: 該当なし（確かめた範囲: D18 の観測できる振る舞い（検査の job の checkout・期待値の取り先・`--expect-license` の有無で比べる相手が変わること）は spec に上がっている。spec の `ref` は workflow の検査が読む字面で、第 3 回から同じ。proposal の Capabilities と specs のディレクトリは変わっていない）
- 観点 4（R31 のほか）: 該当なし（確かめた範囲: 6.1・7.1 の検証はコマンドと rc を持ち、Scenario の名前の grep は `grep -qx` で本数を見る。7.1 の「次の 8 つ」「検証の 9 本」は並ぶ 8 項・重い 1 本・grep の 9 選択肢と合う。7.1 の `--expect-license` を 6.1 の検査の呼び出しが使うが、6.1 は「`--expect-license` は 7.1」と書き、6.1 の検証は `check-guards.sh` の字面の判定だけで 7.1 の実装に依らない）
- 観点 5: 該当なし（確かめた範囲: check_chain.py OK（再生成との一致を含む）。docs/stories/ST53.md の完了の判定は検査の job の木を言わず、Q17 と矛盾しない）
- 観点 7: 該当なし（spec_overlap.py candidates が 0 件）

## 第 4 回の処置の確かめ（R31。2026-11-03）

- R31: 閉じた。Scenario（spec.md:243-247）の AND は「検査する job の作業の checkout は `ref` を持たない（合わせた木）」と「期待する版番号と比べる `LICENSE` を、組む 2 つの job と同じ commit を別の置き場に checkout したものから検査に渡す」の 2 つに分かれた。2 つ目は版番号と `LICENSE` を 1 文に持つが、6.1（tasks.md:154）の偽の `ci.yml` が 5 つになり、`--expect-license built/LICENSE` を抜いた偽と `version.sh source built/Cargo.toml` を `version.sh source` に戻した偽がそれぞれを落とすので、片方だけの実装は緑にならない。Scenario の名前は変わらず、6.1 の本文と検証の grep の名前と一字一句同じ
- Q17 の写しは崩れていない: 直しは D18 の中身（作業の checkout は合わせた木・期待値は組んだ commit を `built/` に checkout したものから）を言い直しただけで、新しい決定を持ち込んでいない。`openspec validate --strict` は valid、`review_triage.py` は OK（指摘 42 件）
