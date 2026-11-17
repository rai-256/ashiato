<!-- 処置の判定: re-review -->

## /code-review（83cb773..daa2110）

見た: Claude Code の組み込みの `/code-review`（6 件）。写しは scripts/code_review_record.py

## R1. The first box run of 2.1 only passed after `npm ci` was run by hand on the host, because the box needs the host's gitignored `web/node_modules`. This change does not fix that, so turning on HARNESS_WORKBOX=tests after merge will still make e2e fail in fresh worktrees.
- 成果物: tools/workbox.json:23
- 根拠: Evidence 2.1, first run (11:04): `sh: 1: playwright: not found`, rc=127, BLOCKED_INFRA. task-2-report.md says the cause was that this worktree had no `web/node_modules`, and that `cd web && npm ci` was run before the second run. Each new Story/CH worktree that the graph creates without `npm ci` will hit the same rc=127 in the box once the workbox is enabled. CH18's stated goal is that nothing breaks on the day the box is turned on, and that goal is not met.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R2. The fix sits at the wrong level. The `-llzma`/`-lbz2` link failure comes from `zip = "2"` with default features, which pull in the unused bzip2-sys and lzma-sys (via xz2). The change adds system -dev packages to the box instead of dropping those features.
- 成果物: crates/server/Cargo.toml:32
- 根拠: `cargo tree -i lzma-sys` / `-i bzip2-sys` resolve only through zip → ashiato-server. The server only reads and writes deflate/stored archives. With `zip = { version = "2", default-features = false, features = ["deflate"] }` the two C deps disappear from every environment: the box, CI, and the Windows release build. Under this change, every new environment needs the same apt patch, or it hits the same link error.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R3. No evidence shows the box can now link from a cold build. 2.1 ran against a target/ shared with the host and already warmed by it (tasks.md says itself that 2.1 can pass without the packages). 1.3 only checks that the packages are installed.
- 成果物: openspec/changes/ch18-workbox-libs-cpus/tasks.md:29
- 根拠: The original failure comes from lzma-sys/bzip2-sys build-script output (pkg-config → dynamic `-llzma`) cached in the shared target/. If the packages lack what the build script needs in the box (for example pkg-config metadata), or the cache still holds host build results, the link error can come back on a clean worktree. The current evidence (dpkg shows the packages installed, and 2.1 passed on a warm target/) would not catch that. Running `cargo clean -p lzma-sys -p bzip2-sys` (or using a separate CARGO_TARGET_DIR) before building in the box was never done.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R4. `cpus` applies to agent boxes as well as verify boxes (workbox.py run_args adds `--cpus` without checking `agent`). The 'two boxes × 6 = 12, the rest stays with the host' budget only holds while HARNESS_WORKBOX=tests.
- 成果物: tools/workbox.json:23
- 根拠: With HARNESS_WORKBOX=all, each agent session also gets a 6-CPU box, and nothing limits how many agent boxes run at once, unlike parallel_verify. Two verify boxes plus two or more agent boxes is 24 or more CPUs on a 16-core machine, which is the oversubscription CH18 says it prevents. There is no separate `agent_cpus` the way there is an `agent_memory`.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R5. `env.CARGO_BUILD_JOBS` stays at 4 while the box quota is now 6 CPUs. The two settings disagree, and cargo builds in the box leave a third of the new quota unused.
- 成果物: tools/workbox.json:34
- 根拠: Inside the box, `cargo test --workspace` compiles with 4 jobs under a 600000/100000 quota, so about 2 CPUs sit idle through the roughly 5-minute build. The value 4 was chosen before any CPU cap existed. Now that `cpus` limits the box, CARGO_BUILD_JOBS should either be removed (cargo's available_parallelism honours cgroup cpu.max) or set to match `cpus`.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）

## R6. The 2.2 allowed-paths regex does not anchor `docs/briefs/CH18`, and it exempts everything under `.superpowers/`. The 'only planned paths touched' check is therefore looser than the completion criterion says.
- 成果物: openspec/changes/ch18-workbox-libs-cpus/tasks.md:36
- 根拠: Files such as `docs/briefs/CH180.md` or `docs/briefs/CH18x/...`, and any file under `.superpowers/` (for example a stray copy of a product file), pass `grep -vE "^(...|docs/briefs/CH18|\.superpowers/|$)"`. 2.2 then reports PASS even though paths outside the plan were touched.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
