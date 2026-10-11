# 整合の確認（2026-10-11・衛星向けの契約とライセンス）

- 基準: `9eda032e241a84cfb633060a53344539504009ae`..`de68884b0e1bda23b23717295fdab0ff2138df20`

対象は PR #62（FR-103 の追加、FR-89 / FR-95 を衛星向けの API に絞る、§5 と扉 #31 で衛星向けの契約と SDK を Apache-2.0、ST42 の書き直し、ST39 / ST45 の requires に ST42）。
文書は head（`de68884b`）の内容で読んだ。作業ツリーにある未 commit の変更（`docs/stories/ST14.md` `ST38.md` `stories.json`）は見ていない。

機械の入力:
- `req_impact.py --base 9eda032e`: FR-89・FR-95（改訂）、FR-103（新規）、扉 #31（新規）。名指しは `docs/production-prep.md` 28・36 行、`docs/stories/INDEX.md` 64 行、`docs/stories/stories.json` 985〜987 行
- `check_chain.py`: 未回収 0・warn 0。FAIL は 2 件で、どちらも既存の `docs/audit/2026-10-11-consistency.md`（作業ツリーで変更中）の書式。今回の改訂とは関係ない

## C1. 衛星が宣言を送る口（ST38）が ST42 より先に内部の契約へ入る道が開いている
- 重大度: 高
- 文書: docs/stories/ST38.md:3
- 要件: FR-62, FR-97, FR-103, FR-89
- 根拠: ST38 は `requires: [ST29]`、layer 3 で、ST42（`requires: [ST29]`、layer 3）とどちらの向きにも依存が無い。ST38 の完了の判定は「衛星の登録の時に読む種類・書く種類・書く記録の分類を宣言させる」「衛星が更新で宣言を増やすと、増えた種類だけが未承認になる」。FR-97 は「登録済みの衛星が宣言する種類を増やす」なので、衛星が自分の資格情報で呼ぶ口になる。一方 ST42 の完了の判定は「衛星の資格情報で受ける口が内部の契約に載っていると CI が落ちる」。INDEX の扉の表（`docs/stories/INDEX.md:296`）は「衛星の口を作る ST39 / ST45 より先に ST42 が契約を分けて反映する」と ST39 / ST45 しか挙げず、ST38 が抜けている。ST38 が先に来ると、衛星が使う口が版の無い内部の契約（`docs/openapi.json`）に載り、FR-89 の守りの外で作られる。ST42 が先に来ると、ST38 は自分の口が衛星向けの契約に載る前提を知らないまま作られ、CI で落ちる
- 直すところ: その文書（ST38 の requires に ST42 を足すか、INDEX の扉 #31 の行に ST38 を足す）
- 機械: 安く捕まえられる（ST42 の CI の検査ができた後なら、衛星の資格情報を受ける口が内部の契約にあれば落ちる。それまでは、`satisfies` に FR-62 / FR-97 のような「衛星が呼ぶ」要件を持つ Story が ST42 を requires していないことを check_chain で見られる）
- 行き先: stories

## C2. ST42 の完了の判定が、未決の「書きは取り込み口 1 本か、口を足すか」を先に決めている
- 重大度: 高
- 文書: docs/stories/ST42.md:54
- 要件: FR-103, FR-99, FR-64
- 根拠: ST42 は「衛星の資格情報で受ける口が内部の契約に載っていると CI が落ちる」。一方 `docs/production-prep.md:30` は「書きは取り込み口 1 か所に不変条件を集める」、`:32`（書きの新しい器）は「上の『書きは取り込み口 1 本』に入れるのか、口を足すのかは**衛星の基盤の Story（/stories で立てる）の上流で決め、ここに戻す**」で未決のまま。FR-99 は衛星の記録にも FR-20 / FR-21 / FR-23 / FR-27 を課していて、これは取り込み口（`/ingest`）が持つ決まりと同じ。衛星の書き込みを `/ingest` で受けると決めた場合、`/ingest` は収集アプリ（内部）と衛星の両方の資格情報を受けることになり、ST42 の検査で必ず落ちる。逆に `/ingest` を衛星向けの契約へ移すと、収集アプリの口が版で固定され、FR-103 の★が挙げた「内部の口は本体の都合で変えてよい」が崩れる。いまの `docs/openapi.json` の `info.description` も「書き込みは 1 本の取り込み口に集約する」と言っている
- 直すところ: その文書（ST42 の完了の判定を「衛星向けの口と内部の口が重なるときの扱い」を ST39 の上流と同じ所で決める形にするか、production-prep の未決に「ST42 の完了の判定が口を分ける側に寄せている」と書く）
- 機械: できない（どちらに決めるかは人間の判断。決まった後は ST42 の検査で捕まる）
- 行き先: stories

