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

## R4. 2.2 の 3 本目の検証は、組んだ APK が渡した版番号かを見ていない。版を無視した APK でも rc=0 になる（実測 2 回）
- 成果物: openspec/changes/st61-auto-version-bump/tasks.md 2.2 の 3 本目 / tools/release/check-artifacts.sh:65-87（`apk-version`）
- 根拠: (1) 長い検証の中で走らせた 1 回目は rc=0 で、出力は `OK: versionCode='1' versionName='0.0.1-dev.20261102'`（`.superpowers/sdd/logs/20261123-215428-apk.log` と `$TMPDIR/cv/apk.log`）。渡したのは `0.4.0-beta.2` で、置き場の APK は別の組み立てのものだった（何が置いたかは突き止めていない。単独で 2 回走らせ直すと `versionCode='40000' versionName='0.4.0-beta.2'`）。
  (2) 複製で `build.gradle.kts` の `val buildVersionInput = buildVersionProvider.get().trim()` を `= ""` にし（`ASHIATO_BUILD_VERSION` を無視する）、同じコマンドを走らせると rc=0 で、出力は `OK: versionCode='1' versionName='0.0.1'`。
  `apk-version` は `--expect-version` が無ければ APK 自身の versionName から計算した値と比べるだけ（78-83 行）。85 行の 40000 の検査は versionName が `0.4.0-beta.2` のときしか働かない。この型は code_verify R25 で一度直した弱さと同じ（79 行の注記）
- kind: technical
- 提案: 2.2 の 3 本目を `tools/release/check-artifacts.sh apk-version --expect-version 0.4.0-beta.2 collector-android/app/build/outputs/apk/release/*.apk` に直す（受け入れ条件は変えない。`plan_fix.py` の訂正の形）。あわせて、組む前に `app/build/outputs/apk/release/` を空にして、ほかの組み立ての残りを拾わないようにする
- 処置: fixed 2.2
- 判定: ADDRESSED（re-review 1 @7054109bb4551bd3fb778dcff2cc64fec3c7775f）

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

## R9. Only Rust accepts a MAJOR file containing `+0`. `text.trim().parse::<u64>()` allows a leading `+` (u64::from_str does), while bash, TS and gradle all require `^\d+$`. That is the same trap that parse_version guards against by checking for digits only (final review R12). Fix: check that the value is ASCII digits only before parsing.
- 成果物: crates/build-stamp/src/lib.rs:68
- 根拠: If tools/release/MAJOR contains "+0", version.sh, the web stamp and gradle all fail, but the Rust server and collector build with major=0. The four implementations then disagree about whether the build is valid.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.1
- 判定: ADDRESSED（re-review 1 @7054109bb4551bd3fb778dcff2cc64fec3c7775f）

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

## settle_review（re-review 2 @e8c8db1）: 処置の直しが新しく壊したもの

## R12. shell を step の鍵でしか見なくなり、workflow・job の単位の `defaults.run.shell` を通す。直す前は字面の `shell:` の行を全部見ていた。複製の number.yml に `defaults: { run: { shell: "python {0}" } }`（workflow の単位）か `shell: "gh release create v1 {0}"`（place の job の defaults.run）を足すと、7054109 の版は rc=1、現在の版は rc=0。現在の number.yml に defaults は無い。同じ形で BASH_ENV/ENV の拒否（:204）も step の env だけで、job・workflow の env に置くと rc=0（こちらは直す前も rc=0）。
- 成果物: tools/release/check-guards.sh
- 根拠: tools/release/check-guards.sh:211（re-review 2 の New Breakage。Minor）
- kind: technical
- 処置: fixed 5.1
- 判定: ADDRESSED（re-review 3 @c862999539b427f4d46416a01d349dbb67f2cffa）

## final review（e721251..60205ce）

final reviewer（whole-branch。`code-reviewer.md`）の指摘。HEAD で `version.sh` `number.sh` `changes.sh` `check-guards.sh`（`--self-test` と `workflows`）・`check-version.sh --self-test`・`draft.sh --self-test`・`check-no-release-branch.sh`・`check_scenarios.py` は rc=0。各 Task の Minor・⚠️ を全体と照合し、直っていたもの（T1 F6・T2 F1・T3 F4/F5・T5 F3・T9 F4）と R10 と同じもの（T4 F4）は外した。配布元の動きに依る ⚠️（`gh release view` の文言・組の動き・Windows の job）は人間の確認 12.1〜12.5 に残す。

## R13. 番号付けの判定の step が落ちても run は緑になる。`number.yml` の `id: decide` は `shell:` を持たず（workflow・job の `defaults` も無い）、GitHub Actions は Linux で `bash -e {0}`（pipefail なし）で走らせるので、`version.sh number … | tee number.txt` の rc は `tee` の 0。左の数字を下げた（D14）・上限を超えた・versionCode に収まらない・MAJOR が壊れた・浅い木・前の印が読めない、の spec が「0 でない終了コードで終わる」と決めた場合がすべて `number=` 空 → 後ろの job を飛ばして success で終わる。`docs/release.md:30` の「止まると番号付けの run が落ち、理由が出る」と食い違う。同じ形（`sh -c 'echo NG >&2; exit 1' | tee x` の後に sed と if）を `bash -e` で走らせると rc=0（final reviewer の実測）。`check-guards.sh` の number_steps は shell を「あれば bash」としか見ず、未指定を通す。直し: decide の step に `shell: bash`（または `set -o pipefail`、パイプをやめて `>number.txt`）、number_workflow に「decide が `shell: bash` を持つ」検査と、外した偽の number.yml で rc≠0 になる mutant を `--self-test` に足す。
- 成果物: .github/workflows/number.yml, tools/release/check-guards.sh
- 根拠: .github/workflows/number.yml:30-35、docs/release.md:30、tools/release/check-guards.sh（number_steps の shell の検査）（final review。Important）
- kind: technical
- 処置: fixed 5.1

