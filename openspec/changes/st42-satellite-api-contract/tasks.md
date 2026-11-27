# ST42 実装タスク — 衛星向けの API の契約を、版を付けて読める形で公開する

読む順: `deep.md`（Q1〜Q8。第 1 回は推奨で先に書いてある）→ `review/deep.md` の「聞かない既定（C）」→ このファイル → `specs/satellite-platform/spec.md` → `specs/data-sensitivity/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提**: requires の ST29（衛星の資格情報）は archive 済みで main にある。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。
件数つきの検証は、試験を走らせた後に `-- --list` の本数が下限以上かを同じコマンドの中で見る（0 本でも rc=0 になるため）。一時ファイルは `$TMPDIR` に書く（`/tmp` の固定名を使わない）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust はコメント（`// Scenario: …`）、bash は `echo "== Scenario: …"`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせる。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change で ST42 が印を置く Scenario は 59 本**（satellite-platform の ADDED。Rust 21 本・台本の本番 4 本・台本の `--self-test` 34 本）。全部このファイルのどこかの Task が名指しする。
  MODIFIED の「衛星の資格情報では、資格情報の要らない口のほかに 2xx を返す口が無い」は本文が変わったので、印を持つ `satellite_routes_tests::every_route_refuses_satellite` を直す（Task 4）。
  MODIFIED で本文をそのまま写した Scenario（satellite-platform の 4 本・data-sensitivity の 2 つの Requirement の全部）の印は ST29・ST44 が持っている。**消さない・名前を変えない**
- **本人の決定（下流は変えない）**: 第 1 回の 7 問は推奨で書いた（答えが違えば `spec --answers` が写し直す）。Q1 道に版（`/satellite/v1/…`）/ Q2 自前の台本で決めた規則 / Q3 版ごとの写し（行き先にあるもの）と比べる /
  Q4 確かめる口は受ける版の一覧を返す / Q6 6 か月は次の版を最初に配った印から / Q7 足したら小さい番号を上げる / Q8 無い版は版の一覧つきで断り、外す前は見出しで予告し、知らせの画面ができるまで版は外せない。
  聞かない既定: D1・D2・D7・D14。仮: D3・D5・D6・D8・D9・D10・D11・D12・D13（反転条件は design）
- **名前は統一する**: 衛星向けの道 `/satellite/v1/me`・版の受け口 `/satellite`・`/satellite/`・`/satellite/{*rest}`、型 `SatelliteApiDoc`（`crates/server/src/satellite_api.rs`）、bin `satellite_api`（`crates/server/src/bin/satellite_api.rs`。`contract <N>`・`page <N>`・`index`）、
  断りの種別 `unauthorized`・`not_satellite`・`unsupported_version`・`not_found`・`method_not_allowed`・`internal`、置き場 `docs/satellite-api/`（`versions.json`・`LICENSE`・`README.md`・`v1/openapi.json`・`v1/README.md`）、
  台本 `tools/check-satellite-contract.sh`（`--self-test`・`--base <commit>`・`--generated <フォルダ>`・`SATELLITE_CONTRACT_TODAY`）、`tools/release/changes.sh satellite`、試験の名前の接頭辞 `st42_`（Rust。`crates/server/src/st42_tests.rs`）
- **`tools/release/changes.sh code` の意味を変えない**（ST61 が番号を付ける判定に使う。D10）。
- **衛星向けの契約の版は製品の版から引かない**（D7）。`tools/check-openapi.sh` と `src/bin/openapi.rs` と `docs/openapi.json` は変えない（ST61 が触っている）。内部の口の契約に衛星向けの口を載せない
- **衛星向けの契約・ページ・LICENSE に `SPDX-License-Identifier:` の行を書かない**（D2。`tools/check-spdx.sh` の例外の表を変えない）
- **衛星の資格情報の値・衛星の識別子の文字列・届いた道の文字列を、読み出しの記録・ログに出さない**（試験の値も偽の値だけ）
- **壊してはいけないもの**（Story）: ST01 の API 契約（`docs/openapi.json`）とそれを見る検査（`tools/check-openapi.sh`）・ST29 の衛星の資格情報（発行・取り消し・外す・内部の口で 403）。既存の試験は変わらず通る
- **新しい Rust の crate・npm の依存・外の道具（oasdiff など）を足さない**（D9）。移行を足さない（D13）
- **長いコマンドは `scripts/quiet-run`**（cargo test・bin を組み直す台本の本番）。台本の `--self-test` は `--generated` で材料を渡し、場合ごとに Rust を組み直さない（D10）。縦串・偽データのシナリオを足すなら Story ごとの新しいファイル（`tools/smoke.d/` など）に置き、共有の台本の本体に書き足さない

