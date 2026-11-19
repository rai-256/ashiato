# CH19 実装タスク — エミュレータの計測テストを 1 クラスだけ走らせる口を足す

読む順: `proposal.md` → このファイル → `specs/emulator-startup/spec.md` → `design.md` → `docs/changes/CH19.md`（依頼の逐語・壊してはいけないもの・完了の判定）→ `tools/android-emulator.sh` → `tools/test-android-emulator.sh` → `CLAUDE.md`。

軽い路（`lane.json` の `reason: same`）。実装の後に、実際の diff が予定のパスの中にあることを機械が見る（外に触れると通常の路へ昇格する）。
検証は各項目の本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。Task 1〜3 は本物の SDK も /dev/kvm も要らない（偽の SDK）が、
検証のコマンドに `android-emulator` の字面があるので verify-run はエミュレータが要ると読む（Codex の sandbox では申請が 1 回要る。`AGENTS.md` の「/dev/kvm（Codex のとき）」）。Task 4 は本物のエミュレータを使う。

## Global Constraints

- **触ってよいのは `tools/android-emulator.sh`・`tools/test-android-emulator.sh`・`AGENTS.md`・`docs/changes/CH19.md`・`openspec/changes/ch19-emulator-only-class/**` だけ**（`lane.json` の予定のパス。グラフが置く `docs/briefs/CH19*` と `.superpowers/` は数えない）。
  `tools/test-usage-volume.sh`・`tools/emulator-start.sh`・`tools/android-backup-roundtrip.sh`・`tools/android-st58-replace.sh`・`tools/usage-volume.sh`・収集アプリのコードと計測テスト・CI・`docs/testing.md`・`openspec/specs/`・既存の change の `tasks.md` には触れない
- **テストには `Scenario: <名前>` の印を置く。** 試験は bash なので `echo 'Scenario: <名前>'` を、その場合を走らせる直前に出す（既存の場合と同じ形）。名前は `specs/emulator-startup/spec.md` の `#### Scenario:` と一字一句合わせる。この change の Scenario は 13 本
- **全体の経路（`ONLY` が無いか空）の既存の行は、字下げも含めて変えない**（design D1）。ONLY の値の検査は AVD の確かめ（`avdmanager list avd`）より前に、ONLY の段の塊は
  `rm -rf "$OUT"/androidTest-results-stage*` の行の後・全体の 1 段目の gradle の前に置き、塊の中の `keep_stage` は字下げする（`assert_normal` が行頭の `keep_stage` の数と段の置き場の数を比べる）。
  塊は `tools/test-usage-volume.sh` の検査（コメントでない最初の `connectedDebugAndroidTest` の行が `adb uninstall dev.ashiato.collector ` の行より後・採寸の試験を `notClass` で除く行の形）を変えずに通る位置に置く
- **試験は台本に試験用の口を足さない**（CH10 の D16）。使い捨ての木に本物の台本を写し、隣を偽物にする。`ONLY` は台本の正規の口で、試験用の口ではない
- **試験の中で場合選びを持つ変数 `ONLY`（`tools/test-android-emulator.sh` の `ONLY=${ASHIATO_TEST_ONLY:-}`）を `CASE` に改める**（台本の新しい口と同じ名前だと、呼んだ人の環境に `ONLY` が export されているとき台本へ漏れる。spec-review R8）。環境変数 `ASHIATO_TEST_ONLY` の名前は変えない。
  `run_emulator` と、台本を直に起こす場合（`assert_term_cleanup` など）は、`ONLY` を渡す場合だけ明示して渡し、渡さない場合は `env -u ONLY` で外す
- 試験の一時の場所・偽のプロセスの後始末・`timeout 30` の見張りは既存の形（`new_tree`・`TMP_ROOTS`・`run_emulator`）に乗せる。`/tmp` の固定名に書かない
- **長いコマンドは `scripts/quiet-run`**（Task 4 の本物のエミュレータ。全体は 1 回 17 分ほど）
- `docs/changes/CH19.md` の「依頼（逐語）」を書き換えない

## Task 1: 偽の SDK が命令を 1 本の記録に残し、全体の命令の列を固定する

