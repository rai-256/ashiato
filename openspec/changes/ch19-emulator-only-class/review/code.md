<!-- 処置の判定: re-review -->
# CH19 code-verify（ch19-emulator-only-class）

HEAD `fda0fbc62`（`1ae6ebe27` から tools/ は変わっていない。それ以降の差分は evidence・tasks・review だけ）。base = `git merge-base origin/main HEAD` = `25ddd53a5`。

## 申告: 9/9 `[x]`・検証コマンド一覧 → 実測

| 項目 | 検証コマンド（要点） | 申告 | 実測（この席で実行） |
|---|---|---|---|
| 1.1 | `tools/test-android-emulator.sh` | PASS | rc=0（35 秒。CH19 の 13 本の印がすべて出る。`only-base` は「飛ばす」） |
| 1.1 | `ASHIATO_TEST_ONLY=only-base ASHIATO_TEST_BASE_REV=$(merge-base)` … `! grep 飛ばす` | PASS | rc=0 |
| 1.1 / 2.1 | `tools/test-usage-volume.sh` | PASS | rc=0 |
| 1.2 / 2.2 | `tools/test-android-emulator.sh --self-test` | PASS | rc=0 |
| 2.1 | `bash -n tools/android-emulator.sh` | PASS | rc=0 |
| 2.1 | base からの diff に `^-[^-]` が 0 行 | PASS | rc=0 |
| 2.2 | 直す前の台本（`ASHIATO_TEST_SCRIPT_REV=$base`）で `only-class` が落ち、印が出る | PASS | rc=0 |
| 3.1 | `ASHIATO_TEST_ONLY=only-st56 tools/test-android-emulator.sh` | PASS | rc=0 |
| 3.2 | AGENTS.md の「長いコマンド」の grep 5 つ | PASS | rc=0 |
| 4.1 | `ONLY=…St56BackupTest` 本物のエミュレータ | PASS（207 秒） | **走らせ直して一致**: `rm -rf collector-android/app/build/outputs/androidTest-results && scripts/quiet-run rv56 -- env ONLY=dev.ashiato.collector.St56BackupTest tools/android-emulator.sh` rc=0、末尾に `OK (1 test)`、段の置き場は 0 個 |
| 4.1 | `ONLY=…MediaPremisesInstrumentedTest` 本物のエミュレータ | PASS（174 秒） | **走らせ直して一致**: `scripts/quiet-run rv1 -- env ONLY=dev.ashiato.collector.MediaPremisesInstrumentedTest tools/android-emulator.sh` 2 分 41 秒で通り、`== 結果 … stage1` が出た |
| 4.2 | `tools/android-emulator.sh`（全体）本物のエミュレータ | PASS（1992 秒） | 走らせ直していない（33 分）。証跡のログ（124 行）の `sha256sum` が記録の `log_sha256`（`d4302fbf…`）と一致し、末尾に ST58 の `OK (1 test)` 7 本と `stage1〜8`。置き場が stage1〜8 の 8 つだったのは、この席の ONLY の実行より前に見たとき（その後の ONLY の実行で置き場と `androidTest-results` は消えた。手 6） |
| 4.3 | 予定のパスの外の差分が無い | PASS | rc=0 |

実測: **一致**（表の 14 行のうち 13 行をこの席で実行し、すべて申告どおり。4.2 だけはログのハッシュと中身で突き合わせた）。完了の判定 6 の「ONLY の 2 回が全体より短い」は 207 秒・174 秒 < 1992 秒で成り立つ。

追加で走らせたもの（実機）: `scripts/quiet-run rvnone2 -- env ONLY=dev.ashiato.collector.NoSuchClassCh19 tools/android-emulator.sh` → **rc=1**。本物の gradle は `There were failing tests` / `BUILD FAILED in 13s` で落ち、XML は `initializationError` の 1 本（`classname="dev.ashiato.collector.NoSuchClassCh19"`）。続けた St56 の起動は同じ AVD の検査を通り、終わった後に qemu は残っていない。無いクラスの名前を渡しても「0 本なのに rc=0」にはならない（R1 の重さはこれで決めた）。

