# CH20 の前提（外の形式に依る決定）

確かめた日: 2026-11-19（questions の段）。確かさは skill の段（1 言っただけ … 5 実物で再現した）。

| # | 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|---|
| P1 | 配布元 `rai-256/ashiato` は公開の repo で、正式版・ベータ版の道 `/repos/rai-256/ashiato/releases?per_page=20` は、認証なしで `200`・本文 `[]` を返す | 「非公開の repo で 404」という原因の見立てが成り立つか（成り立たない）。正式版の部品は正典の「空の一覧は確かめた・新しい版なし」に当たる | 製品と同じ見出し（`User-Agent: ashiato-update-check`・`Accept: application/vnd.github+json`・`Accept-Encoding: gzip`）で `curl`。`gh repo view --json visibility` | `PUBLIC`。`HTTP/2 200`・`content-length: 2`・本文 `5b5d`（`[]`）。gzip で圧縮されずに返る | 実物 | 5 |
| P2 | 開発版の道 `/repos/rai-256/ashiato/releases/tags/nightly` は、いま `404` を返す | 開発版（`-dev.`）の部品の毎日の確認。更新係（`crates/updater/src/lib.rs:173`）も PC の収集アプリ（`crates/collector-windows/src/update_check.rs:482`）も 404 を「配布元が見つからない」（`NotFound`）として失敗に記録する | 同じ `curl` | `404` | 実物 | 5 |
| P3 | 印 `nightly` のリリースが無いのは、開発版の公開の設定が入っていないから（予定の実行は組まずに終わる） | P2 が当面続くか | `gh run list -w nightly`（3 回とも 12〜19 秒で success = 組まずに終わる）。正典 distribution「本人が開発版の公開の設定を入れていない…何も公開せず、印 nightly も動かさない」 | 続く（設定を入れるまで） | 実物（run の一覧）・正典 | 4 |
| P4 | 正典は、開発版の部品が印 `nightly` に 404 を受けたら「確かめられない（配布元が見つからない）」と記録し、7 日続けば「更新を 7 日確かめられていない」と知らせる、と決めている。今の製品はそのとおり | 検査が開発版の道で何を「決めた振る舞い」とするか。依頼の「エラーを記録し続けない」と正典が食い違う | `openspec/specs/distribution/spec.md` の「確かめた結果は…追記で残す」「7 日確かめられなければ…」を読む（review/deep.md R2 の指摘で訂正。初めは「決めていない」と誤って書いた） | 決めている | 正典 `openspec/specs/distribution/spec.md:2897`・`:3236` | 2 |
| P5 | 本人が見たエラーの原因は、まだ分からない | 検査が原因の種類を捕まえるか（範囲） | 本人の Windows の記録を読む: `C:\ProgramData\ashiato-updater\`（`settings/check.json` は `enabled: false`・`since 2026-11-18T22:58:04Z`。`status.json` も `checks.jsonl` も `logs/updater.log` も無い）。PC の収集アプリの `%LOCALAPPDATA%\ashiato\collector\update-check.json`（`checked_at: null`・`intro_shown_at 2026-11-18T22:58:25Z`）。サーバの今日のログに更新の行なし | **確認の記録は 1 行も残っていない**（設定の `enabled: false` は `update-check` の口でしか書かれず、インストールは `true` を書く。よって 07:58 に本人がその口で切った。更新係も PC の収集アプリも、一度も確かめを終えていない）（更新係は確かめた回を必ず `checks.jsonl` に書く）。本人が見た「エラー」が配布元の答えから来たものか、更新係が回っていない・状態を読めない（画面の `not_running` / `unreadable`）から来たものかを、記録から決められない | 実物 | 1（原因は未確定） |
| P6 | 手元の 11-09 の配布物（`ashiato-server-0.1.0-windows-x86_64.zip`）は、入っている版ではない | 開発版の道（P2）が本人のエラーの原因か | 配布物の日付と、更新係のコードの最初の commit（11-12）を比べる（review/deep.md R13） | 11-09 の zip には更新係が入っていない。入っている版は分からない | 実物・git | 3 |
| P7 | 試験の組み立ての更新係は、差し替えの宛先に `https://api.github.com` を渡すと本物の道（`RELEASES_URL`）へ問い合わせ、版を `ASHIATO_UPDATER_VERSION` で変えられる | 本物の配布元に試験の組み立てで当てる方式が成り立つか | `crates/updater/src/lib.rs:76-118`・`:149` を辿る | 成り立つ（`base == GITHUB_API` のとき本物の道を組む）。PC の収集アプリは版を組み立ての定数（`env!("ASHIATO_VERSION")`）からしか取らない（`update_check.rs:328`） | コード | 3 |
| P8 | 認証の無い問い合わせの上限は IP ごとに 1 時間 60 回で、GitHub の runner の IP では他の利用者と分け合う | 1 回の実行の問い合わせ数・届かない日の扱い | 正典 distribution（premises P2 の引用）・手元で `x-ratelimit-remaining: 54` を観測 | 上限は 60。runner の IP で残りがいくつかは実行のたびに違う | 公式文書（正典経由）・実物（手元の IP） | 2 |

