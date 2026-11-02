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

