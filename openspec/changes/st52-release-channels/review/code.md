# ST52 code review

## final review（26aaf50..80c6603）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 2 / Important 9 / Minor 12。判定は「No（直してから）」。
reviewer が手元で走らせたもの: `version.sh` / `check-guards.sh`（`--self-test` と `workflows`）/ `package.sh` / `draft.sh` / `check-version.sh` の `--self-test` はどれも rc=0、
`check_scenarios.py` OK。修正の枝の判定は CI と同じ形の clone で、規則の API は `gh api` で確かめた。
Task ごとの ⚠️ は final reviewer が全体で確かめた。成り立ったのは Task 1（--self-test が CI で走らない → R9）、Task 5（BIND → R2）、Task 9（`rules/tags` の API → R4）、
Task 10・11（§8 → R4・R5・R6）、Task 11（修正の枝 → R3）。そのほかは解消済みか当たらない（Task 2 の 0 tests は報告の誤読、Task 4 の fetch-depth は浅くならない、
Task 6 の R8 は minify なし、Task 8 の版番号の照合は枝の判定より前に走る、Task 9 の gnu の target は使われない）。
fix は 1 回（947f674..25e11e4）。指摘の外に 3 つ: check-artifacts の拒む理由の順（e56fa68）、配る APK を Kotlin の増分の組み立て無しで組む（f3e4e18。前の commit の印が APK に残った）、workspace の clippy -D warnings（69e5f55）。どれも振る舞いを変えないことを re-review が確かめた。証跡は最後の木で 1.1〜11.1 を verify-run に通し全部 PASS。
scoped re-review 1 回で R1〜R14・R16〜R22 は全部 ADDRESSED、新しい Critical / Important なし。新しい Minor の design D2 の旧 API 名は直した（R4）。R15・R23 は直さない（理由は各 R）。

## R1. 配る組み立てが画面の依存を入れないので、package の job がどれも落ちる
- 成果物: tools/release/build.sh / .github/workflows/ci.yml / release.yml / nightly.yml
- 根拠: tools/release/build.sh:21-25（`build_web` は `npm run build` だけ）。package-* の job に setup-node も `npm ci` も無い。新しい runner に `web/node_modules` は無い / Task 6 F13（未修正）
- kind: technical
- 処置: fixed 6.1 —— build.sh の build_web で `npm ci --ignore-scripts` の後に build。3 つの workflow の package-linux / package-windows に setup-node 24（3d27cd87・453323d5）

## R2. `st52-served-ui.spec.ts` は CI では BIND が無く必ず落ちる
- 成果物: web/e2e/st52-served-ui.spec.ts
- 根拠: web/e2e/st52-served-ui.spec.ts:10-12（`process.env.BIND` が無いと throw）。CI の e2e job も `tools/ci-db-env.sh` も BIND を出さない。`stack.sh:33` の export は webServer の子プロセスにしか効かない / Task 5 ⚠️
- kind: technical
- 処置: fixed 5.3 —— web/playwright.config.ts が `tools/ports.sh` から待ち受けを組み spec へ渡す（stack.sh と同じ出所。runner の checkout でも同じ値）。BIND を unset した env で st52-served-ui 1 passed（453323d5）

## R3. 修正の枝の印は CI の plan で必ず落ちる（枝を `refs/heads/release/*` からしか探さない）
- 成果物: tools/release/version.sh
- 根拠: tools/release/version.sh:95。CI と同じ形の clone（`+refs/heads/*:refs/remotes/origin/*` と tags、印を detached で checkout）で `plan v0.4.1` が「NG: 印の commit が main の祖先でない」rc=1 / Task 11 ⚠️
- kind: technical
- 処置: fixed 8.1 —— `refs/remotes/origin/release/*` も探し `origin/` を剥がして比べる。self-test に手元の枝の無い clone を足した（f38cf860）

## R4. `rules/tags/<印>` の API は無く、正式版の publish はすべて `check-guards.sh rules` で止まる
- 成果物: tools/release/check-guards.sh / docs/release.md / openspec/changes/st52-release-channels/tasks.md（12.3）
- 根拠: tools/release/check-guards.sh:14。`gh api repos/cli/cli/rules/branches/trunk` は配列を返すが `gh api repos/cli/cli/rules/tags/v2.0.0` は 404。あわせて `rules` は `GITHUB_REF_NAME` が無いと何も問い合わせずに落ちる（:9-11）ので docs/release.md §4・§8 と tasks 12.3 の呼び方は通らない。tasks 12.3 の repo 名が `ashiato`（既定は `ashiato2`）/ Task 9 ⚠️
- kind: technical
- 処置: fixed 9.1 —— `rulesets` の一覧と詳細で、v* に当たる有効な tag の ruleset の deletion と update / non_fast_forward を見る。GITHUB_REF_NAME 不要・`--repo` は前後どちらでも。docs §4・§8、tasks 12.3 の repo 名（ashiato2）、design D2 の API 名を直した。tasks 9.1 の説明文の旧 API 名は凍結した計画の文のため残し、design D2 を正とする（611b2192・25e11e40）

## R5. 確認バッチ 12.1 は書かれたとおりには通らない（draft.sh の版番号と PR の package の版番号が食い違う）
- 成果物: tools/release/draft.sh / docs/release.md §8 / tasks.md 12.1 / design.md D16
- 根拠: PR の package は `ASHIATO_BUILD_VERSION` を渡さず出所の版（`0.1.0`）で組む（D14・package.sh:168-173）。`draft.sh <run-id> 0.1.0-beta.1` は `check-artifacts --expect-version 0.1.0-beta.1` を当てるので「欠けた配布物」で必ず落ちる。self-test の偽の gh は beta の名前で作るので見えない
- kind: technical
- 処置: fixed 10.1 —— spec の Scenario・D16 の `draft.sh <run-id> <版番号>` を保ち、配布物だけを出所の版（`version.sh source`）と比べる。本文（--notes）に配布物が出所の版の組み立てであると書く。spec を変えずに比べ方だけで閉じたので technical に直した（仮の D は要らない）（9febeab1）

## R6. docs/release.md §8 の印の数え方が P16（下書きは印を作らない）を確かめていない
- 成果物: docs/release.md
- 根拠: docs/release.md §8 で `before=` を draft.sh の**後**で数えている。tasks 12.1 は「作る前と後」
- kind: technical
- 処置: fixed 11.1 —— §8 の `before=` を draft.sh の前へ（611b2192）

