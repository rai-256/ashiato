# CH01 code review（whole-branch）

## final review（9998a96..8fc7691）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。入力は review package
`.superpowers/sdd/tasks/review-9998a961..8fc7691a.diff`（37 commits）、Task 1〜8 の指摘の記録の Minor / ⚠️、evidence.jsonl。
`docs/handoff/CH01.md` は無い。ledger（progress.md）は無く、`Ruling:` / parked / deferred-minor の行は無い。
判定: **With fixes**。9 件の直しは plan と design に沿う。Important 1 件（読む口の移動で `*.d/` の節がアプリの役割の準備より前に走る）、Minor 8 件。

⚠️ の確かめ（reviewer が HEAD で走らせた）: `check-shared-appends.sh --self-test` 22 件 ok・`openspec validate --strict` valid・
`check_scenarios.py` rc=0・`docs/openapi.json` が origin/main と同じ・`npm run typecheck` / `lint` rc=0・vitest FeelView 36 passed・
shellcheck（docker の stable）は `-S warning` で変えた 5 本とも rc=0（既定の重さの rc=1 は base から在る info だけ）・
T1 1.1（base で self-test rc=0）と 1.3（見出しの検査を外すと self-test rc=1）を再現・T5 `^## ` が 11・
T7 偽データ `tools/seed.d/202610141000_st17-daily-feeling.sh:160` が 09-11 の最初の滞在の行に +1・T8 `crates/server/src/feelings.rs:441` の `phase_of(written_at, …)`。
確かめられなかったもの（code-verify の席へ）: T2 2.3（壊した木で NG だけが出て rc=1）・T3 3.2（probe が BuildConfig に入る）・T3 3.3（KILL の再現）。

## R1. 読む口を戻しの前へ動かしたため、`*.d/` の節がアプリの役割（`ashiato_app`）・付与・`app_psql` の準備より前に走る
- 成果物: tools/check-immutable.sh:1174（読む口）・:1330-1332（`./tools/db-roles.sh`・grants.sql・`app_psql()`）/ tools/check-immutable.d/README.md / 読む口の注記
- 根拠: base（9998a961）では `db-roles.sh` が :1486・`app_psql()` が :1488 で、読む口 :1507 より前。HEAD では読む口 :1174 が先で、1〜1174 行に `db-roles` / `ashiato_app` / `grants.sql` は無い（docker-compose.yml にも migrations にも `ashiato_app` は無く、検査は毎回 `down -v` の新しい DB）。
  open で draft でない ST09（#119、確認バッチ 20261023-0512 に入っている）の `origin/feat/st09-media-capture:tools/check-immutable.d/202610181200_st09-media-capture.sh` は「呼び出し元が app_psql と grants の付与を済ませている」と書き、`st09_app -c "INSERT …" >/dev/null` を `set -e` の下でガード無しに撃つ。
  CH01 と ST09 のどちらが後に merge されても、統合した木の `check-immutable.sh` は役割が無いまま psql が落ちて無言で抜ける（NG の行も出ない）。
  README と読む口の注記の「関数（`psql` `app_psql`）を上書きしない」は、節が `app_psql` を使えると読めるが、新しい位置では未定義。design.md:128 のリスク表は逆向き（本体の後ろが `*.d/` に依るか）しか見ていない
- kind: conflict
- 提案: 読む口のかたまりの中（見出しの注記と `for … done` の間）で、`./tools/db-roles.sh`・grants.sql の付与・`app_psql()` の定義を済ませる（`db-roles.sh` は何度走らせても同じ結果。本体の :1330 の再実行は残す）。
  Global Constraint「本体を直すのは読む口の移動と ST17 の節を抜くことだけ」を越えるので design に `D<n>（仮）` と反転条件（役割の準備を読む口に置いて後ろの検査が崩れるなら、D1 の反転 —— 読む口を末尾に戻し節の頭で版を当て直す —— へ）。
  ST09 の節を手元に置いて（commit しない）落ちることを先に見てから直す
- 処置:

## R2. `web/e2e/feel-ch01.spec.ts` の頭のコメントが「DB には何も足さない」と言うが、同じファイルに書き込む e2e が 4 本ある
- 成果物: web/e2e/feel-ch01.spec.ts:5
- 根拠: 同じファイルの Task 7・8 の e2e は乱数で選んだ 2100 年以降の日に書く。Task 7 F3・Task 8 F1 で 2 回指摘されたまま
- kind: technical
- 提案: 頭のコメントを、読み出しの e2e（書かない）と書き込みの e2e（2100 年以降の日・件数は差で数える）の両方を言う形に直す
- 処置:

## R3. ST17 の節の注記が古い（`immutable-check` → `st17-authored-check`）
- 成果物: tools/check-immutable.d/202610222100_st17-feeling-locks.sh:165
- 根拠: 直下の検査が書き換えるのは `st17-authored-check` の行。同じファイル :8 の「`immutable-check` の行は使わない」とも食い違う。Task 2 F3 が未対応
- kind: technical
- 提案: 注記の語を `st17-authored-check` に直す
- 処置:

## R4. 滞在の行の印が、応答の届く順によって消える
- 成果物: web/src/FeelView.tsx:250
- 根拠: 断られた枝が `stayPicks[p.key]` を今の値と照らさずに外す。同じ行で -2 を押す → 返事待ちの間に +1 を押す → +1 が「届かなかった」で先に返り、-2 が「断られた」で後に返ると、+1 の印が消える（D5 は「断られた値」を戻すのであって、後から押した値を消すのではない）
- kind: technical
- 提案: 印が断られた値と同じとき（`picks[p.key] === p.v`）だけ外す。vitest で順の入れ替わりを固定する
- 処置:

## R5. 主張の地域のずれが範囲の外の試験が、格納されないことを件数で見ていない
- 成果物: crates/server/src/authored_rule_tests.rs:110-128
- 根拠: 理由の種別しか見ず、Scenario の THEN「格納されない」を確かめていない。場所の側の同じ Scenario は件数を見る。主張の行は消せないので件数で見る価値がある。Task 4 F1
- kind: technical
- 提案: 場所の側と同じ形で、送る前と後の件数が同じことを確かめる
- 処置:

## R6. vitest の 2 か所が弱い（ボタンが無いと黙って抜ける・3 回目の読み出しを捕まえない）
- 成果物: web/src/__tests__/FeelView.test.tsx:468・:481
- 根拠: :468 の `if (b !== null)` でボタンが無いと検査が抜ける。:481 は `toHaveLength(2)` を待つだけで、後から来る 3 回目を捕まえない。Task 6 F1・F2
- kind: technical
- 提案: ボタンの存在を assert してから押す。読み出しの回数は待った後にもう一度ためてから数える（または読み出しが落ち着いた後に厳密に数える）
- 処置:

## R7. `setNow(p.at)` の意図が書かれていない
- 成果物: web/src/FeelView.tsx:240
- 根拠: 直後の読み直しの `setNow(new Date())` で上書きされ、意味を持つのは読み直しが終わるまでの間と失敗したときだけ。Task 8 F3
- kind: technical
- 提案: その意図をコメントに 1〜2 行
- 処置:

## R8. `tools/check-handoff-reaches-brief.sh` の照合が change 名だけ
- 成果物: tools/check-handoff-reaches-brief.sh:10
- 根拠: needle が change 名だけなので、申し送りの節そのものが brief に載ったかまでは見ない。Task 5 F2
- kind: technical
- 提案: 節の見出しの文で照らすか、照らしているものの限界を注記に書く
- 処置:

## R9. `check-shared-appends.sh` の「動かしただけ」の道に残る限界
- 成果物: tools/check-shared-appends.sh（`mouth_sig` の道）
- 根拠: 同じ commit の中で、動かした先の直後に見出しの無い行を足しても落ちない。seed.sh で最後から 2 行目の扱いが動かす前と変わる。見出しの検査（`heading_added`）があるので実害は小さい。Task 1 F2・F3
- kind: technical
- 提案: 注記に限界を書く
- 処置:
