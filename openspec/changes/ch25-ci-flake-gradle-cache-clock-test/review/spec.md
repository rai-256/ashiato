# CH25 spec-review（成果物どうしの整合）

対象: `openspec/changes/ch25-ci-flake-gradle-cache-clock-test/`（proposal・design・tasks・premises・check-ch25.py・.openspec.yaml・lane.json・lane-verdict.json・deep.md）と `docs/changes/CH25.md`
照らした実物: `.github/workflows/{ci,nightly,release}.yml`、`crates/collector-windows/src/clock.rs`、`tools/release/{draft,build,check-guards,notices}.sh`、`.harness2/scripts/{evidence,lanes,review_triage}.py`、`actions/setup-java` の `src/cache.ts`（workflow が固定している commit `c5195efecf7bdfc987ee8bae7a71cb8b11521c00`。`gh api repos/actions/setup-java/contents/src/cache.ts?ref=c5195ef…` で取得）、CI の実際の run（`gh api …/actions/runs/<id>/jobs`）
前例: `openspec/changes/archive/2026-11-18-ch18-workbox-libs-cpus/review/spec.md`
確かめた日: 2026-11-21（worktree `ashiato2-up-ch25`、HEAD 97df9cd8f、`origin/main` = merge-base = 65eb1de27）。軽い路（`lane.json` の `reason: same`）、`skip_specs: true`

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate ch25-ci-flake-gradle-cache-clock-test --strict` | valid（rc=0。`skip_specs: true` なので delta 0 で通る） |
| `python3 scripts/check_chain.py .` | chain: OK（0 件 / 未回収 0 件 / warn 0 件） |
| `python3 scripts/check_scenarios.py . ch25-ci-flake-gradle-cache-clock-test` | scenarios: OK。warn のうち、この change に関わるのは `check-ch25.py` の文字列 `PC の時計が`（印ではない。害なし）だけ |
| `python3 scripts/review_triage.py . ch25-ci-flake-gradle-cache-clock-test` | triage: OK（この review を書く前の結果。指摘 0 / 仮決め 0） |
| `python3 scripts/spec_overlap.py candidates . ch25-ci-flake-gradle-cache-clock-test` | `[]`（0 件） |
| `python3 .harness2/scripts/evidence.py lint openspec/changes/ch25-…` | `[OK]` |
| `python3 .harness2/scripts/evidence.py unverified … --task 1` / `--task 2` / `--task 3` | `[]` / `[]` / `[]` |

検証コマンドは、`evidence.declared` で取り出した字面を `bash -c` でそのまま走らせた。「直した版」の列は、使い捨ての worktree で、tasks の書いたとおりに直してから走らせた結果:

| 項目 | base の木（今） | 直した版 |
|---|---|---|
| 1.1 の試験（tasks 1.1 の書いたとおりに足し、`fake_ingest` は今のまま） | **rc≠0**。`clock_reference_setup_time_is_not_in_the_difference` が `skew=302239 width=2` で落ちた（組み立ては通った） | — |
| 1.2 | rc=0。ただし **5 本だけ**走った（新しい試験は base にまだ無いため）。R3 | rc=0、6 本（`fake_ingest(shift: Duration)`・`skew_against(Duration::minutes(∓5))` に直した版） |
| 1.3 | rc=0（`OK: clock`） | rc=0（直した版でも、幅と印の字面は残る） |
| 1.4 | **rc=0**（`300 回 PASS`）。R4 | rc=0 |
| 1.4 と同じ実行ファイルを 5000 回 | **22/5000 で落ちた**（0.44%） | 0/5000 |
| 2.1 | **rc=1**（6 つの job がどれも「取り置きを持たない」） | rc=0（6 か所の `with` に `cache: gradle` を足した版） |
| 2.2 | — | rc=0 |
| 2.3 | rc=0（141 秒。自己試験の 4 つの Scenario と本番の判定が通った） | 走らせていない（`collector-android/` に触れない change なので、base と同じ結果になる） |
| 3.1 | rc=0 | 予定の外に未追跡のファイル（`tools/zz-review-probe.txt`）を置くと rc=1。置かなければ rc=0 |
| workflow を読む、2.2 に無い検査（`tools/release/check-guards.sh --self-test`・`check-adr.sh --self-test && check-adr.sh`・`check-test-db-growth.sh --lint`） | — | どれも rc=0。R5 |

予定のパスの外に触れる Task は無い。1.x・2.x の検証が書き込むのは `target/` と `.superpowers/`（ignore 済み）だけで、3.1 も base で rc=0 だった。ただし R1 を直すと `check-ch25.py`（change の中）と `ci.yml` の中で済むので、予定のパスは変わらない。

## R1. `chain` は gradle を呼ぶ（`draft.sh --self-test` → `build.sh apk`）。依頼は `chain` を名指しで挙げているのに、6 つの job から外れている
- 成果物: openspec/changes/ch25-ci-flake-gradle-cache-clock-test/proposal.md（What Changes・Impact）・design.md（D1）・tasks.md（Global Constraints・2.1）・check-ch25.py（`WORKFLOWS`・`GRADLE_RUN`）・docs/changes/CH25.md（何を 1・完了の判定 3）
- 根拠:
  - 呼び出しの経路: ci.yml:735 の `chain` の段 `./tools/release/draft.sh --self-test` → draft.sh:322 の `nightly_self_test` → draft.sh:91-92 `"$root/tools/release/build.sh" apk "$work/source" >/dev/null 2>&1`（偽物への差し替えは無い）→ build.sh:294-305 の `build_apk` が `./gradlew --no-daemon … :app:assembleRelease` を走らせる。先に `make_license_bundle apk` が notices.sh:211 で `gradlew … :app:ashiatoDependencyPoms` も呼ぶ
  - CI の実物: run 44745024963（main への push）の `chain` で、段 `Run ./tools/release/draft.sh --self-test` は 13:33:07→13:35:45（約 2 分半）。run 44744119514（PR）でも 13:26:53→13:29:24。本物の APK を組んでいる長さで、同じ run の `android` と `build-stamp` は skipped だった
  - `chain` には `if:` が無い（`needs` も無い）。どの PR でも、main への push でも、配布元から部品を取り直して release の APK を組む。6 つの job より多く走るので、配布元の不調に当たる機会も最も多い
  - 出力は `>/dev/null 2>&1` に捨てられる。そのため、`chain` が Maven の不調で落ちても、ログに `Could not find …` は出ない。proposal:37 は「`chain` の段を読み、gradle を使う job に数えない」と書くが、段の字面には gradle が現れず、台本の中で呼ばれる
  - `check-ch25.py ci` は、gradle を使う job を `GRADLE_RUN`（check-ch25.py:23-25）と「setup-java を持つか」で数える。`draft.sh` は `GRADLE_RUN` に無く、`chain` は setup-java を持たない。なので「6 つで全部」は、この検査では確かめられていない（base でも直した版でも、`chain` を数えずに通る）
  - design D1 の理由「6 つの job はどれも既に `setup-java` を持ち、`with` に 1 つ足すだけ」は、`chain` には当たらない。`chain` に取り置きを足すと、段が増える。これは Global Constraints（tasks.md:15-16「段の順・ほかの段…は変えない」）と、`check-ch25.py` の `without_cache` の比較（段を足すと「取り置きの指定の外で base から変わった」で落ちる）の両方に当たる
- kind: technical
- 提案: `chain` を gradle を使う job に入れる（依頼の「どこまで: gradle を使う全 job」どおり。要件は変わらない）。取り置きの形は D1 に足して仮で決める。案は 3 つある。
  (a) `chain` に `actions/setup-java`（同じ SHA）+ `cache: gradle` の段を足す。代償として、`chain` の gradle の JDK が、runner の既定から 21 に変わる。
  (b) ci.yml に SHA 固定で既にある `actions/cache` で `~/.gradle/caches`・`~/.gradle/wrapper` を取り置く。
  (c) `gradle/actions/setup-gradle`。
  setup-java の鍵には job が入らない（`cache.ts` の `computeCacheKey` は OS・arch・pattern の hash だけ）。そのため (a) では、`chain` も同じ鍵で「最初に保存した job の中身」を取り合う（R2）。`chain` は `if:` 無しで毎回走るので、保存役になりやすい。中身は release の APK の組み立ての部品だけ。(b) なら鍵を `chain` 専用に分けられる。
  あわせて次を直す: `check-ch25.py` の `WORKFLOWS["…/ci.yml"]` に `chain` を足し、`GRADLE_RUN` に `draft\.sh --self-test` を足す。選んだ形の段だけは「取り置きの指定」として `without_cache` が外す。Global Constraints・2.1・proposal の Impact（37-39 行）・CH25.md の何を 1 と完了の判定 3 は「7 つ」にする。gradle を使う job の洗い出しの根拠も、段の字面でなく、台本の呼び出しをたどった結果に書き直す（下の「確かめた範囲」）
- 処置: fixed D3 （`chain` を gradle を使う job に入れ、ci.yml で既に使っている `actions/cache` で取り置く仮の決定を足した）・fixed tasks.md （Global Constraints と 2.1 を 7 つの job に）・fixed check-ch25.py （`draft.sh --self-test` を gradle の印に・期待の集合に `chain`・`actions/cache` の段を取り置きとみなす）・fixed proposal.md・fixed docs/changes/CH25.md

## R2. 確認バッチ 4.1 の段の名前とログの字面が実物と違う。(2) の「取り寄せが減ったか」は今のログ設定では見えず、D1 の反転条件が発火しない
- 成果物: openspec/changes/ch25-ci-flake-gradle-cache-clock-test/tasks.md（4.1）・design.md（D1 の反転条件）・docs/changes/CH25.md（完了の判定 5）
- 根拠:
  - 6 つの setup-java の段は `name:` を持たない（ci.yml:220・340・366・450、nightly.yml:30、release.yml:43）。run 44684479094（依頼が挙げた run）の job の段の名前は、`Run actions/setup-java@c5195efecf7bdfc987ee8bae7a71cb8b11521c00` と `Post Run actions/setup-java@c5195ef…` だった。tasks.md:55 の `Set up JDK` と `Post Set up JDK` は、どの job にも出ない
  - setup-java（固定の commit）の `src/cache.ts` が出す字面は、復元のとき `Cache restored from key: …`、外れたとき `gradle cache is not found`、保存のとき `Cache saved with the key: …`、当たったとき `Cache hit occurred on the primary key …, not saving cache.`。tasks.md:55 の `Cache not found` はどれとも合わない
  - 「部品の取り寄せが減ったか」（tasks.md:57）: run 44684479094 の android の job（再実行の後の 136268684890）のログを `gh api …/actions/jobs/136268684890/logs` で取り、`download` を grep した。当たったのは 4 行で、action の取得 3 行と `Downloading https://services.gradle.org/distributions/gradle-8.13-bin.zip`（wrapper の取得）1 行だけだった。部品（Maven の jar・POM）の取り寄せの行は無い。android の段は `./gradlew detekt` などを `--info` なしで走らせ、build.sh・draft.sh は `-q` で出力を捨てる。このままでは、本人は「観測できなかった」と答えるしかない。そうなると、D1 の反転条件の 2 つ目（design.md:14「取り寄せが減っていないと分かったら」）は発火しない
  - 見える印は 2 つある。1 つは wrapper の行（`~/.gradle/wrapper` は取り置きに入るので、復元が当たれば `Downloading https://services.gradle.org/…` が消える）。もう 1 つは段の所要時間（同じログで `Run ./gradlew detekt` は 11:47:58→11:48:56 の約 58 秒、`Run ./gradlew :app:assembleDebug` は 11:50:15→11:51:10 の約 55 秒）
  - D1 の代償（design.md:12-13「先に終わった job の取り置きが保存される」）が効くのは、main への push で gradle の job が走ったときだけ。job は `if: needs.changes.outputs.{stamp,android,code}`（ci.yml:22-63）で絞られる。どの job が最初に保存するかは run ごとに違い、`chain`（R1）が加わればいつも走る候補が増える。android の部品（detekt・テストの依存）が取り置きに入るかは、保存した job で決まる
