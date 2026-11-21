# CH22 仮で進めた決定の確かめ

## D16
- 判定: 確か
- 根拠: 失わない側が本当に失わないか —— `tools/release/soak-install.sh:757`〜`758` で、`GITHUB_ACTIONS=true` と zip 2 つの存在の確かめが `soak_run`（`:358`）より前、つまり `soak_reset_machine`（`:385` → `:224`〜`:249` の `rm -rf`）より前にある。実際に走らせて確かめた: 一時の箱に `LOCALAPPDATA/ashiato/collector/outbox.jsonl` を置き、`env -u GITHUB_ACTIONS -u CI` で存在しない zip 2 つを渡すと `NG: 置く段は GitHub Actions の runner でのみ実行できます` rc=1。`GITHUB_ACTIONS=true` でも zip が無ければ `NG: 配る zip を読む前に確認できない` rc=1。どちらでも `outbox.jsonl` は残った（discarded は起きない）。ほかの口も見た: `--self-test-run`（`:745`）は `soak_self_test_command_guard`（`:730`〜`:740`）が `rm`・`sc.exe`・`taskkill.exe` などを偽の命令の置き場から引けないと止め、置き場も相対の `collector-state` に固定する。`--evaluate`・`--list` は reset を呼ばない。CI の側（`.github/workflows/ci.yml:654`）は Actions の runner の上なので `GITHUB_ACTIONS=true` が付き、置く段は止まらない。
  代償 —— 「CI の外では実地の口を使えない」は、spec の Requirement の本文（`specs/distribution/spec.md:5`・`:82`「CI の Windows の runner で」）の範囲の中なので、要件の本文は変わらない。直したのは 1 行の分岐で、高くつかず Story の範囲も越えない。`docs/testing.md:185` は「Windows の実地の確かめ」の台本として並べているが、本人の PC で走らせる手順は書いていない。
  follows: 無し。rests_on: 空だが、土台にしている決定は依頼の本人の決定（CI の Windows の試験の後段に置く。`docs/changes/CH22.md`）と spec の本文で、仮の決定ではない。D1（仮）は台本の口の形を決めているだけで、D1 が反転しても（台本の置き場が変わっても）この守りの要否は変わらない
- 備考: 範囲外のまま残る —— `update-check-install.sh`・`update-apply-install.sh` の reset も CI の上かどうかを見ずに `C:/ProgramData/ashiato` を消す（既存。review/code.md R11 の「範囲外」）

## D17
- 判定: 確か
- 根拠: 失わない側が本当に失わないか —— 300 秒の値は `tools/release/soak-install.sh:14` にあり、自己試験の `:664` が `[[ "$SOAK_WAIT_SECONDS" == 300 ]]` で確かめる。走らせて確かめた: 今の台本の `--self-test` は `OK: soak-install self-test`（rc=0）。複製の台本で `:14` を `SOAK_WAIT_SECONDS=30` に書き換えると rc=1、`NG: self-test: SOAK_WAIT_SECONDS is not 300` で落ちた（review/code.md R10 の再現が塞がっている）。自己試験は値を確かめるだけで、捨てる・外に出す・取らないものは無い（uncaptured / discarded / exported / rewrite-all のどれにも当たらない）。`--self-test-run` が `:749` で `SOAK_WAIT_SECONDS=2` に上書きするのは、その場合の子のプロセスの中だけで、`:664` は親の値を読む。
  代償 —— 「時間を変えるには設計を見直す」は、5 分が本人の決定（spec `:13`「5 分ほど」・`:3` の Requirement の名前・`:80` の Scenario の名前）なので、要件の本文どおりの値を固定するだけ。1 行の assert で、高くつかず範囲も越えない。
  follows: 無し。rests_on: 空。土台は本人の決定「5 分ほど」で、仮の決定ではない（D5（仮）は「置く間」の始まりの時刻を決めていて、長さの 5 分は D5 自身が本人の決定として引いている。design.md:60）
