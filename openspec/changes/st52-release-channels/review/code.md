# ST52 code review

## final review（26aaf50..80c6603）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 2 / Important 9 / Minor 12。判定は「No（直してから）」。
reviewer が手元で走らせたもの: `version.sh` / `check-guards.sh`（`--self-test` と `workflows`）/ `package.sh` / `draft.sh` / `check-version.sh` の `--self-test` はどれも rc=0、
`check_scenarios.py` OK。修正の枝の判定は CI と同じ形の clone で、規則の API は `gh api` で確かめた。
Task ごとの ⚠️ は final reviewer が全体で確かめた。成り立ったのは Task 1（--self-test が CI で走らない → R9）、Task 5（BIND → R2）、Task 9（`rules/tags` の API → R4）、
Task 10・11（§8 → R4・R5・R6）、Task 11（修正の枝 → R3）。そのほかは解消済みか当たらない（Task 2 の 0 tests は報告の誤読、Task 4 の fetch-depth は浅くならない、
Task 6 の R8 は minify なし、Task 8 の版番号の照合は枝の判定より前に走る、Task 9 の gnu の target は使われない）。

## R1. 配る組み立てが画面の依存を入れないので、package の job がどれも落ちる
- 成果物: tools/release/build.sh / .github/workflows/ci.yml / release.yml / nightly.yml
- 根拠: tools/release/build.sh:21-25（`build_web` は `npm run build` だけ）。package-* の job に setup-node も `npm ci` も無い。新しい runner に `web/node_modules` は無い / Task 6 F13（未修正）
- kind: technical
- 処置:

## R2. `st52-served-ui.spec.ts` は CI では BIND が無く必ず落ちる
- 成果物: web/e2e/st52-served-ui.spec.ts
- 根拠: web/e2e/st52-served-ui.spec.ts:10-12（`process.env.BIND` が無いと throw）。CI の e2e job も `tools/ci-db-env.sh` も BIND を出さない。`stack.sh:33` の export は webServer の子プロセスにしか効かない / Task 5 ⚠️
- kind: technical
- 処置:

## R3. 修正の枝の印は CI の plan で必ず落ちる（枝を `refs/heads/release/*` からしか探さない）
- 成果物: tools/release/version.sh
- 根拠: tools/release/version.sh:95。CI と同じ形の clone（`+refs/heads/*:refs/remotes/origin/*` と tags、印を detached で checkout）で `plan v0.4.1` が「NG: 印の commit が main の祖先でない」rc=1 / Task 11 ⚠️
- kind: technical
- 処置:

## R4. `rules/tags/<印>` の API は無く、正式版の publish はすべて `check-guards.sh rules` で止まる
- 成果物: tools/release/check-guards.sh / docs/release.md / openspec/changes/st52-release-channels/tasks.md（12.3）
- 根拠: tools/release/check-guards.sh:14。`gh api repos/cli/cli/rules/branches/trunk` は配列を返すが `gh api repos/cli/cli/rules/tags/v2.0.0` は 404。あわせて `rules` は `GITHUB_REF_NAME` が無いと何も問い合わせずに落ちる（:9-11）ので docs/release.md §4・§8 と tasks 12.3 の呼び方は通らない。tasks 12.3 の repo 名が `ashiato`（既定は `ashiato2`）/ Task 9 ⚠️
- kind: technical
- 処置:

## R5. 確認バッチ 12.1 は書かれたとおりには通らない（draft.sh の版番号と PR の package の版番号が食い違う）
- 成果物: tools/release/draft.sh / docs/release.md §8 / tasks.md 12.1 / design.md D16
- 根拠: PR の package は `ASHIATO_BUILD_VERSION` を渡さず出所の版（`0.1.0`）で組む（D14・package.sh:168-173）。`draft.sh <run-id> 0.1.0-beta.1` は `check-artifacts --expect-version 0.1.0-beta.1` を当てるので「欠けた配布物」で必ず落ちる。self-test の偽の gh は beta の名前で作るので見えない
- kind: conflict
- 処置:

## R6. docs/release.md §8 の印の数え方が P16（下書きは印を作らない）を確かめていない
- 成果物: docs/release.md
- 根拠: docs/release.md §8 で `before=` を draft.sh の**後**で数えている。tasks 12.1 は「作る前と後」
- kind: technical
- 処置:

## R7. publish と package-artifacts で Android SDK の道具（apksigner / apkanalyzer）が PATH に無い
- 成果物: tools/release/check-artifacts.sh / tools/release/sign-apk.sh / workflow
- 根拠: check-artifacts.sh の `command -v apksigner`（fallback 無し）と `apkanalyzer`。apksigner は build-tools の下でどの runner の PATH にも無い。`--allow-debug-key` を付けない経路は self-test でも走っていない（Task 6 F10）。sign-apk.sh には `$ANDROID_HOME/build-tools` を探す fallback がある
- kind: technical
- 処置:

## R8. ベータ番号の範囲が spec（1〜98）と違い 1〜89 になっている
- 成果物: crates/build-stamp/src/lib.rs / tools/release/version.sh / web/build-stamp.ts / collector-android/app/build.gradle.kts / tools/release/version-table.tsv
- 根拠: specs/distribution/spec.md:8・design.md:85 は「N は 1〜98」。実装は build-stamp `lib.rs:61`（1..=89）・`version.sh:18`（`[1-9]|[1-8][0-9]`）・web・gradle が同じ誤り。versionCode の式（beta.98 → …98 < 正式 …99）は 98 まで成り立つ
- kind: technical
- 処置:

## R9. 判定の台本の --self-test が CI で走らない
- 成果物: .github/workflows/ci.yml
- 根拠: CI に入っているのは `check-version.sh --self-test` だけ。`version.sh`・`check-guards.sh`・`package.sh`・`draft.sh` の軽い --self-test は走らない（D8「判定は台本に置き --self-test で確かめる」）/ Task 1 ⚠️
- kind: technical
- 処置:

## R10. 自己試験が `0.1.0` を決め打ちにしていて、出所の版を上げると壊れる
- 成果物: tools/release/build.sh / check-artifacts.sh / sign-apk.sh / draft.sh
- 根拠: build.sh:136-145、check-artifacts.sh の self_test、sign-apk.sh:58、draft.sh の偽の gh。版の check は生きた Cargo.toml と比べるので、docs/release.md の「正式版の後に出所の版を上げる」で 4 本が落ちる（Task 6・7・10 の Minor）
- kind: technical
- 処置:

## R11. nightly の公開で配布物が消えたまま残りうる（C7 違反）、最初の公開も落ちる
- 成果物: .github/workflows/nightly.yml / docs/release.md §5
- 根拠: nightly.yml:149-155 は前の配布物を消してから upload する。upload が落ちると空のまま残る。`gh release upload nightly` はリリースがあることを前提にし、作る手順がどこにも無い
- kind: technical
- 処置:

## R12. `0.1.0-beta.+5` をサーバと PC だけが受ける
- 成果物: crates/build-stamp/src/lib.rs
- 根拠: crates/build-stamp/src/lib.rs:60 の `"+5".parse::<u8>()` が通る / Task 2 F6
- kind: technical
- 処置:

## R13. access_log の `/api` を剥がす処理は通らない
- 成果物: crates/server/src/access_log.rs
- 根拠: access_log.rs:153-157。`strip_api_prefix` が URI を書き換えた後に走る / Task 5 F11
- kind: technical
- 処置:

## R14. `web_assets::fallback` が method を見ず、POST にも画面を 200 で返す
- 成果物: crates/server/src/web_assets.rs
- 根拠: web_assets.rs の fallback / Task 5 F10
- kind: technical
- 処置:

## R15. サーバが配る画面に Host の許可（vite preview の `allowedHosts` に当たるもの）が無い
- 成果物: crates/server/src/web_assets.rs
- 根拠: final reviewer の指摘（Minor）。cookie は host に縛られ、API は認可が要るので危険は小さい
- kind: technical
- 処置:

## R16. check-artifacts.sh の self-test が追跡中の `crates/server/src/lib.rs` に書き足し、trap で戻さない
- 成果物: tools/release/check-artifacts.sh
- 根拠: Task 6 F15
- kind: technical
- 処置:

## R17. version.sh の未使用の変数と、`--is-shallow-repository` の失敗を浅くないとみなす扱い
- 成果物: tools/release/version.sh / tools/check-released-migrations.sh
- 根拠: version.sh:91 の `expected` は未使用。version.sh:71 と check-released-migrations は `git rev-parse --is-shallow-repository` が失敗すると浅くないとみなす / Task 4 F8
- kind: technical
- 処置:

## R18. version.sh が `refs/tags/nightly` を `^{commit}` 無しで引いている
- 成果物: tools/release/version.sh
- 根拠: version.sh:121 / Task 1 F8
- kind: technical
- 処置:

## R19. check-guards.sh の sed の検査が恒に偽で何も確かめていない・`--repo x rules` の形が落ちる
- 成果物: tools/release/check-guards.sh
- 根拠: check-guards.sh:116・:136
- kind: technical
- 処置:

## R20. Windows の手順が pwsh から bash を呼び、使われない gnu の target を入れている
- 成果物: .github/workflows/ci.yml / release.yml / nightly.yml
- 根拠: Windows の package の手順は既定の pwsh。`targets: x86_64-pc-windows-gnu` は windows_target が native を返すので使われない / Task 9 F14
- kind: technical
- 処置:

## R21. build-stamp の job に setup-node が無い（vite 7 は node 20.19 / 22.12 以上が要る）
- 成果物: .github/workflows/ci.yml
- 根拠: check-build-stamp.sh は自分で npm ci するが node の版は runner 任せ / Task 3 ⚠️（一部成り立つ）
- kind: technical
- 処置:

## R22. draft.sh が `--notes` を渡さず、検査していないファイルも載りうる
- 成果物: tools/release/draft.sh
- 根拠: TTY から走らせると gh が本文を対話で聞く恐れ。`"$work"/*` で載せる / Task 10 F4・F5
- kind: technical
- 処置:

## R23. tasks.md の 9.1 が `[ ]` のまま（証跡は PASS）
- 成果物: openspec/changes/st52-release-channels/tasks.md
- 根拠: tasks.md の 9.1 / final reviewer の指摘（Minor）
- kind: technical
- 処置:
