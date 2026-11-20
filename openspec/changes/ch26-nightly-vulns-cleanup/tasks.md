# CH26 実装タスク — 夜間の脆弱性検査の 4 件を片付ける

読む順: `proposal.md` → `design.md`（D1〜D6）→ `premises.md`（P1〜P15）→ このファイル → `docs/changes/CH26.md`（依頼の逐語・壊してはいけないもの・完了の判定）→ `tools/release/pgsql.lock` → `osv-scanner.toml` の頭の注記。

軽い路（`lane.json` の `reason: same`）。実装の後に、実際の diff が予定のパスの中にあることを機械が見る（外れると通常の路へ昇格する）。

## Global Constraints

- **触ってよいのは `Cargo.lock`・`osv-scanner.toml`・`tools/release/pgsql.lock`・`docs/changes/CH26.md`・`openspec/changes/ch26-nightly-vulns-cleanup/**` だけ**（`lane.json` の予定のパスのうち、使う見込みのもの。`Cargo.toml`・`crates/*/Cargo.toml`・`tools/licenses/**`・`docs/release.md` は予定に入っているが、触る理由が出たら手を止め、その理由を ledger に書いてから触る）。
  製品のコード（`crates/**/src`・`collector-android/**`・`web/**`）・`migrations/`・`.github/**`・`tools/*.sh`・`openspec/specs/`・`.harness2` に触れない
- **`cargo update` は crate を名指しして走らせる**（`cargo update -p rustix@1.1.5 --precise …`）。名指しせずに全体を上げない（design D1）
- **例外の表の形は今のまま**: 頭の注記の形（`id` / `ignoreUntil` / `reason`）に合わせ、ほかの行を動かさない・消さない。`ignoreUntil` は書いた日から 90 日以内（`tools/check-vulns.sh` が見る）。`reason` は `届かない: ` で始める（design D2（仮）・D3（仮））
- **`pgsql.lock` の sha256 は、落とした実物から取る**（推測や spec の値を書かない）
- 本物の `gh` で issue を閉じない・立てない（#334〜#337 を閉じるのは merge の後。PR 本文に `Closes` を書かない —— PR 本文はグラフが組む）
- **長いコマンドは `scripts/quiet-run`**（`tools/check-vulns.sh` は osv.dev に問い合わせ数十秒〜数分。PostgreSQL の zip は約 370 MB。`cargo test` も）
- 確かめの台本（`openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh`）は、サブコマンドごとに `echo "== Scenario: <名前>"`（spec の名前を一字一句）を出してから確かめ、落ちたら rc≠0 で止まる。使い捨てのものは `"$TMPDIR"` の下（無ければ `mktemp -d`）に作り、Docker の容器と volume は終わりに消す（`trap`）。`/tmp` の固定名に書かない

## Task 1: crate の勧告 3 件を片付ける（rustix を上げ、libsqlite3-sys と chrono を例外の表へ）

- [ ] 1.1 rustix を上げる（design D1）: `cargo update -p rustix@1.1.5 --precise 1.1.6`（その時点で 1.1.x の最新が 1.1.6 より新しければ、その版でよい）。`Cargo.lock` の差分が rustix の 1 項目（と、それに伴って変わる checksum）だけであることを `git diff Cargo.lock` で見る。ほかの項目が動いたら取り消して、`--precise` で同じことをやり直す。
  検証: `bash -c 'cargo tree -i rustix@1.1.5 >/dev/null 2>&1 && { echo "rustix 1.1.5 が残っている"; exit 1; } || exit 0'` rc=0
  検証: `bash -c 'base=$(git merge-base origin/main HEAD) || exit 1; git diff "$base" -- Cargo.lock | grep -E "^[-+]name = " | grep -v "\"rustix\"" | { ! grep -q .; }'` rc=0
