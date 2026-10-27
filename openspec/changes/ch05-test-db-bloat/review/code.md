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
