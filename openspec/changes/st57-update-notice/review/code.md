<!-- 処置の判定: re-review -->

# code_verify — ST57 `st57-update-notice`（2026-11-12、HEAD 9e87ab558）

## 申告と実測

申告: 実装の項目 15/15 が `[x]`（11.1・11.2 は人間の確認待ちで `[ ]`）。
実測: 下の検証コマンドを作業ツリーでそのまま走らせた（`.env` を読んだ。DB は `docker compose` の `db` が healthy）。

| 項目 | 検証コマンド（tasks.md のまま） | rc | 一致 |
|---|---|---|---|
| 1.1 | `cargo test -p ashiato-update-check st57_choose_` + `--list` ≥14 / 材料の件数 ≥10 | 0 / 0 | 一致 |
| 2.1 | `cargo test -p ashiato-updater st57_` + ≥12 / clippy（windows-gnu） | 0 / 0 | 一致（ただし 10.2 の全体の試験の 1 回目では、この中の 1 本が落ちた。R7） |
| 2.2 | `check-updater-deps.sh` / `--self-test` / `check-offsite.sh` / ci.yml の grep | 0 / 0 / 0 / 0 | 一致 |
| 3.1 | `cargo test -p ashiato-server st57_` + ≥14 / `check-openapi.sh` / `check-offsite.sh` | 0 / 0 / 0 | 一致 |
| 4.1 | e2e `st57-update-notice.spec.ts`（14 passed）/ `tsc --noEmit && vitest run`（519 passed） | 0 / 0 | 一致 |
| 5.1 | `cargo test -p ashiato-setup st57_` + ≥10 / clippy | 0 / 0 | 一致 |
| 6.1 | `check-artifacts.sh --self-test`（536 秒）/ `package.sh --self-test` / `notices.sh --self-test` / `check-licenses.sh` / `check-vulns.sh --base …` | 0 ×5 | rc は一致。ただし self-test の外では宛先の検査が本物の実行ファイルで落ちる（R8） |
| 7.1 | `update-check-install.sh --self-test` / `install-check.sh --self-test` / actionlint | 0 / 0 / 0 | 一致 |
| 8.1 | `cargo test -p ashiato-collector-windows st57_pc_` + ≥23 / 全体 / clippy / `check-no-time-server.sh` / `check-licenses.sh` / `check-updater-deps.sh` | 0 ×6 | 一致 |
| 8.2 | `check-artifacts.sh --self-test` | 0 | 一致 |
| 9.1 | gradle `*St57*` + XML ≥23（29 本）/ `check-android-backup-rules.sh` と `--self-test` / `check-no-time-server.sh` | 0 / 0 / 0 | 一致 |
| 9.2 | `tools/android-emulator.sh` / 結果の XML の St57UpdateQueryTest ≥2 | 未実行 / 0 | **不一致**: 件数の検査が古い写しを数える（R5）。エミュレータの全体の実行（約 25 分）はこの席では走らせていない。最新の段の XML（10:41 UTC、Android 側はその後の変更なし）には 2 本とも失敗なしである |
| 9.3 | `check-artifacts.sh --self-test` / `build.sh --self-test` | 0 / 0 | 一致 |
| 10.1 | install.md などの grep | 0 | 一致 |
| 10.2 | `check_scenarios.py` / `fmt --check && clippy --workspace && cargo test --workspace` / 道具 5 本 | 0 / **101**・101・0（3 回）/ 0 | **揺れる**（R7） |

作業ツリー: 壊す確かめは 1 回ずつ `git checkout --` で戻した。最後に `git status --porcelain` が ` M openspec/changes/st57-update-notice/review/code.md` の 1 行だけであることを確かめた（始める前は空）。e2e が壊した状態で組んだ `target/release/ashiato-server`（git の外）は消した。commit はしていない。

---

## 手 1: 固定値を独立に再計算する

共有の材料 `docs/contracts/update-check-vectors.json` の `cases` 30 件について、spec の「新しい版かどうかは、入っている版のチャンネルで決める」の文（印の形・下書き・配布物の名前・SemVer 2.0 の比べ方）だけから、Python で選び方を組み直した（`$TMPDIR/st57py/indep.py`。Rust・Kotlin の実装は読んでいない）。
`python3 -I indep.py docs/contracts/update-check-vectors.json` → `cases 30 mismatch 0`。期待値は実装の写しではなく、spec と一致している。
`timing` の 12 件は境界の値（23/25 時間・6/8 日・47/49 時間）で、固定の強さは手 4 に書いた。

## 手 2: ガードをわざと壊す

- `check-artifacts.sh` の更新係の宛先の検査（R8）: 配る形に組んだ本物の `ashiato-updater.exe` では落ちる。self-test は偽の実行ファイルに宛先を書き足して通している
- 9.2 の件数の検査（R5）: St57UpdateQueryTest が 0 件の実行でも rc=0
- `check-updater-deps.sh`: Linux の木を見ている。`--target x86_64-pc-windows-gnu` で見ても `webpki-roots` と `webpki-root-certs` は無かった（`--target all` には `webpki-root-certs` が出るが、Windows 向けではない）。指摘にしない
- `update-check-install.sh --self-test`: 判定の関数ごとに、外れた出力で落ちることを見ている（`expect_fail` の呼び出し 8 か所）。CI の差分（`git diff 8f1073321 HEAD -- .github/workflows/ci.yml tools/release/install-check.sh`）も読んだ: 遮断の段（`--block-outbound`）は `install-check.sh` の段より前にあり外す段は無い・job の終わりに `--list` の突き合わせがある・`install-check.sh` の遮断の一覧に `ashiato-updater.exe` がある。`check_zip` の本体は self-test では印を数えるだけ（tasks の取り決めどおり。本物は CI の `install-windows`）

## 手 3: Scenario と test を突き合わせる

`python3 scripts/check_scenarios.py . st57-update-notice` → `scenarios: OK`（rc=0）。change の spec にある 151 本はどれも印がある（置き場ごとの数: PC 23・Android の通知 23・install-check.sh 17・check-artifacts.sh 15・e2e 14・更新係 12・update-check 12・インストーラ 11・update-check-install.sh 10 ほか）。
`[warn] spec に無い Scenario を指す印: 携帯は新しい版をサーバに問い合わせない (St57UpdateCheckTest.kt)`。spec の名前は「携帯の**収集アプリは**新しい版をサーバに問い合わせない」で、`St57UpdateCheckTest.kt:79` の印は名前が違う。
主張と観測の階層がずれているもの: R1（更新係・PC の問い合わせの形）・R3（Android）・R4（PC）。画面の e2e 14 本は、帯の位置（`boundingBox`）・コントラスト（`getComputedStyle`）・360 px の横の流れ・要求の数を本物のブラウザで見ていて、ずれは見つからなかった。

## 手 4: 本人の決定が test で固定されているか

値を書き換えて、st57 の試験を走らせた（どれも戻した）。

