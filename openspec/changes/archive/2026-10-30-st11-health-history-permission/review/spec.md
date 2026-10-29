# ST11 の spec（第 1 回の先読み・`spec --draft`）の独立検査

対象: `openspec/changes/st11-health-history-permission/` の proposal.md・specs/device-collection/spec.md・design.md・tasks.md（deep.md・deep-questions.json・premises.md・review/deep.md と照らした）。
深掘りの答えはすべて `（答え待ち・推奨: …）`。本人の答えが無いことは指摘しない。requirements.md の FR-11・扉 21 の ★ 訂正が未だなのも、record の段が入れる予定（proposal・design D13）として指摘しない。

## 機械の検査（最初に写す）

```
$ openspec validate st11-health-history-permission --strict
Change 'st11-health-history-permission' is valid

$ python3 scripts/check_chain.py .
  [ok]   どれかの Story に拾われた要件: 140/142 件
  [ok]   INDEX の「Story の対象外」: NFR-11, NFR-8
chain: OK (0 件 / 未回収 0 件 / warn 0 件)

$ python3 scripts/check_scenarios.py . st11-health-history-permission
  [FAIL] 担保の無い Scenario: …（33 件。ADDED の 31 本 + MODIFIED に足した 2 本）
scenarios: FAIL (担保なし 33 件 / 名無しの確認待ち 0 件)

$ python3 scripts/review_triage.py . st11-health-history-permission
  指摘 14 件 / 仮決め 0 件 / 要件へ戻すもの 0 件
  [FAIL] deep.md R1 / R4 / R6 / R7 / R8 / R9: escalated なのに deep.md に R<n> が無い
triage: FAIL (6 件)
```

- scenarios の 33 件は、実装の前なので印が無いのが当然。数は tasks.md:14 の「この change が足す Scenario は 33 本」と合う（spec の `#### Scenario:` は 40 本 = 33 + MODIFIED で写した既存の 7 本）。33 本は全部 tasks のどこかの Task が名指ししている（`grep -F` で 1 本ずつ確かめた）
- triage の 6 件は、commit `de0235a7` の本文「escalated の 6 件は record で deep.md に載る」のとおり record の段が入れる予定。**record が実際に R1・R4・R6・R7・R8・R9 を deep.md に書かなければ triage は赤のまま**（ここでは指摘にしない）
- chain の観点 8 は通っているが、`docs/stories/ST11.md:14` と `stories.json:268` の両方が「永久に読めない」のまま揃っているから通っているだけ（観点 5 / R12）

---

## R1. Q8 の推奨の文は「履歴に未対応の端末では求めない」と読めるが、spec と design D6（仮）は「種類は求める」と書いている —— 推奨のまま答えると食い違いが残る
- 分類: 深掘りの推奨と spec の食い違い（観点 1）
- 成果物: openspec/changes/st11-health-history-permission/specs/device-collection/spec.md / design.md（D6）/ deep-questions.json（Q8）/ deep.md（Q8）
- 根拠: deep-questions.json Q8 の問いは「Health Connect が端末に無いとき…や、**履歴の権限に対応していないときは**、どうしますか」で、推奨（`recommended: true`）は「求めずに、使えないことを記録して収集を始める」、その detail は「Q5 の記録に『Health Connect が無い / 更新が要る / **履歴に未対応**』を残す」。deep.md:40-43 も同じ。
  ところが spec.md:90 は「WHEN Health Connect は使えるが、履歴の読み取りか背景での読み取りにその端末が対応していない THE SYSTEM SHALL 対応しているものだけを種類と一緒に求める」、Scenario「履歴の読み取りに対応していない端末では対応しているものだけを求める」（spec.md:115-118）は 4 種類を求める。design.md:147-149 は自分で「Q8 の推奨の文…とずれるので、GO 資料で見せる」と書くが、`docs/briefs/ST11-deep.html` の Q8 は `"rec": ["求めずに、使えないことを記録して収集を始める"]` のままで、`docs/briefs/ST11.html` にも D6 のずれの記述は無い（`grep -o "履歴[^<]\{0,60\}"` で 5 件、どれも題と要件の逐語）。
  本人が Q8 を推奨のまま返すと、第 1 回の流れでは推奨と違う答えだけが `spec --answers` で名指しされるので Q8 は名指しされず、record は deep.md に「求めない」を書き、spec は「求める」のまま残る。しかも D6 の理由（「求めない側は起点を遅らせる」）は Q1 と同じ失い方（uncaptured）で、Q8 が B である根拠「待っても失うものは無い」は履歴に未対応の端末では成り立たない
- kind: conflict
- loss: uncaptured —— （履歴の権限が使えない端末で、読める範囲の起点が遅れる。P13）
- 提案: 本人に渡す前に、Q8 の選択肢を 2 つの場合に分ける（「Health Connect が無い・更新が要る → 求めずに記録する」と「使えるが履歴・背景に未対応 → 種類と対応しているものだけ求める」）。そうすれば推奨のまま答えても spec と合う。分けないなら、Q8 の推奨の文を D6 に合わせて直し、GO 資料にも出す
- 処置: escalated —— 本人に渡す前に、Q8 の推奨を「無い・更新が要る端末は求めずに記録し、履歴に未対応の端末は種類と対応しているものだけを求める」に分け、求めない側の選択肢に失うもの（起点の遅れ）を書いた（`deep-questions.json`・`deep.md`・proposal の表）。design D6（仮）を合わせた

