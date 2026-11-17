<!-- 処置の判定: re-review -->

## /code-review（83cb773..daa2110）

見た: Claude Code の組み込みの `/code-review`（6 件）。写しは scripts/code_review_record.py

## R1. The first box run of 2.1 only passed after `npm ci` was run by hand on the host, because the box needs the host's gitignored `web/node_modules`. This change does not fix that, so turning on HARNESS_WORKBOX=tests after merge will still make e2e fail in fresh worktrees.
- 成果物: tools/workbox.json:23
- 根拠: Evidence 2.1, first run (11:04): `sh: 1: playwright: not found`, rc=127, BLOCKED_INFRA. task-2-report.md says the cause was that this worktree had no `web/node_modules`, and that `cd web && npm ci` was run before the second run. Each new Story/CH worktree that the graph creates without `npm ci` will hit the same rc=127 in the box once the workbox is enabled. CH18's stated goal is that nothing breaks on the day the box is turned on, and that goal is not met.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 箱を有効にしたことで生じる失敗ではない。`web/node_modules` は追跡されず（`.gitignore:8`）、Story / change の worktree で `npm ci` を走らせる台本は無い（`tools/verify-prep.sh:45` は確認バッチ用で、worktree の準備では呼ばれない。`grep -rln "npm ci" .harness2/graph .harness2/scripts tools/*.sh` で worktree の準備に当たるものは 0 件）。箱は worktree をそのまま mount するので、`web/node_modules` が無い worktree ではホスト（作業箱 off）でも同じ `playwright: not found` で落ちる。有効にする前と後で e2e の前提は変わらず、`tools/workbox.json` の欄で直せるものでもない（箱は worktree の中身を作らない）。CH18 の価値（`docs/changes/CH18.md:15`）が言うのは「箱の中でサーバの試験が組み立てで落ちない」ことで、こちらは冷えた組み立てで確かめ直した（R3）

## R2. The fix sits at the wrong level. The `-llzma`/`-lbz2` link failure comes from `zip = "2"` with default features, which pull in the unused bzip2-sys and lzma-sys (via xz2). The change adds system -dev packages to the box instead of dropping those features.
- 成果物: crates/server/Cargo.toml:32
- 根拠: `cargo tree -i lzma-sys` / `-i bzip2-sys` resolve only through zip → ashiato-server. The server only reads and writes deflate/stored archives. With `zip = { version = "2", default-features = false, features = ["deflate"] }` the two C deps disappear from every environment: the box, CI, and the Windows release build. Under this change, every new environment needs the same apt patch, or it hits the same link error.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 「新しい環境はどれも同じ apt の足しが要り、無ければ同じリンクの失敗になる」は再現しない。`lzma-sys-0.1.20/build.rs:21` は pkg-config で liblzma が見つからなければ同梱のソースから静的に組み立てる（bzip2-sys も同じ。`bzip2-sys-0.1.13+1.0.8/build.rs:20-22`）。実測: base の設定（包み無し）の箱で、別の `CARGO_TARGET_DIR` で冷えた状態から `cargo build -p ashiato-server --bin ashiato-server` → rc=0、build script の出力は `rustc-link-lib=static=lzma` / `static=bz2`（`.superpowers/sdd/logs/20261117-204324-cold-base.log`）。元の失敗は、ホスト（pkg-config で liblzma が見つかり `-llzma` で動的にリンクする）と箱が共有する `target/` に、ホストの build script の出力が残っていることから来る。包みを足すと箱の pkg-config がホストと同じ答えを返し、共有した `target/` のまま揃う。CI・Windows の配布版の組み立ては `target/` を共有しないので影響を受けない。`crates/server/Cargo.toml` の zip の features を削るのは製品のコードの変更で、依頼の範囲（`docs/changes/CH18.md:57` 壊してはいけないもの、`tasks.md` Global Constraints）の外

## R3. No evidence shows the box can now link from a cold build. 2.1 ran against a target/ shared with the host and already warmed by it (tasks.md says itself that 2.1 can pass without the packages). 1.3 only checks that the packages are installed.
- 成果物: openspec/changes/ch18-workbox-libs-cpus/tasks.md:29
- 根拠: The original failure comes from lzma-sys/bzip2-sys build-script output (pkg-config → dynamic `-llzma`) cached in the shared target/. If the packages lack what the build script needs in the box (for example pkg-config metadata), or the cache still holds host build results, the link error can come back on a clean worktree. The current evidence (dpkg shows the packages installed, and 2.1 passed on a warm target/) would not catch that. Running `cargo clean -p lzma-sys -p bzip2-sys` (or using a separate CARGO_TARGET_DIR) before building in the box was never done.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 冷えた組み立てで箱がリンクできることを実測した。直した設定（`env -u HARNESS_WORKBOX_CONFIG`）の箱で、ホストと共有しない `CARGO_TARGET_DIR=target/ch18-cold-new` を消してから `cargo build -p ashiato-server --bin ashiato-server` → rc=0（1m03s）。箱の中の `cpu.max` は `600000 100000`、`pkg-config --libs liblzma bzip2` は `-llzma -lbz2`、build script の出力は `rustc-link-lib=lzma` / `rustc-link-lib=bz2`（動的。`.superpowers/sdd/logs/20261117-204211-cold-new.log`）。包みの pkg-config の情報が足りて、ホストの cache に頼らずにリンクできる。比べとして base の設定の箱でも冷えた組み立ては通る（静的な組み立てに落ちる。R2）。つまり包みが要るのは共有した `target/` の場合で、そちらは 2.1（温まった `target/` で rc=0）が確かめている