| 決定 | 書き換え | 結果 |
|---|---|---|
| Q9 7 日（共有の crate `stale`） | `days(7)`→`days(8)` | `st57_pc_seven_days_is_stale` が落ちる → **固定されている** |
| Q9 7 日（携帯 `UpdateCheck.kt:39`） | `7`→`8` | St57 の 29 本が全部緑 → **固定されていない**（R6） |
| Q9 7 日（サーバの「まだ確かめていない」`update_status.rs:177`） | `days(7)`→`days(3)` | サーバの st57 17 本と e2e 14 本（サーバを組み直した）が全部緑 → **固定されていない**（R6） |
| Q13 5 分（`update_check.rs:171`） | `300`→`600` | `st57_pc_idle_boundary_is_five_minutes` が落ちる → **固定されている** |
| C9 48 時間（`updater_not_running`） | `48`→`49` | update-check 17 本・サーバ 17 本が全部緑（同じ回に 5 分も壊したので PC は 5 分の 1 本が落ち、残り 24 本は緑）。材料は 47/49 時間なので、47〜49 時間の間では変えても通る |
| C5 24 時間 | 読んで確かめた | PC の `due(checked+24h)` と Kotlin の `due(now, now-24h)` が 24 時間ちょうどを見ている。固定されている |
| Q5・Q6 新しい版ごとに 1 回 | 版の印の条件を外す | PC・携帯とも全部緑 → **固定されていない**（R3・R4） |
| Q9 確かめられない間に 1 回（携帯） | `stale_notified` の条件を外す | 全部緑 → **固定されていない**（R3） |
| Q3・Q4 切る口（PC・携帯の「切ってあれば問い合わせない」） | PC の `start`・`periodic` と携帯の `check` の早期 return を消す | PC の st57 25 本・携帯の St57 29 本が全部緑 → **固定されていない**（R11） |

## 手 5: tasks の `[x]` と実体

検証コマンドの rc は表のとおり。本文が名指ししたのに無いものは R5・R7・R9 に書いた（9.2 の P10 の書き直し・9.1 の試験の道具・8.1 の標準入力・10.2 の rc）。
`cargo test --test <名前>` 型の指名は無い（`--test` を使う検証コマンドは無い）。

## 手 6: 隙間

- 時計が進んだ状態で入れて後から戻すと、サーバは「確かめられていない」を何か月も出さない（R10）
- 試験の組み立ての携帯は、宛先を差し替えないと本物の配布元に問い合わせる（R2）
- 「捨てたものは復元できない」型は見つからなかった。切った印（`check.json`）は、走らせ直しでは `write_update_check`（`crates/installer/src/main.rs:210-214`）が既にあるファイルを上書きしない。`repair` は更新係に触れない（インストーラの試験と `update-check-install.sh`）

## 手 7: 該当なし

`python3 scripts/plan_fix.py pending openspec/changes/st57-update-notice` → `[]`（rc=0）。`correction-review.md` は作っていない。

---

## R1. 更新係と PC の収集アプリの試験は、FR-116 の問い合わせの形を固定していない（版・端末の識別子を見出しに足しても全部緑）
- 成果物: crates/updater/tests/st57_once.rs / crates/collector-windows/tests/st57_pc_update.rs / crates/updater/src/lib.rs / crates/collector-windows/src/update_check.rs
- 根拠: 一時的に `crates/updater/src/lib.rs:160` の後に `.header("X-Ashiato-Version", installed)` を足し、`crates/collector-windows/src/update_check.rs:356` の後に `.header("X-Ashiato-Version", installed_version())` と `.header("X-Device-Id", "pc-0123456789ab")` を足した → `cargo test -p ashiato-updater st57_` は `13 passed; 0 failed`（rc=0）、`cargo test -p ashiato-collector-windows st57_pc_` は `25 passed; 0 failed`（rc=0）。
  spec の THEN「見出しの名前は許した 5 つの内側…どの見出しの値にも `0.4.0`（と `pc-0123456789ab`）が無い」は、どちらの試験も見ていない。
  `// Scenario: 更新係の問い合わせは版の一覧の取得だけである` の印は、問い合わせを 1 件も出さない試験（宛先なし。`st57_once.rs:325-337`）に付いている。
  `// Scenario: PC の収集アプリの問い合わせは版の一覧の取得だけである`（`st57_pc_update.rs:118-131`）は件数 1 だけを見ていて、方法・道・見出し・本文を読んでいない。
  見出しの値を見ているのは nightly の試験（`st57_once.rs:103-108`。`contains` で 3 つの見出しの値）だけ。
  defaults C2 の「FR-116 の機械の検査は、方法・道・見出しの集合・本文が上のとおりであることを試験で固定する」は、更新係と PC では成り立っていない（FR-116 は、この Story で `exported` を失う唯一の問いの裏づけ）
- kind: technical
- 提案: 受け手が受けた要求を全部読み、見出しの名前の集合 ⊆ {Host, User-Agent, Accept, Accept-Encoding, Connection}・方法 `GET`・道・本文 0 バイト・どの値にも自分の版（と PC の端末の識別子・送り先の合言葉）が無いことを、更新係と PC の両方で assert する。印はその試験へ移す
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）
- 判定: ADDRESSED（re-review 2 @b998446f7a036724e0b53db8ea19d5ec388fb66c）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R2. 試験の組み立て（debug）の携帯は、宛先を差し替えないと本物の配布元 `https://api.github.com` に問い合わせる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateCheck.kt / collector-android/app/build.gradle.kts / tools/android-emulator.sh
- 根拠: `UpdateCheck.kt:122-124` は `testDestination ?: BuildConfig.UPDATE_CHECK_URL（空でなければ）?: "https://api.github.com"`。生成された debug の `BuildConfig.java` は `UPDATE_CHECK_URL = ""`（`build/generated/source/buildConfig/debug/.../BuildConfig.java`）。`android-emulator.sh:103` は `-PupdateCheckUrl` を渡していない。
  一時の Robolectric の試験（示した印あり・送り先あり・`UpdateChecker(app).check(now)`）で、`UpdateCheck.kt` の宛先を決めた直後に値を例外で出させた → `PROBE destination=https://api.github.com debugUrl=[]`（戻して、試験のファイルは消した）。
  `UpdateCheck.kt:125-128` の `no_destination` の枝は、この式では届かない。
  spec「宛先を差し替える口を試験の組み立てにだけ持たせ、試験の組み立ては差し替えの宛先が無ければ確かめない（自動の試験は本物の配布元に問い合わせない）」と C8・D8 に反する。エミュレータの計測テストと録画が debug の組み立てで収集のサービスを動かすと、示した印が立った後の生存信号の時に本物の配布元へ問い合わせうる