- kind: technical
- 提案: 4.1 (1) の段の名前を `Run actions/setup-java@…`／`Post Run actions/setup-java@…` に、字面を `Cache restored from key`・`gradle cache is not found`・`Cache saved with the key`・`Cache hit occurred on the primary key` に直す。(2) は見える問いに替える。たとえば次の 2 つ。
  ・main の run で、どの job の `Post Run actions/setup-java` に `Cache saved with the key` が出たか（job の名前）
  ・その後の PR の android で、`Cache restored from key` が出て `Downloading https://services.gradle.org/distributions/gradle-…` が消えたか。`Run ./gradlew detekt` の段が、base の約 58 秒（run 44684479094）より短いか
 D1 の反転条件にも「保存した job が android 系でなく、android の段の時間が縮まない」を足す。CH25.md の完了の判定 5 も同じ字面に合わせる
- 処置: fixed 4.1 （段の名前を `Run actions/setup-java@…`／`Post Run …` に、字面を `gradle cache is not found`・`Cache restored from key`・`Cache saved with the key`・`Cache hit occurred on the primary key` に。(2) を wrapper の取り寄せの行と detekt の段の時間に替えた）・fixed docs/changes/CH25.md （完了の判定 5）

## R3. 1.2 の検証は、新しい試験が無くても通る。「6 本以上」が本文にしか無い
- 成果物: openspec/changes/ch25-ci-flake-gradle-cache-clock-test/tasks.md（1.2）・check-ch25.py（`check_clock`）・docs/changes/CH25.md（完了の判定 1）
- 根拠:
  - base で `scripts/quiet-run clk -- cargo test -p ashiato-collector-windows --lib clock::tests::clock_reference` は rc=0、5 本だった（上の表）。tasks.md:30 の「（`-- --list` で 6 本以上）」は括弧の中の本文で、`evidence.declared` が取り出すのは `scripts/quiet-run clk -- cargo test …` だけ。0 本の検出（evidence.py:361 の `ran_no_tests`）は passed の和が 0 のときだけ落とす。5 本では効かない。本文の「N 本以上」を読む仕組みは evidence.py に無い
  - `check-ch25.py clock` は、2 つの Scenario の試験の幅と印の字面だけを見る（check-ch25.py:124-140）。`clock_reference_setup_time_is_not_in_the_difference` が在ること、その中で眠ることは見ない
  - `check_clock` は幅の字面だけを固定し、幅に入れる値の出どころは固定しない。`skew_against` が返す差（`(r.wall_after - r.time).num_milliseconds()`）と `width`（`r.uptime_after_ms - r.uptime_before_ms`）の式は見ない。たとえば `skew_against` の返す `width` に定数を足しても、2 つの試験の幅の字面は変わらない。1.2・1.3・1.4 は緑のまま、幅は実質で広がる（コードの読みから。走らせてはいない）
  - そのため、1.1 の試験を足し忘れても、名前を変えても、眠りを外しても、1.2 と 1.3 は緑のまま。残る確かめは、1.1 の FAIL の記録（`same` の路では機械が red を求めない。前例 CH18 R2）と、それを code-verify が読むことだけになる。完了の判定 1 の「用意の間に 1.5 秒眠る時計の試験が…直した後に通る」を、機械は落とせない