- [ ] 1.2 例外の表に 2 行を足す（design D2（仮）・D3（仮））。足す前に、勧告が当たる使い方を確かめる: `crates/collector-windows/src/history/read.rs` で SQLite を開く所（`Connection::open*`）が、写したブラウザの履歴（と `-journal` / `-wal`）を読み取り専用で開くだけで、ほかに本人以外が書けるファイルを開く所が無いこと（`grep -rn "Connection::open" crates/` の試験の外の出現を全部読む）。外れていたら（本人以外が書ける場所の SQLite を開く所がある）手を止め、D2 の反転条件として A で人間に返す。
  `osv-scanner.toml` の末尾に `RUSTSEC-2026-0459` と `RUSTSEC-2026-0543` の 2 行を、design D2・D3 の `reason` で、`ignoreUntil` は書いた日から 90 日（`date -u -d '+90 days' +%F`）で足す。
  足した後に `tools/check-vulns.sh nightly <使い捨ての置き場>` を走らせ、libsqlite3-sys が別の ID（`RUSTSEC-2026-0502` など、P1 の別名）で当たりに残るなら、その ID の行も同じ理由と期限で足す（design D2）。
  この 2（か 3）行は PR 本文の「例外の表で足した・変えた行」に載る（`tools/check-vulns.sh --base <base の commit>` の出力。グラフの PR 本文に写るよう、ledger に `Ruling:` の行で ID・理由・期限を置く。AGENTS.md の「依存の脆弱性」）。
  検証: `scripts/quiet-run vulns -- tools/check-vulns.sh` rc=0
  検証: `bash -c 'out=$(tools/check-vulns.sh --base "$(git merge-base origin/main HEAD)" 2>&1); printf "%s\n" "$out" | grep -qF "RUSTSEC-2026-0459" && printf "%s\n" "$out" | grep -qF "RUSTSEC-2026-0543"'` rc=0
- [ ] 1.3 確かめの台本 `openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh` を作り、サブコマンド `deps` に `specs/dependency-vulnerability-scan/spec.md` の 3 つの Scenario（rustix の当たる版が Cargo.lock に無い・libsqlite3-sys と chrono の勧告は、届かない理由と 90 日以内の期限で例外の表にある・全体の検査と毎晩の検査は、この 3 件の勧告を出さない）を、`echo "== Scenario: <名前>"` の印つきで書く。
  rustix の版の比べは `Cargo.lock` の `name = "rustix"` の次の `version = ` を全部読み、1.0.0 以上 1.1.6 未満が 1 つでもあれば落とす（`sort -V` で比べる。0.38 の系列は通す）。例外の表は `python3` の `tomllib` で読む。全体の検査と毎晩の検査は、出力と材料のファイルに 3 つの ID の文字が無いことを見る（rc は見ない。後で別の勧告が出ても、この 3 件の確かめは崩れない）。
  台本を壊したら落ちることを 1 回ずつ確かめる（使い捨ての写しの `Cargo.lock` の rustix を `1.1.5` に戻す・写しの `osv-scanner.toml` から `RUSTSEC-2026-0543` の行を消す、のそれぞれを `CH26_ROOT=<写し>` で渡して rc≠0）。この壊し方は台本の `--self-test` に置く。
  検証: `scripts/quiet-run ch26-deps -- bash openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh deps` rc=0
  検証: `bash openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh --self-test` rc=0

## Task 2: 同梱の PostgreSQL を 17.12-1 にそろえる

