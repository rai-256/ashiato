<!-- 処置の判定: re-review -->
# code-verify — st61-auto-version-bump（HEAD 2d56b68ba、base e72125187）

## 申告と実測

申告: 18/18（`- [x]` は 1.1〜11.1 の 18 項目。12.1〜12.5 は「人間の確認待ち」で `[ ]` のまま、ここでは対象外）

| 項目 | 検証コマンド（tasks.md の本文のとおり） | 申告 | 実測 |
|---|---|---|---|
| 1.1 | `tools/release/version.sh --self-test` / `tools/release/version_test.sh` / `code 0.2.0=20000・1.0.0=10000000・0.1.10000 は落ちる` | rc=0 | 一致（3 本とも rc=0） |
| 1.2 | `tools/check-version.sh --self-test` / `tools/check-version.sh` | rc=0 | 一致 |
| 2.1 | `cargo test -p ashiato-build-stamp st61_` ＋ `--list` ≥3 | rc=0 | 一致（`st61_` は 4 本） |
| 2.2 | `:app:verifyVersionCodeTable :app:verifyVersionCheckTable` ＋表 ≥10 行 / `check-artifacts.sh --self-test-gradle-range` / `0.4.0-beta.2` で assembleRelease → `apk-version` | rc=0 | rc は一致。ただし 3 本目は 1 回目に `versionName='0.0.1-dev.20261102'` の APK を検査して rc=0（R4） |
| 2.3 | `vitest run src/__tests__/build-stamp.test.ts` / `npm run build` | rc=0 | 一致 |
| 2.4 | `tools/check-openapi.sh` / `check-build-stamp.sh release` / `check-build-stamp.sh android` | rc=0 | 一致 |
| 3.1 | `version.sh --self-test` / 2 つの Requirement の Scenario 名の数え ≥20 | rc=0 | 一致（n=20） |
| 4.1 | `tools/release/number.sh --self-test` | rc=0 | 一致 |
| 4.2 | `check-artifacts.sh --self-test` | rc=0 | 一致 |
| 4.3 | `check-released-migrations.sh --self-test` / `check-contract-version.sh --self-test` | rc=0 | 一致 |
| 5.1 | `check-guards.sh --self-test` / `check-guards.sh workflows` / actionlint number.yml | rc=0 | 一致。`--self-test` は長い検証と同時に走らせた 1 回目だけ rc=1（`本物の ci.yml が 0.0.x の検査を通らない`）で、単独で 2 回走らせると rc=0。再現しないので指摘にしない |
| 6.1 | `changes.sh --self-test` / `check-guards.sh workflows` / actionlint ci.yml | rc=0 | 一致（ただし ci.yml の配線を元に戻しても 3 本とも rc=0。R1） |
| 7.1 | `check-guards.sh --self-test` / `workflows` / 0.4〜0.6 の版が無い grep / `update-apply-install.sh --list` ≥34 | rc=0 | 一致 |
| 8.1 | `version.sh --self-test` / `check-no-release-branch.sh`（訂正後） | rc=0 | 一致 |
| 9.1 | `version.sh --self-test` | rc=0 | 一致 |
| 9.2 | `publish-nightly.sh --self-test` / `check-guards.sh --self-test` / `workflows` / actionlint nightly.yml | rc=0 | 一致 |
| 10.1 | `tools/release/draft.sh --self-test` | rc=0 | 一致 |
| 11.1 | 手順書の grep / `python3 scripts/check_scenarios.py . st61-auto-version-bump` | rc=0 | 一致（`scenarios: OK`） |

actionlint はこの機械に無く、3 本とも docker の `rhysd/actionlint:1.7.7` で rc=0。

## 手ごとの結果

- **手 1（固定値を独立に再計算）**: `version-table.tsv` の 16 行を python で `左×10,000,000＋真ん中×10,000＋右`（上限 209・999・9,999）から計算し直し、数の行は 13 行とも一致した。拒む 3 行（`-beta.99`・`-beta.0`・`-beta.+5`）は形の規則（spec 472 行: N は 1〜98）で拒まれ、`version.sh code` も rc=1。
  `version-check-table.tsv` の 17 行も、spec 472-473 行から python で書いた判定と全行一致した。前の式の `0.1.1`＝10199（`0*1e6+1*1e4+1*100+99`）も独立に一致。配布元（origin = `rai-256/ashiato`）の印 `v*` は 0 個（`git ls-remote --tags origin 'v*'` rc=0・0 行）。印 `v*` で配った APK に、20000 より大きいものは無い。
  表の外の値: 先頭の 0（`0.02.0`）を受けるのは base からの決め（version.sh:20 の /code-review R36）なので、指摘にしない。
- **手 2（ガードを壊す）**: 使い捨ての worktree（片付け済み）で 1 か所ずつ壊し、各検査の rc を見た。
  - `version.sh number`: 確認バッチの判定・文書だけの判定・reuse・祖先・上限・最初の 0.2.0・浅い木・左を上げる・左を下げる（9 個）は自己試験が rc=1 で捕まえた。**main の first-parent の判定（193 行）を消しても rc=0**（R5）
  - `number.sh`: 12 個のうち 9 個は rc=1。**下書きでない release への置き（95 行）・`complete` の下書きの検査（118 行）・手元の印のずれ（49 行）を消しても rc=0**（R5）
  - `check-guards.sh workflows`（number.yml）: 条件 3 つ（push・成功・この repo）・`cancel-in-progress`・`fetch-depth`・`ref` 3 つ（判定・build-linux・build-windows）・権限・cache・unsigned・`branches`・`workflows`・`needs` 2 つ・`gh release edit` の 16 個は rc=1。**`gh release create v<番号>` や `git tag v… && git push` の step を足しても rc=0**（R2）。
    ほかに rc=0 だったのは 6 個: `ASHIATO_RELEASE: "0"`・組み立ての `ASHIATO_BUILD_VERSION: "0.0.1"`・`complete` の条件を外す・印を作る呼び出しを止める・`place` の `needs` を `[number]` にする・`place` に渡す commit を `$(git rev-parse HEAD)` にする（最後のものは checkout が同じ commit なので同値）。実行時に `place`（`check-artifacts.sh --expect-version`・印の有無）が拾うかは、走らせていない
  - 検査の対象を空にする: workflow 4 つだけの写しを `CHECK_GUARDS_ROOT` に渡すと `workflows` は rc=0。ci.yml の更新の試験の `for spec in …` の変数名を変える・版を `"$@"` にして 1 つも拾えなくすると、どちらも rc=1（0 件で緑にならない）
  - `check-guards.sh workflows`（ci.yml）: 更新の試験の版（`0.4.1`）・配る形の job に渡す `0.2.0` は rc=1。**D8 の配線（`BASE`・`actions: read`・`status=success`・前の run を探す段）は、どれを壊しても rc=0**（R1）
  - 画面（`build-stamp.ts`）: 上限（真ん中を広げる）・左の不一致・`0.0`・beta の範囲の 4 個は vitest が rc=1。MAJOR の場所を変えても vitest は rc=0 だが、`check-build-stamp.sh` の 226 行が本物の MAJOR で組むので、そちらが拾う
  - 本物の main の履歴の写しで `version.sh number` を走らせた: ST61 を確認バッチで merge→`0.2.0`、製品→`0.2.1`、文書だけ→付けない、確認バッチ→`0.3.0`。本物の確認バッチ 28 件の題は全部 `Merge pull request #N from rai-256/verify/<日時>` で、D15 の形と合う
- **手 3（Scenario と test）**: `check_scenarios.py` は `scenarios: OK`。印の先を読んだのは、ADDED の新しい Scenario のうち番号の判定・置き場・workflow・差分の基準・versionCode のもの。階層が違うものは R1（spec は「コードの job と配る形の組み立ての job は走る」、試験は `changes.sh base` が commit を返すところまで）と R2（spec は「公開する手順と印 `v*` を作る手順を持たない」、検査は文字の並び 7 つの禁止）。
  `最初の番号は、前の式の 0.1.x の APK より大きい`・`番号を付けても main の commit は増えない`・`署名していない APK は、debug の鍵の APK と同じ組み立て` は、主張と同じ階層で観測している。
  MODIFIED の 8 つの Requirement（spec 579〜915 行）は、Scenario の名前と版を python で抜き出し、`update-apply-install.sh` と並べた。印のある 28 件は、版の振り分け（694-702 行: 前 `0.0.2`→新 `0.0.3`・開発版 `0.0.2-dev.20261115`→`0.0.3-dev.20261116`・知らない版と系統違いは互換の `0.0.4`→`0.0.5`）が spec の版と合う。7.1 が名指しした `DB に新しい版の知らない版があると、戻さずに前の版のまま起きる` は 702 行で `0.0.4`→`0.0.5` に振られる。`update-apply-install.sh` に印の無い 9 件（spec の版が `0.4.0`・`0.5.0` のまま）は、別の試験の印の先を読んでいない
