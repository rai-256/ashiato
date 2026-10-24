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
- 処置: escalated — Q8（2026-10-24 本人がいまの形のままを選んだ。別の席が仮の D11 を否としたので台帳から外して人間に返した）。以下は仮で直したときの記録 —— 9a3d1a03。`db-roles.sh`・grants.sql の付与・`app_psql()` を読む口の見出しの直前に置いた（かたまりの中に置くと `mouth_sig` の判定で `check-shared-appends.sh` が rc=1 になるため外の直前。D11 に理由）。読む口の注記と README を実態に合わせた。ST09 の節（と移行）を手元に置いて rc=2・`role "ashiato_app" does not exist`・NG 行なしを再現（`.superpowers/sdd/logs/20261023-184029-immut-r1-red2.log`）→ 直して rc=0・ST09 の OK 9 行。ST09 なしの OK/NG 138 行は直す前と diff 空。`provisional.py add --id D11 --source review/code.md#R1 --rests-on D1`

## R2. `web/e2e/feel-ch01.spec.ts` の頭のコメントが「DB には何も足さない」と言うが、同じファイルに書き込む e2e が 4 本ある
- 成果物: web/e2e/feel-ch01.spec.ts:5
- 根拠: 同じファイルの Task 7・8 の e2e は乱数で選んだ 2100 年以降の日に書く。Task 7 F3・Task 8 F1 で 2 回指摘されたまま
- kind: technical
- 提案: 頭のコメントを、読み出しの e2e（書かない）と書き込みの e2e（2100 年以降の日・件数は差で数える）の両方を言う形に直す
- 処置: fixed 8.1 — ec69be59。頭のコメントを書かない前半と書く後半（2100 年以降の乱数の日・件数は差）の両方に。ET feel-ch01.spec.ts 8 passed

## R3. ST17 の節の注記が古い（`immutable-check` → `st17-authored-check`）
- 成果物: tools/check-immutable.d/202610222100_st17-feeling-locks.sh:165
- 根拠: 直下の検査が書き換えるのは `st17-authored-check` の行。同じファイル :8 の「`immutable-check` の行は使わない」とも食い違う。Task 2 F3 が未対応
- kind: technical
- 提案: 注記の語を `st17-authored-check` に直す
- 処置: fixed 2.2 — cc059cae

## R4. 滞在の行の印が、応答の届く順によって消える
- 成果物: web/src/FeelView.tsx:250
- 根拠: 断られた枝が `stayPicks[p.key]` を今の値と照らさずに外す。同じ行で -2 を押す → 返事待ちの間に +1 を押す → +1 が「届かなかった」で先に返り、-2 が「断られた」で後に返ると、+1 の印が消える（D5 は「断られた値」を戻すのであって、後から押した値を消すのではない）
- kind: technical
- 提案: 印が断られた値と同じとき（`picks[p.key] === p.v`）だけ外す。vitest で順の入れ替わりを固定する
- 処置: fixed D5 — 6c7c911c。断られた枝で印は `picks[p.key] === p.v`、送り直しは `all[p.key] === p` のときだけ外す。vitest で -2 → +1 → +1 届かず → -2 断られる の順を固定（RED: `.superpowers/sdd/logs/r4-red.log`）

## R5. 主張の地域のずれが範囲の外の試験が、格納されないことを件数で見ていない
- 成果物: crates/server/src/authored_rule_tests.rs:110-128
- 根拠: 理由の種別しか見ず、Scenario の THEN「格納されない」を確かめていない。場所の側の同じ Scenario は件数を見る。主張の行は消せないので件数で見る価値がある。Task 4 F1
- kind: technical
- 提案: 場所の側と同じ形で、送る前と後の件数が同じことを確かめる
- 処置: fixed 4.1 — 04ca55e3。`core.event` の件数が送る前と後で同じことを見る。受理される主張を足す壊しで落ちる。CT ch01_authored_ 14 passed

## R6. vitest の 2 か所が弱い（ボタンが無いと黙って抜ける・3 回目の読み出しを捕まえない）
- 成果物: web/src/__tests__/FeelView.test.tsx:468・:481
- 根拠: :468 の `if (b !== null)` でボタンが無いと検査が抜ける。:481 は `toHaveLength(2)` を待つだけで、後から来る 3 回目を捕まえない。Task 6 F1・F2
- kind: technical
- 提案: ボタンの存在を assert してから押す。読み出しの回数は待った後にもう一度ためてから数える（または読み出しが落ち着いた後に厳密に数える）
- 処置: fixed 6.1 — 33c74953。ボタンを `getByTestId` で求め、`settle()` の後に厳密に 2 回。読み出しを 3 回にする壊しで `got 3` と落ちる

