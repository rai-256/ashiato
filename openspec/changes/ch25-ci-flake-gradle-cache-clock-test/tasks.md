# CH25 実装タスク — CI の偶発的な赤を直す（gradle の部品の取り置き・時計のテストの許容幅）

読む順: `proposal.md` → `design.md` → このファイル → `docs/changes/CH25.md`（依頼の逐語・壊してはいけないもの・完了の判定）→ `premises.md` → `CLAUDE.md`。

軽い路（`lane.json` の `reason: same`）。仕様を持たない change（`.openspec.yaml` の `skip_specs: true`）。時計の試験は正典の Scenario
「PC の時計が進んでいると差が正で残る」「PC の時計が遅れていると差が負で残る」（`openspec/specs/desktop-collection/spec.md`）の印を持つ既存の試験を直すだけで、Scenario は変えない。
実装の後に、実際の diff が予定のパスの中にあることを機械が見る（外に触れると通常の路へ昇格する）。

## Global Constraints

- **触ってよいのは `.github/workflows/ci.yml`・`.github/workflows/nightly.yml`・`.github/workflows/release.yml`・`crates/collector-windows/src/clock.rs`・`docs/changes/CH25.md`・`openspec/changes/ch25-ci-flake-gradle-cache-clock-test/**` だけ**（`lane.json` の予定のパス。グラフが置く `docs/briefs/CH25*` と `.superpowers/` は数えない）。`tools/**`・`collector-android/**`・`openspec/specs/`・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md`・`.harness2` には触れない。ほかに直したいものに気付いても直さず、PR 本文に挙げる
- **`crates/collector-windows/src/clock.rs` は `#[cfg(test)] mod tests` の中だけを変える。** 製品のコード（`HttpDateClock::now` など、`#[cfg(test)]` より前の全部）は base と 1 字も変えない
- **正典の 2 つの Scenario の試験の幅は弱めない。** `clock_reference_pc_ahead_gives_positive_difference` の `(300_000..301_000 + i64::try_from(width).unwrap()).contains(&skew)` と、
  `clock_reference_pc_behind_gives_negative_difference` の `(-300_000 - i64::try_from(width).unwrap()..-299_000 + i64::try_from(width).unwrap())` を字面のまま残す。関数の名前と `/// Scenario: …` の印も変えない
- **workflow で変えるのは、gradle を使う 7 つの job の部品の取り置きの指定だけ**（ci.yml の `build-stamp`・`android`・`android-instrumented`・`package-linux`・`chain`、nightly.yml と release.yml の `package-linux`）。
  `chain` は `./tools/release/draft.sh --self-test` が `tools/release/build.sh apk` を通して gradle を呼ぶ（出力は捨てている）。
  取り置きの形は design D1（仮）: setup-java を持つ 6 つは、既にある `actions/setup-java` の段の `with` に `cache: gradle` と `cache-dependency-path`（`collector-android/**/*.gradle.kts` と `collector-android/**/*.lockfile` を改行で並べる）を足す。
  `chain` は design D3（仮）: checkout の直後に、ci.yml で既に使っている `actions/cache`（同じ commit の SHA と版のコメント）の段を 1 つ足し、`path` に `~/.gradle/caches` と `~/.gradle/wrapper`、
  `key` に `gradle-chain-${{ runner.os }}-${{ hashFiles('collector-android/**/*.gradle.kts', 'collector-android/**/*.lockfile') }}` を置く（setup-java は足さない。JDK を変えない）。
  段の順・ほかの段・job の条件・`on`・`permissions`・ほかの job は変えない。
  D1 の反転条件に当たって `gradle/actions/setup-gradle` を使うなら、`uses:` は 40 桁の commit の SHA に版のコメントを付けて固定し（ほかの `uses:` と同じ）、gradle を呼ぶ最初の段より前に置く
- **依存の版とロックは変えない**（`collector-android/` に触れない）。ロックの検査（`tools/check-gradle-locks.sh`）は今と同じ段・同じ順で走る
- 長いコマンド（`cargo test`・`tools/check-gradle-locks.sh`）は `scripts/quiet-run` で 1 回で待つ

## Task 1: 時計の試験で、用意の時間を差の誤差に入れない

- [ ] 1.1 **直す前に**、用意の時間を確実に長くする試験を `mod tests` に足す: 名前は `clock_reference_setup_time_is_not_in_the_difference`。
  偽の取り込み口を立ててから `HttpDateClock::new` までの間に 1,500 ms 眠り（`std::thread::sleep`）、進んでいる側（ずらし −5 分）と遅れている側（+5 分）を 1 回ずつ読み、
  それぞれ 2 つの Scenario の試験と同じ字面の幅に入ることを確かめる。この時点では今の `fake_ingest`（用意の前に決めた固定の Date）のまま書く。
  書いたら `scripts/verify-run 1.2` を走らせ、**FAIL を記録する**（必須。今の作りでは差に 1.5 秒が入り、幅を外れて必ず落ちる）。ログの失敗が新しい試験の `skew=… width=…` であること（組み立ての失敗でないこと）を自分で確かめる
