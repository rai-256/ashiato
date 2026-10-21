# ST29 の前提（外の形式・振る舞い）

深掘りの問いを立てる前に確かめた前提。確かさは story-upstream の段（1 言っただけ / 2 出所 / 3 辿った / 4 走らせた / 5 実物で再現）。
**失われるもの（`loss`）のある決定は確かさ 4 以上の前提にだけ載せる。**

この Story の決定は、外のサービスの書き出し・OS・端末の振る舞いにほとんど依らない（資格情報も衛星の登録も本体の中で閉じる）。
依るのは、本体がいま使っている名前の形と、HTTP の資格情報の書き方だけ。

| # | 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|---|
| P1 | 本体の論理ソースの名前（`c01-…` `c02-…` `c03-…` `s01-…`）は英小文字・数字・`-` だけで、`:` も `.` も含まない | Q1（衛星の名前を `<衛星の識別子>:<名前>` で前置すると、本体の名前とは形で衝突しない） | `grep -rhoE "'(c0[1-3]\|s01\|v01)[^']*'" migrations/*.sql crates/` で名前を全部抜き、`[:.]` を含むものを数える | 31 種類、`:` と `.` を含むものは 0。ただし本体は**前置き**にも意味を持たせている（`grep -rn "LIKE 'c03-\|startsWith(\"c03-"` → `lib.rs` の書庫の状態と稼働状況・`202610141130_outage_alert.sql:31`・`web/src/App.tsx:54`・`archives.ts:103`）。本体の前置きの形で始まる衛星の識別子と名前は断る（Q1 の既定。deep-review R2） | 実物（repo の移行とコード） | 4 |
| P2 | 種類（FR-88）の登録簿はまだ無い。本体の種類の名前の形は ST38 が決める | Q1（種類の名前にも同じ前置の規則を課す） | `grep -rn "kind" migrations/` / `openspec/specs/` に種類の登録簿が無いことを見る | 無い。ST38（`satellite-platform` / `data-sensitivity`）が作る。本体の種類の名前に `:` を使わないことを ST29 の規則として先に置く | 実物（repo） | 3 |
| P3 | `Authorization: Bearer <値>` の値に使える文字は token68（英数字と `-._~+/` と末尾の `=`） | Q4（資格情報の文字列に衛星の識別子を入れる形）。識別子に `:` を入れると Bearer の値の外に出る | RFC 6750 §2.1 を読む | そのとおり。衛星の識別子（英小文字・数字・`.`・`-`）と区切りの `_` は token68 に収まる。`:` は収まらないので、資格情報の文字列には名前の区切り（`:`）を入れない | 公式文書 https://www.rfc-editor.org/rfc/rfc6750#section-2.1 | 2 |
| P4 | いまの資格情報の判定は `Bearer` の合言葉 1 本（`API_TOKEN`）と画面のログインの印の 2 種類。読み出しの記録の資格情報の欄は `web_session` / `api_token` / `none` の 3 値 | 衛星の資格情報を足す場所（`authorize_quiet`）・読み出しの記録の欄（C3） | コードを読む | `crates/server/src/lib.rs:459-475`（`authorize_quiet`）・`web_session.rs:40-52`（`Caller`）・`access_log.rs:30-38` | 実物（repo のコード） | 2 |
| P5 | 走っている change（ST09 の `PUT /media/originals`・ST13 の `GET /sources`）は、どちらも既存の `authorize` を通る口を足す | 「衛星の資格情報で 2xx を返す口が 0」を route ごとでなく router の全 route で試す（申し送り C2 / C4） | 各 change の design を読む | `st09-media-capture/design.md:168`（資格情報は `/ingest` と同じ）・`st13-scheduled-account-fetch/design.md:162`（ほかの読み出しと同じ認証） | 実物（repo の文書） | 2 |
| P6 | AI の MCP クライアントは、手元の網の中の HTTP の MCP（A-01）に固定の `Authorization: Bearer` を付けられる | 最初の AI 2 つの資格情報の使い道（PERM-14）。ST29 の決定は値の発行までで、渡し方には依らない | 確かめていない（ST27 の上流で、使う MCP クライアントの公式文書と実物で確かめる） | 未確認。付けられなくても、ST29 の資格情報の上に別の渡し方（OAuth の層など）を後から足すだけで、ST29 の決定は失われない | 知識 | 1 |
| P7 | 本体（`ashiato-server`）の通常の依存に、衛星のコードを読み込む部品（`libloading` `wasmtime` `wasmer` `mlua` `rhai` `pyo3` `deno_core` `abi_stable` `extism` など）は無い。`Cargo.lock` の `libloading` は `clang-sys` 経由のビルドの道具で、本体の実行時には入らない | FR-77 の検査（C11）を `Cargo.lock` の字面でなく、本体の通常の依存の木で見る形にする | `cargo tree -p ashiato-server -e normal --target all --prefix none` から名前を抜いて突き合わせ、`cargo tree -i libloading --target all -e all` で引く元を見る（spec の段で走らせた。2026-10-21） | 該当 0。`libloading` を引くのは `clang-sys` だけ | 実物（repo の依存の木） | 4 |
