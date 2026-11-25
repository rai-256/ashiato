<!-- 処置の判定: re-review -->
# CH26 code-verify（HEAD 4194a5a3f・base 274aabf7b）

## 申告と実測

申告: tasks.md の `[x]` 14/14（1.1〜5.4）。検証コマンドは tasks.md の各「検証:」の行。

| 項目 | 検証コマンド（要点） | 実測 rc | 一致 |
|---|---|---|---|
| 1.1 | `check-ch26.sh lock` / `--self-test` | 0 / 0（差は rustix 1.1.5→1.1.6・url 2.5.8→2.5.9・form_urlencoded 1.2.2→1.2.3 だけ） | 一致 |
| 1.2 | `tools/check-vulns.sh` | 0 | 一致 |
| 1.2 | `check-vulns.sh nightly <置き場>` で材料なし | 0・材料 0 件（※1） | 一致 |
| 1.2 | `check-vulns.sh --base <base>` に 0459・0543 | 0・2 行とも出る（期限 2027-02-20） | 一致 |
| 1.3 | `check-ch26.sh vulns` / `--self-test` | 0 / 0 | 一致 |
| 2.1 | `pgsql.sh fetch` → unzip → `slim` → postgres.exe・legalnotice | 0（※1。slim の bin に zlib1.dll あり、libwinpthread なし） | 一致 |
| 2.1 | `check-ch26.sh pg-dlls` | 0（辿った 12 個 = lock の dlls） | 一致 |
| 2.1 | `pgsql.sh --self-test` / `check-pg-major.sh` | 0 / 0 | 一致 |
| 2.2 | `check-ch26.sh pg-lock` / `--self-test` / `check-pg-minor.sh` | 0 / 0 / 0（「17.12 は最新 minor」。網に出た本物） | 一致 |
| 3.1 | `check-ch26.sh minor-swap` | 0（4 Scenario。4831 行の一致・pg_restore rc=0） | 一致 |
| 3.2 | `--self-test` / `check_scenarios.py` | 0 / 0 | 一致 |
| 4.1 | `pgsql-prev-lock.sh --self-test` | 0（9 Scenario＋崩した写し 7 通り） | 一致 |
| 4.1 | `decide`=swap かつ `changed origin/main HEAD` が `true ` | 0（※1） | 一致 |
| 4.2 | `check-workflow` / `windows-build-key.sh --check-workflow` / `--self-test` / chain の yaml の検査 | 0 / 0 / 0 / 0 | 一致（形の検査の弱さは R1） |
| 4.3 | `docs/release.md` の小節の検査 | 0 | 一致 |
| 5.1 | `check_scenarios.py . ch26-nightly-vulns-cleanup` | 0 | 一致 |
| 5.2 | `cargo test --workspace` / `cargo clippy … -D warnings` | 0（2738 通過・0 失敗・5 無視・24 本の試験の実行物） / 0 | 一致 |
| 5.3 | CH26.md の完了の判定の検査 | 0 | 一致 |
| 5.4 | 触ったパスの検査 | 0（※1。base からの差は 7 ファイル＋この change の中だけ） | 一致 |

※1 tasks.md の形のコマンドは、`bash -c '… trap "rm -rf \"$d\"" …'` が実行環境の安全検査で止められたため、`rm -rf` の trap を外した同じ中身で走らせた（判定の式は同じ。使い捨ての置き場は `$TMPDIR` の下に残っている）。

検証コマンドの rc は 14 項目とも申告と一致した。指摘は検査の外側の 2 件（R1・R2）。

## 手 1: 固定値を独立に再計算する

