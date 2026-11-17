# CH18 settle — fix report（回 1）

成果物は変えていない（6 件とも rejected）。再現と反証に使ったもの:

- 冷えた組み立て（直した設定）: `scripts/quiet-run cold-new -- env -u HARNESS_WORKBOX_CONFIG python3 .harness2/scripts/workbox.py run . -- bash -c '… export CARGO_TARGET_DIR=target/ch18-cold-new; rm -rf $CARGO_TARGET_DIR; cargo build -p ashiato-server --bin ashiato-server …'`
  → rc=0（69s）。`cpu.max` = `600000 100000`、`pkg-config --libs liblzma bzip2` = `-llzma -lbz2`、build script の出力 `cargo:rustc-link-lib=lzma` / `cargo:rustc-link-lib=bz2`（ログ `.superpowers/sdd/logs/20261117-204211-cold-new.log`）
- 冷えた組み立て（base の設定。`git show $(git merge-base origin/main HEAD):tools/workbox.json` を `HARNESS_WORKBOX_CONFIG` で指す）: 同じコマンドを `target/ch18-cold-base` で
  → rc=0（83s）。`cpu.max` = `max 100000`、pkg-config は `Package liblzma was not found`、build script の出力 `cargo:rustc-link-lib=static=lzma` / `static=bz2`（ログ `.superpowers/sdd/logs/20261117-204324-cold-base.log`）
- 2 つの target は終わりに消した。
- 読んだ場所: `lzma-sys-0.1.20/build.rs:21`（pkg-config が駄目なら同梱を静的に）・`bzip2-sys-0.1.13+1.0.8/build.rs:20-22`・`.harness2/scripts/workbox.py:62,133,312-313`・`.harness2/scripts/lanes.py:137`・`tools/verify-prep.sh:45`・`.gitignore:8`・`docs/changes/CH18.md:15,57,67`・CH13 の `design.md:36,105`

R ごと:
- R1: 箱を有効にしたことで生じる失敗ではない（`web/node_modules` が無い worktree はホストでも同じく落ちる。worktree で npm ci する台本は無い）→ rejected
- R2: 「新しい環境ではどれもリンクに失敗する」は base の箱での冷えた組み立て（静的に組み立てて rc=0）で反証。zip の features は製品のコードで範囲の外 → rejected
- R3: 冷えた組み立てを直した設定の箱で実測し rc=0（動的なリンク）→ rejected
- R4: `--cpus` は上限。前は上限が無く、置いても過剰な割り当ては増えない。agent の箱は `all` のときだけ → rejected
- R5: `CARGO_BUILD_JOBS` は依頼が変えないと名指しした欄で、値はメモリの見積りに結び付く → rejected
- R6: 正規表現は判定 4 の glob `docs/briefs/CH18*` と `.superpowers/` を写したもので、lanes.py はさらに広い → rejected

review_triage: `python3 scripts/review_triage.py . ch18-workbox-libs-cpus --no-verdict` → `triage: OK` rc=0