## R2. 途切れの対象外・稼働状況の格子に出さない・書き出しに含める、は `device-collection` の外の振る舞いなのに、INDEX の割当も理由も無い
- 分類: 置き場の誤り（観点 3）
- 成果物: openspec/changes/st11-health-history-permission/specs/device-collection/spec.md / proposal.md
- 根拠: spec.md:211-212 と Scenario「健康データの権限の記録が何日も届かなくても途切れの知らせは出ない」「稼働状況の格子に健康データの権限の行は出ない」「書き出しに健康データの権限の記録が入る」（spec.md:223-236）は、途切れの判定（ST14）・稼働状況の格子・書き出しの振る舞い。
  proposal.md:66 は `c01-clock` の前例を理由にしているが、その前例（正典 spec.md:1213-1225「端末の時計の測定記録は外部サービス上の識別子を持たずに受け付けられる」、INDEX.md:347）が `device-collection` に置いたのは**識別子なしで受け付ける**ことだけ。`c01-clock` の途切れの対象外は ST14 の試験（`crates/server/src/outage_registry_tests.rs:37` の `st14_reg_excluded_sources_are_marked`）の Scenario が持っている。
  INDEX.md:81 は ST11 を `device-collection` だけに割り当てていて、proposal.md:65 は「前倒しは無い」と書く。同じ状況の ST09 は `collection-coverage` と `data-durability` にも割り当て、INDEX に理由を書いている（INDEX.md:394）
- kind: technical
- 提案: Requirement の 3・4 文目と 3 本の Scenario を、それぞれの capability の delta（途切れと格子は `collection-coverage`、書き出しは `data-durability`）に移し、INDEX の割当に ST11 を足して理由を書く。移さないなら、proposal と INDEX にその理由を書く
- 処置: fixed proposal.md —— 3 つの性質はこのソースの登録簿の 1 行の値から決まり、途切れ・格子・書き出しの規則は変えないこと、別の capability に置くと data-durability を MODIFIED している ST29・ST41・CH04 と盤面で重なることを Capabilities に書いた

## R3. 論理ソースの種類（FR-102。稼働状況）は移行の注記と契約にしか残らず、種類の列を作る ST38 への申し送りが無い
- 分類: 決定の写り（観点 1・観点 5）
- 成果物: openspec/changes/st11-health-history-permission/tasks.md（Task 9）/ design.md（D1）
- 根拠: Q5 の推奨の detail は「種類は『稼働状況』」で、irreversible は「『稼働状況』は外部 AI に承認済み（FR-88）なので、外部 AI の経路ができた後は許可の状態も外へ出る」（deep-review R7、loss: exported）。
  design.md:51 は「登録簿にはまだ種類の列が無い…この行の移行の注記と `docs/collector-contract.md` に書いておく」とし、spec.md:216 は Requirement の説明文だけで述べる（SHALL も Scenario も無い）。
  種類の列を作るのは ST38（INDEX.md:61。FR-88・FR-102 を満たす）で、`docs/handoff/ST38.md` は同じ型の申し送りを ST17（:21 `s01-feeling`）と ST09（:44 メディアのソースの種類）について既に持っている。tasks の Task 9 は `docs/handoff/ST34.md` だけを書く
- kind: technical
- loss: exported
- 提案: Task 9 に `docs/handoff/ST38.md` へ節を 1 つ足す項目を加える（`c01-health-permission` を「稼働状況」に属させること・その理由・外部 AI に承認済みの種類であること）。検証は `grep -q "c01-health-permission" docs/handoff/ST38.md`
- 処置: escalated —— 外へ出ること自体は Q5 の推奨の irreversible（FR-88）が本人に問うている（`deep.md` Q5 の出所）。ST38 への申し送りを Task 9.2 に足し、proposal の重なりに書いた

## R4. 端末の自動バックアップで「1 度求めた」の印が入れ直した後に戻りうる —— 「入れ直すと初回起動でもう一度求められる」が本人の携帯では成り立たない恐れ
- 分類: 前提が誤りうる（観点 1・観点 5）
- 成果物: openspec/changes/st11-health-history-permission/design.md（D4・D2）/ specs/device-collection/spec.md
- 根拠: design.md:114 は「入れ直すと `SharedPreferences` ごと消えるので初回起動と同じ」とし、D2（design.md:82-84）も「初めて観たとき（入れ直した後を含む）は最後の状態が無いので、必ず 1 件」とする。
  ところが `collector-android/app/src/main/AndroidManifest.xml` には `android:allowBackup` も `dataExtractionRules` も無い（`grep -rn "allowBackup\|fullBackupContent\|dataExtractionRules" collector-android/app/src/main/` は 0 件）。Android の自動バックアップは既定で有効で、`SharedPreferences` を含み、入れ直したときに戻しうる。`docs/briefs/ST09-pr.md:60` も「既定の自動バックアップが有効のまま（バックアップに実際に入るかは確かめていない）」と挙げている。
  戻れば `asked_health_permissions` が残り、Story の完了の判定 1（`docs/stories/ST11.md:39`「アプリを入れ直すと、初回起動で…ダイアログが出る」）と Scenario「入れ直すと初回起動でもう一度求められる」（spec.md:35-38）が本人の携帯で偽になる。最後に記録した状態も戻るので、全部断っていた端末を入れ直すと状態が同じに見え、D2 の「必ず 1 件」も起きない。
  tasks 7.1（tasks.md:132）は `pm reset-permissions` と `pm clear` で入れ直しを模すので、バックアップからの戻しは試験に現れない（確かさ 2: 戻しが本当に起きるかは確かめていない）
