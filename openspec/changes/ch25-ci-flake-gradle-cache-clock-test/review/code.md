<!-- 処置の判定: re-review -->
# CH25 code_verify（2026-11-21）

検証した HEAD: `0058f9074`（base `6642ee755`）。作業ツリーは検証の前後で `git status --porcelain` が空、`git diff` も空（壊した複製はすべて戻した）。

## 申告と実測

申告: 8/8（tasks.md の `[x]` は 1.1〜1.4・2.1〜2.3・3.1。4.1 は人間の確認待ちで `[ ]`、数えない）

| 項目 | 検証コマンド（要約） | 申告 | 実測 |
|---|---|---|---|
| 1.1 | 1.5 秒眠る試験を足し、直す前に `verify-run 1.2` が FAIL | FAIL を記録（evidence 1.2 exit 101） | 一致。証跡のログ `20261121-005431-evidence-1.2-…log:157` が `skew=302281 width=2`（組み立ての失敗でなく新しい試験の幅外れ）。さらに直しを戻した複製で再現: rc=101・`skew=302388 width=2`（下の手 2） |
| 1.2 | `scripts/quiet-run clk -- cargo test -p ashiato-collector-windows --lib clock::tests::clock_reference` | PASS | 一致。rc=0・`6 passed`。`-- --list` で 6 本 |
| 1.3 | `check-ch25.py clock` | PASS | 一致。rc=0 `OK: clock` |
| 1.4 | 5 本であることを確かめてから 3000 回 | PASS | 一致。rc=0 `3000 回 PASS`（33 秒） |
| 2.1 | `check-ch25.py ci` | PASS | 一致。rc=0 `OK: ci` |
| 2.2 | 既存の workflow の検査 9 本 | PASS | 一致。rc=0 |
| 2.3 | `scripts/quiet-run locks -- … check-gradle-locks.sh --self-test && …` | PASS | 一致。rc=0（72 秒。Scenario 4 本） |
| 3.1 | 予定のパスの外に触れていない | PASS | 一致。rc=0 |

## 手 1: 固定値を独立に再計算する

該当なし（確かめた範囲: 試験の幅 `300_000..301_000` / `-300_000-width..-299_000+width` は正典 `openspec/specs/desktop-collection/spec.md:478,483` の数値を写したもので、実装の出力を写した値ではない）。
あわせて tasks 1.4 の見積りを python で再計算した: 1 回あたり 22/5000 で落ちるとき、3000 回すべて通る確率は `1.797e-06`、300 回では `0.266`。tasks の「約 2×10⁻⁶」「4 回に 1 回は通る」と一致。

## 手 2: ガードをわざと壊す

壊した複製で実際に rc≠0 になるかを見た（どれも作業ツリーで一時的に壊し、戻した後に `git status` / `git diff` が壊す前と同じであることを確かめた）。

| 壊したもの | 実行 | 結果 |
|---|---|---|
| `fake_ingest` の Date を、応答の瞬間でなく用意の時点で決める形に戻す（`clock.rs` の `let date = Utc::now() + skew;` を事前に決めた `fixed` に） | 1.2 のコマンド | rc=101。`clock_reference_setup_time_is_not_in_the_difference` が `skew=302388 width=2` で落ちた |
| 同上 | 1.4 のコマンド（3000 回） | rc=1 `落ちた: 258 回目`（試験の出力は捨てる形のコマンドなので見えたのは rc と回数だけ。直す前の作りでは 3000 回の中で落ちる） |
| ci.yml `android` の `cache: gradle` を外す | `check-ch25.py ci` | rc=1 `android が gradle の部品の取り置きを持たない` |
| ci.yml `chain` の `actions/cache` の段を外す | `check-ch25.py ci` | rc=1 `chain が gradle の部品の取り置きを持たない` |
| 製品の定数 `SKEW_RETRY_SEC` を 60→61 | `check-ch25.py clock` | rc=1 `試験の外（製品のコード）が base から変わった` |
| `clock_reference_pc_ahead_gives_positive_difference` の幅を `301_000`→`302_000` | `check-ch25.py clock` | rc=1 `幅が Scenario の字面…でない` |
| 予定の外に未追跡のファイル `tools/zz-ch25-probe` を置く | 3.1 のコマンド | rc=1（ファイル名を出して落ちた） |
| ci.yml の 4 つの `cache-dependency-path`（鍵の材料）をすべて外す | `check-ch25.py ci` | **rc=0 `OK: ci`**（R1） |
| ci.yml `chain` の `key` を `gradle-chain-x-…` に変え `restore-keys: gradle-` を足す | `check-ch25.py ci` | **rc=0 `OK: ci`**（R1） |