- kind: technical
- 提案: debug で宛先が空なら `no_destination` を記録して問い合わせない（PC の `update_url` の `cfg(ashiato_test_build)` と同じ形）。release の `BuildConfig` だけが配布元の定数を持つ。その振る舞いを Robolectric で固定する
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）
- 判定: ADDRESSED（re-review 6 @b42d98b49e7be0bb189627b42c31e2e87f6fffef）

## R3. 携帯の St57 の試験の多くは、印の Scenario の主張を観測していない（7 つの振る舞いを壊しても 29 本が全部緑）
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/St57UpdateNoticeTest.kt / collector-android/app/src/test/kotlin/dev/ashiato/collector/St57UpdateCheckTest.kt
- 根拠: 次を一時的に同時に壊した: (a) `canPost` からチャンネル `update` の重要度の条件を外す（`UpdateCheck.kt:188`）、(b) `notified_version` の条件を外す（`:148`）、(c) `stale_notified` の条件を外す（`:151`）、(d) 宛先を `Config.baseUrl`（サーバ）にする（`:122-124`）、(e) `LocationService` の `updateChecker.check()` の 2 か所の呼び出しを消す。
  → `gradlew :app:testDebugUnitTest --tests "*St57*"` は `BUILD SUCCESSFUL`（rc=0）。St57UpdateCheckTest 6 本・St57UpdateNoticeTest 23 本とも `failures="0"`。
  続けて (f) `SetupActivity` の `markIntroShown()` を消し、(g) `MainActivity` の `!usageAccessOpenedInThisStart` を外した → これも 29 本が全部緑（rc=0）。
  （(h) 携帯の回線のときに問い合わせない形に壊すと、`確認記録は noBackupFilesDir に置く` が落ちた。網の Scenario の試験ではなく、記録の試験が偶然拾っている）
  主張と観測のずれ（St57UpdateNoticeTest.kt の行）:
  `携帯の収集アプリは新しい版をサーバに問い合わせない` → `isNewer("0.5.0","0.4.0")` が偽であること（:98-101）/
  `入れ替えた携帯は、チャンネル update だけを切ってあっても示したとみなさない` → チャンネルの id が `update` であること（:135-140。`IMPORTANCE_NONE` を使う箇所は 0）/
  `入れ替えた携帯は、画面に切り方を出した後の機会に確かめる` → `due(now, now-25h)`（:141-144）/
  `携帯は新しい版ごとに 1 回だけ通知する` → `isNewer` の 2 つの比べ（:187-190）/
  `携帯は 7 日確かめられなければ画面に出し、1 回通知する` → `stale(now, now-8d)`（:197-200）/
  `携帯は携帯の回線でも確かめる` → `due(now, null)`（:201-204。`ShadowConnectivityManager` は 0）/
  `入れ替えた携帯は、同じ回に別の画面を前に出したときは…` → `isFinishing` が偽（:175-178。`getNextStartedActivity` は 0）/
  `入れ替えた携帯は、設定の画面を開くと示したとみなす` → スイッチの位置だけで、印を見ていない（:116-134）/
  `入れ替えた携帯は、通知を後から許すと次の機会に…` → 許可を変えずに画面の行を見る（:111-115）/
  `入れ替えた携帯は、ランチャー以外から開かれて…` → `isFinishing` だけで、Task 9.1 が求めた「印が立つ」の assert が無い（:150-153）/
  `入れ替えた携帯は、確認の窓を「設定を開く」で閉じた回は…` → 押した直後に印が無いことだけで、戻った後に立つことを見ていない（:158-162）
- kind: technical
- 提案: Scenario ごとに GIVEN を作り（`IMPORTANCE_NONE` のチャンネル・`ShadowConnectivityManager` で回線だけ・loopback の受け手・`getNextStartedActivity`）、THEN の観測（受け手の件数・サーバの宛先の件数・`ShadowNotificationManager` の通知の数・印）を assert する。印の名前を spec に合わせる（`St57UpdateCheckTest.kt:79`）
- 処置: fixed 9.1
- 判定: NOT ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）: St57UpdateNoticeTest.kt:98-101・183-190・197-204 は isNewer/stale/due を直に呼ぶだけで、元の弱い観測のまま。IMPORTANCE_NONE・ShadowConnectivityManager・getNextStartedActivity などの grep も 0 件。diff はこのファイルに触れていない
- 判定: NOT ADDRESSED（re-review 2 @b998446f7a036724e0b53db8ea19d5ec388fb66c）: St57UpdateNoticeTest.kt の直った 3 本（IMPORTANCE_NONE のチャンネル・スイッチ・通知 1 件）以外は未処置。:131 isNewer、:251 stale、:255 due を直に呼ぶだけで、ShadowConnectivityManager・getNextStartedActivity・BUTTON_NEGATIVE は 0 件。LocationService の updateChecker.check() 呼び出しと宛先（baseUrl に壊す (d)）、(f)(g) を固定する試験が無い。
- 判定: ADDRESSED（re-review 3 @589b84e3bb0a57fe180ef8905c3961e32a979013）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）
- 判定: ADDRESSED（re-review 6 @b42d98b49e7be0bb189627b42c31e2e87f6fffef）

## R4. PC の収集アプリの「新しい版ごとに 1 回」・setup の既定・起動時と 1 時間ごとの呼び出しは、どの試験も固定していない
- 成果物: crates/collector-windows/tests/st57_pc_update.rs / crates/collector-windows/src/update_check.rs / crates/collector-windows/src/setup.rs / crates/collector-windows/src/runtime.rs
- 根拠: 一時的に同時に壊した: `update_check.rs:255` の `notified_version` の条件を外す / `setup.rs` の空の行の既定を「確かめない」・`y` だけ有効にする / `runtime.rs:416` の `set_update_checker` を何もしない形にする（起動時と 1 時間ごとの確認が両方とも消える）。
  → `cargo test -p ashiato-collector-windows` は rc=0（`430 passed`・`25 passed` ほか、失敗 0）。
  `// Scenario: PC は新しい版ごとに 1 回だけ通知する`（`st57_pc_update.rs:384-397`）は 1 回確かめて札 `update` が出たことだけを見る（spec の THEN は「2 回続けた後に 0.5.1」で 2 回）。
  `// Scenario: PC の setup は確かめるかを聞き、既定は確かめる`（:344-364）は setup を呼ばない（標準入力の差し替えは 0。Task 8.1 は「標準入力を差し替える」と書く）。
  `// Scenario: PC で update-check off にすると問い合わせが無くなる`（:336-342）は `enabled()` が偽になることだけで、問い合わせの件数を見ていない。
  `runtime.rs` の 1 時間ごとの呼び出し（:511-534）を通す試験は Linux に無い（`tests/runtime_windows.rs` はこの席では走らせられない）