手 1: 該当なし（確かめた範囲: lock の sha256・動かした 3 つの crate の checksum・本物の zip の版の読み取り・VC の実行時 DLL の注記）
- `tools/release/pgsql.lock` の `sha256`: python の `hashlib` で、EDB の URL から 17.12-1 の zip（386,263,884 バイト）を流しながら計算した → `a2cd8a77…a04c`。lock の値と一致
- `Cargo.lock` で動かした 3 つの crate の `checksum`: `static.crates.io` から落とした `.crate` を `hashlib` で計算した → rustix 1.1.6・url 2.5.9・form_urlencoded 1.2.3 とも一致
- `zip-version`: 自己試験では手で作った zip しか読まない。そこで本物の CI の成果物（run 45176338048 の `package-windows-test-old` の `ashiato-server-0.4.0-windows-x86_64.zip`）に当てた → `17.6-1`・rc=0
- lock の注記の VC の実行時 DLL: 17.12-1 の zip で `objdump -p` を引き直した。`MSVCP140.dll` を取り込むのは、一覧の中では `icuuc67.dll` だけ。`lib/*.dll` には `MSVCP140`・`VCRUNTIME140_1` を取り込むものが無い。`VCRUNTIME140_1.dll` を取り込むのは `stackbuilder.exe` と `wx*.dll`（slim が写さないもの）だけ。注記どおり

## 手 2: ガードをわざと壊す

- `check-vulns.sh`: lock と表を写して git の木にし、`CHECK_VULNS_ROOT` で渡した。表から `RUSTSEC-2026-0459` の行を消す → rc=1（0459・別名の 0502・GHSA-72w2-rjgg-q729 が出る）。rustix を 1.1.5 に戻す → rc=1（0486 が出る）。足した行の `ignoreUntil` を 2099-01-01 にする → rc=1（`期限が 90 日を越える (RUSTSEC-2026-0459)`）
- `check-ch26.sh`: 自己試験の崩し方 7 通りと、括弧の比べの 5 通りは全部落ちる（`--self-test` rc=0 の中身）
- `pgsql-prev-lock.sh check-workflow`: spec の 7 通りのほかに、ci.yml の写しを 7 通り崩した。2 通りは落ち、5 通りは rc=0 で通った。通った 5 通りのうち 4 通りは、残っている別のガードが job を落とす。lock を変えた PR を 17.12 → 17.12 で緑にするのは m1 だけ → R1

## R1. CI の形の検査は `changed` に渡す引数を見ていない。lock を変えた PR を「変えていない」と判定させる崩し方を通し、17.12 → 17.12 の緑を許す
- 成果物: `tools/release/pgsql-prev-lock.sh`（`CHECK_PY` の `s_take`）/ `.github/workflows/ci.yml`（`changes` の `f` の step の `pgsql-prev-lock.sh changed "$PR_BASE" "$PR_HEAD"`）
- 根拠: ci.yml の写しを崩して `tools/release/pgsql-prev-lock.sh check-workflow <写し>` を走らせた（rc はコマンドの直後に取った）。
  - m1 `changed "$PR_BASE" "$PR_HEAD"` → `changed "$PR_HEAD" "$PR_HEAD"`: **rc=0**。`tools/release/pgsql-prev-lock.sh changed HEAD HEAD` は `false`・rc=0 を返す。そのため、lock を変えた PR でも `pgsql=false` になり、`prevlock` も 2 か所の `check-zips` も走らない。古い側は今の lock で組まれ、install-windows は 17.12 → 17.12 で緑になる（deep.md の第 1 回の合意が塞ぐとした穴）。落とすガードは残らない
  - ほかに rc=0 で通った 4 通り（どれも、残っている別のガードが job を落とす。実害の道は確かめていない）:
    - m2 `prevlock` の `[ "$r" = swap ] || { …; exit 1; }` を外す: `decide` が `same` を返すのは改行だけが違う lock のときだけ
    - m3 組み立ての step の `check-zips` に `|| true`: save の後の `check-zips` が残る
    - m4 save の後の `check-zips` に `|| true`: 組み立ての step の `check-zips` が残る。ただし使い回した run では、版の確かめはこの 1 か所だけになる
    - m5b `decide … || { …; exit 1; }` → `|| true`: 次の行の `[ "$r" = swap ]` が落とす（大きい版 16 の lock で `r=[16 17]`・判定 rc=1 を実測）
  - 捕まった 2 通り: m5 `decide` の呼び出しを消す → rc=1。m6 `cp "$PREV_LOCK" …` の向きを逆にする → rc=1
  - 検査が見るのは step の有無・順・`if:`・部分文字列で、`changed` の引数は見ていない。design D8 と完了の判定 8 は「CI の緑は 17.6-1 → 17.12-1 で更新と戻しが通ったことを含む」と言う。この前提の支えは、この検査と本物の PR の run だけになる。本物の run はまだ 1 度も無い（`feat/ch26-nightly-vulns-cleanup` は origin に無く、`gh run list --branch` は空。#369 は上流の文書の PR で、Windows の job は SKIPPED）。spec の Scenario「CI の定義の形が 1 つでも崩れると…」が挙げる 7 通りは全部落ちるので、Scenario の字面は満たしている。ずれているのは、主張の階層（CI の緑なら版の組を含む）と検査の階層（step の形）の間