- [x] 1.1 `tools/test-android-emulator.sh` の偽物を、受け取った命令を**同じ 1 本の記録**（`FAKE_SEQ_LOG`。木の中の `seq.log`）に 1 行ずつ足す形にする: 偽の gradle は `gradle <引数>`、偽の adb は `adb <引数>`、偽の `android-media-fixtures.sh` は `media-fixtures`、偽の `android-st58-replace.sh` は `st58-replace`（既存の `FAKE_LOG`・`FAKE_ADB_LOG` への記録は残す。既存の場合が読む）。
  偽の gradle は、引数に `connectedDebugAndroidTest` があるときだけ結果の XML を書く（`installDebug` だけのときは書かない。spec-review R1）。`classname` は、引数に `-Pandroid.testInstrumentationRunnerArguments.class=<名前>` があればその名前、無ければ今と同じ `fake`、`FAKE_GRADLE_CLASS` が渡されたときはその値にする。`FAKE_GRADLE_RC` が渡されたときは、XML を書いた後にその rc で終わる。
  偽の adb は、`FAKE_VERIFY_OUT` が渡されたとき、`St56BackupTest#verifyRestored` の `am instrument` にその値を返す（無ければ今どおり `OK (1 test)`）。
  場合選びの変数を `CASE` に改め（Global Constraints）、`run_emulator` に `ONLY` を渡す引数を足す（既存の呼び出しは変えずに済む形で）。命令の列の比べは、`seq.log` から `getprop`・`devices`・`get-state`・`wait-for-device`・`emu avd name` を含む行を除いたもので行う（spec と同じ一覧）。
  場合を 2 つ足す（`ASHIATO_TEST_ONLY` の一覧にも足す）:
  `only-empty`: `echo 'Scenario: ONLY が空なら、渡さないときと同じ命令の列で全段を走らせる'`（`ONLY` を渡さない木と `ONLY=` の木。どちらも rc=0・段の置き場の数が 2 つで同じで、行頭の `keep_stage` の行の数と等しい（8 と決め打たない。`assert_normal` と同じ数え方）・`st58-replace` が 1 回・命令の列が同じ） /
  `only-base`: `echo 'Scenario: ONLY を渡さなければ、この change の前の版の台本と同じ命令の列で全段を走らせる'`（比べる元の版は明示した `ASHIATO_TEST_BASE_REV` だけ。渡されなければ `比べる元の版が無いので飛ばす: only-base` を出して通す —— merge の後に全体の段を足す Story で落ちないよう、`origin/main` を既定にしない。元の版の台本は `git show <rev>:tools/android-emulator.sh` で写す（`ASHIATO_TEST_SCRIPT_REV` と同じ写し方））。
  検証: `tools/test-android-emulator.sh` rc=0 / `bash -c 'out=$(ASHIATO_TEST_ONLY=only-base ASHIATO_TEST_BASE_REV=$(git merge-base origin/main HEAD) tools/test-android-emulator.sh) && ! grep -q "飛ばす" <<<"$out"'` rc=0 / `tools/test-usage-volume.sh` rc=0
- [x] 1.2 `--self-test` に、`only-base` が全体の経路の変化を本当に見ていることの確かめを足す: 写した台本の全体の経路から `adb shell pm reset-permissions` の行を 1 つ消すと `only-base` の比べ（元の版は消していない台本）が落ち、同じ行を gradle の段の後ろへ動かしても落ちる（gradle と adb をまたいだ順を見ている。壊すのは使い捨ての木の写しだけ）。
  検証: `tools/test-android-emulator.sh --self-test` rc=0

## Task 2: ONLY の口（ふつうのクラス・専用の台本のクラス・形の違う値）

