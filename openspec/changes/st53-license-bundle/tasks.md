# ST53 実装タスク — 配布物に許諾が揃い、Google Play 開発者サービスとの組み合わせを許す

読む順: `deep.md`（**本人の答え Q1〜Q9**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/distribution/spec.md` → `specs/verify-environment/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提**: requires の ST52 は archive 済みで main にある。配る道具（`tools/release/`）・workflow（`release.yml`・`nightly.yml`・`ci.yml` の `package` の job）・手順書（`docs/release.md`）がそこにある。

検証は各タスクの本文に書いてある。「動いた」ではなく、コマンドと終了コードで判定する。
件数つきの検証は、試験を走らせた後に、`-- --list` の本数（gradle は結果の XML）が下限以上かを同じコマンドの中で見る（0 本でも rc=0 になるため）。一時ファイルは `$TMPDIR` に書く（`/tmp` の固定名を使わない）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** bash は `echo "== Scenario: …"`、Kotlin / Rust はコメント。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせる。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 51 本**（`distribution` 48・`verify-environment` 3）。どれも ADDED で、**全部このファイルのどこかの Task が名指しする**
- **本人の決定（下流は変えない）**: Q1 許可の文は英文の案を正本にし、日本語は README の参考訳 / Q2 許可が受けるのは `play-services-location`・`-base`・`-basement`・`-tasks` の 4 つの名指し /
  Q3 Environment `release` に本人を必須の承認者として付け、エージェントの資格情報を管理と承認の権限の無いものに替えてから鍵を置く / Q4 最初の本物の公開は ST53 が main に入った後に本人が行う（確認バッチは下書き）/
  Q5 ソースと部品の表示を `AGPL-3.0-only WITH AdditionRef-ashiato-play-services` に / Q6 携帯の許諾は APK の中に置くだけ（画面は作らない）/ Q7 正式版・ベータ版は印、それ以外は commit でソースの場所を示す /
  Q8 確認バッチの準備はエミュレータにだけ APK を入れる / Q9 サーバの画面にソースの場所を出さない。
  第 2 回: Q10 移行の 3 ファイルは書き換えず例外に載せる（最初の印から後は前進側の 2 つを直せないことを受け入れる。D4）/ Q11 表示の検査は先頭 25 行・表示の無いファイルは見ない・例外は wrapper の 2 つと移行の 3 つ（D5）/
  Q12 携帯の POM に許諾が 2 つ以上並べば AND で読む。`@pom` を取り寄せ、取れなければ落とす（D7）/ Q13 携帯の依存の一覧は検査と同じ出力から（D8）/
  Q14 PR の組み立ての `SOURCE.txt` は PR の head の commit（D9）/ Q15 開発版と main への push は main の commit、PR は head の commit（D16）。
  第 3 回: Q16 PR の組み立ては head の commit を checkout して組む（合わせた merge の commit からは組まない）。`SOURCE.txt`・組み立ての印・組んだソースが同じ commit を指し、組み立ての印との比べはすべての組み立てで残す（D9。D17 は外した）。
  仮: D6（自分の部品は名前で通し、`AGPL-3.0-only` を許可の一覧から外す）・D10（PR の成果物の掃除）・D11（`approvals` の中身）。
  第 4 回: Q17 配布物を検査する `package-artifacts` は合わせた木のまま走らせ、期待する版番号と比べる `LICENSE` だけを配布物を組んだ commit から取る（要件は変えない。D18）。
  （第 1 回は推奨で先に書いた。推奨と違った答えは、上流が写してから下流へ渡す）
- **名前は統一する**: 正本 `LICENSES/AdditionRef-ashiato-play-services.txt`、式 `AGPL-3.0-only WITH AdditionRef-ashiato-play-services`、配布物の 3 点 `LICENSE`・`THIRD-PARTY-NOTICES.txt`・`SOURCE.txt`（APK は `assets/licenses/`）、
  道具 `tools/check-spdx.sh <license|headers>`・`tools/check-licenses.sh [rust|node|android]`・`tools/licenses/pom-license-names.tsv`・`tools/licenses/texts/<SPDX>.txt`・`tools/release/notices.sh <server|collector-windows|apk|source|commit>`・`tools/verify-prep-device.sh`、
  gradle の task `ashiatoDependencyPoms`・property `ashiato.licenseAssets`、環境変数 `ASHIATO_SOURCE_REF`・`ASHIATO_SOURCE_REPO`・`ASHIATO_RELEASE_REVIEWER`（commit を渡す環境変数は足さない。commit は組み立ての印と同じ出所。D9）、`check-guards.sh` の副コマンド `approvals`・`agent-credential`
- **判定の論理は台本に置き、workflow の YAML に書かない**（ST52 の D8）。台本の `--self-test` は、使い捨ての git の木・偽の `gh` / `adb`（`PATH` の先に置く台本）・偽の入力のファイルで閉じる
- **下流の実装の中では配布元に何も書かない**: Environment・承認者・ルールセット・下書き・印・鍵・資格情報・成果物の削除は、どれも本人の手か確認バッチ（末尾の「人間の確認待ち」）
- **CLA（`docs/CLA.md`）を書き換えない。** `CONTRIBUTING.md` の受け入れ条件（CLA の節）も変えない（壊してはいけないもの）
- **依存の許諾の検査の許可の一覧は、`AGPL-3.0-only` を外す以外は 1 つも変えない**（壊してはいけないもの。D6（仮））
- **一括の書き換えは `git ls-files` を起点にし、symlink（mode `120000`）を辿らない**（`scripts`・`.claude/skills` などは harness2 への symlink）
- **新しい Rust の crate・npm の部品・Android の依存を足さない**（gradle の task は標準の API だけで書く）
- **長いコマンドは `scripts/quiet-run`**（cargo test・gradle・`tools/smoke.sh`）

## Task 1: 追加の許可の正本と、`LICENSE`・README・CONTRIBUTING