## R7. `setNow(p.at)` の意図が書かれていない
- 成果物: web/src/FeelView.tsx:240
- 根拠: 直後の読み直しの `setNow(new Date())` で上書きされ、意味を持つのは読み直しが終わるまでの間と失敗したときだけ。Task 8 F3
- kind: technical
- 提案: その意図をコメントに 1〜2 行
- 処置: fixed 8.1 — 3412ef8e

## R8. `tools/check-handoff-reaches-brief.sh` の照合が change 名だけ
- 成果物: tools/check-handoff-reaches-brief.sh:10
- 根拠: needle が change 名だけなので、申し送りの節そのものが brief に載ったかまでは見ない。Task 5 F2
- kind: technical
- 提案: 節の見出しの文で照らすか、照らしているものの限界を注記に書く
- 処置: fixed 5.3 — 095eb156・92a287d4。見出しに needle を含む節の見出しと本文の全部を、記号と空白を除いて照らす。本文の 1 文を消した brief で NG。限界は注記

## R9. `check-shared-appends.sh` の「動かしただけ」の道に残る限界
- 成果物: tools/check-shared-appends.sh（`mouth_sig` の道）
- 根拠: 同じ commit の中で、動かした先の直後に見出しの無い行を足しても落ちない。seed.sh で最後から 2 行目の扱いが動かす前と変わる。見出しの検査（`heading_added`）があるので実害は小さい。Task 1 F2・F3
- kind: technical
- 提案: 注記に限界を書く
- 処置: fixed 1.2 — 4127c2fc。限界 2 点を注記に書いた（規則は変えない）

## fix wave（130aea7..4127c2f）の scoped re-review

席: re-reviewer（SDD の `re-review-prompt.md`）。R1〜R9 すべて ADDRESSED、fix の diff に新たな Critical / Important は無し。
R1 を読む口のかたまりの外の直前に置いたことは、R1 を直し、共有の本体の規則の範囲内（`check-shared-appends.sh` rc=0 を再確認）。
修正の報告: `.superpowers/sdd/ch01-final-fix-report.md`。最後の検証は typecheck / lint / VT 37 / ET 8 / CT 14 / check-shared-appends と self-test /
shellcheck `-S warning` / `openspec validate --strict` / `check_scenarios.py` / `check-immutable.sh` / clippy / fmt がすべて rc=0（報告の申告）。
残り 1 件（R4 と同じ形・fix wave は 1 回だけなので finish へ）を R10 に写す。

## R10. 受理された枝が、後から押した値の送り直しを無条件に消す（R4 と同じ形）
- 成果物: web/src/FeelView.tsx:234
- 根拠: R4 の直しで断られた枝は `all[p.key] === p` で送り直しを守るようになったが、受理された枝は `setPending((all) => without(all, p.key))` を無条件に呼ぶ。-2 の返事待ちに +1 を押し、+1 が届かず先に返ってから -2 が受理されると、+1 の印は残るのに送り直しが消え「もう一度送る」が出ない（直す前から在った挙動。fix の実装者と re-reviewer の両方が指摘）
- kind: technical
- 提案: 受理された枝も `all[p.key] === p` のときだけ外す。vitest で順の入れ替わりを固定する
- 処置: fixed D5 — 受理された枝は、この 1 件の送り直しか、この 1 件が対象で最後に押したもののときだけ送り直しを外す（`FeelView.tsx` の `lastSent`）。vitest「-2 の返事待ちに +1 を押し、+1 が届かず先に返って -2 が受理されて後に返ると…」を足し、直す前の `FeelView.tsx` で落ちることを見た（R17 と同じ直し）## code_verify（独立検証。HEAD 84acd2e4）

席: code_verify（実装者でない）。全部この席で走らせた。作業ツリーは壊す前と後で `git status --porcelain` / `git diff` がどちらも空のまま
（`/tmp/ch01v-status0` `/tmp/ch01v-diff0` と比べた）。変異は `web/src/FeelView.tsx` と `crates/server/src/ingest.rs` に一時的に入れ、そのたびに戻した。
`tools/check-immutable.sh` は仕様どおり `docker compose down -v` で この worktree の DB を作り直すので、走らせた後に `docker compose up -d --wait db` と `tools/db-roles.sh` で立て直した（偽データは入っていない）。

申告: 22/22（`- [x]` 22・`- [ ]` 0）。検証コマンドは tasks.md の各項目のとおり（CT / VT / ET は Global Constraints の定義）。