## C3. AI の衛星が使う MCP が、版と契約（FR-89 / FR-95 / FR-103）の対象かどうか書かれていない
- 重大度: 中
- 文書: docs/requirements.md:722
- 要件: FR-59, FR-60, FR-89, FR-103
- 根拠: FR-103 は「衛星が使う API の契約（口・型・記録の封筒の形・資格情報の渡し方・版）」を公開する。FR-59 の★は「**AI は衛星の 1 つとして扱い、つなぎ方はいったん MCP とする**」、FR-60 は「MCP 経由で問い合わせを受ける」。ST42（`docs/stories/ST42.md`）も ST27（`docs/stories/ST27.md` の完了の判定「Claude から MCP 経由で問い合わせると…」）も、MCP の道具（tool）の形に版を付けるか、衛星向けの契約のページに載せるかを書いていない。OpenAPI で書く衛星向けの契約には MCP は載らないので、どちらとも読める
- 直すところ: requirements（FR-103 か FR-89 の★で MCP を含めるか外すかを 1 行で決める）
- 機械: できない
- 行き先: handoff ST27

## C4. 「ライセンスの地図 L2 の推奨」と書いているが、L2 の推奨は内部の `docs/openapi.json` そのものを Apache-2.0 にする形だった
- 重大度: 低
- 文書: docs/requirements.md:1142
- 要件: 扉 #31, FR-103
- 根拠: 要件は「本体（S-01 ほか）と、内部の口の契約（`docs/openapi.json`）は AGPL-3.0 のまま。理由: …（ライセンスの地図 L2 の推奨）」。`docs/audit/2026-10-09-license-map.html` の L2 は「推奨 (a) Apache-2.0。**docs/openapi.json と、将来の SDK のフォルダにだけ別の LICENSE を置く**」。Apache-2.0 を選んだことは L2 の推奨どおりだが、内部の `docs/openapi.json` を AGPL に残して契約を分けるのは L2 から変えた点で、出所の書き方だと L2 がそう推奨したと読める（同じ地図の「画面用の内部 API を衛星の契約にしない」は分ける側の根拠になる）
- 直すところ: requirements（「L2 の推奨（Apache-2.0）。置き場は L2 の『docs/openapi.json に LICENSE』から変え、契約を分けた側に置く」のように出所と変えた点を分ける）
- 機械: できない
- 行き先: requirements

## C5. README・CONTRIBUTING はリポジトリ全体を AGPL-3.0-only と言い、ST42 の完了の判定はそれを直すことを含まない
- 重大度: 低
- 文書: README.md:45
- 要件: 扉 #31, FR-103
- 根拠: README「AGPL-3.0-only（[`LICENSE`](LICENSE)）」、`CONTRIBUTING.md:5`「本体は **AGPL-3.0-only** です」。§5（`docs/requirements.md:1140`）はリポジトリの中に Apache-2.0 の契約のページ（FR-103 は「リポジトリの中のブラウザで読めるページ」）を置く。ST42 の完了の判定は「Apache-2.0 を宣言し」だけで、別の LICENSE ファイルや README の「ライセンス」節を更新することは入っていない。いまは Apache-2.0 のものがまだ無いので誤りではないが、ST42 が契約を出した日から README がリポジトリの一部について間違う
- 直すところ: その文書（ST42 の完了の判定に「Apache-2.0 の LICENSE を契約の置き場に置き、README のライセンスの節に書く」を足す）
- 機械: 安く捕まえられる（Apache-2.0 を宣言する契約ファイルがあるのに README に `Apache-2.0` が無ければ落とす grep）
- 行き先: stories