- [x] 1.1 `LICENSES/AdditionRef-ashiato-play-services.txt` を作る（D1・D2）。`Listed libraries:` より前は、`specs/distribution/spec.md` の最初の Requirement の英文を 1 字も変えずに写す（段落の間は空行 1 つ）。`Listed libraries:` の節には、`com.google.android.gms:play-services-location`・`-base`・`-basement`・`-tasks` を 1 行ずつ書く（`- ` で始め、座標だけ）。
  `LICENSE` を「正本の全行 → 空行 2 つ → 今の AGPL-3.0 の全文（変えない）」にする。
  `README.md` のライセンスの節を書き直す（AGPL-3.0 と追加の許可を名指しする・正本は英文・日本語の参考訳・正本へのリンク）。`CONTRIBUTING.md` の冒頭の許諾の文に、貢献も同じ許諾（追加の許可を含む）の下で配ることを書き足す
  検証: `bash -c 'n=$(wc -l < LICENSES/AdditionRef-ashiato-play-services.txt); head -n "$n" LICENSE | cmp - LICENSES/AdditionRef-ashiato-play-services.txt'` rc=0 / `bash -c 'grep -q "AdditionRef-ashiato-play-services" README.md && grep -q "参考訳" README.md && grep -q "追加の許可" CONTRIBUTING.md'` rc=0 / `bash -c 'git diff --quiet origin/main -- docs/CLA.md'` rc=0 / `bash -c 'diff <(git show origin/main:CONTRIBUTING.md | sed -n "/^## /,\$p") <(sed -n "/^## /,\$p" CONTRIBUTING.md)'` rc=0（最初の見出しより後ろ —— CLA の節 —— は変えない。書き足すのは冒頭の許諾の文だけ）
- [x] 1.2 `tools/check-spdx.sh license` を作る（D1）。次の 2 つを見る。
  - `LICENSE` の先頭の N 行（N は正本の行数）が正本と同じこと
  - その後ろ（空行 2 つの後）の sha256 が、台本の中の定数と同じこと。定数は、今の `LICENSE`（AGPL-3.0 の全文）の sha256
  違えば rc≠0 で、どちらが違うかを示す。
  さらに、正本の `Listed libraries:` より前が台本の中の英文の定数（spec の英文の写し）と同じこと、`README.md` のライセンスの節が `AdditionRef-ashiato-play-services` と「参考訳」と「正本は英文」を持つことを見る。
  `--self-test` は使い捨ての木で次を走らせる。
  - `追加の許可の正本は、深掘りで決めた英文と同じ`（今の木で通る。1 語を変えた正本で落ちる）
  - `LICENSE の先頭は追加の許可の正本と同じ`
  - `LICENSE の許可の文を正本と変えると、許諾の文の検査が落ちる`（`but not a modified version` を消した `LICENSE`）
  - `LICENSE の AGPL の全文を変えると、許諾の文の検査が落ちる`（AGPL の全文の 1 行を書き換えた `LICENSE`）
  - `README のライセンスの節は、追加の許可と正本の言葉を示す`（参考訳の段落を消した README で落ちる）
  - `許可の文の名指しは、位置のライブラリとそれが引く 3 つだけ`（今の木の正本に、4 つの座標がちょうどあり、`play-services-ads`・`play-services-measurement` が無い）
  検証: `tools/check-spdx.sh --self-test` rc=0 / `tools/check-spdx.sh license` rc=0

## Task 2: ソースと部品の許諾の表示

- [x] 2.1 `crates/server/src/coverage/tests/mod.rs` の 21 行目の表示を先頭の行へ移す。`git ls-files -s` を起点に、mode が `120000` でないファイルの先頭 25 行にある `SPDX-License-Identifier: AGPL-3.0-only` を `SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services` に書き換える（Q5）。
  移行の 3 つ（`migrations/202609211400_browser_history_record_id.sql`・`.down.sql`・`migrations/202610181200_media_capture.sql`）は書き換えない（D4・Q10）。gradle の wrapper の 2 つ（`Apache-2.0`）にも触らない。
  根の `Cargo.toml` の workspace の `license`、`web/package.json` の `license`、`web/package-lock.json` の `packages[""].license` を同じ式にする。
  `docs/openapi.json` は、`src/bin/openapi.rs` で作り直す（`info.license.identifier` が同じ式になる）
  検証: `tools/check-spdx.sh headers` rc=0 / `bash -c 'n=$(git ls-files -s | awk "\$1 != \"120000\" {print \$4}" | grep -v -e "^migrations/" -e "^collector-android/gradlew" -e "^docs/" -e "^openspec/" | while read -r f; do head -n 25 "$f" | grep -q "SPDX-License-Identifier: AGPL-3.0-only *\$" && echo "$f"; done | wc -l); test "$n" -eq 0'` rc=0（例外の外に古い表示が 1 つも残っていない）/ `tools/check-openapi.sh` rc=0 / `bash -c 'cd web && npm ci && npm run build'` rc=0 / `bash -c 'cargo metadata --format-version 1 --no-deps --offline | python3 -c "import json,sys; s={p[\"license\"] for p in json.load(sys.stdin)[\"packages\"]}; sys.exit(s != {\"AGPL-3.0-only WITH AdditionRef-ashiato-play-services\"})"'` rc=0
