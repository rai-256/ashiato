# ST42 の前提（外の形式・振る舞い）

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| P1 リポジトリは公開で、外の作り手が読める | 契約のページの置き場（Q5）・版の形（Q1） | `gh repo view --json visibility`（2026-11-27） | `PUBLIC` | 実物 | 5 |
| P2 GitHub Pages は有効にしていない | ページの形（Q5。HTML を置いても GitHub はソースとして見せる） | `gh api repos/{owner}/{repo}/pages`（2026-11-27） | 404（Pages の設定が無い） | 実物 | 5 |
| P3 GitHub はリポジトリの中の Markdown を、ブラウザで整形して見せる | ページの形（Q5） | repo の `README.md`・`docs/*.md` が整形されて見える（いつもの使い方） | 見える | 公式文書（https://docs.github.com/en/get-started/writing-on-github） | 2 |
| P4 いまの内部の契約は `AGPL-3.0-only WITH …` を宣言し、securitySchemes を持たず、`info.version` は出所の版 | 衛星向けの契約の宣言を上書きする要否 | `jq '.info, .components.securitySchemes' docs/openapi.json` | license は AGPL、securitySchemes は null、version は `0.1.1` | 実物 | 5 |
| P5 utoipa は 1 つの crate に `#[derive(OpenApi)]` の型を 2 つ持て、`info(license(...))` で crate の既定を上書きできる | 2 つの契約を同じ crate から別々に生成する（完了の判定 1） | `crates/server/src/lib.rs:5046` の `ApiDoc`・utoipa 5 の文書 | 1 つ目はある。2 つ目と上書きは走らせていない | 既存のテスト材料 / 公式文書（https://docs.rs/utoipa/5） | 2 |
| P6 衛星の資格情報は `Authorization: Bearer ashsat_<識別子>_<乱数>` で渡し、いまは資格情報の要らない口のほかすべて 403 | whoami の口（Q4）・securitySchemes | `openspec/specs/satellite-platform/spec.md`「衛星の資格情報では、まだどの口も通らない」・`crates/server/src/lib.rs:845` | 正典とコードが一致 | 実物 | 4 |
| P7 OpenAPI の差分で壊す変更を判定する道具は、手元にも CI にも入っていない | 判定の作り（Q2） | `which oasdiff` | 無い | 実物 | 5 |
| P8 配った印（タグ `v*`）はまだ 0 個 | 版の比べの基準（Q3）・並べる期間の起点（Q6） | `git tag -l 'v*'` | 0 個 | 実物 | 5 |
| P9 経路の試験は、内部の契約の path とサーバの経路の一致を見て、経路の入れ子（`.nest(` など）を禁じ、衛星の資格情報では資格情報の要らない口のほかすべてが 403 であることを見る | 衛星の口を別の契約に置く・道に版を入れる・確かめる口を置く（Q1・Q4） | `crates/server/src/satellite_routes_tests.rs:105-127` を読んだ（review/deep.md R5） | 3 つとも、そのままでは落ちる。試験を 2 つの契約の和で見る形に直す | 実物 | 5 |
| P10 印（`v*`）を取り寄せて読めるのは CI の rust の job だけ。最初の印の時期は決まっていない | 印を読む検査の置き場・前の版を外してよい日（Q6） | `.github/workflows/ci.yml:126-148`・`git ls-remote --tags origin`（空） | rust の job に置く。印が出るまで外してよい日は決まらない | 実物 | 5 |

失われるもの（`loss`）のある決定はこの Story に無い（Q1 は review/deep.md R8 で「仮でよい」に分け直した）。
