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
- 処置:

## R2. `verify_env_check` に EXIT の trap が無く、途中で止まると名前の無い volume が残る
- 成果物: tools/docker-tidy.sh:373-446
- 根拠: 探りのコンテナを外した後に止まると、印を持たない匿名 volume が残り次の `cleanup_probes` で見つからない。T3 F1
- kind: technical
- 処置:

## R3. 探りが消えたことの確かめが印のついたもの全体で、並走する self-test と混ざる
- 成果物: tools/docker-tidy.sh:356 / :434 / :333 / :381
- 根拠: 控えた `$c` の id でなく印全体で見ている。`cleanup_probes` はほかの木で走っている self-test の探りまで消しうる。T2 F4
- kind: technical
- 処置:

## R4. 「1 行」と rc≥1 を固定していない
- 成果物: tools/docker-tidy.sh:412
- 根拠: `grep -qE '…rc=[0-9]+'` は行の数も rc が 0 でないことも見ない。T3 F2
- kind: technical
- 処置:

## R5. ci.yml の自分の行の確かめが文字列一致だけ
- 成果物: tools/docker-tidy.sh:312-315 / tools/check-test-db-killed.sh:275-276
- 根拠: rust job の `- run:` 行であることまでは見ていない。コメントや別の job に書いても通る。T7 F3
- kind: technical
- 処置:

## R6. 冒頭のコメントの判定の順がコードと逆
- 成果物: tools/check-test-db-killed.sh:18-20
- 根拠: コメントは「空振り → 印」、コードは「印 → 空振り」。T5 F2
- kind: technical
- 処置:

## R7. `probe_active` が datname で絞っていない
- 成果物: tools/check-test-db-killed.sh:75
- 根拠: 前の回のまだ眠っている backend に反応しうる（殺した回の database は印の前にできるので判定は崩れない）。T5 F3
- kind: technical
- 処置:

## R8. `exe_path` が cargo の標準エラーを捨て、ビルドの失敗の理由が出ない
- 成果物: tools/check-test-db-killed.sh:87
- 根拠: コンパイルが落ちても「path が取れなかった」としか出ない。T5 F4
- kind: technical
- 処置:

## R9. 殺す前のきれいな回の rc を捨てている
- 成果物: tools/check-test-db-killed.sh:186
- 根拠: 落ちていても秒数だけが基準として出る。T6 F1
- kind: technical
- 処置:

## R10. `CH05_PROBE_AT` の値の確かめが database を作った後
- 成果物: crates/server/src/killed_probe_tests.rs:67
- 根拠: 値が誤っていても回の database を作ってから落ちる。T4 F1
- kind: technical
- 処置:

## R11. testing.md の検査の数と並びが食い違う
- 成果物: docs/testing.md:31
- 根拠: 「検査 12 本 … / CH05 の 4 行」の数と並びが合わない。T7 F1
- kind: technical
- 処置:

## R12. 文書が docker の要ることを書いていない
- 成果物: docs/flow-gates.md（CH05 の追加行）/ docs/testing.md §4.4.1
- 根拠: `docker-tidy.sh --self-test` は docker の daemon と `postgres:17-alpine` を要する。隣の CH06 の行は「daemon に触らない」と書いている
- kind: technical
- 処置:

## R13. 死んだ命令がある
- 成果物: tools/docker-tidy.sh:220
- 根拠: `echo "$gm" >"$tmp/wts"` は直後の `: >` で消える。T2 F5
- kind: technical
- 処置:

## R14. design の CI の行の時間のうち 2 つが証跡で裏付けられない
- 成果物: openspec/changes/ch05-test-db-bloat/design.md（計測の記録）
- 根拠: `--self-test` の 2 本（3.8 / 2.0 秒）は reviewer の計測（3.6 / 1.9 秒）と合うが、`--verify-env-check` の 16.3 秒と本体の 52.9 秒はどのログにも無く、6.1 の evidence の `duration_s`（13〜32 秒）とすぐには合わない。合否には使っていない。T7 ⚠️
- kind: technical
- 処置:

参考（指摘でない）: `ashiato` の行数の判定は CH06 の後は `core.event` が無く 0 と 0 を比べる（効く守りは大きさで design のとおり）。
`insert` の時点で殺されるのは行を書く前（回の database ごと消すので検査の意味は変わらない）。T2 F6（`*-verify` が広い）は spec の本文どおり。