## 手 1: 該当なし（確かめた範囲: 期待値を直書きしたハッシュ・数値は無い。ONLY の形の正規表現を python の `re` で別に評価し、spec の正しい値 2 つが True、だめな値 4 つ（パッケージ無し・`,`・`#`・空白）と `$` 入りが False で、bash の `=~` と一致。改行入りも bash で False）

## 手 2: ガードを壊す

tools の写しを `/tmp/ch19probe.*` に作り、その写しだけを壊して、該当する場合を走らせた（作業ツリーは前後で `git status --porcelain` と `git diff` のハッシュが同じ。写しは消した）。

| 壊したもの | 走らせた場合 | rc |
|---|---|---|
| 壊さない写し（下地の確認） | only-class | 0 |
| ONLY の St56 の経路から `./tools/android-backup-roundtrip.sh` を消す | only-st56 | 1（落ちた） |
| ONLY の St56 の経路の `exit 0` を消す（ふつうの経路へ落ちる） | only-st56 | 1（落ちた） |
| ONLY の塊の `am force-stop` と `pm reset-permissions` を消す | only-order | 1（落ちた） |
| 採寸の試験の `case` の分岐を外す | only-dedicated | 1（落ちた） |
| 形の検査（`=~`）を `if false` にする | only-bad | 1（落ちた） |
| St56 の `OK (1 test)` の照合を `true \|\|` で潰す | only-st56-fail | 1（落ちた） |
| アニメーションの設定 3 つを `window_animation_scale` だけにする | only-prep | **0（落ちない）** → R2 |
| 先頭の `adb uninstall dev.ashiato.collector.test` の行を消す | only-order | **0（落ちない）** → R4 |
| ONLY の St56 の塊から `adb shell bmgr enabled \| grep -qx …` を消す | only-st56 | **0（落ちない）** → 手 4 |
| `git init` した写しに今の台本を commit し、全体の経路の Health の段の前の `pm reset-permissions` を 1 行消す | only-base（`ASHIATO_TEST_BASE_REV=HEAD`。本物の入口の `assert_only_base`） | 1（落ちた）。消さない写しでは 0 |
| 同じ壊し方 | only-empty | 0（落ちない。今の台本同士の比べなので設計どおり） |
| 呼んだ側の環境に `ONLY='bad value'` を置く | `tools/test-android-emulator.sh` / `--self-test` | どちらも 0（試験へ漏れていない） |
| `git show $base:AGENTS.md` に 3.2 の grep を掛ける | 3.2 の検査 | base には `ONLY=dev.ashiato.collector`・`scripts/verify-run`・`-- --list` が無く落ちる（足した 1 行を見ている） |
| 偽の gradle が結果の置き場だけ作って XML を書かない | only-zero を書き換えて観測 | 台本は **rc=0** で `== 結果` を出した → R1 |

`--self-test` が既に持つ壊し方（`class=` → `notClass`・XML の確かめを外す・前の結果を消す行を外す・全体の経路の `pm reset-permissions` を消す / 動かす）は rc=0 で通る。

検査の外側: CI（`.github/workflows/ci.yml:183-184`）は `ASHIATO_TEST_BASE_REV` を渡さないので、`only-base` は CI ではいつも「飛ばす」。spec が「元の版を渡す PR の証跡で効く」と書いた設計どおりで、PR の証跡（1.1・3.1）では元の版を渡して通っている。指摘にはしない。

## 手 3: Scenario と test

`python3 scripts/check_scenarios.py . ch19-emulator-only-class` → `scenarios: OK`（rc=0。CH19 の 13 本は担保なしの一覧に無い）。印の先の assert を 1 本ずつ読み、主張と観測を突き合わせた。11 本は WHEN / THEN の主張どおりの観測がある。2 本は主張より観測が弱い（only-prep は R2、only-order は R4）。「渡したクラスの結果が無ければ」は、観測した形（別のクラスの XML）では落ちるが、XML が 1 本も無い形が通る（R1）。

## 手 4: 本人の決定