- kind: premise
- 提案: design D4 に、印と最後の状態をバックアップから外すこと（`dataExtractionRules` / `fullBackupContent` で `collector` の置き場を除く。既存の `sent_to_usage_access` にも効く）を書き、manifest の試験で固定する。外さないなら Scenario と完了の判定の「入れ直す」に「バックアップから戻さない場合」の条件を書く
- 処置: escalated —— Q4 の context に自動バックアップの前提（確かめていない）を書いて本人に見せる（`deep.md` Q4 の出所）。どの答えでも失わない側として design D14（仮）（健康データの印と最後の状態を専用の置き場に持ち、バックアップから外す）・spec の本文・Task 2.1 の試験に入れた

## R5. 4 本の Scenario が計測テスト（`AT`）にしか印を持たず、tasks 自身の「`AT` だけで閉じない」に反する
- 分類: tasks の検証（観点 4）
- 成果物: openspec/changes/st11-health-history-permission/tasks.md（Task 7.1）
- 根拠: tasks.md:28-29「`AT` を挙げた項目は、同じ振る舞いを `GT`…でも確かめる」、tasks.md:134「同じ振る舞いの `GT` は Task 2・5・6」。
  ところが `初回起動で健康データの権限の画面が出る` / `種類と履歴と背景をすべて許すとすべて許された状態になる` / `入れ直すと初回起動でもう一度求められる` / `種類を許して履歴の読み取りだけを断っても収集は始まる` の 4 本は、tasks.md の中で 7.1（:129-130）に 1 回ずつしか出てこない（`grep -cF` で 1）。Task 2・5・6 の Scenario の一覧（:70・:106-109・:120）に無い。
  手元の `/dev/kvm` が無いことがある（tasks.md:28 自身が書く）ので、この 4 本は CI でしか確かめられない
- kind: technical
- 提案: Task 5.1 の `HealthPermissionFlowTest` に、偽の `HealthPermissions` で「求めた intent が 6 つを含む」「4 種類だけ許した後の状態で収集が始まる」「印を消した `SharedPreferences` で開くともう一度求める」を足して 4 本の印を置く。置けないもの（Health Connect の画面の並び）は、7.1 に「`AT` だけで閉じる理由」を書く
- 処置: fixed 5.1 —— 4 本を `HealthPermissionFlowTest` にも名指しし（偽の `HealthPermissions` で求めたもの・読み直した状態・`HEALTH_PREFS` を消して開く）、画面そのものは 7.1 だけが見ると書いた

## R6. Task 7.1 の 8 本は、それぞれ別の Health Connect の権限の状態から始める必要があるのに、2 段実行の 1 段（1 回の `pm reset-permissions`）に全部載る
- 分類: tasks の検証（観点 4）
- 成果物: openspec/changes/st11-health-history-permission/tasks.md（Task 7.1）/ design.md（Risks）
- 根拠: `@NeedsPristinePermissions` の本は `tools/android-emulator.sh` の 2 段目（`annotation="$A"`）でまとめて走り、段の前に `pm reset-permissions` が 1 回だけある。自分の権限を自分で外すとプロセスが死ぬ（同じ台本の注記、`docs/handoff/ST11.md:44-47`）。
  7.1 の本は「すべて許す」「履歴だけ断る」「すべて断る」「入れ直す」で始まりの状態がそれぞれ違い、さらに 2 回断ると以後は画面が出ない（premises.md P6）ので、同じ段で並ぶと順番（「クラスの並びは dex 次第」。`tools/android-emulator.sh` の 3 段目の注記）で結果が変わる。
  tasks.md:132 の「`pm reset-permissions` と `pm clear` の後（`tools/android-emulator.sh` の段の前提づくり）」は、台本も CI（`.github/workflows/ci.yml` の android-instrumented）も「`pm clear` は使わない —— 前景サービスが残っていると Failed」と書いていて、段の前提に `pm clear` は無い。design.md:227 の「断る本を最後の段に置く」は段を足すことだが、`tools/android-emulator.sh` と `ci.yml` は tasks の「触るもの」（tasks.md:6-7）に無い
- kind: technical
- 提案: 7.1 に、本ごとの始まりの状態と、それを作る段（`tools/android-emulator.sh` と `ci.yml` に足す段。どの本をどの段に置くか）を書き、「触るもの」に 2 つのファイルを足す。「入れ直す」は段の間で `pm reset-permissions` をして、印は試験の中で自分の `SharedPreferences` を消すなら、そう書く（`pm clear` を前提にしない）
- 処置: fixed 7.1 —— 始まりの状態ごとに 3 つのクラスに分け、`tools/android-emulator.sh` と `ci.yml` に段を足す（触るものに追加）。`pm clear` を前提にせず、入れ直しは試験の中で自分の `HEALTH_PREFS` を消す。断る本は 1 段に 1 つまで

