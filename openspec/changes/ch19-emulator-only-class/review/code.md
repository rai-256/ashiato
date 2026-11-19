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

## R2. 「アニメーションの設定 3 つ」の Scenario は 1 つしか見ておらず、gradle より前であることも見ていない
- 成果物: tools/test-android-emulator.sh（`assert_only_prep`）/ specs/emulator-startup/spec.md
- 根拠: spec の THEN は「アニメーションの設定 3 つ（…）が届き、写真と動画を入れる台本が 1 回呼ばれ、どちらも gradle の計測テストより前」。`assert_only_prep`（`tools/test-android-emulator.sh:608-616`）は `window_animation_scale` の 1 つが在ることと、`media-fixtures` の行番号が gradle の行より前であることだけを見る（アニメーションの行の順は比べていない。読んだだけ）。写しの台本で `for key in` を `window_animation_scale` だけにすると、`ASHIATO_TEST_ONLY=only-prep` は rc=0（落ちない）
- kind: technical
- 提案: 3 つの鍵をそれぞれ grep し、それぞれの行番号が `connectedDebugAndroidTest` の行より前であることを見る（`media-fixtures` と同じ形）

## R3. 環境に `ONLY` が残っていると、全体の検証コマンドの証跡が 1 クラスの実行のまま PASS になり、記録からは見分けられない
- 成果物: tools/android-emulator.sh / AGENTS.md（`scripts/evidence.py` は harness2 側）
- 根拠: 台本は `ONLY` を環境から読む（`tools/android-emulator.sh:21`・`120`）。`scripts/evidence.py:419` と `:480` は検証コマンドを `{**os.environ, …}` で起こすので、呼んだ側が `ONLY` を export していれば `tools/android-emulator.sh`（全体のコマンド）が ONLY の経路を通る。その経路は rc=0 で終わる（only-class が rc=0、本物でも rv1 が通った）。証跡の記録の `env` は `kvm`・`java_home`・`android_home` だけで（`evidence.jsonl` の 4.2 の行）、`command` も全体と同じ文字列になる。違いが残るのは summary の `stage1` と `stage1〜8` の差だけ。AGENTS.md は「絞った実行は Task の完了の証跡にしない」と書くが、それを機械では止めていない。CH19 の試験側は `CASE` に改めて漏れを防いだ（`ONLY='bad value'` を置いて `tools/test-android-emulator.sh` と `--self-test` がどちらも rc=0 を実測。spec-review R8）が、台本の利用者の側の漏れは開いている
- kind: technical
- 提案: ONLY の経路の終わりの行（`== 結果`）の前に、標準エラーへ `== 絞った実行（ONLY=…）。全体ではない` を出す（ONLY の塊の中なので、全体の経路の行は変わらない）。あわせて harness2 の `evidence.py` が `ONLY` を記録の `env` に残すか外すかを決める（harness2 #271 の絞った検証と一緒に。ここでは提案だけ）

## R4. 「アプリを外し、止めて、権限を戻す」の順の Scenario は、先頭の命令が無くても通る
- 成果物: tools/test-android-emulator.sh（`assert_only_order`）
- 根拠: `assert_only_order` の awk（`tools/test-android-emulator.sh:620-625`）は、見つからなかった行の番号を 0 のまま `u<a && a<f && f<p && p<g` で比べる。写しの台本から `adb uninstall dev.ashiato.collector.test >/dev/null 2>&1 || true` を消すと、`ASHIATO_TEST_ONLY=only-order` は rc=0（落ちない）。spec の THEN は `uninstall dev.ashiato.collector.test` が命令の記録にあることを主張している。この行は全体の経路と共有なので、PR の証跡の `only-base` では落ちるが、Scenario の印の先のテストは見ていない
- kind: technical
- 提案: awk の END で 5 つの番号がすべて 0 でないことも条件にする（`u && a && f && p && g && u<a && …`）。`--self-test` に、この行を消すと `only-order` が落ちることを足す