## R7. publish と package-artifacts で Android SDK の道具（apksigner / apkanalyzer）が PATH に無い
- 成果物: tools/release/check-artifacts.sh / tools/release/sign-apk.sh / workflow
- 根拠: check-artifacts.sh の `command -v apksigner`（fallback 無し）と `apkanalyzer`。apksigner は build-tools の下でどの runner の PATH にも無い。`--allow-debug-key` を付けない経路は self-test でも走っていない（Task 6 F10）。sign-apk.sh には `$ANDROID_HOME/build-tools` を探す fallback がある
- kind: technical
- 処置: fixed 6.2 —— `tools/release/android-tools.sh` の `android_tool`（PATH → cmdline-tools/latest/bin → build-tools の最新）を check-artifacts・sign-apk・build の self-test が使う。`--allow-debug-key` 無しの経路も self-test で走る（3d27cd87）

## R8. ベータ番号の範囲が spec（1〜98）と違い 1〜89 になっている
- 成果物: crates/build-stamp/src/lib.rs / tools/release/version.sh / web/build-stamp.ts / collector-android/app/build.gradle.kts / tools/release/version-table.tsv
- 根拠: specs/distribution/spec.md:8・design.md:85 は「N は 1〜98」。実装は build-stamp `lib.rs:61`（1..=89）・`version.sh:18`（`[1-9]|[1-8][0-9]`）・web・gradle が同じ誤り。versionCode の式（beta.98 → …98 < 正式 …99）は 98 まで成り立つ
- kind: technical
- 処置: fixed 2.1 —— 4 部品を 1〜98 に。version-table.tsv に beta.98 と拒む行（beta.99・beta.0・beta.+5）を足し version.sh と gradle が同じ表で確かめる（cd40d4b1）

## R9. 判定の台本の --self-test が CI で走らない
- 成果物: .github/workflows/ci.yml
- 根拠: CI に入っているのは `check-version.sh --self-test` だけ。`version.sh`・`check-guards.sh`・`package.sh`・`draft.sh` の軽い --self-test は走らない（D8「判定は台本に置き --self-test で確かめる」）/ Task 1 ⚠️
- kind: technical
- 処置: fixed 9.2 —— ci.yml の chain で version.sh・check-guards.sh・package.sh・draft.sh の --self-test を走らせる（453323d5）

## R10. 自己試験が `0.1.0` を決め打ちにしていて、出所の版を上げると壊れる
- 成果物: tools/release/build.sh / check-artifacts.sh / sign-apk.sh / draft.sh
- 根拠: build.sh:136-145、check-artifacts.sh の self_test、sign-apk.sh:58、draft.sh の偽の gh。版の check は生きた Cargo.toml と比べるので、docs/release.md の「正式版の後に出所の版を上げる」で 4 本が落ちる（Task 6・7・10 の Minor）
- kind: technical
- 処置: fixed 6.1 —— build・check-artifacts・sign-apk・draft の self-test の版を `version.sh source` から組む（3d27cd87・611b2192）

## R11. nightly の公開で配布物が消えたまま残りうる（C7 違反）、最初の公開も落ちる
- 成果物: .github/workflows/nightly.yml / docs/release.md §5
- 根拠: nightly.yml:149-155 は前の配布物を消してから upload する。upload が落ちると空のまま残る。`gh release upload nightly` はリリースがあることを前提にし、作る手順がどこにも無い
- kind: technical
- 処置: fixed 9.3 —— 無ければ `gh release create nightly --prerelease` で作り、`upload --clobber` の後に今回に無い名前だけを消す。docs §5 も合わせた（453323d5）

## R12. `0.1.0-beta.+5` をサーバと PC だけが受ける
- 成果物: crates/build-stamp/src/lib.rs
- 根拠: crates/build-stamp/src/lib.rs:60 の `"+5".parse::<u8>()` が通る / Task 2 F6
- kind: technical
- 処置: fixed 2.1 —— 数字だけかを先に見る。`0.1.0-beta.+5` を拒む試験を足した（cd40d4b1）

## R13. access_log の `/api` を剥がす処理は通らない
- 成果物: crates/server/src/access_log.rs
- 根拠: access_log.rs:153-157。`strip_api_prefix` が URI を書き換えた後に走る / Task 5 F11
- kind: technical
- 処置: fixed 5.1 —— 通らない剥がしを消した（3f24d429）

## R14. `web_assets::fallback` が method を見ず、POST にも画面を 200 で返す
- 成果物: crates/server/src/web_assets.rs
- 根拠: web_assets.rs の fallback / Task 5 F10
- kind: technical
- 処置: fixed 5.1 —— 画面のファイルがある道で GET / HEAD 以外は `Allow: GET,HEAD` 付きの 405。`st52_web_assets_only_get_and_head` を足した（3f24d429）

## R15. サーバが配る画面に Host の許可（vite preview の `allowedHosts` に当たるもの）が無い
- 成果物: crates/server/src/web_assets.rs
- 根拠: final reviewer の指摘（Minor）。cookie は host に縛られ、API は認可が要るので危険は小さい
- kind: technical
- 処置: rejected: crates/server/src/web_assets.rs:49-59 の fallback が返すのは組み込んだ静的なファイル（GET / HEAD）だけで、crates/server/src/web_session.rs:196 の cookie は Host 限定（Domain 無し・SameSite=Strict）、cookie は host に縛られ API は認可が要るので、Host を偽っても読めるのは公開の画面と同じもの。悪用の経路は次の security-review の席が辿る

## R16. check-artifacts.sh の self-test が追跡中の `crates/server/src/lib.rs` に書き足し、trap で戻さない
- 成果物: tools/release/check-artifacts.sh
- 根拠: Task 6 F15
- kind: technical
- 処置: fixed 6.2 —— 書き換えている間だけ控えを持ち EXIT・INT・TERM の trap で戻す。走らせた後に lib.rs の差分なし（3d27cd87）

## R17. version.sh の未使用の変数と、`--is-shallow-repository` の失敗を浅くないとみなす扱い
- 成果物: tools/release/version.sh / tools/check-released-migrations.sh
- 根拠: version.sh:91 の `expected` は未使用。version.sh:71 と check-released-migrations は `git rev-parse --is-shallow-repository` が失敗すると浅くないとみなす / Task 4 F8
- kind: technical
- 処置: fixed 1.1 —— 未使用の変数を消し、`--is-shallow-repository` の失敗で version.sh も check-released-migrations.sh も落とす（f38cf860）

## R18. version.sh が `refs/tags/nightly` を `^{commit}` 無しで引いている
- 成果物: tools/release/version.sh
- 根拠: version.sh:121 / Task 1 F8
- kind: technical
- 処置: fixed 1.1 —— `refs/tags/nightly^{commit}` で引き、注釈つきの印を self-test に足した（f38cf860）

## R19. check-guards.sh の sed の検査が恒に偽で何も確かめていない・`--repo x rules` の形が落ちる
- 成果物: tools/release/check-guards.sh
- 根拠: check-guards.sh:116・:136
- kind: technical
- 処置: fixed 9.1 —— 恒に偽の sed を消し、code の判定を外した偽の ci.yml で workflows が落ちることを見る。`--repo x rules` も通る（611b2192）

