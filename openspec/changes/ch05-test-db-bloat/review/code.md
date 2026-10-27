# CH05 コードのレビュー（ブランチ全体）

## final review（0bda269..6c8f1eb）

席: `superpowers:requesting-code-review` の `code-reviewer.md`（review package `.superpowers/sdd/tasks/review-0bda2699..6c8f1eba.diff`）。
範囲の起点は CH06 の実装を取り込んだ `0bda2699`（それより前は CH04・CH06 の commit）。
判定: **Yes**（Critical なし・Important なし・Minor 13・⚠️ の残り 1）。reviewer が手元で走らせたもの: 3 本の台本の `bash -n` OK /
`tools/check-test-db-killed.sh --self-test` rc=0（3.6 秒）/ `tools/docker-tidy.sh --self-test` rc=0（1.9 秒）/ `tools/docker-tidy.sh --dry-run` 何も挙げない /
`python3 scripts/check_scenarios.py . ch05-test-db-bloat` OK（warn のみ）/ `openspec validate ch05-test-db-bloat --strict` valid / 8.1・8.2 のコマンド rc=0 /
`stat_ashiato` の SQL を DB に読み取りで当てて数値 2 つ（`0` と `9705139`）。

Task ごとの ⚠️ の振り分け: T2①（ci.yml の自分の行）・T2②（`--verify-env-check` の中身）・T3（CI の行 `ci.yml:95`）・T4（探りの SQL と SIGKILL の同期。6.1 の 4 回 PASS と 6.2 の rc=1）・
T5（`stat_ashiato` が数値を返す）は解消。T7（CI の行の時間）は R14。

## R1. spec に無い Scenario の印がある
- 成果物: tools/check-test-db-killed.sh:258
- 根拠: `Scenario: 統計や一覧が読めない・数値でなければ、黙って通らず落ちる` は specs に無く、`check_scenarios.py` は `[warn]` を出すだけで通る
- kind: technical
- 処置: fixed 5.1 —— tools/check-test-db-killed.sh:268 の echo から `Scenario:` を外した（spec は不変）。8.1 の check_scenarios でこの台本の warn が消えた

## R2. `verify_env_check` に EXIT の trap が無く、途中で止まると名前の無い volume が残る
- 成果物: tools/docker-tidy.sh:373-446
- 根拠: 探りのコンテナを外した後に止まると、印を持たない匿名 volume が残り次の `cleanup_probes` で見つからない。T3 F1
- kind: technical
- 処置: fixed 3.2 —— tools/docker-tidy.sh:357-381 の `init_probe` が EXIT の trap（`cleanup_probes`）と INT/TERM/HUP→exit 130 を張り、`make_probe` が id・network・volume を作った直後に控える。`kill -TERM $$` を差して rc=130・探りのコンテナ・ネットワーク・匿名 volume が残らないことを確かめて戻した

## R3. 探りが消えたことの確かめが印のついたもの全体で、並走する self-test と混ざる
- 成果物: tools/docker-tidy.sh:356 / :434 / :333 / :381
- 根拠: 控えた `$c` の id でなく印全体で見ている。`cleanup_probes` はほかの木で走っている self-test の探りまで消しうる。T2 F4
- kind: technical
- 処置: fixed 3.2 —— 回ごとの印 `dev.ashiato.ch05-tidy-run=<乱数>`（tools/docker-tidy.sh:135,360）を付け、`cleanup_probes`（:391-399）はこの印と控えた volume だけを消し、消えたことは控えた id で見る（:384-386）。ほかの木の探りに見立てたコンテナを置いて self-test rc=0・そのコンテナが残ることを確かめた。開始時の掃除は外した（残りは R15）

## R4. 「1 行」と rc≥1 を固定していない
- 成果物: tools/docker-tidy.sh:412
- 根拠: `grep -qE '…rc=[0-9]+'` は行の数も rc が 0 でないことも見ない。T3 F2
- kind: technical
- 処置: fixed 3.2 —— tools/docker-tidy.sh:444-449,463 で `片付けに失敗した` が 1 行・rc≥1、`切った接続:` が 1 行であることを数える

## R5. ci.yml の自分の行の確かめが文字列一致だけ
- 成果物: tools/docker-tidy.sh:312-315 / tools/check-test-db-killed.sh:275-276
- 根拠: rust job の `- run:` 行であることまでは見ていない。コメントや別の job に書いても通る。T7 F3
- kind: technical
- 処置: fixed 7.1 —— rust job の `- run:` 行だけを awk で切り出して行全体で照合（tools/docker-tidy.sh:325 `ci_rust_runs`、tools/check-test-db-killed.sh:287）。ci.yml の行をコメントにすると両方の self-test が rc=1 を確かめて戻した