## R7. Task 2.1 の `! grep -rn "永久に読めない" collector-android/` は、組み立ての中間物（`app/build/`）の古い manifest に当たって落ちうる
- 分類: tasks の検証が書いたまま走らない（観点 4）
- 成果物: openspec/changes/st11-health-history-permission/tasks.md（Task 2.1）
- 根拠: tasks.md:71。`collector-android/app/build/` は gitignore されているが grep は見る。本体の checkout（`~/dev/ashiato2`）では `grep -rl "永久に読めない" collector-android/app/build` が `intermediates/merged_manifest/…/AndroidManifest.xml`・`merged_manifests/…`・`packaged_manifests/…`・`manifest-merger-blame-debug-report.txt` の 4 件に当たる（manifest の注記は合成後も残る）。
  手元で一度組んだ後、`GT` が作り直さない variant（androidTest・release など）の中間物が残っていれば、ソースを直しても rc=1 になる
- kind: technical
- 提案: `bash -c '! git grep -n "永久に読めない" -- collector-android/'` にする（追跡しているファイルだけを見る）
- 処置: fixed 2.1 —— `git grep` にした

## R8. Task 2.1 の `:app:lintDebug` は、このプロジェクトのどこでも関門になっていない Android lint を初めて関門にする
- 分類: tasks の検証（観点 4）
- 成果物: openspec/changes/st11-health-history-permission/tasks.md（Task 2.1）
- 根拠: tasks.md:72 `… :app:lintDebug :app:detekt` rc=0。`docs/testing.md:36` の android job の lint は「detekt + ktlint」、`docs/testing.md:174` と `.github/workflows/ci.yml` の android job は `./gradlew detekt` だけで、`lintDebug` を走らせる場所は無い（`grep -rn lintDebug .github docs/testing.md` は 0 件。tasks.md:72 だけ）。lint の baseline も無い（`collector-android/app/lint*` は無い）。
  既存のコードに Android lint の誤りがあれば、ST11 と関係なく 2.1 が閉じられない
- kind: technical
- 提案: `:app:lintDebug` を外して `:app:detekt` だけにする。Android lint を関門にするなら、別の change で baseline を作ってから
- 処置: fixed 2.1 —— `:app:lintDebug` を外し `:app:detekt` だけにした

## R9. Task 8.1 の「`CollectorContractFixture` に 1 件足し、台本はその JSON を読む」は、在る仕掛けの形と合わない
- 分類: tasks が存在しない仕掛けを前提にしている（観点 4）
- 成果物: openspec/changes/st11-health-history-permission/tasks.md（Task 8.1・Task 1.2）
- 根拠: tasks.md:147。`collector-android/app/src/test/kotlin/dev/ashiato/collector/CollectorContractFixture.kt` は Kotlin の試験の部品で、`docs/collector-contract.md` の節の表（3 列: 欄・型・省略の規則）を読んで `payload` の形と照らす（`assertPayloadMatchesContract`）。bash の台本が読める JSON は出さない。
  節の見出しも `contractTable("## C-01（\`$logicalSource\`）が送る \`payload\` の形")` の形で引くが、Task 1.2（tasks.md:59）の見出しは「端末の健康データの権限（`c01-health-permission`）が送る `payload` の形」で、この形に当たらない（`c01-clock` の節 `docs/collector-contract.md:281` も同じ理由でこの部品の外）。D2 の `features` / `granted` は入れ子のオブジェクトで、3 列の表の型の欄（`assertJsonType`）に載るかも決まっていない。
  縦串の前例（`tools/smoke.d/202610200900_st09-media-capture.sh`）は、記録の形を台本の中に書いている
- kind: technical
- 提案: 8.1 は台本の中に D2 の形の JSON を書く（ST09 と同じ）とし、端末が作る形と契約が同じであることは 4.1 の形の試験（`ClockSkewPayloadShapeTest` の型）と 1.2 の契約の表で固定する、と書き直す。`CollectorContractFixture` に足すなら、見出しの形と入れ子の欄の扱いを 1.2 に書く
- 処置: fixed 8.1 —— 縦串は台本の中に D2 の形の JSON を書く（ST09 と同じ）。形の一致は 4.1 の形の試験と 1.2 の契約の節で固定する。1.2 の見出しも `C-01（`c01-health-permission`）が送る `payload` の形` に揃えた

## R10. Task 2.1 の試験が、Task 3.1 で作る定数 `HEALTH_PERMISSIONS` を使う
- 分類: tasks の依存順（観点 4）
- 成果物: openspec/changes/st11-health-history-permission/tasks.md（Task 2.1・3.1）
- 根拠: tasks.md:69「`HealthManifestTest`…: manifest が `HEALTH_PERMISSIONS` の 6 つをちょうど宣言している」。`HEALTH_PERMISSIONS` を置くのは design D5（design.md:131）で、tasks では 3.1 の `HealthPermissions.kt`（tasks.md:76-79「`HEALTH_PERMISSIONS` の 6 つの名前」）。2.1 を先に閉じようとすると、まだ無い定数を参照して組めない
- kind: technical
- 提案: `HEALTH_PERMISSIONS` を置くのを 2.1 に移す（依存の要らない定数なので manifest と同じ Task に置ける）か、2.1 と 3.1 の順を入れ替える
- 処置: fixed 2.1 —— 定数 `HEALTH_PERMISSIONS` を 2.1 で `HealthPermissions.kt` に先に置く。3.1 は同じファイルに口を足す

