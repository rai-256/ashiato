# ST42 design — 衛星向けの API の契約を、版を付けて読める形で公開する

## Context

なぜ要るかは proposal.md の Why。ここでは今の持ち方と制約だけを書く。

- **契約の生成**: `crates/server/src/lib.rs` の `ApiDoc`（`#[derive(OpenApi)]`）が内部の口の契約を持ち、`src/bin/openapi.rs` が標準出力に書き出す。`tools/check-openapi.sh` が `docs/openapi.json` と突き合わせる。
  許諾の欄は utoipa が crate の `license`（`AGPL-3.0-only WITH AdditionRef-ashiato-play-services`）から引く。`info.version` は出所の版（distribution「…出所の版にする」）。securitySchemes は無い（premises P4）
- **utoipa は明示した `info(license(...))` で crate の既定を上書きする**（`utoipa-gen-5.5.0/src/openapi/info.rs:80-81`。review/deep.md 手順 2）。1 つの crate に 2 つ目の `#[derive(OpenApi)]` を置けることは、まだ走らせていない（premises P5、確かさ 2。Task 1 で 4 に上げる）
- **衛星の資格情報**: `Authorization: Bearer ashsat_<識別子>_<乱数>`。`authorize_quiet` が `Caller::Satellite { satellite, credential }` を返し、`access_log::middleware` がどの口でも `Verdict::Forbidden`（403・本文 `forbidden`）にする。
  `ashsat_` で始まって引けない値は 401（ST29 D19（仮））。正典「衛星の資格情報では、まだどの口も通らない」（premises P6）
- **経路の試験**: `satellite_routes_tests::every_route_refuses_satellite` は `lib.rs` の `.route(` の path の集合が内部の契約の path の集合と一致することを見て、`.merge(` `.nest(` `.nest_service(` を禁じる（premises P9）。`collector_scope_tests` も `ApiDoc::openapi()` の口を回す
- **CI の判定**: `tools/release/changes.sh code` は `docs/` と `openspec/` と `*.md` だけの変更で `rust` の job を飛ばす。`docs/satellite-api/` の写しだけを変えた PR では、写しの検査が走らない
- **配った印**: `v*` は 0 個（premises P8）。印を取り寄せるのは `rust` の job だけ（premises P10。`ci.yml:126-148`）
- **許諾の表示の検査**: `tools/check-spdx.sh` は先頭 25 行に `SPDX-License-Identifier:` を持つ追跡中のファイルだけを、決めた式と例外の表に照らす

## Goals / Non-Goals

**Goals**
- 衛星の作り手が、リポジトリの中のページを読めば、何をどう呼べばよいか（口・型・資格情報・断り・版）が分かる
- 同じ版の中で衛星を壊す変更は、main に入る前に CI が止める。写しを書き換えても抜けられない
- 前の版を黙って外せない。外す前の予告の形と、外した版を呼ばれたときの応答は、版 1 の衛星が受け取る形として今決めておく

**Non-Goals**
- 記録の封筒の形・衛星の書き込みの口（ST39）、承認（ST38）、衛星の読み出し（ST45）
- 内部の口を衛星向けの契約にも載せるか（ST39 の上流）
- AI が使う MCP の道具（ST27 の上流）
- 本人に知らせる画面（外す予定の版を最近使った衛星を出す）。最初に前の版を外す作業の上流が作る
- SDK（扉 #31 は SDK も Apache-2.0 と決めたが、この Story は SDK を作らない）

## Decisions

### D1. 衛星向けの契約は、同じ crate の 2 つ目の `#[derive(OpenApi)]` で、別の bin が書き出す（聞かない既定 1）

- `crates/server/src/satellite_api.rs` に衛星向けの口と型、`SatelliteApiDoc`（`#[derive(OpenApi)]`）を置く。`ApiDoc` には衛星向けの口も型も載せない
- 新しい bin `crates/server/src/bin/satellite_api.rs` が 2 つの出力を持つ: `contract <大きい版>`（契約の JSON を標準出力へ）と `page <大きい版>` / `index`（ページの Markdown を標準出力へ）。
  `src/bin/openapi.rs` は変えない（内部の口の契約は今のまま）