- kind: technical
- 提案: `s_take` で、`changed` の引数が `"$PR_BASE" "$PR_HEAD"` であることと、その env が `github.event.pull_request.base.sha`・`head.sha` であることを照合し、m1 を `mutate` に足す。あわせて、2 か所の `check-zips` に `|| true` / `|| :` が付いていないことも見る（m3・m4）。merge gate では、この PR の `package-windows` のログに `古い側 17.6-1 → 新しい側 17.12-1` があることを確かめる（docs/release.md の新しい小節のとおり）

## 手 3: Scenario と test を突き合わせる

`python3 scripts/check_scenarios.py . ch26-nightly-vulns-cleanup` は rc=0。この change の Scenario 38 件の印の先は、`check-ch26.sh` が 13 件、`pgsql-prev-lock.sh` が 13 件、`windows-build-key.sh` が 12 件（MODIFIED の既にある Scenario）だった。前の 2 つの台本の 26 件は、本文を読み、印の先が主張を観測しているかを 1 件ずつ見た。`windows-build-key.sh` の 12 件は本文を読んでいない。見たのは `--self-test` と `--check-workflow` の rc=0 だけ。
- 毎晩の検査の Scenario は、出力と材料の置き場の両方で 4 つの ID を探す。rc=1 を許しているが、1.2 の検証が素の rc=0 を別に見ている
- 「同梱の版が最新の minor なら、遅れの検査は通る」は、lock の版を最新とした写しを渡す条件つきの観測で、Scenario の字面どおり。本物の網に出る `check-pg-minor.sh` も rc=0
- minor-swap の 4 Scenario は、本物の `postgres:17.6`（PG_VERSION=17.6-2.pgdg13+1）と `postgres:17.12`（17.12-1.pgdg13+1）の像を使う。initdb の引数は `crates/installer/src/db.rs:12` の `initdb_args` と一致している
- 新しい Requirement の本体（Windows の install-windows が、古い側を入れてから新しい側への更新と戻しを通す）を手元で観測する試験は無い。CI の run に任せる設計（design 165 行目・完了の判定 8）で、その run はまだ無い（R1・見ていない範囲）

階層のずれは R1 のほかに見つからなかった。

## 手 4: 本人の決定が test で固定されているか

| 決定 | 書き換えたら落ちる試験 |
|---|---|
| R2 = C（lock を変える PR だけ前の lock で古い側を組む） | `check-workflow`（spec の 7 通り）。ただし R1 の m1 は落ちない |
| Q1（判定と確かめは台本に置く） | chain の yaml の検査（4.2）と、`check-workflow` が見る `decide`・`check-zips` の有無 |
| Q2（鍵に `prev-pgsql=<sha256\|none>`） | `check-workflow` の `s_key` と崩し方 `no-prev-key` |
| Q3（動いている版は見ない） | 「しない」の決定なので、固定するものが無い |
| Q-impl-1（url を 2.5.9 へ） | `check-ch26.sh --self-test` の (b2) |
| Q-impl-2（退避の戻しだけ括弧の入れ子を揃える） | 揃える側は `--self-test` の 5 通りが固定する。**Scenario 1〜3 を揃えずに比べる側は、どの試験も固定していない** → R2 |
| 例外の期限（書いた日 2026-11-22 から 90 日 = 2027-02-20） | `check-vulns.sh`（写しで 2099-01-01 にすると rc=1。手 2） |