- kind: technical
- 提案: 偽の出し手で「0.5.0 を 2 回 → 0.5.1」で `update` が 2 回になることを見る。`run_setup_flow` に空の行と `n` を流して `settings.json` を見る。`Runtime` に偽の UpdateChecker（または一時の置き場と受け手）を差し、`tick_at` を 1 時間進めて確かめが 1 回出ることを見る
- 処置: fixed 8.1
- 判定: NOT ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）: st57_pc_update.rs:358-377 の setup 試験は run_setup_flow も標準入力も使わず start を呼ぶだけ。版ごとに 1 回は 2 回続けて 0.5.1 を見ていない。runtime の set_update_checker を通す試験は無い
- 判定: ADDRESSED（re-review 2 @b998446f7a036724e0b53db8ea19d5ec388fb66c）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R5. 9.2 の件数の検査は前の実行の写しを数え続け、St57UpdateQueryTest が 0 件の実行でも rc=0 になる
- 成果物: tools/android-emulator.sh
- 根拠: いまの検査が数えるファイルは `connected/st57-update/TEST-St57UpdateQueryTest.xml` だけで、中の `timestamp="2026-11-12T08:47:41"`（2 件）。`connected/debug/` の最新の XML（10:44 UTC）は 0 件。
  `keep_stage`（`android-emulator.sh:96-100`）は `find "$OUT/androidTest-results" -name 'TEST-*.xml'` で `connected/st57-update/` の古い写しも段へ写す。末尾の loop（`:159-165`）は段の中の最初の該当ファイルを写し直すので、古い写しがそのまま残る。
  一時の置き場（`$TMPDIR/st57-emu.*`）に、古い写しと St57 を含まない新しい結果だけを置き、`keep_stage` と末尾の loop をそのまま走らせた → 検証コマンドは `rc=0`（今回の St57UpdateQueryTest は 0 件）。
  最新の段の XML（`androidTest-results-stage1/TEST-ashiato-api35(AVD) - 15-_app-.xml`、10:41 UTC）には St57UpdateQueryTest の 2 本があり、失敗は 0。いまの試験が落ちているのではなく、検査がそれを見ていない
- kind: technical
- 提案: 写しを作る前に `connected/st57-update/` を消し、今回の実行の XML（`connected/debug/` か今回の段）だけから数える。または写しを止めて、件数の検査を最新の段の XML に向ける
- 処置: fixed 9.2
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）

## R6. 携帯の 7 日とサーバの「まだ確かめていない」の 7 日は、値を変えても試験が通る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateCheck.kt / crates/server/src/update_status.rs / collector-android/app/src/test/kotlin/dev/ashiato/collector/St57UpdateCheckTest.kt / crates/server/src/st57_tests.rs
- 根拠: `UpdateCheck.kt:39` の `7` を `8` にした → gradle `*St57*` は rc=0（29 本、失敗 0）。材料の `timing` は 6 日（偽）と 8 日（真）なので、7〜8 日の間の値は通る。
  `update_status.rs:177` の `days(7)` を `days(3)` にした → `cargo test -p ashiato-server st57_` は `17 passed`、e2e `st57-update-notice.spec.ts` は 14 passed（`target/release/ashiato-server` は 20:51:50 に組み直した）。サーバの試験は 0 日・2 日（pending）と 7 日（stale）、e2e は 2 日しか見ていない。
  Q9（本人の決定「7 日」）のうち、PC と共有の crate は固定されている（手 4）
- kind: technical
- 提案: 材料の `timing` に「7 日ちょうど（真）」と「7 日 − 1 秒（偽）」を足す（Rust と Kotlin の両方が通す）。サーバの `since` の 7 日も、7 日ちょうどと 7 日 − 1 秒で見る
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）
- 判定: ADDRESSED（re-review 6 @b42d98b49e7be0bb189627b42c31e2e87f6fffef）

## R7. Task 10.2 の `cargo test --workspace` は 3 回のうち 2 回 rc=101。1 回目は更新係の時間切れの試験の理由が `Unreachable` だった
- 成果物: crates/updater/tests/st57_once.rs
- 根拠: tasks の 10.2 の 2 つ目のコマンドの 1 回目（この席はほかに何も走らせていない）→ `[quiet-run] all: rc=101`、`st57_timeout_reason_is_recorded ... FAILED`（`st57_once.rs:304`。`left: Some(Unreachable) right: Some(Timeout)`。その前の経過の assert（29 秒以上 40 秒未満）は通った）。
  2 回目（`all2`。この席の gradle と並べて走らせた）は、ST57 と関係の無い `drops_tests::drops_api_idempotent`（PostgreSQL の `40P01 deadlock detected`）で server の lib の試験が落ちて rc=101。cargo がそこで止まり、更新係の試験までは走らなかった。
  3 回目（`cargo test --workspace --no-fail-fast`、ほかに何も走らせていない）は rc=0。単独の `cargo test -p ashiato-updater st57_` は 3 回とも緑。
  原因は推定: 受け手は要求を読んでから 31 秒眠って閉じ（`st57_once.rs:291-294`）、更新係の全体の時間切れは 30 秒（`crates/updater/src/lib.rs:151`）で、差は 1 秒しか無い。受け手が先に閉じると、接続が切れて `Unreachable` になりうる。再現の条件は確かめていない
- kind: technical
- 提案: 受け手は閉じずに、試験が終わるまで（30 秒より十分長く）接続を握ったままにし、時間切れの理由が `Timeout` 以外になる余地をなくす（推定の原因に対する直し。直した後に全体の試験を何回か走らせて確かめる）
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）

## R8. 配る形の更新係の実行ファイルには、`check-artifacts.sh` が探す宛先の文字列がひと続きで入らない
- 成果物: tools/release/check-artifacts.sh / crates/updater/src/lib.rs
- 根拠: `ASHIATO_RELEASE=1 cargo build --release -p ashiato-updater --target x86_64-pc-windows-gnu`（一時の `CARGO_TARGET_DIR`）→ `strings ashiato-updater.exe | grep -cF 'https://api.github.com/repos/rai-256/ashiato/releases?per_page=20'` は `0`（rc=1）。実行ファイルの中では `…-updater-devhttps://api.github.comcrates/updater/src/lib.rs` と `/repos/rai-256/ashiato/releases?per_page=20/repos/…` が別々の所にある（`lib.rs:20` の `GITHUB_API` と update-check の `RELEASES_PATH` を実行時に繋いでいる）。
  `check-artifacts.sh:227` はこの文字列が無いと `die 'Windows の更新係に GitHub releases の宛先が無い'`。self-test（`:398`）は偽の実行ファイルにこの文字列を書き足して通している。
  PR の CI の `package-artifacts`（`ci.yml:559`）と release.yml の `check-artifacts.sh release/all` は、本物の Windows の zip でこの検査を走らせる。PR はまだ無く、CI の Windows（MSVC）の組み立てでの結果は確かめていない。この席の組み立ては windows-gnu である
