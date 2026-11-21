# CH26 実装タスク — 夜間の脆弱性検査の 4 件を片付ける

読む順: `proposal.md` → `design.md`（D1〜D9）→ `premises.md`（P1〜P21）→ このファイル → `docs/changes/CH26.md`（依頼の逐語・壊してはいけないもの・完了の判定）→ `tools/release/pgsql.lock` → `osv-scanner.toml` の頭の注記。

軽い路で始め、spec の段で通常の路へ昇格した（`lane-verdict.json` の `promoted`）。第 1 回の合意で本人が R2 を C に変えた（`deep.md`）ので、lock を変える PR の CI の形（Task 4）を足し、本人の PC で見る確認（前の 5.1）を削った。

## Global Constraints

- **触ってよいのは `Cargo.lock`・`osv-scanner.toml`・`tools/release/pgsql.lock`・`tools/release/pgsql-prev-lock.sh`（新しい）・`.github/workflows/ci.yml`・`docs/release.md`・`docs/changes/CH26.md`・`openspec/changes/ch26-nightly-vulns-cleanup/**` だけ**（Task 5 の 5.4 が機械で見る）。
  製品のコード（`crates/**/src`・`collector-android/**`・`web/**`）・`migrations/`・ほかの workflow・`tools/check-*.sh`・組み立ての台本（`tools/release/build.sh`・`package.sh`・`pgsql.sh`・`update-apply-install.sh`・`windows-build-key.sh`）・`openspec/specs/`・`.harness2` に触れない。
  `Cargo.toml`・`crates/*/Cargo.toml`・`tools/licenses/**` は触る理由が出たら手を止め、理由を ledger に書く。`windows-build-key.sh --check-workflow` が ci.yml の変更で落ち、ci.yml の書き方では避けられないときだけ、その検査を同じ PR で直してよい（design D9（仮）。理由を ledger に書く）
- **`.github/workflows/ci.yml` で変えるのは `package-windows`（前の lock を取る・鍵に入れる・古い側の組み立て・版の確かめ）と `chain`（台本の自己試験と形の検査）だけ**。lock を変えない PR の組み立てと試験は今のまま（design D5・D7（仮））
- **`cargo update` は crate を名指しして走らせる**（`cargo update -p rustix@1.1.5 --precise …`）。名指しせずに全体を上げない（design D1）
- **例外の表の形は今のまま**: 頭の注記の形（`id` / `ignoreUntil` / `reason`）に合わせ、既にある行を動かさない・変えない・消さない。`ignoreUntil` は書いた日から 90 日以内（`tools/check-vulns.sh` が見る）。`reason` は `届かない: ` で始める（design D2（仮）・D3（仮））
- **`pgsql.lock` の sha256 は、落とした実物から取る**（推測や spec の値を書かない）
- 本物の `gh` で issue を閉じない・立てない（#334〜#337 は merge の後に本人が閉じる。完了の判定 7）
- **長いコマンドは `scripts/quiet-run`**（`tools/check-vulns.sh` は osv.dev に問い合わせ数十秒〜数分。PostgreSQL の zip は約 370 MB。`cargo test` も）
- 確かめの台本（`openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh`）は、サブコマンドごとに `echo "== Scenario: <名前>"`（spec の名前を一字一句）を出してから確かめ、落ちたら rc≠0 で止まる。比べる元の版は `CH26_BASE`（無ければ `git merge-base origin/main HEAD`）、読む木は `CH26_ROOT`（無ければ repo の根）。
  使い捨てのものは `"$TMPDIR"` の下（無ければ `mktemp -d`）に作り、Docker の容器と volume は終わりに消す（`trap`）。`/tmp` の固定名に書かない

## Task 1: crate の勧告 3 件を片付ける（rustix を上げ、libsqlite3-sys と chrono を例外の表へ）

- [ ] 1.1 rustix を上げる（design D1）: `cargo update -p rustix@1.1.5 --precise 1.1.6`（その時点で 1.1.x の最新が 1.1.6 より新しければ、その版でよい）。
  `check-ch26.sh` を作り、サブコマンド `lock` に `specs/dependency-vulnerability-scan/spec.md` の Scenario「上げた後の Cargo.lock は、上げた crate のほかの組を動かさない」を印つきで書く（元の版と今の `Cargo.lock` を `python3` の `tomllib` で読み、`[[package]]` の `(name, version)` の組の差が「rustix 1.1.5 が消え、1.1.6 以上の 1.1.x の rustix が 1 つ足された」だけであることを見る）。
  `--self-test` に、`CH26_ROOT` の使い捨ての写しで (a) 写しの `Cargo.lock` の rustix 1.1.x を `1.1.5` に戻すと落ちる・(b) 写しの別の項目の `version` を 1 つ書き換えると落ちる、を置く。
  検証: `bash openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh lock` rc=0
  検証: `bash openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh --self-test` rc=0