## R11. Task 4.2 の検証は `HealthPermissionRecordTest` の本数を見るが、本文は試験を `SourceIndependenceTest` に置いてよいと書く
- 分類: tasks の検証（観点 4）
- 成果物: openspec/changes/st11-health-history-permission/tasks.md（Task 4.2・7.2）
- 根拠: tasks.md:95「`SourceIndependenceTest` か新しい試験で」、tasks.md:98「検証: `GT HealthPermissionRecordTest`（9 本以上。4.1 と合わせて）」。`SourceIndependenceTest` に置くと `HealthPermissionRecordTest` の本数は 8 のままで、下限 9 を満たせない（置き場を選ぶと検証が通らない）。
  また 7.2 の「`GT SourceIndependenceTest`（既存の本数 + 1 以上）」・「`AT PermissionDeniedInstrumentedTest`（既存の本数以上）」は数が書かれていない。いまの本数は `SourceIndependenceTest` が `@Test` 10 本、`PermissionDeniedInstrumentedTest` が 1 本
- kind: technical
- 提案: 4.2 の置き場を 1 つに決めて、検証の本数をそれに合わせる。7.2 の下限を数で書く（`SourceIndependenceTest` 11 本以上・`PermissionDeniedInstrumentedTest` 1 本以上。4.2 を `SourceIndependenceTest` に置くならさらに +1）
- 処置: fixed 4.2 —— 置き場を `HealthPermissionRecordTest` に決めた。7.2 の下限を数で書いた（`SourceIndependenceTest` 11 本以上・`PermissionDeniedInstrumentedTest` 1 本以上）

## R12. 要件の誤った理由の写しは「永久に読めない」の 6 か所の外にもある —— D13 の訂正の範囲が grep の語に依っている
- 分類: Story / 要件の本文との一致（観点 5）
- 成果物: openspec/changes/st11-health-history-permission/design.md（D13）/ tasks.md（Task 2.1）
- 根拠: design.md:217 は `grep -rn "永久に読めない"` の 6 か所を範囲にする。同じ理由から出た文が、この語を含まずに残る:
  `docs/requirements.md:950`（FR-73「収集を後回しにしても、権限の宣言は後回しにできない」）とその逐語 `docs/stories/ST34.md:20`、`collector-android/README.md:100`（「5 つの権限は、いずれも宣言しない期間が取り返せないか…」。ST11 で健康の権限は 6 つ増え、「5 つ」も合わなくなる）。
  訂正後の理由（履歴の権限は後から与えても全履歴が読める。P2）では「後回しにできない」「取り返せない」は言い過ぎになる。chain の観点 8 は、ST11.md と `stories.json` の両方が誤った理由のまま揃っているから通っている
- kind: premise
- 提案: D13 の範囲に `docs/requirements.md:950`（record の段。ST34.md は再生成で揃う）と `collector-android/README.md:100`（Task 2.1）を足す。Task 2.1 の検証に README の「5 つの権限」の数え直しを入れる
- 処置: escalated —— Q1 の推奨の detail の訂正の範囲に FR-73 と README の「取り返せない」を足した（`deep.md` Q1 の出所。訂正は record の段）。design D13 に FR-73:950 と README:100 を足し、README は Task 2.1 で直す

## R13. Q6 の推奨の見出しは「外へ出さないか」のままで、spec と design D7 は「ほかへは送らない」と書かないとしている
- 分類: 深掘りの推奨と spec の食い違い（観点 1）
- 成果物: openspec/changes/st11-health-history-permission/deep.md（Q6）/ deep-questions.json（Q6）/ proposal.md
- 根拠: deep.md:33 と deep-questions.json の Q6 の推奨の label は「短い固定の文（何を読み、どこへ送り、**外へ出さないか**）」、proposal.md:46 の表も同じ。deep-review R11 で detail の文例は「そこから先に何を渡すかは、あしあとの承認の設定で決まります」に直したが、label は直っていない。
  spec.md:134 は「健康の種類は外部 AI にも承認されている（FR-88）ので、『ほかへは送らない』とは書かない」、proposal.md:24 は「（何を読み、どこへ送り、そこから先は承認の設定で決まること）」と書く。本人は label を選ぶので、選んだ文と spec が言うことが違う
- kind: conflict
- 提案: Q6 の推奨の label を「短い固定の文（何を読み、どこへ送り、その先は承認の設定で決まること）」に直し、deep.md と proposal.md:46 の表の写しも揃える
- 処置: escalated —— Q6 の推奨の label を「短い固定の文（何を読み、どこへ送り、その先は承認の設定で決まること）」に直し、deep.md と proposal の表を揃えた（本人に渡す前）

