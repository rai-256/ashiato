# CH05 spec のレビュー（`/story-upstream CH05 spec --draft`）

対象: `openspec/changes/ch05-test-db-bloat/` の proposal.md・specs/test-database/spec.md・design.md・tasks.md、`docs/changes/CH05.md`。
前提の資料: deep.md（第 1 回。答え待ち・推奨で読んだ）・deep-questions.json・premises.md・review/deep.md。
CH04 は `/home/yosis/dev/ashiato2-ch04`（`feat/ch04-test-db-growth` e27a521a）の `crates/server/src/testdb.rs`・`tools/check-test-db-growth.sh`・design / spec を読んだ。
DB は `docker exec ashiato2-ch04-db-1 psql -U ashiato -d postgres` で読むだけ（SELECT のみ）。

## 機械の検査

```
$ openspec validate ch05-test-db-bloat --strict
Change 'ch05-test-db-bloat' is valid                                   rc=0

$ python3 scripts/check_chain.py .
chain: OK (0 件 / 未回収 0 件 / warn 0 件)                              rc=0

$ python3 scripts/check_scenarios.py . ch05-test-db-bloat
scenarios: FAIL (担保なし 18 件 / 名無しの確認待ち 0 件)                 rc=1
  （18 件は CH05 の Scenario 全部。spec --draft の段では試験がまだ無いので想定どおり。
    18 件とも tasks.md のどれかの Task が一字一句で名指していることを grep -F で確かめた → 指摘にしない）

$ python3 scripts/review_triage.py . ch05-test-db-bloat
  [FAIL] deep.md R1: escalated なのに deep.md に R1 が無い
  [FAIL] deep.md R3: escalated なのに deep.md に R3 が無い
  [FAIL] deep.md R5: escalated なのに deep.md に R5 が無い
  [FAIL] deep.md R6: escalated なのに deep.md に R6 が無い
triage: FAIL (4 件)                                                    rc=1   → R3

$ python3 scripts/evidence.py lint openspec/changes/ch05-test-db-bloat
[OK] 検証コマンド（ch05-test-db-bloat）                                  rc=0
```

## deep の各 Q の推奨の写り先（指摘ではなく、確かめた範囲）

| Q | 推奨 | 写り先 | ずれ |
|---|---|---|---|
| Q1 | CH04 の上に積み、CH04 が持たないものだけ | proposal Why・Impact、design Context・Non-Goals、tasks の冒頭・1.1、CH05.md 価値・壊してはいけないもの | `requires: [CH04]` が front matter に無い（R2） |
| Q2 | 検証の前に大きさを測り、閾値（64 MB）を超えたら作り直す | spec Requirement 1・D1・D4・tasks 1.3・2.1・2.2 | 「64 MB」→「64 MiB」、既定値が Scenario に無い（R11） |
| Q3 | 回の助言ロックか接続があれば見送る。判定と作り直しは「1 つの取引の中で」先にロック | spec Requirement 2・D2・tasks 1.3 | 取引ではなく session の助言ロック（`DROP DATABASE` は取引の中で打てないと D2 が理由を書いている。意図どおりで指摘にしない）。組み方が無い（R7）、鍵の database（R8） |
| Q4 | 中の `ashiato` database だけ | spec Requirement 1 本文・D2・tasks Global Constraints・1.3 の grep、CH05.md:43 | なし |
| Q5 | `-st`・`-ch`・`-up-*` だけ | spec Requirement 1・Scenario「作業ツリーの名前から…」・D3・tasks 1.2 | なし（規則が 2 つになる点は R15） |
| Q6 | 量で見る・時間は記録だけ・殺す時点は 2 つ・完了の条件を書き直す | spec Requirement 3・D6・tasks 3.1・4.1・4.2、CH05.md:38-41 | 「大きさが同じ」→「1 MiB 以内」（R16） |
| Q7 | どれも入れない（(1) は申し送り） | proposal やらないこと・design Non-Goals・CH05.md 壊してはいけないもの | なし（申し送りが下流の DB の前提になる点は R1） |

---

## R1. 下流の作業ツリーは、いまのままでは自分の DB を立てられない（ネットワークの割り当てが尽きている）。tasks の DB を使う検証がすべてこれに乗るのに、前提として書かれていない
- 成果物: openspec/changes/ch05-test-db-bloat/tasks.md
- 根拠: `premises.md:15`（この worktree で `tools/verify-env.sh` が `all predefined address pools have been fully subnetted` で落ちた）/ いま `docker network ls` は 32 本（premises と同じ。読むだけで確かめた）/ 下流の作業ツリー `ashiato2-ch05` は `git worktree list` に無く、立てるときに新しい compose の project（新しいネットワーク）が要る / `tasks.md:9`（DB を使う検査は `./tools/db.sh up -d --wait db && ./tools/db-roles.sh` が前提）・1.3・2.1・2.2・3.1・4.2・6.3 の検証がこれに乗る / ネットワークの片付けは Q7・design Non-Goals（`design.md:26`）で別の change へ申し送り、proposal の Impact（`proposal.md:40-56`）と `docs/changes/CH05.md` には「下流の DB が立たない」ことへの言及が無い
- kind: premise
- 提案: tasks 1.1 の前提に `tools/verify-env.sh` rc=0（この作業ツリーの DB が立つ）を足し、落ちたら「申し送った片付けの change を先に通すか、人間がコンテナの無いネットワークを外す」と書く。proposal の Impact の「依存」にも 1 行足す（申し送り先の change が CH05 の下流より先に要るかを決める）
- 処置: escalated （第 1 回の Q7 の context に依りを書き足し、本人が (1) を選び直せるようにした。deep.md の Q7 の節に R1 を書いた。あわせて tasks 1.1 の前提に `tools/verify-env.sh` rc=0、proposal の Impact に下流の前提を書いた）

## R2. `docs/changes/CH05.md` の front matter が `requires: []` のまま。tasks 1.1 の検証と archive の順が、これに依っている
- 成果物: docs/changes/CH05.md
- 根拠: `docs/changes/CH05.md:5`（`requires: []`）/ `proposal.md:49`（`requires: [CH04]` は答えの後に record が書く）/ `scripts/story_deps.py:262-263`（`take` は requires に無い依存を断る）・`:429-432`（`included` は deps.json に記録が無いと FAIL）→ `tasks.md:34` の `story_deps.py included CH05 CH04` が通らない / CH05 の `test-database` は `## Purpose` を持たない ADDED（`specs/test-database/spec.md:1`）で、CH04 より先に archive されると正典に Purpose の無い capability ができる（順を守るのは requires だけ）
- kind: technical
- 提案: 推奨（Q1）で書いた版なので、spec --draft の段で `requires: [CH04]` を CH05.md に入れておく（Q1 の答えが違えば spec --answers が外す）か、record の処置の必須項目として deep.md に書く。どちらにしても「下流の入口の前に要る」ことを tasks 1.1 に書く
- 処置: fixed deep.md （Q1 の節に「答えの後に record が `requires: [CH04]` を書く」。questions の段の review/deep.md R1 の処置どおり、front matter は答えの後に書く —— いま書くと答えの前に admission が「proposal まで」に変わる。tasks 1.1 にも requires に依ることを書いた）

## R3. `review_triage.py` が FAIL: review/deep.md で escalated にした R1・R3・R5・R6 が deep.md に名指されていない
- 成果物: openspec/changes/ch05-test-db-bloat/deep.md
- 根拠: `python3 scripts/review_triage.py . ch05-test-db-bloat` → `[FAIL] deep.md R1/R3/R5/R6: escalated なのに deep.md に R<n> が無い`（rc=1）/ `review/deep.md:37,51,66,74` の処置は escalated（Q1・Q1・Q5・Q7 にした）/ `deep.md:5-38` に R 番号の引用が 1 つも無い
- kind: technical
- 提案: deep.md の Q1・Q5・Q7 の節に、どの指摘から来た問いか（`review/deep.md R1・R3` / `R5` / `R6`）を 1 行ずつ書く。R3 の「壊してはいけないものを CH04 から引き継ぐ」は CH05.md に入った（`docs/changes/CH05.md:72-73`）ので、その旨も Q1 の節に
- 処置: fixed deep.md （Q1・Q5・Q7 の節に出所の R を書いた）