- [x] 2.2 `tools/check-spdx.sh headers` を作る（D5・Q11）。`git ls-files -s` の mode `120000` を飛ばし、残りのファイルの先頭 25 行から `SPDX-License-Identifier:` の値を取る。
  決めた式でもなく、台本の中の例外の一覧（パスと許す値の組。wrapper の 2 つは `Apache-2.0`、移行の 3 つは `AGPL-3.0-only`）にも合わなければ、rc≠0 でパスと値を示す。通したときは、例外として通したファイルを `  例外: <パス> <値>` の形で 1 行ずつ出す。引数が無ければ `license` と `headers` の両方を走らせる。
  `--self-test` に、使い捨ての木で次の 4 つを足す。
  - `決めた許諾の表示でないソースがあれば、表示の検査が落ちる`（`GPL-3.0-only` の Rust のファイルを足す）
  - `表示の検査は symlink の先を読まない`（木の外の `GPL-3.0-only` のファイルへの symlink を足して rc=0）
  - `例外の一覧のファイルは、元の表示のままで表示の検査を通る`（今の木で rc=0 で、`  例外:` の行がちょうど 5 つ）
  - `例外の一覧のファイルでも、許す値と違う表示なら表示の検査が落ちる`（移行のファイルに `GPL-3.0-only` を書いた木）
  `tools/check-openapi.sh` の今の比べで、`API の契約の写しは、部品と同じ許諾の式を持つ` を確かめる。印は `tools/check-spdx.sh --self-test` の中に `echo "== Scenario: API の契約の写しは、部品と同じ許諾の式を持つ"` を置き、`docs/openapi.json` の `info.license.identifier` を読んで比べる
  検証: `tools/check-spdx.sh --self-test` rc=0 / `tools/check-spdx.sh` rc=0

## Task 3: 依存の許諾の検査の節と、自分の部品（Rust・Node）

- [ ] 3.1 `tools/check-licenses.sh` を、引数で節（`rust`・`node`・`android`）を選べる形にする（D15）。引数が無ければ 3 つ全部。中間のファイルは `$TMPDIR`（無ければ `mktemp -d`）に書き、`/tmp/meta.json` をやめる。
  SPDX の式の評価（今は 2 つの節に同じ関数がある）は、台本の中の 1 か所にまとめる。
  許可の一覧から `AGPL-3.0-only` を外す（D6（仮））。ほかの値は変えない。
  Rust の節は `workspace_members` の部品を、Node の節は根の部品を、許諾が `AGPL-3.0-only WITH AdditionRef-ashiato-play-services` のときだけ通す。
  試験のために、入力を差し替える口を持たせる。
  - `ASHIATO_LICENSES_CARGO_META`: `cargo metadata` の JSON の置き場
  - `ASHIATO_LICENSES_NODE_ROOT`: `node_modules` の置き場
  `--self-test` は、偽の入力で次の 3 つを走らせる。
  - `他人の部品の AGPL の許諾は、許諾の検査が落とす`（`AGPL-3.0-only` の他人の crate）
  - `自分の部品は、決めた許諾の式で許諾の検査を通る`（偽の workspace の部品と偽の根）
  - 自分の部品の名前で、決めた式でない許諾（`AGPL-3.0-only` だけ）を持つものが落ちること
  - `許可の一覧は、AGPL-3.0-only を外したほかは前と同じ`（台本の許可の一覧を、自己試験の中に書いた spec の 17 の値と集合で比べる）
  - `node_modules が無ければ、画面の節は通さずに落ちる`（`ASHIATO_LICENSES_NODE_ROOT` に無い置き場）
  検証: `tools/check-licenses.sh --self-test` rc=0 / `bash -c 'cd web && npm ci && ../tools/check-licenses.sh rust node'` rc=0 / `bash -c 'grep -q "^ALLOW=.*AGPL-3.0-only" tools/check-licenses.sh && exit 1 || exit 0'` rc=0

## Task 4: 携帯の依存の許諾（POM の書き出しと Android の節）

- [ ] 4.1 `collector-android/app/build.gradle.kts` に task `ashiatoDependencyPoms` を足す（D7・Q12）。
  - `releaseRuntimeClasspath` の解決済みの部品（`ModuleComponentIdentifier`）ごとに、detached configuration で `<group>:<name>:<version>@pom` を解決する（online なら gradle が `.pom` を取り寄せて cache に置く。`--offline` で cache に無ければ解決に失敗して task が落ちる。部品を飛ばさない）
  - `<licenses>` の `<name>`・`<url>` を読む。空なら `<parent>` を辿る（深さの上限 5）
  - property `ashiato.pomOut` の置き場に、`group:name:version<TAB>名前<TAB>URL<TAB>出所` の行を書く。出所は、部品自身の POM なら `self`、親から読んだなら `parent`。許諾が無ければ名前を `-` にする
  - property `ashiato.extraRepo` があれば、その置き場を maven の置き場として先に足す（自己試験の使い捨ての置き場のため。配る組み立ては渡さない）
  依存は足さない。
  検証（取り寄せる段 → 読む段）: `bash -c 'out="$TMPDIR/poms.tsv"; off="$TMPDIR/poms-offline.tsv"; scripts/quiet-run poms -- ./collector-android/gradlew -p collector-android -q :app:ashiatoDependencyPoms -Pashiato.pomOut="$out" && scripts/quiet-run poms-offline -- ./collector-android/gradlew -p collector-android --offline -q :app:ashiatoDependencyPoms -Pashiato.pomOut="$off" && cmp "$out" "$off" && grep -q "^com.google.android.gms:play-services-location:" "$out" && test "$(cut -f1 "$out" | sort -u | wc -l)" -ge 50'` rc=0