## 問いへの効き方

- P1・P5・P6 から、本人のエラーの原因が「配布元の答え」だとは言えない。だから第 1 回で、本人が何を見たかを問う（Q1）。原因は直しの change が探す。
  この change は、原因に依らず「本物と話したときの振る舞い」を機械で見る検査を作る
- P2・P4 から、開発版の道は**今日すでに**毎日の失敗を記録する組み合わせで、それは正典どおり。依頼との食い違いの扱いを問う（Q7）
- P9〜P11 は、本物と Rust の 2 部品のすでに見つかった食い違い。この change で直すかを問う（Q9）。依頼は正典の「自動の試験は本物に問い合わせない」とぶつかる（Q10）
- P7 から、状態（同じ版・新しい版がある）は試験の組み立ての版の上書きで作れるが、PC の収集アプリは版を上書きできない（Q3・Q4）

| P9 | 印 `nightly` の道の答えはリリース 1 件のオブジェクトで、Rust の `parse_releases` は配列しか読まない | 開発版の公開を入れた日からの開発版の確認・Q5 の形の比べ | 別の席が小さな crate で `parse_releases` にオブジェクトを通した（review/deep.md R3） | `Err(InvalidJson("invalid type: map, expected a sequence"))`。試験の受け手と共有の材料は配列を返すので、試験は通る | 走らせた | 4 |
| P10 | Rust の 2 部品は本文を 64 KiB までしか読まない（正典に無い決まり）。配布物 4 つのリリースは 8 件ほどで超える | 正式版の確認の将来・Q5 | 公開の repo のリリース 3 件（157,897 バイト）を通した（review/deep.md R4） | `Err(TooLarge)` | 走らせた | 4 |
| P11 | PC の収集アプリは `5xx` と別の宛先への転送を「配布元に届かない」と記録する（正典は「読めない答え」） | Q8 | `crates/collector-windows/src/update_check.rs:479-485`・`:394-397`（review/deep.md R5） | 食い違う | コード | 2 |
| P12 | 配る形の更新係のサービスは、起きた直後に 1 回確かめ、その後は 1 時間ごとに起きるが、前の確かめ（成否に依らない）から 24 時間たつまで問い合わせない | 毎晩の検査の 180 秒の待ち（D3）と、部品の問い合わせが 1 回の実行で 3 回まで（MODIFIED）が成り立つか | `crates/updater/src/lib.rs:280-293`（`next_check = Instant::now()` で始め、`once` の後 3600 秒）・`:226`（`due` が偽なら問い合わせない）・`:252-253`（失敗でも `checked_at` を書く）・`crates/update-check/src/lib.rs:255-258`（`due` は 24 時間） | 成り立つ（起動直後に 1 回。失敗しても同じ実行の中ではやり直さない） | コード | 3 |