- kind: technical
- 提案: 更新係も PC の収集アプリ（`update_check.rs:13` の `RELEASES_URL`）と同じく、全体の URL を 1 つの定数で持つ。self-test の偽の実行ファイルは、本物の組み立てと同じ形（2 つの断片）からも作り、それが落ちることを見る
- 処置: fixed 6.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R9. `[x]` の Task の本文にあって、実体の無いもの
- 成果物: openspec/changes/st57-update-notice/premises.md / collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/St57UpdateQueryTest.kt
- 根拠: 9.2「届いた見出しの名前と値（機種名は伏せる）を premises.md の P10 に確かさ 4 で書き直す」→ `premises.md:20` の P10 はいまも「知識 … | 知識 1」のまま（最後に変えた commit は `e1a02996c`。深掘りの第 1 回）。
  9.2「見出しの名前が 5 つの内側・本文 0 バイト・`Build.MODEL`・`Build.VERSION.RELEASE`・`BuildConfig.VERSION_NAME` がどの値にも無い」→ `St57UpdateQueryTest.kt:37-42` が見るのは 3 つの見出しがあることと `Authorization:`・`Cookie:` が無いことだけ（`grep -c 'Build.MODEL'` は 0）。
  9.1 が名指しした道具: `grep -c` で `IMPORTANCE_NONE` 0・`ShadowConnectivityManager` 0・`getNextStartedActivity` 0・`BUTTON_NEGATIVE` 0・`areNotificationsEnabled` 0（St57*.kt の 2 ファイル）。
  8.1「`PC の setup は確かめるかを聞き、既定は確かめる`（標準入力を差し替える）」→ `st57_pc_update.rs` の `stdin`・`Cursor`・`run_setup_flow` は 0
- kind: technical
- 提案: P10 を、9.2 の計測テストで受けた見出しから書き直す（機種名は伏せる）。St57UpdateQueryTest に見出しの名前の集合と値の assert を足す。9.1・8.1 の残りは R3・R4 の直しで埋める
- 処置: fixed 9.2
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）

## R10. 時計が進んだ状態で入れてから戻すと、更新係が一度も結果を書かない間、サーバは「確かめられていない」を出さない
- 成果物: crates/server/src/update_status.rs
- 根拠: `update_status.rs:174-182` は `status.json` が無いとき、`now - since >= 7 日` のときだけ `stale` にし、`since` が未来なら `pending`。
  一時の試験（`classify(Some(settings(true, now()+N日)), None, …)`。走らせて消した）→ `since=now+1d / +30d / +365d` のどれも `state=pending reason=None`、`since=now-30d` は `state=stale`。
  `status.json` があるときは `ashiato_update_check::stale` が未来の時刻を「過ぎた」とみなす（C5「最後に確かめた時刻が今より未来なら、期限が来たとみなす…Q9 の 7 日も同じ規則で数える」）ので、2 つの道で規則が違う。
  インストールの時刻（`since`）は、インストーラが走ったときの時計で書く（`crates/installer/src/main.rs:218`）。時計が 1 年進んだまま入れ、直した後に更新係が起きない（サービスが止まっている・落ち続ける）と、帯は 1 年近く出ない
- kind: technical
- 提案: `since` が未来のときも、共有の `stale` と同じく「過ぎた」とみなす（`stale(now, settings.since)` を使う）。その場合の試験を足す
- 処置: fixed 3.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R11. PC と携帯の「切ってあれば問い合わせない」を消しても、試験は全部緑（入れ替えの門が代わりに止めている）
- 成果物: crates/collector-windows/tests/st57_pc_update.rs / collector-android/app/src/test/kotlin/dev/ashiato/collector/St57UpdateNoticeTest.kt / crates/collector-windows/src/update_check.rs / collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateCheck.kt
- 根拠: 一時的に `update_check.rs:142-144`（`start`）と `:165-167`（`periodic`）の `if !self.enabled()? { return Ok(None); }` を消し、`UpdateCheck.kt:114` の `if (!prefs.getBoolean("enabled", true)) return false` を消した → `cargo test -p ashiato-collector-windows st57_pc_` は `25 passed; 0 failed`、gradle `*St57*` は rc=0（29 本、失敗 0）。
  切った試験はどれも「示した印」を立てていない: PC の `st57_pc_disabled_does_not_stale`（`st57_pc_update.rs:521-528`）は空の置き場で `set_enabled(false)` するので `update-check.json` が無く `intro_shown` が偽、`start` は入れ替えの門（`update_check.rs:147-154`）で止まる。携帯の `スイッチを切ると確認しない`（`St57UpdateNoticeTest.kt:183-186`）も印を立てずに `check` を呼ぶので、`UpdateCheck.kt:115-118` の門で止まる。`PC で update-check off にすると問い合わせが無くなる`（`:336-342`）は `enabled()` を読むだけ。
  spec の `PC で update-check off にすると問い合わせが無くなる`・`携帯でスイッチを切ると問い合わせが無くなる` の THEN（受け手への問い合わせが 0 件）は、示した後の端末では誰も見ていない。ここが壊れると、本人が切った後も配布元へ問い合わせが出る（元の IP と時刻が外に出る）
- kind: technical
- 提案: 示した印を立て、送り先を設定し、確かめる時期を過ぎた端末で切り、loopback の受け手への問い合わせが 0 件であることを PC（`start` と `periodic` の両方）と携帯で assert する
- 処置: fixed 8.1
- 判定: NOT ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）: PC の st57_pc_update.rs:535-541 は空の置き場で入れ替えの門が止める。携帯の St57UpdateNoticeTest.kt:183-186 は印を立てない。受け手の件数が 0 になることを見る試験は無い
- 判定: ADDRESSED（re-review 2 @b998446f7a036724e0b53db8ea19d5ec388fb66c）
- 判定: ADDRESSED（re-review 3 @589b84e3bb0a57fe180ef8905c3961e32a979013）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）
- 判定: ADDRESSED（re-review 6 @b42d98b49e7be0bb189627b42c31e2e87f6fffef）

## security review（8f10733..ef3a33a）

見た経路:
- 画面 `GET /update-status`（と `/api/update-status`）→ `api_router` の `route_layer(access_log::middleware)` → 無資格 401・API の合言葉 403（`collector_allows` に無い）・ログインで `update_status::get`（4。下の台本）
- 更新係が書く `status.json`・`settings\check.json` → サーバの `read_bounded`（64 KB）→ `parse_status`（SemVer・RFC 3339）→ 外れたら `unreadable`。画面へは版と時刻だけ、React で文字として出す（`dangerouslySetInnerHTML` は 0 件）（3）
- 更新係・PC の収集アプリ・携帯の問い合わせ → 見出しは定数 3 つ、本文なし、自分の版・端末の識別子・送り先の URL は載せない（`crates/updater/src/lib.rs:158-163`・`update_check.rs:353-358`・`UpdateCheck.kt:127-132`）。証明書は OS の根で確かめる（`ureq` の `platform-verifier`。検証を切る設定は無い）（2）
- 試験の差し込み（`ASHIATO_UPDATER_TEST_*`・`settings\test-hooks.json`・`ASHIATO_COLLECTOR_TEST_UPDATE_URL`・`ASHIATO_SETUP_TEST_UPDATER_URL`）→ どれも `cfg(ashiato_test_build)` の内側、配る組み立て（`ASHIATO_RELEASE=1`）では宛先は定数（`lib.rs:84-112`・`update_check.rs:316-326`・`main.rs:17-28`）（3）
- 携帯の `update_check.xml` → cloud-backup・device-transfer・full-backup の 3 つで除外、記録は `noBackupFilesDir`（2）
- インストーラ → `C:\ProgramData\ashiato-updater` を作り ACL を付ける（初回 `main.rs:1100-1149`・走らせ直し `:1275-1294`）→ R12
- 依存: 足した crate は `rustls-platform-verifier` とその OS 別の部品・`semver`。`webpki-root-certs` は lock に入ったが windows-msvc / linux-gnu の木には無い（`cargo tree --target … -i webpki-root-certs` は空、`tools/check-updater-deps.sh` は緑）（4）