- [x] 2.1 `tools/android-emulator.sh` に design D1・D3（仮）・D4・D5（仮）のとおり `ONLY` の口を足す。冒頭のコメントの使い方に `ONLY=dev.ashiato.collector.<クラス> ./tools/android-emulator.sh` の 1 行と、なぜ要るか（CH19）を足す。
  ふつうのクラスの段: 前の実行の結果（`$OUT/androidTest-results`）を消す → `adb shell am force-stop dev.ashiato.collector` → `adb shell pm reset-permissions` → 全体と同じ `-Pashiato.baseUrl=http://127.0.0.1:18787` と `-Pandroid.testInstrumentationRunnerArguments.class="$ONLY"` だけを付けた `connectedDebugAndroidTest` → 字下げした `keep_stage` →
  `androidTest-results-stage1` の XML に `classname="$ONLY"` が無ければ `error: ONLY=<クラス> のテストが 1 本も走っていない` を標準エラーに出して rc=1 → 結果の置き場を出して `exit 0`。値の誤りと専用の台本のクラスは rc=1（D4・D5 の文言）。
  検証: `bash -n tools/android-emulator.sh` rc=0 / `tools/test-usage-volume.sh` rc=0 / `bash -c 'base=$(git merge-base origin/main HEAD) && test "$(git diff "$base" -- tools/android-emulator.sh | grep -c "^-[^-]")" -eq 0'` rc=0（全体の経路の行を消していない・書き換えていない）
- [x] 2.2 `tools/test-android-emulator.sh` に次の場合を足す（鍵は `ASHIATO_TEST_ONLY` の一覧にも足す）。それぞれの前に印を出す:
  `only-class`: `echo 'Scenario: ONLY にクラスを渡すと、そのクラスだけに絞った計測テストを 1 回だけ走らせる'` /
  `only-prep`: `echo 'Scenario: ONLY でも、全体と同じにアニメーションを切り、写真と動画を入れる'` /
  `only-order`: `echo 'Scenario: ONLY の段の前に、アプリを外し、止めて、権限を戻す'`（`seq.log` の順で見る） /
  `only-keep`: `echo 'Scenario: ONLY と KEEP=1 を一緒に渡すと、通った後にエミュレータを止めない'` /
  `only-fail`: `echo 'Scenario: ONLY のクラスのテストが落ちたら、rc≠0 で止まり、エミュレータを止める'`（`FAKE_GRADLE_RC=1`。`のテストが 1 本も走っていない` が標準エラーに無いことも見る） /
  `only-zero`: `echo 'Scenario: 渡したクラスの結果が無ければ、0 本として止まる'`（`FAKE_GRADLE_CLASS=dev.ashiato.collector.Other`） /
  `only-stale`: `echo 'Scenario: 前の実行の結果が残っていても、今回の結果として数えない'`（走らせる前に木の `collector-android/app/build/outputs/androidTest-results/connected/TEST-stale-_app-.xml` に渡したクラスの `classname` を持つ XML を置き、`FAKE_GRADLE_CLASS=dev.ashiato.collector.Other`。rc=1 と `error:` の行を見る） /
  `only-dedicated`: `echo 'Scenario: 入れ替えの試験か採寸の試験を渡すと、専用の台本の名前を出して起こさずに止まる'` /
  `only-bad`: `echo 'Scenario: 完全修飾のクラス名 1 つの形でない値を渡すと、起こさずに止まる'`。
  中身は spec の各 Scenario の WHEN / THEN のとおり（偽のエミュレータが呼ばれたかは `FAKE_ARGS` が無いこと、`avdmanager` は `FAKE_AVD_LOG` が空であることで見る）。
  `--self-test` に、写した台本の ONLY の段から次を外すとその場合が落ちることを足す: `class=` の絞り → `only-class` / XML の確かめ → `only-zero` / 前の実行の結果を消す行 → `only-stale`。
  検証: `tools/test-android-emulator.sh` rc=0 / `tools/test-android-emulator.sh --self-test` rc=0 / `bash -c 'base=$(git merge-base origin/main HEAD); out=$(ASHIATO_TEST_ONLY=only-class ASHIATO_TEST_SCRIPT_REV=$base tools/test-android-emulator.sh 2>&1) && exit 1; grep -q "Scenario: ONLY にクラスを渡すと、そのクラスだけに絞った計測テストを 1 回だけ走らせる" <<<"$out"'` rc=0（直す前の台本では、その場合の確かめまで走って落ちる）