- Q1「St56 は全体と同じ準備・往復・確かめだけ」: 往復の行か `exit 0` を消すと only-st56 が落ちる（手 2）。ただし固定されているのは spec の Scenario が挙げた鍵の行（installDebug・`bmgr enable true`・prepare・`backupnow`・`bmgr restore`・verify の順）だけで、`bmgr enabled | grep -qx …` の確かめを消しても落ちない。spec の Scenario はこの行を挙げていないので指摘にはしない（「全体と同じ命令」を全部固定したければ、St56 の塊と全体の 2〜3 段目の命令の列の比べが要る）
- Q1 に含まれる「入れ替え・採寸は起こさずに止める」: 分岐を外すと only-dedicated が落ちる。固定されている
- 「指定が無ければ今の 8 段のまま」: 8 を直書きしたテストは無い（spec が「決め打たない」と決めた）。全体の段を変えて落ちるのは `only-base` に元の版を渡したときだけで、CI では飛ぶ（手 2 の外側を参照）。spec が選んだ形なので指摘にはしない
- 「AGENTS.md に絞り方と最後に全体を verify-run で」: 3.2 の grep 5 つで固定されている（rc=0）

## 手 5: `[x]` と実体

9 件とも検証コマンドが実在し、rc=0（表）。`cargo test --test` 型のコマンドは無い。Task 4 の 3 本は証跡のログが実在し、`log_sha256` の付いた PASS がある。

## 手 6: 隙間

- ONLY の経路は `$OUT/androidTest-results` を消すので、全体の実行が残した `connected/st57-update` の写しも消える。design D3 が書いた受け入れ済みの代償で、全体を走らせ直せば作り直されるので、失って戻せないものではない
- この席の ONLY の実行 2 回で、4.2 が残した stage1〜8 の置き場と `androidTest-results`（`connected/st57-update` の写しを含む）が消えた（実測。D3 の代償どおり）
- 本物の gradle に無いクラスを渡すと rc=1 で落ちる（上の実機）。「0 本なのに rc=0」は本物の経路では起きなかった
- 環境に `ONLY` が残ったまま全体の検証コマンドを証跡に取ると、1 クラスの実行が全体の PASS として残る → R3

## 手 7: 該当なし（`python3 scripts/plan_fix.py pending openspec/changes/ch19-emulator-only-class` → `[]`、rc=0）

---

## R1. ONLY の「0 本なら止まる」の確かめが、XML が 1 本も無いときに素通りして rc=0 になる
- 成果物: tools/android-emulator.sh / tools/test-android-emulator.sh
- 根拠: `tools/android-emulator.sh:141-142` は `find … -name 'TEST-*.xml' -exec grep -Fq "classname=\"$ONLY\"" {} +` の rc で判定している。`find` は当たるファイルが 0 件だとコマンドを 1 度も起こさずに rc=0 を返す（空のディレクトリで `find $e -type f -name 'TEST-*.xml' -exec grep -Fq x {} +` → rc=0 を実測）。写しの偽の gradle を「置き場だけ作って XML を書かない」に変えると、台本は rc=0 で `== 結果: … stage1` を出した（`probe rc=0`）。`only-zero` は「別のクラスの XML がある」形しか見ていないので緑のまま。本物の gradle に無いクラスを渡したときは `BUILD FAILED` で先に落ちるので、いまの実機ではまだ起きていない。ただ design D3 の前提「0 本でも終わり方が分かりにくい」に対して、この確かめは開いたまま失敗する
- kind: technical
- 提案: `grep -rqF "classname=\"$ONLY\"" "$OUT/androidTest-results-stage1"`（ファイルが無ければ rc=1）にする。`only-zero` に「XML を書かない」形を足し、`--self-test` に今の `find` の形へ戻すと落ちることを足す
- 処置: fixed 2.1 — `grep -rqF --include='TEST-*.xml'` に替え、XML を一切書かない偽 Gradle の場合とガードを外した場合を self-test で落とす。
- 判定: ADDRESSED（re-review 1 @4f1c6925f30ed4c02fe54c9ba6d4657bbb554299）