| 項目 | 申告 | 実測 | 一致 |
|---|---|---|---|
| 1.1 直す前の self-test | rc=0 | base（9998a961）の写しで `--self-test` rc=0・ok 11 | 一致 |
| 1.2 self-test | rc=0・ok 21 以上 | rc=0・ok 22・NG 0 | 一致 |
| 1.2 / 2.2 / 3.1 `shellcheck … rc=0` | rc=0 | 手元に shellcheck が無い（rc=127）。docker の `koalaman/shellcheck:stable` で既定の重さは **rc=1**（info のみ。base の 3 本も rc=1）。`-S warning` なら rc=0 | **不一致**（R13） |
| 1.3 見出しの検査を外した写し | rc=1・`self-test NG` | `if heading_added …` の 4 行を消した写しで rc=1・`self-test NG: 本体の途中に # ==== ST99 …`。`git diff --stat` は前後で同じ | 一致 |
| 1.4 文書と smoke/seed | grep -L 空・diff 無し | 空・`git diff --quiet origin/main -- tools/smoke.sh tools/seed.sh` rc=0 | 一致 |
| 2.1 移す前の出力 | rc=0・OK ≥1・OK feeling ≥1 | base の worktree で rc=0・OK 138・OK feeling 18 | 一致 |
| 2.2 移した後 | rc=0・OK の集合が同じ ほか | rc=0・OK 138・`diff` 空・`== 書き換え禁止の節: 202610222100_st17-feeling-locks`・本体に ST17 の節なし・本体の変数の上書きなし | 一致 |
| 2.3 壊した down.sql | rc=1・NG あり・OK なし | HEAD: rc=1・`NG 戻しで主観の行か…減った`・OK の行なし。base: rc=1・NG と OK が並ぶ（壊し方が効く） | 一致 |
| 2.4 印の移動・branch の検査 | diff 空・rc=0 | diff 空・`check-shared-appends.sh feat/ch01-…` rc=0 | 一致 |
| 3.1 local.properties | 各 rc | check-ignore rc=0・grep 該当なし・`bash -n` rc=0 | 一致 |
| 3.2 BuildConfig に入る | rc=0・grep あり | gradle rc=0・`BuildConfig.java` に `ch01-probe`。後で local.properties を消した | 一致 |
| 3.3 KILL | 追跡ファイルに残らない | gradlew を印＋`sleep 300` の stub にして `record-env.sh up`（REC_PLATFORMS=android）を `setsid`、印が出てから `kill -9 -<pgid>`。HEAD: `gradle.properties` の status 空・追跡と未追跡のどちらにもトークン無し（無視される local.properties にだけある）。base: ` M collector-android/gradle.properties` でトークンが残る | 一致 |
| 4.1 / 4.2 CT | 各 1 本以上 | `ch01_authored_` 14・`st17_reject_` 31・`ingest_rejects_out_of_range_tz_offset` 1・`place_` 163（`-- --list` も同数）。clippy rc=0・`origin != "authored"` 0 | 一致 |
| 5.1 / 5.2 / 5.3 | CT 2 本以上・grep・検査 | `ch01_stay_open_` 2・`span_of` 1・`event_by_source_time` 1・`check-handoff-reaches-brief.sh` rc=0・`^## ` 11 | 一致（申し送りの中身は R14） |
| 6.1 / 7.1 / 8.1 VT | passed | `FeelView.test.tsx` 37 passed | 一致 |
| 6.1 / 6.2 / 7.1 / 7.2 / 8.1 ET | passed | **作り直した `web/dist` で** feel-ch01 8・feel 21・feel-parts 15・feel-limits 7 passed（作り直す前の dist でも同数。ただし R11） | 一致（定義に穴: R11） |
| 6.1 typecheck / lint | rc=0 | rc=0 / rc=0 | 一致 |
| 9.1 | 各 rc=0 | validate・check_scenarios（scenarios: OK）・log-private・fmt・check-openapi・openapi.json が main と同じ・check-shared-appends・`npm run build` すべて rc=0 | 一致 |

手 1: 直書きの期待値は `/api/feelings?from=2026-09-09&to=2026-10-08`（`FeelView.test.tsx`。30 日の起点）だけ。python の `date(2026,10,8) - timedelta(days=29)` = 2026-09-09 で一致。ハッシュ等の固定値は足していない。

手 2: 「落ちる」を確かめる検査を壊した写しで走らせた —— 1.3（見出しの検査を外すと self-test rc=1）・2.3（壊した down.sql で rc=1）・3.3（base で token が残る）は全部効いた。
検査の外側として、見出しの型の外の書き方を `/tmp` の使い捨ての clone で試した（R12）。

手 3: delta の新しい Scenario 19 件（主張 6・場所 5・日をまたぐ 4・滞在の行 3・読み出しの失敗 1）の印の先を 1 件ずつ読んだ。主張・場所の順は理由の種別を `assert_eq!`、
範囲の外の 3 件は件数（主張は利用者の行の前後・場所は id）、画面は e2e が `aria-pressed` と `/api/feelings` の `phase` を見ていて、主張の階層とテストの階層は合っている。
変異で確かめたもの: `authored_breach` の外部識別子と地域のずれの順を入れ替える → `ch01_authored_` 2 本落ちる / `crossed = false` → vitest 1 本落ちる（作り直した dist の ET も落ちる）/
滞在の行の印の戻しを消す → vitest 2 本落ちる / 読み出しを 29 日に縮める → vitest 1 本落ちる。

