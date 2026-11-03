# ST55 — サーバを入れて、そのまま使い始められる

## Why

サーバはいま、開発の台本と Docker（WSL の中）でしか動かない。画面の合言葉を決める口も、どの Story も持っていない。
配布の方針の改訂（2026-11-03。FR-108〜FR-111・FR-132）で、利用者に Docker Desktop も WSL も入れさせず、サーバに PostgreSQL 17 を同梱し、両方を Windows のサービスにすると決めた。
ST57（更新の知らせ）・ST58（更新と戻し）・ST56（収集アプリの繋ぎ先）・ST59（合言葉を画面から変える）は、どれも「インストールした形」のサービスの名前・置き場・秘密の持ち方を土台にする。
本人の環境を配った版へ切り替えるのも、このインストーラで入れてから行う（FR-108 の ★ 2026-11-03 補足・`docs/release.md` §7）。

## What Changes

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q1〜Q7 はどれも「仮でよい」（失うものが無い）問いで、本人の答えが推奨と違えば `spec --answers` が書き直す。
> 失うものがある決定（文字の並べ方・置き場の権限・取り込みの利用者・ログの行き先・取り込み待ちの場所）は、失わない側に倒した聞かない既定（premises の C1〜C18 = design の D1〜D18）にした。

- **インストーラ `ashiato-setup.exe` を作る**（Q2 の推奨）: Rust の小さな実行ファイルを、サーバの Windows の配布物（`ashiato-server-<版番号>-windows-x86_64.zip`）に入れる。zip を展開して開くと、
  管理者の確認（UAC）を 1 回だけ出し、プログラムを `C:\Program Files\ashiato` に、状態を `C:\ProgramData\ashiato` に置き、同梱の PostgreSQL 17 とサーバを自動で起きる 2 つの Windows のサービスにする（FR-108）。
  入れるのはサーバとデータベースだけで、PC の収集アプリは今までどおり別の zip（Q1 の推奨）。インストーラはネットワークに出ない（D12）
- **サーバに Windows のサービスとして動く口を足す**（D2）: `ashiato-server service`。開始・停止の知らせを受け、止めるときは今の取り込みを終えてから終わる。設定と秘密は環境変数（サービスの登録簿。ほかのアカウントが読める。P7）でなく、
  状態の置き場の中の設定のファイルから読む（D4）。DB が接続を受けるまで待ってから起動の検査に入る（D11）。ログは置き場の `logs` のファイルに書く（D17（仮））
- **配る DB を製造準備の既定値で作る**（FR-108 の ★ 2026-11-03 補足・FR-132）: PostgreSQL 17、OS に依らない文字の並べ方（`builtin` の `C.UTF-8`。D1）・チェックサム・合言葉の認証（`trust` を残さない）・`127.0.0.1` の 55400 番だけで待ち受け（D5）・ログの 3 つの設定（D6）。
  DB の 3 つの役割（管理者・所有者・実行時）をインストーラが同梱の `psql` で作り、インストールのときだけ移行を当てる（D7・D11）
- **状態の置き場を FR-109 の形にする**（D3（仮）・D14）: `C:\ProgramData\ashiato` は継承を切り、2 つのサービスのアカウント（合言葉を持たない仮想アカウント）だけが書け、インストールした本人だけが読める。
  DB・写真の原本・書き出し・移行の前の退避・書庫の写し・ログ・設定（秘密を含む）をここに置く。書き出しと写真の原本の既定の置き場も、インストールした形ではここにする（`data-durability`・`device-collection` の MODIFIED）
- **秘密は自動で作り、人は見ない**（FR-110・D7）: DB の 3 つの合言葉・API の合言葉・ログインの印の鍵（`WEB_SESSION_KEY`。ST44）を OS の暗号の乱数で作り、設定のファイルにだけ置く。画面にも記録にも出さない。
  走らせ直しは足りないものを足すだけで、既にある秘密・DB・設定を上書きしない（D10（仮））