- [ ] 2.1 `tools/release/pgsql.lock` を design D4 のとおりにする: `version=17.12-1`・`url=https://get.enterprisedb.com/postgresql/postgresql-17.12-1-windows-x64-binaries.zip`・`sha256` は落とした zip の値（`tools/release/pgsql.sh fetch <置き場>` が照合に使う形。先に `sha256sum` で取ってから書き、その後 `fetch` が通ることで照合する）。
  続けて、17.12-1 の zip を展開し、lock の注記と同じ手順（`x86_64-w64-mingw32-objdump -p` で `executables` の実行ファイル・`dlls` の DLL・`lib/*.dll` の `DLL Name:` を辿る）で、実行ファイルから辿れる同梱の DLL の集まりと、VC の実行時の DLL（`VCRUNTIME140.dll`・`MSVCP140.dll`・`VCRUNTIME140_1.dll`）を取り込む所を調べ直す。
  一覧が 17.6-1 と同じなら `dlls=` は変えず、注記の「checked 2026-11-07 on the zip with the sha256 above」を、調べた日と 17.12-1 の zip に書き換える。違えば `dlls=` をそろえ、注記の中身も調べた結果に書き換える。`VCRUNTIME140_1.dll` を本体の実行ファイルか一覧の DLL が取り込むようになっていたら、`tools/release/build.sh` の変更が要る —— 予定のパスの外なので手を止め、A で人間に返す。
  検証: `scripts/quiet-run pg-fetch -- bash -c 'd=$(mktemp -d); trap "rm -rf \"$d\"" EXIT; z=$(tools/release/pgsql.sh fetch "$d/cache") && test -f "$z" && unzip -q "$z" -d "$d/x" && tools/release/pgsql.sh slim "$d/x" "$d/slim" && test -f "$d/slim/bin/postgres.exe" && test -f "$d/slim/LICENSES/postgresql-legalnotice.html"'` rc=0
  検証: `tools/release/pgsql.sh --self-test` rc=0
  検証: `tools/check-pg-major.sh` rc=0
- [ ] 2.2 `check-ch26.sh` にサブコマンド `pg-lock` を足し、`specs/distribution/spec.md` の「同梱の PostgreSQL は 17.12-1 で、大きい版は 17 のまま」の 2 つの Scenario（lock の版と URL は 17.12-1 を指し、大きい版は 17・同梱の版の遅れの検査が通る）を印つきで書く。
  遅れの検査は、`{"major": 17, "latestMinor": 12, "relDate": "2026-11-12", …}` の写し（ほかの欄は `https://www.postgresql.org/versions.json` の 17 の行と同じ形）を使い捨ての置き場に書き、`PG_VERSIONS_URL=file://<写し>` で今の lock に対して走らせて rc=0 を見る（網に出ない）。
  `--self-test` に、写しの lock の `version` を `17.6-1` に戻すと `pg-lock` が落ちることを足す（`CH26_ROOT` の写しで）。
  検証: `bash openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh pg-lock` rc=0
  検証: `bash openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh --self-test` rc=0
  検証: `tools/check-pg-minor.sh` rc=0

## Task 3: 17.6 の DB の置き場を 17.12 が使い、17.6 に戻しても使えることを走らせて確かめる

- [ ] 3.1 `check-ch26.sh` にサブコマンド `minor-swap` を足し、`specs/distribution/spec.md` の「17.6 が使っていた DB の置き場を、17.12 がそのまま使い、17.6 に戻しても使える」の 2 つの Scenario を印つきで書く（design D5（仮））:
  使い捨ての置き場（ホストの `"$TMPDIR"` の下のフォルダか、名前に乱数を含む Docker の volume）で `postgres:17.6` を起こし（ポートは空いているものを選ぶ。固定の番号を使わない）、サーバの移行を当てる（`cargo build -p ashiato-server` した `ashiato-server migrate` を、`tools/stack.sh` と同じ環境の渡し方でこの DB に向ける）。
  移行で作られた表の 1 つに行を入れる（入れる表と列は、移行の SQL を読んで、NOT NULL の列を埋められる単純な表を選ぶ。選んだ表を台本の注記に書く）。
  正規化した `pg_dump`（spec の Requirement の本文のとおり、版の行と `\restrict` / `\unrestrict` の行を除く。pg_dump は `postgres:17.12` の像の中のもので取る）を取り、止めて同じ置き場で `postgres:17.12` を起こし、同じ形で取って比べる。もう一度 `ashiato-server migrate` を走らせて rc=0 と、移行の記録の表の行の数が変わらないことを見る。
  17.12 で同じ表に 1 行足して取り、止めて同じ置き場で `postgres:17.6` を起こし、同じ形で取って比べ、足した行があることを見る。容器と置き場は `trap` で消す。
  どこかで中身が変わる・起きないときは、直そうとせずに止め、design D5 の反転条件として A で人間に返す。
  検証: `scripts/quiet-run ch26-swap -- bash openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh minor-swap` rc=0