手 4: 本人の答え Q1〜Q7 の値ごとに変異を入れた。Q5・Q6・Q7・Q4（滞在の行）・Q1/Q2（self-test・check-immutable）は落ちる。**Q4 の「日の欄は spec どおり（断られても選んだ値を残す）」は値を変えても全部通る**（R15）。
Q3（直さない）は `ch01_stay_open_` 2 本が今の振る舞いを固定している。

手 5: `[x]` 22 件の検証コマンドは全部実在し、表のとおり走った。`--test <名前>` 型は無い。食い違いは shellcheck の rc（R13）と ET の定義（R11）。

手 6: 確かめた範囲 —— KILL の経路（3.3 で再現）・読む口の前の役割の準備（D11。HEAD の `check-immutable.sh` が rc=0 で、`app_psql` は読む口の直前 :1179 にある）・
`*.d/` の節の `return 0`（`.` で読まれるので節だけ抜ける）・受理されても断られても印と送り直しが残るか（R10 は既に在る。断られた枝は `all[p.key] === p` と `picks[p.key] === p.v` で守られている）。
「捨てたものは復元できない」型の新しい経路は見つからなかった。ST23 へ渡す文書の誤り 1 件を R14 に書く。

手 7: 訂正 1 件（5.3）。申し送りの節を消すと新しいコマンドは rc=1（`correction-review.md`）。`plan_fix.py check` rc=0。

## R11. `ET` の定義は `web/dist` を作り直さないので、直した後の e2e が古い画面を見て緑になる
- 成果物: tasks.md Global Constraints の `ET` / tools/stack.sh:63-67（サーバも :57-61 が同じ条件）/ evidence.jsonl の ET の 16 行
- 根拠: `tools/stack.sh` は `web/dist/index.html` があれば build しない。`FeelView.tsx:289` を `const crossed = false;` にして作り直さずに
  `npx playwright test e2e/feel-ch01.spec.ts -g '振り返りとして残した'` → rc=0・1 passed。`npm run build` の後に同じもの → rc=1・1 failed。
  tasks の ET（6.1 / 6.2 / 7.1 / 7.2 / 8.1）と evidence.jsonl の PASS が、その時点の HEAD の画面を見ていた保証が無い（この席で作り直した dist では 4 本とも通った）
- kind: technical
- 提案: `ET` の定義の頭に `cd web && npm run build` を足すか、`stack.sh` が `web/src` より古い dist を作り直す。サーバの release バイナリも同じ
- 処置: fixed 6.2 — `tools/stack.sh` が、既定の置き場のサーバと `web/dist` がソースより古ければ作り直す（`newer_src`）。tasks の `ET` の文字列はそのまま成り立つ形になる。直した後の `ET feel.spec.ts feel-ch01.spec.ts feel-parts.spec.ts feel-limits.spec.ts` で `== サーバを build（…ソースより古い）`・`== 画面を build（…ソースより古い）` が出て 51 passed（`.superpowers/sdd/logs/20261023-193938-et-finish.log`）。kind は premise → technical（finish が直した: 検証コマンドの文字列は変えず、コマンドが頼る道具の欠陥を直したので、計画の前提を人間に問う形にならない。失うものも無い。code-verify は作り直した dist で 4 本とも通ることを確かめ済み）

## R12. 本体の途中への節の見出しの検査は、3 つの型の外の書き方を通す
- 成果物: tools/check-shared-appends.sh:52-62（`heading_added`）/ AGENTS.md:27 / docs/testing.md:96
- 根拠: CH01 を merge した後の木を main にした使い捨ての clone で、`feat/st99-*` が `tools/check-immutable.sh:856`（ST33 の節の前）に 2 行足した。
  `echo "== ST99: 写真の錠"` → rc=1（落ちる）。`echo "== 写真の錠（ST99）"` → rc=0・`# ==== 写真の錠と台帳` → rc=0・`echo '== ST99: 写真の錠'` → rc=0。
  文書は「本体の途中に新しい節の見出しを足すと落とす」と型の条件なしに書く。D2 が 3 つの型に絞ったのは設計どおり
- kind: technical
- 提案: `^echo ["']== .*ST[0-9]{2}` まで広げるか、文書に「見出しの型は 3 つ。型の外は落ちない」と限界を書く
- 処置: fixed 1.4 — 文書に限界を書いた（`AGENTS.md` の「テスト」・`docs/testing.md` §4・`tools/check-shared-appends.sh` の頭の注記）。見出しの型は D2 の 3 つのまま（広げると既存の節の中の進みの echo まで落とす）