- [ ] 1.2 例外の表に 2 行を足す（design D2（仮）・D3（仮））。足す前に、勧告が当たる使い方を確かめる（design D2 の理由が成り立つか）:
  (a) `grep -rn "Connection::open" crates/` の試験の外の出現を全部読み、開くのがブラウザの履歴の写し（`read.rs` の `copy_and_read` を通るもの）だけであること。
  (b) 写しの元の置き場がどこから来るか（`crates/collector-windows/src/history/locate.rs`。Firefox の `profiles.ini` の `IsRelative=0` の `Path=` は利用者の置き場の外を指しうる。`:208`）を読み、利用者の置き場の外の profile も読むことを D2 の理由の前提として確かめる（P16）。
  (a) で、ブラウザの履歴の写しのほかに本人以外が書ける場所の SQLite を開く所が見つかったら、手を止めて D2 の反転条件として A で人間に返す（(b) は D2 が既に反転条件に挙げているので、見つけただけでは止めない）。
  `osv-scanner.toml` の末尾に `RUSTSEC-2026-0459` と `RUSTSEC-2026-0543` の 2 行を、design D2・D3 の `reason` で、`ignoreUntil` は書いた日から 90 日（`date -u -d '+90 days' +%F`）で足す。
  足した後に `tools/check-vulns.sh nightly <使い捨ての置き場>` を走らせ、libsqlite3-sys が別の ID（`RUSTSEC-2026-0502` など、P1 の別名）で当たりに残るなら、その ID の行も同じ理由と期限で足す（design D2）。
  この 2（か 3）行は PR 本文の「例外の表で足した・変えた行」に載る（`tools/check-vulns.sh --base <base の commit>` の出力。グラフの PR 本文に写るよう、ledger に `Ruling:` の行で ID・理由・期限を置く。AGENTS.md の「依存の脆弱性」）。
  検証: `scripts/quiet-run vulns -- tools/check-vulns.sh` rc=0
  検証: `scripts/quiet-run vulns-nightly -- bash -c 'd=$(mktemp -d); trap "rm -rf \"$d\"" EXIT; tools/check-vulns.sh nightly "$d/f" && test -z "$(ls -A "$d/f" 2>/dev/null)"'` rc=0
  検証: `scripts/quiet-run vulns-base -- bash -c 'out=$(tools/check-vulns.sh --base "$(git merge-base origin/main HEAD)" 2>&1); printf "%s\n" "$out" | grep -qF "RUSTSEC-2026-0459" && printf "%s\n" "$out" | grep -qF "RUSTSEC-2026-0543"'` rc=0
- [ ] 1.3 `check-ch26.sh` にサブコマンド `vulns` を足し、`specs/dependency-vulnerability-scan/spec.md` の残りの 3 つの Scenario（例外の表の既にある行は、中身ごと残る・足した例外の行は、正典の理由の形を持つ・全体の検査と毎晩の検査は、片付けた勧告を出さない）を印つきで書く。
  例外の表は元の版と今を `tomllib` で読んで比べる。全体の検査と毎晩の検査は、どちらも rc が 2 でないことと、出力と材料のファイルに 3 つの ID の文字が無いことを見る（rc=1 は許す。後で別の勧告が出ても、この 3 件の確かめは崩れない）。
  `--self-test` に、写しの `osv-scanner.toml` で (a) 既にある行の `reason` を書き換えると落ちる・(b) 足した行の `reason` を `問題ない` にすると落ちる、を置く。
  検証: `scripts/quiet-run ch26-vulns -- bash openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh vulns` rc=0
  検証: `bash openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh --self-test` rc=0

## Task 2: 同梱の PostgreSQL を 17.12-1 にそろえる