- kind: technical
- 提案: `check-ch25.py clock` に、`fn clock_reference_setup_time_is_not_in_the_difference(` が在ること、その本体に `std::thread::sleep` と `1_500`（または `1500`）が在ること、2 つの幅の字面（`RANGES` と同じもの）が在ることを足す。あわせて、`skew_against` の本体の差と幅の式（`(r.wall_after - r.time).num_milliseconds()` と `r.uptime_after_ms - r.uptime_before_ms`）が字面のまま在ることも見る。1.2 の検証は今のまま 1.3 がこれを受け持つ。あるいは 1.2 の検証を `bash -c` にして、`-- --list | grep -c ': test$'` が 6 以上であることも判定させる
- 処置: fixed check-ch25.py （1.3 の `clock` が、新しい試験の在りかと 1,500 ms の眠り・幅の字面、2 つの Scenario の試験がずらしで呼ぶこと、差と幅の式を見る。base で rc=1、直した下書きで rc=0 を確かめた）

## R4. 1.4 の 300 回は、base でもおよそ 4 回に 1 回通る。直したことの確かめとしては弱い
- 成果物: openspec/changes/ch25-ci-flake-gradle-cache-clock-test/tasks.md（1.4）・docs/changes/CH25.md（完了の判定 1）
- 根拠:
  - base の実行ファイルで、1.4 と同じ絞り込みを 5000 回走らせると 22 回落ちた（0.44%）。300 回すべて通る確率は (1−0.0044)^300 ≈ 0.27。実際に、1.4 の字面は base で `300 回 PASS`・rc=0 だった
  - 直した版は 0/5000。300 回は約 3 秒なので、回数を増やしても時間はかからない
  - 1.4 は出力を `-q >/dev/null` で捨て、走った本数を確かめない。実行ファイルの選び方（`kind==["lib"]` の最後）や `--skip` の字面がずれて 0 本になっても、rc=0 で `300 回 PASS` が出る（AGENTS.md の「0 本でも rc=0」）