## R2. 「アニメーションの設定 3 つ」の Scenario は 1 つしか見ておらず、gradle より前であることも見ていない
- 成果物: tools/test-android-emulator.sh（`assert_only_prep`）/ specs/emulator-startup/spec.md
- 根拠: spec の THEN は「アニメーションの設定 3 つ（…）が届き、写真と動画を入れる台本が 1 回呼ばれ、どちらも gradle の計測テストより前」。`assert_only_prep`（`tools/test-android-emulator.sh:608-616`）は `window_animation_scale` の 1 つが在ることと、`media-fixtures` の行番号が gradle の行より前であることだけを見る（アニメーションの行の順は比べていない。読んだだけ）。写しの台本で `for key in` を `window_animation_scale` だけにすると、`ASHIATO_TEST_ONLY=only-prep` は rc=0（落ちない）
- kind: technical
- 提案: 3 つの鍵をそれぞれ grep し、それぞれの行番号が `connectedDebugAndroidTest` の行より前であることを見る（`media-fixtures` と同じ形）
- 処置: fixed 2.2 — 3 つの animation key を個別に存在・Gradle より前で確認する。
- 判定: ADDRESSED（re-review 1 @4f1c6925f30ed4c02fe54c9ba6d4657bbb554299）

## R3. 環境に `ONLY` が残っていると、全体の検証コマンドの証跡が 1 クラスの実行のまま PASS になり、記録からは見分けられない
- 成果物: tools/android-emulator.sh / AGENTS.md（`scripts/evidence.py` は harness2 側）
- 根拠: 台本は `ONLY` を環境から読む（`tools/android-emulator.sh:21`・`120`）。`scripts/evidence.py:419` と `:480` は検証コマンドを `{**os.environ, …}` で起こすので、呼んだ側が `ONLY` を export していれば `tools/android-emulator.sh`（全体のコマンド）が ONLY の経路を通る。その経路は rc=0 で終わる（only-class が rc=0、本物でも rv1 が通った）。証跡の記録の `env` は `kvm`・`java_home`・`android_home` だけで（`evidence.jsonl` の 4.2 の行）、`command` も全体と同じ文字列になる。違いが残るのは summary の `stage1` と `stage1〜8` の差だけ。AGENTS.md は「絞った実行は Task の完了の証跡にしない」と書くが、それを機械では止めていない。CH19 の試験側は `CASE` に改めて漏れを防いだ（`ONLY='bad value'` を置いて `tools/test-android-emulator.sh` と `--self-test` がどちらも rc=0 を実測。spec-review R8）が、台本の利用者の側の漏れは開いている
- kind: technical
- 提案: ONLY の経路の終わりの行（`== 結果`）の前に、標準エラーへ `== 絞った実行（ONLY=…）。全体ではない` を出す（ONLY の塊の中なので、全体の経路の行は変わらない）。あわせて harness2 の `evidence.py` が `ONLY` を記録の `env` に残すか外すかを決める（harness2 #271 の絞った検証と一緒に。ここでは提案だけ）
- 処置: fixed 2.1 — ONLY 経路が標準エラーに「絞った実行・全体ではない」を必ず残し、偽 SDK 試験もその印を照合する。evidence.py の環境記録はこの Story の予定外であり変更しない。
- 判定: ADDRESSED（re-review 1 @4f1c6925f30ed4c02fe54c9ba6d4657bbb554299）

## R4. 「アプリを外し、止めて、権限を戻す」の順の Scenario は、先頭の命令が無くても通る
- 成果物: tools/test-android-emulator.sh（`assert_only_order`）
- 根拠: `assert_only_order` の awk（`tools/test-android-emulator.sh:620-625`）は、見つからなかった行の番号を 0 のまま `u<a && a<f && f<p && p<g` で比べる。写しの台本から `adb uninstall dev.ashiato.collector.test >/dev/null 2>&1 || true` を消すと、`ASHIATO_TEST_ONLY=only-order` は rc=0（落ちない）。spec の THEN は `uninstall dev.ashiato.collector.test` が命令の記録にあることを主張している。この行は全体の経路と共有なので、PR の証跡の `only-base` では落ちるが、Scenario の印の先のテストは見ていない
- kind: technical
- 提案: awk の END で 5 つの番号がすべて 0 でないことも条件にする（`u && a && f && p && g && u<a && …`）。`--self-test` に、この行を消すと `only-order` が落ちることを足す
- 処置: fixed 2.2 — 5 つの命令が全て存在することを awk の順序判定に加えた。
- 判定: ADDRESSED（re-review 1 @4f1c6925f30ed4c02fe54c9ba6d4657bbb554299）