- [ ] 4.2 `tools/licenses/pom-license-names.tsv` を作る（D7・Q12）。今の依存の POM に出る名前を全部、SPDX の名前に対応させる（例: `The Apache Software License, Version 2.0` → `Apache-2.0`、`Android Software Development Kit License` → `PROPRIETARY`）。
  `tools/check-licenses.sh` の Android の節を作り、今の echo を消す。
  - 4.1 の task を走らせる（`--offline` を付けない。cache に無い `.pom` は取り寄せる。`ASHIATO_GRADLEW` で gradlew の場所を差し替えられる）
  - 名前を表で直し、`PROPRIETARY` は正本の `Listed libraries:`（D2）の `group:name` にある部品だけを通す
  - 1 つの部品が名前を 2 つ以上持てば、全部の名前が許可の一覧にあるか名指しに当たるときだけ通す（AND。1 つでも外れれば、その名前を示して落とす）
  - 表に無い名前・許諾の無い部品・名指しの外の `PROPRIETARY` は、座標と名前を示して落とす
  - task が落ちたか出力が空なら、「対象が無い」と示して落とす
  - 最後に、通した `PROPRIETARY` の部品を 1 行ずつ `  通した PROPRIETARY: <group>:<name>:<version>` の形で出す
  `ASHIATO_LICENSES_POM_LIST`（偽の依存の一覧）と `ASHIATO_LICENSES_ADDITION`（偽の正本）で入力を差し替えられるようにする。
  `--self-test` に次の 6 つを足す。
  - `名指しの外の Google のライブラリを足すと、許諾の検査が落ちる`
  - `対応表に無い許諾の名前の部品があれば、許諾の検査が落ちる`
  - `許諾の名前を 2 つ持つ部品は、片方でも許可の外なら許諾の検査が落ちる`
  - `許諾の名前を 2 つ持つ部品は、両方が許可されていれば通る`
  - `名指しの一覧を正本から消すと、その部品で許諾の検査が落ちる`
  - `検査する対象が無い節は、通さずに落ちる`（`ASHIATO_GRADLEW=false`）
  検証: `tools/check-licenses.sh --self-test` rc=0 / `bash -c 'tools/check-licenses.sh --self-test 2>&1 | grep -c "== Scenario: 許諾の名前を 2 つ持つ部品は、\(片方でも許可の外なら許諾の検査が落ちる\|両方が許可されていれば通る\)" | grep -qx 2'` rc=0
- [ ] 4.3 今の木で Android の節を走らせ、`今の携帯の依存は、許諾の検査を通る` と `今の携帯の依存で、オープンソースでないとして通すのは名指しの 4 つだけ` を確かめる（印は `tools/check-licenses.sh` の Android の節の中の `echo "== Scenario: …"`）。
  `POM が許諾を持たない部品は、親の POM の許諾で判定する` は、`tools/check-licenses.sh --self-test` の中で、使い捨ての maven の置き場（許諾の無い POM とその親の POM と空の jar）を作り、`-Pashiato.extraRepo` でその部品を足した使い捨ての写しの木で 4.1 の task を走らせて確かめる（出所が `parent` の行ができ、Android の節が Apache-2.0 として通す）。gradle が使えない環境では、この Scenario は「対象が無い」で落ちる（飛ばさない）。
  `POM を取り寄せられない部品があれば、許諾の検査が落ちる` も同じ使い捨ての置き場に、`.module` と jar はあるが `.pom` の無い部品を足し（`releaseRuntimeClasspath` の解決は通る）、`--offline` で 4.1 の task と Android の節を走らせて、rc≠0・その座標・POM を取れない旨の文が出て、`Could not find` の依存の解決の失敗ではないことを見る（`@pom` の失敗を黙って飛ばす実装はここで落ちる）
  検証: `bash -c 'scripts/quiet-run lic-android -- tools/check-licenses.sh android && test "$(tools/check-licenses.sh android 2>&1 | grep -c "^  通した PROPRIETARY: com.google.android.gms:")" -eq 4'` rc=0 / `bash -c 'scripts/quiet-run lic-self -- tools/check-licenses.sh --self-test && tools/check-licenses.sh --self-test 2>&1 | grep -q "== Scenario: POM が許諾を持たない部品は、親の POM の許諾で判定する" && tools/check-licenses.sh --self-test 2>&1 | grep -q "== Scenario: POM を取り寄せられない部品があれば、許諾の検査が落ちる"'` rc=0

## Task 5: 依存の一覧とソースの場所を組む台本（`tools/release/notices.sh`）

- [ ] 5.1 `tools/release/notices.sh <server|collector-windows> <出力> [--target <triple>]`（D8・Q13）。
  - `cargo tree -p <crate> -e normal --target <triple>` の部品を集め、各部品の置き場の `LICENSE*`・`COPYING*`・`NOTICE*` を写す
  - 部品の許諾のファイルが無ければ、`tools/licenses/texts/<SPDX>.txt` の全文と `authors` を書く。どちらも無ければ落とす
  - Rust の標準ライブラリの項 `rust-std`（`MIT OR Apache-2.0`。全文と著作権の表示は sysroot の `share/doc/rust/`）を足す。sysroot に無ければ落とす
  - `server` には、`web/package-lock.json` の `dev: true` でない部品（`web/node_modules` の `LICENSE*`）を足す
  - 項は名前の順に `== <名前> <版>` → `License: <式>` → 全文の形で書く
  `tools/licenses/texts/` には、許可の一覧の SPDX のうち、今の依存で全文が見つからない部品が使うものだけを置く。
  入力を差し替える口（`ASHIATO_NOTICES_RUST_LIST`・`ASHIATO_NOTICES_NODE_ROOT`）を持たせ、`--self-test` に次の 2 つを足す。
  - `許諾の全文も著作権の表示も見つからない部品があれば、組み立てが落ちる`（許諾のファイルも `authors` も無く、`tools/licenses/texts/` にも無い式の偽の部品。出力のファイルができないことも見る）
  - `サーバの依存の一覧は、画面に入る Node の部品を含み、開発だけの部品を含まない`（今の木の `server` の出力に `== react ` があり `== vitest ` が無い）
  検証: `bash -c 'o="$TMPDIR/n.txt"; tools/release/notices.sh collector-windows "$o" --target x86_64-pc-windows-gnu && grep -q "^== rust-std " "$o" && grep -q "^License: MIT OR Apache-2.0" "$o" && test "$(grep -c "^== " "$o")" -ge 20'` rc=0
- [ ] 5.2 `tools/release/notices.sh apk <出力> <aar の置き場の一覧>`（D8・Q13）: Task 4.1 の task の出力の部品の項を書く（検査と同じ部品の集合）。全文は、jar / aar の `META-INF/LICENSE*`・`NOTICE*` か `tools/licenses/texts/`。
  名指しの gms の 4 つは、aar の `third_party_licenses.txt` を `third_party_licenses.json` の `start`・`length` で切り出し、`(gms の中の第三者)` の見出しで足す
  検証: `bash -c 'o="$TMPDIR/a.txt"; scripts/quiet-run notices-apk -- tools/release/notices.sh apk "$o" && grep -q "^== com.google.android.gms:play-services-location " "$o" && grep -q "(gms の中の第三者)" "$o"'` rc=0