## Task 1: 2 つ目の契約と書き出す bin、版の一覧（D1・D2・D7・D8）

- [ ] 1.1 `docs/satellite-api/versions.json` を置く（`{"versions": [{"major": 1, "version": "1.0", "sunset": null, "notice": null}]}`）。`crates/server/src/satellite_api.rs` を足し、版の一覧を `include_str!` で読む型と、試験から別の一覧を渡せる口を持たせる。
  `SatelliteApiDoc`（`#[derive(OpenApi)]`）に `info(license(name = "Apache-2.0", identifier = "Apache-2.0"))`・securitySchemes（HTTP bearer。説明に `Authorization: Bearer ashsat_<識別子>_<乱数>`）・断りの型（`error` は文字列。既知の値を説明に並べる）を持たせる。
  口は Task 2 が足す（ここでは型と文書の枠）。`info.version` は版の一覧から入れる関数で作る（`ASHIATO_BUILD_VERSION` を読まない）。
  `crates/server/src/bin/satellite_api.rs` を足す（`contract <N>` は契約の JSON、`page <N>` と `index` は Task 5 が中身を作る）。premises P5 を確かさ 4 に上げる（premises.md の P5 の行の結果・出所・確かさを書き直す）
  試験（`crates/server/src/st42_tests.rs`。DB を使わない。名前は `st42_contract_` で始める）: `衛星向けの契約は内部の口の契約と別に作られ、互いの口を持たない`（Task 2 の後に `/satellite/v1/me` を見る形で通る。ここでは内部の契約に `/satellite` が無いことと、衛星向けの契約に `/events` が無いことまで）・
  `衛星向けの契約は Apache-2.0 を宣言し、内部の口の契約は今の式のまま`・`衛星向けの契約は資格情報の渡し方を持ち、すべての口に付ける`（Task 2 の後に 401・403 を見る形で通る）
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-server st42_contract_ && test "$(cargo test -p ashiato-server st42_contract_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0 /
  `bash -c 'cargo run -q -p ashiato-server --bin satellite_api -- contract 1 | jq -e ".info.license.identifier == \"Apache-2.0\" and .info.version == \"1.0\"" >/dev/null'` rc=0 /
  `scripts/quiet-run openapi -- tools/check-openapi.sh` rc=0（内部の口の契約は変わらない）

## Task 2: 確かめる口と、衛星向けの道の断り・読み出しの記録（D5・D6・D13）