- **手 4（本人の決定）**: Q1（CI が通り製品が変わった commit）・Q3（確認バッチで真ん中）・Q4（0.0.x。`Cargo.toml` を `0.1.0` にすると `check-version.sh` が rc=1、ci.yml の更新の試験の版を `0.4.1` にすると `workflows` が rc=1）・最初の `0.2.0` は、値を変えると落ちる。
  Q2 の掛け算（`10000000`→`1000000`）は自己試験が rc=1。**Q2 の上限（真ん中 999・左 209）は狭めても全部通る**（R3）
- **手 5（`[x]` と実体）**: 18 項目の検証コマンドは全部実在し、rc=0（上の表）。`cargo test --test <名前>` の型は無い。2.2 の 3 本目は R4
- **手 6: 該当なし**（確かめた範囲: 走らせたもの —— `version.sh number` を本物の main の写しで走らせ、取り消された・文書だけの merge の後も番号が子孫に入ることを見た。読んだだけのもの —— 次の 2 つ）。番号の組み立ての消し方と印 `build-*` の守りは、`docs/handoff/ST62.md` の `## st61-auto-version-bump R3` にあり、`scripts/story_brief.py` が `docs/handoff/` を ST62 の上流へ渡す。`docs/stories/ST62.md` の本文には、まだ写っていない（「下書き」「build-」「消し」は 0 件）。defer の指摘は要らない。
  取り消された merge の中身を次の番号へ入れる経路は、`version.sh number` の側は自己試験（`取り消された merge の中身は、後の文書だけの merge の番号に入る`）が持つ。CI の側の配線は R1
- **手 7**: 訂正は 1 件（8.1）。`correction-review.md` に書いた。base の修正の枝の例外を `plan()` に戻すと、新しいコマンド `check-no-release-branch.sh` は rc=1（落ちた）、同じ壊し方で `version.sh --self-test` も rc=1。戻した後の `git status --porcelain`・`git diff` は前と同じ

## R1. main への push の CI の差分の基準（D8）を ci.yml に配線したことを、どの検査も見ていない。元の `github.event.before` に戻しても 6.1 の検証は全部緑のまま
- 成果物: .github/workflows/ci.yml（24 行の `changes` の job の `permissions`・36 行の `id: prev` の段・48 行の `BASE`）/ tools/release/changes.sh / tools/release/check-guards.sh
- 根拠: 複製の ci.yml で `BASE: ${{ github.event.pull_request.base.sha || github.event.before }}` に戻し、`id: prev` の段を消した。6.1 の検証 3 本はどれも rc=0（`changes.sh --self-test` rc=0・`check-guards.sh workflows` rc=0・actionlint rc=0）。
  ほかに `permissions: { contents: read }`（`actions: read` を外す）・`status=success` を外す（取り消された run の commit が基準になる）・`if: false` で前の run を探す段を止める、の 3 つも `workflows` が rc=0。`check-guards.sh` に `prev`・`PREV_SUCCESS`・`changes.sh base` を見る行は 0 件（grep）。
  Scenario `取り消された run の後の文書だけの merge でも、コードの job が走る`（spec 242 行。THEN は「コードの job と配る形の組み立ての job は走る」）の印は `changes.sh:52` にあり、観測しているのは `changes.sh base` が渡した commit を返すことだけ。ci.yml がそれを `BASE` に使うかは見ていない
- kind: technical
- 提案: `check-guards.sh workflows` に、ci.yml の `changes` の job が push で `changes.sh base` の出力を `BASE` に使うこと・`actions: read` を持つこと・問い合わせが `status=success` を持つことの検査を足し、`--self-test` で 1 つずつ外した偽の ci.yml が rc≠0 になることを見る。12.4（人間の確認）は main に入った後でしか働かない
- 処置: fixed 6.1
- 判定: ADDRESSED（re-review 1 @7054109bb4551bd3fb778dcff2cc64fec3c7775f）
- 判定: ADDRESSED（re-review 2 @e8c8db1ac2f8ed27e0ed3dd56c2900ece8fb7156）
- 判定: ADDRESSED（re-review 3 @c862999539b427f4d46416a01d349dbb67f2cffa）
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R2. 番号付けの workflow の検査は、公開と印 `v*` を作る step を、文字の並び 7 つの禁止でしか見ていない。`gh release create v<番号>` を足しても通る
- 成果物: tools/release/check-guards.sh:162 / .github/workflows/number.yml
- 根拠: 複製の number.yml の `place` の job に `- run: gh release create "v${{ needs.number.outputs.number }}" --target "${{ github.event.workflow_run.head_sha }}" number/all/*`（下書きでない＝公開して印 `v*` を作る）を足すと、`check-guards.sh workflows` は rc=0。
  `- run: git tag "v${{ … }}" && git push origin "v${{ … }}"` を足しても rc=0。`gh release edit … --draft=false` を足したときは rc=1。
  検査は `gh release (edit|delete)|--draft=false|--clobber|publish-nightly|refs/tags/v|sign-apk` の grep だけ（162 行）。Scenario `番号付けの workflow は、前の組み立てを使い回さず、書く権限をその job にだけ持つ` の THEN は「公開する手順と印 `v*` を作る手順を持たない」
- kind: technical
- 提案: 禁止の並びを足すより、`contents: write` の 2 つの job の `run:` を許す形（`tools/release/version.sh number`・`number.sh tag|complete|place`・決まった `echo`/`sed`/`tee`）に限る。`gh` と `git tag`/`git push` を直に呼ぶ step があれば落とす
- 処置: fixed 5.1
- 判定: ADDRESSED（re-review 1 @7054109bb4551bd3fb778dcff2cc64fec3c7775f）
- 判定: ADDRESSED（re-review 2 @e8c8db1ac2f8ed27e0ed3dd56c2900ece8fb7156）
- 判定: ADDRESSED（re-review 3 @c862999539b427f4d46416a01d349dbb67f2cffa）
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R3. versionCode の上限（真ん中 999・左 209）は、狭めても試験が 1 本も落ちない。受ける側の境目の行が、どの表にも無い
- 成果物: tools/release/version-table.tsv / tools/release/version-check-table.tsv / tools/release/version.sh:42-43 / web/build-stamp.ts:59 / crates/build-stamp/src/lib.rs:79-81 / collector-android/app/build.gradle.kts:44-45
- 根拠: 複製で `version.sh:43` の `MINOR <= 999` を `<= 998` にすると `version.sh --self-test` rc=0。`:42` の `MAJOR <= 209` を `<= 100` にしても rc=0。
  `build-stamp.ts` の `LIMITS` を `[209, 998, 9999]`・`[100, 999, 9999]` にしても vitest rc=0（右を `9998` にしたときは、表の `0.250.9999` の行で rc=1）。
  上限ちょうどを受ける値（`0.999.x`・`209.x.x`）は、表 2 つ・`lib.rs`・`build-stamp.test.ts`・`build.gradle.kts`・`version_test.sh` に 0 件（grep。`version.sh:325` の `0.999.4` は印の名前で、`version_code` を通らない）。
  Rust と gradle は表の行だけを当てるので同じ穴があるはずだが、壊して走らせてはいない（grep で行が無いことだけを見た）。
  上限が狭まると、番号付けは届く前に「上限を超える番号は付けない」で止まる（D14）
- kind: technical
- 提案: `version-table.tsv` に `0.999.9999`→9999999 と `209.999.9999`→2099999999 を、`version-check-table.tsv` に `0.999.9999`（左 0）ok と `209.0.0`（左 209）ok を足す。4 か所が同じ表を読むので、1 か所の変更で済む
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @7054109bb4551bd3fb778dcff2cc64fec3c7775f）
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R4. 2.2 の 3 本目の検証は、組んだ APK が渡した版番号かを見ていない。版を無視した APK でも rc=0 になる（実測 2 回）
- 成果物: openspec/changes/st61-auto-version-bump/tasks.md 2.2 の 3 本目 / tools/release/check-artifacts.sh:65-87（`apk-version`）
- 根拠: (1) 長い検証の中で走らせた 1 回目は rc=0 で、出力は `OK: versionCode='1' versionName='0.0.1-dev.20261102'`（`.superpowers/sdd/logs/20261123-215428-apk.log` と `$TMPDIR/cv/apk.log`）。渡したのは `0.4.0-beta.2` で、置き場の APK は別の組み立てのものだった（何が置いたかは突き止めていない。単独で 2 回走らせ直すと `versionCode='40000' versionName='0.4.0-beta.2'`）。
  (2) 複製で `build.gradle.kts` の `val buildVersionInput = buildVersionProvider.get().trim()` を `= ""` にし（`ASHIATO_BUILD_VERSION` を無視する）、同じコマンドを走らせると rc=0 で、出力は `OK: versionCode='1' versionName='0.0.1'`。
  `apk-version` は `--expect-version` が無ければ APK 自身の versionName から計算した値と比べるだけ（78-83 行）。85 行の 40000 の検査は versionName が `0.4.0-beta.2` のときしか働かない。この型は code_verify R25 で一度直した弱さと同じ（79 行の注記）