## R14. 設定の画面で後から**許した**ことの記録・写真と動画を止めないこと・通知の操作が Health Connect の「更新が要る / 読めない」で出ないこと、は Requirement の本文だけが言っている
- 分類: Scenario の検証可能性（観点 2）
- 成果物: openspec/changes/st11-health-history-permission/specs/device-collection/spec.md
- 根拠:
  - spec.md:60「本人が後から Health Connect の設定で**許す・外す** THE SYSTEM SHALL 次に…記録する」に対し、Scenario は「後から設定で外すと次の見直しで記録される」（:182-185）だけ。断った後に通知から開いて許す（Q4 の推奨の道そのもの）の記録は Scenario に無い
  - spec.md:57「位置・アプリ利用・**写真と動画**の取得を止めない」に対し、Scenario は位置（:69）と位置・アプリ利用（:319-322）だけ
  - spec.md:59「WHILE Health Connect が使える THE SYSTEM SHALL 通知に…操作を出す」に対し、出ない側の Scenario（:81-84）は題が「使えない端末」なのに WHEN は「Health Connect が無い端末」だけで、更新が要る・読めない端末で出ないことは言っていない
  - spec.md:150-151 の記録の欄（履歴・背景に端末が対応しているか・どこで観たか）は、どの Scenario の THEN にも出ない
- kind: technical
- 提案: 「断った後に設定で許すと次に観たときに記録される」「健康データの権限を断っても写真と動画の記録が積まれる」を足す。通知の Scenario の WHEN を「Health Connect が使えない（無い・更新が要る・読めない）」にするか題を「無い」にそろえる。対応の欄と観た契機は「履歴に対応していない端末」と「断ると…」の THEN に 1 行ずつ足す
- 処置: fixed 5.1 —— spec に「断った後に設定で許すと次に観たときに記録される」を足し、MODIFIED の Scenario を「位置とアプリ利用と写真は集まる」にした。通知の出ない側の WHEN を無い・更新が要る・読めないの 3 つにした。対応の欄は「履歴に対応していない端末」の AND、契機は「初めて観た」「断ると」「後から外すと」の THEN に入れた。tasks 5.1 が名指しする

## R15. 「初めて観たとき」「断ると」の「記録が 1 件」は、D9 では最初の起動に 2 件できる流れと数え方が決まっていない
- 分類: Scenario の検証可能性（観点 2）
- 成果物: openspec/changes/st11-health-history-permission/specs/device-collection/spec.md / design.md（D9・D4）
- 根拠: design.md:183-184 では、使える端末の最初の起動で `app_open`（`auto_requested=false`）の 1 件と、結果が返ったときの `request_result`（`auto_requested=true` に変わる）の 1 件ができる。Scenario「初めて観たときに…記録が 1 件、未送信に積まれる」（spec.md:162-165）と「断ると…記録が 1 件、未送信に積まれる」（:177-180）の「1 件」は、未送信の全部の数なのか、その契機の記録の数なのかで真偽が変わる（全部の数なら、断った後は 2 件で偽）。
  また design.md:110 は「健康データの段に入るのは `availability = available` で…」とし、design.md:183 は `app_open` を「`proceed()` が健康データの段を通る」ときにする。字面どおりだと Health Connect が無い端末では段に入らず `app_open` の観測が起きないので、Scenario「Health Connect が無い端末では…記録に、Health Connect が無いことが載る」（spec.md:98-102）が起動では満たされない
- kind: technical
- 提案: 2 本の THEN を「その契機（開いた時 / 結果が返った時）の記録が 1 件」と書く。D9 の `app_open` は「使えるかに依らず、起動ごとに最初に `proceed()` を通るとき」と書き、D4 の段に入る条件（求めるか）と分ける
- 処置: fixed D9 —— THEN を「その契機の記録が 1 件」にした。D9 の `app_open` を Health Connect が使えるかに依らない契機にし、D4 の求める条件と分けた

## R16. design D4 の「印は結果が返ったときに立てる」は観測できる振る舞い（Health Connect の画面の途中でプロセスが終わると次の起動でもう一度求める）なのに、Scenario が無い
- 分類: 置き場の誤り（観点 3）
- 成果物: openspec/changes/st11-health-history-permission/design.md（D4）/ specs/device-collection/spec.md
- 根拠: design.md:112「開いたときに立てると、…答えていないのに二度と出ない（ST13 の /code-review R33 と同じ型）」。これは「自動で求めるのは 1 度だけ」（spec.md:8）の「1 度」が何を数えるかを決める振る舞いで、本人に見える（答えずに終わった回は数えない）。`openspec archive` は main specs しか更新しないので、design にだけあると正典から落ちる。
  同じく design.md:110-111「全部許されていれば印を立てずに次へ進む」も、その後に設定で外すと次の起動で 1 度求める、という見える振る舞いになる
- kind: technical
- 提案: 「健康データの権限の画面に答えずにアプリが終わると、次の起動でもう一度求められる」の Scenario を足し、spec.md:8 の「1 度だけ」を「結果が返った 1 度」と書く。全部許されていたときの扱いも 1 文足す
- 処置: fixed D4 —— spec の本文を「結果が返った 1 度」にし、「権限の画面に答えずにアプリが終わると次の起動でもう一度求められる」の Scenario と、全部許されていたときの扱いの 1 文を足した（tasks 5.1）