## R6. 冒頭のコメントの判定の順がコードと逆
- 成果物: tools/check-test-db-killed.sh:18-20
- 根拠: コメントは「空振り → 印」、コードは「印 → 空振り」。T5 F2
- kind: technical
- 処置: fixed 5.1 —— tools/check-test-db-killed.sh:19 のコメントを「印 → 空振り → rc」に

## R7. `probe_active` が datname で絞っていない
- 成果物: tools/check-test-db-killed.sh:75
- 根拠: 前の回のまだ眠っている backend に反応しうる（殺した回の database は印の前にできるので判定は崩れない）。T5 F3
- kind: technical
- 処置: fixed 5.1 —— tools/check-test-db-killed.sh:74-76 の `probe_active` が印の `run=` から読んだ回の database 名（`^[a-z0-9_]+$` を確かめる）で `datname` を絞る

## R8. `exe_path` が cargo の標準エラーを捨て、ビルドの失敗の理由が出ない
- 成果物: tools/check-test-db-killed.sh:87
- 根拠: コンパイルが落ちても「path が取れなかった」としか出ない。T5 F4
- kind: technical
- 処置: fixed 5.1 —— tools/check-test-db-killed.sh:87-97 で cargo の stderr を取り、失敗時に rc と末尾を出す。呼び出しは `|| return 1`（:190）。偽の cargo で rc=101 にして rc=1 と stderr が出るのを確かめた

## R9. 殺す前のきれいな回の rc を捨てている
- 成果物: tools/check-test-db-killed.sh:186
- 根拠: 落ちていても秒数だけが基準として出る。T6 F1
- kind: technical
- 処置: fixed 6.1 —— tools/check-test-db-killed.sh:193-198 できれいな回の rc を取り、0 でなければ出力を出してその rc で落ちる。cargo を rc=101 にして rc=101 で落ちるのを確かめた

## R10. `CH05_PROBE_AT` の値の確かめが database を作った後
- 成果物: crates/server/src/killed_probe_tests.rs:67
- 根拠: 値が誤っていても回の database を作ってから落ちる。T4 F1
- kind: technical
- 処置: fixed 4.1 —— crates/server/src/killed_probe_tests.rs:63-66 の `assert!` を `testdb::pool()` の前に。`CH05_PROBE_AT=bogus` で panic し新しい database が 0 を確かめた

## R11. testing.md の検査の数と並びが食い違う
- 成果物: docs/testing.md:31
- 根拠: 「検査 12 本 … / CH05 の 4 行」の数と並びが合わない。T7 F1
- kind: technical
- 処置: fixed 7.2 —— docs/testing.md:31 を「検査 19 本」にし ci.yml の並びどおりに（re-review が ci.yml:75-96 を数えて一致を確かめた）

## R12. 文書が docker の要ることを書いていない
- 成果物: docs/flow-gates.md（CH05 の追加行）/ docs/testing.md §4.4.1
- 根拠: `docker-tidy.sh --self-test` は docker の daemon と `postgres:17-alpine` を要する。隣の CH06 の行は「daemon に触らない」と書いている
- kind: technical
- 処置: fixed 7.2 —— docs/flow-gates.md:18・docs/testing.md:125 に「docker の daemon と `postgres:17-alpine` を要する」

## R13. 死んだ命令がある
- 成果物: tools/docker-tidy.sh:220
- 根拠: `echo "$gm" >"$tmp/wts"` は直後の `: >` で消える。T2 F5
- kind: technical
- 処置: fixed 2.1 —— tools/docker-tidy.sh:225 の死んだ `echo` を消した

## R14. design の CI の行の時間のうち 2 つが証跡で裏付けられない
- 成果物: openspec/changes/ch05-test-db-bloat/design.md（計測の記録）
- 根拠: `--self-test` の 2 本（3.8 / 2.0 秒）は reviewer の計測（3.6 / 1.9 秒）と合うが、`--verify-env-check` の 16.3 秒と本体の 52.9 秒はどのログにも無く、6.1 の evidence の `duration_s`（13〜32 秒）とすぐには合わない。合否には使っていない。T7 ⚠️
- kind: technical
- 処置: fixed 7.3 —— design.md の計測の記録を測り直した値と測り方（`$EPOCHREALTIME` / `time` / quiet-run の表示、ビルド込みか）に置き換え、証跡の無かった 52.9 秒を消した。af5e78d9 の evidence（3.2 は 23 秒、6.1 は 17 秒）と矛盾しない