- [ ] 2.1 `GET /satellite/v1/me` を `lib.rs` の router に字面の `.route(` で足す（`.nest(` を使わない）。応答は D6 の形（`{"satellite": {"id", "name"}, "api_versions": [{"version", "sunset"}]}`）で、ほかの欄を持たない。
  `access_log::middleware` を直す: `/satellite` で始まる経路では、衛星の資格情報を通し、API の合言葉・画面のログインは 403 `{"error": "not_satellite"}`、資格情報無し・認められないものは 401 `{"error": "unauthorized"}`（どちらも `application/json`）。
  この道だけ、行はハンドラの後に返す状態で書き、書けなければ応答を捨てて 500 `{"error": "internal"}`（D13）。ほかの経路の衛星の 403（本文 `forbidden`）・内部の口の 401 / 403 の本文・ハンドラの前に書く動きは変えない。
  `SatelliteApiDoc` にこの口を載せる（401・403 の応答と断りの型を付ける）。`docs/satellite-api/v1/openapi.json` を bin で書き出してコミットする
  試験（`crates/server/src/st42_tests.rs`。名前は `st42_me_` で始める）: `確かめる口は、衛星の識別子・名前と受ける版の一覧を返す`・`取り消した資格情報か外した衛星の資格情報で確かめる口を呼ぶと 401 になる`・`資格情報の無い求めは、衛星向けの口で種別つきの 401 になる`・
  `API の合言葉と画面のログインでは、衛星向けの口は通らない`・`確かめる口を呼ぶと、衛星と 1 本の番号つきで ok の行が残る`（行の全列に識別子の文字列も資格情報の値も無いことまで見る）・`衛星でない資格情報で衛星向けの口を呼ぶと、forbidden・403 の行が残る`・
  `読み出しの記録に書けないときは、確かめる口は 500 で衛星の情報を返さない`（ST29・ST44 の試験が読み出しの記録を書けなくする作り方に合わせる）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st42_me_ && test "$(cargo test -p ashiato-server st42_me_ -- --list 2>/dev/null | grep -c ": test$")" -ge 7'` rc=0 /
  `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-server st42_contract_ && test "$(cargo test -p ashiato-server st42_contract_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0

## Task 3: 版の受け口・405・Sunset の見出し（D11・D12 のサーバの側）

- [ ] 3.1 受け口 `.route("/satellite", any(...))`・`.route("/satellite/", any(...))`・`.route("/satellite/{*rest}", any(...))` を足す。`{*rest}` の最初の区切りが `v<版の一覧にある大きい版>` なら 404 `{"error": "not_found"}`、そうでなければ（区切りが無いものを含む）404 `{"error": "unsupported_version", "api_versions": [...]}`。
  `/satellite/v1/me` のメソッド違いは、その経路の fallback で 405 `{"error": "method_not_allowed"}`。資格情報の判定は middleware が先に行う（Task 2）。行の口の名前は経路の形（`MatchedPath`）で、結果 `forbidden`・状態 404 / 405（D13）。
  版の一覧の項目が `sunset` を持つとき、その大きい版の道（`/satellite/v<N>/…`）への応答に、middleware の外側の層で見出し `Sunset`（その日の 00:00 UTC の IMF-fixdate）を付ける（401・403 にも付く）。持たなければ付けない。
  試験は Task 1 の「別の一覧を渡せる口」で `sunset: 2027-07-10` の一覧を渡して組む。断りの種別と `Sunset` の説明を `SatelliteApiDoc` に載せ（受け口は契約の口にしない）、`docs/satellite-api/v1/openapi.json` を書き出し直す
  試験（`crates/server/src/st42_tests.rs`。名前は `st42_version_` で始める）: `版の一覧に無い版の道は、版の一覧つきの 404 になる`・`版の後ろに道の無い求めも、版の一覧つきの 404 になる`・`受ける版の中の、契約に無い道は not_found になる`・
  `契約にある道へ違うメソッドで送ると、種別つきの 405 になる`・`資格情報の無い求めは、版の一覧に無い版の道でも 401 になる`・`外す日を持つ版の応答には Sunset の見出しが付く`・`外す日を持つ版の道では、資格情報の断りにも Sunset の見出しが付く`・
  `外す日を持たない版の応答には Sunset の見出しが付かない`・`無い版の道への求めは、届いた道の文字列を残さずに 1 行残る`
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st42_version_ && test "$(cargo test -p ashiato-server st42_version_ -- --list 2>/dev/null | grep -c ": test$")" -ge 9'` rc=0

## Task 4: 経路の試験と許可の一覧の試験を、2 つの契約の和で見る（D11）