- kind: technical
- 提案: 2.2 の 3 本目を `tools/release/check-artifacts.sh apk-version --expect-version 0.4.0-beta.2 collector-android/app/build/outputs/apk/release/*.apk` に直す（受け入れ条件は変えない。`plan_fix.py` の訂正の形）。あわせて、組む前に `app/build/outputs/apk/release/` を空にして、ほかの組み立ての残りを拾わないようにする
- 処置: fixed 2.2
- 判定: ADDRESSED（re-review 1 @7054109bb4551bd3fb778dcff2cc64fec3c7775f）
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）

## R5. 置き場と番号の判定の守り 4 つに、試験が無い。消しても自己試験は通る
- 成果物: tools/release/number.sh:95・:118・:49 / tools/release/version.sh:193
- 根拠: 複製で 1 か所ずつ消し、自己試験の rc を見た（下の「消えると」は、コードを読んだ見立てで、消した状態で走らせてはいない）。
  - `number.sh:95`（`build-<番号>` が下書きでなければ置かない）を `|| true` にすると、`number.sh --self-test` rc=0。消えると、公開済みの release に署名していない APK を足す（spec 146 行「下書きは公開せず」の外へ書く）
  - `number.sh:118`（`complete` が下書きでなければ `complete=no`）を消すと rc=0。消えると、公開済みの release を「揃っている」と読み、組み立てと置く job を飛ばす
  - `number.sh:49`（手元の印 `build-<番号>` が別の commit を指せば止まる）を `|| true` にすると rc=0。消えると、手元の印のまま別の commit を push する
  - `version.sh:193`（main の first-parent の上に無い commit には付けない。design D4 の 2.）を `true ||` にすると `version.sh --self-test` rc=0
  - 見立て（走らせていない）: 本物の CI では、49 行と 193 行に当たる状態はほぼ起きない（新しい checkout・main の push の run だけ）。95 行と 118 行は、人が下書きを公開した後に番号付けが走り直したときに当たる
- kind: technical
- 提案: `number.sh --self-test` の偽の `gh` に `isDraft:false` の release を返させ、`place` が rc≠0 で書く要求 0 件・`complete` が `complete=no` になることを見る。`version.sh` の自己試験に、前の番号の子孫で main の first-parent の上に無い commit（merge された枝の中の commit）を足す
- 処置: fixed 4.1
- 判定: ADDRESSED（re-review 1 @7054109bb4551bd3fb778dcff2cc64fec3c7775f）
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## code-verify の結論
- 結論: 申告と実測は不一致（5 件。R1・R2・R3・R4・R5）。18 項目の検証コマンドは全部実在し rc=0 だったが、ci.yml の差分の基準の配線（R1）・番号付けの workflow の公開と `v*` の禁止（R2）・versionCode の上限（R3）・APK の版の検証（R4）・置き場の下書きの守りなど 4 つ（R5）は、壊しても検査が落ちない
- 確かめた範囲: tasks 1.1〜11.1 の検証コマンドを全部実行した。対応表 2 つと前の式の値を python で独立に計算した。version.sh・number.sh・number.yml・ci.yml・build-stamp.ts の守りを 70 か所ほど複製で壊して rc を見た。本物の main の履歴の写しで番号の判定を走らせた。check_scenarios と印の先を突き合わせた。handoff/ST62 の受け渡しを確かめた。手 7 は 8.1 を作業ツリーで壊して戻した
- 見ていない範囲: number.yml を壊して `workflows` が rc=0 だった 6 個（環境変数・`complete`・印の呼び出し・`place` の `needs`）を、実行時に `place` が拾うか。MODIFIED のうち `update-apply-install.sh` に印の無い 9 件の印の先。配布元（GitHub）での実際の動き（P18〜P20、job の単位の concurrency・`gh api` の run の並び・GITHUB_TOKEN での印の push。12.1〜12.5）。Windows でしか走らない更新の試験（MODIFIED の 8 つの Requirement の `update-apply-install.sh` の本体。`--list` の本数だけを見た）。Rust と gradle の判定を壊す試験（表の行を grep しただけ）。`check-guards.sh --self-test` の 1 回目の rc=1 の原因

## security review（e721251..ca09d0e）

見た経路:
- main への push の CI 成功 → `number.yml`（`workflow_run`）の `if`（`event == 'push'`・`conclusion == 'success'`・`head_repository == github.repository`。number.yml:14）→ `contents: write` は `number` と `place` の job だけ、組み立ての job は workflow の `contents: read` のまま・`persist-credentials: false`（2）
- `version.sh number <head_sha>` → main の first-parent の上に無い commit は `number=`（version.sh:193）→ `number.sh tag` が番号の形（`^[0-9]+\.[0-9]+\.[0-9]+$`）と 40 桁の commit を確かめてから `git push refs/tags/build-<番号>`（number.sh:16-25）（4）
- `${{ … outputs.number }}`・`head_sha` の `run:` への埋め込み → 番号は `build_numbers` の正規表現を通った印の名前か計算した数字だけ、head_sha は GitHub の hex（3）
- 印 `build-*` の push → 起動する workflow は無い（`release.yml` は `tags: ['v*']`、GITHUB_TOKEN の push はそもそも workflow を起こさない）（2）
- `number.sh place` → 下書き（`--draft`）にだけ置き、下書きでなければ止まる（number.sh:95）。署名していない・debug の鍵の APK は下書きにだけ載り、公開は人の手（2）
- nightly の `plan` → `commit=` は main の first-parent の上の `build-*` の印の commit だけ（version.sh:145）→ 署名の鍵（secrets）を持つ `publish` の job はその commit を checkout し、`publish-nightly.sh` が 40 桁の hex でなければ落ちる（4）
- ci.yml の `changes` → `gh api` の前に成功した main の push の run の `head_sha` → `changes.sh base` が `git cat-file -e` で木にあるときだけ返す → `BASE`。`actions: read` を足しただけで書く権限は無い（3）
- ci.yml の更新の試験の組み立て → GITHUB_TOKEN の extraheader の組み直しは書き方の変更だけで、出力・ログへの経路は前と同じ（2）
- `build-stamp`（Rust・web・gradle）→ 読むのは `tools/release/MAJOR`・`Cargo.toml`・`ASHIATO_BUILD_VERSION` だけで、記録の中身・端末の値は入らない（2）

安全の決め手: 書く権限か署名の鍵を持つ job が checkout して走らせる commit は、main の first-parent の上の commit に限られる（number は version.sh:193、nightly は version.sh:145）—— 確かさ 4。使い捨ての木で、main の外の commit に大きな番号の印 `build-9.9.9` を置いても `nightly-plan` は main の上の `build-0.2.0` の commit を返し、main の外の commit に `version.sh number` は `number=`（付けない）を返した。`number.sh tag '1.2.3;id' HEAD` は「番号の形が違う」、`number.sh tag 1.2.3 'HEAD;id'` は「commit が無い」で rc=1
見ていない範囲: 配布元（GitHub）での実際の動き（`workflow_run` の `branches` の絞りと `head_repository` の値、下書きの release の見える範囲、job の単位の concurrency）は走らせていない。`build.sh` の中身（差分は版番号の受け渡しだけ）は辿っていない

指摘なし

## /code-review（e721251..481b4cf）

見た: Claude Code の組み込みの `/code-review`（5 件）。写しは scripts/code_review_record.py

## R6. On Windows runners, any build that is given a version number will likely fail with 「左の数字のファイルが数字 1 つでない」. `major_file` reads `tools/release/MAJOR` with `$(cat …)`, which strips only the trailing `\n`. A CRLF checkout leaves `0\r`, and `^[0-9]+$` rejects it. `build.sh resolve_version` calls `version.sh check`, which now always calls `major_file`; the Rust, TS and gradle versions all trim the value. Fix: strip `\r` and surrounding whitespace before the regex, or add `tools/release/MAJOR text eol=lf` to `.gitattributes`.
- 成果物: tools/release/version.sh:33
- 根拠: windows-latest checks out with core.autocrlf (build.sh:46 already notes the Windows checkout can be CRLF), so MAJOR becomes "0\r\n". Then number.yml build-windows runs `build.sh server-windows` with ASHIATO_BUILD_VERSION=0.2.1, and resolve_version runs `version.sh check`, which dies. The same happens in nightly package-windows (0.x.y-dev.*) and in the ci.yml package-windows loop (ASHIATO_BUILD_VERSION=$v). No Windows artifacts are produced.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @7054109bb4551bd3fb778dcff2cc64fec3c7775f）
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R7. On a main push, the CI diff base can be a commit that is not an ancestor of HEAD, so code jobs get skipped. `base()` only checks that PREV_SUCCESS_SHA exists in the tree, not that it is an ancestor of HEAD. Fix: in base(), also require `git merge-base --is-ancestor "$sha" HEAD`, and fall back to `all` when that fails.
- 成果物: tools/release/changes.sh:24
- 根拠: C1 (a code change) fails on main and C2 (docs only) succeeds. Re-running C1's run picks the newest successful run, C2, as BASE. `git diff C2 C1` then lists only the docs, so code=false and the Rust, e2e and Windows jobs are skipped. C1's run turns success even though C1's own code was not tested here, and number.yml then fires for C1. The same thing happens when the newest success points at a commit that was force-pushed away.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 6.1
- 判定: ADDRESSED（re-review 1 @7054109bb4551bd3fb778dcff2cc64fec3c7775f）