## R13. tasks の `shellcheck … rc=0` は、既定の重さでは base でも rc=1 で、成り立たない
- 成果物: tasks.md 1.2 / 2.2 / 3.1 / tools/check-immutable.sh:1179
- 根拠: 手元は `shellcheck: command not found`（rc=127）。docker の `koalaman/shellcheck:stable` で変えた 5 本は rc=1（info: SC2015 ×6・SC2086 ×3・SC1091・SC2016・SC2028・SC2329）。
  base の 3 本も rc=1（SC2015 ×7 ほか）。`-S warning` なら rc=0。HEAD で増えた 1 件は SC2329（`app_psql` を本体の中で呼ばない。`*.d/` が `.` で読んで使う）
- kind: daily
- 提案: tasks の検証を `shellcheck -S warning …` に書き直すか、`app_psql` に `# shellcheck disable=SC2329` と理由を付ける
- 処置: fixed D12 仮 — tasks の `shellcheck … rc=0` は `-S warning` で読む（base も既定の重さで rc=1）。R22 で二度目の `app_psql` を消したので、この change が増やした SC2329 も消えた（既定の重さの info は SC1091 ×1・SC2015 ×5・SC2086 ×3 で、どれも base から在る）。`-S warning` で `check-shared-appends.sh` `record-env.sh` `check-immutable.sh` rc=0（`stack.sh` の SC2155 ×2 は base から在る）

## R14. ST23 への申し送りが、気分の論理ソースを `s03-feeling` と書いている（実物は `s01-feeling`）
- 成果物: docs/handoff/ST23.md:111 / crates/server/src/feelings.rs:16
- 根拠: `grep -rn 's03-feeling'` は docs/handoff/ST23.md:111 の 1 行だけ。`feelings.rs:16` は `pub const SOURCE: &str = "s01-feeling";`、錠の節も `s01-feeling` を使う。
  `check-handoff-reaches-brief.sh` は申し送りの文字が brief に載ることだけを見るので、誤りのまま ST23 の brief に届く（rc=0）
- kind: technical
- 提案: `s01-feeling` に直す
- 処置: fixed 5.3 — `docs/handoff/ST23.md:111` を `s01-feeling` に直した（`grep -rn s03-feeling` が 0 件）

## R15. 本人の決定「日の欄は断られても選んだ値を残す」（Q4）を、どのテストも固定していない
- 成果物: web/src/FeelView.tsx:248 / web/e2e/feel.spec.ts:299-315 / web/src/__tests__/FeelView.test.tsx:338-350 / tasks.md 7.2
- 根拠: 断られた枝（`FeelView.tsx:248` の直後）に `if (p.key === "day") setPick(null);`（滞在の行と同じく最新の記録の値に戻す）を足すと、
  vitest `FeelView.test.tsx` 37 passed、**`npm run build` で作り直した dist** で `ET feel.spec.ts -g '断られ|届かなかった'` 3 passed（rc=0）。
  7.2 が挙げる「断られたとき理由の文が出る」は、e2e も vitest もひとことの欄の値しか見ず、快–不快のボタンの `aria-pressed` を見ない
- kind: technical
- 提案: その e2e（か vitest）で、断られた後も押した快–不快が `aria-pressed="true"` のままであることを見る
- 処置: fixed 7.2 — e2e `feel.spec.ts`「ひとことが長すぎるとして断られると…」と vitest の同じ試験で、断られた後も `valence-1` が `aria-pressed="true"` のままであることを見る。断られた枝に `if (p.key === "day") setPick(null);` を足す壊しで vitest が落ちる（`1 failed | 39 passed`）

## security review（9998a96..84acd2e）

