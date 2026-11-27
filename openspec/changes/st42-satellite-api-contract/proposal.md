# ST42 — 衛星向けの API の契約を、版を付けて読める形で公開する

## Why

衛星（本体の外のアプリ）は本体と別に作られ、別に更新される。本体の更新で衛星が黙って壊れると、壊れている間に衛星が書くはずだった記録は取れない（FR-89）。
いまの API の契約（`docs/openapi.json`）は本体の収集アプリと画面の口だけを持つ。資格情報の渡し方も、衛星向けの版の規則も無く、許諾は AGPL のままである（premises P4）。
このまま衛星の口を足すと、内部の口の変更がすべて衛星への壊す変更に数えられる。衛星の口を作る ST38・ST39・ST45 より先に、契約を分け、版を付け、壊す変更を CI で止める。

## What Changes

- 衛星向けの API の契約を、内部の口の契約とは別の生成で作る。`docs/satellite-api/v1/openapi.json` に置き、Apache-2.0 を宣言する（扉 #31。crate の既定の AGPL を上書きする）。
  資格情報の渡し方（`Authorization: Bearer ashsat_…`）と断りの形（種別の欄 `error` を持つ JSON）を載せる
- 契約から、ブラウザで読める Markdown のページ（`docs/satellite-api/README.md`・`docs/satellite-api/v1/README.md`）を機械で作る。Apache-2.0 の全文 `docs/satellite-api/LICENSE` を隣に置く。
  `README.md`・`CONTRIBUTING.md` のライセンスの節に、本体（AGPL-3.0）と契約（Apache-2.0）を分けて書く
- 衛星の口は道に大きい版を持つ（`/satellite/v1/…`。Q1）。契約の版は `1.0` の形で、製品の版と別に数える（Q7）。版の一覧の正本は `docs/satellite-api/versions.json` の 1 つにする
- 自分が誰かを確かめる口 `GET /satellite/v1/me` を置く（ST29 Q7）。衛星の識別子・名前と、サーバが受ける版の一覧を返す（Q4）。記録は返さない
- サーバに無い版・外した版を呼ばれたら、404 と版の一覧で断る。契約に無い道は 404 `not_found`、違うメソッドは 405。外す日が決まった版の応答（断りを含む）には `Sunset` の見出しを付ける（Q8）
- 新しい検査 `tools/check-satellite-contract.sh` を置く。同じ版の中の壊す変更（決めた規則。Q2）と、小さい番号の上げ忘れ（Q7）があれば CI を落とす。比べる相手は main にある版ごとの写し（Q3）。
  前の版を外す変更も CI が止める。外せる条件は、新しい版を最初に配った日から 6 か月たち（Q6）、本人に知らせる画面の Requirement が正典にある（Q8）ことで、いまはどちらも満たさない
- `docs/satellite-api/`・`README.md`・`CONTRIBUTING.md` だけを変えた PR でも `rust` の job が走るよう、`tools/release/changes.sh` に新しい判定 `satellite` を足す（製品に触れたかの判定 `code` は変えない）
- 正典「衛星の資格情報では、まだどの口も通らない」を、衛星向けの契約の口だけを例外にする形に書き直す

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `satellite-platform`: 衛星向けの API の契約・版・壊す変更の検査・前の版を並べる規則・無い版の断り・確かめる口・衛星向けの口の読み出しの記録を足す（ADDED 7 本）。
  「衛星の資格情報では、まだどの口も通らない」を、契約の口だけを例外にする形に書き直す（MODIFIED 1 本）
- `data-sensitivity`: 「API の合言葉で通る口は、送る口と通知に要る読みの許可の一覧だけ」の「画面のログインの印ではほかの口は全部通る」から、衛星向けの契約の口を外す（MODIFIED。Scenario は今のまま）。「誰がいつ読んだかを、追記のみの読み出しの記録に残す」の「（403）のとおりに書く」を、衛星向けの道の 404・405 も含むように直す（MODIFIED。ST59 も同じ Requirement を書き直すが、直す文は別。archive の 3-way で合わせる）。MODIFIED は 2 本

