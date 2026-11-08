# 作業箱を有効にする前の宿題（CH13）

`tools/workbox.json` は依頼の中身のまま置いた（書き換えない約束）。`HARNESS_WORKBOX` を `tests` か `all` にする前に、ここに挙げたものを
直すか測るかする。どれも既定（off）の間は効かない。直すのは別の change か harness2 側（CH13 の深掘り Q1。design D2（仮））。

出所: 上流の調べ（`premises.md`）と深掘りの独立検査（`review/deep.md` の R1〜R10）。確かめた harness2 は `feat/workbox` の `b4797fbbb`（2026-11-08）。

## 0. 先に確かめること

- [ ] harness2 の main に `feat/workbox` が入ったら、`scripts/workbox.py` の冒頭の鍵の一覧と `tools/workbox.json` の鍵（`apt` `memory` `tmp_size` `docker` `host_only` `env` `parallel_verify` `agent_memory` `prep_files`）を突き合わせる。
  `load_config` は知らない鍵を検査しないので、名前が変わっていても黙って効かない（R9）
- [ ] 依頼の完了の判定の残り: docker のある環境で `python3 .harness2/scripts/workbox.py run . -- bash -c 'pgrep -fc /'` が 10 未満を出す。
  **`HARNESS_WORKBOX` を外したシェルから走らせる** —— `tests` の下ではこのコマンド自身が箱に包まれ、箱の中には docker が無いので落ちる（R1。CH13 の Q3 で、この change の確かめから外した）

## 1. docker が要るのに渡らない（箱の中で落ちる）

箱は検証コマンドの**字面**に `docker` の欄の正規表現が当たるときだけ docker を渡す。中で呼ぶ台本は見ない。

- [ ] 画面の e2e（`cd web && npm run test:e2e`）: playwright の `webServer.command` が `bash ../tools/stack.sh up` を起こし、`stack.sh` が `docker compose up` する（`web/playwright.config.ts:49-50`・`tools/stack.sh:99`）。archive の tasks に 21 行、走っている st56 の 5.x
- [ ] `cargo test --workspace` と `-p ashiato-server --test server_startup`: `crates/server/tests/server_startup.rs:1904` の試験が `docker` を直に呼び、`:1949` が `tools/pg_dump-docker.sh` を使う（R2）。archive の tasks に `--workspace` が 39 行
- [ ] workflow の検査（`actionlint "$f" || docker run rhysd/actionlint …`）: ホストに `actionlint` が無く、docker に退く。走っている st53 の tasks など
- [ ] 中で docker を使う台本を呼ぶ台本: `tools/load-repeat.sh`（`tools/db.sh exec`）・`tools/st25-seed-check.sh`（`tools/stack.sh up`）・`tools/check-st23-derived-erasure.sh`（`tools/check-immutable.sh`）（R2）

## 2. ホストが要るのに `host_only` に当たらない

- [ ] `tools/release/install-check.sh`・`tools/release/build.sh`: `powershell.exe` を中で呼ぶ（字面には出ない）（R3）
- [ ] `tools/android-side-by-side.sh`・`tools/usage-volume.sh`: エミュレータを自分で起こす（R3）。箱の中では `/dev/kvm` と `/mnt/c` が見えず、エミュレータは検証の箱の 6g の中で起きる

## 3. 当たりすぎ

- [ ] 正規表現に錨が無く、台本の名前を grep するだけの検証にも docker の socket が渡る（tasks の検証コマンドで 9 件。例 `! grep -n 'gradle.properties' tools/record-env.sh`）（R4）。
  socket を渡した箱からは `docker run -v $HOME:…` でホームが見え、「`.aws` `.config/gh` は見えない」が成り立たない。直し方の候補: 呼び出しの位置に錨を置く（`(^|[;&|( ]|bash )(\./)?tools/…`）か、harness2 側で実行する台本で判定する

## 4. `prep_files` の不足（`all` のときだけ効く）

- [ ] `tools/db-roles.sql`（`tools/db-roles.sh:45` が流す）と `docker-compose.override.yml`（`tools/env-init.sh:24` が木にあれば使う）が無い（R5）

## 5. 測っていない

- [ ] 画面の e2e 1 本が検証の箱の 6g に収まるか（release のビルド + Chromium。`web/playwright.config.ts:57` の待ちは 10 分。その間、席 1 つを握る）（R6）
- [ ] `all` で 5 本のときの空きメモリ（agent 4g × 5 + 検証 6g × 2 = 32g。WSL の VM と同じ大きさ。ホストの langgraph・DB の容器はこの外。箱の `/tmp` の tmpfs も箱のメモリに数えられる）（R6）
- [ ] agent の箱の中で agent 自身が走らせる cargo のビルドが 4g に収まるか（R6）

手元で確かめ済み（依頼の「確かめたこと」。`tests`）: 箱の中で cargo・node・JDK・psql・claude・codex が動く・`cargo test -p ashiato-server --lib access_log_tests` と `tools/check-migrations.sh`・`tools/check-pg-major.sh` が PASS・worktree 5 本の cargo test が席 2 つで順に流れて空きメモリの最低 12GB。

## 6. 後の Story・運用

- [ ] 後の Story が新しく書く検証コマンドで、1 の漏れが同じ形で起きる（中で docker を使う台本を字面に出さない）。規約か検査を置くかは、箱を有効にするときに決める（CH13 の Q2。design D5（仮））
- [ ] 作業箱のイメージが `apt` を変えるたびに 1GB 級で溜まり、古いタグを消す処理が無い（harness2 側。R10）