- **画面の合言葉はインストールの最後に端末で決める**（FR-111・Q3 の推奨）: 2 回打たせて一致を見て、決めたら既定のブラウザで画面を開く。忘れたら `ashiato-setup.exe password` で決め直す（管理者の確認が 1 回出る。それまでのログインは全部切れる）。
  合言葉は元の値を持たず、照らすための値だけを設定に置く（D8。`data-sensitivity` の MODIFIED）
- **Takeout をダウンロードするだけで入ることを続ける**（FR-108 の ★ 2026-11-03 補足・D9・D16（仮））: インストーラは本人の「ダウンロード」にサーバのアカウントの読みを、本人のプロファイルの直下の取り込み待ち（`%USERPROFILE%\ashiato\取り込み待ち`）に読み書きを足し、
  取り込みの利用者に収集アプリと画面と同じ既定の利用者を渡す
- **消す口を作る**（Q4 の推奨）: Windows の「アプリと機能」に載せ、消すとサービスとプログラムを消し、記録の置き場は残してその場所を示す
- **Tailscale には触らない**（Q6 の推奨）: 最後に「無人で動かす（Run unattended）」の入れ方と、`tailscale serve` の転送先を 18700 に替えるコマンドを示す
- **C: の空きを見る**（Q7 の推奨）: 置き場は `C:\ProgramData\ashiato` に固定し、C: の空きが初年度の見積もり（40 GB）より少なければ知らせる（止めはしない）
- **配る形を Windows の CI で確かめる**（FR-108 の「Windows の CI の試験 1 本」・Q5 の推奨）: 配る zip から管理者の権限でインストールし、サービスの種類・置き場の権限・DB の設定・移行を当ててサーバが起きる・ログインできる・Takeout が入るを見る。
  本物の UAC と画面は確認バッチで Windows サンドボックスの中の録画で見る
- **同梱する PostgreSQL は要るものだけ**（FR-108・D12・D19）: 版と指紋を固定した EDB の Windows 向けの zip から `bin`・`lib`・`share` の要るものと許諾の文を取り、管理画面（pgAdmin）や StackBuilder を入れない。指紋が違えば組まない
- **本人の切り替えの手順を書き直す**（`docs/release.md` §7・D18（仮））: インストーラで入れ、いまの開発の `API_TOKEN` と繋ぎ先は引き継がない。収集アプリを配った版のサーバに繋ぐのは ST56 から

**BREAKING ではない**: 開発と試験の DB（`docker-compose.yml` の `postgres:17-alpine`・worktree ごとの DB）と開発の台本（`.env` と環境変数で起こす `ashiato-server`）は変えない。
インストールした形（`ashiato-server service`）だけが設定のファイルと新しい既定の置き場を使う。

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `distribution`: インストール（管理者の確認 1 回・2 つのサービス・プログラムと状態の置き場）・サービスとして動くサーバ・配る DB の作り方・状態の置き場の権限・秘密の生成と持ち方・DB の役割と移行・走らせ直し・画面の合言葉を決める口と決め直す口・取り込み待ちとダウンロード・待ち受けの番号・消す口・Tailscale の案内と空きの知らせ・同梱する PostgreSQL・ログの行き先（ADDED）
- `data-sensitivity`: ST44 が置き直した「記録を読むには画面の合言葉のログインが要り、API の合言葉では読めない」を MODIFIED（インストールした形では画面の合言葉を照らすための値だけで持ち、長さと API の合言葉との違いは決める時に確かめる。D8）。ST44 の delta（`docs/briefs/deps/ST44@24b34ce`）から写した
- `data-durability`: 「全記録を自宅 PC の決まったフォルダに書き出せる」を MODIFIED（インストールした形では既定の置き場を状態の置き場の `export` にし、権限はその置き場を受け継ぐ。申し送り C6）
- `device-collection`: 「D-02 の置き場は設定で変えられ、撮影の年/月のフォルダに撮影日時と元の名前で並ぶ」を MODIFIED（インストールした形では既定の置き場を状態の置き場の `media` にする。申し送り C6）