検査の外側も探した: `tools/` で `gradlew` を含む 19 本と、それを呼ぶ workflow の job を突き合わせた。7 つ以外の job で、名前が当たったものの呼ばれ方:
- ci `rust` の `check-contract-version.sh`（引数なしと `--self-test`）: gradle は `write_copy`（`--write`）の中だけ。`sed -n '170,$p' … | grep write_copy\|gradle` は 0 件で、自己試験は呼ばない
- ci `rust` の `test-android-emulator.sh`（引数なしと `--self-test`）: `gradlew` は偽物を書き出す行（`:101` `cat > "$root/collector-android/gradlew"`）だけ
- `tmp-leak-check.sh`: ci `rust` は `rust`（cargo の道）、`android` の job は `android`（取り置きあり）、chain は `--self-test`（命令は `TMP_LEAK_CMD` の偽物。gradlew を呼ばない）
- ci `package-artifacts`・release `publish` の `check-artifacts.sh <置き場>`: gradlew は自己試験の関数（`:45` `:345` `:373` 以降）の中だけ
- ci `web` の `check-licenses.sh rust node`: android の道でない
- nightly / release の `package-windows`: `build.sh server-windows` と `collector-windows` だけで `apk` を呼ばない
- `vulns-nightly.yml` は `check-ch25.py` の対象外だが、`run:` に gradlew・build.sh が無い

どれも CI では gradle を走らせず、7 つの外に漏れた job は無い。鍵の材料の glob 3 つは実在のファイルに当たる（3 本・2 本・1 本。0 件で setup-java が落ちる形にはならない）。
chain の `actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0`（ci.yml:727）は、既存の ci.yml:548・557（package-windows）・release.yml:75 と同じ SHA と版のコメント（Global Constraints どおり）。

## 手 3: Scenario と test を突き合わせる

`python3 scripts/check_scenarios.py . ch25-ci-flake-gradle-cache-clock-test` → rc=0 `scenarios: OK`（3117 件 / 担保あり 3116 / 人間の確認待ち 1。CH25 に関わる Scenario の欠けは無い）。
この change が持つ印は既存の 2 本（`clock.rs:475` 進んでいる側、`clock.rs:485` 遅れている側）で、名前・印・幅の字面は base のまま（`check-ch25.py clock` rc=0、上の手 2 で壊すと落ちる）。
試験の差の式 `(r.wall_after - r.time).num_milliseconds()` は製品の記録の式 `clock_record.rs:36` と同じ字面。試験の幅は正典の「± 幅」に対し、進んでいる側の下限を `300_000`（幅を引かない）と狭めに取っており、弱めてはいない（base から不変）。
新しい試験 `clock_reference_setup_time_is_not_in_the_difference` は Scenario の印を持たない（Scenario を増やさない設計どおり）。指摘なし。

## 手 4: 本人の決定が test で固定されているか

該当なし（確かめた範囲: 軽い路で `deep.md` に「本人の答え」が無い。design の D1〜D3 はすべて（仮）。仮の形のうち、取り置きを持つかは `check-ch25.py ci` が固定するが、鍵の材料は固定しない → R1）。
`docs/changes/CH25.md` の完了の判定 1〜5 は、それぞれ tasks 1.1・1.2・1.4 / 1.3 / 2.1〜2.3 / 3.1 / 4.1 に対応し、1〜4 は上の表で rc=0 を確かめた。ただし判定 3 の「部品の取り置きを持つ」は依頼の「鍵にロックを入れる」（CH25.md「何を」1）まで含むのに、2.1 の検査は持つかどうかしか見ない（R1）。

## 手 5: tasks の `[x]` と実体

上の表のとおり、`[x]` の 8 項目の検証コマンドはすべて実在し、手元で rc=0（1.1 は FAIL の記録が本体で、証跡のログに新しい試験の幅外れがある）。
`cargo test` は `--lib` の名前の絞り込みで、`-- --list` で 6 本（1.2）・`--skip` 後に 5 本（1.4 のコマンド自身が `-eq 5` で確かめる）を確かめた。`--test <名前>` 型は無い。

## 手 6: 隙間

該当なし（確かめた範囲: この change は試験と CI の取り置きだけで、記録を持つ経路に触れない。製品のコードは base と 1 字も違わない（`check-ch25.py clock`）。
取り置きから部品を取ることで、ロックの検査が今と同じに効くかは、手元の `~/.gradle/caches`（`modules-2` が 1.2G。部品が既にある状態）で 2.3 が rc=0 になることで近似を見た。CI の上での復元の振る舞いは 4.1 の人間の確認待ちに残っている）。