## R4. tasks 4.3 の検証が、言っていること（片付けを外すと rc=1 で名前が出る）を確かめていない。CH05.md の完了の判定はこれを根拠にしている
- 成果物: openspec/changes/ch05-test-db-bloat/tasks.md
- 根拠: `tasks.md:78-80`（本文は「rc=1 で殺した回の database の名前が出ることを見てから、元に戻す」、検証は `git diff --quiet HEAD -- crates/server/src/testdb.rs` だけ —— 4.3 を走らせずにチェックしても rc=0）/ `docs/changes/CH05.md:90`（「CH04 の片付けを外すと同じ台本が rc=1 で落ちる（tasks 4.3）」）/ 結果は「計測の記録」に書くだけで、それを見る検証も無い
- kind: technical
- 提案: 1 つの `bash -c` にまとめる: `trap 'git checkout -- crates/server/src/testdb.rs' EXIT` を張り、`sweep_previous_runs(&mut admin).await;` の行を sed で外し、`tools/check-test-db-killed.sh` を走らせて `rc=1` かつ出力に `ashiato_run_` が出ることを判定する（commit しない、はそのまま）。復元の `git diff --quiet` は後ろに残す
- 処置: fixed 6.2 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 4.3）（`trap` で戻す 1 つの `bash -c` で、片付けの行を外して rc=1 と `ashiato_run_` を判定する）

## R5. tasks 5.3 の検証はいまでも rc=0（空振り）。測る場所（CI）も下流が待てる場所か書かれていない。design の計測の参照先の Task 番号が古い
- 成果物: openspec/changes/ch05-test-db-bloat/tasks.md
- 根拠: `tasks.md:92` の `grep -A3 "^## 計測の記録" … | grep -qv "下流が tasks"` をいまの design で走らせて rc=0（`grep -v -q` は見出しの行そのものが当たらないので常に通る）/ 測るのは CI の rust job の伸び（`tasks.md:91`）で、PR が push された後でしか取れない（下流がいつ push できるかは書かれていない）/ `design.md:145`（「下流が tasks 3.2・6.3 で書く」）・`design.md:92`（「tasks 3.2」）・`design.md:136`（「tasks 6.3 で CI の時間を測って書く」）—— tasks に 3.2 は無く、計測を書くのは 2.2・4.2・4.3・5.3
- kind: technical
- 提案: 計測の記録の各項目に見出しや鍵（例 `- 作り直しの判定:` `- 次の回の秒数:` `- 片付けを外した回:` `- CI の伸び:`）を design に先に置き、検証は「各鍵の行が空でない」を `grep -E '^- CI の伸び: [0-9]'` のように見る。CI の伸びが PR の後でしか取れないなら、手元で 3 行の所要時間を測る形にするか、「人間の確認待ち」でなく merge gate の後の記録だと書く。design の 3.2・6.3 を 2.2・4.2・4.3・5.3 に直す
- 処置: fixed 7.3 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 5.3）（design の計測の記録に鍵を先に置き、2.2・4.2・5.3 は鍵の行に数字があることを見る。5.3 は CI でなく手元で 3 行の時間を測る）

## R6. 「検証の準備は、作り直しの判定を呼んでから終わる」の THEN が「作り直したか見送ったか」の両方を通すので、作り直しが起きない検証の準備でも緑になる。CH05.md と tasks 6.3 は「作り直した ashiato の上で」と言っている
- 成果物: openspec/changes/ch05-test-db-bloat/specs/test-database/spec.md
- 根拠: `spec.md:41-44`（閾値 0・Story の作業ツリーで走らせ、THEN は「作り直したか見送ったかの行が出る」）/ `tasks.md:53`（`--verify-env-check` も同じ either/or）/ `docs/changes/CH05.md:95`（「server の試験の全体が作り直した `ashiato` の上で通り（tasks 6.3）」）・`tasks.md:101-102`（6.3 は `--verify-env-check` の後に cargo test）—— 見送ったときも rc=0 なので、6.3 は作り直していない `ashiato` の上でも通る / 閾値 0 で接続の無い作業ツリーなら、正しい結果は「作り直した」だけ
- kind: technical
- 提案: Scenario を「接続も回も無い状態で、閾値 0 で走らせると、`ashiato` を作り直したことが標準エラーに出て、走らせる前に置いた印（表か行）が無い」にする。`--verify-env-check` は見送りなら理由を出して rc≠0（rc=2 の「種類が違う」とは分ける）
- 処置: fixed design.md （Q8 の後の spec（2026-10-25）で作り直し・見送りを外したので、この指摘の対象（旧 2.2）は design に無い。Non-Goals に書いた。元の処置: fixed 2.2 （spec の Scenario を「検証の準備は、膨れた ashiato を作り直してから終わる」にし、印の表が消えることを見る。見送りは rc=1。D5））

## R7. design D2 の「1 つの session」を psql でどう組むかが書かれていない。分けて打つと手順 3 の鍵が手順 6 の前に外れる
- 成果物: openspec/changes/ch05-test-db-bloat/design.md
- 根拠: `design.md:44`（「1 つの session の中で次を順に行う」）・`:48-54`（手順 3〜8 は前の結果で分かれる: 取れなければ見送り / 行があれば見送り / 55006 なら見送り）/ 助言ロックは session に付く（psql を 2 回起こすと 1 回目の終わりで外れる）/ `psql -c` に複数の文を渡すと 1 つの暗黙の取引になり、`DROP DATABASE` は取引の中で打てない / 55006 を見送りに変えるには、その文だけ `ON_ERROR_STOP` を外して `:LAST_ERROR_SQLSTATE` を `\if` で分ける形が要る（コンテナの psql は 17.11 で使える。`psql --version` で確かめた）/ CH04 の鍵の形そのもの（`pg_locks` に `classid = 4820253 AND objsubid = 2`）は ch04 の DB の `pg_locks` で `advisory|5|4820253|1338153147|2|t` と実在を確かめた（成り立つ）
- kind: technical
- 提案: D2 に組み方を 1 行書く: 標準入力から 1 つの psql に台本を流し（各文が自分の取引）、結果は `\gset` で受け、分岐は `\if`、DROP の前後だけ `\set ON_ERROR_STOP 0` / `1` にして `:LAST_ERROR_SQLSTATE` で 55006 を見る。判定の出力（見送りの理由）は psql の `\echo` か `\warn` で標準エラーへ
- 処置: fixed design.md （Q8 の後の spec（2026-10-25）で作り直し・見送りを外したので、この指摘の対象（旧 D2）は design に無い。Non-Goals に書いた。元の処置: fixed D2 （psql の組み方: 標準入力・`\gset`・`\if`・`ON_ERROR_STOP` の切り替えと `:LAST_ERROR_SQLSTATE`・`\warn`・`application_name`））

## R8. 「回を作っている途中」の鍵（4820252）は database ごとに分かれる。self-test の背景の psql がどの database で鍵を取るかが決まっておらず、本物の CH04 とぶつからない形でも self-test は通りうる。本物の回の鍵で見送ることを確かめる検査も無い
- 成果物: openspec/changes/ch05-test-db-bloat/design.md
- 根拠: CH04 は鍵を `postgres` database に繋いだ接続で取る（`ashiato2-ch04/crates/server/src/testdb.rs:274,281`（`postgres_url` で `pg_advisory_lock(4820252)`）・`:142-157`（回の鍵も `postgres`））。ch04 の DB の `pg_locks` の回の鍵は `database = 5`（`postgres`）/ 助言ロックは database ごとの名前空間で、`pg_try_advisory_lock(4820252)` は同じ database で取った鍵としかぶつからない（`pg_locks` を読む手順 4 は全 database を見るので影響しない）/ `design.md:44` は判定を `-d postgres` で打つと書くが、`design.md:89`（D5 の背景の psql）は繋ぐ先を書いていない —— 背景も判定も使い捨ての database に繋ぐ実装だと、self-test は通り、本物の CH04 の作成中は見送らない / spec の Requirement 2 の 3 つの Scenario（`spec.md:54-68`）はどれも self-test の偽の鍵でしか確かめない
- kind: technical
- 提案: D2・D5 に「判定の session も、self-test の背景の psql も `postgres` database に繋ぐ（CH04 と同じ。助言ロックは database ごと）」と書く。あわせて 4.2 に 1 つ足す: `committed` の探りが眠っている間（本物の回の鍵を持っている）に、閾値 0・種類 `st`・使い捨ての対象で `tools/test-db-rebuild.sh` を走らせ、「走っている回」の見送りが出ることを見る（新しいファイルは要らない）
- 処置: fixed design.md （Q8 の後の spec（2026-10-25）で作り直し・見送りを外したので、この指摘の対象（旧 D5）は design に無い。Non-Goals に書いた。元の処置: fixed D5 仮 （判定も self-test の背景も `postgres` database。D2・D6・spec にも書き、`committed` の眠りの間に本物の回の鍵で見送ることを確かめる））