見た経路:
- 経路の数: `git diff 9998a96..HEAD -- crates/ web/src` に `.route(` `.nest(` `.layer(` `TcpListener` は 1 行も無い。画面の `fetch` は既存の `/api/feelings` の範囲を 1 日 → 30 日に変えた 1 本だけ（`FeelView.tsx:118`）。認証の外に出る新しい入口は無い（4。grep を走らせた）
- 取り込み: `POST /api/ingest` → `lib.rs` `ingest_request` → `IngestRequest::authored_breach`（`ingest.rs:124`）→ 主張・主観・場所の拒否。由来と端末 → 外部識別子 → 地域のずれの順は旧コードと同じで、主観は旧コードに無かった地域のずれの検査が足されただけ（拒否が増える側にしか動かない）。値は SQL へ `.bind` で渡り、文字列として組み込まれない（3）。`cargo test -p ashiato-server ch01_` → 16 passed（4）
- 主観の検査（`feelings.rs:248` 付近）と始まりの日の読み出し（`:627`）は、コメントだけの変更。SQL は変わっていない（2）
- 画面: 断られた／保存した通知（`FeelView.tsx:238-260`・`messages.ts:436`）が出すのは、押した値・記入時刻・対象日・滞在の範囲だけ。console やログへは出さない（2）
- 録画の token: `record-env.sh:167-168` が `.env` の `API_TOKEN` と `ASHIATO_USER_ID` を一時 worktree の `collector-android/local.properties` に書く（`.gitignore:29` で無視）→ `app/build.gradle.kts:14-17` が読んで APK に埋め込む。worktree は `record-run:593` の `git worktree remove --force` で消える。token は実行ごとの `openssl rand`（`record-env.sh:86`）で、使う先のサーバも同じ片付けで消える（3）
- 同じファイルの権限と写し: `umask 077` は `:121` で戻るので、`local.properties` は 0644 で作られる（`.env` は 0600）。Windows の組み立ての `rsync`（`:184`）は `local.properties` も C: のキャッシュへ写す。/tmp の写しで 2 つとも再現した（`600 .env` / `644 …/local.properties`。写し先に 2 つとも残る）。ただし同じ token は差分の前から `.env` として同じ場所に写っている。実行が終わればその token で通るサーバは無いので、差分が増やす露出は無い（4）
- 依存: `build.gradle.kts` は標準の `java.util.Properties` を import しただけ。crate・npm・gradle の依存は足していない（2）
- 道具: `check-handoff-reaches-brief.sh` は `python3 -I` に引数でパスを渡す。`check-immutable.d/…st17-feeling-locks.sh` の `psql` に入るのは台本の中で作った UUID だけ（2）
- テストの固定値: `web/e2e/feel-ch01.spec.ts:94` の `dev-token-0123456789abcdef` は、既存の `feel.spec.ts:17`・`tools/stack.sh:33` と同じ開発用の既定値（2）

安全の決め手: この差分は待ち受けも経路も足しておらず、サーバ側の変更は取り込みの拒否を 1 関数にまとめた（`authored_breach`）だけで、受け入れる範囲は広がっていない —— 確かさ 4。経路を足す行が 0 であることを grep で確かめた。`cargo test -p ashiato-server ch01_` は 16 passed。その中で、規則の順（由来 > 外部識別子 > 地域のずれ）と、範囲の外の地域のずれが件数として格納されないことを assert している

指摘なし

範囲外: `record-env.sh:184` の `rsync` が `.env`（DB の合言葉・画面の合言葉・token）を `/mnt/c/dev/ashiato2-rec-build/src` に写し、片付けでも消さない（差分の前から。値は実行ごとの乱数）

## /code-review（9998a96..84acd2e）

見た: Claude Code の組み込みの `/code-review`（9 件）。写しは scripts/code_review_record.py

## R16. The API token and user ID are now left in collector-android/local.properties after the build. The Windows step that runs next rsyncs the whole worktree to the persistent C: build cache, and local.properties is not excluded, so the token gets copied there and stays.
- 成果物: tools/record-env.sh:168
- 根拠: Run with REC_PLATFORMS="android windows": the Android build writes ashiato.apiToken=<token> into $WT/collector-android/local.properties and nothing removes it. The Windows block then runs `rsync -a --delete --exclude target --exclude .git ... "$WT/" /mnt/c/dev/ashiato2-rec-build/src/`. The token ends up in /mnt/c/dev/ashiato2-rec-build/src/collector-android/local.properties, a cache that is never cleaned up. Before this change, `git checkout -- gradle.properties` restored the file before the rsync, so the token never left the worktree.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.1 — `tools/record-env.sh` の Windows の写しで `collector-android/local.properties` を除き、写し先に前の版が残した写しも消す。/tmp の写しで再現: 旧は写し先に `ashiato.apiToken=SECRET` が残り、新は `grep -r apiToken` rc=1

## R17. The R4 race fix covers only the rejected path. The accepted path still calls `without(all, p.key)` unconditionally, so a late acceptance of an earlier send deletes a newer pending resend for the same key.
- 成果物: web/src/FeelView.tsx:230
- 根拠: On a stay row, press +1 (A, request slow). Then press +2 (B); B is unreachable, so pending[row]=B and a resend is offered. A's 'accepted' reply arrives next and runs setPending(without(all,row)). B vanishes from pending, the resend control disappears, and the user's last choice (+2) is never stored, even though stayPicks still shows +2.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D5 — R10 と同じ直し（受理された枝は `all[p.key] === p || lastSent.current[p.key] === p` のときだけ送り直しを外す）。vitest で -2（返事待ち）→ +1（届かない）→ -2 受理 の順を固定し、送り直しが +1 を送る。直す前の `FeelView.tsx` で落ちる