## R8. The bash `check_rule` does not enforce the same rule as Rust, TS and gradle, even though the comments say all four apply the same rule and the same table pins it. bash only checks that the source version has no suffix. The other three reject any source version that is not `0.0.<N≥1>`. Every row of version-check-table.tsv uses source 0.0.1, so the table cannot catch the difference. Fix: add the `0.0.<N≥1>` check to check_rule, and add table rows whose source is not 0.0.x (for example 0.4.0 with 0.4.0 → ng).
- 成果物: tools/release/version.sh:111
- 根拠: With source 0.4.0 and given version 0.4.0, `version.sh check` returns ok, while build-stamp, build-stamp.ts and gradle all fail. version.sh's own self-test passes only because of this gap: its tree's Cargo.toml is 0.4.0 and `plan v0.4.0` depends on it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 1 @7054109bb4551bd3fb778dcff2cc64fec3c7775f）
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R9. Only Rust accepts a MAJOR file containing `+0`. `text.trim().parse::<u64>()` allows a leading `+` (u64::from_str does), while bash, TS and gradle all require `^\d+$`. That is the same trap that parse_version guards against by checking for digits only (final review R12). Fix: check that the value is ASCII digits only before parsing.
- 成果物: crates/build-stamp/src/lib.rs:68
- 根拠: If tools/release/MAJOR contains "+0", version.sh, the web stamp and gradle all fail, but the Rust server and collector build with major=0. The four implementations then disagree about whether the build is valid.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 1 @7054109bb4551bd3fb778dcff2cc64fec3c7775f）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R10. The branch has a commit whose message is just `wip`. CLAUDE.md (git section) says: 「コミットは `feat:` / `fix:` / `docs:` / `chore:` + 日本語 1 行、本文に「なぜ」」. Fix: reword the commit to the project's format, or squash it into a neighbouring commit.
- 成果物: openspec/changes/st61-auto-version-bump/tasks.md:1
- 根拠: `git log origin/HEAD..HEAD --format=%s` includes a subject that is exactly "wip". It has no type prefix, no Japanese one-line summary and no body explaining why, which breaks the /home/yosis/dev/ashiato2-st61/CLAUDE.md commit rule.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: escalated — `deep.md` 第 2 回 Q5（loss: discarded）。問いは `deep-questions-r2.json` / `docs/briefs/ST61-deep-r2.html`。**答え（2026-11-27）**: 推奨のまま「そのまま merge する（書き直さない）」→ 58f7a4d8c の題「wip」は main に残す。履歴は書き換えず、成果物も変えない（中身は直後の 53a74e0ce の題で読める）。これからの題を commit-msg の hook で止めるのはこの Story の範囲の外

## settle_review（re-review 1 @7054109）: 処置の直しが新しく壊したもの

## R11. 新しい number_steps は書き方によって公開の step を通す。number.yml の写しに 1 つずつ足して python 部分を走らせると、flow 形式の `- { run: "gh release create v1" }`、複数行の plain scalar の 2 行目の `&& gh release create v1`、`echo a & /usr/bin/gh release create v1`、`find . -maxdepth 0 -exec /usr/bin/gh release create v1 \;` はすべて rc=0（素の `gh release create` は rc=1）。現在の number.yml にはどれも無い。
- 成果物: tools/release/check-guards.sh
- 根拠: tools/release/check-guards.sh:149（re-review 1 の New Breakage。Minor）
- kind: technical
- 処置: fixed 5.1
- 判定: ADDRESSED（re-review 2 @e8c8db1ac2f8ed27e0ed3dd56c2900ece8fb7156）
- 判定: ADDRESSED（re-review 3 @c862999539b427f4d46416a01d349dbb67f2cffa）
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## settle_review（re-review 2 @e8c8db1）: 処置の直しが新しく壊したもの

## R12. shell を step の鍵でしか見なくなり、workflow・job の単位の `defaults.run.shell` を通す。直す前は字面の `shell:` の行を全部見ていた。複製の number.yml に `defaults: { run: { shell: "python {0}" } }`（workflow の単位）か `shell: "gh release create v1 {0}"`（place の job の defaults.run）を足すと、7054109 の版は rc=1、現在の版は rc=0。現在の number.yml に defaults は無い。同じ形で BASH_ENV/ENV の拒否（:204）も step の env だけで、job・workflow の env に置くと rc=0（こちらは直す前も rc=0）。
- 成果物: tools/release/check-guards.sh
- 根拠: tools/release/check-guards.sh:211（re-review 2 の New Breakage。Minor）
- kind: technical
- 処置: fixed 5.1
- 判定: ADDRESSED（re-review 3 @c862999539b427f4d46416a01d349dbb67f2cffa）
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## final review（e721251..60205ce）

final reviewer（whole-branch。`code-reviewer.md`）の指摘。HEAD で `version.sh` `number.sh` `changes.sh` `check-guards.sh`（`--self-test` と `workflows`）・`check-version.sh --self-test`・`draft.sh --self-test`・`check-no-release-branch.sh`・`check_scenarios.py` は rc=0。各 Task の Minor・⚠️ を全体と照合し、直っていたもの（T1 F6・T2 F1・T3 F4/F5・T5 F3・T9 F4）と R10 と同じもの（T4 F4）は外した。配布元の動きに依る ⚠️（`gh release view` の文言・組の動き・Windows の job）は人間の確認 12.1〜12.5 に残す。

## R13. 番号付けの判定の step が落ちても run は緑になる。`number.yml` の `id: decide` は `shell:` を持たず（workflow・job の `defaults` も無い）、GitHub Actions は Linux で `bash -e {0}`（pipefail なし）で走らせるので、`version.sh number … | tee number.txt` の rc は `tee` の 0。左の数字を下げた（D14）・上限を超えた・versionCode に収まらない・MAJOR が壊れた・浅い木・前の印が読めない、の spec が「0 でない終了コードで終わる」と決めた場合がすべて `number=` 空 → 後ろの job を飛ばして success で終わる。`docs/release.md:30` の「止まると番号付けの run が落ち、理由が出る」と食い違う。同じ形（`sh -c 'echo NG >&2; exit 1' | tee x` の後に sed と if）を `bash -e` で走らせると rc=0（final reviewer の実測）。`check-guards.sh` の number_steps は shell を「あれば bash」としか見ず、未指定を通す。直し: decide の step に `shell: bash`（または `set -o pipefail`、パイプをやめて `>number.txt`）、number_workflow に「decide が `shell: bash` を持つ」検査と、外した偽の number.yml で rc≠0 になる mutant を `--self-test` に足す。
- 成果物: .github/workflows/number.yml, tools/release/check-guards.sh
- 根拠: .github/workflows/number.yml:30-35、docs/release.md:30、tools/release/check-guards.sh（number_steps の shell の検査）（final review。Important）
- kind: technical
- 処置: fixed 5.1
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R14. `version.sh number` は `changes.sh code` の rc=1 だけでなく、rc=2・127・141 も「文書だけ」と読み、`number=` を返して exit 0 する（Task 3 F2 が残っている）。台本が無い・壊れているときも黙って番号が付かない（R13 と同じく止まらない）。直し: rc を取り、1 だけを「付けない」、それ以外は die。自己試験に、changes.sh が rc=2 を返す偽で rc≠0 になる Scenario を足す。
- 成果物: tools/release/version.sh
- 根拠: tools/release/version.sh:217（final review。Important）
- kind: technical
- 処置: fixed 3.1
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R15. `number.sh` の `trap "rm -f '$err'" RETURN` は、後ろの die が exit で抜けると走らず、`$TMPDIR/number-gh-err.*` が残る。`number.sh --self-test` 1 回で 2 個残る（final reviewer の実測）。`HARNESS_TMP_LEAK=fail` に当たる恐れ。直し: stderr を変数で受けるか、EXIT の trap で消す。
- 成果物: tools/release/number.sh
- 根拠: tools/release/number.sh:86-87（Task 4 F2）（final review。Minor）
- kind: technical
- 処置: fixed 4.1
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）

## R16. 本番の `apk_version` に試験の固定値の特例 `[ "$version_name" != 0.4.0-beta.2 ] || [ "$version_code" = 40000 ]` が入っている。直前で `version.sh code "$want"` と比べているので冗長で、試験の値が本番の台本に漏れている。直し: 特例を消す（要るなら apk-build の検証の側へ）。
- 成果物: tools/release/check-artifacts.sh
- 根拠: tools/release/check-artifacts.sh:84（final review。Minor）
- kind: technical
- 処置: fixed 2.2
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）

## R17. release・beta の系統でも `*-unsigned.apk` があれば中身を検査して通す。release.yml は `release/all/*` をそのまま公開に上げ、check-guards は release.yml で `ASHIATO_APK_ALSO_UNSIGNED` を禁じていない。今は立てていないので届かないが、D2（仮）の「署名していない APK は下書きにだけ」を機械が守っていない。直し: `--allow-debug-key` の無い検査では `-unsigned.apk` があれば落とす（と自己試験の mutant）。
- 成果物: tools/release/check-artifacts.sh
- 根拠: tools/release/check-artifacts.sh:379-381、design.md D2（final review。Minor）
- kind: technical
- 処置: fixed 4.2
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）