- 衛星向けの契約の `info.version` は `docs/satellite-api/versions.json` から入れる（D8）。組み立てに渡した版番号・出所の版を読まない
- 内部の口と同じ型を衛星向けの契約にも載せる場合（ST39 以降）は、その型の変更が衛星向けの契約の差にも出る。D9 の検査がそれを壊す変更として拾うので、黙って変わらない

反転条件: 2 つ目の derive が utoipa で組めない（Task 1 で分かる）なら、1 つの derive から作った文書を道で 2 つに分ける関数に替える（出力の形は同じ）。

### D2. 許諾は契約の欄と隣の LICENSE で示し、ファイルに SPDX の行を置かない（聞かない既定 2・扉 #31）

- `SatelliteApiDoc` は `info(license(name = "Apache-2.0", identifier = "Apache-2.0"))` を明示する
- `docs/satellite-api/LICENSE` は `tools/licenses/texts/Apache-2.0.txt` をそのまま写す（検査が同じ中身かを見る）。版をまたいで 1 枚にし、`v1/` には置かない（Q1 の context の既定は「`v1/` に並べる」だったが、版ごとに同じ全文を写すことになるので版の外に置いた。review/spec.md R5）
- 契約の JSON・ページの Markdown は `SPDX-License-Identifier:` の行を持たない。`tools/check-spdx.sh` の例外の表と、正典 distribution の「例外として通したもの 5 つ」の Scenario は変わらない（ST61 も distribution を触っているので、ここを動かさない）
- `README.md` の「ライセンス」の節と `CONTRIBUTING.md` の「ライセンス」の節に、「本体は AGPL-3.0-only（追加の許可つき）。衛星向けの API の契約（`docs/satellite-api/`）は Apache-2.0。契約から生成したクライアントには AGPL が及ばない」を足す。
  CONTRIBUTING の CLA は今のまま（契約への貢献も同じ CLA で受ける）
- ページを作る Rust のコード（`satellite_api.rs`・bin）は本体の一部で、AGPL の表示のまま

反転条件: ページの中にも SPDX の表示が要ると分かったら（外の道具が読むなど）、`check-spdx.sh` の例外の表に `docs/satellite-api/` を Apache-2.0 で足し、distribution の Requirement を書き直す change を出す。

### D3（仮）. ページは 2 枚で、契約から機械で作る（聞かない既定。Q5 を外して既定にした）

- `docs/satellite-api/README.md`（入口）: 何の契約か・許諾（Apache-2.0 と LICENSE へのリンク）・版の一覧の表（版・外す日・ページへのリンク）・版の規則（同じ版では壊さない・壊す変更は次の大きい版・前の版は 6 か月以上並べる・外す日は `Sunset` で予告）
- `docs/satellite-api/v1/README.md`（版ごと）: 資格情報の渡し方・口ごとの節（メソッドと道・説明・求め・応答の状態の番号と型）・型の表・断りの種別の表（`error` の値・状態・意味。「知らない値は状態の番号で扱う」）・契約の JSON へのリンク
- どちらも bin の出力で、手で書かない。検査（D10）が作り直して差を見る
- ページの文言は日本語。口・欄・値の名前は契約のまま

反転条件: 外の作り手から英語のページを求められたら、同じ bin で英語のページを足す（契約の JSON は同じ）。

### D4. 衛星向けの口の道は `/satellite/v<大きい版>/`（Q1 の推奨）

- 経路は `lib.rs` の router に `.route("/satellite/v1/me", get(...))` の字面で足す（`.nest(` を使わない。経路の試験の抽出がそのまま使える）
- 版の受け口 `.route("/satellite", any(...))`・`.route("/satellite/", any(...))`・`.route("/satellite/{*rest}", any(...))` の 3 本を置く（D11）。axum は字面の道を先に当てるので、`/satellite/v1/me` はこちらに来ない（`{*rest}` は空の区切りに当たらないので、`/satellite` と `/satellite/` は別に置く。review/spec.md R8）
- 大きい版を増やすときは、同じ形で `/satellite/v2/…` を足す（ハンドラは版ごとに分けてよい）