## R18. On rejection, the stay-row mark is cleared by comparing values (`picks[p.key] === p.v`) instead of checking which send set it, so a rejected earlier send can clear the mark of a later in-flight or pending send that has the same value.
- 成果物: web/src/FeelView.tsx:252
- 根拠: On one stay, press +1 (A), then +2 (B), then +1 again (C). A comes back rejected after C was pressed. picks[row] is 1, which equals A.v, so the mark is removed. If C is then unreachable, C waits in pending while the row shows no selection or the old value. The screen no longer matches the value waiting to be resent, which is exactly the R4 case the comment says it guards against.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D5 — 断られた枝で印を外すのは、断られた 1 件がその行で最後に押したもの（`lastSent.current[p.key] === p`）のときだけにした（値で比べない）。vitest で +1（返事待ち）→ +2 受理 → +1 届かない → 最初の +1 断られる の順を固定（+1 の印と送り直しが残る）。直す前の `FeelView.tsx` で落ちる

## R19. After the mouth moved, protected range 1 is only [line just before the mouth heading, done]. The new prep block (db-roles, grants, app_psql) now sits between the last section and the mouth, so a Story can append a body section after ST21 or inside the prep block and not be caught.
- 成果物: tools/check-shared-appends.sh:92
- 根拠: On the post-CH01 main, a feat/st50 branch inserts `echo "== 新しい検査"; psql -c ...` (no ST-numbered heading) right after ST21's last line (1171) or after the grants line (1178). p is the app_psql line (1179), so hunk_hits [p,e] does not match an insertion after line ≤1178, and heading_added does not fire either. The check passes, and every Story ends up fighting over the same spot before the prep block. The in-file note ('この見出しより前の最後の節の後ろに書き足すと…落とす') is no longer true.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D11 — 再現: CH01 の後の木を main にした使い捨ての clone で、`feat/st50-x` が ST21 の最後の行（`tools/check-immutable.sh:1172`）の後ろに `echo "== 新しい検査"` を足すと、直す前の検査は rc=0、直した後は rc=1。守る範囲 1 を準備の頭の注記（`PREP`）の直前の空でない行から始める。`--self-test` に 3 件（最後の節と準備の間 → 落とす / 準備と読む口の間 → 落とす / 最後の節より前を直す → 通す）。D11 に追記

## R20. heading_added rejects any `# ---…STNN` or `echo "== STNN` line added anywhere in the body unless a heading with the same ST number is removed in the same diff. That also rejects the documented, allowed case of a Story adding its own sub-check inside an existing section.
- 成果物: tools/check-shared-appends.sh:52
- 根拠: ST22 and ST33 did exactly this inside the ST19 section of check-immutable.sh (`# --- ST22: 削除の台帳は追記のみ`, `# --- ST33: 書き出しの台帳は追記のみ`), and the script header says '本体の途中の節の中身を直すのは通す（ST47 が書庫の節に自分の検査を足す）'. A feat/st47 branch that adds `# --- ST47: 書庫の…` inside the ST12 section now gets 'NG …途中に Story の節の見出しを足した', a false CI failure.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: D2 は `# --- ST22: …` の型を見出しとして明示的に含めている（design.md:49）。本人の決定 Q2 は「新しい節の見出しを足したら落とす」で、ST 番号つきの小見出しで自分の検査を足すのは、既存の節の中でも新しい Story の節（`*.d/` に置くもの）。見出しを足さずに節の中身を直すのは通る（`--self-test` の「本体の途中の節を直す…のは通す」）。頭の注記の「ST47 が書庫の節に自分の検査を足す」が型の小見出しも許すと読めたので「見出しを足さずに」と書き直し、`--self-test` に「既存の節の中に `# --- ST47:` の小見出しを足しても落とす」を足した（rc=1 で ok）。反転は D2 の反転条件のとおり

## R21. On failure, post() resets the shared `sentNote.current` to the snapshot taken when the send started. Stay-row presses pass that same day-note snapshot, so a failed stay send can wipe out a day-note send that succeeded in the meantime.
- 成果物: web/src/FeelView.tsx:268
- 根拠: Press a stay row (restoreNote = sentNote.current = 'a'). While it is in flight, edit the note to 'b' and press a day valence: sentNote.current='b' and it is accepted. Then the stay send is rejected or unreachable and sets sentNote.current='a'. Leaving the note field compares 'b' with the stale 'a' and sends a duplicate day record.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.1 — 失敗したときにひとことの控えを戻すのは、日の欄の 1 件がまだ最後に押したもののときだけにした（滞在の行の失敗は控えに触れない）。vitest で 滞在の行（返事待ち）→ ひとことを書いて日の欄の「快」受理 → 滞在の行が断られる → 欄を離れる の順を固定し、日の記録を 2 度目に送らない（`posts` が 2 件のまま）。直す前の `FeelView.tsx` で落ちる