安全の決め手: `/update-status` は `access_log::middleware` の `route_layer` の内側にあり、通り抜けの 3 つ（`/healthz`・`/session`・`/collector-setup`）にも `collector_allows` にも入っていない —— 確かさ 4。`scripts/quiet-run st57sec -- cargo test -p ashiato-server --lib st57_update_status` → `st57_update_status_requires_login_and_rejects_api_token ... ok`（資格なし 401・`Bearer` の合言葉 403）・`st57_update_status_is_not_cached ... ok`、`2 passed; 0 failed`。

## R12. 更新係の置き場は持ち主を Administrators に取り直さないので、先に置き場を作った一般の利用者が「確かめた結果」と「切った印」を書き換えられる
- 成果物: crates/installer/src/main.rs / crates/installer/src/commands.rs
- 根拠: 状態の置き場は「先に誰かが作っていた置き場でも、持ち主を Administrators にしてから ACL を付ける（持ち主は ACL を書き換えられるので、ほかの利用者を持ち主のままにしない）」（`main.rs:1129-1131`）を守るが、`take_ownership_command` は `layout.state_dir` だけを見る（`commands.rs:612-614`）。
  更新係の置き場は初回も走らせ直しも `fs::create_dir_all`（既にあれば何もしない）→ `updater_acl_commands`（`/inheritance:r /grant:r` だけ。`/setowner` は無い。`commands.rs:526-551`）で、持ち主は作った人のまま残る。走らせ直し（ST57 より前の版からの入れ替え。`main.rs:1275-1294`）は `ensure_real_directory` も通らない。
  そのため、インストールより前に `C:\ProgramData\ashiato-updater` を作れた一般の利用者（本人の権限で動く悪意のあるプログラムを含む）は、持ち主の暗黙の DACL の書き換えで自分に書きを足し、`status.json`（偽の「新しい版 X が出ています」）や `settings` の差し替え（`enabled: false` で更新の確認を黙って止める）ができる。D6「`settings\check.json` は管理者と SYSTEM だけが書く」が成り立たない。
  コードの経路は 1 歩ずつ辿った（3）。`ProgramData` に一般の利用者が子のフォルダを作れることと、持ち主が DACL を書き換えられることは Windows の既定の知識（1。既存のコメント `main.rs:1129` が同じ前提に立つ）。Windows の上では走らせていない
- kind: technical
- 提案: 更新係の置き場にも `/setowner *S-1-5-32-544 /T /C` を ACL の前にかけ（初回・走らせ直しの両方）、走らせ直しでも 3 つのフォルダに `ensure_real_directory` を通す。`take_ownership_command` の試験に更新係の置き場を足す
- 処置: fixed 5.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）

## /code-review（8f10733..6276a48）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R13. The updater's build.rs switches `ashiato_test_build` on `ASHIATO_RELEASE` but never emits `cargo:rerun-if-env-changed=ASHIATO_RELEASE`. The server, installer and collector-windows build.rs all emit it. `emit_rerun` already emits rerun-if-changed lines, which turn off Cargo's default rerun, so changing only ASHIATO_RELEASE does not rerun the script.
- 成果物: crates/updater/build.rs:5
- 根拠: A test build of ashiato-updater is made first (no ASHIATO_RELEASE), then `tools/release/build.sh` builds it with ASHIATO_RELEASE=1 in the same target dir and the sources are unchanged. build.rs does not rerun, so the shipped updater is still a test build. `now_and_url` then returns url=None and every check is NoDestination. The ASHIATO_UPDATER_TEST_URL and ASHIATO_UPDATER_TEST_NOW env overrides also stay in the shipped binary.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）

## R14. `choose()` swallows JSON parse and shape errors (runCatching → empty list). `check()` then treats any non-null Choice as a successful check, so an unreadable body counts as ok.
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateCheck.kt:56
- 根拠: A captive portal, proxy, or cross-protocol redirect that HttpURLConnection does not follow returns 200 with an HTML or non-array body. `choose` returns Choice(null,false), `record(ok=true)` sets last_ok_at and clears stale_notified, and `putString("latest", null)` erases an update found earlier. The 7-day stale warning never fires and the 'update available' row disappears.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）
- 判定: ADDRESSED（re-review 6 @b42d98b49e7be0bb189627b42c31e2e87f6fffef）

## R15. `once_at` overwrites `status.found_at = Some(at)` on every successful check, even when `latest` has not changed. The PC collector only updates it when the version changes (`if state.latest != latest`).
- 成果物: crates/updater/src/lib.rs:270
- 根拠: Version 0.5.0 is found on 11/01 and checked daily. found_at moves forward each day, so the web band and settings show '更新あり 0.5.0（11/12 に見つけた）' instead of 11/01. found_at is also set when latest is None.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R16. A status.json that fails to parse makes `once_at` return Err before anything is written. The updater never repairs or replaces the file, so it stops checking for good.
- 成果物: crates/updater/src/lib.rs:228
- 根拠: The service is killed during the non-atomic `fs::write` in `save_status` (or the disk is full) and status.json is truncated. Every hourly `once()` then returns Error::Invalid and only logs it. The server keeps showing 'unreadable' and no update or stale notice ever appears until someone deletes the file by hand.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R17. `save_status` uses plain `fs::write`, which truncates and then writes, while the server reads status.json at any time. The collector uses `fsutil::atomic_write` for the same kind of file.
- 成果物: crates/updater/src/lib.rs:129
- 根拠: GET /update-status runs while the updater is in the middle of writing. `parse_status` sees an empty or partial file and the settings page and band report 'unreadable' for that request. A crash at that point leaves the file corrupt, which leads to the permanent failure in the previous finding.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R18. In `check()`, a toast failure from `self.show(...)?` returns early after the JSONL record was appended but before `self.save(&state)`, so checked_at, latest and last_ok_at are lost.
- 成果物: crates/collector-windows/src/update_check.rs:262
- 根拠: WinRT toast fails (notifications service off or an AUMID problem) while an update is available. Each hourly `periodic` sees checked_at unchanged, so `due` is true and it calls GitHub again, appends another record, and fails again. It sends 24 requests a day instead of 1 and the state file never records the result.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 8.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R19. `update_check_service_commands(enabled=true)` always appends a bare `sc start`, which fails with 1056 when the service is already running. main.rs already has an `ensure_running` helper for this case.
- 成果物: crates/installer/src/commands.rs:603
- 根拠: A user who already has the update check on runs `ashiato-setup.exe update-check on`. `sc config` succeeds, `sc start ashiato-updater` exits non-zero (1056, already running), and `update_check` reports failure for a correct state.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 5.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）