## 手 7: 機械が承認した検証コマンドの訂正

該当なし（確かめた範囲: `python3 scripts/plan_fix.py pending openspec/changes/ch25-ci-flake-gradle-cache-clock-test` → `[]` rc=0）。

## 指摘

## R1. 2.1 の検査（`check-ch25.py ci`）は取り置きの有無しか見ず、design D1・D3 が決めた鍵の材料（ロックを含む）を外しても緑のまま
- 成果物: `openspec/changes/ch25-ci-flake-gradle-cache-clock-test/check-ch25.py` / `.github/workflows/ci.yml` / `.github/workflows/nightly.yml` / `.github/workflows/release.yml`
- 根拠: ci.yml の 4 つの `cache-dependency-path` を作業ツリーで外して `python3 …/check-ch25.py ci` → rc=0 `OK: ci`。chain の `key` を `gradle-chain-x-…` に変えて `restore-keys: gradle-` を足しても rc=0 `OK: ci`。
  `check-ch25.py` の `has_cache` は `with.cache == "gradle"` と `actions/cache` の `path` に `.gradle/caches` があることだけを見る（鍵の材料・`restore-keys` は見ない）。`without_cache` が `cache-dependency-path` を除いて base と比べるので、そこも素通りになる。
  design D1 は「ロックだけを書き直す変更（`--write-locks`）で取り置きが古いまま残らないよう、鍵を 3 つの hash にする」を取り置きの形の理由にしている（`design.md` D1）。
  今の workflow の中身は D1・D3 のとおり（目で確かめた）で、ずれているのは主張と検査の階層: 2.1 と完了の判定 3 は「D1・D3 の形（鍵にロックを入れる）」を完了の条件にしているが、
  検証コマンドが観測するのは取り置きの有無だけで、鍵の材料の正しさは機械の検証のどこにも無い（この change の残りの Task でも、これから鍵を直す fix でも、材料を落として緑になる）
- kind: technical
- 提案: `check-ch25.py ci` に、setup-java の段の `cache-dependency-path` が 3 つの glob ちょうどであること、chain の `key` が `hashFiles` に同じ 3 つを持ち `restore-keys` を持たないことを足す。足した後、上の 2 つの壊し方で rc=1 になることを確かめる
- 処置: fixed check-ch25.py

## /code-review（6642ee7..f0cb162）

見た: Claude Code の組み込みの `/code-review`（6 件）。写しは scripts/code_review_record.py

## R2. Release and nightly builds may now assemble the APK from compiled outputs another run put in the cache, not from source. `collector-android/gradle.properties` sets `org.gradle.caching=true`, so `~/.gradle/caches` (what `cache: gradle` saves) includes the build-output cache `build-cache-1`, not just downloaded dependencies. Design D2 only considered dependency downloads when it said verification is no weaker. Fix: keep `build-cache-1` out of the restored cache in package-linux (nightly, release, ci), e.g. delete `~/.gradle/caches/build-cache-1` after restore or pass `--no-build-cache` in `build.sh apk`; or don't use the cache for release builds at all.
- 成果物: .github/workflows/release.yml:47
- 根拠: A main run (or any run that can write the default-branch cache) saves `~/.gradle/caches/build-cache-1` holding compiled classes or dex for the app. A later tag push runs release.yml, `tools/release/build.sh apk` gets cache hits for those tasks, and the signed APK we ship contains bytes that were never compiled from this tag's source in this run. Nothing checks them, because the project has no dependency verification and build-cache entries are never checked.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: Gradle の build cache はタスクの入力から出力を一意にする key で復元する。タグの source が異なれば task key も異なり、同一入力の `FROM-CACHE` は再利用として正しい。D2 が扱う部品の信頼境界（`design.md:21-23`）を、根拠の無い出力混入の懸念だけで変えない。