## R22. `app_psql` is now defined twice with the same body (lines 1179 and 1341), and db-roles plus grants.sql run twice. The second definition is redundant, and the two copies can drift apart.
- 成果物: tools/check-immutable.sh:1179
- 根拠: Someone later edits one definition (connection flags, role) and not the other. The .d sections and the app-role check then silently test under different connections. Only the re-run of db-roles/grants after DROP SCHEMA is needed, not a second function definition.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D11 — `tools/check-immutable.sh` の二度目の `app_psql()` を消し、`db-roles.sh` と grants.sql の当て直しだけを残した（`grep -n 'app_psql()'` は :1179 の 1 か所）。D11 に追記

## R23. The feeling branch maps `AuthoredBreach::OffsetOutOfRange` to MalformedFeeling, but by the code's own comment this arm can never be reached. That leaves dead code, and the shared rule's order (origin → external id → offset) does not match how feelings actually apply it.
- 成果物: crates/server/src/lib.rs:1062
- 根拠: Someone loosens the feeling +9:00 restriction above and expects the shared rule to keep checking the offset range. The order then quietly changes for feelings only, and authored_rule_tests have no test for an out-of-range feeling offset, so nothing would catch it. Either drop the arm with an `unreachable!` or split the offset check out of authored_breach.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 4.2 — 枝は match を閉じるために残し（届いても形の理由で断る側に倒す）、そのことを `lib.rs` のコメントに書いた。主観の順（地域 → 外部識別子）を `ch01_authored_feeling_offset_out_of_range_beats_external_id` で固定した（CT `ch01_authored_` 15 passed。主観の地域の検査を外す壊しで `left: "feeling_has_external_id"` と落ちる）

## R24. The refactor that introduced authored_breach was committed as `refactor(CH01): …` (774d28e4), but the project's commit rule allows only feat/fix/docs/chore.
- 成果物: crates/server/src/ingest.rs:117
- 根拠: Rule in /home/yosis/dev/ashiato2-ch01/CLAUDE.md, section 'git': 'コミットは `feat:` / `fix:` / `docs:` / `chore:` + 日本語 1 行'. Commit 774d28e4 'refactor(CH01): 人が書いた記録の規則を 1 つの関数にまとめる' uses a prefix outside that list.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 再現した（`git log --format=%s -1 774d28e4` が `refactor(CH01): …`）。直すには 774d28e4 より後の 40 本ほどの commit を書き換えることになり、`review/code.md`・`fix report`・`provisional.jsonl` が引く SHA（9a3d1a03 ほか）がすべて指す先を失う。中身の規則（`.githooks/pre-commit` は接頭辞を見ない）の違反は 1 行の件名だけで、振る舞いも検査も変えない。以後の commit は `feat` / `fix` / `docs` / `chore` に限る


## finish の fix（84acd2e..73530d7）の scoped re-review

席: re-reviewer（SDD の `re-review-prompt.md`。finish を書いた文脈から独立）。入力は `.superpowers/sdd/ch01-finish-fix.diff` と上の R10〜R24 の `処置:`。
判定: R10〜R19・R21〜R23 は ADDRESSED、R20・R24 は rejected の理由が成り立つ。R11 の kind の付け替え（premise → technical）は妥当。fix の diff に新しい Critical / Important は無し。
re-reviewer が自分で走らせたもの: vitest `FeelView.test.tsx` 40 passed・`check-shared-appends.sh --self-test` rc=0。

fix は 1 回だけなので、残った Minor 2 件は直さずに park する（どれも振る舞いの決定を変えない）:
- parked: `tools/stack.sh` の `newer_src` がサーバの作り直しで見るのは `crates/server` `migrations` `Cargo.*` だけで、サーバが依存する別の crate を直しても作り直さない。
  Ruling: 今のサーバは workspace の別の crate に依存していない（`crates/server/Cargo.toml` に path 依存なし）ので、今は起きない。依存を足す Story が監視の対象も足す
- parked: `FeelView.tsx` の受理された枝の `if (p.key === "day") sentNote.current = p.note;` は、最後に押した 1 件かを見ない。古い日の送り直しが、より新しい押下の返事待ちの間に受理されると、控えが古いひとことになる。
  Ruling: 送り直しは「もう一度送る」を押したときだけで、押すと pending の最新の 1 件しか送らない。その間に新しく押した 1 件が返事待ちで残る窓はほぼ無く、起きても欄を離れたときに同じひとことをもう一度送る（1 件増える）だけで、値は失われない