## R20. Windows の手順が pwsh から bash を呼び、使われない gnu の target を入れている
- 成果物: .github/workflows/ci.yml / release.yml / nightly.yml
- 根拠: Windows の package の手順は既定の pwsh。`targets: x86_64-pc-windows-gnu` は windows_target が native を返すので使われない / Task 9 F14
- kind: technical
- 処置: fixed 9.2 —— 3 つの workflow の package-windows を `shell: bash` と `$RUNNER_TEMP` にし gnu の target を外した（453323d5）

## R21. build-stamp の job に setup-node が無い（vite 7 は node 20.19 / 22.12 以上が要る）
- 成果物: .github/workflows/ci.yml
- 根拠: check-build-stamp.sh は自分で npm ci するが node の版は runner 任せ / Task 3 ⚠️（一部成り立つ）
- kind: technical
- 処置: fixed 3.4 —— build-stamp の job に setup-node 24（453323d5）

## R22. draft.sh が `--notes` を渡さず、検査していないファイルも載りうる
- 成果物: tools/release/draft.sh
- 根拠: TTY から走らせると gh が本文を対話で聞く恐れ。`"$work"/*` で載せる / Task 10 F4・F5
- kind: technical
- 処置: fixed 10.1 —— `--notes` を渡し、載せるのは check-artifacts が見た 4 つの名前だけ。余分なファイルが載らないことを self-test で確かめる（611b2192・9febeab1）

## R23. tasks.md の 9.1 が `[ ]` のまま（証跡は PASS）
- 成果物: openspec/changes/st52-release-channels/tasks.md
- 根拠: tasks.md の 9.1 / final reviewer の指摘（Minor）
- kind: technical
- 処置: rejected: 9.1 は `tasks_phase.py` が PR・gate の段に分ける項目で、`[ ]` のまま残すのが正常（`[x]` はグラフの mark が付ける）。証跡は最後の木で PASS

## code_verify（HEAD 3025eaeb。独立検証 2026-11-02）

席: code_verify（申告と実態のずれ）。**tasks.md の全項目の検証コマンドを HEAD で走らせ直した**（2026-11-02 07:44〜08:10。`evidence.py` の `declared()` で本文の 49 本を取り出し、台本 `/tmp/st52cv/run.py` で 1 本ずつ作業ツリーの根で走らせた。結果 `/tmp/st52cv/results.tsv`・`/tmp/st52cv/log/`。証跡（evidence.jsonl）には書いていない）。
変異と探りは作業ツリーの外の複製（`/tmp/st52cv/repo`・`repo2`。HEAD を `git clone` した独立の木）で当て、作業ツリーのコードは触っていない —— 前後の `git status --porcelain` と `git diff` はどちらも空で一致（`check-artifacts.sh --self-test` が一時に書き換える `crates/server/src/lib.rs` も戻っている）。
`plan_fix.py pending` → `[]`、`plan_fix.py check` → rc=0。`git merge-tree --write-tree origin/main HEAD`（origin/main 26aaf500。HEAD より先の commit 0）→ rc=0。

### 申告: 24/29（`- [x]` 24・`- [ ]` 5 —— 9.1 と 12.1〜12.4）・検証コマンド一覧

| 項目 | 検証コマンド（tasks.md の本文どおり） | 実測（HEAD 3025eaeb） | 一致 |
|---|---|---|---|
| 1.1 | `version.sh --self-test` / `code 0.3.0-beta.2 = 30002 && code 1.0.0 = 1000099` | rc=0 / rc=0 | 一致 |
| 1.2 | `grep -q "release/" CLAUDE.md` | rc=0 | 一致 |
| 2.1 | `cargo test -p ashiato-build-stamp st52_`（list ≥3） | 4 passed・rc=0 | 一致 |
| 2.2 | `--test server_startup st52_bin_`（≥3）/ `check-openapi.sh` / `cargo test -p ashiato-server migration_ledger` | 4 passed（122.5 秒）/ rc=0 / lib 23 passed | 一致 |
| 3.1 | gradle `:app:verifyVersionCodeTable` と表 ≥8 行 / `ASHIATO_BUILD_VERSION=<出所>-beta.2 assembleRelease && check-artifacts.sh apk-version` / `--self-test-gradle-range` | rc=0 / rc=0（`versionCode='10002' versionName='0.1.0-beta.2'`）/ rc=0 | 一致（ただし R25） |
| 3.2 | vitest `build-stamp.test.ts` / `npm run build` | rc=0 / rc=0 | 一致 |
| 3.3 | `check-version.sh --self-test` / `check-version.sh` | rc=0 / rc=0 | 一致 |
| 3.4 | `check-build-stamp.sh release` / `check-build-stamp.sh android` | rc=0（637 秒）/ rc=0 | 一致（ただし R25） |
| 4.1 | `check-released-migrations.sh --self-test` / 本体 | rc=0 / rc=0 | 一致（ただし R28） |
| 4.2 | `check-contract-version.sh --self-test` / 本体 | rc=0 / rc=0 | 一致 |
| 4.3 | `grep -c "ST52 で処置した" docs/handoff/ST52.md` ≥3 | rc=0 | 一致 |
| 4.4 | `rust` の job に `fetch-depth: 0` があり `--depth=1` が無い / 深さ 1 の clone で 2 本とも「履歴が浅い」で落ちる | rc=0 / rc=0 | 一致 |
| 5.1 | `cargo test -p ashiato-server st52_web_`（≥5）/ `access_log` / `check-openapi.sh` | lib 6 passed / lib 14 passed / rc=0 | 一致 |
| 5.2 | `--test server_startup st52_bin_release`（≥1） | 1 passed（17.5 秒） | 一致 |
| 5.3 | e2e `st52-served-ui.spec.ts` / `login.spec.ts stack.spec.ts` | 1 passed / 8 passed | 一致 |
| 6.1 | `build.sh --self-test` | rc=0（95 秒） | 一致 |
| 6.2 | `check-artifacts.sh --self-test` | rc=0（221 秒） | 一致 |
| 7.1 | `make-key.sh --self-test` / `grep "^| P17 " premises.md` | rc=0 / rc=0 | 一致（ただし R27） |
| 7.2 | `sign-apk.sh --self-test` | rc=0（147 秒） | 一致（ただし R26） |
| 8.1 | `version.sh --self-test` | rc=0 | 一致（ただし R24） |
| 9.1 `[ ]` | `check-guards.sh --self-test` | rc=0 | —（`[ ]` は final review R23 のとおり PR・gate の段の項目） |
| 9.2 | `check-guards.sh --self-test` / `package.sh --self-test` / actionlint `ci.yml` / `check-guards.sh workflows` | rc=0 / rc=0 / rc=0（`actionlint` は PATH に無く docker の `rhysd/actionlint:1.7.7` で無出力）/ rc=0 | 一致 |
| 9.3 | `check-guards.sh --self-test` / actionlint `release.yml`・`nightly.yml` / `workflows` / 判定の job に `fetch-depth: 0` | rc=0 / rc=0・rc=0（同上）/ rc=0 / rc=0 | 一致 |
| 10.1 | `draft.sh --self-test` | rc=0 | 一致 |
| 11.1 | `docs/release.md` の 8 語 / handoff 3 本の grep / `check_scenarios.py . st52-release-channels` | rc=0 / rc=0 / rc=0（`scenarios: OK`） | 一致 |