## R9. design D2 の手順 5（接続の数）は、autovacuum などクライアントでない backend も数えるので、膨れた直後の `ashiato` で見送りが続きうる
- 成果物: openspec/changes/ch05-test-db-bloat/design.md
- 根拠: `design.md:51`（`pg_stat_activity` に `datname = '<対象>'` の接続が 1 本でもあれば見送る）/ PostgreSQL の文書の振る舞いとして、`pg_stat_activity` は autovacuum worker の行にも `datname` を持つ（実物では観測していない。ch04 の DB で読めたのは `backend_type` の列があり `client backend` の行が見えることまで）。行を多く入れた直後の database ほど autovacuum が走る / `DROP DATABASE` 自体は autovacuum を止めて進むので、手順 6 に任せれば足りる / 同じ族: `tests/server_startup.rs` は回の鍵を持たずに `ashiato` に繋ぎ直しを繰り返す（`ashiato2-ch04/crates/server/tests/server_startup.rs:47,55,168`）ので、接続が 0 本の間に作り直されると後半が移行の無い `ashiato` を見る（検証は直列なので起きにくいが、Risks に無い）
- kind: technical
- 提案: 手順 5 を `backend_type = 'client backend'` に絞る（自分の pid も除く）。Risks に「回の鍵を持たない `ashiato` の利用者（`tests/server_startup.rs`）は、同じ作業ツリーで検証の準備と並べると、接続の切れ目で作り直されうる」を 1 行足す
- 処置: fixed design.md （Q8 の後の spec（2026-10-25）で作り直し・見送りを外したので、この指摘の対象（旧 D2）は design に無い。Non-Goals に書いた。元の処置: fixed D2 仮 （`backend_type = 'client backend'` と自分の pid を除く。Risks に 1 行・spec の本文を「クライアントの接続」に））

## R10. design D6 の「殺す前に S0 を読む」は、探りが database を作った後（殺す直前）に読んでも字面は満たし、そのとき差が空になって必ず空振りで落ちる。探りの起こし方の `--exact` の名前も省略されている
- 成果物: openspec/changes/ch05-test-db-bloat/design.md
- 根拠: `design.md:106`（「殺す前に、一覧 S0 …を読み、殺した後に一覧 S1 を読む。殺した回の database = S1 にあって S0 に無いもの」）—— 探りは `testdb::pool()` の初回で `<id>_tpl`・`<id>`、`isolated()` で `<id>_x_…` を作る（`ashiato2-ch04/crates/server/src/testdb.rs:271-311,422-441`）ので、印を待った後に S0 を読むと 3 つとも S0 に入る / `ashiato` の行数と大きさの「殺す前」も同じ曖昧さ / `design.md:104`（`--ignored --exact … --nocapture`）—— 試験は `lib.rs` のモジュールの下に入るので、`--exact` に渡す名前は `killed_probe_tests::ch05_killed_probe`（`ch05_killed_probe` だけだと 0 本で、印を待つ 120 秒の上限で落ちる）
- kind: technical
- 提案: D6 を「探りを起こす前に S0・`ashiato` の行数と大きさを読む」に直し、`--exact killed_probe_tests::ch05_killed_probe` と書く。待ちの上限で落ちたときは「探りが走らなかった（0 本・印が無い）」を出す
- 処置: fixed D2 仮 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 D6）（探りを起こす前に S0 を読む・`--exact killed_probe_tests::ch05_killed_probe`・印が来ないときの文言）

## R11. 本人の決定に入る数値が、Scenario に無い（実装が変えても spec が落ちない）
- 成果物: openspec/changes/ch05-test-db-bloat/specs/test-database/spec.md
- 根拠: 閾値の既定 64 MiB は Requirement の本文だけ（`spec.md:5`）で、Scenario は「閾値を超える」「閾値以下」としか言わない（`spec.md:14-22`）。deep の推奨は「64 MB」（`deep-questions.json` の Q2 の推奨の選択肢の detail。`deep.md:13` の答えの行は数値を持たない）、design・spec は「64 MiB」（`design.md:36`。67,108,864 バイト）で単位もずれる / 体積の 20,001 件は Requirement の本文だけ（`spec.md:72`）/ 印を待つ上限 120 秒は design だけ（`design.md:105`）/ 「待たずに終わる」の 10 秒は tasks だけ（`tasks.md:44`）
- kind: technical
- 提案: 「閾値の既定は 64 MiB で、64 MiB ちょうどは作り直さず、それを超えれば作り直す」の Scenario を 1 本足す（`TEST_DB_REBUILD_LIMIT_MB` を渡さずに、境界の大きさを偽の測りで渡す形でよい）。「回を作っている途中なら」の THEN に「10 秒以内に」を入れる。deep の Q2 の推奨の「64 MB」は record で「64 MiB」に揃える
- 処置: fixed design.md （Q8 の後の spec（2026-10-25）で作り直し・見送りを外したので、この指摘の対象（旧 D1）は design に無い。Non-Goals に書いた。元の処置: fixed D1 仮 （spec に「閾値の既定は 64 MiB で、ちょうどは作り直さず、超えれば作り直す」を足し `--decide` で確かめる。「回を作っている途中なら」に 10 秒以内。64 MB を 64 MiB として読んだ。record で答えの文に揃える））

## R12. Requirement の本文だけが言っていて Scenario が言っていない振る舞いがある
- 成果物: openspec/changes/ch05-test-db-bloat/specs/test-database/spec.md
- 根拠:
  - `spec.md:50`「消そうとしたときに PostgreSQL が『使われている』と断った」→ 見送り —— Scenario が無い（手順 5 と 6 の間に繋いだ接続。`design.md:52`）
  - `spec.md:51`「判定から作り直しの終わりまで…助言ロックを持ち、その間に新しい回が database を作り始めないようにする」—— Scenario が無い
  - `spec.md:6`「同じ DB にある他の database には触らない」—— Scenario が無い（CH04 の回の database が同じクラスタにある）
  - `spec.md:74`「`ashiato` の大きさが殺す前から 1 MiB を超えて増えていない」の外れ —— self-test の Scenario は行数の増え（`spec.md:95-97`）だけで、大きさだけが増えたときに落ちることを確かめる Scenario が無い
- kind: technical
- 提案: 4 本足す: 「DROP が 55006 で断られたら見送る」（判定の DROP の前で止まる差し込み口を self-test に持たせるか、`TEST_DB_REBUILD_*` で DROP の文を偽に替える）/「作り直しの間は、別の接続の `pg_try_advisory_lock(4820252)` が取れない」/「作り直しの後も、同じ DB の別の database の行は残る」/「`--self-test` の偽の統計で、行数が同じで大きさだけ 1 MiB を超えて増えれば rc=1」
- 処置: fixed design.md （Q8 の後の spec（2026-10-25）で作り直し・見送りを外したので、この指摘の対象（旧 D5）は design に無い。Non-Goals に書いた。元の処置: fixed D5 仮 （spec に「判定の後に繋いだ接続があれば…」「作り直しの間は、別の接続が回の作成の鍵を取れない」「大きさだけが 1 MiB を超えて増えていれば落ちる」を足し、別の database の行が残ることを 1 本目の AND にした。`TEST_DB_REBUILD_PAUSE_S`））

## R13. design にしかない、外から見える振る舞いがある（archive で正典に入らない）
- 成果物: openspec/changes/ch05-test-db-bloat/design.md
- 根拠:
  - `design.md:79-80`（D4）: 作り直しの台本が rc≠0（測れない・見送り以外のエラー・消した後に作れない）なら検証の準備も rc≠0 で終わる（ハーネスは BLOCKED_INFRA）。spec の Requirement 1（`spec.md:5-11`）は失敗のときを言わない
  - `design.md:105`（D6）: 印を 120 秒待って来なければ rc=1
  - `design.md:106`（D6）: S1 と S0 の差に、印のファイルに書いた名前が入っていることを見る
  - `design.md:109-110`（D6）: 判定の順（最初に外れたもので落ちる）と、落ちた時点の名前（`insert` / `committed`）を出すこと
- kind: technical
- 提案: 少なくとも D4 の「作り直しの判定が失敗すれば、検証の準備は理由を標準エラーに出して rc≠0 で終わる」を Requirement 1 の本文と Scenario（self-test で測りを失敗させる）に移す。D6 の 3 つは Requirement 3 の本文へ移し、120 秒の待ちと時点の名前は self-test の Scenario にする
- 処置: fixed D2 仮 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 D6）（spec の Requirement 1 に判定の失敗 → 検証の準備も rc≠0 と Scenario、Requirement 3 に判定の順・空振り・120 秒の待ち・時点の名前と Scenario「探りの印が来なければ…」）