## security review（25ddd53..112ae15）

見た経路:
- 環境変数 `ONLY` → `tools/android-emulator.sh:21-36` の case と正規表現の検査 → `:139` の gradle 引数 `-Pandroid.testInstrumentationRunnerArguments.class="$ONLY"`（引用付き）と `:141-142` の `grep -Fq`（固定文字列）。シェル・gradle の引数・正規表現のどれにも検査を通った形でしか入らない（4）
- `ONLY=dev.ashiato.collector.St56BackupTest` → `:122-133` は固定の命令（`bmgr`・`am instrument`・既存の `tools/android-backup-roundtrip.sh`）だけで、`ONLY` の値は比較にしか使わない（3）
- 待ち受けと接続先: 差分の gradle 呼び出しの `ashiato.baseUrl` は既存の経路と同じ `http://127.0.0.1:18787`。新しい listen・外部への送信先は無い（2: `tools/android-emulator.sh:138`）
- 出力: `echo "$verify_out"` はエミュレータ上の計測テストの出力（台本が作る試験のデータ）で、記録の中身ではない（2: `:129-131`）
- `tools/test-android-emulator.sh` の追加分は一時の写しの中の偽の gradle で、`rm -rf`・`mktemp` 以外の外への作用（通信・eval）は無い（2: 差分を grep して該当が `sed -i` の 1 行だけ）
- 依存: gradle / Cargo / npm の定義に差分なし（4: `git diff … -- '*.gradle*' '*Cargo*' '*package*.json'` が 0 行）
- 認可・秘密: 差分は開発者の手元の道具と AGENTS.md だけで、API・画面・token・合言葉に触れない（2）

安全の決め手: `ONLY` は使われる前に必ず `^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)+$` の検査を通り、外れたら `exit 1` する（`tools/android-emulator.sh:32-35`。`:120` 以降の使い所はすべてこの後）—— 確かさ 4。`:21-36` を切り出して走らせ、`a.b;touch /tmp/x`・`a.b$(id)`・改行入り・`a.b -Pfoo=1`・`a.b,c.d`・`a.b#m`・`-Dx.y`・`a.b"`・`a.b*` はすべて `error: … 完全修飾のクラス名 1 つの形でない` で止まり、`dev.ashiato.collector.St56BackupTest` だけが通った

指摘なし

## /code-review（25ddd53..ce62bc9）

見た: Claude Code の組み込みの `/code-review`（9 件）。写しは scripts/code_review_record.py

## R5. The zero-tests guard passes when the stage directory has no XML at all. `find ... -exec grep -Fq ... {} +` returns 0 when nothing matches the file pattern, because grep never runs, so `if ! find` does not fire. Fix: count the files first (for example `compgen -G` or `find ... | grep -q .`), or use `grep -rqF --include='TEST-*.xml' "classname=\"$ONLY\"" "$OUT/androidTest-results-stage1"`, which returns 1 when no file matches.
- 成果物: tools/android-emulator.sh:141
- 根拠: ONLY names a class that gradle filters to nothing, or gradle writes no XML for some reason, and gradle exits 0. `keep_stage` makes an empty `androidTest-results-stage1`. The find returns 0, the script prints the results path and exits 0. A run where zero tests ran is reported as a pass. `assert_only_zero` and `assert_only_stale` only cover the case where an XML exists with the wrong classname, so the tests miss this.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1 — R1 と同じ `grep -rqF --include='TEST-*.xml'` の判定と XML 無しの試験で、空の段を rc=1 にする。
- 判定: ADDRESSED（re-review 1 @4f1c6925f30ed4c02fe54c9ba6d4657bbb554299）