- [ ] 2.1 `tools/release/pgsql.lock` を design D4 のとおりにする: `version=17.12-1`・`url=https://get.enterprisedb.com/postgresql/postgresql-17.12-1-windows-x64-binaries.zip`・`sha256` は落とした zip の値（先に `sha256sum` で取ってから書き、その後 `tools/release/pgsql.sh fetch` が通ることで照合する）。
  `check-ch26.sh` にサブコマンド `pg-dlls` を足し、Scenario「lock の DLL の一覧は、zip の実行ファイルから辿れる同梱の DLL と一致する」を印つきで書く（`pgsql.sh fetch` で落として展開し、`executables` から `x86_64-w64-mingw32-objdump -p` の `DLL Name:` を、zip の `bin` にある DLL に当たるかぎり辿る。大文字小文字を区別せずに比べる。Windows の DLL と `VCRUNTIME140*.dll`・`MSVCP140*.dll` は数えない）。
  `pg-dlls` が落ちたら `dlls=` を辿った集まりにそろえる。あわせて、VC の実行時の DLL（`VCRUNTIME140.dll`・`MSVCP140.dll`・`VCRUNTIME140_1.dll`）をどれが取り込むかを同じ手順で調べ、lock の注記を 17.12-1 の zip について調べた日と結果に書き換える（同じなら日付と対象の zip だけ）。
  `VCRUNTIME140_1.dll` を本体の実行ファイルか一覧の DLL が取り込むようになっていたら、`tools/release/build.sh` の変更が要る —— 予定のパスの外なので手を止める（軽い路から外れる。ledger に書いて昇格させる）。
  検証: `scripts/quiet-run pg-fetch -- bash -c 'd=$(mktemp -d); trap "rm -rf \"$d\"" EXIT; z=$(tools/release/pgsql.sh fetch "$d/cache") && test -f "$z" && unzip -q "$z" -d "$d/x" && tools/release/pgsql.sh slim "$d/x" "$d/slim" && test -f "$d/slim/bin/postgres.exe" && test -f "$d/slim/LICENSES/postgresql-legalnotice.html"'` rc=0
  検証: `scripts/quiet-run pg-dlls -- bash openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh pg-dlls` rc=0
  検証: `tools/release/pgsql.sh --self-test` rc=0
  検証: `tools/check-pg-major.sh` rc=0
- [ ] 2.2 `check-ch26.sh` にサブコマンド `pg-lock` を足し、`specs/distribution/spec.md` の 3 つの Scenario（lock の url は lock の version の zip を指す・小さい版を上げても、大きい版と実行ファイルと許諾の置き場は変わらない・同梱の版が最新の minor なら、遅れの検査は通る）を印つきで書く。
  遅れの検査は、`https://www.postgresql.org/versions.json` の 17 の行と同じ形で `latestMinor` を lock の小さい版にした写しを `PG_VERSIONS_URL=file://…` で、lock の版の zip の名前の空のファイルを置いた写しを `PG_EDB_URL_BASE=file://…/postgresql-` で渡し、今の lock に対して走らせて rc=0 を見る（網に出ない。`tools/check-pg-minor.sh` の自己試験と同じ渡し方）。
  `--self-test` に、写しの lock の `url` を別の版の zip にすると落ちる・`executables` から 1 つ消すと落ちる、を置く。
  検証: `bash openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh pg-lock` rc=0
  検証: `bash openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh --self-test` rc=0
  検証: `tools/check-pg-minor.sh` rc=0

## Task 3: 17.6 の DB の置き場を 17.12 が使い、17.6 に戻しても使え、17.12 の退避を 17.6 が戻せることを走らせて確かめる

- [ ] 3.1 `check-ch26.sh` にサブコマンド `minor-swap` を足し、`specs/distribution/spec.md` の「小さい版を上げる前に使っていた DB の置き場を、…」の 4 つの Scenario を印つきで書く（design D5）。前の小さい版は `postgres:17.6`、新しい小さい版は `postgres:17.12` の像:
  - 置き場はホストの `"$TMPDIR"` の下のフォルダか名前に乱数を含む Docker の volume。ポートは空いているものを選ぶ（固定の番号を使わない）。
    initdb はインストーラと同じ引数で作る（`POSTGRES_INITDB_ARGS` に `--locale-provider=builtin --builtin-locale=C.UTF-8 --encoding=UTF8 --data-checksums`。`crates/installer/src/db.rs` の `initdb_args` から写したと台本の注記に書く）
  - 移行は `cargo build -p ashiato-server` した `ashiato-server migrate` を、`tools/stack.sh` と同じ形でこの DB に向ける: 先に `tools/db-roles.sh` と同じ役割を作り（`tools/stack.sh:101` の渡し方）、`MIGRATE_BACKUP=off`（使い捨て）で走らせる
  - 移行で作られた表の 1 つに行を入れる（NOT NULL の列を埋められる単純な表を移行の SQL から選び、台本の注記に書く）
  - 「中身が同じ」は、`postgres:17.12` の像の中の `pg_dump`（平文）の出力から、版の行（`-- Dumped from` / `-- Dumped by` で始まる行）と毎回変わる鍵の行（`\restrict` / `\unrestrict` で始まる行）を除いて比べる（design D5）
  - 退避の Scenario は、17.12 で起きている DB を 17.12 の `pg_dump --format=custom`（サーバの移行の前の退避と同じ形式。`crates/server/src/migrate_backup.rs`）で取り、`postgres:17.6` の別の容器の空の DB に 17.6 の `pg_restore` で戻して比べる
  - 容器と置き場は `trap` で消す
  どこかで中身が変わる・起きない・戻せないときは、直そうとせずに止め、design D5 の反転条件として A で人間に返す。
  検証: `scripts/quiet-run ch26-swap -- bash openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh minor-swap` rc=0