## R17. design D8 の Android 13 以前の開き先（Health Connect のデータの管理の画面）は、Scenario の「収集アプリの Health Connect の権限の画面」と違う
- 分類: design と spec の食い違い（観点 3）
- 成果物: openspec/changes/st11-health-history-permission/design.md（D8）/ specs/device-collection/spec.md
- 根拠: design.md:171「13 以前は client の `HealthConnectClient.getHealthConnectManageDataIntent(context)`（Health Connect のアプリの画面）」。spec.md:59 と Scenario「常駐の通知から健康データの権限の画面が開く」（:76-79）は版を限らずに「収集アプリの Health Connect の権限の画面が開く」と言う。データの管理の画面が収集アプリの権限の画面か（そこから何回押せば権限に届くか）は premises に無い（P12 は Android 15 の `MANAGE_HEALTH_PERMISSIONS` だけ。13 以前は確かさ 2 の P15）。
  6.1 の試験（tasks.md:121）は `PendingIntent` の action と package を見るだけなので、13 以前の開き先が違っても通る
- kind: technical
- 提案: Scenario の WHEN を「Android 14 以降の端末で」に絞り、13 以前は「Health Connect の画面が開く」と別に書く。あるいは D8 を 13 以前でも収集アプリの権限の画面を開く口に替え、どちらかを Open Questions に置く
- 処置: fixed D8 —— spec の本文に版ごとの開き先を書き、Scenario の WHEN を Android 14 以降に限った。13 以前の開き先を Open Questions に置いた

## R18. 端末のログについて、Requirement は「記録したことと契機だけ」と言い、design D11 と tasks は `availability` も出す
- 分類: spec と design / tasks の食い違い（観点 3）
- 成果物: openspec/changes/st11-health-history-permission/specs/device-collection/spec.md / design.md（D11）/ tasks.md
- 根拠: spec.md:155「端末のログに、許されているものの中身を出さず、記録したことと契機**だけ**を出す」。design.md:206「ログは `health_permission_recorded`（`trigger` と `availability` だけ）」、tasks.md:41・:87 も `availability` を出す。Scenario（spec.md:202-205）は「種類ごとの許可の値は含まれない」なので、`availability` があっても通り、本文の「だけ」との差は試験に出ない
- kind: technical
- 提案: spec.md:155 を「記録したことと契機と Health Connect が使えるかだけを出す」に直す（D11 に合わせる）か、D11 と tasks から `availability` を外す
- 処置: fixed D11 —— spec の本文を「記録したことと契機と Health Connect が使えるかだけ」にした（D11 に合わせた）

## R19. 説明の画面の Scenario の THEN は 4 つの中身を言うが、試験は定数との一致だけを見るので、文を変えても通る
- 分類: Scenario の検証可能性（観点 2）
- 成果物: openspec/changes/st11-health-history-permission/specs/device-collection/spec.md / tasks.md（Task 2.1）
- 根拠: Scenario「権限の画面からプライバシーポリシーを開くと説明の画面が出る」の THEN（spec.md:139）は「読む種類と送る先と承認の設定と、いまは読まないことが出る」の 4 つを 1 行に束ねている。tasks.md:69 の試験は「開いた画面の文が `HEALTH_RATIONALE`」で、定数の中身から「承認の設定」を落としても、試験は定数と画面を比べるだけなので緑のまま
- kind: technical
- 提案: 試験に、文が 4 つの種類の名前・「自宅の PC」・「承認の設定」・「取り込みを入れてから」を含むことを 1 つずつ assert する、と 2.1 に書く
- 処置: fixed 2.1 —— 文が 4 つの種類の名前・「自宅の PC」・「承認の設定」・「取り込みを入れてから」を 1 つずつ含むことを assert する

## R20. Q5 の推奨の「端末にも残す」に当たる振る舞いが spec に無い
- 分類: 決定の写り（観点 1）
- 成果物: openspec/changes/st11-health-history-permission/specs/device-collection/spec.md / design.md（D2・D11）
- 根拠: deep.md:28 の推奨は「S-01 へ記録として送る（新しい論理ソース。**端末にも残す**）」。spec と design で端末に残るのは、未送信の間の記録（送れば消える。D11）と、判定に使う最後の状態の 4 欄（時刻を持たない。design.md:82）だけ。観た時刻や履歴を端末に残す振る舞いは無く、何が「端末にも残す」なのかが書かれていない
- kind: technical
- 提案: 「端末にも残す」が最後の状態（時刻なし）のことなら、spec.md の Requirement にそう 1 文書き、GO 資料で本人に見せる。時刻つきで端末に残す意味なら、置き場と保持の期間を design に足して Scenario を 1 本置く
- 処置: fixed D2 —— 「端末にも残す」は最後の状態（時刻なし）と未送信の記録のことと D2 に書き、spec の本文にも 1 文足した