## R6. The `only-base` Scenario passes without checking anything when `ASHIATO_TEST_BASE_REV` is not set. The default run (`tools/test-android-emulator.sh`, as used by CI and the verify runs) never sets it. It prints the Scenario marker and returns 0. Fix: make the skip visible to the Scenario check (non-zero rc or a distinct marker), or make sure CI always passes a base rev.
- 成果物: tools/test-android-emulator.sh:578
- 根拠: A later change edits the full-path command sequence by mistake. The plain test run still prints 'Scenario: ONLY を渡さなければ…同じ命令の列で全段を走らせる' and exits 0, so the Scenario looks covered while nothing was compared.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: tasks.md:32 の Task 1.1 は、merge 後の別 Story で余計に落とさないため base が無いときは「飛ばす」を出して rc=0 とすることを明示している。CI に base を必須化する変更は予定外であり、この Scenario の PR 証跡は Task の指定どおり base を明示して実行する。
- 判定: ADDRESSED（re-review 1 @4f1c6925f30ed4c02fe54c9ba6d4657bbb554299）

## R7. The self-test for `only-base` does not exercise `assert_only_base`. It runs `run_emulator` and `filtered_seq` on two copies of the current script and compares them directly. Fix: drive `assert_only_base` itself against a broken copy, for example by pointing `ASHIATO_TEST_BASE_REV` at a commit or tree that holds the mutated script.
- 成果物: tools/test-android-emulator.sh:502
- 根拠: `assert_only_base` loses its comparison, or stops applying `ASHIATO_TEST_SCRIPT_REV`, so both trees are the current script. `--self-test` still passes because it never calls the function, and `only-base` then always passes.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.2 — self-test は一時 git tree の base commit と reset-permissions を消した作業版を用意し、`assert_only_base` 自体へ渡して失敗を確かめる。
- 判定: ADDRESSED（re-review 1 @4f1c6925f30ed4c02fe54c9ba6d4657bbb554299）

## R8. The ONLY St56 path ignores the result of the `prepareBackup` instrumentation. `am instrument` exits 0 even when the test fails, and its output is not grepped for `OK (1 test)` the way `verifyRestored` is. The full path at line 160 has the same flaw and this copies it. Fix: capture the output and check `^OK (1 test)` before running the backup round trip, in both places.
- 成果物: tools/android-emulator.sh:126
- 根拠: `prepareBackup` throws (assertion or crash). The script goes on to backup and restore and then fails later at verify or backup with 'バックアップが取れない' or '復元の確かめが通らない'. That misleads the person using ONLY to narrow the cause.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 3.1 — 新設した ONLY の St56 経路では prepare の出力も `OK (1 test)` と照合して、往復前に止める。全体経路は tasks.md Global Constraints の「既存の行を変えない」により変更しない。
- 判定: ADDRESSED（re-review 1 @4f1c6925f30ed4c02fe54c9ba6d4657bbb554299）

## R9. The ONLY validation accepts any dotted identifier, so dedicated names and non-test types get through. Annotations such as `dev.ashiato.collector.NeedsPristinePermissions` and misspelled dedicated classes boot the emulator. The dedicated-script check compares exact strings only. Nested classes (`Outer$Inner`) are rejected even though instrumentation can run them. Fix: check that the class source exists under `collector-android/app/src/androidTest` (`<Name>.kt` or `.java`), so the error comes before the emulator boots.
- 成果物: tools/android-emulator.sh:32
- 根拠: `ONLY=dev.ashiato.collector.NeedsPristinePermissions` passes validation, boots the emulator (minutes) and runs a gradle class filter on an annotation. The run ends in a gradle error or zero tests, and combined with the find bug above it may report success.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: design.md:51 の D5 と tasks.md:39 の Task 2.1 が定める入力契約は「完全修飾のクラス名 1 つ」の形であり、ファイル配置を API 契約にするものではない。提案の source 探索は Kotlin/Java と nested class の対応を新たに定義し、予定外の正当な instrumentation 名を拒むため、この Story では増やさない。XML 無しの成功は R1/R5 で止まる。
- 判定: ADDRESSED（re-review 1 @4f1c6925f30ed4c02fe54c9ba6d4657bbb554299）