### D5（仮）. 衛星向けの口の断りの本文と種別（聞かない既定 3）

- 衛星向けの道（`/satellite` そのものか、`/satellite/` で始まる道。ST29 の管理の口 `/satellites…` は含まない）の断りは、すべて `application/json` で `{"error": "<種別>"}`（版の受け口は欄を足す。D11）
- 版 1 の種別: `unauthorized`（401。資格情報が無い・認められない）・`not_satellite`（403。API の合言葉・画面のログイン）・`unsupported_version`（404）・`not_found`（404）・`method_not_allowed`（405）・`internal`（500。読み出しの記録に書けない）
- `error` は契約の型で列挙にせず文字列にする（説明に既知の値を並べる）。後の Story（ST38 の `not_approved` など）が種別を足しても、型の選べる値が増えることにならず、D9 の規則で壊さない側に入る。
  ページに「知らない値は状態の番号で扱う」と書く
- 401 を JSON にするのは衛星向けの道だけ。内部の口の 401 / 403 の本文（`unauthorized` / `forbidden` の文字列）は変えない
- API の合言葉の 403 は、許可の一覧の判定（`collector_allows`）がそのまま断る。本文だけを衛星向けの道で JSON にする

反転条件: 衛星の作り手が種別の欄の名前に別の慣習（RFC 9457 の problem+json など）を求めたら、版 2 で替える（版 1 の形は残す）。

### D6（仮）. 確かめる口は `GET /satellite/v1/me` で、衛星自身と版の一覧だけを返す（Q4 の推奨・ST29 Q7）

- 200 の本文: `{"satellite": {"id": "<識別子>", "name": "<名前>"}, "api_versions": [{"version": "1.0", "sunset": null}]}`。`sunset` は `YYYY-MM-DD` か `null`
- 本体の中の番号（衛星・資格情報の uuid）・登録の時刻・最後に使われた時刻・承認は返さない
- 外した衛星・取り消した資格情報は `authorize_quiet` が引けないので 401（今の D19（仮）のまま）

反転条件: 衛星の作り手が「どの 1 本の資格情報で呼んだか」を知りたいと分かったら、資格情報の名前を欄として足す（足すのは壊さない側。小さい版を上げる）。

### D7. 衛星向けの API の版は製品の版と別に数える（聞かない既定。Q7 の推奨）

- 版は `<大きい版>.<小さい版>`。始まりは `1.0`
- 同じ大きい版の中で足す変更（D9）をしたら小さい版を 1 上げる。壊す変更は次の大きい版の `.0` から
- 内部の口の契約の版（出所の版。ST61 が数え方を変える）とも、送信契約の版（`GET /contract`）とも混ぜない

### D8（仮）. 版の一覧は `docs/satellite-api/versions.json` の 1 つ

- 形: `{"versions": [{"major": 1, "version": "1.0", "sunset": null, "notice": null}]}`。`sunset` は外す日（`YYYY-MM-DD`）、`notice` は本人に知らせる画面の Requirement の名前
- サーバはこのファイルを組み立ての時に埋め込み（`include_str!`）、確かめる口・版の受け口・`Sunset` の見出しに使う。試験は別の一覧を渡して組めるようにする（`Sunset` の Scenario は試しの材料で確かめる）
- 検査（D10）は、一覧の大きい版ごとに契約の写しがあり、`info.version` が一覧と同じことを見る
- 出した印と外してよい日は一覧に持たず、検査が印から計算する（Q6 の points の「版・出した印・外してよい日」の一覧とは欄が違う。手で書く欄を減らし、印と食い違わないようにした。review/spec.md R5）

