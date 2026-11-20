# CH26 — 夜間の脆弱性検査の 4 件を片付ける（chrono・libsqlite3-sys・rustix・同梱の PostgreSQL 17.12）

> 軽い路（`lane.json` の `reason: same`）。深掘りと合意は省いてある。依頼（`docs/changes/CH26.md`）の何を・なぜ・どこまでは本人の決定として扱う。
> 製品の振る舞いは変えない。依存の版・例外の表・同梱の PostgreSQL の小さい版だけを替える。

## Why

夜間の脆弱性検査（`vulns-nightly.yml`）は 2026-11-19 から毎晩赤で、立った 4 件の issue（#334〜#337）は手付かずのまま。
赤が続くと、新しい勧告が出ても赤に埋もれて気づけない。配る物に既知の脆弱性を残さない。

2026-11-21 に `tools/check-vulns.sh nightly` を走らせると、当たりは crate の 3 件（`premises.md` P1）。同梱の PostgreSQL は 17.6-1 で、17 の最新は 17.12（P10）。

| issue | 当たり | 直る版 | この change の処置 |
|---|---|---|---|
| #336 | rustix 1.1.5（`RUSTSEC-2026-0486`） | 1.1.6（P2） | `Cargo.lock` だけで 1.1.6 以上に上げる（D1） |
| #335 | libsqlite3-sys 0.30.1（`RUSTSEC-2026-0459`、6.8） | 0.39.0 = rusqlite 0.41。`rusqlite = "0.32"` の互換の範囲（0.32 の系列）には無い（P4） | 互換の範囲で上げられない。例外の表に `届かない:` の理由と 90 日以内の期限で書く（D2（仮）） |
| #334 | chrono 0.4.45（`RUSTSEC-2026-0543`。保守終了） | 無い（P8） | 上げる版が無い。例外の表に書く（D3（仮））。期限が来れば検査がまた落ちる |
| #337 | 同梱の PostgreSQL 17.6-1 | 17.12-1（EDB の zip あり。P10） | `tools/release/pgsql.lock` の版・URL・sha256 を 17.12-1 にそろえ、DLL の一覧を 17.12 の zip で調べ直す（D4）。17.6 の DB の置き場を 17.12 が使い、17.6 に戻しても使えることを走らせて確かめる（D5（仮）） |

## What Changes

- `Cargo.lock`: rustix 1.1.5 を 1.1.6 以上へ（`cargo update -p rustix@1.1.5 --precise <1.1.6 以上>`。`Cargo.toml` は変えない）。rustix 0.38.44 は当たらないので触らない
- `osv-scanner.toml`: `RUSTSEC-2026-0459`（libsqlite3-sys）と `RUSTSEC-2026-0543`（chrono）の 2 行を、`届かない: <理由>` と書いた日から 90 日以内の期限で足す。理由は勧告が実際に当たる使い方かを確かめた結果（design D2・D3）
- `tools/release/pgsql.lock`: `version=17.12-1`・その版の EDB の URL・落とした zip の sha256。`dlls=` と VC の実行時の DLL の注記は、17.12-1 の zip を `objdump -p` で調べ直した結果にそろえる（変わらなければ注記の日付と zip を書き換えるだけ）。大きい版（17）・実行ファイルの一覧は変えない
- 確かめの台本を change の中に置く（`openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh`。Scenario の印を持つ）: `Cargo.lock` で動いたのが rustix だけ・例外の表の既にある行が残る・lock の形（版と URL の一致・大きい版と実行ファイルと許諾の置き場が同じ・DLL の一覧が 17.12 の zip と一致）・17.6 で移行を当てて行を入れた DB の置き場を 17.12 が同じ中身で使い、17.6 に戻しても使え、17.12 の退避を 17.6 が戻せること（Docker の `postgres:17.6` / `postgres:17.12`）
- `docs/changes/CH26.md`: 価値・壊してはいけないもの・完了の判定と front matter の capability
- `docs/release.md` の「PostgreSQL の小さい版を上げる PR」の手順（変更前の小さい版を同梱した zip から更新と戻しを通す）は変えない。この change の PR でそれをどう通すかは本人に問うている（design D5・`deep.md` の R2。答えを待つ）

**変えないもの**: 製品のコード（`crates/**/src`・`collector-android`・`web`）・移行・API と送信の契約・`Cargo.toml` の依存の範囲・CI と workflow・検査の台本（`tools/check-*.sh`）・正典の Requirement の文言。

## Capabilities

### New Capabilities

なし。

### Modified Capabilities

なし（`## MODIFIED` を書かない。軽い路）。次の 2 つに ADDED の Requirement を足す:

- `dependency-vulnerability-scan`: 夜間の検査が挙げた 3 件の crate の勧告の処置（上げたもの・例外の表に書いたもの）で、全体の検査が通る
- `distribution`: 同梱の PostgreSQL の小さい版を 17.12 にそろえ、17.6 の DB の置き場をそのまま使え、17.6 に戻しても使える

## Impact

- 触れる予定のパス: `lane.json` のとおり（`Cargo.lock`・`osv-scanner.toml`・`tools/release/pgsql.lock`・`docs/changes/CH26.md`、この change の中）。`Cargo.toml` / `crates/*/Cargo.toml` / `tools/licenses/**` / `docs/release.md` は予定に入っているが、触らない見込み（rustix は lock だけで上がる。新しい依存は足さない）
- **走っている CH24（zip を 2 → 9 に上げる）との重なり**: CH24 も `Cargo.lock` と `osv-scanner.toml` を触る（zip の行を消す）。CH26 は rustix の 1 項目と例外の表の 2 行を足すだけで、zip の行・zip の依存には触れない。後から merge する側が rebase で合わせる（`Cargo.lock` は `cargo update -p rustix@1.1.5 --precise …` を取り直せば解ける）。CH24 が `tempfile` や `rustix` の版を動かしていれば、rebase の後に P1 の検査を取り直す
- 配る物: PC の zip・サーバの zip の中の PostgreSQL が 17.12 に替わる。サーバの zip の組み立ての使い回しの鍵（`tools/release/windows-build-key.sh`）は `pgsql.lock` を含むので、この PR の CI は Windows の組み立てを使い回さずに組み直す
- 例外の表の 2 行は 90 日で切れ、そのときまた夜間の検査が落ちる（chrono は直る版が出ないかぎり、置き換えるまで毎回。design D3）