## R2. 本人の決定 Q-impl-2 の「Scenario 1〜3（置き場の入れ替え）は揃えずに完全一致で比べる」を、どの試験も固定していない
- 成果物: `openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh:357`・`:377`
- 根拠: 作業ツリーで一時的に、`:357` と `:377` の `sw_same` を `sw_same_flat` に変えた。`scripts/quiet-run cv-h4-swap -- bash …/check-ch26.sh minor-swap` は **rc=0**、`check-ch26.sh --self-test` も **rc=0** だった。その後 `git checkout --` で戻し、sha256 と `git status --porcelain` が変える前と同じことを確かめた。今の 17.6/17.12 の置き場の入れ替えでは違いが出ないので、今は実害が無い。ただ、決定の片側（入れ替えでは表記の違いも許さない）は、書き換えても全部通る
- kind: technical
- 提案: `--self-test` に、`cmd_minor_swap` の Scenario 1〜3 の比べが `sw_same`（揃えない比べ）であることを見る 1 行を足す（たとえば台本の本文を grep する）。比べ方を引数で選ぶ関数にして、括弧の入れ子だけが違う 2 つの平文を「入れ替えの比べ」が違うと判定することを見るのでもよい

## 手 5: tasks の `[x]` と実体

手 5: 該当なし（確かめた範囲: 14 項目の検証コマンドとテストの名前は全部実在し、rc=0 で終わった（上の表）。`cargo test --test <名前>` の形はこの tasks.md に無い。挙がった台本とサブコマンド（`check-ch26.sh` の 7 つ・`pgsql-prev-lock.sh` の 6 つ）も実在する）

## 手 6: 隙間

手 6: 該当なし（確かめた範囲: PostgreSQL 17.7〜17.12 のリリースノートを移行の SQL と突き合わせた。SQLite を開く製品の経路・lock の古い版の残り・lock だけを変える PR での job の出し分けも見た）
- PostgreSQL 17.7〜17.12 のリリースノートの「Migration to Version」と、REINDEX を勧める項目を全部読み、`migrations/*.sql`（down を除く）と突き合わせた。勧告の対象は、btree_gist・ltree・pg_trgm の gist・GIN・bpchar の btree・TOAST の索引の REINDEX CONCURRENTLY・論理デコード・pgcrypto。この DB はどれも使っていない。gist は `core.event` の `tstzrange` の部分索引 1 つだけ（`migrations/202610231200_event_end_time.sql:401`）で、どの項目にも当たらない
- D2 の例外の理由の前提: 製品の経路で SQLite を開くのは、`crates/collector-windows/src/history/read.rs:121`・`:164`・`:283` の読み取り専用の 3 か所だけだった（`read.rs` の試験の module は `:292` から）。grep に出た `collect.rs:840` の `Connection::open(profile.join("History"))` は `collect.rs:817` の `#[cfg(test)]` の中で、ほかの残りは試験と `examples/`
- lock の古い版の残り: `git grep 17.6` で見つかったのは自己試験の材料だけ。旧 zip の sha256（`d378882a…`）はどこにも残っていない
- lock だけを変える PR: `tools/release/changes.sh code` は rc=0 を返し、`windows=true` の型にも当たる。`package-windows`・`install-windows` は飛ばない

## 手 7: 機械が承認した検証コマンドの訂正を、壊して確かめる

手 7: 該当なし（確かめた範囲: `python3 scripts/plan_fix.py pending openspec/changes/ch26-nightly-vulns-cleanup` が `[]` を返した）

## code-verify の結論
- 結論: 不一致（2 件。R1・R2）。検証コマンドの rc は 14/14 項目で申告と一致。ずれは検査の外側（CI の形の検査が `changed` の引数を見ていない・Q-impl-2 の片側が固定されていない）
- 確かめた範囲: tasks の全検証コマンドを再実行した（※1 の 3 つは rm の trap を外した同じ中身）。lock の sha256 と 3 つの crate の checksum を hashlib で計算し直し、本物の CI の成果物に zip-version を当て、17.12 の zip の VC の実行時 DLL を objdump で引き直した。check-vulns と check-workflow は壊した写しで走らせた。Q-impl-2 の片側は作業ツリーで壊して戻した。PostgreSQL 17.7〜17.12 のリリースノートを移行の SQL と突き合わせた
- 見ていない範囲: Windows の runner の上の CI（`changes` が書く pgsql の値・`prevlock` の浅い fetch・Git Bash の上の `unzip` と `check-zips`・`install-windows` の 17.6-1 → 17.12-1 の更新と戻し）。この branch の PR の run がまだ 1 度も無いので観測していない。ほかに、`windows-build-key.sh` の MODIFIED の 12 Scenario の本文と、merge の後の夜間の検査が緑か（完了の判定 7）も見ていない