- kind: technical
- 提案: 回数を 3000 にする（base で全部通る確率は約 2×10⁻⁶、時間は約 30 秒）。ループの前に `test "$("$bin" clock::tests::clock_reference --skip clock_reference_setup_time --list | grep -c ': test$')" -eq 5` を足して、本数を固定する。1.4 を「直す前に落ちる」証跡にはしない（1.1 が受け持つ）ので、base の FAIL の記録は求めなくてよい
- 処置: fixed 1.4 （3000 回にし、ループの前に 5 本であることを固定した。base で「落ちた: 7 回目」の rc=1 を確かめた）

## R5. 2.2 は、変える job（package-linux）を読む `check-guards.sh` を含まない
- 成果物: openspec/changes/ch25-ci-flake-gradle-cache-clock-test/tasks.md（2.2）・docs/changes/CH25.md（壊してはいけないもの 4）
- 根拠: `tools/release/check-guards.sh` の `workflow`（check-guards.sh:139-）は、ci.yml・nightly.yml・release.yml の `package-linux` の job の中身（`job_block … package-linux`）を grep で読む。ここは、この change が `with` を変える場所そのもの。`chain` が `--self-test` で走らせている（ci.yml:730）。2.2 の 3 つの検査（依存の検査の配線・Windows の組み立ての鍵・一時フォルダの配線）には入っていない。直した版では rc=0 を確かめた（上の表）。今は壊れないが、`chain` の形（R1）や `setup-gradle` への反転で段が増えると、ここが見張りになる
- kind: technical
- 提案: 2.2 の `bash -c` に `tools/release/check-guards.sh --self-test;` を足し、CH25.md の壊してはいけないもの 4 に「配る組み立ての守り」を添える。R1 で `chain` に段を足すなら、`./tools/check-adr.sh --self-test && ./tools/check-adr.sh`（ci.yml:703。ci.yml を読む）も足す
- 処置: fixed 2.2 （`tools/release/check-guards.sh --self-test` と `tools/check-adr.sh --self-test`・`tools/check-adr.sh` を足した）・fixed docs/changes/CH25.md （壊してはいけないもの 4）