## R14. tasks 6.2 の検証は、CH04 が先に archive されると deps.json が無く落ち、CH04 の SHA からの差分には後で取り込んだ main の変更も入る。「壊してはいけないもの」の一部を見ていない
- 成果物: openspec/changes/ch05-test-db-bloat/tasks.md
- 根拠: `tasks.md:100`（`json.load(open("…/deps.json"))["CH04"]["sha"]`）/ `scripts/story_deps.py:421-427`（`included` は依存が archive 済みなら deps.json の記録が無くても OK を返す）→ 1.1 は通り 6.2 だけが FileNotFoundError / KeyError / `git diff --quiet "$sha" --` は作業ツリーと CH04 の SHA を比べるので、CH05 が main を取り込んだ後は、その間に他の Story が `tools/smoke.sh`・`migrations/` などに入れた差分で落ちる / `docs/changes/CH05.md:74`（「試験の期待値（assert）・`// Scenario:` の印・試験の名前と本数」）を見る検証がどこにも無い（CH05 が触ってよい試験のファイルは `killed_probe_tests.rs` と `lib.rs` の 1 行だけ）
- kind: technical
- 提案: 基準を「CH04 を含む `origin/main` との merge-base（CH04 が未 merge なら deps.json の SHA）」にする小さな式にし、比べ方を `git diff --name-only <基準>...HEAD` の許可リスト（`tools/test-db-rebuild.sh` `tools/check-test-db-killed.sh` `tools/verify-env.sh` `tools/ports.sh` `crates/server/src/killed_probe_tests.rs` `crates/server/src/lib.rs` `.github/workflows/ci.yml` `docs/` `openspec/changes/ch05-test-db-bloat/`）に替える。`lib.rs` の差分が 1 行の追加だけであることも見る
- 処置: fixed 8.2 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 6.2）（CH05 の commit を題の `(CH05)` で拾い、触ったファイルの許可リストと `lib.rs` が追加だけかを見る。commit の題の規則を Global Constraints に）

## R15. 作業ツリーの種類の規則が `tools/ports.sh` の中で 2 つになる（port の分岐と新しい関数）。ports.sh を変えるのに、既存の `tools/check-ports.sh --self-test` を tasks が走らせない
- 成果物: openspec/changes/ch05-test-db-bloat/design.md
- 根拠: `design.md:66-68`（D3: 関数 `ashiato_worktree_kind <根の名前> <main の名前>` を足し、port の分岐はいまのまま）/ いまの port の分岐は main の checkout を名前でなく「git dir と共通 dir が同じ」で見分け、`-st` は `^<main>-st0*([0-9]+)$`（`tools/ports.sh:48-57`）—— 名前だけの関数と同じ答えになる保証は、どの検証も持たない / `spec.md:11`（「規則は、port の規則と同じ 1 か所（`tools/ports.sh`）に置く」）は同じファイルであることしか言っていない / `tasks.md:37-38` の 1.2 の検証は関数の 2 例と `PORT:=` の行の差分だけ。CI は `tools/check-ports.sh --self-test` を走らせる（`.github/workflows/ci.yml` の rust job）が、tasks は走らせない
- kind: technical
- 提案: port の分岐を新しい関数の答え（`main`・`st`・`verify`・それ以外）で分ける形にして規則を 1 つにする（port の値は変えない）か、そうしないなら `tools/check-ports.sh --self-test` の一時の repo（本物の worktree）で `ASHIATO_WORKTREE_KIND` も確かめる。どちらでも 1.2 の検証に `tools/check-ports.sh --self-test` rc=0 を足す
- 処置: fixed design.md （Q8 の後の spec（2026-10-25）で作り直し・見送りを外したので、この指摘の対象（旧 D3）は design に無い。Non-Goals に書いた。元の処置: fixed D3 仮（port の分岐を種類で分ける形にして規則を 1 つに。port の値は変えない）・1.2（`tools/check-ports.sh --self-test` rc=0））

## R16. CH05.md・deep の推奨は「`ashiato` の行数と大きさが殺す前と変わらない（同じ）」、spec は大きさに 1 MiB の許しを持つ
- 成果物: docs/changes/CH05.md
- 根拠: `docs/changes/CH05.md:40`（「`ashiato` の行数と大きさが殺す前と変わらない」）・`:89`（同じ）/ `deep-questions.json` の Q6 の推奨の detail（「`ashiato` の行数と大きさが殺す前と同じ」）/ `spec.md:74,83,88`（「1 MiB を超えて増えていない」）・`design.md:113`（許しの理由）
- kind: conflict
- 提案: CH05.md の完了の条件と完了の判定を「行数は同じ・大きさは 1 MiB を超えて増えない」に揃える。本人が推奨のまま答えたときに、答えた文と spec が違う形で残らないよう、record で Q6 の答えにも許しを書き添える
- 処置: fixed D2 仮 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 D6）（docs/changes/CH05.md の完了の条件と完了の判定を「行数は同じ・大きさは 1 MiB を超えて増えない」に揃え、許しの理由を書いた。record で Q6 の答えにも添える）

## R17. tasks 4.1 の印の数の下限（7）が、4.1 に割り当てた Scenario（5 本）より多い。2 本は 4.2 の分
- 成果物: openspec/changes/ch05-test-db-bloat/tasks.md
- 根拠: `tasks.md:70-73`（4.1 が名指すのは 5 本、検証は `grep -c "Scenario: " … -ge 7`）/ `tasks.md:74-75`（時点の 2 本の印は 4.2）/ `tasks.md:86`（CI の 1 本は 5.1）—— 書いた順に進めると 4.1 の検証が 4.2 の分を先取りしないと通らない。`grep -c` は行を数えるので、同じ印を 2 回書いても数が合う
- kind: technical
- 提案: 4.1 の下限を 5 にし、4.2 で 7、5.1 で 8 に上げる。数ではなく名指した Scenario の名前ごとに `grep -qF` する形にすれば、重複でも満たせない
- 処置: fixed 5.1 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 4.1）（数でなく、名指した Scenario の名前ごとに `grep -qF`。4.2・5.1 も同じ）

## R18. proposal の Impact の重なりが、走っているブランチと合わない
- 成果物: openspec/changes/ch05-test-db-bloat/proposal.md
- 根拠: `proposal.md:53`（ci.yml の rust job の重なりを「CH04・st09」と書く）—— `git diff --name-only origin/main...<branch>` で見ると、ci.yml を触るのは `feat/ch04-test-db-growth` と `feat/st29-satellite-credentials`（rust job の末尾に `check-satellite-boundary.sh` の 2 行）で、`feat/st09-media-capture` は origin/main から 0 commit 先 / `docs/testing.md` は `feat/st29-satellite-credentials` も触る（`git diff --stat` で 2 行の追加・1 行の削除）が、Impact（`proposal.md:46`）に重なりとして書かれていない / `crates/server/src/lib.rs` は st06-replay-task7・st22・st23・st29・st41 も触る（「走っている Story」とだけ書いてある。1 行の追加なので害は小さい）
- kind: technical
- 提案: ci.yml の重なりを「CH04・ST29（どちらも rust job の行の追加）」に直し、docs/testing.md の重なり（ST29）を足す
- 処置: fixed proposal.md （ci.yml は CH04・ST29、docs/testing.md は CH04・ST29、lib.rs は CH04・ST06・ST22・ST23・ST29・ST41）

## R19. `tools/test-db-rebuild.sh --self-test` の使い捨ての database は、self-test が殺されると誰も片付けない
- 成果物: openspec/changes/ch05-test-db-bloat/design.md
- 根拠: `design.md:88`（対象は `ch05_rebuild_probe_<乱数>`、`trap … EXIT` で消す。名前に `_tmp_` を入れない）/ `trap … EXIT` は SIGKILL では走らない / CH04 の片付けが消すのは `ashiato_run_<12 桁>` で始まるものと、`ashiato_run_` で始まらない `_tmp_` を含む名前だけ（`ashiato2-ch04/crates/server/src/testdb.rs:172-213`）なので、`ch05_rebuild_probe_*` は残り続ける —— CH05 が止めようとしている症状と同じ型
- kind: technical
- 提案: self-test の初めに、前の self-test が残した `ch05_rebuild_probe_*` を消す（この頭は CH05 の self-test だけが使う。並んだ self-test の分を消さないよう、self-test 自身も `postgres` で専用の助言ロックを持ち、取れたときだけ消す）。D5 にその 1 行と、Scenario に「前の self-test の残りは次の self-test の後に無い」を足すかを決める

---

観点 3（置き場）のうち `proposal.md` の Capabilities と `specs/` のディレクトリの一致: 該当なし（確かめた範囲: Capabilities は `test-database` だけで、`specs/test-database/` だけがある。change は `docs/stories/INDEX.md` の capability 表に載らず、`grep -n "CH05\|test-database" docs/stories/INDEX.md` は 0 件で前倒しも無い）。
`specs/` の実装の名前（台本の path・`core.event`・`ashiato_owner`）は、CH04 の `test-database` の spec と同じ書き方で、`check_scenarios.py` が名前で縛るので指摘にしない。