## R4. `cpus` applies to agent boxes as well as verify boxes (workbox.py run_args adds `--cpus` without checking `agent`). The 'two boxes × 6 = 12, the rest stays with the host' budget only holds while HARNESS_WORKBOX=tests.
- 成果物: tools/workbox.json:23
- 根拠: With HARNESS_WORKBOX=all, each agent session also gets a 6-CPU box, and nothing limits how many agent boxes run at once, unlike parallel_verify. Two verify boxes plus two or more agent boxes is 24 or more CPUs on a 16-core machine, which is the oversubscription CH18 says it prevents. There is no separate `agent_cpus` the way there is an `agent_memory`.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: `--cpus` は上限で、取り置きではない。`.harness2/scripts/workbox.py:312-313` は `cpus` が無ければ `--cpus` を付けないので、この change の前は agent の箱にも検証の箱にも上限が無かった。`cpus: 6` を置いても、どの箱も前より多くの CPU を取れるようにはならない（過剰な割り当ては増えない。減るだけ）。CH18 が有効にする前提は `HARNESS_WORKBOX=tests`（`docs/changes/CH18.md:15`）で、agent の箱は `all` のときだけ起きる（`workbox.py:133`）。`agent_cpus` のような欄は `workbox.KEYS`（`workbox.py:62`）に無く、足すのは harness2 の変更で、この change の範囲の外

## R5. `env.CARGO_BUILD_JOBS` stays at 4 while the box quota is now 6 CPUs. The two settings disagree, and cargo builds in the box leave a third of the new quota unused.
- 成果物: tools/workbox.json:34
- 根拠: Inside the box, `cargo test --workspace` compiles with 4 jobs under a 600000/100000 quota, so about 2 CPUs sit idle through the roughly 5-minute build. The value 4 was chosen before any CPU cap existed. Now that `cpus` limits the box, CARGO_BUILD_JOBS should either be removed (cargo's available_parallelism honours cgroup cpu.max) or set to match `cpus`.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: `CARGO_BUILD_JOBS` は依頼が変えないと決めた欄（`docs/changes/CH18.md:57`「`env` の `CARGO_BUILD_JOBS`」を壊してはいけないものに名指し）で、値 4 は CPU ではなくメモリの見積りに結び付いている（CH13 の `openspec/changes/archive/2026-11-10-ch13-workbox-config/design.md:36,105`: 箱のメモリ 6g / agent の 4g に cargo が収まるかを測る前提）。jobs を 6 に上げるか外すと、同時に走る rustc が増えてメモリの前提を外れる。「CPU の上限ができたから合わせるべき」は成り立たない。残りの 2 本は試験の実行（libtest は cgroup の上限に沿った並列で走る）・DB の容器・smoke で使われる。2.1 は今の値で rc=0

## R6. The 2.2 allowed-paths regex does not anchor `docs/briefs/CH18`, and it exempts everything under `.superpowers/`. The 'only planned paths touched' check is therefore looser than the completion criterion says.
- 成果物: openspec/changes/ch18-workbox-libs-cpus/tasks.md:36
- 根拠: Files such as `docs/briefs/CH180.md` or `docs/briefs/CH18x/...`, and any file under `.superpowers/` (for example a stray copy of a product file), pass `grep -vE "^(...|docs/briefs/CH18|\.superpowers/|$)"`. 2.2 then reports PASS even though paths outside the plan were touched.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 2.2 の正規表現は完了の判定の文言とそのまま一致する。判定 4（`docs/changes/CH18.md:67`）は「`docs/briefs/CH18*` と `.superpowers/` は数えない」で、glob の `CH18*` は `CH180.md` も含む。正規表現の `^docs/briefs/CH18` と `^\.superpowers/` は、この glob と接頭辞をそれぞれ写したもの。昇格を機械で決める `.harness2/scripts/lanes.py:137` はさらに広く、`docs/briefs/` と `.superpowers/` を丸ごと数えない。2.2 が判定より緩いことはない