**capability の照合**: INDEX.md の capability 表は ST55 を `distribution`（インストール）に割り当てていた。インストーラ・サービス・置き場・秘密は `distribution` で合っている。
画面の合言葉の持ち方（D8）は `data-sensitivity`（アクセス制御）、書き出しと原本の既定の置き場は `data-durability` と `device-collection` の今の Requirement が持つので、その 3 つに ST55 を足した（INDEX.md の訂正の節）。後続の振る舞いの前倒しではない。

**requires**: ST52（配布物の形と版の出所）と ST28（DB の役割・画面のログイン）は archive 済みで正典にある。**ST44 を requires に足した**（spec_overlap の観点 7）——
インストーラは ST44 が足したログインの印の鍵（`WEB_SESSION_KEY`）も作って置き、D8 は ST44 が置き直した Requirement を書き換える。ST44 は下流の gate を通った SHA `24b34ce` の spec を読んだ（`deps.json`）。archive は待たない。

## Impact

- 新しい crate `crates/installer`（bin `ashiato-setup`。作業場の member。版は根の `Cargo.toml` の 1 つの出所）。端末に書くので、この crate だけ `print_stdout` を許す（D20）
- サーバ: `crates/server/src/main.rs`（`service` の口）・新しい `crates/server/src/service.rs`（設定のファイルを読み、サービスの制御を受け、DB を待つ）・`crates/server/src/web_session.rs`（照らすための値。D8）・
  `crates/server/src/lib.rs`（ログの行き先・起動の検査）・`crates/server/src/export.rs`・`media.rs`・`archive/config.rs`（インストールした形の既定の置き場）。新しい依存はサービスの制御（`windows-service`）・合言葉の KDF（`argon2`）・端末の伏せ字の入力（`rpassword`）の 3 つ（D2・D8・D23）。`tools/check-licenses.sh` に通す
- 配る道具: `tools/release/package.sh`（サーバの Windows の zip に `ashiato-setup.exe` と PostgreSQL の要るものを入れる）・`tools/release/check-artifacts.sh`（中身の検査）・新しい `tools/release/pgsql.sh`（版と指紋を固定して EDB の zip を取り、要るものを切り出す）・新しい `tools/release/install-check.sh`（Windows の CI のインストールの試験）
- CI: `.github/workflows/ci.yml` に Windows のインストールの job（`install-windows`）を足す。`release.yml` の Windows の組み立ても同じ `package.sh` を通る
- 文書: `docs/release.md` §7（本人の切り替え）、新しい `docs/install.md`（入れる・合言葉を決め直す・消す・Tailscale・記録を手で消す）、`docs/archive-inbox.md`（インストールした形の取り込み待ち）、`docs/production-prep.md`（配る DB の既定値の行）
- 申し送り: `docs/handoff/ST53.md`（同梱した PostgreSQL の許諾を FR-127 の一覧と NFR-26 の検査へ。C13）、`docs/handoff/ST30.md`（インストールした形のバックアップの置き場は `C:\ProgramData\ashiato\backups`）、`docs/handoff/ST54.md`（開発版の名前・置き場・待ち受けの表。D14）、`docs/handoff/ST57.md`・`ST58.md`（サービスの名前と置き場の並び。D14）
- **走っている change との重なり**: archive 以外は `st44-collector-credential-scope`（下流。PR #231）だけ。重なるのは `web_session.rs`（ログインの印の材料）・`lib.rs` の起動の検査（`WEB_SESSION_KEY` の検査）・`data-sensitivity` の Requirement 1 つ。
  ST44 は requires に入れ、ST44 の実装の上に積む（下流の入口が ST44 の gate 済みの SHA を取り込む）。**ST44 へは差し戻さない** —— 印の材料の変更（D8）は ST55 の中で行い、ST44 の D8（仮）の「印の内容は変えない」は配る形でだけ外れる（開発の形の印は変えない）