## Task 4: lock を変える PR では、前の lock の zip から今の lock の zip への更新と戻しを CI の Windows で通す（本人の決定 R2 = C）

- [ ] 4.1 `tools/release/pgsql-prev-lock.sh` を作る（design D7（仮）・D8（仮））。サブコマンド:
  `decide <前の lock> <今の lock>`（中身が同じなら `same`、違い大きい版が同じなら `swap` を出して rc=0。大きい版が違えば 2 つの大きい版を出して rc≠0。`version=` が無い・`<大>.<小>-<n>` の形でない lock も rc≠0）、
  `zip-version <サーバの zip>`（zip の `THIRD-PARTY-NOTICES.txt` の `== PostgreSQL` の節の `version: ` の値を出す。節か値が無ければ rc≠0。P19）。
  `--self-test` に、`specs/distribution/spec.md` の Scenario「lock の中身が違い大きい版が同じなら、古い側を前の lock で組む」「lock の中身が同じなら、今のまま組む」「前の lock と大きい版が違えば、古い側を組まずに落ちる」「組んだ zip の同梱の版を、許諾の文から読む」を印つきで置く（使い捨ての lock と zip は `"$TMPDIR"` の下か `mktemp -d`）。
  検証: `tools/release/pgsql-prev-lock.sh --self-test` rc=0
  検証: `bash -c 'base=$(git merge-base origin/main HEAD) && d=$(mktemp -d) && trap "rm -rf \"$d\"" EXIT && git show "$base:tools/release/pgsql.lock" > "$d/prev.lock" && test "$(tools/release/pgsql-prev-lock.sh decide "$d/prev.lock" tools/release/pgsql.lock)" = swap'` rc=0
- [ ] 4.2 `.github/workflows/ci.yml` の `package-windows` を design D5・D7（仮）〜D9（仮）のとおりにし、`pgsql-prev-lock.sh` に `check-workflow [<ci.yml>]` を足す:
  - 鍵の step より前に、PR の run なら `github.event.pull_request.base.sha` を浅く fetch して `tools/release/pgsql.lock` を `"$RUNNER_TEMP"` の下へ取り出し、`decide` で `swap` / `same` を決めて出力に置く step（取れなければ warning を出して `same`。`decide` が rc≠0 なら落とす）
  - 鍵の step の `WINDOWS_BUILD_KEY_TOOLS` に `prev-pgsql=<swap なら前の lock の sha256、でなければ none>` を足す
  - 試験の zip を組む step で、`swap` のときだけ `old` を組む前に前の lock を `tools/release/pgsql.lock` に写し、組んだ後に `git checkout -- tools/release/pgsql.lock` で戻す（今の `git checkout -- Cargo.toml Cargo.lock` と同じ形。`dev-old`・`compat-old` は置き換えない）
  - 組んだ後（使い回したときも走る。`if:` に `cache-hit` を書かない）に、`zip-version` で `package-windows-test-old` の zip が「`swap` なら前の lock の版、でなければ今の lock の版」、`package-windows-test-new` の zip が今の lock の版であることを確かめ、違えば落とす。`古い側 <版> → 新しい側 <版>` を出力に出す。この step に `# Scenario: lock を変えた PR では、前の lock の版の zip から今の lock の版の zip へ更新し、戻しも通る` の印を置く（Windows の実地の確かめは `install-windows` の今の `update-apply-install.sh` の step がする）
  - `check-workflow` は上の形（`decide` の step が鍵の step より前・鍵の `prev-pgsql=`・`old` だけを置き換えて戻す・版の確かめの step が `cache-hit` に依らない）を python3 と yaml で読んで見る。`--self-test` に、Scenario「CI の定義は、古い側だけを前の lock で組み、版を確かめ、前の lock を鍵に入れる」（今の ci.yml で rc=0）と「CI の定義の形が崩れると、確かめは落ちる」（4 通りに崩した写しで rc≠0）を印つきで足す
  - `chain` の job に `./tools/release/pgsql-prev-lock.sh --self-test` と `./tools/release/pgsql-prev-lock.sh check-workflow` を足す（`windows-build-key.sh --check-workflow` の隣）
  検証: `tools/release/pgsql-prev-lock.sh check-workflow` rc=0
  検証: `tools/release/pgsql-prev-lock.sh --self-test` rc=0
  検証: `tools/release/windows-build-key.sh --check-workflow` rc=0
  検証: `scripts/quiet-run wbk -- tools/release/windows-build-key.sh --self-test` rc=0
  検証: `python3 -c 'import yaml,sys; d=yaml.safe_load(open(".github/workflows/ci.yml",encoding="utf-8")); r=[s.get("run","") for s in d["jobs"]["chain"]["steps"]]; sys.exit(0 if any("pgsql-prev-lock.sh --self-test" in x for x in r) and any("pgsql-prev-lock.sh check-workflow" in x for x in r) else 1)'` rc=0