**手 1（固定値の再計算）: 該当なし（一致）。** `tools/release/version-table.tsv`（version.sh の self-test と gradle の `verifyVersionCodeTable` が共に読む表）の 15 行を、spec の式（`MAJOR×1,000,000 + MINOR×10,000 + PATCH×100 + 末尾`。末尾はベータ N・正式 99・開発版 0、N は 1〜98、MINOR・PATCH ≤99）から python で独立に計算し直した → 12 行の値がすべて一致、拒む 3 行（`beta.99`・`beta.0`・`beta.+5`）も python が拒む。tasks 1.1 の 2 つの値（30002・1000099）も同じ。

**手 2（ガードを壊す）:** 複製で 1 か所ずつ壊した。
- `check-released-migrations.sh` の浅い木の止め（`= true` → `= never`）→ `--self-test` rc=1（`FAIL: 浅い木で検査が通った`）/ `check-contract-version.sh` の同じ止め → rc=1 / `version.sh` の `require_full_history` → rc=1（`浅い木では、印の判定は通らない` で止まる）
- `check-contract-version.sh` の印の型 `'v[0-9]*'` → `'*'` → rc=1（`NG: 並べられない印: nightly`）
- `check-released-migrations.sh` の印の型 `'v[0-9]*'` → `'*'`（2 か所）→ **rc=0**（**R28**）
- 検査の外側: `crates/server/Cargo.toml` を `version = "0.9.0"`（`version.workspace` を外す）にする → `check-version.sh` rc=0。ただし `build.sh` は版を渡さないときも出所の版を `ASHIATO_BUILD_VERSION` に入れて組む（`tools/release/build.sh:13-20`・`:46`）ので、PR の `package` の組み立てで `version()` の出所との照合に当たって止まるはず（照合が `Err` を返すことは 2.1 の試験が持つ。この組み合わせの通しは走らせていない）。指摘にはしない
- 修正の枝の判定に、配った移行の**中身**の書き換えを当てた → `plan` rc=0（**R24**）

**手 3（Scenario と test）:** `check_scenarios.py` rc=0（この change の 111 本はすべて印あり）。印の先が主張を観測しているかを見た範囲 —— 浅い木の 4 本（clone `--depth=1` で rc≠0 と「履歴が浅い」の両方を見る）、資格情報（`secret-token-for-test` を焼き込んだ本物の APK で `API_TOKEN` が出て値が出ないこと）、`.dirty`（追跡中の `lib.rs` を書き換えて本物に組む）、nightly の 3 つと APK 2 名、配った印の順（版番号の順・ベータの数・正式版優先）は THEN を観測している。
ずれは 3 か所に出た: 「渡した版番号は、4 つの部品の印に同じ値で入り…」と「組み立てた APK の versionCode は、渡した版番号から計算した値」（**R25**）、「識別子とチャンネルの鍵を取り違えると、署名しない」の鍵の側（**R26**）、「開発版の印は、配った印に数えない」の移行の側（**R28**）。

**手 4（本人の決定）:** 複製で値を 1 つずつ変えて、関係する試験を走らせた。
- 落ちる（固定されている）: Q3 正式版の末尾 99 → 98（`version.sh --self-test` rc=1、`NG: version-table.tsv: 0.3.0`）/ Q6 開発版の日付の `TZ=Asia/Tokyo` → `UTC`（rc=1、`main が進んだ日は、開発版を作り直す`）/ Q12 浅い木の止め 3 本（上の手 2）/ Q10 送信契約の側（上の手 2）/ D3 作る鍵の `-validity 10000` → `400`（`make-key.sh --self-test` rc=1、`鍵の有効期限が 25 年未満`）/
  Q4 開発版に APK を足した偽の `nightly.yml`（`check-guards.sh` の self-test が自分で持つ）/ cron `0 15 * * *`（`check-guards.sh workflows` が grep で見る）
- **固定されていない**: D3 の確かめの下限「25 年」（**R27**）、Q10 の移行の側（**R28**）、Q6「ベータは印から渡す」の携帯の側（**R25**）

**手 5（`[x]` と実体）:** 24 件とも本文の検証コマンドが実在し rc=0（上の表）。`--test server_startup` は `crates/server/tests/server_startup.rs` が実在。件数つきは全部 `-- --list` で下限以上（2.1 は 4、2.2 は 4、5.1 は 6、5.2 は 1）。
`cargo test … migration_ledger` / `access_log` は quiet-run の要約の末尾が `server_startup` の `0 passed` だが、全文のログで lib が 23 passed / 14 passed を確かめた。
ただし 3.1 の 2 本目と 3.4 の 1 本目は、rc=0 でも項目が名指す Scenario の「渡した版番号」を観測していない（R25）。

**手 6（隙間）:** 「捨てたものは復元できない」型を先に探した。
- 修正の枝の印は、CI（`ci.yml:3-4` は `push: main` と `pull_request` だけ）も印の workflow（`release.yml` は `version.sh plan` だけ）も配った移行の凍結を通さずに公開される（**R24**）。当てた DB の移行は戻せず、`v*` は消せない
- 申し送り: ST53（`requires: [ST52]`。最初の印・本物の鍵・本人の環境の切り替え）・ST54（`requires: [ST52]`。開発版の APK と鍵）は `docs/stories/` に実在し、`docs/handoff/ST53.md`・`ST54.md`・`ST58.md` に ST52 の節がある（11.1 の検証が rc=0）。
  deep.md の「要件へ戻すもの」が「change で拾う」とした **K1**（`tools/verify-prep.sh` が繋がった端末に debug の鍵の `dev.ashiato.collector` を入れる）は、どこにも拾われていない（**R30**）

**手 7（機械が承認した訂正）: 該当なし**（`plan_fix.py pending` → `[]`）。