反転条件: 版ごとの情報（出した印・移り先）を一覧に持たせる必要が出たら、欄を足す（サーバが読まない欄は足すだけで済む）。

### D9（仮）. 壊す変更の規則（Q2 の推奨: 自前の台本で、決めた規則を見る）

比べる相手の写し（old）と作り直した契約（new）を、同じ大きい版ごとに比べる。`$ref` は比べる前に `components.schemas` から展開する。

| 種類 | 規則 | 判定 |
|---|---|---|
| 口 | old にある（道, メソッド）が new に無い | 壊す |
| 口 | old の口の応答の状態の番号が new に無い | 壊す |
| 応答の型 | old の欄が new に無い・old の `required` の欄が new で必須でない | 壊す |
| 型 | 同じ場所の欄・引数の `type` か `format` が違う | 壊す |
| 送る側 | new の求めの本文・引数に、old に無い必須の欄がある・old で任意の欄が必須になった | 壊す |
| 送る側 | 求めの欄の `enum` の値が減った | 壊す |
| 資格情報 | `components.securitySchemes` の方式が消えた・変わった・口の `security` が変わった | 壊す |
| 許諾 | `info.license` が違う | 壊す |
| 口・欄 | new に口・応答の状態の番号・応答の欄・任意の送る欄が増えた | 足す（小さい版の上げ忘れを見る） |

- 応答の欄の `enum` は、版 1 では使わない（D5 の種別は文字列）。使うようになったら「応答の `enum` が増えた」を壊す側に足す
- 台本は bash と jq で書く（`tools/check-contract-version.sh` と同じ道具）。`--self-test` が規則ごとに、わざと壊した材料で落ちることと、足す変更で通ることを見る
- 規則に無い壊し方（参照の付け替えで意味が変わる、説明だけで意味を変える など）は見逃しうる（Q2 の代償）

反転条件: 規則に無い壊し方で衛星が壊れたら、規則を足す。規則の数が手に負えなくなったら、OpenAPI の差分の道具（oasdiff）を版で固定して CI に入れる（Q2 のもう 1 つの案）。

### D10（仮）. 衛星向けの契約の検査は新しい台本 `tools/check-satellite-contract.sh`（聞かない既定。Q3 の推奨）

- 1 回の実行で見るもの: (1) bin で作り直した契約とページが、コミットされたものと同じ (2) LICENSE の中身・README と CONTRIBUTING のライセンスの節 (3) 版の一覧と契約の写しの一致 (4) 比べる相手との壊す変更・小さい版 (5) 前の版を外す・外す日を置く規則（D12）
- 比べる相手は引数 `--base <commit>`。CI は `changes` の job と同じ比べる相手（今は `github.event.pull_request.base.sha || github.event.before`。ST61 が main への push の比べる相手を「前に成功した main の run」に変えたら、それに合わせる）を渡す。渡されない（手元）なら `git merge-base HEAD origin/main`
- 作り直した契約とページは、既定では bin で作る（本番。`cargo run` を含むので `scripts/quiet-run` で走らせる）。`--generated <フォルダ>` を渡すと、そのフォルダの契約とページを作り直したものとして使う（`--self-test` が材料の JSON を置いて当てるため。場合ごとに Rust を組み直さない。review/spec.md R16）。作り直したもの・コミットされた写し・比べる相手は別の材料で、写しの書き換えでは抜けられない
- ページの相対リンクの先が木にあることも (1) で見る（review/spec.md R19）
- 検査は最初に落ちたところで止めず、(1)〜(5) の当たったものを全部示してから 0 でない終了コードで終わる（材料の木では、作り直した契約とコミットされた写しのずれと規則の違反が同時に出ることがある。Scenario の THEN は、示されたものの中に当たった規則があるかで見る）
- 比べる相手の commit を読めない → 落とす。比べる相手に `docs/satellite-api/v<N>/openapi.json` が無い → その版は比べずに通し、そう示す
- 今日の日付は `SATELLITE_CONTRACT_TODAY`（`YYYY-MM-DD`）で差し替えられる（自己試験と 6 か月の Scenario のため）。無ければ UTC の今日
- `tools/check-openapi.sh` に書き足さない（ST61 が触っている。共有の台本の取り合いを避ける）
- CI は `rust` の job の、印を取り寄せた後（`check-contract-version.sh` の後）に `--self-test` と本番の 2 行を置く
- `tools/release/changes.sh` に新しい判定 `satellite`（`^docs/satellite-api/`・`^README\.md$`・`^CONTRIBUTING\.md$`・この台本と `changes.sh` 自身に触れたら真）を足し、`changes` の job の出力に `satellite` を足し、`rust` の job の `if` を `code || satellite` にする。
  `code` の意味（製品に触れたか。ST61 が番号を付ける判定に使う）は変えない（review/spec.md R17・R18）