## R3. Six jobs share one cache key (build-stamp, android, android-instrumented, package-linux, nightly, release), and setup-java saves nothing after an exact hit. So the cache only ever holds what the first job to finish downloaded. If build-stamp, which only runs the BuildConfig tasks, finishes first, the android job never gets its detekt, Robolectric or test dependencies cached. That stays true until a `.gradle.kts`, lockfile or the wrapper changes, so the download flake this change targets stays in the job that hit it. Fix: give each job its own key prefix (e.g. `actions/cache` with `gradle-<job>-...` like chain), or switch to `gradle/actions/setup-gradle`. The design's fallback (D1) only triggers after merge.
- 成果物: .github/workflows/ci.yml:224
- 根拠: On main, build-stamp finishes first and saves the cache under `setup-java-Linux-x64-gradle-<hash>`. The android job then restores that key on every run, gets an exact hit and skips saving. It keeps downloading detekt and the test dependencies from Maven Central on every run and still fails when that download fails.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: `design.md:13-16` が同一 key の最初の job だけが保存すること、その結果 job 固有の部品が残らないことを明記して受容し、merge 後の復元失敗または 4.1(2) の観測を反転条件にしている。提示された「build-stamp が先に終わる」はその既知の代償の再記述で、反転条件を満たす実測ではない。

## R4. With `org.gradle.caching=true`, the restored `~/.gradle/caches` also brings back stored test results, so Gradle can report unit-test tasks as FROM-CACHE instead of running them. The cache key only covers build scripts, lockfiles and the wrapper, and the cache is never re-saved on a hit, so entries saved once stay for good. Whether the android job really runs its tests now depends on what an earlier run saved. Fix: leave `build-cache-1` out of the cached paths, or check that `testDebugUnitTest` actually runs on a cache hit.
- 成果物: .github/workflows/ci.yml:732
- 根拠: The android job restores a build cache that already holds `:app:testDebugUnitTest` results for the same inputs. Gradle marks the task FROM-CACHE, so tests with non-deterministic behaviour, like the flake class this change fixes in Rust, are never re-run on CI. The test-stability signal the change relies on gets weaker without anyone noticing.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: `FROM-CACHE` は同一 task inputs の成果物を復元する Gradle の正規の挙動であり、Task 2 は Android の試験を毎 run 実行する要件を持たない。根拠は将来の非決定性を仮定するだけで、現行 workflow で test task が誤って cache-hit する再現を示していない。

## R5. Task 2.1's check only verifies that a Gradle cache is present. It does not check the cache key inputs (lockfiles, `.gradle.kts` files, wrapper properties) that design D1 and D3 rely on. Removing every `cache-dependency-path`, or giving chain a different key plus `restore-keys`, still prints `OK: ci`. Fix: in `has_cache`, require the three globs in `cache-dependency-path` or in the `hashFiles(...)` key, and reject `restore-keys`.
- 成果物: openspec/changes/ch25-ci-flake-gradle-cache-clock-test/check-ch25.py:1
- 根拠: A later fix drops `collector-android/**/*.lockfile` from `cache-dependency-path`. A `--write-locks`-only change then restores a stale cache under the same key, while `check-ch25.py ci` (task 2.1 and completion criterion 3) still passes. code_verify already found this (its R1).（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed check-ch25.py

## R6. The upper bound of the 'clock behind' range (`-299_000 + width`) can still be off by about 1 ms. `width` is the difference of two whole-millisecond uptime readings, so it can come out up to 1 ms below the real interval. The skew itself is truncated toward zero, which pushes negative values upward. If the server's whole-second `Date` cut falls in the last millisecond of a second and almost all of the measured interval comes after it, the test fails. The same bound appears in `clock_reference_setup_time_is_not_in_the_difference` (around line 516). Fix: add 1 ms of slack to the upper bound (`-299_000 + width + 1`), or compute the skew with `num_microseconds`.
- 成果物: crates/collector-windows/src/clock.rs:489
- 根拠: Server reply at T = x.9995 s, so `r.time` is cut to x s plus 5 min. The client reads `wall_after` 2.4 ms later and the uptime readings give width = 2. The real skew is -300000 + 999.5 + 2.4 = -298998.1, which truncates to -298998. That is not below -299000 + 2 = -298998, so the assertion panics.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R7. The new test `clock_reference_setup_time_is_not_in_the_difference` copies the 'clock ahead' and 'clock behind' checks line for line instead of reusing `skew_against`. It also has no doc comment, unlike every other test in this module. If the shared range logic changes, these four copies can drift apart. Fix: give `skew_against` a sleep parameter (or add a helper that sleeps before creating the clock) and call it twice from the new test, with a one-line `///` comment saying why it sleeps 1.5 s.
- 成果物: crates/collector-windows/src/clock.rs:496
- 根拠: Someone later adjusts the range in `clock_reference_pc_ahead_gives_positive_difference` and the 'clock behind' test, but not the copies in the new test. The regression test then checks different bounds than the spec, which Task 1.3's `check-ch25.py clock` only enforces on the two tests marked with a Scenario.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