## scoped re-review（80edad0..af5e78d）

fix round 1（`204779c8` 修正・`af5e78d9` 証跡。報告 `.superpowers/sdd/ch05-final-fix-report.md`）を `re-review-prompt.md` で見た: **R1〜R14 すべて ADDRESSED、新しい Critical/Important なし。**
fix の後の木（204779c8）で `scripts/verify-run` 2.1 / 3.2 / 5.1 / 6.1 / 8.1 / 8.2 が PASS（evidence.jsonl）、`cargo test -p ashiato-server killed_probe -- --list` 2 本、
`cargo fmt --all --check`・`cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0。re-review の Minor 2 件を R15・R16 に残す。

## R15. 開始時の探りの掃除を外したので、SIGKILL された回の探りは次の回でも消えない
- 成果物: tools/docker-tidy.sh:401 付近（`real_docker_body`）
- 根拠: R3 の直しで開始時の `cleanup_probes` を外した。EXIT の trap は SIGKILL では走らないので、そのときの探り（`ch05tidy<乱数>-st97_default` のネットワーク・匿名 volume を含む）が残り、`<main>-` で始まらないので本物の `tidy` も拾わない。手元で SIGKILL が重なるとネットワークのアドレスの枠を食う。CI は毎回新しい runner。spec の Scenario とは食い違わない（re-review が verify-environment/spec.md:35-37・65-67・116-122 を確かめた）
- kind: technical
- 処置: rejected: R3（並走するほかの木の self-test の探りを消さない）の対価として受け入れる。開始時に印全体を掃けば並走を壊し、回の印だけでは前の回を拾えない。残るのは `--self-test` / `--verify-env-check` が SIGKILL されたときだけで、印 `dev.ashiato.ch05-tidy-probe=1` で `docker ps -a --filter label=…` から見つけられる。Ruling として PR 本文に挙げる

## R16. `openssl` が無いと `--self-test` が理由を出さずに rc=1
- 成果物: tools/docker-tidy.sh（case の `--self-test) init_probe && self_test`）
- 根拠: `init_probe` の乱数に `openssl` を使い、無いと黙って rc=1。fix の前も `real_docker_body` の中で同じく `return 1` していた
- kind: technical
- 処置: rejected: fix の前も tools/docker-tidy.sh（80edad0e）の `real_docker_body` が `r=$(openssl rand -hex 4) || return 1` で同じく rc=1 で落ちており、振る舞いは変わらない。黙って通る向きではなく落ちる向き。tools/docker-tidy.sh:359 の `|| return 1` で rc=1 になる。`command -v openssl` は手元で /usr/bin/openssl

参考（指摘でない）: `ashiato` の行数の判定は CH06 の後は `core.event` が無く 0 と 0 を比べる（効く守りは大きさで design のとおり）。
`insert` の時点で殺されるのは行を書く前（回の database ごと消すので検査の意味は変わらない）。T2 F6（`*-verify` が広い）は spec の本文どおり。

## code_verify（独立検証。HEAD 8626aba1、2026-10-27）

申告: 15/15（tasks.md の `[x]` 1.1〜8.3）。検証コマンドは tasks.md から `scripts/evidence.py` の `declared()` で取り出した 40 本と、
取り出されない `openspec validate ch05-test-db-bloat --strict` の 1 本（R21）。全部をこの worktree で走らせ直した（DB を使うものは直列）。

| 項目 | 走らせたもの | 申告 | 実測 |
|---|---|---|---|
| 1.1 | verify-env（rc=0・5 秒）/ story_deps included ×2 / testdb.rs・verify-env.sh の grep ×2 | PASS | 一致（全部 rc=0） |
| 2.1 | `bash -n` / `docker-tidy.sh --self-test`（17 件 ok）/ 印の grep / design の grep | PASS | 一致 |
| 3.1 | `bash -n` / verify-env / `check-verify-env-reset.sh --self-test` / `EV quiet-run ch06 …`（44 秒） | PASS | 一致 |
| 3.2 | `EV docker-tidy.sh --verify-env-check`（17 秒）/ 印の grep / 準備の後の `--dry-run` が空 / design の grep | PASS | 一致 |
| 4.1 | `cargo build --tests` / `NL ch05_ 2` / `EV … ch05_next_run`（1 passed）/ `check-test-db-growth.sh --lint` | PASS | 一致 |
| 5.1 | `bash -n` / `--self-test` / 印の grep | PASS | 一致 |
| 6.1 | `EV quiet-run killed -- tools/check-test-db-killed.sh`（12 秒。committed の次の回 1.9 秒）/ grep ×2 | PASS | 一致 |
| 6.2 | 片付けを外した複製で rc=1・`ashiato_run_6f730f3be0ac`・`_tpl`・`_x_0cf9ec6e2cf5` が挙がる / 戻った / design の grep | PASS | 一致 |
| 7.1〜7.3 | self-test ×2 / ci.yml の grep / docs の grep / design の grep | PASS | 一致 |
| 8.1 | `check_scenarios.py`（scenarios: OK）/ MODIFIED の塊の diff / `openspec validate --strict`（valid） | PASS | 一致（diff は origin/main の正典に対しても rc=0） |
| 8.2 | 触ったファイルの許しの一覧 / lib.rs・verify-env.sh の削除行なし | PASS | 一致 |
| 8.3 | verify-env / `EV quiet-run ct -- cargo test -p ashiato-server`（148 秒。ログ `.superpowers/sdd/logs/20261027-213418-ct.log`: lib 1406 passed・2 ignored、server_startup 9 passed） | PASS | 一致 |

走らせる前と後で `git status --porcelain` と `git diff` は同じ（壊した複製はすべて `mktemp -d` の下、または戻して確かめた）。

### 手 1: 固定値の再計算
python で独立に計算した: `1 MiB = 1048576`、`5000000 + 1 MiB = 6048576`（許す境目。self-test の値と一致）・`+1 = 6048577`（落ちる値と一致）、
`generate_series(0, 20000)` は 20001 件（spec の 20,001 件と一致）。不一致なし。

### 手 2: ガードを壊す（複製で self-test / 作業ツリーで一時的に壊して戻す）
`tools/docker-tidy.sh` の守り 15 個を 1 つずつ潰した複製で `--self-test`: 14 個は rc=1、**1 個（docker-tidy.sh:95）は rc=0 で生き残った**（R17）。
`tools/check-test-db-killed.sh` の判定 11 個を潰した複製で `--self-test`: spec の Scenario が名指す 6 個（残り・行数・大きさ・空振り・rc の伝播・時点 2 つ）は全部 rc=1。
生き残った 5 個（印の名前の照合 :158・待ちの上限の rc :141・統計の数値の確かめ :69・`1 passed` の確かめ :84・`probe_active` :130）は、どれも後ろの判定が同じ入力で rc=1 にする受け止め役か、本物の経路（6.1・6.2 が担保）だけのもので、Scenario の主張は崩れない。
`tools/verify-env.sh:123` を作業ツリーで壊して `--verify-env-check`: `reset` の条件を外す → rc=1（`NG: 印の無い準備と --plan は片付けを呼ばない`）/ `|| echo …` を外す → rc=1（3 件 NG）。
**片付けの行を `reset_db`・`clear_blocking_network` の後ろへ動かす → `--verify-env-check` rc=0・`check-verify-env-reset.sh` rc=0**（R18）。どれも戻して `git diff --quiet -- tools/verify-env.sh` を確かめた。

### 手 3: Scenario と test
`python3 scripts/check_scenarios.py . ch05-test-db-bloat` → `scenarios: OK`（CH05 の印の warn は無い）。印の先を 1 件ずつ見て、階層が違うのは 2 件（R17・R19）。

### 手 4: 本人の決定がテストで固定されているか
| 決定 | 壊したもの | 結果 |
|---|---|---|
| Q6 殺す時点は 2 つ | `for at in insert committed` → `insert` だけ | self-test rc=1（固定されている） |
| Q6 量で見る（1 MiB・行数） | 許しを +5 バイト / 行数の比較を外す | どちらも self-test rc=1 |
| Q6 秒数は記録するだけ（合否に使わない） | —（偽の次の回は一瞬で終わる） | **秒数で落とす判定を足しても全部通る**。偽の次の回を遅くした回が self-test に無い。低い（いまの台本は秒数を比べていない）ので R にしない |
| Q7 volume は消さない | `rm -f` → `rm -fv` | self-test rc=1（偽の docker の引数と、本物の volume の両方で落ちる） |
| Q7 片付けるのは作業ツリーが消えたものだけ | path の有無・一覧・親・`<main>-`・main・`-verify` の確かめを 1 つずつ外す | どれも self-test rc=1 |
| Q7 ネットワークも（生きている作業ツリーのものは外さない） | docker-tidy.sh:95 を外す | **self-test rc=0**（R17） |
| Q8 作り直し・見送りは作らない | — | 該当コードなし（`test-db-rebuild.sh` は無い）。確かめるものなし |
本人の決定でない値: 待ちの上限 120 秒（self-test は 1〜5 秒で走る）と 20,001 件（探りの行数を変えても検査は落ちない）は、値を変えても全部通る。どちらも design / spec の値なので R にしない。

### 手 5: `[x]` と実体
15 項目とも、名指しのコマンドが実在し rc=0（上の表）。`cargo test --test <名前>` 型は無い。`NL ch05_ 2` は 2 本（`ch05_killed_probe`・`ch05_next_run`）。
8.1 の `openspec validate` だけは証跡に一度も載っていない（R21）。

### 手 6: 隙間
片付けが外したコンテナの名前の無い volume は、外した後にどの作業ツリーのものか分からなくなる（R20）。
手 7 の範囲ではないが、origin/main（CH07 の archive 後）と 4 ファイルで衝突する（R22）。

### 手 7: 該当なし
`python3 scripts/plan_fix.py pending openspec/changes/ch05-test-db-bloat` → `[]`（rc=0）。`correction-review.md` は作らない。

## R17. 生きている作業ツリーのネットワークを残す守り（docker-tidy.sh:95）を外しても self-test が通る
- 成果物: tools/docker-tidy.sh:95 / tools/docker-tidy.sh:242-258（self-test の「コンテナの無いネットワーク」）
- 根拠: :95 を `:` に替えた複製で `bash tools/docker-tidy.sh --self-test` → rc=0（NG なし。`ok: 前の 1 つが無く、後の 7 つが残り rc=0`）。
  Scenario「コンテナの無いネットワークは、作業ツリーが残っていなければ外す」は「コンテナが 1 つも繋がっていないが `git worktree list` にある作業ツリーの project のネットワーク」が残ることを言うが、
  self-test の該当（n2 `mainrepo-st10_default`）には同じ project の印を持つコンテナ k2（:245）があり、:102-104 の「project の印のコンテナが残る」で残っている。:95 の守りを観測していない
- kind: technical
- 提案: その場合の網を、project の印のコンテナが 1 つも無い生きている作業ツリー（setup の `mainrepo-st11` など）にして、:95 を外すと落ちることを確かめる

## R18. 片付けを DB を消す前・network の確かめの前に置くことを、どの検査も固定していない
- 成果物: tools/verify-env.sh:123 / tools/docker-tidy.sh:404-469（`--verify-env-check`）/ specs/verify-environment/spec.md「片付けはその判定より前に終わり」/ docs/changes/CH05.md の価値（「番号を継いだ木の subnet を、消えた作業ツリーの DB が塞いで検証の準備が止まることも防ぐ」）
- 根拠: 作業ツリーで片付けの行を `clear_blocking_network` の後ろへ動かし、`tools/docker-tidy.sh --verify-env-check` → rc=0（NG なし）、`tools/check-verify-env-reset.sh` → rc=0。戻して `git diff --quiet -- tools/verify-env.sh` rc=0。
  順を逆にすると、消えた作業ツリーの DB が subnet を塞ぐ木で `clear_blocking_network` が rc=1（BLOCKED_INFRA）になり、CH05 が防ぐと書いた止まり方がそのまま起きるが、それを見る Scenario も試験も無い
- kind: technical
- 提案: `--verify-env-check` に 1 件足す: 自分の network を外した状態で、この木の subnet（`docker-compose.subnet.yml`）の network に探りのコンテナ（`<main>-st97`・消えた path）を繋いで `HARNESS_VERIFY=1 tools/verify-env.sh` が rc=0 になることを見る。spec に Scenario を 1 本足す

## R19. Scenario「体積の行を入れる途中で殺した」の主張と、`insert` の時点で起きていることの階層が違う（行は 1 つも書かれていない）
- 成果物: specs/test-database/spec.md Scenario「体積の行を入れる途中で殺した回の database は、次の回の後に残らない」/ crates/server/src/killed_probe_tests.rs:21・:75 / design.md D2
- 根拠: 前の final review の「参考」が「`insert` の時点で殺されるのは行を書く前」と書いて指摘にしなかったものを、実測で確かめた。
  探りを `CH05_PROBE_AT=insert` で直接起こし、印 `inserting` の後 5 分 10 秒（`state=active`）の時点で回の database（`ashiato_run_e48e699548ab`）の `pg_relation_size('core.event')` = **0**。
  `EXPLAIN SELECT 1 FROM pg_sleep(600), generate_series(0, 20000)` は `Nested Loop` の外側が `Function Scan on pg_sleep` で、600 秒眠り終えるまで 1 行も作らない。
  実装は tasks 4.1・design D2 のとおり（「`pg_sleep(600)` を前に置いた 1 文」「文が走っている時間を作る」「見るのは、確定したかに依らず次の回の後に残らないことだけ」）で、spec の WHEN「体積の行を入れる文が走っている間」も字面では満たす。
  ずれは Scenario の名前と THEN の読み手が受け取る「体積の行を入れる途中」（書きかけの行がある状態）と、試験が観測する「行を書く前の、走っている文」の差。書きかけの未確定の行がある状態で殺す場合は一度も起きていない。
  （後片付け: 探りを止めて `ch05_next_run` を走らせ、探りの backend が 0 本・回の database はいまの回の 2 つだけに戻ったことを確かめた）
- kind: technical
- 提案: spec の Scenario の名前と WHEN を「行を書く前の、走っている INSERT の文の間に殺した」に書き直す（D2 の意図どおり。実装は変えない）。書きかけの行の場合も見たいなら、それは別の時点として足す（D2 の決定を変えるので、そのときは design に戻す）

## R20. 片付けが外したコンテナの volume が、どの作業ツリーのものだったか分からなくなる
- 成果物: tools/docker-tidy.sh:111-113（外したコンテナの 1 行）/ design.md「計測の記録」の `- 片付けたもの:`
- 根拠: DB の volume は名前の無い volume で、印は `{"com.docker.volume.anonymous":""}` だけ（`docker volume inspect` を ashiato2-st11-db-1 の volume で実行）。project も作業ツリーも持たない。
  コンテナを外すと、この volume は `docker volume ls -q --filter dangling=true` の 129 個に混ざる。片付けの 1 行はコンテナの名前と path だけで volume の名前を出さず、design の記録（2026-10-27 に外した 19 個）にも volume は無い。
  Q7 の「volume は消さない」は中身を残すが、どれが st29 などの DB だったかの結び付きは外した時点で捨てられ、戻せない
- kind: irreversible
- 提案: spec への足し（いまの spec の「1 つごとに名前と、コンテナならその作業ツリーの path」も Q7 の「volume は消さない」も volume の名前を出せとは言っていないので、spec 違反ではない）: 外す前に `docker inspect` の Mounts から volume の名前を取り、外した 1 行に足す（`--dry-run` も同じ形）

## R21. 8.1 の `openspec validate … --strict` は証跡に一度も載っていない（ハーネスがコマンドとして拾わない）
- 成果物: openspec/changes/ch05-test-db-bloat/tasks.md 8.1 / openspec/changes/ch05-test-db-bloat/evidence.jsonl / scripts/evidence.py:67（`CMD_LIKE`）
- 根拠: `evidence.py` の `declared()` で 8.1 から取れるのは `check_scenarios.py` と diff の 2 本だけ（`CMD_LIKE` が `openspec` で始まるものを拾わない）。evidence.jsonl の 8.1 は PASS 4 件ともその 2 本。
  手で走らせた `openspec validate ch05-test-db-bloat --strict` は rc=0（`is valid`）。ただし `[INFO] … Archive would refuse this delta: verify-environment: target spec does not exist` を出す —— このブランチには `openspec/specs/verify-environment/spec.md` が無い（origin/main には CH06 の archive `99c0b535` で入っている）
- kind: defer
- 提案: harness2 の `CMD_LIKE` に `openspec\s` を足す（harness2 側）。CH05 は origin/main を取り込んでから archive する（R22）

## R22. origin/main（CH07 の archive 後）と 4 ファイルで衝突する
- 成果物: tools/verify-env.sh / .github/workflows/ci.yml / docs/flow-gates.md / docs/testing.md
- 根拠: `git merge-tree --write-tree HEAD origin/main` → `CONFLICT (content)` 4 件。verify-env.sh は CH07 が同じ場所（`reset_db` の直前）に `reset_done=` を足した（`HARNESS_RESET` の宣言の印）。ci.yml は ST29 の `check-satellite-boundary.sh` 2 行と同じ末尾。どちらも両方を残せば済む形
- kind: conflict
- 提案: 片付けの行は `reset_done=` の後・`reset_db` の前に置く（片付けは `reset_done` に触らない）。解いた後に 3.1・3.2・8.2 を走らせ直す

## security review（5cc2caa..8626aba）

範囲: `--first-parent` の CH05 の分（`0bda2699..HEAD`。CH04・CH06 を取り込んだ 2 つの commit は除く）。コードの変更は `crates/server/src/killed_probe_tests.rs`（新規）・`lib.rs`（+3）・`tools/check-test-db-killed.sh`（新規）・`tools/docker-tidy.sh`（新規）・`tools/verify-env.sh`（+3）・`ci.yml`（+4）。依存（Cargo / npm / gradle）の変更は無い（`git diff --stat 0bda2699..HEAD -- '*Cargo.toml' '*Cargo.lock' '*package*.json' '*.gradle*'` が空）。

見た経路:
- `lib.rs:109-111` の `#[cfg(test)] mod killed_probe_tests` → router に足した口は無く、本番の組み立てには入らない。`cargo build -p ashiato-server --bin ashiato-server` で作った実行ファイルに `ch05_probe` / `CH05_PROBE` の文字列は 0 件（対照の `ingest` は 84 件）（4）
- 探りの SQL（`killed_probe_tests.rs:13-23`）→ 埋め込むのはコードの中の固定の句（`""` / `"pg_sleep(600),"`）だけ。外から来る値は `$1` `$2` で bind する（3）
- 環境変数 `CH05_PROBE_MARK` → 印のファイルへの追記（`killed_probe_tests.rs:26-34`）→ `#[ignore]` で、試験のバイナリの中でしか届かない（2）
- `tools/verify-env.sh:123`（`decision=reset` のときだけ）→ `tools/docker-tidy.sh` → `docker rm -f <id>` / `docker network rm <name>`。比較（`docker-tidy.sh:72-78,94-95`）は右辺を引用符で囲んでいて glob にならない。volume を消す命令は打たない（4。self-test の「打った命令は ps・inspect・network ls/inspect/rm・rm -f <id> だけ」が ok）
- 実データの DB（main の checkout の `ashiato2-db-1`。`docs/network.md` §6）→ project が `ashiato2` で `<main>-*` に当たらず、さらに `$wd == "$main"` で外される（`docker-tidy.sh:72,74`）。この機械の本物の docker で `tools/docker-tidy.sh --dry-run` を走らせると、何も出さずに rc=0（main・`-verify`・生きている作業ツリー 8 件と、別の repo（lifeLog2）のコンテナには当たらない）（4）
- `TEST_DOCKER_TIDY_DOCKER` など差し替えの環境変数 → `read -ra` で命令として走る（`docker-tidy.sh:32`）→ 環境を握った人は PATH も握っているので、新しい境界ではない（3）
- `tools/check-test-db-killed.sh` → 試験の DB への psql（`:41-76`）と、失敗のときの `mark.log` の末尾 20 行 → 中身は探りが入れた偽の行と database の名前だけで、記録の中身は出ない（3）
- 待ち受け: 新しい listen・平文の接続先・外への送信は無い（本物の docker の探りは `postgres:17-alpine sleep 600` で、ports を開けない。`docker-tidy.sh:373-375`）（2）