反転条件: `rust` の job が重くなりすぎたら、印の取り寄せと一緒に別の job へ移す（印を取り寄せない job に置くと、印が 0 個に見えて外す規則が黙って通る）。

### D11（仮）. 版の受け口と、資格情報・版の判定の順

- 受け口（`/satellite`・`/satellite/`・`/satellite/{*rest}`）は、`{*rest}` の最初の区切りが `v<一覧にある大きい版>` なら 404 `{"error": "not_found"}`、そうでなければ（区切りが無いものを含む）404 `{"error": "unsupported_version", "api_versions": [...]}`
- 契約の道のメソッド違いは、その経路の `MethodRouter` の fallback が 405 `{"error": "method_not_allowed"}` を返す（axum の既定の空の 405 にしない）
- 判定の順: 読み出しの記録の middleware が資格情報を先に見る（無い・認められない → 401、API の合言葉・画面のログイン → 403 `not_satellite`）。衛星の資格情報だけがハンドラ（確かめる口・版の受け口）に届く
- 受け口の行の口の名前は経路の形（`/satellite/{*rest}` など。axum の `MatchedPath`）。届いた道の文字列は記録に残さない（D13）
- `Sunset` の見出しは、`/satellite/v<N>/` で始まる道の応答に、middleware の外側の層で付ける（401・403 の断りにも付く。review/spec.md R7）
- 経路の試験は、router の `.route(` の path の集合が「内部の契約の path ∪ 衛星向けの契約の path ∪ `/healthz` ∪ 受け口の 3 本」に一致することを見る（新しい Scenario。`every_route_refuses_satellite` から分けた。review/spec.md R12）。403 で断ることは内部の契約の口で見る

### D12（仮）. 前の版を外す規則と、外す日の予告（Q6・Q8 の推奨）

- 外す変更: 比べる相手の版の一覧にある大きい版が、new の版の一覧・契約の写し・サーバの経路（`SatelliteApiDoc` の道）のどれかから消えた
- 外せる条件: (a) 外す版 N の次の版 N+1 の契約の写し `docs/satellite-api/v<N+1>/openapi.json` を最初に含んだ印 `v*`（写しを含む印のうち、配った日が最も早いもの）の配った日から 6 か月（暦の月で、同じ日。同じ日が無い月は月の最終日）以上たった。
  配った日は、注釈つきの印を作った日（tagger の日）と印の commit の日の遅い方（軽い印は commit の日）。日は UTC の日付
  (b) 版の一覧の N の項目の `notice` が、`openspec/specs/**/spec.md` の `### Requirement:` の名前と一字一句同じ。両方を満たさなければ落とす（review/spec.md R1・R2）
- 外す日を置く変更: N の項目の `sunset` が (a) の 6 か月の終わりより前なら落とす。N+1 の印が無ければ `sunset` は置けない
- サーバの `Sunset` の見出しは RFC 8594 の形（IMF-fixdate。`sunset` の日の 00:00 UTC）。`Deprecation` の見出しと `Link: rel="successor-version"` は付けない（版の一覧の `sunset` と確かめる口で足りる）
- いま `notice` を持つ項目は無く、印も無いので、外す変更はいつも落ちる（Q8: 知らせの画面ができるまで版は外せない）。この条件を変えるのは、知らせの画面を作る作業（今はどの Story も持っていない）

反転条件: 配った日を公開の日（GitHub の Release）で数えるべきと分かったら、Release の日を読む形に替える（遅い側に動くだけ）。この repo の印の作り方（注釈つきか）は確かめていない（`tools/`・`.github/` に `git tag` を作る行は無い。ST61 が作る）

### D13（仮）. 衛星向けの道の読み出しの記録

- middleware は衛星向けの道（`/satellite` そのものか `/satellite/` で始まる経路。`/satellites…` は今どおり）で、衛星の資格情報を `Verdict::Pass` にする（ほかの経路は今どおり `Forbidden`）。API の合言葉・画面のログインは `Forbidden`、資格情報無しは `Unauthorized`
- 衛星向けの道だけ、行は**ハンドラの後に**、返す状態で書く（今の middleware はハンドラの前に固定の状態で書く。衛星向けの口は記録を返さず DB を変えないので、後で書いても「行の無い読み出し」は生まれない。review/spec.md R9）:
  確かめる口の 200 → 結果 `ok`・状態 200、受け口の 404・405 → 結果 `forbidden`・状態 404 / 405、断りの 401 → `unauthorized`、403 → `forbidden`（結果の語は今の CHECK の中から選び、移行を足さない。ST59 の D9（仮）と同じ形）
- 行を書けなければ、作った応答を捨てて 500 `{"error": "internal"}` を返す
- ほかの道の middleware の動き（ハンドラの前に書き、書けなければハンドラを呼ばずに 500）は変えない
- 行の形は satellite-platform に ADDED で置く。正典 data-sensitivity「誰がいつ読んだかを…」は、「（403）のとおりに書く」の 1 文だけを 404・405 も含むように直す（MODIFIED。ST59 も同じ Requirement を書き直すが、直す文は別で、archive の 3-way で合わせる。review/spec.md R10）

反転条件: 版の受け口の 404 を `forbidden` と読むのが読み手を誤らせると分かったら、CHECK に `not_found` を足す移行を作る（`migrations/ORDER` の末尾）。

### D14. 載せないもの（聞かない既定 4・5）

- AI が使う MCP の道具は衛星向けの契約に入れない（`docs/handoff/ST27.md` C3）
- 記録の封筒の形は ST39 が同じ版に追加として載せる。内部の口を両方に載せるかも ST39 の上流（`docs/handoff/ST39.md`。深掘りの段で書いた）

## Risks / Trade-offs

- **版 1 は main に入った日から凍る**（Q3 の代償）: 確かめる口・資格情報の渡し方・401 / 403 / 404 の断りの形・版の受け口の形は、衛星がまだ無くても、変えたくなれば版 2 にして 6 か月以上並べる。ST38・ST39・ST45 の上流は、口を足す（追加）形でしか版 1 に積めない
- **規則に無い壊し方は見逃しうる**（Q2 の代償）: D9 の表の外の変更は通る。衛星が書き・読み始めた後（ST39・ST45）に見逃すと、衛星が書くはずだった記録が取れない
- **版を外せない**（Q8 の代償）: 知らせの画面を作る Story がまだ無い。版 2 を出しても、版 1 は外せないまま並び続ける
- **配った日は印の日**: 印を作った日と commit の日の遅い方で数えるので、印を後から付けても早い側にはずれない。ただし GitHub の Release の公開が印より遅れると、その分だけ作り手の移る期間は 6 か月より短い
- **`satellite` の判定で、`docs/satellite-api/`・`README.md`・`CONTRIBUTING.md` だけを変える PR でも `rust` の job が走る**: README を直す PR も重い job を待つ。回数は少ない

## Migration Plan

DB の移行は無い。配った版が無い（premises P8）ので、版 1 は最初の配布から衛星向けの契約を持つ。

## Open Questions

無し（第 1 回の問いの推奨で書いた。答えが推奨と違えば `spec --answers` が写し直す）。