## R18. place の job は push しないのに checkout が `persist-credentials: true`（Task 5 F4）。書く権限の token を作業木に残す必要が無い。直し: `false`。
- 成果物: .github/workflows/number.yml
- 根拠: .github/workflows/number.yml:107（final review。Minor）
- kind: technical
- 処置: fixed 5.1
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）

## R19. `version-check-table.tsv` に左が 1 の行は `1.0.0 ok` だけ。「左 1 で番号を持たない `0.0.1` が通る」「左 1 で `0.2.0` を断る」行が無く、MAJOR を上げた後に 4 か所のどこかが誤って断る・通す実装になっても表で捕まらない（Task 1 F4 の一部）。直し: その 2 行を足す。
- 成果物: tools/release/version-check-table.tsv
- 根拠: tools/release/version-check-table.tsv（final review。Minor）
- kind: technical
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R20. gradle の表の検証で `columns[2].toLong()` が `runCatching` の中にあり、3 列目が壊れた ng の行も「断った」として通る（Task 2 F2）。形の誤りの文が出所と MAJOR を含まない（Task 2 F3）。直し: 表の列の読み取りを `runCatching` の外に出す。
- 成果物: collector-android/app/build.gradle.kts
- 根拠: collector-android/app/build.gradle.kts:444（final review。Minor）
- kind: technical
- 処置: fixed 2.2
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R21. number_workflow の `if:` の検査は 3 つの条件をそれぞれ「含むか」でしか見ず、`||` で結んだ偽の number.yml でも通る（Task 5 F2）。直し: `&&` で結んだ形を見るか、条件を分けて読む、と `||` の mutant。
- 成果物: tools/release/check-guards.sh
- 根拠: tools/release/check-guards.sh（number_workflow の `if:` の検査）（final review。Minor）
- kind: technical
- 処置: fixed 5.1
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R22. 更新の試験の版の検査は `old|new|dev-*|compat-*` の名前の項目しか拾わず、別の名前の項目を足すと見逃す。`0.0.*` は `0.0.0` も通す（Task 7 F2・F3）。直し: 版の形の値を名前に依らず拾い、`0.0.[1-9]*` に限る。
- 成果物: tools/release/check-guards.sh
- 根拠: tools/release/check-guards.sh:331-339（final review。Minor）
- kind: technical
- 処置: fixed 7.1

残りの害の小さいもの（R にしない）: `draft.sh:296` の `build-` の守りは自己試験の中だけで中身が無い（本番は `draft.sh:19` で構造上は安全。T10 F1/F2）・version.sh `major_file` の既定の道の二重（T1 F3）・number() の MAJOR 検査の写し（T3 F3）・「文書だけ」の Scenario が 2 つの merge で組む（T3 F6）・number.sh の偽の check-artifacts が `--expect-version` を照合しない（T4 F3）・ci.yml の `all` の暗黙の番兵（T6 F2）・nightly-plan の出力の形（T9 F3）・docs/release.md に「最初の番号は 0.2.0」が無い（T11 F1）。
- 判定: ADDRESSED（re-review 4 @c667ca2c6110d4140c770dda07ec482fdb16bb2c）
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## final review（e721251..c442010）

final reviewer（whole-branch の 2 回目。`code-reviewer.md`）の指摘。直しの波（60205ce..c442010）の R13〜R22 はどれも効いていて、新しく壊れたものは無かった。HEAD で `check-guards.sh --self-test`・`check-guards.sh workflows`・`version.sh --self-test`・`number.sh --self-test`・`changes.sh --self-test`・`version_test.sh`・`check-version.sh --self-test` は rc=0、TMPDIR に何も残らない。`cargo test -p ashiato-build-stamp` は 16 passed、`vitest build-stamp.test.ts` は 14 passed。R15 を RETURN trap に戻すと `number.sh --self-test` は rc=1。判定: merge してよい（Critical・Important なし、Minor 4）。

## R23. R20 の直し（表の 3 列目を読み取る処理を、断る検査の外に出す）は gradle にしか届いていない。TS は major を `stampWith` の中で解釈し、ng の行を `toThrow()` だけで見ている。bash の表の試験も 3 列目を確かめない。3 列目が壊れた ng の行は「断った」として通る（Task 2 F2 の TS と bash の半分）。final reviewer が一時 worktree の表に `0.2.0	0.0.1	x	ng` を足して実測した結果は、vitest 14 passed、`version.sh --self-test` rc=0。Rust（`lib.rs:490` の panic）と gradle は落ちる。Global Constraints の「4 か所が同じ表を読む」のうち、表の形を守るのは 2 か所だけ。直し: TS では `/^\d+$/` の検査を `stampWith` の外でする。bash では表を読むループで 3 列目が数字かを確かめる。
- 成果物: web/src/__tests__/build-stamp.test.ts, tools/release/version.sh
- 根拠: web/src/__tests__/build-stamp.test.ts:117-119、tools/release/version.sh（表の試験。:380-386 付近）（final review 2。Minor）
- kind: technical
- 処置: fixed 2.3
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R24. 前の番号と木が同じ commit にも番号が付く。`changed` が空のとき `changes.sh code <<<"$changed"` には空行が 1 行渡り、`grep -qvE` がその空行を拾って rc=0（製品に触れた）になる。実測では `changes.sh code <<<""` は rc=0、`printf '' | changes.sh code` は rc=1。0.2.1 の後に、製品に触れる merge B（run は取り消された）と、その revert C が続くとする。A..C の差分は空なのに、C に 0.2.2 が付く。中身が同じで番号が違う版が 2 つできる。D4（仮）の「前の番号から製品が変わった commit に付ける」とずれる（ブランチの中に前からある形。直しの波が作ったものではない）。CI の側（ci.yml:60 も同じく空の差分で code=true）は「迷ったら走らせる」の安全側なので、直すのは番号付けの側だけでよい。直し: version.sh の number で `[[ -z "$changed" ]]` を「変わっていない」と扱う。自己試験に、空の差分（revert）の Scenario を足す。仮の決定 D4 の解釈に関わるので、直すか残すかは処置の表で決める。
- 成果物: tools/release/version.sh
- 根拠: tools/release/version.sh:216-219、design.md D4（final review 2。Minor）
- kind: premise
- 処置: escalated

## R25. nightly.yml の `id: plan` は、R13 と同じ形（`shell:` が無いまま `version.sh nightly-plan | tee plan.txt`）が残っている。今は害が無い。`nightly-plan` が落ちる道（require_full_history・main_ref・rev-list）はどれも出力より前にあり、出力が空なら後ろの `grep` が rc=1 になって step が落ちる。出力の後に落ちる道を足すと、step は緑のまま通る。直し: `shell: bash` を書き、number.yml とそろえる。
- 成果物: .github/workflows/nightly.yml
- 根拠: .github/workflows/nightly.yml:19-20（final review 2。Minor）
- kind: technical
- 処置: fixed 9.2
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）

## R26. 注釈が、実際には出ていない CI の文を記録として引いている。注釈には「2026-11-16 の CI。『更新 0.0.2 は移行が落ちたので、0.0.3 に戻しました』」とあるが、元の文は 0.4.0・0.5.0 だった（Task 7 F4。code.md の R にも「残りの害の小さいもの」にも入っていない）。直し: 引用を外し、起きたことだけを書く。
- 成果物: tools/release/update-apply-install.sh
- 根拠: tools/release/update-apply-install.sh:187-188（final review 2。Minor）
- kind: technical
- 処置: fixed 7.1

残りの害の小さいもの（R にしない）: Task 3 F7（number_self_test の RETURN trap が関数の後も残る。今は害が無い）。
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）

# code-verify 2 回目 — st61-auto-version-bump（HEAD 829858061、base e72125187）

Task 13（deep.md 第 3 回 Q6）が足された後の再検証。前回（HEAD 2d56b68ba）からの直し（R1〜R26 の処置）と Task 13 を含む木で、手 1〜7 をやり直した。

## 申告と実測

申告: 19/19（`- [x]` は 1.1〜11.1 の 18 項目と 13.1。12.1〜12.5 は「人間の確認待ち」で `[ ]` のまま、ここでは対象外）