- [ ] 4.3 `docs/release.md` の「### PostgreSQL の小さい版を上げる PR」の節を、手で通す手順から「`pgsql.lock` を変える PR では、CI の `package-windows` が更新の試験の古い側の zip を PR の base の lock で組み、`install-windows` が古い側から新しい側への更新と戻しを Windows で通す。PR では `install-windows` が緑で、`package-windows` の出力に `古い側 <前の版> → 新しい側 <今の版>` があることを見る。大きい版が違う lock では CI が落ちる」に書き換える（design D8（仮））。ほかの節は変えない。
  検証: `python3 -c 'import re,sys;t=open("docs/release.md",encoding="utf-8").read();m=re.search(r"^### PostgreSQL の小さい版を上げる PR\n(.*?)(?=^#{1,3} )",t,re.S|re.M);s=m.group(1) if m else "";sys.exit(0 if all(k in s for k in ("package-windows","install-windows","base")) and "結果を PR に残してから" not in s else 1)'` rc=0

## Task 5: 壊していないことを確かめ、文書をそろえ、触ったのが予定のパスだけであることを確かめる

- [ ] 5.1 `check_scenarios` で、この change の 19 の Scenario（dependency-vulnerability-scan 4・distribution 15）に印があることを見る（Task 1〜4 の後）。
  検証: `python3 scripts/check_scenarios.py . ch26-nightly-vulns-cleanup` rc=0

- [ ] 5.2 Rust の試験と clippy が今のまま通ることを見る（壊してはいけないもの 1。rustix 1.x は試験と組み立ての `tempfile` だけが使う。P3）。
  検証: `scripts/quiet-run rust -- cargo test --workspace` rc=0
  検証: `scripts/quiet-run clippy -- cargo clippy --workspace --all-targets -- -D warnings` rc=0
- [ ] 5.3 `docs/changes/CH26.md` の「## 完了の判定」の節に、1.2 で例外の行を書いた日と期限（`ignoreUntil`）を書き足す（「例外の行を書いた日: YYYY-MM-DD、期限: YYYY-MM-DD」の 1 行。ほかの節と依頼の逐語は変えない）。2.1 で `dlls=` を変えたなら、変えたことも同じ節に 1 行で書く。
  検証: `python3 -c 'import re,sys;t=open("docs/changes/CH26.md",encoding="utf-8").read();m=re.search(r"^## 完了の判定\n(.*?)(?=^## |\Z)",t,re.S|re.M);sys.exit(0 if m and re.search(r"例外の行を書いた日: \d{4}-\d{2}-\d{2}、期限: \d{4}-\d{2}-\d{2}",m.group(1)) and t.count("\n## 完了の判定\n")==1 else 1)'` rc=0
- [ ] 5.4 この change が触ったファイルが予定のパスの中だけであることを確かめる（作業ツリーの変更と未追跡のファイルも見る。完了の判定 5）。
  検証: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(Cargo\.lock|osv-scanner\.toml|tools/release/pgsql\.lock|tools/release/pgsql-prev-lock\.sh|\.github/workflows/ci\.yml|docs/release\.md|docs/changes/CH26\.md|openspec/changes/ch26-nightly-vulns-cleanup/|docs/briefs/CH26|\.superpowers/|$)"'` rc=0