## K1. 「`lib.rs` から生成した `docs/openapi.json` が AGPL-3.0-only を宣言」の出どころは `lib.rs` でなく `Cargo.toml` の `license`
- 重大度: 中
- 文書: docs/requirements.md:1143
- 要件: 扉 #31, FR-103
- 根拠: 要件「いまの `docs/openapi.json`（`crates/server/src/lib.rs` から生成）は `AGPL-3.0-only` を宣言している」。`docs/openapi.json:6-9` の `info.license` は `AGPL-3.0-only`。しかし `crates/server/src/lib.rs:2330-2401` の `#[openapi(... info(title, version, description))]` に license は無く、値は `Cargo.toml:8` の `license = "AGPL-3.0-only"`（`crates/server/Cargo.toml:5` の `license.workspace = true`）から utoipa が引いている。ST42 が同じ crate から衛星向けの契約を生成すると、明示して上書きしない限り AGPL-3.0-only が付く。ST42 の実装者が「lib.rs の宣言を変えれば済む」と探して見つけられない
- 直すところ: requirements（出どころを `Cargo.toml` の `license`（utoipa が引く）に直す）か、ST42 の上流で拾う
- 機械: 安く捕まえられる（ST42 の検査で、衛星向けの契約の `info.license.identifier` が `Apache-2.0` であることを jq で見る）
- 行き先: requirements

## K2. FR-103 の★「版も無く」は、`info.version` が `0.1.0` である事実と食い違って読める
- 重大度: 低
- 文書: docs/requirements.md:724
- 要件: FR-103, FR-89
- 根拠: 要件「資格情報の渡し方（securitySchemes）も版も無く」。`docs/openapi.json` に `securitySchemes` は無い（確かめた）が、`info.version` は `"0.1.0"`（`crates/server/src/lib.rs` の `version = "0.1.0"`）がある。無いのは FR-89 の言う「衛星を壊さない約束としての API の版」（パスや契約ごとの版）で、OpenAPI の版の欄ではない
- 直すところ: requirements（「API の版（パスや契約ごとの版。`info.version` はパッケージ版の写し）」のように言い分ける）
- 機械: できない
- 行き先: requirements

## 未確認

- ST42 の完了の判定「同じ版の中で既存の衛星を壊す変更を入れると CI が落ちる」に使える差分検査（oasdiff など）が CI で動くかは試していない
- `tools/check-licenses.sh` の許可一覧（`AGPL-3.0-only` と `MPL-2.0` を含む）は、SDK を Apache-2.0 で配るときに SDK の依存へそのまま使えないと production-prep が書いているが、SDK はまだ無いので確かめようがない
- CLA（`docs/CLA.md:16`「提供分を AGPL-3.0 以外の条件でもライセンスすること」）が、Apache-2.0 の部分への貢献を受けるのに足りるかの法的な読みはしていない（文言の上では AGPL 以外の条件を許すので足りるように読める）

## 触らなくてよいと判断したもの（理由つき）

- `docs/openapi.json` と `lib.rs` の説明文「読みは PostgREST が別に自動生成する」—— 既存の `docs/audit/2026-10-11-consistency.md` の C9 / K1 で処置済みの指摘。今回の改訂で変わっていない
- `docs/production-prep.md:149` `:235` の「API の契約」1 本の記述 —— 2026-09-08 の実行記録で、A-1 の行（`:36`）が 2 本に分けると追記している
- INDEX の layer —— ST42 は ST29（layer 2）の後で layer 3、ST39 / ST45 は layer 4 で、requires の順と合う。`check_chain.py` の再生成の一致も通っている
- 扉 #31 の決着状態「決定済」と §5 の「締めると割れる（緩める向きには変えられる）」—— 向きは一致している
- ライセンスの地図の L3（V-02 の共同著作物を要件に書く）—— 既存の整合の確認（`2026-10-11-consistency.md:270`）が「当面は個人での開発で保留」と判断しており、今回の改訂は L2 だけを扱った
- 走っている change（`st08-browser-history` `st12-archive-ingestion` `st21-place-registry` `st25-day-timeline`）—— 衛星の口・契約・ライセンスに触れない。印を付けるものは無い