## R14. `version.sh number` は `changes.sh code` の rc=1 だけでなく、rc=2・127・141 も「文書だけ」と読み、`number=` を返して exit 0 する（Task 3 F2 が残っている）。台本が無い・壊れているときも黙って番号が付かない（R13 と同じく止まらない）。直し: rc を取り、1 だけを「付けない」、それ以外は die。自己試験に、changes.sh が rc=2 を返す偽で rc≠0 になる Scenario を足す。
- 成果物: tools/release/version.sh
- 根拠: tools/release/version.sh:217（final review。Important）
- kind: technical
- 処置: fixed 3.1

## R15. `number.sh` の `trap "rm -f '$err'" RETURN` は、後ろの die が exit で抜けると走らず、`$TMPDIR/number-gh-err.*` が残る。`number.sh --self-test` 1 回で 2 個残る（final reviewer の実測）。`HARNESS_TMP_LEAK=fail` に当たる恐れ。直し: stderr を変数で受けるか、EXIT の trap で消す。
- 成果物: tools/release/number.sh
- 根拠: tools/release/number.sh:86-87（Task 4 F2）（final review。Minor）
- kind: technical
- 処置: fixed 4.1

## R16. 本番の `apk_version` に試験の固定値の特例 `[ "$version_name" != 0.4.0-beta.2 ] || [ "$version_code" = 40000 ]` が入っている。直前で `version.sh code "$want"` と比べているので冗長で、試験の値が本番の台本に漏れている。直し: 特例を消す（要るなら apk-build の検証の側へ）。
- 成果物: tools/release/check-artifacts.sh
- 根拠: tools/release/check-artifacts.sh:84（final review。Minor）
- kind: technical
- 処置: fixed 2.2

## R17. release・beta の系統でも `*-unsigned.apk` があれば中身を検査して通す。release.yml は `release/all/*` をそのまま公開に上げ、check-guards は release.yml で `ASHIATO_APK_ALSO_UNSIGNED` を禁じていない。今は立てていないので届かないが、D2（仮）の「署名していない APK は下書きにだけ」を機械が守っていない。直し: `--allow-debug-key` の無い検査では `-unsigned.apk` があれば落とす（と自己試験の mutant）。
- 成果物: tools/release/check-artifacts.sh
- 根拠: tools/release/check-artifacts.sh:379-381、design.md D2（final review。Minor）
- kind: technical
- 処置: fixed 4.2

## R18. place の job は push しないのに checkout が `persist-credentials: true`（Task 5 F4）。書く権限の token を作業木に残す必要が無い。直し: `false`。
- 成果物: .github/workflows/number.yml
- 根拠: .github/workflows/number.yml:107（final review。Minor）
- kind: technical
- 処置: fixed 5.1

## R19. `version-check-table.tsv` に左が 1 の行は `1.0.0 ok` だけ。「左 1 で番号を持たない `0.0.1` が通る」「左 1 で `0.2.0` を断る」行が無く、MAJOR を上げた後に 4 か所のどこかが誤って断る・通す実装になっても表で捕まらない（Task 1 F4 の一部）。直し: その 2 行を足す。
- 成果物: tools/release/version-check-table.tsv
- 根拠: tools/release/version-check-table.tsv（final review。Minor）
- kind: technical
- 処置: fixed 1.1

## R20. gradle の表の検証で `columns[2].toLong()` が `runCatching` の中にあり、3 列目が壊れた ng の行も「断った」として通る（Task 2 F2）。形の誤りの文が出所と MAJOR を含まない（Task 2 F3）。直し: 表の列の読み取りを `runCatching` の外に出す。
- 成果物: collector-android/app/build.gradle.kts
- 根拠: collector-android/app/build.gradle.kts:444（final review。Minor）
- kind: technical
- 処置: fixed 2.2

## R21. number_workflow の `if:` の検査は 3 つの条件をそれぞれ「含むか」でしか見ず、`||` で結んだ偽の number.yml でも通る（Task 5 F2）。直し: `&&` で結んだ形を見るか、条件を分けて読む、と `||` の mutant。
- 成果物: tools/release/check-guards.sh
- 根拠: tools/release/check-guards.sh（number_workflow の `if:` の検査）（final review。Minor）
- kind: technical
- 処置: fixed 5.1

## R22. 更新の試験の版の検査は `old|new|dev-*|compat-*` の名前の項目しか拾わず、別の名前の項目を足すと見逃す。`0.0.*` は `0.0.0` も通す（Task 7 F2・F3）。直し: 版の形の値を名前に依らず拾い、`0.0.[1-9]*` に限る。
- 成果物: tools/release/check-guards.sh
- 根拠: tools/release/check-guards.sh:331-339（final review。Minor）
- kind: technical
- 処置: fixed 7.1

残りの害の小さいもの（R にしない）: `draft.sh:296` の `build-` の守りは自己試験の中だけで中身が無い（本番は `draft.sh:19` で構造上は安全。T10 F1/F2）・version.sh `major_file` の既定の道の二重（T1 F3）・number() の MAJOR 検査の写し（T3 F3）・「文書だけ」の Scenario が 2 つの merge で組む（T3 F6）・number.sh の偽の check-artifacts が `--expect-version` を照合しない（T4 F3）・ci.yml の `all` の暗黙の番兵（T6 F2）・nightly-plan の出力の形（T9 F3）・docs/release.md に「最初の番号は 0.2.0」が無い（T11 F1）。