## R6. premises と D1 が書く取り置きの鍵が、固定した setup-java の実物と違う。`gradle.properties` もロック（`*.lockfile`）も鍵に入らない
- 成果物: openspec/changes/ch25-ci-flake-gradle-cache-clock-test/premises.md（1 行目）・design.md（D1 の 11 行目・代償）・proposal.md（Impact の「ロックの検査が今と同じに効く理由」）
- 根拠:
  - setup-java の固定の commit `c5195ef…` の `src/cache.ts` では、gradle の pattern は `**/*.gradle*`・`**/gradle-wrapper.properties`・`buildSrc/**/Versions.kt`・`buildSrc/**/Dependencies.kt`・`gradle/*.versions.toml`・`**/versions.properties` の 6 つ。鍵は `setup-java-${RUNNER_OS}-${arch}-gradle-${hashFiles}`。premises.md:5 と design.md:11 が挙げる `**/gradle.properties` は無い
  - `git ls-files` をこの 6 つに当てると、鍵に入るのは `collector-android/{build.gradle.kts,settings.gradle.kts,app/build.gradle.kts,gradle/wrapper/gradle-wrapper.properties}` の 4 つだけ。`collector-android/app/gradle.lockfile`・`collector-android/buildscript-gradle.lockfile`・`collector-android/gradle.properties` は入らない
  - `restore` は `restoreKeys` を持たず、`save` は鍵が当たると保存しない（cache.ts の `matchedKey === primaryKey` で return）。そのため、ロックだけを書き直す変更（AGENTS.md の `--write-locks`）では鍵が変わらない。取り置きは古い中身のまま固定され、新しい版の部品は、`*.gradle.kts` が次に変わるまで毎回配布元から取られる。ロックの検査の結果は変わらない（解決はロックが縛る）ので、壊れはしない。ただ、取り置きの効き目が黙って落ちる