## security review（274aabf..5c0a5ed）

見た経路:
- PR の run → `changes` の step が `pgsql-prev-lock.sh changed "$PR_BASE" "$PR_HEAD"`（引数は env 経由で引用付き。出力は `git merge-base` の SHA）→ `pgsql_from` を出力 → `package-windows` の `prevlock` が env の `PGSQL_FROM` を引用付きで `git fetch`・`git show` に渡す（`.github/workflows/ci.yml:36-56`・`:527-544`）。式の値を `run:` に直に展開する箇所は無い（3）
- `prevlock` の token は既存の鍵の step と同じ形（subshell の中の `GIT_CONFIG_*`、引数にも出力にも出ない。`set -x` 無し）。workflow は `pull_request` だけで `pull_request_target` は無く、`permissions: contents: read`（`ci.yml:1-10`）（2）
- 前の lock は分岐点（base 側の commit）から取る → `cp` で old を組む間だけ置き換え → `pgsql.sh fetch` が lock の版・url の形・sha256 を照合してから使う（`tools/release/pgsql.sh:14-45`。キャッシュの zip も照合し、合わなければ捨てて取り直す）。PR が lock を書き換えて差し込めるものは、今の lock を書き換えるのと同じ範囲で、新しく増えない（4）
- 配る zip（`package-windows-release`）は置き換えの step より前の step で今の lock のまま組む（`ci.yml:604-606`）。置き換えは `--test-build` の old だけで、main への push の run は `pgsql=false` で置き換えに入らない（3）
- 組み立ての使い回しの鍵に前の lock の指紋（無ければ `none`）が入る → 前の lock で組んだ old の zip を、lock を変えない run が引くことは無い（`ci.yml:553`・`:566`）（2）
- `tools/release/pgsql.lock` の 17.12-1 は https の EDB の url と sha256 の固定（sha256 は code-verify が hashlib で計算し直して一致）。dlls の入れ替え（libwinpthread-1 → zlib1）は objdump で引き直した結果（code-verify 手 6）（2）
- 依存: `rustix` 1.1.5 → 1.1.6・`url` 2.5.8 → 2.5.9・`form_urlencoded` 1.2.2 → 1.2.3・`windows-sys` 0.52 → 0.59（既存の版への寄せ）。どれも crates.io の既存の名前の patch 上げで、新しい crate は無い（`Cargo.lock` の差分）（2）
- 例外 `RUSTSEC-2026-0459`（libsqlite3-sys）: 製品で SQLite を開くのは `crates/collector-windows/src/history/read.rs:121`・`:164`・`:283` の読み取り専用の写しだけ（`collect.rs:840` は `#[cfg(test)]` の中）。この差分は経路を足しておらず、新しく届く入力は無い。届かない理由の前提（P7）と反転条件は design D2 にあり、仮の決定として確認バッチに出る（2）
- 例外 `RUSTSEC-2026-0543`（chrono の保守終了）: 欠陥の勧告ではなく、届く入力は変わらない（1）

安全の決め手: 「同梱する PostgreSQL の zip は、どの lock から組むときも `pgsql.sh fetch` が lock の sha256 と照合してから使い、配る zip は置き換えの前に今の lock で組む」—— 確かさ 4。`tools/release/pgsql.sh --self-test` が rc=0（「同梱する PostgreSQL の指紋が違えば組まない」「壊れた PostgreSQL のキャッシュは捨てて取り直す」が通る）。`tools/release/pgsql-prev-lock.sh --self-test` も rc=0（置き換えが old だけ・戻す・鍵に指紋、を崩した写し 7 通りが全部落ちる）
見ていない範囲: Windows の runner の上の本物の PR の run（`prevlock` の fetch と token の扱いを実地で観測していない。この branch の PR の run がまだ無い）。17.12-1 の zip の中身そのもの（EDB の出したものを信じる前提は 17.6-1 と同じ）

指摘なし