観点 5 の check_chain の観点 8: 該当なし（change は要件から来ないので観点 8 は無い。`check_chain.py` は rc=0）。依頼（逐語）の範囲: 依頼 1（殺した後の次の回を機械で確かめる）は Requirement 3、依頼 2（膨れたら機械が作り直す・使っている途中は作り直さない）は Requirement 1・2 が持ち、手順を覆したこと（`down -v` → database だけ）は `docs/changes/CH05.md:43` に書いてある。範囲を越える Scenario は無い。

観点 6: 該当なし（`spec --draft` の段で、`--answers` ではない）。
- 処置: fixed design.md （Q8 の後の spec（2026-10-25）で作り直し・見送りを外したので、この指摘の対象（旧 D5）は design に無い。Non-Goals に書いた。元の処置: fixed D5 仮（self-test の初めに、専用の助言ロック 4820254 を取れたときだけ `ch05_rebuild_probe_` の残りを消す））


---

## `spec --answers Q7` の段（2026-10-25）

## R20. 今日 main に入った CH06 が、CH05 と同じ `tools/verify-env.sh` で「毎回構わず作り直す」。CH05 の Requirement 1・2（膨れたら作り直す・使っている途中は見送る）は検証の準備で働かなくなり、向きも逆
- 成果物: openspec/changes/ch05-test-db-bloat/specs/test-database/spec.md・design.md（D1〜D5）・proposal.md（Impact の重なり）
- 根拠: `origin/main:docs/changes/CH06.md` の完了の判定（`HARNESS_VERIFY=1` のとき `tools/verify-env.sh` がこの worktree の DB を空から作り直す・切った接続の数を出す）/ `origin/main:openspec/changes/ch06-reset-verify-db/deep.md` Q2（構わず作り直す）・Q3（subnet を割り当てる）/
  `origin/main:openspec/changes/ch06-reset-verify-db/design.md` D5・D6（subnet の明示）・Non-Goals（消えた worktree の残りは片付けない）/ `git worktree list` に `ashiato2-ch06 [feat/ch06-reset-verify-db]` / `premises.md:11`（ハーネスは検証の前に毎回 `HARNESS_VERIFY=1` で `tools/verify-env.sh` を呼ぶ）/
  この上流の木は origin/main から 20 commit 遅れていて、手元に `openspec/changes/ch06-*` が無い（`spec_overlap.py` は origin/main を読むが、CH06 の capability は `verify-environment` で Requirement が重ならないので候補に出ない —— R22）/ `proposal.md` の Impact は「`tools/verify-env.sh`・`tools/ports.sh` を走っている feat/* で触るものは無い」と書いていた
- kind: premise
- 提案: CH05 の範囲を本人に問い直す（Q2〜Q5 は本人の答えで、この段で変えない）。推奨は「殺した後の検査（Requirement 3）と片付け（Requirement 4。Q7）だけを残し、`requires: [CH04, CH06]`」。片付けは CH06 の Non-Goals を埋めるが、CH06 の Requirement の本文（ほかの project のコンテナに触らない）とは食い違う（R21）
- 処置: escalated （第 2 回の問い Q8 にした。`deep-questions-r2.json`・`deep.md` の「第 2 回」に `(未回答)` で書いた。Q7 の片付け（D9）は CH06 の self-test の compose の project を外さない形にし、proposal の Impact の重なりに CH06 を書き足した）

### 機械の検査（`spec --answers Q7` の段。2026-10-25、作業ツリーの差分の上で）

```
$ openspec validate ch05-test-db-bloat --strict
Change 'ch05-test-db-bloat' is valid                                   rc=0
$ python3 scripts/evidence.py lint openspec/changes/ch05-test-db-bloat
[OK] 検証コマンド（ch05-test-db-bloat）                                 rc=0
$ python3 scripts/review_triage.py . ch05-test-db-bloat
指摘 32 件 / 仮決め 9 件 / 要件へ戻すもの 0 件  triage: OK             rc=0
$ python3 scripts/spec_overlap.py check . ch05-test-db-bloat
{"ok": true, "candidates": 0}                                          rc=0
$ python3 scripts/spec_overlap.py candidates . ch05-test-db-bloat
[]
$ python3 scripts/deep_draft.py status openspec/changes/ch05-test-db-bloat
{"state": "diverged", "diverged": ["Q7"], "pending": [], "premise": [], ...}   rc=0
  （answers.md を書く前。第 1 回だけを見る —— 第 2 回の Q8 は `deep_draft.py later` の側で、いまは答えが無く ids: []）
$ python3 scripts/check_scenarios.py . ch05-test-db-bloat
scenarios: FAIL (担保なし 32 件 / 名無しの確認待ち 0 件)               （下流の前なので想定どおり。32 本 = Global Constraints の本数と一致）
$ python3 scripts/check_chain.py .
chain: OK (0 件 / 未回収 0 件 / warn 0 件)
```

観点 7: 該当なし（`spec_overlap.py candidates` が 0 件）。0 件なのは、`spec_overlap.py` が `git ls-tree origin/main openspec/changes/`（58 行目）で CH06 を読んだうえで、CH06 の delta が `verify-environment` で CH05 の `test-database` と Requirement が重ならないから。
重なりは Requirement の名前ではなく `tools/verify-env.sh` の振る舞いの上にある（R20・R21）。requires に CH06 を足すかは Q8 の答えで決まるので、ここで「前提」の行を立てない。

確かめた範囲（指摘ではない）: 片付けの条件を実物の docker と突き合わせた（読むだけ。2026-10-25）。`ashiato2-*` の compose のコンテナ 26 個はすべて `com.docker.compose.project` と `…project.working_dir` の印を持ち、
path が無く `git worktree list` にも無いのは 15 個（うち running は ch01・ch02・st13・st33 の 4 つ）で `premises.md:16` と一致。`ashiato2-st22-rA` の project は `ashiato2-st22-ra`（小文字）。
DB のコンテナの mount は `volume <64 桁の id> /var/lib/postgresql/data`（名前の無い volume。`postgres:17-alpine` の `Config.Volumes` が `/var/lib/postgresql/data`）、volume は 98 個で `premises.md:17` と一致。
compose のネットワークの印は `com.docker.compose.project`・`…network`・`…version`・`…config-hash` だけで working_dir を持たない（spec がネットワークを名前で見るのは実物どおり）。
main の project は `ashiato2`（`<main>-` で始まらない）、`ashiato2/.claude/worktrees/agent-*` の作業ツリーは親のフォルダが違い、`lifeLog2` の project は `lifelog2`・`agent-…` で、どれも外れない。
`ashiato2-verify-env`（`-verify` で終わらない生きた作業ツリー）は path があるので外れない。

## R21. CH06 の Requirement の本文は「検証の準備は、ほかの compose project のコンテナには触らない」と言い、CH05 の Requirement 4 は同じ検証の準備で、ほかの project のコンテナを止めて外す。両方が archive されると正典が矛盾する。R20 の「片付けは CH06 と重ならない」は誤り
- 成果物: openspec/changes/ch05-test-db-bloat/specs/test-database/spec.md:147-157・review/spec.md R20 の提案・deep-questions-r2.json の Q8 の context・proposal.md:60-61
- 根拠: `git show origin/main:openspec/changes/ch06-reset-verify-db/specs/verify-environment/spec.md` の 1 つ目の Requirement（9〜13 行目）「WHEN 検証の準備（`tools/verify-env.sh`）が `HARNESS_VERIFY=1` で呼ばれ…THE SYSTEM SHALL …ほかの compose project のコンテナ・volume と、compose の外のコンテナ・volume には触らない」/
  CH06 design の Non-Goals（24〜25 行目）「消えた worktree の compose project（network 15 個・動いたままの DB 4 個）…の片付け（Q3 の推奨の「触らない」）。例外は D5 の 1 つだけ」/ CH05 spec.md:150-151（`<main>-` の project の、消えた作業ツリーのコンテナを動いていても止めて外す）・spec.md:147（検証の準備のたびに）/
  `premises.md:11`（ハーネスは検証の準備を毎回 `HARNESS_VERIFY=1` で呼ぶ —— CH05 の片付けは必ず CH06 の Requirement と同じ呼び出しの中で起きる）/ R20 の提案「片付けは CH06 の Non-Goals を埋めるので重ならない」・Q8 の context「CH06 は消えた作業ツリーの残りの片付けを持たない（CH06 の Non-Goals）」は、Non-Goals だけを読んで Requirement の本文を見ていない
- kind: conflict
- 提案: 第 2 回の問い Q8 の context に「CH06 の Requirement は検証の準備が他の project に触らないと言い、CH05 Q7 の片付けはそれを覆す（CH06 Q3 の推奨の「触らない」と CH05 Q7 の本人の答えが逆）」を足して一緒に問う。
  片付けを残すなら、CH05 は CH06 の `verify-environment` の Requirement を MODIFIED にして例外（消えた作業ツリーの `<main>-` の project）を書き足すか、片付けの Requirement を `verify-environment` の側に置き、`requires: [CH04, CH06]` にする（capability の置き場も CH06 が `tools/verify-env.sh` の capability を立てたので変わる。proposal.md:39 の「同じ `test-database` に置く」の理由を書き直す）
- 処置: escalated （Q8 の context と推奨の選択肢の detail に足して一緒に問う。`deep-questions-r2.json`。R20 の提案の「重ならない」も直した）

## R22. R20 の根拠「この上流の木は origin/main から 20 commit 遅れていて、`openspec/changes/ch06-*` が無い（`spec_overlap.py` は CH06 を見ない）」は誤り。`spec_overlap.py` は origin/main を読む
- 成果物: openspec/changes/ch05-test-db-bloat/review/spec.md（R20 の根拠）
- 根拠: `scripts/spec_overlap.py:12`（走っている change: origin/main の `openspec/changes/<lo>-*/`）・`:58`（`git ls-tree -r --name-only origin/main openspec/changes/`）/ `git ls-tree -r --name-only origin/main openspec/changes | grep ch06` に `ch06-reset-verify-db/specs/verify-environment/spec.md` がある / fetch の後も `spec_overlap.py candidates` は `[]`
- kind: technical
- 提案: R20 の根拠を「`spec_overlap.py` は CH06 を読むが、CH06 の delta は `verify-environment` で Requirement の名前が重ならないので候補にならない。重なりは `tools/verify-env.sh` の振る舞い（R21）」に直す。木が遅れていることは事実だが、候補が 0 件の理由ではない
- 処置: fixed review/spec.md （R20 の根拠を「origin/main を読むが capability が違うので候補に出ない」に直した）

## R23. Scenario 1 は「名前の無い volume つき」のコンテナを言うが、D9 の本物の docker の 1 件は「名前つきの volume」を挿す。名前つきの volume は `docker rm -v` でも消えないので、この 1 件は実装が `-v` を付けても緑になる
- 成果物: openspec/changes/ch05-test-db-bloat/design.md:165・specs/test-database/spec.md:160-163
- 根拠: `docker rm --help` の `-v, --volumes   Remove anonymous volumes associated with the container`（消すのは名前の無い volume だけ）/ design.md:165「self-test が作った名前つきの volume を挿す」/ spec.md:161 の WHEN「（名前の無い volume つき）」/
  実物の DB の volume は名前の無い volume（`docker inspect ashiato2-st13-db-1` の Mounts、`postgres:17-alpine` の `Config.Volumes` が `{"/var/lib/postgresql/data":{}}`）。本人の答え「volume は消さない」が守るのはこの名前の無い volume
- kind: technical
- 提案: 本物の 1 件は `postgres:17-alpine` の `VOLUME` が作る名前の無い volume のまま起こし、片付けの前に `docker inspect` で volume の id を控え、片付けの後に `docker volume inspect <id>` が rc=0 であることを見る形にする（D9 を直す）。後始末はその id を消す
- 処置: fixed D4 仮 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 D9）（本物の 1 件は image の `VOLUME` で付く名前の無い volume を使い、その id を控えて残ることを見る。後始末は控えた id だけ）

## R24. Scenario 1 の「volume の数は片付けの前と同じ」は、共有の docker では他の作業ツリーが volume を作る・消すので揺れる。しかも 1 つの THEN と 2 つの AND に、外したこと・volume が残ること・出力の 3 つを束ねている
- 成果物: openspec/changes/ch05-test-db-bloat/specs/test-database/spec.md:160-164
- 根拠: spec.md:163 / 同じ機械で走る CH06 は、`HARNESS_VERIFY=1` の検証の準備のたびに worktree の DB の volume を消して作り直す（`origin/main:openspec/changes/ch06-reset-verify-db/specs/verify-environment/spec.md` の 1 つ目の Requirement）。CH06 の self-test も sentinel の volume を作って消す（CH06 design の検査の本体 5.）。`docker compose up` も名前の無い volume を作る /
  本人の決定の中身（volume を消さない）が AND の 2 つ目にあり、外したことだけを見る self-test でも Scenario の印は付く
- kind: technical
- 提案: 「volume の数」を「控えた id の volume が残っている」（R23）に替え、volume が残ることを別の Scenario に分ける（題も「外したコンテナの volume は残る」にする）。出力の行は Requirement の本文の「1 つごとに名前と path を出す」の Scenario として分けてよい
- 処置: fixed specs/verify-environment/spec.md （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 specs/test-database/spec.md）（数ではなく控えた id の volume が残ることを見る。Scenario を「外される」と「名前の無い volume は残る」に分けた）

## R25. D9 の self-test の本物の 1 件は、本物の main の名前のまま片付けを走らせるので、self-test を走らせるたびに機械の本物の残り（いまは 15 個）も外す。tasks 0.1 の self-test が 0.2 の「外すものの一覧を写してから片付ける」より先に本物を片付けてしまい、一覧が空になる。他の作業ツリーの片付けが self-test のコンテナを先に外しても緑になる
- 成果物: openspec/changes/ch05-test-db-bloat/design.md:165-166・tasks.md:38-50
- 根拠: design.md:165「project `<main>-st97-<乱数>`・working_dir `<親のフォルダ>/<main>-st97-<乱数>`…を作り、片付けを走らせ」（`TEST_DOCKER_TIDY_MAIN` を使うとは書いていない —— 差し替えは design.md:164 で偽の docker の側だけに書かれている）/
  片付けの条件（spec.md:150-151）は実物の 15 個にも当たる（上の「確かめた範囲」）/ tasks.md:38-43 の 0.1 の検証が `tools/docker-tidy.sh --self-test` を走らせ、tasks.md:47 の 0.2 がその後で `--dry-run` の一覧を「計測の記録」に写す /
  design.md:166「別の作業ツリーの片付けが先に外していても、無いことと volume が残ることは同じに見える」—— 自分の片付けが壊れていても、並んだ別の作業ツリーの検証の準備が外せば self-test は通る（D8 の self-test は「使い捨ての名前だけを触る」と書いていて、形も揃わない）
- kind: technical
- 提案: 本物の 1 件は `TEST_DOCKER_TIDY_MAIN` を乱数の main（例 `<親のフォルダ>/tidyst-<乱数>`）にし、`TEST_DOCKER_TIDY_WORKTREES` を空のファイルにして、self-test が作った project（`tidyst-<乱数>-st97`）だけが条件に当たる形にする。
  これで本物の残りに触らず、他の作業ツリーの片付け（本物の main の名前で見る）も self-test のコンテナに当たらない
- 処置: fixed D4 仮 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 D9）（本物の 1 件は `TEST_DOCKER_TIDY_MAIN` で main を `$TMPDIR/ch05tidy<乱数>` に差し替える。本物の `ashiato2-*` の残りに当たらず、他の作業ツリーの片付けもこの 1 件を外さない）

## R26. `--dry-run` は「外すものを同じ形で出す」と言うが、D9 はネットワークを「コンテナを外した後に」読むので、dry-run はコンテナを外す結果で空くネットワーク（いまの ch02・st13・st33 の 3 本）を挙げない。tasks 0.2 が写す「片付けたもの」がそのぶん欠ける
- 成果物: openspec/changes/ch05-test-db-bloat/design.md:158-159・specs/test-database/spec.md:156,182-185・tasks.md:47-50
- 根拠: design.md:159「コンテナを外した後に読む（外したコンテナのネットワークも同じ回で空く）」/ spec.md:156「`--dry-run` は外すものを同じ形で出すだけで、何も外さない」/ 実物: `ashiato2-ch02_default`・`ashiato2-st13_default`・`ashiato2-st33_default` は消えた作業ツリーの動いているコンテナが繋がっている（`docker network ls`・`docker ps -a`）ので、外さない dry-run の時点では「繋がっているコンテナが無い」に当たらない /
  Scenario 6（spec.md:182-185）の WHEN「作業ツリーが消えたコンテナとネットワーク」は、そのネットワークに当のコンテナが繋がっているかを決めていないので、繋がっていない形で書けば dry-run の欠けは見えない。
  あわせて tasks.md:49 の `[ -z "$(tools/docker-tidy.sh --dry-run 2>&1)" ]` は、その前の `tools/verify-env.sh` との間に別の作業ツリーが消える（merge の後の片付け）と落ちる
- kind: technical
- 提案: D9 に「dry-run は、外すと判定したコンテナの印の project を『コンテナが無い』とみなしてネットワークを判定する」を書き、Scenario 6 の WHEN を「消えた作業ツリーの動いているコンテナと、それが繋がっているネットワーク」にして THEN に両方の名前を求める。
  tasks 0.2 の空の確かめは `tools/verify-env.sh` と同じ `bash -c` に束ねるか、落ちたら片付けをもう一度走らせて見る形にする
- 処置: fixed D4 仮 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 D9）（外すコンテナを決めた後、それを除いて空くネットワークも同じ回・`--dry-run` で挙げる。spec の本文と `--dry-run` の Scenario にも書いた。tasks 0.2 の空の確かめは片付けを走らせた直後に同じ `bash -c` で見る）

## R27. Scenario 8 の禁止の字面（「`-v` を含む `rm`」）は `docker rm --volumes` と `docker rm -fv` を拾わず、許す側（「一覧と印の読み出し」）はどの subcommand かが決まっていない
- 成果物: openspec/changes/ch05-test-db-bloat/specs/test-database/spec.md:192-194
- 根拠: spec.md:194 / `docker rm --help`（`-v, --volumes`。短い flag は `-fv` と束ねられる）/ design.md:156 は `docker ps -a … --format`、design.md:158 は `docker network ls`・`docker ps -aq --filter network=`、spec が名指さない `inspect` を使うかは実装次第
- kind: technical
- 提案: THEN を「書き留めた命令の subcommand は `ps`・`network ls`・`inspect`・`rm`・`network rm` のどれかで、`rm` の引数に `-v`・`--volumes`、または `v` を含む短い flag の束が無く、`volume`・`system`・`compose down` が無い」のように、許すものを列挙して禁止を引数の形で書く
- 処置: fixed specs/verify-environment/spec.md （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 specs/test-database/spec.md）（許す subcommand を `ps`・`inspect`・`network ls`・`network inspect`・`rm`・`network rm` に、`rm` の引数を `-f` と id だけに決めた）

## R28. Requirement の本文・CH05.md・tasks が言っていて、Scenario が言っていないものがある
- 成果物: openspec/changes/ch05-test-db-bloat/specs/test-database/spec.md:143-157
- 根拠:
  - CI: `docs/changes/CH05.md:98`「CI の rust job が 4 つの検査を走らせる」・proposal.md:48「4 行（片付けの self-test を含む）」・tasks.md 5.1 が `tools/docker-tidy.sh --self-test` を ci.yml に足す。CI の Scenario（spec.md:143-145）の THEN は 3 つだけを挙げ、tasks 5.1 の self-test の中の検査も「この 3 行」を見る
  - spec.md:156「外すものが無ければ何も出さない」—— tasks.md:49 の `[ -z "$(tools/docker-tidy.sh --dry-run 2>&1)" ]` と CH05.md:97 がこれに頼るが、Scenario が無い（実装が「外すものは無い」と 1 行出すと、0.2 が片付けた後でも落ちる）
  - spec.md:157「外すのに失敗したとき…rc≠0」—— Scenario 7 は「docker に問えない」だけ
  - spec.md:153-155 のネットワークの条件のうち「繋がっているコンテナがある」「印の無いネットワーク」「別の repo の project」で外さないこと —— Scenario 5 は空の・生きている・`-verify` の 3 つだけ（design.md:164 の偽の docker の場合分けには「コンテナの繋がったネットワーク」がある）
- kind: technical
- 提案: CI の Scenario の THEN に `tools/docker-tidy.sh --self-test` を足す（または片付けの Requirement に CI の Scenario を 1 本足す）。「外すものが無ければ標準エラーに何も出さず rc=0」「`rm` が落ちれば rc≠0 で理由が出る」「繋がったコンテナのある・印の無い・別の repo のネットワークは残る」を Scenario にする
- 処置: fixed specs/verify-environment/spec.md （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 specs/test-database/spec.md）（「外すものが無ければ何も出さない」「外す命令の失敗で rc≠0」「残すネットワーク（コンテナの繋がった・印の無い・別の repo）」「CI が片付けの self-test を走らせる」を Scenario にした。tasks 0.1・0.2・5.1 が名指す）

## R29. design D9 にしかない、外から見える振る舞い（並んだ片付けで `No such container` / `No such network` を外せたものとして rc=0）。archive で正典に入らない
- 成果物: openspec/changes/ch05-test-db-bloat/design.md:160
- 根拠: design.md:160 / spec.md:157 は「外すのに失敗したときは rc≠0」とだけ言い、D9 の例外は spec に無い —— spec だけを読んだ実装は `No such` で rc≠0 を返し、検証の準備が並ぶたびに「片付けに失敗した」の行が出る
- kind: technical
- 提案: Requirement の本文に「もう無いもの（`No such`）は外せたものとして扱う」を足し、偽の docker が `rm` に `No such container` を返す Scenario を 1 本足す
- 処置: fixed specs/verify-environment/spec.md （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 specs/test-database/spec.md）（Requirement の本文と Scenario「もう無いものを外そうとしたら、外せたものとして扱う」に移した）

## R30. 数と形が Q7 の写しに追いついていない（tasks の範囲の 1 行・5.3・Global Constraints の例外・D9 と tasks 0.1 の口・proposal の文の繋がり）
- 成果物: openspec/changes/ch05-test-db-bloat/tasks.md:7,26,121・design.md:152,167・proposal.md:61
- 根拠:
  - tasks.md:7「書くのは、作り直しの台本・検証の準備の 1 行・…・CI の 3 行・文書だけ」—— Q7 で片付けの台本・検証の準備の 2 行目・CI の 4 行目が入ったのに「だけ」のまま（下流は Task 0 と食い違う範囲の宣言を読む）
  - tasks.md:121 の 5.3 は「CI に足した 3 行の時間」「`- 3 行の時間:`」のまま 4 つ目を「同じ行に添える」。検証（tasks.md:122）は 4 つ目の秒数が無くても通る
  - tasks.md:26 の例外「self-test が自分で作った volume を消す後始末は除く」は `docker rm -f`・`docker network rm` の文に付いていて、前の文の `docker volume rm` を書かない、の例外になっていない。self-test の後始末（作ったコンテナ・ネットワークを trap で消す）も「片付けの本体にだけ書く」に反する
  - design.md:152 の口は「引数なし / `--dry-run` / `--self-test`」の 3 つで、Scenario 7 を `--self-test` の中で見る（design.md:167）。tasks.md:38 は `--verify-env-check` を 4 つ目の口にし、Scenario 7 を 0.2 の `--verify-env-check` で見る
  - proposal.md:61「…`<main>-` に絞って外さない（design D9）`tools/ports.sh` は…」—— 句点が無く、CH06 の文と `tools/ports.sh` の文が 1 つに繋がっている
- kind: technical
- 提案: tasks.md:7 に片付けの台本と検証の準備の 2 行目・CI の 4 行を足す。5.3 を「4 行の時間」にし、検証の grep を 4 つ目の秒数まで見る形にする。Global Constraints の例外を「`tools/docker-tidy.sh --self-test` が自分で作ったコンテナ・ネットワーク・volume の後始末」に書き直す。D9 の口に `--verify-env-check` を足すか tasks を D9 に合わせる。proposal.md:61 に句点を足す
- 処置: fixed tasks.md （範囲の 1 行・Scenario の数 36・5.1 の計 4 行・5.3 と計測の鍵「CI の行の時間」・Global Constraints の例外を self-test の後始末に限る・D9 と tasks 0.1 の口を 4 つに揃えた。proposal の句点も直した）

### R21〜R30 の直しの確かめ（2026-10-25）

```
$ openspec validate ch05-test-db-bloat --strict                        → valid  rc=0
$ python3 scripts/evidence.py lint openspec/changes/ch05-test-db-bloat  → [OK]   rc=0
$ python3 scripts/review_triage.py . ch05-test-db-bloat                 → triage: OK
$ python3 scripts/spec_overlap.py check . ch05-test-db-bloat            → {"ok": true, "candidates": 0}
$ python3 scripts/deep_draft.py status openspec/changes/ch05-test-db-bloat → "state": "diverged"（直しで仕様の digest が変わり、前の answers.md が古くなった。書き直した）
$ python3 scripts/check_scenarios.py . ch05-test-db-bloat               → 担保なし 36 件（下流の前。spec の `#### Scenario:` も 36 本で Global Constraints と一致）
$ python3 scripts/check_chain.py .                                       → chain: OK
```

閉じたもの: R21（Q8 の context・推奨の detail・deep.md の Q8 の出所と効く先に入った）・R22（R20 の根拠を直した）・R23（D9 の本物の 1 件が image の `VOLUME` の名前の無い volume を使い id で見る）・
R24（Scenario を「外される」と「名前の無い volume は残る」に分け、数でなく id で見る）・R25（main を `$TMPDIR/ch05tidy<乱数>` に差し替え、本物の `ashiato2-*` にも他の作業ツリーの片付けにも当たらない）・
R26（spec の本文・`--dry-run` の Scenario・D9 が「外すコンテナを除いて空くネットワーク」を挙げ、tasks 0.2 の空の確かめは片付けの直後に同じ `bash -c`）・R27（許す subcommand と `rm` の引数を列挙した）・
R28（4 つを Scenario にし、0.1・0.2・5.1 が名指す）・R29（本文と Scenario に移した）。
R30 は大半が閉じた（tasks.md:7・5.3 の 4 行と鍵・Global Constraints の volume の例外・proposal の句点）。残りは R34。
計測の鍵の行（design.md:199-201）は全角の「：」で始まる置き場で、tasks の `grep -qE "^- …: .*[0-9]"`（半角の「: 」）は置き場のままでは rc=1（実測）—— 空振りしない。

## R31. R25 の直しで、self-test の本物の 1 件（`ch05tidy<乱数>-st97` のコンテナ・ネットワーク・名前の無い volume）は、途中で落ちると誰も片付けない。本物の片付けは main の名前が違うので当たらず、Global Constraints は self-test がコンテナとネットワークを消す後始末を許していない
- 成果物: openspec/changes/ch05-test-db-bloat/design.md:166-169・tasks.md:25-27
- 根拠: design.md:168「main の名前が乱数なので…他の作業ツリーの片付けもこの 1 件を外さない」—— 同じ理由で、self-test が片付けの前・途中で落ちた（片付けが壊れている・殺された・`docker network create` の後で `docker run` が落ちた）ときの残りも、どの片付けも外さない。
  動いている `postgres:17-alpine` のコンテナと、既定の割り当てから取ったネットワーク（`premises.md:15` の枯渇の元）が、self-test を落とすたびに 1 つずつ溜まる /
  design.md:168 の後始末は「控えた id の volume を消す」だけ / tasks.md:25-27 は `docker rm -f`・`docker network rm` を「片付けの本体にだけ書く」とし、例外は self-test の `docker volume rm` 1 つだけ —— 下流は self-test に `trap` でコンテナとネットワークを消す行を書けない
- kind: technical
- 提案: D9 と Global Constraints の例外を「`--self-test` が自分で作ったもの（控えたコンテナの id・ネットワークの名前・volume の id）を `trap … EXIT` で消す後始末」に広げる。片付けを確かめる前に消さないよう、trap は片付けの後の確かめを終えてから、残っていれば消す形にする
- 処置: fixed D4 仮 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 D9）（self-test が作るものに印 `dev.ashiato.ch05-tidy-probe=1` を付け、trap で消し、落ちた残りは次の self-test の初めに印で消す。Global Constraints の例外をこの後始末に広げた）

## R32. 本物の 1 件の置き場 `$TMPDIR/ch05tidy<乱数>`（「作る」）は、`TMPDIR` の無い CI では `/ch05tidy<乱数>` になり、作れない
- 成果物: openspec/changes/ch05-test-db-bloat/design.md:166-167・tasks.md:30
- 根拠: design.md:166 `TEST_DOCKER_TIDY_MAIN=$TMPDIR/ch05tidy<乱数>`（作る）/ `.github/workflows/ci.yml` は `TMPDIR` を置かない（`grep -n TMPDIR .github/workflows/ci.yml` が 0 件。origin/main でも 0 件）。GitHub の runner は既定で `TMPDIR` を置かない（置くのは `RUNNER_TEMP`）/
  既存の台本は一時の置き場を `mktemp -d` で取る（`tools/check-ports.sh:15`・`tools/check-no-time-server.sh:53`・`tools/test-usage-volume.sh:28`。`mktemp` は `TMPDIR` が無ければ `/tmp` を使う）/ CI の rust job で走る（spec の「CI の rust job は、片付けの self-test を走らせる」）
- kind: technical
- 提案: 置き場を `d=$(mktemp -d)` にし、main を `$d/ch05tidy<乱数>`・working_dir を `$d/ch05tidy<乱数>-st97` にする（親のフォルダが同じになる）と D9 に書く。tasks.md:30 の「`"$TMPDIR"` の下に作る」も `mktemp -d`（`TMPDIR` が無ければ `/tmp`）と読める形にする
- 処置: fixed D4 仮 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 D9）（置き場は `mktemp -d` の下。`$TMPDIR` を直に使わない）

## R33. Scenario「片付けが docker に問えない・外せなければ、片付けは rc≠0 で落ち、検証の準備は続く」は、WHEN に 2 つの場合、THEN と AND に 3 つの主張を束ねている。印は 1 つなので、どれか 1 つを確かめただけでも担保が付く。置き場も design と tasks で違う
- 成果物: openspec/changes/ch05-test-db-bloat/specs/test-database/spec.md:201-204・design.md:165・tasks.md:44-46
- 根拠: spec.md:202 の WHEN（docker に問えない命令に替えて片付けと `tools/verify-env.sh`、続けて外す命令だけが失敗する命令に替えて片付け）・spec.md:203-204（どちらも rc≠0・理由が出る / 検証の準備は 1 行出して rc=0）/
  design.md:165 は「外す命令の失敗で rc≠0」を `--self-test`（偽の docker）の側に挙げ、tasks.md:46 は同じものを `--verify-env-check` の中で見るとし、印もそちらに置く
- kind: technical
- 提案: 「docker に問えなければ片付けは rc≠0 で理由を出す」「外す命令が（もう無い以外の理由で）失敗すれば rc≠0 で理由を出す」「片付けが落ちても検証の準備は 1 行出して DB を起こし rc=0」の 3 本に分け、前の 2 本は `--self-test`（偽の docker）、3 本目は `--verify-env-check` に置く。design と tasks の置き場を揃える
- 処置: fixed specs/verify-environment/spec.md （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 specs/test-database/spec.md）（3 本に分けた: docker に問えない（self-test）・外す命令の失敗（self-test）・検証の準備は続く（`--verify-env-check`）。tasks 0.1・0.2 の名指しも分けた）

## R34. R30 の残り: D9 の最初の箇条はまだ口を 3 つと言い、Risks はまだ「3 行の時間」と言う
- 成果物: openspec/changes/ch05-test-db-bloat/design.md:152,183
- 根拠: design.md:152「新しい台本 `tools/docker-tidy.sh`（引数なし = 片付ける / `--dry-run` / `--self-test`）」と design.md:170「口は 4 つ: …`--verify-env-check`」が同じ D9 の中で食い違う / design.md:183「tasks 5.3 で 3 行の時間を手元で測って書く」—— tasks 5.3 と計測の鍵（design.md:201）は「CI の行の時間」の 4 行
- kind: technical
- 提案: design.md:152 に `--verify-env-check` を足すか「口は下の 4 つ」に替える。design.md:183 を「4 行の時間（計測の記録の `CI の行の時間`）」にする
- 処置: fixed D4 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 D9）（152 行目の口を 4 つに、Risks の「3 行の時間」を「CI の 4 行の時間」に）

## R35. tasks 0.2 は、`tools/verify-env.sh` に片付けの 1 行を足してから外すものの一覧を写す順になっている。足した後は、検証の準備（`scripts/verify-run` が検証のコマンドの前に毎回呼ぶ）が本物を片付けるので、一覧を写す前に消えうる
- 成果物: openspec/changes/ch05-test-db-bloat/tasks.md:44-50
- 根拠: tasks.md:44（verify-env.sh に 1 行を足す）→ tasks.md:47（続けて `--dry-run` の出力を「片付けたもの」に写してから `tools/verify-env.sh` を走らせて片付ける）/ `tools/verify-env.sh:2`「ハーネス（scripts/verify-run）が検証コマンドの前に毎回呼ぶ」・`premises.md:11` /
  1 行を足した後に下流が `tools/verify-env.sh` か `scripts/verify-run` を一度でも走らせれば、本物の残り（いまは 15 個のコンテナと 3 本のネットワーク）は記録の前に外れ、記録の確かめ（`grep -qE "^- 片付けたもの: .*[0-9a-z]"`）は「なし」などの英数字の無い字面だと落ち、何かを書けば中身に依らず通る
- kind: technical
- 提案: 一覧を写すのを 0.1 の終わり（`tools/docker-tidy.sh` はあるが `tools/verify-env.sh` はまだ呼ばない時点）に移し、0.2 は 1 行を足して片付け、空になったことを見るだけにする
- 処置: fixed 2.1 （Q8 の後の spec（2026-10-25）で番号を振り直した。旧 0.1）（一覧を写すのを 0.1 の終わりに移し、0.1 の検証に置いた。0.2 は片付けて時間を測るだけ）

（R31〜R35 の直しはどれも Q7 の片付けの中身（Requirement 4 の Scenario・D9（仮）・tasks 0.1・0.2・Global Constraints の例外）で、答え以外の変更の判定は変わらない。review/answers.md の spec-digest は直しの後に呼び出し元が取り直した。）