- [ ] 5.3 `tools/release/notices.sh source <出力> <版番号> <commit>`（D9・D16・Q14・Q15・Q16）: `SOURCE.txt` の 3 行を書く。
  - `ASHIATO_SOURCE_REF` が `v<版番号>` なら `Ref: v<版番号>` を書く
  - `ASHIATO_SOURCE_REF` が空でなく `v<版番号>` と違えば落とす
  - `ASHIATO_SOURCE_REF` が空なら `Commit: <40 桁>` を書く。`<commit>` は呼び出し元（6.1）が渡す。40 桁の 16 進でなければ落とし、`SOURCE.txt` を作らない
  `Repository:` は `ASHIATO_SOURCE_REPO`（既定 `https://github.com/rai-256/ashiato`）。
  `--self-test` は `正式版の配布物のソースの場所は、その版の印`・`開発版と main への push の配布物のソースの場所は、組み立てた commit`（`nightly` の文字が無い）・`PR の組み立ての配布物のソースの場所は、PR の head の commit`・`組み立ての commit が 40 桁の 16 進でなければ、配布物を作らずに落ちる`・`印が版番号と違う公開の組み立ては、配布物を作らずに落ちる`（rc≠0 で `SOURCE.txt` ができない）を走らせる。
  commit の選び方は、組み立ての印（`crates/build-stamp`・`build.gradle.kts` の `buildCommit`）と同じ順 —— `ASHIATO_BUILD_COMMIT`（空でなければ）、無ければ `git rev-parse HEAD` —— で、`notices.sh` の副コマンド `commit` に置く（40 桁の 16 進を 1 行出す。それ以外（`unknown` を含む）なら落とす）。6.1 の `build.sh` はそれを呼ぶ。
  開発版・main への push と PR の 2 つの Scenario は、自己試験が使い捨ての git の木で当てる: main の commit を checkout した木と、main に無い枝の commit を detached で checkout した木（PR の head。`actions/checkout` と同じ形）のそれぞれで、`commit` の出力が `git rev-parse HEAD` と同じで、`SOURCE.txt` がその commit を持つことを見る。40 桁でない Scenario は `ASHIATO_BUILD_COMMIT=main` で当てる（5.3 だけで検証でき、6.1 を待たない）
  検証: `tools/release/notices.sh --self-test` rc=0 / `bash -c 'tools/release/notices.sh --self-test 2>&1 | grep -c "== Scenario: \(開発版と main への push の配布物\|PR の組み立ての配布物のソースの場所は\|組み立ての commit が 40 桁\)" | grep -qx 3'` rc=0

## Task 6: 配布物に 3 点を入れる（`build.sh`・gradle・`release.yml`）

- [ ] 6.1 `tools/release/build.sh` の `build_server`・`build_collector` で、実行ファイルと同じ置き場に `LICENSE`（木の写し）・`THIRD-PARTY-NOTICES.txt`（5.1）・`SOURCE.txt`（5.3。commit は `tools/release/notices.sh commit` の出力。D9）を作り、tar / zip の根に入れる（`archive_windows` は複数のファイルを受ける形にする）。
  `build_apk` は 3 点を一時の置き場に組み（5.2・5.3）、`-Pashiato.licenseAssets=<置き場>` で gradle に渡す。`build.gradle.kts` は、その property があるときだけ `release` の assets の source set に `licenses/` として足す（D13）。
  `release.yml` の `package-linux`・`package-windows` の組み立ての手順に `ASHIATO_SOURCE_REF: ${{ github.ref_name }}` を渡す。`nightly.yml` と `ci.yml` は渡さない。
  `ci.yml` の `package-linux`・`package-windows` の `actions/checkout` に `ref: ${{ github.event.pull_request.head.sha || github.sha }}` を足す（`with:` は block 形式に書き直す —— flow 形式の `{ … }` の中では値を引用符で囲むことになり、字面の判定が揺れる。PR では head の commit、main への push では push の commit を checkout する。`persist-credentials: false` は残す。D9・Q16）。
  `ci.yml` の `package-artifacts` は、作業の checkout を今のまま（`ref` を渡さない。合わせた木）にし、その後に 2 つ目の `actions/checkout`（block 形式で `ref: ${{ github.event.pull_request.head.sha || github.sha }}`・`path: built`・`persist-credentials: false`）を足す。検査の呼び出しを `tools/release/check-artifacts.sh package/all --allow-debug-key --expect-version "$(tools/release/version.sh source built/Cargo.toml)" --expect-license built/LICENSE` にする（D18・Q17。`--expect-license` は 7.1）。ほかの job の checkout と `release.yml`・`nightly.yml` は変えない。
  `tools/release/check-guards.sh` の `workflows` に、`package-linux`・`package-windows` の checkout がその `ref` を持つこと、`package-artifacts` の作業の checkout（`path:` を持たない checkout）が `ref` を持たず、`path: built` の checkout がその `ref` を持ち、`check-artifacts.sh` の呼び出しが `version.sh source built/Cargo.toml` と `--expect-license built/LICENSE` を持つことの判定を足し、`--self-test` に `PR の配布物は、PR の head の commit から組まれ、同じ commit の期待値で検査される` を足す（今の `ci.yml` で通り、次の 5 つの偽の `ci.yml` がどれも落ちる: 組む 2 つのどちらかから `ref` を抜いた / `package-artifacts` の作業の checkout に `ref` を足した / `path: built` の checkout を抜いた / `--expect-license built/LICENSE` を抜いた / `version.sh source built/Cargo.toml` を `version.sh source` に戻した。既存の偽の workflow の作り方（`CHECK_GUARDS_ROOT`）に合わせる）
  検証: `bash -c 'o="$TMPDIR/b"; scripts/quiet-run build-linux -- tools/release/build.sh server-linux "$o" && t=$(ls "$o"/ashiato-server-*-linux-x86_64.tar.gz) && tar -tzf "$t" | sort | tr "\n" " " | grep -q "LICENSE SOURCE.txt THIRD-PARTY-NOTICES.txt ashiato-server"'` rc=0 / `bash -c 'o="$TMPDIR/b"; scripts/quiet-run build-apk -- tools/release/build.sh apk "$o" && unzip -l "$o"/ashiato-collector-*.apk | grep -c "assets/licenses/\(LICENSE\|THIRD-PARTY-NOTICES.txt\|SOURCE.txt\)$" | grep -qx 3'` rc=0 / `bash -c 'grep -c "ASHIATO_SOURCE_REF: \${{ github.ref_name }}" .github/workflows/release.yml | grep -qx 4 && ! grep -q ASHIATO_SOURCE_REF .github/workflows/nightly.yml .github/workflows/ci.yml'` rc=0 / `bash -c '! grep -rq ASHIATO_SOURCE_COMMIT .github/workflows tools/release && ! grep -q pull_request.head.sha .github/workflows/release.yml .github/workflows/nightly.yml'` rc=0 / `tools/release/check-guards.sh workflows` rc=0 / `bash -c 'tools/release/check-guards.sh --self-test 2>&1 | grep -c "== Scenario: PR の配布物は、PR の head の commit から組まれ、同じ commit の期待値で検査される" | grep -qx 1'` rc=0