capability の照合（`docs/stories/INDEX.md`）: ST42 は `satellite-platform`（API の版）と `record-envelope`（API 契約）に割り当てられている。
`record-envelope` に積むのは FR-103 の「記録の封筒の形」で、衛星の書き込みを作る ST39 が同じ版に追加として載せる（review/deep.md R4、`docs/handoff/ST39.md`）。ST42 の時点では衛星が書く封筒が無いので、`record-envelope` には delta を置かない。
`distribution` の「ソースと部品の許諾の表示は…1 つの式にそろえる」は変えない。衛星向けの契約のファイルは `SPDX-License-Identifier:` の行を持たず、許諾は契約の `info.license` と隣の LICENSE で示す（design D2）。
前倒しは無い。`docs/stories/INDEX.md` の capability 表の `data-sensitivity` の行に ST42 を足し、`record-envelope` の ST42 に「封筒は ST39 が同じ版に追加する」を注記した。

## FR-103 のうち ST42 で満たさないもの

- **記録の封筒の形**: 衛星が書く口（ST39）が無いので、ST42 の契約には載らない。ST39 が衛星向けの契約の同じ版に追加として載せ、そこで FR-103 の封筒の部分を満たす（deferred ST39。`docs/handoff/ST39.md`）
- **内部の口（取り込みの口など）を衛星向けの契約にも載せるか**: Story の完了の判定どおり、衛星の書き込みを作る ST39 の上流が決める
- **AI が使う MCP の道具**: この契約に入れない（`docs/handoff/ST27.md` C3。ST27 の上流が決める）
- **本人に知らせる画面**（外す予定の版を最近使った衛星を出す）: 最初に前の版を外す作業の上流で作る。いまはどの Story も持っていない。それまで版を外す変更は CI が止める（Q8）

## Impact

- サーバ（`crates/server`）: `lib.rs`（router に衛星の口と版の受け口・衛星向けの契約の型 `SatelliteApiDoc`）・新しい `satellite_api.rs`（確かめる口・版の一覧・断りの本文・`Sunset`）・`access_log.rs`（衛星向けの口では衛星の資格情報を通す）・
  新しい bin `src/bin/satellite_api.rs`（契約とページを書き出す）・`satellite_routes_tests.rs`・`collector_scope_tests.rs`（2 つの契約の和で見る）・新しい `st42_tests.rs`。移行は無い
- 文書: 新しい `docs/satellite-api/`（`versions.json`・`LICENSE`・`README.md`・`v1/openapi.json`・`v1/README.md`）・`README.md`・`CONTRIBUTING.md`
- 道具と CI: 新しい `tools/check-satellite-contract.sh`（`--self-test` つき）・`tools/release/changes.sh`（新しい判定 `satellite`）・`.github/workflows/ci.yml`（`changes` の job の出力 `satellite`・`rust` の job の `if` と、印を取り寄せた後に 2 行）
- 内部の契約 `docs/openapi.json` は変わらない（衛星の口を載せない。D1）。`tools/check-openapi.sh` には触れない
- **走っている change との重なり**:
  - ST61（`st61-auto-version-bump`）は `tools/check-openapi.sh`・`docs/openapi.json` の `info.version`・`ci.yml`・`tools/release/changes.sh`（`base` と `--self-test` を足し、`code` を番号を付ける判定に使う）を触る。ST42 は `code` の意味を変えず、別の判定 `satellite` を足す（review/spec.md R18）。main への push の比べる相手は `changes` の job に合わせる（D10）。ST42 は `check-openapi.sh` に書き足さず、衛星向けの契約の版を出所の版から引かない（D7）ので、ST61 の版の変更は衛星向けの契約に効かない。`ci.yml` は `rust` の job の別の行に足す
  - ST59（`st59-change-password-from-web`）は `lib.rs` の router・`access_log.rs` の素通しの口・`docs/openapi.json`・正典 data-sensitivity「誰がいつ読んだかを…」を触る。ST42 は同じ Requirement を書き直さず、衛星向けの口の読み出しの記録を `satellite-platform` に ADDED で置く（D13）。router と middleware は別の行で、後から merge する側が rebase で追従する
  - CH21・CH26 は `ci.yml` を触る（`rust` の job の `check-private.sh --self-test` の後・`changes` の出力ほか）。ST42 は印を取り寄せた後の検査の並びに足し、同じ行を取り合わない
- **requires**: ST29（衛星の資格情報）は archive 済み
- **できなくなること**: 衛星向けの契約に載った口と型は、同じ版の中で消す・型を変える・必須を足すことができなくなる（main に入った日から。Q3）。前の版を外すことは、本人に知らせる画面ができるまでできない（Q8）