| 項目 | 検証コマンド（tasks.md の本文のとおり） | 申告 | 実測 |
|---|---|---|---|
| 1.1 | `version.sh --self-test` / `version_test.sh` / `code 0.2.0=20000・1.0.0=10000000・0.1.10000 は落ちる` | rc=0 | 一致（3 本とも rc=0） |
| 1.2 | `check-version.sh --self-test` / `check-version.sh` | rc=0 | 一致 |
| 2.1 | `cargo test -p ashiato-build-stamp st61_` ＋ `--list` ≥3 | rc=0 | 一致（`st61_` は 5 本、5 passed） |
| 2.2 | `:app:verifyVersionCodeTable :app:verifyVersionCheckTable` ＋表 ≥10 行 / `check-artifacts.sh --self-test-gradle-range` / `check-artifacts.sh apk-build 0.4.0-beta.2`（訂正後） | rc=0 | 一致（3 本目は `versionCode='40000' versionName='0.4.0-beta.2'`） |
| 2.3 | `vitest run src/__tests__/build-stamp.test.ts` / `npm run build` | rc=0 | 一致（14 passed） |
| 2.4 | `check-openapi.sh` / `check-build-stamp.sh release` / `check-build-stamp.sh android` | rc=0 | 一致 |
| 3.1 | `version.sh --self-test` / 2 つの Requirement の Scenario 名の数え ≥20 | rc=0 | 一致（n=21。Task 13 の 1 本が増えた） |
| 4.1 | `number.sh --self-test` | rc=0 | 一致 |
| 4.2 | `check-artifacts.sh --self-test` | rc=0 | 一致 |
| 4.3 | `check-released-migrations.sh --self-test` / `check-contract-version.sh --self-test` | rc=0 | 一致 |
| 5.1 | `check-guards.sh --self-test` / `workflows` / actionlint number.yml | rc=0 | 一致 |
| 6.1 | `changes.sh --self-test` / `workflows` / actionlint ci.yml | rc=0 | 一致 |
| 7.1 | `check-guards.sh --self-test` / `workflows` / 0.4〜0.6 の版が無い grep / `update-apply-install.sh --list` ≥34 | rc=0 | 一致 |
| 8.1 | `version.sh --self-test` / `check-no-release-branch.sh`（訂正後） | rc=0 | 一致 |
| 9.1 | `version.sh --self-test` | rc=0 | 一致 |
| 9.2 | `publish-nightly.sh --self-test` / `check-guards.sh --self-test` / `workflows` / actionlint nightly.yml | rc=0 | 一致 |
| 10.1 | `draft.sh --self-test` | rc=0 | 一致 |
| 11.1 | 手順書の grep / `check_scenarios.py . st61-auto-version-bump` | rc=0 | 一致（`scenarios: OK`。spec の Scenario は 129 本＝128＋Task 13 の 1） |
| 13.1 | `version.sh --self-test` / Scenario `前の番号の commit と差分が空の commit には番号を付けない` の印の grep | rc=0 | 一致 |

actionlint はこの機械に無く、3 本とも docker の `rhysd/actionlint:1.7.7` で rc=0。

## 手ごとの結果

- **手 1（固定値を独立に再計算）**: `version-table.tsv` の 18 行を python で `左×10,000,000＋真ん中×10,000＋右`（上限 209・999・9,999、beta は 1〜98）から計算し直し、18 行とも一致。`version-check-table.tsv` の 27 行（R3・R8・R19 で増えた行を含む）も、spec 476-479 行の規則を python で書き直した判定と全行一致（mismatch 0）
- **手 2（ガードを壊す）**: 使い捨ての worktree（片付け済み）で 1 か所ずつ壊した。
  - Task 13: `version.sh` の空の差分の分岐（`[[ -z "$changed" ]]`）を消すと `version.sh --self-test` rc=1（`number=0.2.2` が付いて落ちる）
  - 前回の後の直し: R14（`changes.sh code` の rc≠0/1 を「付けない」に戻す）→ rc=1・R7（基準が HEAD の祖先かの検査を消す）→ `changes.sh --self-test` rc=1・R6（`trim` を素通しにする）→ `version.sh --self-test` rc=1・R8（bash の出所の版 0.0.x の検査を消す）→ rc=1・R18（place の `persist-credentials` を true）→ `workflows` rc=1・R21（`if:` を `||` に）→ rc=1・R22（更新の試験に `0.4.1:extra` を足す・`0.0.0` を名乗る）→ どちらも rc=1・R23（表に 3 列目 `x` の ng 行を足す）→ bash の自己試験と vitest がどちらも rc=1（vitest は「3 列目が数字でない」で落ちた）
  - number.yml の守り（R2・R11・R12・R13）: `id: decide` の `shell: bash` を消す・workflow の単位に `defaults: { run: { shell: "python {0}" } }` を足す・workflow の単位に `env: { BASH_ENV: … }` を足す・flow 形式の `- { run: "gh release create v1" }` を足す・`find … -exec /usr/bin/gh release create v1 \;` を足す・`gh release create "v1"` を足す、の 6 つはどれも `check-guards.sh workflows` が rc=1
  - R9: `crates/build-stamp/src/lib.rs` の `parse_major` を `text.trim().parse::<u64>().ok()`（`+0` を通す）に戻すと `cargo test -p ashiato-build-stamp st61_` は rc=101（`st61_major_file_accepts_only_digits` が落ちる）
  - R16: `check-artifacts.sh` に `0.4.0-beta.2`・`40000` の特例の行は 0 件（grep rc=1。壊してはいない）
  - **R25 の直し（nightly.yml の `id: plan` の `shell: bash`）は、消しても `check-guards.sh workflows` が rc=0**（R27）
  - R15: `number.sh --self-test` の前後で `$TMPDIR/number-gh-err.*` の数は 16→16（残っている 16 個は 2026-11-23 の日付で、直す前のもの）
- **手 3（Scenario と test）**: `check_scenarios.py` は `scenarios: OK`。新しい Scenario `前の番号の commit と差分が空の commit には番号を付けない`（spec 28 行）は、試験（version.sh の自己試験）が spec の GIVEN と同じ木（`0.2.1` → `crates/server/src/lib.rs` に触れる merge B → revert の merge C）を作り、`git diff --quiet build-0.2.1 HEAD` で差分が空であることを先に確かめてから `number=` と理由の行を見る。主張と同じ階層。design.md D4 の 3. も Q6 の答えに合わせて書かれている（69 行）
- **手 4（本人の決定）**: 1 つずつ値を変えて `version.sh --self-test` を走らせた。Q1（文書だけには付けない: `changes.sh code` の rc=1 を「触れた」として番号の計算へ流す）は rc=1（文書だけの木に `number=0.2.1` が付いて落ちた）・Q2（`10000000`→`1000000`）・Q3（確認バッチの題の `/verify/` を変える）・最初の `0.2.0`（`minor=1` に）・Q6（空の差分の分岐を消す）はどれも rc=1。Q4（根の `Cargo.toml` を `0.1.0`）は `check-version.sh` が rc=1（`NG: 出所の版番号が 0.0.<1 以上> の形でない: 0.1.0`）。Q5（commit の題を書き直さない）はコードの値ではないので試験の対象外
- **手 5（`[x]` と実体）**: 19 項目の検証コマンドは全部実在し、rc=0（上の表）。`cargo test --test <名前>` の型は無い。`cargo test … st61_` は `--list` で 5 本
- **手 6: 該当なし**（確かめた範囲: Task 13 の分岐の周り —— 前の印が無い木で空の差分の commit（first-parent の親と比べる）・確認バッチの merge の後の空の差分（次に番号が付く commit で真ん中が上がる。`prev_commit..commit` の merge を数えるので消えない）・空の差分で `number=` を返したときの number.yml（後ろの job を飛ばして終わる。番号も組み立ても作らないので失うものは無い）を読んだ。走らせたもの —— `git diff --name-only` だけが落ちる偽の `git` を `PATH` の先に置き、印 `build-0.2.0` の後に製品の commit がある使い捨ての木で `version.sh number HEAD` を走らせると rc=128 で止まった（`set -euo pipefail` の下で `changed=$(…)` が落ちる。空の差分と読んで `number=` の rc=0 にはならない。偽を外すと `number=0.2.1` rc=0）。番号の組み立ての消し方は前回と同じく `docs/handoff/ST62.md` の `## st61-auto-version-bump R3` にある（11.1 の grep が rc=0）
- **手 7**: 訂正は pending が 1 件（2.2 `2026-11-23T13:46:06+00:00`）。本体の作業ツリーで `build.gradle.kts:83` を `ASHIATO_BUILD_VERSION` を無視する形に壊すと、新しいコマンド `check-artifacts.sh apk-build 0.4.0-beta.2` は rc=1（`NG: APK の versionName が渡した版番号と違う: versionCode='1' versionName='0.0.1'（期待 0.4.0-beta.2）`）で落ちた。戻した後の `git status --porcelain`・`git diff` は壊す前と cmp で一致。`correction-review.md` に書いた。8.1 は前回の節がある。壊した木で組んだ APK が置き場（gitignore）に残ったので、戻した後に同じコマンドを走らせ直して rc=0（`versionCode='40000' versionName='0.4.0-beta.2'`）を見た。`plan_fix.py pending` は `[]`、`plan_fix.py check` は rc=0（2 件とも確かめ済み）