## Task 7: 配布物の検査に 3 点を足す（`check-artifacts.sh`）

- [ ] 7.1 `tools/release/check-artifacts.sh` の配布物ごとの検査に、次を足す（D8・D9・Q16）。
  - 3 点があること（tar / zip の根、APK の `assets/licenses/`）
  - `LICENSE` が期待する `LICENSE` と同じこと。`--expect-license <file>` を足し、渡されればそのファイルと、渡されなければ今どおり台本の木の `LICENSE`（`$root/LICENSE`）と比べる。`--expect-license` のファイルが無ければ rc=2（使い方の誤り）。`draft.sh`・`release.yml`・自己試験の呼び出しは渡さない（D18・Q17）
  - サーバと PC の収集アプリの一覧に `== rust-std ` の項があること
  - APK の一覧に `(gms の中の第三者)` があること
  - `SOURCE.txt` が、`--allow-debug-key` が無く `--channel release|beta` なら `Ref: v<期待する版番号>`、`--channel nightly` と `--allow-debug-key`（PR・main への push と下書き）なら組み立ての印の commit と同じ 40 桁の 16 進の `Commit:` を持つこと（比べはすべての組み立てで残す。D9・Q16）。既定のチャンネル（`release`）と `draft.sh`・`ci.yml` の呼び出しは変えない
  欠けたら、欠けたファイルと配布物の名前を示して rc≠0。
  `--self-test` に、手で組んだ小さな配布物（偽の実行ファイルの tar / zip と、3 点を `assets/licenses/` に入れた zip。APK の版と署名の検査は通さず、3 点の検査の関数だけを当てる）で次の 8 つを足す（下の重い自己試験の 1 本と合わせて、検証の 9 本になる）。
  - `許諾のファイルが欠けた配布物は、公開しない`（tar から `THIRD-PARTY-NOTICES.txt` を抜く）
  - `配布物の LICENSE が木の LICENSE と違えば、公開しない`（tar の `LICENSE` を AGPL の全文だけにする）
  - `PR の組み立てと下書きの配布物の検査は、head の commit のソースの場所で通る`（`--allow-debug-key` で、組み立ての印と同じ 40 桁の `Commit:` の配布物が rc=0）
  - `debug の鍵を許す検査でも、組み立ての印の commit でないソースの場所の配布物は落ちる`（`--allow-debug-key` で `Ref: v0.4.0`・`Commit: main`・7 桁の `Commit:`・組み立ての印と違う 40 桁の `Commit:` の 4 つの配布物がどれも rc≠0）
  - `ソースの場所がチャンネルの決まりと合わない配布物は、公開しない`（`--channel beta` で `Commit:`）
  - `Rust の実行ファイルの依存の一覧は、標準ライブラリを含む`（`rust-std` の項を抜いたら rc≠0、あれば通る）
  - `携帯の APK の依存の一覧は、Google のライブラリの中の第三者を含む`（第三者の見出しを抜いたら rc≠0）
  - `配布物の LICENSE は、渡した期待する LICENSE と比べる`（木の `LICENSE` の末尾に 1 行足したファイルを `LICENSE` に持つ配布物が、そのファイルを `--expect-license` に渡すと rc=0、渡さないと rc≠0 で `LICENSE` の違いを示す）
  今の重い自己試験（`package-linux` の job）は Linux で本物の 2 つ（サーバの Linux 版と APK）を組み、Windows の 2 つは偽の zip を置く。偽の zip にも 3 点を入れ、既存の失敗の Scenario がそれぞれ意図した理由（出力の文）で落ちることを保つ。そのうえで `4 つの配布物は、許諾・依存の一覧・ソースの場所を持つ` を足す（本物の 4 つは `package-artifacts` の job の `check-artifacts.sh` が毎回見る）
  検証: `scripts/quiet-run artifacts -- tools/release/check-artifacts.sh --self-test` rc=0 / `bash -c 'tools/release/check-artifacts.sh --self-test 2>&1 | grep -c "== Scenario: \(許諾のファイルが欠けた\|配布物の LICENSE が\|PR の組み立てと下書き\|debug の鍵を許す検査でも\|ソースの場所がチャンネル\|Rust の実行ファイルの依存\|携帯の APK の依存\|4 つの配布物は\|配布物の LICENSE は、渡した\)" | grep -qx 9'` rc=0

## Task 8: 承認を経る公開の確かめ（`check-guards.sh`・`release.yml`）