- [ ] 4.1 `satellite_routes_tests::every_route_refuses_satellite` を直す: 403 で断ることを見る口は内部の契約の口だけにし、経路の集合の一致は新しい試験に移す。`.merge(` `.nest(` `.nest_service(` の禁止は残す。
  `// Scenario: 衛星の資格情報では、資格情報の要らない口のほかに 2xx を返す口が無い` の印は名前を変えない。
  `collector_scope_tests` の「API の説明に載る口」を回す試験に衛星向けの契約の口を加え、API の合言葉で 403 になることを見る（許可の一覧に足さない）。
  新しい試験（`crates/server/src/st42_tests.rs`。名前は `st42_routes_` で始める）: `サーバの経路は、2 つの契約の口と生存確認と版の受け口を合わせたものにちょうど一致する`（`lib.rs` の `.route(` の path の集合が「内部の契約の path ∪ 衛星向けの契約の path ∪ `/healthz` ∪ 受け口の 3 本」と一致。`/selftest/panic` は今の試験と同じく除く）・
  `版の一覧にある版の口は、サーバの経路にある`（版の一覧の大きい版ごとに `SatelliteApiDoc` のその版の口を衛星の資格情報で呼び、404 でも 405 でもない）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server satellite_routes && test "$(cargo test -p ashiato-server satellite_routes -- --list 2>/dev/null | grep -c ": test$")" -ge 2'` rc=0 /
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server collector_scope && test "$(cargo test -p ashiato-server collector_scope -- --list 2>/dev/null | grep -c ": test$")" -ge 1'` rc=0 /
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st42_routes_ && test "$(cargo test -p ashiato-server st42_routes_ -- --list 2>/dev/null | grep -c ": test$")" -ge 2'` rc=0
- [ ] 4.2 ST29・ST44 の衛星と資格情報・許可の一覧・読み出しの記録の試験が変わらず通る（どの絞り込みも 1 本以上あることを `-- --list` で見る）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; for p in satellite_ access_log collector_scope; do scripts/quiet-run ct -- cargo test -p ashiato-server "$p" || exit 1; test "$(cargo test -p ashiato-server "$p" -- --list 2>/dev/null | grep -c ": test$")" -ge 1 || exit 1; done'` rc=0

## Task 5: ページ・LICENSE・README と CONTRIBUTING、検査の台本の土台（D2・D3・D10 の 1〜3）

- [ ] 5.1 bin の `page <N>` と `index` を作り、D3 の 2 枚（`docs/satellite-api/README.md`・`docs/satellite-api/v1/README.md`）を書き出してコミットする。断りの種別の表に「知らない値は状態の番号で扱う」を書く。
  `docs/satellite-api/LICENSE` に `tools/licenses/texts/Apache-2.0.txt` を写す。`README.md` と `CONTRIBUTING.md` の「ライセンス」の節に、本体（AGPL-3.0-only・追加の許可）と衛星向けの契約（`docs/satellite-api/`・Apache-2.0）を分けて書く（D2）。
  `tools/check-satellite-contract.sh` を足し、(1) 契約とページを bin で作り直して（`--generated` があればそのフォルダを使う）コミットされたものと比べ、ページの相対リンクの先が木にあるかを見る (2) LICENSE と README・CONTRIBUTING の節 (3) 版の一覧と契約の写しの一致（大きい版ごとの写しの有無・`info.version`）を見る。
  `--self-test` は `$TMPDIR` の下の使い捨ての git の木に材料を置いて当てる（手本は `tools/check-contract-version.sh --self-test`）。
  本番（今の木に当てる。`echo "== Scenario: …"`）: `衛星向けの契約のページは、口・資格情報の渡し方・断りの種別・版・許諾を示す`・`契約の置き場の LICENSE は Apache-2.0 の全文で、README と CONTRIBUTING は 2 つのライセンスを分けて書く`・
  `衛星向けの契約の版は、版の一覧の版で、渡した版番号に依らない`（`ASHIATO_BUILD_VERSION=0.4.0-beta.2` で作り直して差が無い）。
  `--self-test`: `コミットされた衛星向けの契約がコードとずれると検査が落ちる`・`コミットされた衛星向けの契約のページがコードとずれると検査が落ちる`・`LICENSE が違うか、README か CONTRIBUTING の節が契約のライセンスを書いていないと検査が落ちる`・`版の一覧と契約の版が食い違うと検査が落ちる`
  あわせて premises P3 を確かめる: `gh api -X POST /markdown -f mode=gfm -F text=@docs/satellite-api/v1/README.md` の出力に `<table>` があり、リンクが `<a href` になる（premises.md の P3 の行の結果・出所を書き直し、確かさを 4 にする。CI には置かない）
  検証: `bash -c 'out="$(tools/check-satellite-contract.sh --self-test)" && test "$(printf "%s\n" "$out" | grep -c "^== Scenario: ")" -ge 4'` rc=0 / `bash -c 'out="$(tools/check-satellite-contract.sh)" && test "$(printf "%s\n" "$out" | grep -c "^== Scenario: ")" -ge 3'` rc=0 / `tools/check-spdx.sh` rc=0（例外の表は変わらない）

## Task 6: 壊す変更の規則と比べる相手（D9・D10 の 4）

- [ ] 6.1 `tools/check-satellite-contract.sh` に、`--base <commit>`（無ければ `git merge-base HEAD origin/main`）の写しと作り直した契約を大きい版ごとに比べる検査を足す。D9 の表の規則を `$ref` を展開してから当て、当たった規則と場所（道・メソッド・欄）を示して落とす。
  足す変更があって小さい版が上がっていなければ落とす。比べる相手の commit を読めなければ落とし、その版の写しが無ければ比べずに通してそう示す。比べるのはコミットされた写しでなく作り直した契約（写しの書き換えでは抜けられない）。
  送る側の規則は、材料の契約に任意の送る欄 `note`・`tag`、選べる値 `mode`（`a` `b`）を持つ試しの口を置いて当てる
  `--self-test`: `同じ版の中で口を消すと検査が落ちる`・`同じ版の中で口の応答の状態の番号を消すと検査が落ちる`・`同じ版の中で応答の欄を消すと検査が落ちる`・`同じ版の中で応答の欄を必須でなくすると検査が落ちる`・
  `同じ版の中で欄の型を変えると検査が落ちる`・`同じ版の中で欄の形式を変えると検査が落ちる`・`同じ版の中で送る側の任意の欄を必須にすると検査が落ちる`・`同じ版の中で送る側に新しい必須の欄か引数を足すと検査が落ちる`・
  `同じ版の中で送る側の選べる値を減らすと検査が落ちる`・`同じ版の中で資格情報の方式を消すと検査が落ちる`・`同じ版の中で口の求める資格情報を変えると検査が落ちる`・`同じ版の中で許諾の欄を変えると検査が落ちる`・
  `口を足したのに小さい版を上げていないと検査が落ち、上げれば通る`・`応答の状態の番号を足したのに小さい版を上げていないと検査が落ちる`・`任意の送る欄を足したのに小さい版を上げていないと検査が落ちる`・`応答の欄を足すのは壊す変更に数えない`・
  `内部の口の変更では衛星向けの契約の検査は落ちない`・`写しを書き換えても、比べる相手は行き先の写しなので通らない`・`比べる相手を読めないと検査は落ち、その版の写しが無ければ比べずに通る`
  検証: `bash -c 'out="$(tools/check-satellite-contract.sh --self-test)" && test "$(printf "%s\n" "$out" | grep -c "^== Scenario: ")" -ge 23'` rc=0 / `bash -c 'scripts/quiet-run sat -- tools/check-satellite-contract.sh --base "$(git merge-base HEAD origin/main)"'` rc=0

## Task 7: 前の版を外す規則と外す日（D12 の検査の側）