## R24. 修正の枝で配った移行の中身を書き換えても、印の判定が通って公開される（配った移行の凍結を一度も通らない）
- 成果物: tools/release/version.sh / .github/workflows/release.yml / docs/release.md
- 根拠: 使い捨ての木（`/tmp/st52cv/hotfix`）で `v0.3.0` から `release/0.3` を切り、`migrations/202601010000_init.sql` の中身だけを書き換え（ORDER は同じ）、出所を 0.3.1 にして `v0.3.1` を打った → `tools/release/version.sh plan v0.3.1` **rc=0**（`channel=release`）。同じ木で `tools/check-released-migrations.sh` は rc=1（`NG 移行 202601010000_init の中身が印 v0.3.0 の版から変わっている`）。
  `plan` が修正の枝で見るのは `migrations/ORDER` と `docs/contract/envelope.json` の差だけ（version.sh:107-108）。`release.yml` は `version.sh plan` しか走らせず（release.yml:19）、凍結の 2 本（`check-released-migrations.sh`・`check-contract-version.sh`）を呼ばない。
  `ci.yml` の起動は `push: { branches: [main] }` と `pull_request` だけ（ci.yml:3-4）で、`docs/release.md` §6 の手順は `git push -u origin release/0.1`（docs/release.md:64）→ 印を push で PR を作らない。だから修正の枝の commit はどの凍結の検査も通らずに公開される。
  spec の SHALL の文面は ORDER と envelope.json だけだが、Requirement の題は「移行と封筒の形を変えない」で、ST48 の「配った移行は、名前も中身も変えられない」は枝を問わない。当てた DB の移行は戻せず、配った印 `v*` は消せない（D2）
- kind: technical
- 提案: `release.yml` の `plan` の job（`fetch-depth: 0`）で `check-released-migrations.sh` と `check-contract-version.sh` も走らせ、落ちれば公開しない（枝の上でも祖先の印を数える形は 4.1・4.2 がもう持つ）。`version.sh --self-test` に「修正の枝で配った移行の中身を変えた印は公開しない」を足す
- 処置: fixed 8.1 —— 修正の枝の判定（`version.sh plan`）が ORDER に加えて `migrations/` 全体を元の印と比べ、中身だけを変えた印も公開しない。self-test に「ORDER は同じで配った移行の中身を書き換えた印」を足し、比べる行を消すと self-test が rc=1 になることを確かめた（2f284a59）

## R25. 「渡した版番号」の 2 つの Scenario を、印の先の試験が観測していない（gradle が渡した版番号を無視しても 3.1・3.4 が緑）
- 成果物: tools/check-build-stamp.sh / tools/release/check-artifacts.sh / .github/workflows/ci.yml / collector-android/app/build.gradle.kts
- 根拠: 複製で `build.gradle.kts:49` を `val buildVersionInput = ""`（`ASHIATO_BUILD_VERSION` を読まない）にして commit した →
  tasks 3.1 の 2 本目と同じ `ASHIATO_BUILD_VERSION=0.1.0-beta.2 gradlew :app:assembleRelease && check-artifacts.sh apk-version` → **rc=0**（`OK: versionCode='10099' versionName='0.1.0'`）。
  `tools/check-build-stamp.sh release`（tasks 3.4 の 1 本目と ci.yml:162 の形）→ **rc=0**（`== Scenario: 渡した版番号は、4 つの部品の印に同じ値で入り、印は .dirty を持たない` が出る）。
  `apk-version` は APK 自身の versionName から versionCode を計算して比べるだけで、渡した値と比べない（check-artifacts.sh:20-22）。spec の THEN は「versionCode は 40002 で、versionName は `0.4.0-beta.2`」。
  `check-build-stamp.sh release` は呼び手の環境に `ASHIATO_BUILD_VERSION` があるときだけそれを期待し、無ければ出所の版を期待する（check-build-stamp.sh:183-186）。tasks 3.4 の本文は「`ASHIATO_BUILD_VERSION=<出所>-beta.2` を渡して」だが、検証コマンドにも ci.yml:162 にもこの値が無いので、この節は版番号を渡さずに組んでおり、すぐ後の「版番号を渡さない組み立て」と同じものを 2 度見ている
- kind: technical
- 提案: release 節が自分で `ASHIATO_BUILD_VERSION="$cargo_version-beta.2"` を立てて 4 部品を組む（呼び手の環境に頼らない）。`apk-version` に `--expect-version <版番号>` を足し、versionName と versionCode の両方を渡した値と比べる（3.1 の検証も `--expect-version "$(version.sh source)-beta.2"` にする）
- 処置: fixed 3.4 —— `check-build-stamp.sh release` が呼び手の環境に頼らず `<出所>-beta.2` を自分で 4 部品に渡し（出所の版と同じなら落とす）、`check-artifacts.sh apk-version --expect-version` で versionName と versionCode を渡した値と比べる。3.1 の凍結した検証コマンドは `apk-version` を引数なしで呼ぶので渡した値を見ないまま（旧コマンドは FAIL しないので plan_fix では直せない）。この Scenario は 3.4（CI の build-stamp の job）が担保する（2f284a59）

## R26. 開発版の識別子の APK に正式な鍵を当てても、`sign-apk.sh` は署名する（見ているのは識別子と引数のチャンネルだけで、鍵を見ない）
- 成果物: tools/release/sign-apk.sh
- 根拠: 複製（`repo2`）で `make-key.sh official` の使い捨ての鍵を作り、`ASHIATO_BUILD_VERSION=0.1.0-dev.20261101 ASHIATO_APK_UNSIGNED=1 build.sh apk` で組んだ APK（`apkanalyzer` で `dev.ashiato.collector.dev`）に、正式な鍵の env で `sign-apk.sh <apk> dev <出力>` → **rc=0**。
  できた APK の `Signer #1 certificate SHA-256 digest: 1be1ed92…d013bb` は正式な鍵の `SHA256: 1B:E1:ED:92:…:D0:13:BB` と一致（`/tmp/st52cv/probe-sign.log`）。
  sign-apk.sh:27 は APK の識別子とチャンネルの引数が合うかしか見ず、渡された鍵がそのチャンネルの鍵かを見ない。self-test の取り違えの 2 本は、どちらもチャンネルの引数も一緒に取り違えている（識別子の検査で落ちる）。
  spec は「開発版の識別子 `dev.ashiato.collector.dev` の APK に正式な鍵を…当てようとしたら、署名せずに失敗する」。開発版の Environment に鍵を置くのは ST54 で、そこへ正式な鍵を置き間違えると、開発版の APK が正式な鍵で配られる（nightly の `check-artifacts --channel nightly` は今 APK の指紋を見ない）
- kind: technical
- 提案: 署名の前に鍵の証明書の指紋を読み、チャンネルの期待値（正式は `ASHIATO_EXPECT_CERT_SHA256`、開発版は ST54 で置く変数）と比べ、違えば署名しない。self-test に「識別子とチャンネルは合っていて鍵だけが違う」の 2 本を足す
- 処置: fixed 7.2 —— `sign-apk.sh` が署名の前に鍵の証明書の指紋を読み、正式は `ASHIATO_EXPECT_CERT_SHA256`・開発版は `ASHIATO_EXPECT_DEV_CERT_SHA256` と比べ、無いか違えば署名しない。release.yml の署名の段に指紋の変数を渡し、check-guards の workflows がその有無を見る。self-test に「識別子とチャンネルは合っていて鍵だけが違う」2 本と「期待の指紋が無い」を足した。開発版の変数名は docs/release.md と docs/handoff/ST54.md に書いた（2f284a59）

## R27. D3 の「有効期限 25 年以上」を確かめる下限は、どの試験にも固定されていない（下限を 1 年にしても、作る鍵と下限を両方下げても緑）
- 成果物: tools/release/make-key.sh
- 根拠: 複製で make-key.sh:22 の `+25 years` → `+1 years` → `make-key.sh --self-test` **rc=0**。さらに make-key.sh:37 の `-validity 10000` → `-validity 400` も同時に変える → **rc=0**（約 1.1 年の鍵を作り、確かめも通る）。
  self-test が確かめに当てる鍵は、10000 日（約 27.4 年）の 3 本（作った鍵・2048 bit・`-startdate 2024/01/01`）だけで、25 年に満たない鍵が無い。Scenario「決めた長さに満たない鍵を確かめで落とす」は長さだけで、期限の下限は誰も観測していない。
  spec は「作った鍵の種類・長さ・有効期限を読み直して確かめ、満たさなければ失敗する」。鍵は作った後に替えられない（D3 の反転条件「作った後は替えない」・扉 34）
- kind: technical
- 提案: self-test に `-validity 9000`（約 24.6 年）の 4096 bit の鍵を作り、`verify` が rc≠0 で「25 年未満」と出すことを見る 1 本を足す
- 処置: fixed 7.1 —— self-test に 4096 bit・`-validity 9000`（約 24.6 年）の鍵を足し、`verify` が「25 年未満」で落ちることを見る。下限を `+1 years` に・作る鍵を `-validity 400` にした複製で self-test が rc=1 になることを確かめた（2f284a59）

## R28. 「開発版の印は、配った印に数えない」（配った移行の側）を、印の型を `*` にしても self-test が通す
- 成果物: tools/check-released-migrations.sh
- 根拠: 複製で check-released-migrations.sh の `git tag --list 'v[0-9]*'` と `git tag --merged HEAD --list 'v[0-9]*'`（:27・:34）を `'*'` にした → `--self-test` **rc=0**（`check-released-migrations self-test OK`）。
  self-test のこの木は「移行を 1 つ置いて HEAD に `nightly` を打つだけ」（:160-161）で、`nightly` を配った印に数えても比べる差が無いので通る。spec の THEN「配った印が 0 個として通る」を観測していない（送信契約の側は `version.sh order` が `nightly` を並べられずに落ちるので固定されている）
- kind: technical
- 提案: `nightly` を打った後に、その commit の移行の中身を変える commit を足した木で rc=0 を見る（`nightly` を数えれば落ちる）。または出力に「配った印が 0 個」があることを見る
- 処置: fixed 4.1 —— self-test の nightly の木に、印の後で配った移行の中身を変える commit を足した。印の型を `'*'` にした複製で self-test が NG になることを確かめた（2f284a59）

## R29. 配る組み立ての Scenario を持つ self-test 4 本（`build.sh`・`check-artifacts.sh`・`sign-apk.sh`・`make-key.sh`）は、CI のどの job でも走らない
- 成果物: .github/workflows/ci.yml / tools/release/build.sh / tools/release/check-artifacts.sh / tools/release/sign-apk.sh / tools/release/make-key.sh
- 根拠: `grep -n "self-test" .github/workflows/ci.yml` → ST52 の台本で並ぶのは `check-version.sh`・`version.sh`・`check-guards.sh`・`package.sh`・`draft.sh` だけ（ci.yml:434-440）。final review R9 の fix（453323d5）もこの 4 本を足していない。
  この 4 本が印を持つ Scenario（`.dirty` / `.debug` / 資格情報 / 綺麗な配布物 / 欠けた配布物 / 4 つの名前 / nightly の 2 本 / 署名の 5 本 / 鍵の 2 本 / 桁の超過の gradle）は、tasks の検証として手元で 1 回走っただけで、後の PR がこれらを壊しても CI は気づかない。
  （R25 の変異を CI のどこが最初に捕まえるかは走らせていない）
- kind: technical
- 提案: `package-linux`（Android SDK と Rust が既にある job）で `check-artifacts.sh --self-test`・`sign-apk.sh --self-test`・`make-key.sh --self-test` を走らせる（`build.sh --self-test` は `check-artifacts.sh` の self-test が同じ組み立てを含む）。重さで外すなら、外す理由と代わりの担保を tasks に書く
- 処置: fixed 9.2 —— ci.yml の package-linux で `check-artifacts.sh`・`sign-apk.sh`・`make-key.sh` の `--self-test` を走らせ（`build.sh` の組み立ては check-artifacts の self-test が含む）、check-guards の workflows がこの 3 本の有無を見る（消した複製で rc=1）。CI の runner には `scripts/quiet-run` が無いので sign-apk の self-test は自分で控えに書く形にした（2f284a59）

## R30. deep.md が「change で拾う」とした K1（確認バッチの準備が、繋がった端末に debug の鍵の版を入れる）が、どの change にも Story にも載っていない
- 成果物: tools/verify-prep.sh / docs/audit/2026-11-01-st52-door34-consistency.md / openspec/changes/st52-release-channels/deep.md
- 根拠: deep.md:129「K1（`tools/verify-prep.sh` が本人の携帯に debug 鍵の版を入れうる）は change で拾う」。`docs/changes/` は CH04〜CH08 だけ（origin/main も同じ）、`git log --all --oneline | grep -iE "verify-prep|K1"` に該当なし、`docs/handoff/ST5*.md` に `verify-prep` も st52 の K1 も無い。
  `tools/verify-prep.sh:55-62` は今も `adb devices` の最初の端末へ `adb -s "$device" install -r app-debug.apk` を入れる（この change で変わっていない。`git diff 26aaf500 HEAD -- tools/verify-prep.sh` は空）。
  Q1 の答え（本人の携帯にまだ入っていない）と扉 34 ★「最初から正式な鍵の版」の下では、確認バッチの間に本人の携帯を繋ぐと debug の鍵の版が入り、正式な鍵の版へ移るときに入れ直し（未送信を失いうる）が要る状態がもう一度できる
- kind: defer
- 提案: `scripts/hx change` で K1 を CH として起こす（監査の「直すところ」と「機械」の欄がそのまま依頼になる）。それまでは ST53 の確認の手順に「確認バッチの準備で本人の携帯を繋がない」を書く
- 処置: followup ST53 —— tasks.md は凍結しているので docs/handoff/ST53.md に K1 の節（ST53 の上流で引き受けるか `hx change` で CH を起こす・それまでは確認バッチで本人の携帯を繋がない）を書いた

## security review（26aaf50..3025eae）

見た経路:
- 外 → `strip_api_prefix`（lib.rs:4223-4240。`/api` を剥がすだけ）→ `api_router` の route（handler の認可は今までと同じ）→ 応答。剥がしは route の前なので、`/api/<道>` と `/<道>` は同じ handler・同じ `refresh_cookie` / `access_log` を通る（4）
- 外 → route に当たらない道 → `web_assets::fallback`（web_assets.rs:50-78）→ build 時に `include_bytes!` した表の**完全一致**だけを返す。ファイル系を読まないので `../` は届かない。無ければ 404、GET/HEAD 以外は 405（3）
- `ASHIATO_WEB_DIST`（`build.sh` の `npm run build` の出力の写し）→ `build.rs` の `collect_files` → サーバの実行ファイルに埋め込み → 認証なしで配る。中身は vite の束で記録は含まない（`web/public` は無い）（2）
- tag `v*` の push → release.yml `plan`（`version.sh plan` が `^v[0-9]+\.[0-9]+\.[0-9]+(-beta\.…)?$` で形を固定。version.sh:90）→ `${{ needs.plan.outputs.version }}` を `run:` に埋める。形が固定なので shell に入る字は数字・`.`・`-beta.` だけ（3）
- release.yml `publish`（`environment: release`）→ Secret 4 つを env で `sign-apk.sh` へ → `mktemp` の鍵（0600）→ `apksigner` → `trap` で消す。ログに鍵・合言葉を出す行は無い（2）
- 収集アプリの APK の接続先（`BASE_URL` / `API_TOKEN` / `USER_ID`）→ `build.sh` が `-Pashiato.*=` を空で渡す → `check-artifacts.sh` の `check_apk_fields` が空であることを見る（2）
- 公開の repo への fork の PR → ci.yml `pull_request` → `package-linux` / `package-windows` / `package-artifacts` → artifact `release-package` → `draft.sh <run-id>` → `gh release create --draft --target main`（R31）（3）
- nightly.yml `publish` → `gh release upload` / `git push --force …refs/tags/nightly`（`contents: write` はこの job だけ。`vars.ASHIATO_NIGHTLY_PUBLISH == 'on'` で止まる）（2）

安全の決め手: 「`/api` を剥がすのは route を選ぶ前で、剥がした後は既存の handler の認可だけが道を決める。route に当たらない道は、build 時に埋め込んだ画面の表の完全一致しか返さない」 —— 確かさ 4。
st52_web_tests.rs に一時的にテストを足して走らせ（終えた後に `git checkout` で戻した）、認証なしで、データを読む GET 22 本の道を 7 通りの書き方（`/api<p>`・`?x=1` 付き・`/api/<p>`・`/api/api<p>`・`/API<p>`・末尾 `/`・`/api/.<p>`）で、書き込みの POST 12 本を `/api<p>` で叩いた。
結果は `scripts/quiet-run secprobe -- cargo test -p ashiato-server --lib st52_secprobe_unauth -- --nocapture` → rc=0。どの道も 200 にならず、401・400（`/api/coverage` は引数の検査が先で、56 byte の引数の誤りを返す。記録は出ない）・404 のどれか。`GET /api/session` は 401（ログ: `.superpowers/sdd/logs/20261102-081503-secprobe.log`）

## R31. `draft.sh` は run-id の出所を確かめないので、公開の repo への fork の PR が組んだ配布物を、この repo の下書きのリリース（`--target main`）に載せられる
- 成果物: tools/release/draft.sh / .github/workflows/ci.yml
- 根拠: `gh repo view --json visibility` → `PUBLIC`。ci.yml:4 は `pull_request` で走り、追加された `package-linux` / `package-windows` / `package-artifacts`（ci.yml:350-397）には fork を除く `if:` が無く、PR の枝のコードで組んだものを `release-package` として上げる。
  draft.sh:13-22 が見るのは `run_id` が数字であることだけ（`gh run view --json headRepository,event,headBranch` のような出所の確かめは無い）。`gh run download "$run_id" --name release-package` → `check-artifacts.sh --allow-debug-key`（印・名前・接続先の欄・debug の鍵を認める検査。誰が組んだかは見ない）→ `gh release create v<版> … --draft --target main`。
  手順（docs/release.md:88・95）は run-id を人間が選ぶだけで、どの run を選ぶべきかを書いていない。下書きは外からは見えないが、そのまま「公開」を押すと、fork のコードで組んだサーバ・APK が、この repo の `v<版>` として配られる（確かさ 3。fork の run を実際に取り寄せてはいない）
- kind: technical
- 提案: draft.sh で `gh run view "$run_id" --json headRepository,event,conclusion` を読み、`headRepository` がこの repo で、`conclusion == success` のときだけ進める。self-test の偽の gh に「fork の run」を返させて落ちることを見る 1 本を足す
- 処置: fixed 10.1 —— `draft.sh` が `gh api repos/{owner}/{repo}/actions/runs/<run-id>` で run の出所を読み、head の repo がこの repo・workflow が `ci`・結論が `success` のときだけ取り寄せる。self-test に fork・ci 以外・失敗の 3 つの run を足し、どれも下書きを作らないことを見る。docs/release.md §8 に選ぶ run を書いた（2f284a59）

範囲外: なし

## /code-review（26aaf50..3025eae）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R32. stack.sh decides whether to rebuild the server only from file mtimes. Whether the existing binary actually has the UI embedded (ASHIATO_WEB_DIST) is never checked, so a server built without the UI gets reused.
- 成果物: tools/stack.sh:71
- 根拠: Some other step builds target/release/ashiato-server without ASHIATO_WEB_DIST: tools/record-env.sh:150, a developer's plain `cargo build --release`, or build.rs re-running when the env var changes. That binary is newer than web/dist and the sources, so newer_src is false and stack.sh starts it as is. `/` and `/#/coverage` return 404, and web/e2e/st52-served-ui.spec.ts fails or the operator sees no UI.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 5.3 —— 再現: 画面なしで組んだバイナリが web/dist より新しいと、時刻の比べだけでは作り直さない。stack.sh が画面を組み込んで組んだときだけ印（`target/release/.ashiato-server.web-dist`）に画面の置き場を書き、印が無い・置き場が違う・バイナリが印より新しいなら作り直す（4 通りを使い捨ての木で確かめた）（2f284a59）