- [ ] 3.2 `check_scenarios` で、この change の 7 つの Scenario に印があることを見る。
  検証: `python3 scripts/check_scenarios.py . ch26-nightly-vulns-cleanup` rc=0

## Task 4: 壊していないことを確かめ、文書をそろえ、触ったのが予定のパスだけであることを確かめる

- [ ] 4.1 rustix を上げた後も、依存の木のほかの所が動いていないこと（壊してはいけないもの 1）と、Rust の試験が通ることを見る。rustix 1.x は試験と組み立ての `tempfile` だけが使う（P3）ので、試験の全体で確かめる。
  検証: `scripts/quiet-run rust -- cargo test --workspace` rc=0
  検証: `scripts/quiet-run clippy -- cargo clippy --workspace --all-targets -- -D warnings` rc=0
- [ ] 4.2 `docs/changes/CH26.md` の「完了の判定」に、1.2 で例外の行を書いた日と期限（`ignoreUntil`）を書き足す（「例外の行を書いた日: YYYY-MM-DD、期限: YYYY-MM-DD」の 1 行。ほかの節と依頼の逐語は変えない）。2.1 で `dlls=` を変えたなら、変えたことも同じ節に 1 行で書く。
  検証: `bash -c 'grep -qE "例外の行を書いた日: [0-9]{4}-[0-9]{2}-[0-9]{2}、期限: [0-9]{4}-[0-9]{2}-[0-9]{2}" docs/changes/CH26.md'` rc=0
- [ ] 4.3 この change が触ったファイルが予定のパスの中だけであることを確かめる（作業ツリーの変更と未追跡のファイルも見る。完了の判定 5）。
  検証: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(Cargo\.lock|osv-scanner\.toml|tools/release/pgsql\.lock|docs/changes/CH26\.md|openspec/changes/ch26-nightly-vulns-cleanup/|docs/briefs/CH26|\.superpowers/|$)"'` rc=0

## 人間の確認待ち

- [ ] 5.1 確認バッチで本人に聞く（2 つ）:
  (1) 本人の PC（か、本人がサーバを入れている Windows の PC）に配布版のサーバが入っているか。入っていれば、この change の後の配布物（PostgreSQL 17.12 を同梱）で更新し、更新の後に画面で今までの記録（滞在・稼働状況の格子）が見えるか。入っていなければ「入っていない」と答える（「見えなかった」と区別する。design D5 の反転条件には当てない）。
  更新が落ちて前の版に戻った・記録が見えないときは、design D5 の反転条件に当たる（CI で小さい版の違う zip の間の更新を走らせる change を立てる）。
  (2) 例外の表に書いた 2 行（libsqlite3-sys と chrono。`届かない:`）の理由で受け入れてよいか。とくに libsqlite3-sys は「開くのは本人のブラウザが書いた履歴の写しだけで、書ける者は収集アプリの記憶をもともと読める」（design D2（仮）。P7 は確かさ 1）。受け入れないなら、rusqlite 0.41 へ上げる change（Rust 1.89 以上が要る）を通常の路で立てる。
  chrono の行は 90 日後に切れ、夜間の検査がまた落ちて issue が立つ（design D3（仮））。そのとき置き換えるか延ばすかを決める。
  > 物理: realdata（本人の PC に入っている配布版と、その DB の記録は、機械から見えない）