- [ ] 7.1 `tools/check-satellite-contract.sh` に、比べる相手の版の一覧にある大きい版が、版の一覧・契約の写し・作り直した契約から消えたら、D12 の (a)（次の版の写しを最初に含んだ印 `v*` の配った日から 6 か月。配った日は印を作った日と commit の日の遅い方。暦の月、同じ日が無い月は最終日）と
  (b)（項目の `notice` が正典の `### Requirement:` の名前と一字一句同じ）を両方満たすときだけ通す検査を足す。外す日（`sunset`）を置く変更は、(a) の 6 か月の終わり以後のときだけ通す。今日は `SATELLITE_CONTRACT_TODAY` で差し替えられる
  `--self-test`（使い捨ての木に偽の版 2 と偽の印を作る。印を作った日は `GIT_COMMITTER_DATE` で決める。本物の印も本物の版 2 も使わない）: `新しい大きい版を足し、前の版を残せば検査は通る`・`次の版の印がまだ無ければ、前の版は外せない`・
  `次の版を最初に配った日から 6 か月たつ前は、前の版は外せない`・`印を commit より後に作ったときは、印を作った日から数える`・`同じ日が無い月は、月の最終日まで外せない`・`6 か月たっても、本人に知らせる画面の名前が無ければ前の版は外せない`・
  `知らせの画面の名前が正典に無ければ前の版は外せない`・`サーバの経路から前の版の口を消しても外す変更に数える`・`6 か月の終わりより前の外す日は置けない`。本番: `今の木では、どの版も外す日を持たず、外す変更も無い`
  検証: `bash -c 'out="$(tools/check-satellite-contract.sh --self-test)" && test "$(printf "%s\n" "$out" | grep -c "^== Scenario: ")" -ge 32'` rc=0 / `bash -c 'out="$(tools/check-satellite-contract.sh)" && test "$(printf "%s\n" "$out" | grep -c "^== Scenario: ")" -ge 4'` rc=0

## Task 8: CI の配線と、置き場だけの PR でも検査が走ること（D10）

- [ ] 8.1 `tools/release/changes.sh` に新しい判定 `satellite`（`^docs/satellite-api/`・`^README\.md$`・`^CONTRIBUTING\.md$`・`^tools/check-satellite-contract\.sh$`・`^tools/release/changes\.sh$` に触れたら真）を足す。`code` と `deps` は変えない。
  `.github/workflows/ci.yml` の `changes` の job に出力 `satellite` を足し、`rust` の job の `if` を `needs.changes.outputs.code == 'true' || needs.changes.outputs.satellite == 'true'` にする（比べる相手が引けず「全部走らせる」ときは `satellite` も真）。
  `rust` の job の `./tools/check-contract-version.sh` の後に `./tools/check-satellite-contract.sh --self-test` と `./tools/check-satellite-contract.sh --base "$SATELLITE_BASE"`（`changes` の job と同じ比べる相手）を足す（印を取り寄せた後。D10 の反転条件）。ほかの行は変えない
  `--self-test`: `衛星向けの契約の置き場か README・CONTRIBUTING だけを変えた PR でもサーバの検査の job が走る`・`衛星向けの契約の置き場だけの変更は、製品に触れた変更に数えない`
  検証: `bash -c 'out="$(tools/check-satellite-contract.sh --self-test)" && test "$(printf "%s\n" "$out" | grep -c "^== Scenario: ")" -ge 34'` rc=0 / `tools/check-vulns.sh --self-test` rc=0（`changes.sh` の既存の自己試験）/ `bash -c 'grep -q "check-satellite-contract.sh --self-test" .github/workflows/ci.yml && grep -q "check-satellite-contract.sh --base" .github/workflows/ci.yml && grep -q "outputs.satellite" .github/workflows/ci.yml'` rc=0

## Task 9: 全体の確かめ

- [ ] 9.1 サーバの試験の全体が通る
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server'` rc=0
  絞った検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st42_ && test "$(cargo test -p ashiato-server st42_ -- --list 2>/dev/null | grep -c ": test$")" -ge 21'` rc=0
- [ ] 9.2 内部の口の契約と、許諾・境界の検査が変わらず通る
  検証: `scripts/quiet-run openapi -- tools/check-openapi.sh` rc=0 / `tools/check-spdx.sh --self-test` rc=0 / `tools/check-spdx.sh` rc=0 / `scripts/quiet-run clippy -- cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0
- [ ] 9.3 spec の Scenario の印がすべて試験にある
  検証: `python3 scripts/check_scenarios.py . st42-satellite-api-contract` rc=0