## R20. Redirect validation compares only `host()` and not the scheme. The server updater checks both scheme and authority.
- 成果物: crates/collector-windows/src/update_check.rs:382
- 根拠: The release query gets a 3xx with `Location: http://api.github.com/...`. The host matches, so the collector follows it over plaintext HTTP, and a network attacker can feed a fake release list. That list causes false update toasts pointing users to a version chosen by the attacker.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 8.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R21. `reasons` has no `no_destination` entry, although the server's `reason_name` can return it.
- 成果物: web/src/messages.ts:109
- 根拠: The server reports reason `no_destination`. The settings page shows '理由 —', and the band's stale line shows '更新係が動いていない', which wrongly says the updater is not running.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 4.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）

## R22. `write_update_check` and `rewrite_update_check` are near-identical copies (create the parent dir, build {enabled, since}, write). `chrono_like_now`/`civil_from_days` hand-roll RFC 3339 formatting that chrono already provides elsewhere in the workspace.
- 成果物: crates/installer/src/main.rs:210
- 根拠: Two writers and a custom calendar routine must be kept in sync. A fix to one, such as an atomic write or a change to the `since` format, will be missed in the other. The settings write is also not atomic, unlike the existing `write_settings` helper a few lines above.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 5.1
- 判定: ADDRESSED（re-review 1 @b3e0dd71e50befa89c12e36be771cf20cc3cbc81）

## settle_review（re-review 3 @589b84e）: 処置の直しが新しく壊したもの

## R23. updateStale を組むのに getSharedPreferences(UpdateCheck.PREFS) を 3 回呼んでいる。1 つの変数にまとめられる。振る舞いは壊れない。
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt
- 根拠: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:98-109（re-review 3 の New Breakage。Minor）
- kind: technical
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 4 @ae80523fdea6fb18e5ff5684ffe73e3f9441f501）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R24. start() のたびに exemptSettingsOpened = false にするので、「設定を開く」を押した回の保護は start() が再び呼ばれた回で外れる。戻った後に印が立つ試験はあり、実害は見つからなかった。
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt
- 根拠: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:82-83（re-review 3 の New Breakage。Minor）
- kind: technical
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 4 @ae80523fdea6fb18e5ff5684ffe73e3f9441f501）
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## final review（8f10733..379effb）

見た: SDD の final reviewer（`requesting-code-review/code-reviewer.md`）。review package `.superpowers/sdd/tasks/review-8f1073321..379effb58.diff`、Task 1〜10 の findings の Minor と ⚠️、`docs/handoff/ST57.md`。R25〜R30 と R32 の根拠の行は、この席で読んで確かめた（試験は走らせていない）。
申し送り「FR-112 の改訂の整合の確認（`docs/audit/2026-11-11-st57-fr112-consistency.md`）」の C2 と K1 は処置済み（spec.md:679・737 の Scenario、`UpdateCheck.kt:182-183` がチャンネル `update` の重要度を見る）。K2 は design の Risks（design.md:251）に残し、本物の Windows で確かめてから上流へ返す。指摘は無い。

## R25. 携帯の「確かめられていない」の通知と行に、最後に確かめた日と理由が無い
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateCheck.kt
- 根拠: `UpdateCheck.kt:149`（`post("更新を 7 日確かめられていません")`）と `MainActivity.kt:534`（行の本文だけ）。spec.md:679 と design D20 は「更新を 7 日確かめられていません（最後に確かめた <M>/<D>・<理由>）」と定め、確かめられたことが無ければ「—」とする。PC（`crates/collector-windows/src/update_check.rs:272-277`）は日と理由を付ける。`St57UpdateNoticeTest.kt:304-305` は `contains("7 日確かめられていません")` だけを見るので緑。直し方の案: 文を組む関数を 1 つにして通知と行で共有し、試験で全文を比べる
- kind: technical
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）
- 判定: ADDRESSED（re-review 6 @b42d98b49e7be0bb189627b42c31e2e87f6fffef）

## R26. 携帯の「更新あり」の日付が UTC の `YYYY/MM/DD` になる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt
- 根拠: `MainActivity.kt:527`（`found_at.take(10).replace('-', '/')`）。ISO の UTC 時刻の先頭 10 文字なので「更新あり 0.5.0（2026/11/10 に見つけた）」になり、端末の日付と 1 日ずれることもある。spec.md:641 の Scenario と D20 は「11/10 に見つけた」。`St57UpdateNoticeTest.kt:127` は `startsWith("更新あり 0.5.0")` だけを見る
- kind: technical
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R27. 携帯で確認を切っても「確かめられていない」の行が出続ける
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt
- 根拠: `MainActivity.kt:428-432`。`updateStale` の条件は `last_reason != null && stale(...)` だけで、確認が有効かを見ていない。失敗した後にスイッチを切ると `check()` は早く返って記録を更新しないので、行が残り続ける。7 日の Requirement（spec.md:679「WHILE その部品の更新の確認が有効である」）に反する。切ったときの Scenario（spec.md:713）の試験は PC にしか無い
- kind: technical
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R28. 携帯は失敗を全部 `unreachable` として記録する
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateCheck.kt
- 根拠: `UpdateCheck.kt:137`（`if (choice == null) "unreachable"`）。`responseCode` を読まないので、403・429・404・5xx・読めない本文・時間切れのどれも `unreachable` になる。spec.md:406 は各部品に 6 つの理由を求める。R25 で理由を画面と通知に出すと、誤った理由を見せる。直し方の案: `responseCode` で分け、`SocketTimeoutException` を timeout、`choose` の例外を unreadable にする
- kind: technical
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）
- 判定: ADDRESSED（re-review 6 @b42d98b49e7be0bb189627b42c31e2e87f6fffef）

## R29. 携帯は別のホストへの転送を黙って追う
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateCheck.kt
- 根拠: `UpdateCheck.kt:126-132`。`HttpURLConnection` の `instanceFollowRedirects` を既定（true）のままにしているので、同じ scheme なら `api.github.com` 以外のホストへの 3xx も追う。spec.md:406 は別の宛先への転送を「読めない答え」（確かめられない）として扱うと定める。R20 は PC だけの指摘で、携帯のこの点は前の席に無い。直し方の案: `instanceFollowRedirects = false` にし、更新係（`crates/updater/src/lib.rs` の `request()`）と同じく、手で 3 回まで scheme と authority を比べて追う
- kind: technical
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）
- 判定: ADDRESSED（re-review 6 @b42d98b49e7be0bb189627b42c31e2e87f6fffef）

## R30. 試験の zip で Windows の実地の台本を走らせると、試験の時計を書くときに更新係の宛先を消す
- 成果物: tools/release/update-check-install.sh
- 根拠: `tools/release/update-check-install.sh:97-101`（`write_test_clock` が `settings/test-hooks.json` を `{"now":…}` だけで上書きする）と `crates/installer/src/main.rs:1361-1365`（インストーラは同じファイルに `{"url":…}` を書く）。更新係はサービスなので環境変数を受け継がず、宛先はこのファイルからしか取れない。上書きした後は `NoDestination` になって受け手への問い合わせが増えず、`before + 1` の比較（:205 付近）で die する見込み。「切ると問い合わせが無くなる」の確かめは、宛先が無いので空振りでも通る。証拠は `--self-test` だけで、本物の Windows では走っていない。直し方の案: 既存の JSON を読み、`now` を足して書く
- kind: technical
- 処置: fixed 7.1
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R31. 申し送りの C4・C5・C6・K1 が拾われていない（帯のコントラスト・通知の見分け・未来形・携帯の通知 ID）
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md
- 根拠: 申し送り「FR-113 の改訂の整合の確認（`docs/audit/2026-11-11-st57-fr113-consistency.md`）」（`docs/handoff/ST57.md`）。どれも tasks.md と design に入っていない。
  - C4: spec.md:488・517 と e2e の閾値（`web/e2e/st57-update-notice.spec.ts:48` の `toBeGreaterThanOrEqual(3)`）は 3:1 のまま。NFR-18 は本文に 4.5:1 を求める。実際の比は 12.4 と 8.0 なので、ずれているのは spec と試験の閾値だけ
  - C5: 携帯の 3 種の通知が既存の「サーバの更新」の知らせを置き換えず、文が違うことを示す Scenario が無い（PC は `crates/collector-windows/tests/st57_pc_update.rs:437` にある）。既存のチャンネル `contract` の表示名は「サーバの更新」（`ContractNotifier.kt:91,155`）、新しい `update` は「更新」で、OS の設定の画面で紛らわしく並ぶ
  - C6: spec.md:638・686、proposal.md:29、design D22 に「推奨のままなら FR-113 を ★ で改訂する」の未来形が残る。Q9 の Requirement（spec.md:686）の導出元に FR-113 が無い
  - K1: `SETUP_NOTIFICATION_ID = 6`（`LocationService.kt:1474`）と `CHANNEL_NOTIFICATION_ID = 6`（`ContractNotifier.kt:104`）が重なったまま。新しい `UpdateCheck.kt:178` は `notify(text.hashCode(), …)` で ID を文の hash にしているので、既存の ID とぶつかりうる。ID を覚えていないので取り消せず、確かめられた後も「7 日確かめられていません」が残り、0.5.1 を見つけても 0.5.0 の通知が残る（PC は札で置き換えている）
- kind: conflict
- 処置: fixed D30 仮
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R32. PC の収集アプリは、壊れた `update-check.json` で確認が以後ずっと止まる（R16 の処置が PC に届いていない）
- 成果物: crates/collector-windows/src/update_check.rs
- 根拠: `crates/collector-windows/src/update_check.rs:96-104`（`state()` は読めない JSON で `Err` を返し、`start` と `periodic` が毎回失敗する）。更新係は R16 で `unwrap_or_default()` に直した
- kind: technical
- 処置: fixed 8.1
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R33. 更新係の置き場とサービス名の正本が 3 か所にある
- 成果物: crates/updater/src/lib.rs
- 根拠: 申し送り「ST55 D14 — 更新係が読む配置の正本」（`docs/handoff/ST57.md`）。`crates/installer/src/layout.rs:19-36`、`crates/updater/src/lib.rs:309-334`、`crates/server/src/service.rs:329-330`（`UPDATER_RELEASE_DIR` / `UPDATER_DEVELOPMENT_DIR`）が同じ文字列を別々に持ち、開発版の判定もそれぞれにある。直し方の案: 依存の向き（更新係とサーバはインストーラに依らない）を保ったまま、共有の crate（例: `update-check` か小さな layout の crate）に 1 か所で置く
- kind: technical
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）

## R34. Task の Minor の残り（PC の日付・operation_id・携帯の JSON Lines）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateCheck.kt
- 根拠:
  - Task 9 F12: `UpdateCheck.kt:166-168` は JSON Lines を文字列の連結で組む。`latest` の pre は `[^+]+` なので `"` を含みうり、その行が壊れる
  - Task 8 F13: PC の日付は UTC の `%m/%d`（`crates/collector-windows/src/update_check.rs:274`）で、画面は DAY_TZ を使う
  - Task 3 F9: `crates/server/src/update_status.rs:121` の `#[utoipa::path]` に `operation_id` が無い
  - design.md:186 の「チャンネルを引数に取る 1 か所の関数」に反し、`UpdateCheck.canPost` は `SetupNotifier` の判断と共有せず別に書いている
- kind: technical
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 5 @c82b3eb14946a25e5b56f2606acfd57308a423f5）
- 判定: ADDRESSED（re-review 6 @b42d98b49e7be0bb189627b42c31e2e87f6fffef）

## settle_review（re-review 5 @c82b3eb）: 処置の直しが新しく壊したもの

## R35. stale の行の試験が期待値を UpdateCheck.staleText(...) で組むので、文言や日付の形を変えても通る。
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/St57UpdateNoticeTest.kt
- 根拠: collector-android/app/src/test/kotlin/dev/ashiato/collector/St57UpdateNoticeTest.kt:305（re-review 5 の New Breakage。Minor）
- kind: technical
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 6 @b42d98b49e7be0bb189627b42c31e2e87f6fffef）

## R36. R29（別ホストへの転送で確かめられない）と R26（見つけた日の表示）に当たる試験が無い。コードは直っているが、固定していない。
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/St57UpdateNoticeTest.kt
- 根拠: collector-android/app/src/test/kotlin/dev/ashiato/collector/St57UpdateNoticeTest.kt（re-review 5 の New Breakage。Minor）
- kind: technical
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 6 @b42d98b49e7be0bb189627b42c31e2e87f6fffef）

## R37. 改行以外の制御文字（\t など）をエスケープしない。latest はバージョン文字列由来なので、実害は見つからなかった。
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateCheck.kt (record の json())
- 根拠: collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateCheck.kt (record の json())（re-review 5 の New Breakage。Minor）
- kind: technical
- 処置: fixed 9.1
- 判定: ADDRESSED（re-review 6 @b42d98b49e7be0bb189627b42c31e2e87f6fffef）