## R33. The release section's first scenario ('a given version goes into all 4 stamps') only runs with ASHIATO_BUILD_VERSION set. CI never sets it, so that block repeats the no-env block and the given-version path is never tested.
- 成果物: tools/check-build-stamp.sh:183
- 根拠: The build-stamp job runs `./tools/check-build-stamp.sh release` with no ASHIATO_BUILD_VERSION, so expected_version=cargo_version. A regression where, say, gradle or vite ignores ASHIATO_BUILD_VERSION still passes CI. CI also pays for two full 4-component builds that check the same thing.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.4 —— 再現: ci.yml の build-stamp の job は版番号を渡さず、release 節は次の節と同じものを見ていた。R25 と同じ直し（2f284a59）

## R34. The invalid-shape loop uses 0.4.0-* versions, but the source version is 0.1.0. Each build fails on the base mismatch whether or not the shape check works, so the scenario proves nothing about shape rejection.
- 成果物: tools/check-build-stamp.sh:235
- 根拠: Remove the beta 1..98 or dev.YYYYMMDD checks from parse_version in crates/build-stamp. `ASHIATO_BUILD_VERSION=0.4.0-beta.99` still fails because 0.4.0 != 0.1.0, so the scenario '決めた形でない版番号を渡すと、組み立てが失敗する' stays green. It should use `${cargo_version}-beta.99` and similar.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.4 —— 再現: 出所は 0.1.0 で、`0.4.0-*` は形の検査の前に出所との照合で落ちる。形を崩す版番号を `<出所>-rc.1` などにした（2f284a59）

## R35. The new shallow-history guard fails open: if `git rev-parse --is-shallow-repository` errors, `|| echo false` treats the repo as not shallow. check-released-migrations.sh and version.sh were deliberately made fail-closed for this case (final review R17).
- 成果物: tools/check-contract-version.sh:107
- 根拠: On an old git or a broken repo state, rev-parse fails and the check goes on to count tags as if history were full. That is the exact case the guard was added to stop, and it behaves differently from the sibling scripts.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 4.2 —— 再現: `|| echo false` で問い合わせの失敗を浅くないとみなしていた。check-released-migrations.sh と同じく失敗で落とす（2f284a59）

## R36. version_code/version_sort_key do bash arithmetic on regex captures that may have leading zeros, so they are read as octal. This disagrees with the Kotlin, Rust and TS parsers, which use decimal.
- 成果物: tools/release/version.sh:30
- 根拠: Version 0.010.0 passes parse_version and check-version.sh (`^0\.[0-9]+\.[0-9]+$`). bash computes MINOR=8, giving 80099, while gradle computes 100099, so check-artifacts `apk-version` reports a versionCode mismatch. 0.08.0 makes `(( 08 <= 99 ))` error, and it is reported as 'MINOR が上限を超えた'.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 1.1 —— 再現: `0.08.0` の比較が bash の 8 進数の誤りで落ち、`0.010.0` は 80099 になった。parse_version で `10#` で読み、self-test に `0.010.0 = 100099`・`0.08.0 = 80099` を足した（`10#` を外した複製で rc=1）（2f284a59）

## R37. The web_dist temp directory from mktemp is removed only by cleanup_build_tmp at the end of a successful build_server. There is no trap, so a failed npm or cargo build (set -e exit) leaves it behind.
- 成果物: tools/release/build.sh:23
- 根拠: A cargo build failure in server-linux or server-windows (for example ASHIATO_BUILD_VERSION not matching the source) exits before cleanup_build_tmp. A full copy of web/dist is left in $TMPDIR on every failed run, which trips the project's tmp-leak checks (HARNESS_TMP_LEAK=fail).（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 6.1 —— 再現: cargo を落とす偽の命令で、`$TMPDIR` に `ashiato-web.*` が残った。組み立ての入口で `trap cleanup_build_tmp EXIT` を張り、同じ手で残らないことを確かめた（2f284a59）

## R38. This new file only does `import "./buildStamp.test"`, so vitest registers and runs every suite in buildStamp.test.ts twice.
- 成果物: web/src/__tests__/build-stamp.test.ts:2
- 根拠: Each describe/it in buildStamp.test.ts, including 'the three assemblies carry the embedded stamp', runs twice on every `npm test`. That doubles the time, and one failure shows up as two.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.2 —— 再現: build-stamp.test.ts が buildStamp.test.ts を import し、両方が走っていた。中身を build-stamp.test.ts へ移し buildStamp.test.ts を消した（12 passed）（2f284a59）

## R39. The web job still runs `cargo build --release` for the server before `npm run build`. The new web/dist is then newer than the binary, so tools/stack.sh always rebuilds the server with ASHIATO_WEB_DIST, and the first release build is wasted.
- 成果物: .github/workflows/ci.yml:233
- 根拠: Every CI web job compiles ashiato-server in release mode twice: once without the UI, then again inside stack.sh with the UI. The first one should be dropped, or run after `npm run build` with ASHIATO_WEB_DIST set.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 5.3 —— e2e の job の画面なしの `cargo build --release` を消し、stack.sh が画面を組み込んで 1 度だけ組む（2f284a59）

## R40. The sed/regex that reads the [workspace.package] version from Cargo.toml is copied again in check-openapi.sh, check-version.sh, web/e2e/st49-client-version.spec.ts and web/build-stamp.ts. `tools/release/version.sh source` and build-stamp.ts's sourceVersion already do this.
- 成果物: tools/check-openapi.sh:6
- 根拠: If the Cargo.toml layout changes (for example `version.workspace` or a different key order), the 5+ copies need separate fixes and can drift apart, and the checks then disagree on what the source version is. The bash callers should call `tools/release/version.sh source`, and the e2e spec should import sourceVersion from build-stamp.ts.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.3 —— bash の写し（check-openapi.sh・check-version.sh）は `tools/release/version.sh source [<Cargo.toml>]` を呼び、e2e は build-stamp.ts の `sourceVersion` を import する。残る読み方は Rust（build-stamp crate）・Kotlin（gradle）・TS（build-stamp.ts）・bash（version.sh）の言語ごとに 1 つずつで、どれも同じ表と self-test で確かめる（2f284a59）

## R41. The standalone `providers.environmentVariable("ASHIATO_BUILD_VERSION")` line is dead code. buildVersionProvider.get() is already called at configuration time, and that call is what registers the env var as a configuration-cache input.
- 成果物: collector-android/app/build.gradle.kts:62
- 根拠: It does nothing, and its comment suggests the env var would otherwise go untracked, which is wrong and misleads later maintainers. It also creates a second, unused provider for the same env var. It should be deleted.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.1 —— 設定時に `buildVersionProvider.get()` を呼ぶので、環境変数は既に設定の入力になる。使われない行と誤った注記を消した（2f284a59）