- [ ] 8.1 `tools/release/check-guards.sh` に `approvals [--repo] [--reviewer <login>]` と `agent-credential [--repo]` を足す（D11（仮））。既存の `rules`・`environments`・`workflows` の判定と Scenario は変えない。
  - `approvals`: `gh api repos/<repo>/environments/release` を読み、`required_reviewers` の規則・`reviewers` に `User` の `<login>`・`can_admins_bypass: false` を求める。`prevent_self_review` は見ない。`<login>` の既定は `ASHIATO_RELEASE_REVIEWER`、無ければ `<owner>`
  - `agent-credential`: `gh api repos/<repo>` の `.permissions` の `admin`・`maintain` がどちらも `false` を求める
  偽の `gh` に `environments/release$`（詳細）と `repos/<repo>$`（権限）の応答を足す。今の `environments/release*`（配る対象の規則）の応答とは分ける。
  `--self-test` に次の 6 つを足す。
  - `承認者の無い配布用の Environment では、公開の前の確かめが落ちる`
  - `管理者が承認を飛ばせる設定では、公開の前の確かめが落ちる`
  - `本人が承認者で管理者が飛ばせなければ、公開の前の確かめを通る`（`prevent_self_review: false` で通る）
  - `本人が承認者にいなければ、公開の前の確かめが落ちる`
  - `管理者の資格情報では、エージェントの資格情報の確かめが落ちる`
  - `管理者でない資格情報では、エージェントの資格情報の確かめを通る`
  検証: `tools/release/check-guards.sh --self-test` rc=0
- [ ] 8.2 `release.yml` の `publish` に、`check-guards.sh rules` の後・`sign-apk.sh` の前で `tools/release/check-guards.sh approvals` を足す（D11（仮））。
  `check-guards.sh workflows` に、`publish` の job の中で `check-guards.sh approvals` の行が `sign-apk.sh` の行より前にあることの確かめを足す。
  `--self-test` に `公開の job は、署名の前に承認者を確かめる` を足す（2 行を入れ替えた偽の `release.yml` と、`approvals` を消した偽の `release.yml` で、どちらも rc≠0）
  検証: `tools/release/check-guards.sh --self-test` rc=0 / `tools/release/check-guards.sh workflows` rc=0 / `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/release.yml` rc=0

## Task 9: 確認バッチの準備はエミュレータにだけ入れる

- [ ] 9.1 `tools/verify-prep-device.sh` に関数 `install_on_emulators <apk>` を置く（D14）。`adb devices` の状態が `device` で、名前が `emulator-` で始まる端末にだけ `adb -s <名前> install -r <apk>` を呼び、入れた名前を 1 行ずつ出す。
  1 台も無ければ「エミュレータが無いので入れていない（手で入れる: `adb -s <エミュレータ> install -r <apk>`）」の 1 行を出す（D14）。`tools/verify-prep.sh` の 58〜61 行の端末の選び方を、これの `source` と呼び出しに替え、出力をそのまま `android_note` にする。`verify-prep.sh` には `adb` も `install` の文字も残さない（`install_on_emulators` の呼び出しを除く）。
  `tools/verify-prep-device.sh --self-test` は、偽の `adb`（呼ばれた引数を書き残す）で次の 3 つを走らせる。
  - `実機とエミュレータが繋がっていれば、エミュレータにだけ入れる`
  - `実機だけが繋がっていれば、何も入れない`
  - `準備の台本は、端末を選ぶ手順を通して APK を入れる`（`tools/verify-prep.sh` に、`install_on_emulators` の呼び出しの行を除いて `adb` と `install` の文字が無く、呼び出しが 1 つある）
  `tools/check-db-version-stop.sh` が `verify-prep.sh` から切り出している行（`project=` と `run.sh` の部分）を壊さない
  検証: `tools/verify-prep-device.sh --self-test` rc=0 / `bash -c 'bash -n tools/verify-prep.sh'` rc=0 / `bash -c 'grep -v install_on_emulators tools/verify-prep.sh | grep -Eq "adb|install" && exit 1 || exit 0'` rc=0

## Task 10: CI の置き場・手順書・文書

- [ ] 10.1 `.github/workflows/ci.yml`（D15）。
  - `web` の job: `../tools/check-licenses.sh rust node`
  - `package-linux` の job: `tools/check-licenses.sh android` と `tools/check-licenses.sh --self-test`・`tools/release/notices.sh --self-test`
  - `chain` の job（全 PR）: `tools/check-spdx.sh --self-test` と `tools/check-spdx.sh`
  - `verify-prep-device.sh --self-test` は、軽い自己試験の並ぶ job（`check-private.sh --self-test` と同じ job）に置く
  検証: `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/ci.yml` rc=0 / `bash -c 'for k in "check-licenses.sh rust node" "check-licenses.sh android" "check-spdx.sh --self-test" "verify-prep-device.sh --self-test" "notices.sh --self-test"; do grep -q "$k" .github/workflows/ci.yml || exit 1; done'` rc=0 / `tools/release/check-guards.sh workflows` rc=0