安全の決め手: **CH05 は外から届く入口を 1 つも足していない。Rust の変更は `#[cfg(test)]` の中だけで、片付けがすることは、作業ツリーが消えた `<main>-*` のコンテナとネットワークを（volume を消さずに）外すことだけ。** —— 確かさ 4。走らせたものは 3 つ: `tools/docker-tidy.sh --self-test` が rc=0・NG 0 件（本物の docker の 1 件で、外したコンテナの volume が残ることも ok）。本番の実行ファイルに探りの文字列が 0 件。この機械の本物の docker で `--dry-run` が何も出さずに rc=0。

指摘なし（片付けた後の volume の持ち主が分からなくなることは R20 が持っているので、重ねて書かない）

範囲外: `tools/docker-tidy.sh --verify-env-check` は、この木の試験用の DB を本物の reset で作り直す（`:457`）。CI と試験用の DB だけの話で、漏れではない

## /code-review（5cc2caa..8626aba）

見た: Claude Code の組み込みの `/code-review`（8 件）。写しは scripts/code_review_record.py

## R23. The network removal check is looser than the container one. Containers must sit in the same parent folder as main and have a working_dir that is missing on disk. A network only needs a matching project-name prefix, no live worktree basename, and no attached or same-project containers.
- 成果物: tools/docker-tidy.sh:94
- 根拠: Two cases. (1) Another clone such as /elsewhere/ashiato2-foo, outside the parent folder, has a project named ashiato2-foo and only its network is left (after `compose rm` etc.); verify-env's tidy removes that network although it would never touch the containers. (2) A directory exists but is not in `git worktree list` (st12 in the self-test); its container is kept, but its network is removed once no container is attached. The asymmetry means tidy deletes things outside its stated scope ("only worktrees that have disappeared").（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R24. `docker ps -aq --filter network=` (and the `ps -a` / `network ls` calls at :59 and :85) capture stderr with `2>&1` into the text they parse, so CLI warnings are read as container IDs or rows.
- 成果物: tools/docker-tidy.sh:96
- 根拠: If the docker CLI prints a warning such as `WARNING: Error loading config file ...` to stderr and still exits 0, each word of the warning lands in `att`. Those words are not in will_rm, so keep=1 and no network is ever removed. Nothing reports it, so the network pool exhaustion this CH is meant to fix keeps happening with no visible cause. For `ps -a`, the warning line becomes a bogus key in c_proj.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R25. `probe_active` treats a psql failure (db.sh exec failing, DB down, etc.) the same as "no matching statement", so wait_mark keeps looping until the time limit.
- 成果物: tools/check-test-db-killed.sh:74
- 根拠: At the insert point, the `inserting` mark has arrived but tools/db.sh cannot reach the DB. wait_mark waits the full 120 seconds, then fails with "探りの印（inserting）が…来なかった（探りが走らなかった・0 本の絞り込み）". The message blames the probe and hides the real cause (cannot query pg_stat_activity). This is the same "don't silently pass list/stat failures" problem list_checked/stat_checked were written to prevent.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R26. Containers whose worktree appears in `git worktree list` but whose path is gone (prunable, i.e. deleted with rm -rf and never pruned) are kept forever, so the commonest "vanished worktree" case is never cleaned up.
- 成果物: tools/docker-tidy.sh:23
- 根拠: A worktree directory removed by hand stays listed as prunable until `git worktree prune` or gc (default expiry 3 months). Every verify-env run leaves that DB container and network in place, they keep holding the docker network pool, and the CH05 Q7 problem continues.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R27. Network removal runs one `docker ps -aq --filter network=` per candidate network (N+1 docker calls) on the verify-env reset path, which runs before every verification.
- 成果物: tools/docker-tidy.sh:96
- 根拠: In the state where networks have piled up to dozens (the situation this CH is fixing), each reset launches the docker CLI dozens of times and verify preparation slows down. Instead, fetch `docker ps -a --format '{{.ID}}|{{.Networks}}'` once, build a network-to-containers table, and look it up.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R28. The awk that pulls the `- run:` lines of ci.yml's rust job is copied into three scripts: check-test-db-killed.sh:287, docker-tidy.sh:325 (`ci_rust_runs`), and the near-identical version at check-test-db-growth.sh:250.
- 成果物: tools/check-test-db-killed.sh:287
- 根拠: If the job layout in ci.yml changes (indentation, job name, a matrix), all three self-tests must be fixed separately, and missing one leaves a check that falsely passes or falsely fails. The obvious form is one shared helper in tools/ (for example `tools/lib/ci-runs.sh`) sourced by all three.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R29. The "time the next run with EPOCHREALTIME, print rc and seconds, dump output and return rc on failure" block is copied almost verbatim in killed_check (:194-198) and point (:160-164).
- 成果物: tools/check-test-db-killed.sh:194
- 根拠: A fix to the timing or failure output (e.g. R9-style rc handling) has to be made in both places, and if one is missed the clean run and the next run behave differently on failure. A single `timed_next <label>` function would cover both.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R30. `ch05_next_run` is not `#[ignore]`, so every `cargo test --workspace` runs a trivial test that only does `SELECT 1`. The script could reach the same next run with the existing `testdb::sweep()` or an existing test.
- 成果物: crates/server/src/killed_probe_tests.rs:89
- 根拠: All CI and local test runs pick up an extra test that checks nothing, and the `ch05_` prefix it occupies can collide with filters like check-test-db-growth's `ch04_`. Other options: make it ignored and run it with `--ignored --exact`, or have the script run an existing test such as CH04's testdb tests.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