- kind: technical
- 提案: （外の形式の事実の訂正で、固定の commit の source で確かめ済み。人間に問う前提ではない）premises.md の 1 行目の鍵の列を、上の 6 つの pattern と「ロック・`gradle.properties` は入らない」に直し、出所に固定の commit の `src/cache.ts` を足す。D1 の代償に「ロックだけの更新では鍵が変わらず、取り置きが古いまま残る」を足す。消したいなら `cache-dependency-path` に `collector-android/**/*.gradle.kts`・`collector-android/**/gradle.lockfile`・`collector-android/buildscript-gradle.lockfile`・`collector-android/gradle/wrapper/gradle-wrapper.properties`・`collector-android/gradle.properties` を並べる。ただしこれは `with` に 1 つ足す形の外になるので、`check-ch25.py` の `without_cache` が外すこと（今も外している）を確かめる

## 観点 1: deep の決定が正典に写っているか
該当なし（確かめた範囲: deep.md は軽い路の申告だけで、本人の答え・当初案を覆したもの・要件へ戻すものが無い。申告の中身「CI の取り置き・`#[cfg(test)]` の中だけ・製品のコードに触れない」は、proposal・design・tasks・CH25.md と食い違わない。`skip_specs: true` で delta は無い）

## 観点 2: Scenario が検証可能か
該当なし（確かめた範囲: この change は Scenario を持たない。印を持つ既存の試験 2 本「PC の時計が進んでいると差が正で残る」「…遅れていると差が負で残る」は、幅と印の字面を固定する検査（1.3）がある。差と width の式は固定されない（R3）。直した版で、幅の字面と印が残り、6 本が通ることを確かめた。新しい試験の存在を機械が見ない点は R3）

## 観点 3: 置き場の誤り
該当なし（確かめた範囲: design D1・D2 は CI の取り置きの形で、製品の観測できる振る舞いではない。specs は無く、proposal の Capabilities は「なし」、CH25.md の `capability: []` と一致する。正典に CI の取り寄せを言う Requirement は無い）

## 観点 4: tasks が検証を持つか
R3・R4・R5 のほかに該当なし（確かめた範囲: 1.2〜3.1 のどれもコマンドと rc の終了条件を持つ。`evidence.py unverified` は Task 1〜3 で `[]`、`lint` は OK。1.1 は「検証:」を持たないが、1.2 の verify-run で FAIL を記録すると書いてある。base で 1.1 の試験が `skew=… width=…` で落ちることも確かめた。依存順は Task 1・Task 2 が独立で、3.1 が最後。人間の確認待ちは 4.1 に分けてある（字面は R2））