## R21. 走っている change との重なりの記述で、ST23 が `excluded.rs` を触ることが抜けている
- 分類: 走っている change との重なり
- 成果物: openspec/changes/st11-health-history-permission/proposal.md
- 根拠: proposal.md:85「書き出し（`data-durability`）は ST29・ST41・CH04 が触っているが、ST11 は…`excluded.rs` の一覧に名前を 1 つずつ足すだけ」。`openspec/changes/st23-record-erasure/tasks.md:6`・`:69` は `export/excluded.rs` を触る（capability は `record-deletion` / `browsing-views` なので data-durability の並びに出ない）。ST29 の成果物には `excluded.rs` が出てこない（`grep` で 0 件）。ST41 は `SPAN_RULES` / `NO_COORD_SOURCES` に触らないと書いている（`openspec/changes/st41-event-end-time/tasks.md:7`）ので、一覧への 1 行の追加とは当たらない
- kind: technical
- 提案: proposal.md:85 の `excluded.rs` を触る change を「ST23・ST41・CH04」に直す（ST29 は `migrations/ORDER` だけ）。解き方（後から merge する側が rebase）は今のままでよい
- 処置: fixed proposal.md —— `excluded.rs` を触る change を ST23・ST41・CH04 に直した

## R22. 完了の判定 2 は「許可した日時」だが、spec が残すのは「観た時刻」で、設定から許した場合は最大 6 時間ずれる
- 分類: Story と deep の決定の一致（観点 5）
- 成果物: docs/stories/ST11.md / openspec/changes/st11-health-history-permission/specs/device-collection/spec.md
- 根拠: `docs/stories/ST11.md:40`「許可した日時が記録される」。spec.md:148 は観る契機（開いた時・結果が返った時・6 時間ごとの見直し）での「観た時の端末の時計の時刻」を残し、design.md:187 は「変わった瞬間は 6 時間の幅になる」とする。初回の画面で許したときは結果が返った時刻で一致するが、断った後に設定で許したときの記録は許した時刻ではない
- kind: daily
- 提案: 確認バッチで本人に見せる文に「設定から許したときは、許した後に最初に観た時刻（最大 6 時間後）が残る」と書く。完了の判定を record の段で「許可の状態を観た時刻が記録される」に揃えるかは、R12 の要件の訂正と一緒に決める
- 処置: fixed D9 仮 —— 「許可した日時」は「許可を観た時刻」になり、設定から許したときは最大 6 時間後になることを D9（仮）に書いた。確認バッチと PR 本文で本人に見せる。完了の判定の文を揃えるかは record の段（要件の訂正と一緒）

---

- 観点 1: 上の R1・R3・R13・R20。そのほかは確かめた範囲で写っている（Q3 の 4 種類・Q4 の 1 度だけと通知・Q5 の新しい論理ソース・識別子なし・途切れの対象外・時刻の 3 つと判定に時刻を使わないこと（R9）・Q7 の順・Q9 の背景・Q10 の THEN / AND の 5 点（`docs/handoff/ST11.md` の点 1〜5）。数値（4 種類・1 度・6 時間・30 日）はどれも Scenario にある）
- 観点 3 の MODIFIED: 「ソースごとに独立して収集する」は正典（`openspec/specs/device-collection/spec.md:872-942`）の全文を写している（`diff` の差は導出元の 1 行への追記と、末尾に足した Scenario 2 本だけ）。spec に関数名・crate 名・列名は無い。proposal の Capabilities（`device-collection`）と `specs/` のディレクトリは一致する（INDEX との差は R2）
- 走っている change との重なり: `openspec/changes/` の st23・st29・st41・ch03・ch04 の `specs/` に `device-collection` は無く、`MainActivity.kt`・`LocationService.kt`・`AndroidManifest.xml` を触るものも無い。`migrations/ORDER` は全部が末尾に 1 行足すが、union の merge で解ける。重なりは R21 だけ
- 観点 4 のそのほか: `-- --list` の下限は `CT` の定義（tasks.md:19-20）にあり、`/tmp` の固定名は使わず `"$TMPDIR"` を指定している（tasks.md:27。8.1 の `post` が書く `/tmp/smoke.body` は既存の `tools/smoke.sh` の手伝い）。名指しされた道具（`scripts/quiet-run`・`tools/check-migrations.sh`・`tools/check-immutable.sh`・`tools/check-log-private.sh`・`tools/check-shared-appends.sh`・`tools/android-emulator.sh`・`tools/smoke.d/README.md`）と既存の試験・部品（`ClockSkewPayloadShapeTest`・`MainActivityTest`・`MediaPermissionTest`・`SourceIndependenceTest`・`UnusedAppRestrictionsTest`・`LocationServiceTest`・`LocationServiceClockTest`・`testOutbox()`（`TestOutbox.kt`）・`ClockSkewMeasurer`・`NeedsPristinePermissions`・計測テスト 5 クラス・`clock_tests.rs`・`outage_store`・`excluded.rs` の `SPAN_RULES` / `NO_COORD_SOURCES` / `SPAN_NEXT_SOURCES`）は在る。移行の `acquisition` / `fetch_trigger` を trigger に入れさせる形（design D1）は `migrations/202610211000_source_acquisition.sql` の `source_acquisition_fill` と合う。人間の確認待ちが要る Task は無い（実機は Q2 で版を聞くだけ）
- 観点 5 のそのほか: `satisfies` は FR-11 だけで、spec の導出元に挙がる FR-2・FR-7・FR-19・FR-23・FR-61・FR-88・FR-102 は満たそうとしているのではなく、既存の正典と前例からの制約（FR-102 を満たすのは ST38。R3）
- 観点 6: 該当なし（`spec --draft`。`--answers` ではない）