- [ ] 1.2 偽の取り込み口（`fake_ingest`）が、**応答を返す瞬間の時刻**にずらしを足して `Date` を返すようにする（引数を固定の時刻からずらし `chrono::Duration` に変える。ずらしの無い試験は `Duration::zero()`）。
  `skew_against` もずらしを受け取る形にし、2 つの Scenario の試験は `skew_against(Duration::minutes(-5))`（進んでいる側）・`skew_against(Duration::minutes(5))`（遅れている側）にする。1.1 の試験も同じ形に合わせる。
  幅の字面・関数の名前・Scenario の印は変えない（Global Constraints）。直した後に `scripts/verify-run 1.2` で PASS を記録する。
  検証: `scripts/quiet-run clk -- cargo test -p ashiato-collector-windows --lib clock::tests::clock_reference` rc=0（`-- --list` で 6 本以上）
- [ ] 1.3 製品のコードが base のままで、2 つの Scenario の試験の幅と印が字面のまま残っていることを確かめる。
  検証: `python3 openspec/changes/ch25-ci-flake-gradle-cache-clock-test/check-ch25.py clock` rc=0
- [ ] 1.4 直した試験を繰り返し走らせても落ちないことを確かめる（直す前は 1 回あたりおよそ 0.5〜1% で落ちていた。依頼の見積り）。
  試験の実行ファイルを 1 回組み立て、`clock::tests::clock_reference` のうち 1.1 の試験（1.5 秒眠る）を除く 5 本であることを確かめてから、3000 回走らせる
  （spec-review の実測: 直す前の作りでも Linux では 5000 回に 22 回しか落ちず、300 回では 4 回に 1 回は通る。3000 回なら直す前に全部通る見込みは約 2×10⁻⁶。1.4 は直す前の FAIL の証跡にしない —— それは 1.1 が持つ）。
  検証: `bash -c 'set -euo pipefail; bin=$(cargo test -p ashiato-collector-windows --lib --no-run --message-format=json 2>/dev/null | python3 -c "import sys,json;print([j[\"executable\"] for j in map(json.loads,sys.stdin) if j.get(\"reason\")==\"compiler-artifact\" and j.get(\"executable\") and j[\"target\"][\"kind\"]==[\"lib\"]][-1])"); test "$("$bin" clock::tests::clock_reference --skip clock_reference_setup_time --list | grep -c ": test$")" -eq 5; for i in $(seq 3000); do "$bin" clock::tests::clock_reference --skip clock_reference_setup_time -q >/dev/null || { echo "落ちた: $i 回目"; exit 1; }; done; echo "3000 回 PASS"'` rc=0

## Task 2: gradle を使う job で、部品を CI の取り置きから取る

- [ ] 2.1 gradle を使う 7 つの job に部品の取り置きを置く（design D1（仮）・D3（仮）。Global Constraints の 7 つと形）。
  base の workflow では次の検証が「部品の取り置きを持たない」で落ちる（上流が 2026-11-21 に確かめた）。直した後に PASS になることを `scripts/verify-run 2.1` で記録する。
  検証: `python3 openspec/changes/ch25-ci-flake-gradle-cache-clock-test/check-ch25.py ci` rc=0
- [ ] 2.2 workflow を読む既存の検査が、直した workflow でも通ることを確かめる（依存の検査の配線・Windows の組み立ての鍵・試験の一時フォルダの配線）。
  配る組み立ての守り（`check-guards.sh`。`package-linux` を読む）と ADR の検査（ci.yml を読む）も含める。
  検証: `bash -c 'set -e; tools/check-vulns-wiring.sh --self-test; tools/check-vulns-wiring.sh; tools/release/windows-build-key.sh --check-workflow; tools/check-tmp-hygiene.sh --self-test; tools/check-tmp-hygiene.sh; tools/tmp-leak-check.sh --self-test; tools/release/check-guards.sh --self-test; tools/check-adr.sh --self-test; tools/check-adr.sh'` rc=0
- [ ] 2.3 ロックの検査が今と同じに効くことを確かめる（自己試験はロックを壊すと落ちることを見る。依存の版は変えていない）。
  検証: `scripts/quiet-run locks -- bash -c 'tools/check-gradle-locks.sh --self-test && tools/check-gradle-locks.sh'` rc=0

## Task 3: 触ったのが予定のパスだけであることを確かめる

- [ ] 3.1 この change が触ったファイルが予定のパスの中だけであることを確かめる（作業ツリーの変更と未追跡のファイルも見る。`git` が落ちたら落ちる。完了の判定 4）。
  検証: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(\.github/workflows/(ci|nightly|release)\.yml|crates/collector-windows/src/clock\.rs|docs/changes/CH25\.md|openspec/changes/ch25-ci-flake-gradle-cache-clock-test/|docs/briefs/CH25|\.superpowers/|$)"'` rc=0

## 人間の確認待ち

- [ ] 4.1 確認バッチで本人に聞く（CI の上でしか見えないもの）:
  (1) この PR の CI で、gradle を使う job（android・android-instrumented・build-stamp・package-linux・chain）が緑で、`Run actions/setup-java@…` の段のログに取り置きの復元の試み
  （`gradle cache is not found` か `Cache restored from key`。chain は `Run actions/cache@…` の段）、後片付けの段（`Post Run actions/setup-java@…`・`Post Run actions/cache@…`）に
  `Cache saved with the key` か `Cache hit occurred on the primary key` の行が出ているか。
  (2) merge の後、main の run が取り置きを保存した後の PR で、android の job の setup-java の段に `Cache restored from key` が出て、`./gradlew detekt` の段のログから
  wrapper の `Downloading https://services.gradle.org/…` の行が消え、段の時間が base（約 58 秒）より短くなったか（観測できなければ「観測できなかった」と答える。D1 の反転条件の材料）