## R27. R25 の直し（nightly.yml の判定の step に `shell: bash`）を、どの検査も守っていない。消しても `check-guards.sh workflows` は緑のまま
- 成果物: .github/workflows/nightly.yml:21 / tools/release/check-guards.sh:310-333（nightly の検査）
- 根拠: 使い捨ての worktree で nightly.yml の `id: plan` の `shell: bash` の行（とその注釈）を消すと、`tools/release/check-guards.sh workflows` は rc=0。number.yml で同じ行（`id: decide`）を消すと rc=1（`NG: … number.yml number の 2 番目の step: run の shell が bash と書かれていない`。`check-guards.sh:240`）。
  nightly の検査（:319-333）は cron・`nightly-plan` の呼び出し・`ref:` などを見るが、`shell` を見る行は 0 件（grep）。R25 自身が書くとおり、今は `nightly-plan` が出力の前に落ちるので害は無い。出力の後に落ちる道を足したときに、step が緑のまま通る形へ黙って戻る
- kind: technical
- 提案: nightly の検査に「`id: plan` の step（または job・workflow の defaults）が `shell: bash` を持つ」を足し、`--self-test` に外した偽の nightly.yml で rc≠0 になる mutant を足す。number.yml と同じ関数（`check_shell` の型）を使えば 1 か所で済む
- 処置: fixed 9.2
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## code-verify の結論
- 結論: 申告と実測は不一致（1 件。R27）。19 項目の検証コマンドは全部実在し rc=0 だったが、R25 の直し（nightly.yml の判定の step の `shell: bash`）は消しても検査が落ちない（今は害が無い）
- 確かめた範囲: tasks.md の `[x]` 19 項目の検証コマンドを全部実行した（cargo・gradle・apk-build・npm build・check-build-stamp を含む）。2 つの表を python で独立に計算し直した。Task 13 の分岐、前回の後の直し（R2・R6〜R9・R11〜R14・R18・R21〜R23・R25）、本人の決定 6 つを使い捨ての木で壊して走らせた。`git diff` が落ちる偽の git で Task 13 の新しい道を走らせた。新しい Scenario の印の先を読んだ。手 7 の 2.2 は作業ツリーで壊して戻した
- 見ていない範囲: 配布元（GitHub）での実際の印・下書きの作成と number.yml・nightly.yml の実際の run（人間の確認待ち 12.1〜12.5）。Windows の runner での組み立て。R1・R3〜R5・R17・R19・R20 の直しは、今回は壊していない（前回と re-review 1〜4 の判定に任せた）。R15 は一時ファイルの数、R16 は grep だけで、壊していない。R24 は Task 13 として壊した。R10・R26 はコードの守りではないので対象外

## security review（e721251..cc4b9bc）

見た経路: fork の PR → ci（pull_request）→ workflow_run → number job。`number.yml:14` の `event == 'push'` と `head_repository.full_name == github.repository`、`branches: [main]` で書く権限の job に入らない。後ろの 3 job は `needs: number` なので一緒に飛ぶ（3）
見た経路: main の push → number job（`contents: write`、`persist-credentials: true`）。`run:` に `${{ }}` で埋める値は head_sha（GitHub が出す hex）と `steps.decide.outputs.number` だけ。number は version.sh の `^build-[0-9]+\.[0-9]+\.[0-9]+$`（`version.sh:181,195`）と数の計算からしか作られず、merge の題（`version.sh:244-246`）は正規表現の判定に使うだけで出力されない。`number.sh:17` でも形を確かめ直す（4）
見た経路: number.sh tag/place/complete → `git push` は `refs/tags/build-<番号>` の 1 つだけで、強制しない。`gh release create --draft`・`upload` は `--clobber` なし。下書きでない release には書かない（`number.sh:54,93,96,109`）（2。self-test は既存）
見た経路: build-linux/windows（workflow 既定の `contents: read`、秘密なし、`persist-credentials: false`）→ artifact → place（write）。download-artifact は同じ run の `number-*` だけを取る。APK の組み立ては `-Pashiato.apiToken= -Pashiato.userId=` を空のまま組む（`build.sh` assemble_apk）。署名していない APK は `--allow-debug-key` の無い置き場で落ちる（check-artifacts.sh）（2）
見た経路: nightly の publish（署名の秘密あり）→ `ref: needs.plan.outputs.commit` で checkout して台本を走らせる。commit は build-* の印が指す commit のうち main の first-parent 上のものに限られる（`version.sh:153-157`）。印を置けるのは書く権限を持つ人だけなので、未レビューのコードに秘密が渡る経路は無い（3）
見た経路: ci.yml の `id: prev`（push のときだけ、`actions: read`）→ gh api の head_sha → `PREV_SUCCESS_SHA` → `changes.sh base` → `$GITHUB_OUTPUT`。値に改行や `$(id)` を入れると `all` になる（4）。`HEAD~1` のような rev 式は 40 桁に直さずそのまま返す（`changes.sh:31` は `$sha` を返す）。ただし値の出所は GitHub API の head_sha だけで、外から入れる口は無い
見た経路: build-stamp（Rust `lib.rs` の `major_file`）・gradle・web/build-stamp.ts → `tools/release/MAJOR` と `ASHIATO_BUILD_VERSION` を読む。どちらも CI か手元の組み立てが与える値で、外からの入力は無い。数字の正規表現と上限を越えれば落ちる（2）
見た経路: 足した文書・証跡（evidence.jsonl・code.md・docs/briefs/ST61-deep-r3.html）の `+` 行に token・鍵・記録の中身が無いか grep した。出たのは手元の worktree の絶対パスだけ（既存の文書と同じ扱い）（4）
安全の決め手: 書く権限を持つ job（number・place）の `run:` に入るのは、GitHub が出す hex の sha と、`^[0-9]+\.[0-9]+\.[0-9]+$` に限った番号だけ。印の名前・merge の題・枝の名前は、そこまで届かない —— 確かさ 4。`/tmp/st61-sec/probe.sh` で確かめた。使い捨ての木に悪意のある名前の印（`build-0.2.0$(touch${IFS}pwned)`・`build-9.9.9-x"`・`build-1.0.0;id`）、`$(…)` を持つ枝、`number=6.6.6` などの行を混ぜた merge の題を置き、本物の `version.sh number` を走らせた。結果は `number=0.3.0` の 1 行だけで、`pwned` はできなかった。reuse の経路も同じだった。`changes.sh base` は改行と `$(id)` を入れた値で `all` を返した。書く権限の job に fork の run が入らないことは、`number.yml:14` の条件と GitHub の workflow_run の仕様を突き合わせた 3 で止まった。実際の GitHub の run では確かめていない
見ていない範囲: GitHub 上での実際の number.yml・nightly.yml の run（branch protection や印の保護の設定を含む）。Windows の runner の組み立て。check-guards.sh の workflow 検査の中身

指摘なし

## /code-review（e721251..bfa8104）

見た: Claude Code の組み込みの `/code-review`（4 件）。写しは scripts/code_review_record.py

## R28. On a main push with a large changed-file list, CI marks the push as `code=false` and skips every code job. The branch rewrote `changes.sh code` as a single `grep -q` to stop a SIGPIPE false negative, but the caller in ci.yml still pipes `printf '%s\n' "$files" | tools/release/changes.sh code` under `set -euo pipefail`. When the list is over the 64 KB pipe buffer, `grep -q` exits on the first match, printf dies with SIGPIPE (rc=141), and `pipefail` makes the `if` false. The `deps` call on the next line has the same problem. Fix: pass the list with a here-string (`tools/release/changes.sh code <<<"$files"`), as `version.sh number` already does.
- 成果物: .github/workflows/ci.yml:59
- 根拠: Reproduced with 20,000 `crates/x*.rs` paths: `printf "%s\n" "$files" | tools/release/changes.sh code` returns rc=141, so the step writes code=false. The old changes.sh also returned false (rc=1), so the bug predates this branch, but the fix stops short of the caller. A main push after a large refactor skips the rust, e2e and android jobs, reports success, and number.yml then numbers the untested commit.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 6.1
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）