- [ ] 10.2 `docs/release.md` を書き直す（D3・D10・D12・D18）。
  - 冒頭: 本物の作業は ST53 が main に入った後
  - §3「エージェントの資格情報」: fine-grained token に持たせる権限と持たせない権限（D3）、ハーネスの token の差し替え、`tools/release/check-guards.sh agent-credential --repo rai-256/ashiato` を差し替えた資格情報で走らせて rc=0
  - §4「ルールセットと承認者」: ST52 のルールセット、`release` に本人を必須の承認者として付ける、管理者の飛ばしを外す、自分の承認を禁じない、`check-guards.sh approvals --repo rai-256/ashiato` rc=0。workflow の中の確かめは守りの本体でない（印の commit の workflow で走るため）
  - §5「鍵と Secrets」: ST52 の §3 の中身
  - 承認するときに見ること: 印の commit が main の祖先で、PR を通って入ったこと
  - 最初の印の最後: `gh release download` と `check-artifacts.sh --channel` の 2 行。印 `v<版>` を checkout した手元で走らせる（D12）
  - merge の後の PR の成果物の掃除（D10）
  - 下書きの手順（`draft.sh`）: その PR の head の commit を checkout した手元で走らせる（配布物は head から組まれ、`draft.sh` は手元の木の版番号と `LICENSE` で比べる。D18）
  - ST53 の確認バッチの手順（末尾の「人間の確認待ち」）。準備の間は本人の携帯を繋がない
  検証: `bash -c 'for h in "エージェントの資格情報" "agent-credential" "approvals" "必須の承認者" "gh release download" "成果物" "携帯を繋がない" "head の commit を checkout"; do grep -q "$h" docs/release.md || exit 1; done'` rc=0 / `bash -c 'a=$(grep -n "エージェントの資格情報" docs/release.md | head -1 | cut -d: -f1); b=$(grep -n "ASHIATO_SIGNING_KEYSTORE_B64" docs/release.md | head -1 | cut -d: -f1); test "$a" -lt "$b"'` rc=0 / `bash -c 'a=$(grep -n "check-guards.sh approvals" docs/release.md | head -1 | cut -d: -f1); r=$(grep -n "必須の承認者" docs/release.md | head -1 | cut -d: -f1); b=$(grep -n "ASHIATO_SIGNING_KEYSTORE_B64" docs/release.md | head -1 | cut -d: -f1); test -n "$a" && test -n "$r" && test "$a" -lt "$b" && test "$r" -lt "$b"'` rc=0（資格情報 → 承認者 → 鍵の順。review/spec.md R12）
- [ ] 10.3 `docs/testing.md`・`CONTRIBUTING.md` の手元の検査の行を `../tools/check-licenses.sh rust node` に、`docs/production-prep.md` の「依存ライセンスの検査」の行に ★ で携帯の節と表示の検査（ST53）を足す。`CLAUDE.md` の「構成」の `tools` の行に `licenses/`（許諾の対応表と全文）を足す
  検証: `bash -c 'grep -q "check-licenses.sh rust node" docs/testing.md && grep -q "check-licenses.sh rust node" CONTRIBUTING.md && grep -q "ST53" docs/production-prep.md && grep -q "licenses/" CLAUDE.md'` rc=0 / `python3 scripts/check_scenarios.py . st53-license-bundle` rc=0

## 人間の確認待ち（確認バッチ。PR の CI が緑になった後）

下流の実装の中には PR の CI の実行が無いので、配布元での確かめはここで行う。Scenario ではない —— どれも前提（premises）の確かさを上げる確かめで、結果は確認バッチが `premises.md` に書く。**準備の間は本人の携帯を繋がない**（Q8。Task 9 が入るまで）。

- [ ] 11.1 **下書きのリリースで許諾の 3 点を見る**（Q4・D12）: この Story の PR の `package` の run で `tools/release/draft.sh <run-id> 0.1.0-beta.1` を、その PR の head を checkout した手元（`git rev-parse HEAD` が run の `head_sha` と同じ）で走らせる（D18）。
  作る前と後で `git ls-remote --tags origin 'v*' | wc -l` が同じこと、4 つの配布物を落として 3 点があり、`SOURCE.txt` が `Commit:` を持つことを見る。APK は `unzip -l` で `assets/licenses/` を見る。見た後に下書きを消す。
  `SOURCE.txt` の `Commit:` が、その run の PR の head の commit（`gh api repos/rai-256/ashiato/actions/runs/<run-id> --jq .head_sha`）と同じで、配布物の組み立ての印の commit とも同じ（`.dirty` を持たない）ことを見る（Q16・P18）。その run の `package-linux`・`package-windows` と `package-artifacts` の `built/` の checkout が head を checkout して通ったことで P17 を見る（同じ repo の枝の PR の範囲で P17、P18 の確かさを 4 にする。fork の PR の側の P17 はこの run では見られず、確かさ 2 のまま）。食い違えば D9 を上流へ返す
- [ ] 11.5 **PR の head を後から取れるか**（P14。FR-127 ★ 2026-11-03。`package` も ST53 のコードも要らないので、確認バッチの中で ST53 が main に入る前に行う）:
  使い捨ての枝に 1 commit を足して PR を開き、`git rev-parse HEAD` で head を控える → 枝を rebase して force-push し直し、新しい head も控える → PR を merge せずに閉じる → 別の空の clone で、控えた 2 つ（push し直す前の head と最後の head）を `git fetch origin <commit>` で取れるかを見る。最後に枝を消す。
  取れれば P14 の確かさを 4 にする。取れなければ D9 の反転条件で上流へ返す（A。ST53 を main に入れない）
- [ ] 11.2 **承認者が job を止めることと、権限を絞った token が承認できないこと**（P8・P11・P12 後半・D3・D11（仮））: 使い捨ての Environment `st53-probe` に本人を必須の承認者として付け、使い捨ての Secret を置く。
  main に無い枝の commit に、その Environment を使う workflow を足して 1 回だけ走らせ、承認するまで job が始まらず Secret が読まれないことを見る。
  さらに、`docs/release.md` §3 の権限で作った token で、その保留中の実行の承認（`gh api -X POST repos/rai-256/ashiato/actions/runs/<id>/pending_deployments`）を試み、断られることを見る。最後に Environment と枝と token を消す。
  承認できた・承認の前に job が始まった場合は、鍵を置かず、Q3 を人間に聞き直す。P8・P11・P12 の確かさを 4 にする
- [ ] 11.3 **公開の job から承認者の設定を読めるか**（D11（仮）の反転条件）: 11.2 の workflow の中で `tools/release/check-guards.sh approvals --repo rai-256/ashiato --reviewer <本人>`（`GITHUB_TOKEN`）を走らせ、`st53-probe` を `release` の代わりに読ませて rc を見る。
  読めなければ、D11（仮）の反転条件で上流へ返す。結果を P15 として書く
- [ ] 11.4 **GitHub の許諾の表示**（D1 の反転条件・Risks）: この Story の枝の `LICENSE` を GitHub の画面で開き、許諾の判定の表示（AGPL-3.0 か Other か）を見て、結果を P16 として書く。Other でも止めない（README が名指しする）。気になったら違和感で返す