## Task 3: バックアップの試験の手順と、AGENTS.md の書き方

- [x] 3.1 `tools/android-emulator.sh` の ONLY の塊に、design D2（Q1）のとおり `St56BackupTest` の経路を足す（全体の 2〜3 段目と同じ命令を同じ順。**`keep_stage` は呼ばない**。`OK (1 test)` の照合と文言は全体と同じ。XML の確かめはしない）。全体の経路の行は変えない（関数に括り出さない）。
  `tools/test-android-emulator.sh` に場合を 2 つ足す:
  `only-st56`: `echo 'Scenario: バックアップの試験を渡すと、gradle の計測テストを呼ばずに準備・往復・確かめを順に走らせる'`（`androidTest-results` の無い木で。段の置き場が 1 つも無いことも見る） /
  `only-st56-fail`: `echo 'Scenario: バックアップの試験の確かめが通らなければ、全体と同じ文言で止まる'`（`FAKE_VERIFY_OUT='FAILURES!!!'`）。
  検証: `tools/test-android-emulator.sh` rc=0 / `bash -c 'ASHIATO_TEST_ONLY=only-st56 tools/test-android-emulator.sh'` rc=0 / `bash -c 'out=$(ASHIATO_TEST_ONLY=only-base ASHIATO_TEST_BASE_REV=$(git merge-base origin/main HEAD) tools/test-android-emulator.sh) && ! grep -q "飛ばす" <<<"$out"'` rc=0
- [x] 3.2 `AGENTS.md` の「長いコマンド」に design D6 の 1 項目を足す（`<!-- harness2:long-commands -->` の印と既存の行は変えない）。
  検証: `bash -c 's=$(awk "/^### 長いコマンド/{f=1;next} /^##/{f=0} f" AGENTS.md); grep -q "harness2:long-commands" <<<"$s" && grep -q "ONLY=dev.ashiato.collector" <<<"$s" && grep -q "scripts/verify-run" <<<"$s" && grep -q "cargo test" <<<"$s" && grep -q -- "-- --list" <<<"$s"'` rc=0

## Task 4: 本物のエミュレータで絞った実行と全体を走らせ、触ったのが予定のパスだけであることを確かめる

- [x] 4.1 本物のエミュレータで、ふつうのクラスとバックアップの試験を ONLY で 1 回ずつ走らせる（`/dev/kvm` が要る。開けなければ verify-run が `BLOCKED_INFRA` を返す）。
  落ちたら、落ちたのが台本の ONLY の経路か、試験の中身か（同じクラスが全体の実行で通るか）をログで分け、試験の中身なら直さずに PR 本文に挙げる。
  St56 は、置き場の無い状態でも通ることを見るため、先に `collector-android/app/build/outputs/androidTest-results` を消してから走らせる（spec-review R1）。
  検証: `bash -c 'rm -rf collector-android/app/build/outputs/androidTest-results && scripts/quiet-run emu56 -- env ONLY=dev.ashiato.collector.St56BackupTest tools/android-emulator.sh'` rc=0 / `scripts/quiet-run emu1 -- env ONLY=dev.ashiato.collector.MediaPremisesInstrumentedTest tools/android-emulator.sh` rc=0
- [x] 4.2 本物のエミュレータで全体（ONLY なし）を走らせる（壊してはいけないもの 1。1 回 17 分ほど。1 回で待つ）。落ちたら、同じ失敗が base の台本でも起きるか（`git stash` を使わず、`git show <base>:tools/android-emulator.sh` を一時の場所に写して走らせる）で分け、base でも落ちるなら直さずに PR 本文に挙げて A として人間に返す。
  検証: `scripts/quiet-run emu -- tools/android-emulator.sh` rc=0
- [x] 4.3 この change が触ったファイルが予定のパスの中だけであることを確かめる（作業ツリーの変更と未追跡のファイルも見る）。
  検証: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(tools/android-emulator\.sh|tools/test-android-emulator\.sh|AGENTS\.md|docs/changes/CH19\.md|openspec/changes/ch19-emulator-only-class/|docs/briefs/CH19|\.superpowers/|$)"'` rc=0