## 観点 5: change と CH25.md の整合
R1・R2 のほかに該当なし（確かめた範囲: CH25.md の「完了の判定」は空ではなく、tasks 1.1〜4.1 と対応する。「壊してはいけないもの」の 4 項はそれぞれ 1.3・2.1（`collector-android/` の差分も見る）・2.2・2.3 で固定される。依頼（逐語）の範囲は、次のとおり突き合わせた。
- 両方向の時計の試験: 1.2 で 2 つの Scenario の試験を `skew_against(Duration::minutes(∓5))` に替え、1.1 の試験が両方向を読む
- 幅を弱めない: 1.3 が幅の字面を固定する。差と width の式は固定しない（R3）
- 製品のコード（`HttpDateClock::now`）を変えない: 1.3 が `#[cfg(test)]` より前を base と字で比べる
- 依存の版を変えない: 2.1 が `collector-android/` の差分を見る
- ロックの検査が今と同じに効く: 2.3（base で rc=0・141 秒を確かめた）と 2.1 の「段は変えない」
- 欠けているのは、gradle を使う全 job のうち `chain`（R1）だけ。依頼を越える変更は無い

gradle を使う job の洗い出しは、`tools/` と `web/e2e` から `gradlew`・`build.sh apk`・`check-licenses.sh android`・`draft.sh` を grep し、それぞれを呼ぶ workflow の段まで戻ってたどった。setup-java を持たない job（`changes`・`vulns`・`rust`・`collector-windows`・`collector-windows-runtime`・`web`・`e2e`・`smoke`・`package-windows`・`install-windows`・`package-artifacts`、nightly と release の `plan`・`package-windows`・`publish`、`vulns-nightly.yml` の 3 つ）のうち、本物の gradle に届くのは `chain` の `draft.sh --self-test` だけだった。ほかは次のとおり本物の gradle に届かない。
- `rust` の `check-contract-version.sh`: 引数なし。gradle を呼ぶのは `--write` のときだけ
- `rust` の `test-android-emulator.sh`: 偽の gradlew を置く
- `chain` の `package.sh --self-test`: 偽の build.sh を使う
- `release.yml` の `publish` の `sign-apk.sh`・`check-artifacts.sh`: gradle を呼ぶのは `--self-test` の中だけ）

## 観点 6: 答えが写ったか
該当なし（`spec --answers` ではない。軽い路で問いが無い）

## 観点 7: 走っている change との前提関係
観点 7: 該当なし（spec_overlap.py candidates が 0 件）

## D1（仮）・D2（仮）の妥当性（依頼された見どころ）
- D1: `setup-java` の `cache: gradle` を選んだ理由（外の action が増えない・`with` に 1 つ足すだけ）は、6 つの job には成り立つ。ただし `chain`（R1）には成り立たない。代償の書き方は鍵の中身（R6）と、保存する job が決まらないこと（R2）を落としている。反転条件の 2 つ目は、今のログでは観測できない（R2）。仮として置くこと自体は妥当で、workflow の数行を戻せば元に戻る
- D2: 配る組み立て（nightly・release の `package-linux`）に取り置きを使うことは、依頼の「gradle を使う全 job」に合う。3 つの workflow の `on:` を読んだ。ci.yml は `push`（main）と `pull_request`、nightly.yml は `schedule` と `workflow_dispatch`、release.yml は `push`（`v*` の印）で、`pull_request_target` のように PR の中身が main の scope で走る入口は無い。なので、main の scope に書けるのは、main への push と schedule の nightly（と main で起こした `workflow_dispatch`）だけ。design.md:20 の「書けるのは main への push の run だけ」は字面が狭く、schedule の nightly も main の scope に書く。ただ、どれも main の中身なので結論は変わらない。PR と印の run は、それぞれ自分の scope に書く。印の run が読めるのは自分の印と main の scope だけで、premises.md:6 と合う。部品の checksum の確かめ（`verification-metadata.xml`）が今も無いことは、`git ls-files | grep -i verification-metadata` が 0 件であることで確かめた。そのため「確かめの強さは変わらない」は成り立つ。反転条件（確かめを入れたら・main 以外が書けるようになったら見直す）も筋が通っている。指摘は無い
- 処置: fixed premises.md （鍵の材料を固定した版の pattern に直し、ロックと `gradle.properties` が入らないことを書いた）・fixed D1 （`cache-dependency-path` に `collector-android/**/*.gradle.kts` と `collector-android/**/*.lockfile` を並べ、ロックだけの更新でも鍵が変わる形にした）・fixed tasks.md （Global Constraints）