## R10. The ONLY path runs `rm -rf "$OUT/androidTest-results"`, which also deletes `connected/st57-update`. The full path keeps that directory for the Task evidence commands, and the ONLY path never recreates it, even for `ONLY=...St57UpdateQueryTest`. Fix: recreate the st57-update copy when ONLY is St57UpdateQueryTest, or document that an ONLY run wipes the earlier evidence.
- 成果物: tools/android-emulator.sh:121
- 根拠: A developer runs the full suite, then an ONLY run to check one class. The ST57 evidence that the Task verification commands read under `connected/st57-update` is gone and those checks fail.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: design D3 と code-verify 手 6 は、ONLY が前回の結果（`st57-update` を含む）を消す代償を受け入れている。全体を走らせ直せば再生成され、ONLY の単発実行に別 Story の証跡を保存する責務はない。
- 判定: ADDRESSED（re-review 1 @4f1c6925f30ed4c02fe54c9ba6d4657bbb554299）

## R11. The key-to-Scenario table in the file header was not updated. It lists the old keys only, while 13 new `only-*` keys were added to the `ASHIATO_TEST_ONLY` list and the run lines. Fix: add a `only-empty | ...` through `only-st56-fail | ...` row for each new key.
- 成果物: tools/test-android-emulator.sh:3
- 根拠: Anyone reading the header (or a tool that maps keys to Scenarios from it) cannot find the new ONLY cases.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.2 — ヘッダの鍵と Scenario の表に 13 件を追加した。
- 判定: ADDRESSED（re-review 1 @4f1c6925f30ed4c02fe54c9ba6d4657bbb554299）

## R12. `assert_only_base` runs `unset ASHIATO_TEST_SCRIPT_REV` without restoring the caller's value. When the whole file runs, every case after `only-base` silently switches from the caller's script revision to the current script. Fix: save the previous value and restore it afterwards, or scope the assignment, for example `ASHIATO_TEST_SCRIPT_REV=$base new_tree`, together with a local variable.
- 成果物: tools/test-android-emulator.sh:582
- 根拠: `ASHIATO_TEST_SCRIPT_REV=<old> tools/test-android-emulator.sh` tests the old script for the cases before only-base and the current script for only-class through only-st56-fail. The results mix two script versions.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1 — 呼び出し元の `ASHIATO_TEST_SCRIPT_REV` の有無と値を保存し、base の木を作った後に復元する。
- 判定: ADDRESSED（re-review 1 @4f1c6925f30ed4c02fe54c9ba6d4657bbb554299）

## R13. `run_usage` does not strip `ONLY` from the environment. `run_emulator` and `assert_term_cleanup` now do (`env -u ONLY`). If the caller has `ONLY` exported, as AGENTS.md now suggests, it leaks into the usage-volume cases. Fix: add `-u ONLY` to the `env` call in `run_usage` for consistency.
- 成果物: tools/test-android-emulator.sh:381
- 根拠: A developer exports `ONLY=dev.ashiato.collector.X` in their shell and then runs `tools/test-android-emulator.sh`. The usage-* cases run with ONLY set, which differs from the isolation the other cases get. It is harmless today only because usage-volume.sh does not read ONLY.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1 — `run_usage` も `env -u ONLY` で呼び出し元の絞り込みを外す。
- 判定: ADDRESSED（re-review 1 @4f1c6925f30ed4c02fe54c9ba6d4657bbb554299）

## settle_review（re-review 1 @4f1c692）: 処置の直しが新しく壊したもの

## R14. 偽 gradle が XML を書かないとき androidTest-results が存在せず、keep_stage の find が存在しないディレクトリで rc=1 になり set -e で終わる。error メッセージの前に落ちるので assert_only_no_xml は常に偽になる。self-test は改変していない木で偽になることを期待しているため、空振りで緑になる。空の段で grep のガードが働くことは試験で守られていない（本体の grep -r は正しい）。
- 成果物: tools/test-android-emulator.sh
- 根拠: tools/test-android-emulator.sh:676-683（assert_only_no_xml）と self-test の呼び出し（557）（re-review 1 の New Breakage。Important）
- kind: technical
- 処置: fixed 2.2 — 偽 Gradle は XML を省く場合も結果ディレクトリを作る。`only-zero` は XML 無しでも指定の error で rc=1 になることを確認し、self-test は grep ガードを外した写しで `assert_only_no_xml` が失敗することを確認する。