## R29. The bash version rule accepts numbers that the Rust, Kotlin and TypeScript copies reject, so the four copies no longer agree. `parse_version` and `major_file` read each number with `$((10#…))`, which wraps silently above 2^64. A huge left number, or a huge value in the MAJOR file, becomes 0 and passes as a valid number. Rust (`u64` parse fails), gradle (`toLongOrNull` fails) and TS (`isSafeInteger` fails) all reject it. Fix: check the digit count first (for example `${#BASH_REMATCH[1]} <= 9`) in `parse_version`, `major_file` and the left-number read in `number()`, and add such a row to version-check-table.tsv.
- 成果物: tools/release/version.sh:21
- 根拠: `tools/release/version.sh check 18446744073709551616.2.0` → rc=0, and `version.sh code` returns 20000. Writing `18446744073709551616` into tools/release/MAJOR makes `version.sh major` print 0. So bash's check (release.yml / build.sh) passes while the Rust build.rs and gradle fail later with a different error. The version-check-table test does not catch this.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R30. Until the first number exists, a product change whose CI run was cancelled is never numbered. `number()` with no `prev` compares the commit only to its first parent (`$commit^1`). After a number exists it compares to the previous number's commit, which covers this case; without one there is no such base. Fix: when there is no previous number, compare against the root of the first-parent history (the empty tree) or the last `v*` tag, so every change since then counts.
- 成果物: tools/release/version.sh:228
- 根拠: No build-* tag exists yet. Merge A (product code) is pushed, then merge B (docs only) right after. ci's `cancel-in-progress: true` cancels A's run, and B's run succeeds. number(B) diffs B^1=A..B, sees docs only, and returns `number=`. A's code change is not numbered or built until another product change is merged.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 再現した（使い捨ての木。印 build-* なし・製品に触れる merge A・文書だけの merge B）。`version.sh number B` は `number=`、取り消された A の run を走らせ直すと `number(A)=0.2.0`、次に製品に触れる merge C は `number(C)=0.2.0`（A の中身を含む）。この挙動は spec の Requirement（specs/distribution/spec.md:11「番号の印がまだ 1 つも無い THEN 製品が変わったかを、その commit の first-parent の親からの差分で決める」）と Scenario「番号の印が無い木では、文書だけの merge に番号を付けない」（:58）、design.md D4 の「番号の印がまだ 1 つも無いときは…first-parent の親からの差分」（review/spec.md R5 で決めた）そのもの。提案の直し（空の木と比べる）では、印の無い木の文書だけの merge に 0.2.0 が付き、Scenario :58 が落ちる。最後の `v*` の印は repo に 1 つも無い（`git tag -l "v*"` が空）ので基準にならない。取り消された中身は失われず、A の run の走らせ直しか次の製品の merge で番号に入る（Requirement :8 は保たれる）。起きるのは最初の番号が付く前の 1 回だけで、番号が遅れるだけ
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R31. The release section now fails whenever the source version is not exactly 0.0.1. The added `[ "$cargo_version" = 0.0.1 ] || fail …` pins the root Cargo.toml version. The spec allows any source version of the form 0.0.<N> (N ≥ 1), and check-version.sh and version.sh accept it. Fix: check `0.0.<N≥1>` with the same regex as check-version.sh, or drop the line.
- 成果物: tools/check-build-stamp.sh:272
- 根拠: If the source version is moved to 0.0.2 in a PR, the build-stamp job (`check-build-stamp.sh release`) fails with "この repo の木の出所の版が 0.0.1 でない" even though every build is correct.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 固定の 0.0.1 は spec の Scenario「版番号を渡さずに組んだ部品の版番号は 0.0.1」（specs/distribution/spec.md:267-270。GIVEN「この repo の木」THEN「どれも `0.0.1`」）の THEN そのもので、続く「出所の版と違う 0.0.x を渡すと、組み立てが失敗する」（:506。GIVEN「出所の版が `0.0.1` の木」）の前提でもある。check-build-stamp.sh:275-279 は `ASHIATO_BUILD_VERSION=0.0.2` で組んで失敗することを見るので、出所を 0.0.2 に上げた木では、この行を消しても直後の検査が「出所と違う 0.0.x の組み立てが成功した」で落ちる（指摘の「組み立ては全部正しいのに通らない」は、行を外しても変わらない）。出所の版を上げるなら spec の 2 つの Scenario を一緒に変える変更になり、そのときにこの行が先に「出所の版が 0.0.1 でない」と理由を示して止めるのが意図どおり。`0.0.<N≥1>` に広げると、Scenario :267 の THEN を確かめなくなる
- 判定: ADDRESSED（re-review 5 @bc6452b35e5d55e560ec22bcaa1f1caaf1b1c518）

## final review（e721251..2e16097）

final reviewer（whole-branch の 3 回目。`code-reviewer.md`）の指摘。R27〜R31 の処置（c442010..2e16097）は効いていて、新しく壊れたものは無かった。HEAD の一時 worktree で `version.sh --self-test`・`check-guards.sh --self-test`・`check-guards.sh workflows`・`changes.sh --self-test`・`number.sh --self-test` は rc=0、`vitest build-stamp.test.ts` は 14 passed、`cargo test -p ashiato-build-stamp` は 16 passed、gradle `:app:verifyVersionCodeTable :app:verifyVersionCheckTable` は rc=0（表に 5 行足した後の HEAD で）。Task 単位の Minor・⚠️ は全部、処置済みか code.md の「R にしない」の行（:294・:328）か工程の話で、新しく R にするものは無い。判定: merge してよい（Critical・Important なし、Minor 2）。

## R32. release.yml の `id: plan` に、R13・R25 と同じ形（`shell:` が無いまま `version.sh plan … | tee plan.txt`）が残っている。既定の `bash -e` は pipefail を持たず、パイプの rc は `tee` の 0 になる
- 成果物: .github/workflows/release.yml:17-19, tools/release/check-guards.sh
- 根拠: release.yml:17-19 に `shell:` が無い（workflow・job の `defaults` も無い）。今は害が無い —— `plan()` の die はどれも最初の echo より前にあり、出力が空なら後ろの `grep '^channel=' plan.txt` が落ちて step も落ちる。ただこの枝は `plan()` に D11 の止め（印の commit に番号が無い）を足しており、その効き目はこの step が落ちることに懸かっている。number.yml（R13）と nightly.yml（R25・R27）は `shell: bash` を足して check-guards で守ったが、release.yml だけがそろっていない（final review。Minor。読んだだけ、確かさ 3）
- kind: technical
- 提案: release.yml の `id: plan` に `shell: bash` を足し、check-guards の `job_block "$release" plan` に nightly と同じ検査と、外した偽の release.yml で rc≠0 になる自己試験を足す
- 処置: fixed 8.1
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## R33. MAJOR のファイルの桁の扱いが 4 か所でそろっていない。19 桁の MAJOR を bash は断り、Rust・gradle・TS は受ける。R29 の直しは版の文字列の側だけに届いた
- 成果物: tools/release/version.sh:47-53, web/build-stamp.ts:62, crates/build-stamp/src/lib.rs:80, collector-android/app/build.gradle.kts:54-58
- 根拠: MAJOR に `1000000000000000000`（19 桁）を入れて `0.0.1` を渡すと、bash の `major_file` は `fits_decimal` で「数字が大きすぎる」と止まる（実測 rc=1）。Rust の `parse_major`（u64）・gradle の `majorFrom`（Long）は受け、TS の `parseMajor` は `Number()` を返すだけで `isSafeInteger` を見ないので受ける（読んだだけ）。version-check-table.tsv の 3 列目は `major_file` を通らないので表では捕まらない。209 を超える MAJOR は番号に使えないので困る場面はほぼ無いが、「4 か所がそろう」（R8・R29）は MAJOR の側で成り立たない（final review。Minor。確かさ 2）
- kind: technical
- 提案: 4 か所の MAJOR の読みに同じ上限（versionCode の左の上限 209、または bash と同じ 18 桁）を入れ、表に MAJOR の境目の行を足す。あるいは R29 の範囲は版の文字列だけと注記する
- 処置: fixed 1.1
- 判定: ADDRESSED（re-review 6 @480aba6da40002e497bf26ef51f25846732ec2aa）

## final review（e721251..4aeeb8c）

final reviewer（whole-branch の 4 回目。`code-reviewer.md`）の指摘。R32・R33 の直し（2e16097..4aeeb8c）は効いていて、新しく壊れたものは無かった。release.yml の `shell: bash` を外すと `check-guards.sh workflows` が落ちる。4 か所（bash・Rust・TS・gradle）の MAJOR の桁の条件をそれぞれ外すと、どれも表の `0.0.1 0.0.1 1000000000000000000 ng` の行で落ちる（実測）。HEAD の一時 worktree で `version.sh --self-test`・`check-guards.sh --self-test`・`check-guards.sh workflows`・`changes.sh --self-test`・`number.sh --self-test`・`tools/check-version.sh --self-test`・`version_test.sh` は rc=0 だった。`cargo test -p ashiato-build-stamp` は 16 passed、`vitest build-stamp.test.ts` は 14 passed、gradle の `:app:verifyVersionCodeTable :app:verifyVersionCheckTable` は rc=0。Task 単位の Minor・⚠️ は、どれも処置済みか、code.md の「R にしない」の行か、人間の確認（12.x）に入っていて、新しく R にするものは無い。範囲外の観察として、evidence.jsonl の末尾は bf0312e98 で、480aba6da の後に `verify-run` の証跡が無い。判定: merge してよい（Critical・Important なし、Minor 1）。

## R34. 19 桁以上の MAJOR を断るときの文が Rust だけ違う。Rust は「左の数字のファイルが数字 1 つでない」と出し、bash・gradle・TS は R33 の直しで「数字が大きすぎる」と出す
- 成果物: crates/build-stamp/src/lib.rs
- 根拠: crates/build-stamp/src/lib.rs:69-74・:80-87（`parse_major` は `Option<u64>` を返すので、形の誤りと桁あふれを区別できない）。比べた先は tools/release/version.sh:51・collector-android/app/build.gradle.kts:58・web/build-stamp.ts:66。断るかどうかは 4 か所でそろっていて、ずれているのは診断の文だけ（final review 4。Minor。読んだだけ）
- kind: technical
- 提案: `parse_major` を `Result<u64, &'static str>` にして、形の誤りと「数字が大きすぎる」を分けて報告する
- 処置: fixed 2.1
