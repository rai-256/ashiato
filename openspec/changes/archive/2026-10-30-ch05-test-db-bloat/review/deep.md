# CH05 深掘りの問いの独立検査（deep-review）

対象: `openspec/changes/ch05-test-db-bloat/deep-questions.json`（Q1〜Q5）・`premises.md`
入力: `docs/changes/CH05.md`（satisfies: []・doors なし）、既存コード、各 worktree の DB（docker の DB は読むだけ。`SELECT` と `pg_stat_activity` のみ）。
手順 1〜8 を問いの一覧を見る前にやり、その後で突き合わせた。

## 手順の記録

1. **要件どうしの衝突** —— CH05 は `satisfies: []`。要件の衝突は無い。代わりに、**走っている change CH04**（`feat/ch04-test-db-growth`、`/home/yosis/dev/ashiato2-ch04`。下流 Task 9〜10 が残る・未 merge）の
   依頼と第 1 回の答えが、CH05 の依頼 1 と同じ対象を既に決めている（R1・R2）。CH04 の依頼「トリガを外す・緩めることはしない」（`ashiato2-ch04/docs/changes/CH04.md:31`）と CH05 Q1 の選択肢 3 が食い違う（R3）
2. **扉の幅** —— change に doors は無い（`docs/changes/CH05.md` の front matter）。該当なし
3. **新たに立つ一方通行** —— 作り直しは DB の中身を捨てる（`discarded`）。試験だけの DB なら中身は試験が作り直すので戻るが、
   main の checkout・`-verify` の DB（R5）と、印の無い Docker の volume（R6）はそうと言えない
4. **日常に影響する選択** —— 人間の日常に直接効くのは、確認バッチ（`-verify`）の最中に DB が作り直されて本人が入れた状態が消える場合だけ（R5）。電池・通知は無関係。検証 1 回あたりの時間は Q2 が扱っている
5. **既存コードが依頼を満たしていない箇所**（grep と DB の読み出しで確かめた）
   - 体積の 2 本（`deletion_tests.rs:257-275`、`export_file_tests.rs:1452-1478`）は共有の `ashiato` に入れて消さない —— premises のとおり
   - `fresh_db()` の後片付け（`stay_tests.rs:1322-1338` など 5 か所）は戻り値の future を待たないと走らない。殺されれば `*_tmp_*` が残る（いまは 0 個 —— premises のとおり）
   - **既に「作り直し」をしている経路がある**: `tools/smoke.sh:31,38`・`tools/stack.sh:24,76`（`HARNESS_VERIFY=1` で `STACK_RESET=1` → `down -v`）・`tools/check-immutable.sh:10`・`tools/check-panic-log.sh:15`。どれも同じ worktree で走っている試験を見ずに `down -v` する（R4）
   - `tools/verify-env.sh` は `HARNESS_VERIFY` を読んでいない（`grep -rn HARNESS_VERIFY tools/` は `stack.sh` だけ）
6. **過去の取りこぼし**（`python3 scripts/upstream_misses.py report`）
   - 「DB を共有する連鎖の検証は … 前段が残す行に後段が依存しないか（DB の作り直しを入口に入れるか）」（ST33 Q10）→ 当てはまる。閾値で作り直すと、同じ項目の複数のコマンドの間で DB が消えうる（R8）
   - 「共有の読む口を動かす design では … open の feat/* が置いたものが何に頼っているかを確かめる」（CH01 Q8）→ 当てはまる。CH04 が `testdb.rs` を作り替え中（R1）
   - 「推奨に『失うものが無い』と書く前に … 辿る」（ST08 Q11）→ Q5 の「どれも別の change にしても失うものは無い」は volume・main の DB を辿っていない（R5・R6）
   - 時計・画面・端末・取り込みの族 → 該当なし（確かめた範囲: 観点の一覧 38 件。CH05 は試験の足場と docker だけを触る）
7. **族の網羅** —— 「膨れの置き場」の族: (a) `ashiato` の行 (b) 殺された回の database（`ashiato_run_*`・`*_tmp_*`） (c) 印の無い volume (d) 消えた worktree のコンテナ・ネットワーク。一覧は (a)(b) だけ（R6）。
   「使っている途中で壊す」の族: 走っている `cargo test`・接続が 0 本の間の回・起きているサーバ・確認バッチ・検査と削除の間に始まった試験・既存の `down -v` の経路。一覧は接続の数だけ（R4・R7）。
   「殺す時点」の族は Q4 の 1 点だけ（R9）
8. **前提の確かさ** —— premises.md の行 8（接続の数で走っている試験が見える、確かさ 4）は CH04 の premises C6 と食い違い、確かめ方も走らせた試験ではない（R7）。行 10（ネットワークの枯渇）の結果は正しいが、Q3 が依る「取り直せない」は測っていない（R10）。`loss` の載った決定は一覧に無い

---

## R1. 依頼 1 は走っている change CH04 が既に決めて実装している。CH05 は CH04 との重なりを問わずに同じことを問い直している
- 成果物: openspec/changes/ch05-test-db-bloat/deep-questions.json
- 根拠: `/home/yosis/dev/ashiato2-ch04/openspec/changes/ch04-test-db-growth/deep.md:5-8`（Q1「回ごとの database ＋ 書き出しの試験だけ試験ごとの database」が答え済み）/ `ashiato2-ch04/docs/changes/CH04.md:15`（前の回の分は次の回の始まりに消える）・`:65`（「各 worktree の `ashiato` に積もった今のデータと、使われていない Docker の volume（この change では消さない。Q4）」）/ `ashiato2-ch04/openspec/changes/ch04-test-db-growth/tasks.md` の 1.1〜7.3 が `[x]`（`testdb::isolated()`・`scratch()`・片付け・`tools/check-test-db-growth.sh`・CI）、9.x・10.x が未了 / `git -C /home/yosis/dev/ashiato2-up-ch05 grep -n ashiato_run_ origin/main -- crates/server/src/testdb.rs` は 0 件（未 merge）/ ch04 の DB に `ashiato_run_84887f33cf52*` が実在（`pg_database`）/ CH05 の front matter は `requires: []`、`deep-questions.json` と `premises.md` が ch04 に触れるのは「測った場所」としてだけ
- kind: conflict
- 提案: CH05 を CH04 の残り（CH04 Q4 で「この change では何もしない」とした今の積もり・volume と、依頼 2 の作り直し、殺した後の試験）に絞る範囲の問いを第 1 回に置く（A。CH04 と違う向きを選ぶと走っている change の作り直しになる）。`requires: [CH04]` を問いの context に書き、proposal の Impact に `testdb.rs`・`ci.yml`・`tools/check-test-db-growth.sh` の重なりを書く前提にする
- 処置: escalated （第 1 回の問い Q1 にした。CH04 の上に積み、CH04 が持たないもの（今の積もり・殺した後の検査・依頼の 2）だけを推奨にした。`requires: [CH04]` は答えの後に record が CH05.md へ書く。premises.md に CH04 の行を足した）

## R2. Q1 は不要（CH04 で答えが出ている）。残すなら premise=A ではなく、選択肢の細部も CH04 と食い違う
- 成果物: openspec/changes/ch05-test-db-bloat/deep-questions.json
- 根拠: R1 の根拠 / Q1 選択肢 1 の「`ashiato_test_<時刻>_<乱数>`」「接続の無い前の run の database を消す」に対し CH04 は `ashiato_run_<12 桁>`（`ashiato2-ch04/crates/server/src/testdb.rs:275`）と session の助言ロック（`ashiato2-ch04/openspec/changes/ch04-test-db-growth/design.md:49,110`）/ `scripts/ask_wizard.py:78`（premise は「既に決めたことの根拠が事実と違っていた」）—— CH05 では何も決まっておらず、依頼の例（使い捨ての database）は正常に終わった回の残りもそのまま受ける。4 つの loss のどれも名付けられない
- kind: technical
- 提案: Q1 を削り、design に「CH04 の D1〜D6 に乗る」を C として残す。前提の食い違い（正常に終わった回も残す）は「価値」の文に書けば足りる
- 処置: fixed deep-questions.json （CH04 の答えを問い直す問いを外し、範囲の問いにした。premise にしたのは依頼の前提「誰も止めていない」が実物と違ったため。名前・ロックの細部は選択肢から外し CH04 のままとした）

## R3. Q1 の選択肢 3（トリガを外して前の残りを消す）は、CH04 の依頼で本人が既に禁じている
- 成果物: openspec/changes/ch05-test-db-bloat/deep-questions.json
- 根拠: `ashiato2-ch04/docs/changes/CH04.md:31`（「DB の書き換え禁止のトリガ（消せない記録）を試験のために外す・緩めることはしない」）・`:61`（壊してはいけないもの）/ `ashiato2-ch04/openspec/changes/ch04-test-db-growth/premises.md` の C4
- kind: conflict
- 提案: Q1 を残す場合も選択肢 3 は外す（選べない選択肢を人間に見せない）。CH05 の「壊してはいけないもの」の候補に CH04 の 61〜64 行（トリガ・`ashiato` を使う 3 つの経路・`testdb::pool()` の形）を引き継ぐ。**「壊してはいけないもの」を問う／既定を示す問いが一覧に 1 つも無い**（SKILL の change の questions は第 1 回で「何を変えないか」を問う）ので、ここに置く
- 処置: escalated （トリガを外す選択肢を Q1 から消し、範囲を Q1 で人間に問う。壊してはいけないもの（トリガ・`ashiato` を使う 3 つの経路・`testdb` の形）は CH04 の 61〜64 行を引き継ぎ、record で CH05.md に書く）

## R4. 既にある `down -v` の経路（smoke・stack・check-immutable・check-panic-log）が、同じ worktree で走っている試験を見ずに DB を作り直している。依頼の「走っている試験がある DB は作り直さない」をそれにも当てるかが無い
- 成果物: openspec/changes/ch05-test-db-bloat/deep-questions.json
- 根拠: `tools/smoke.sh:31,38` / `tools/stack.sh:24`（`HARNESS_VERIFY=1` なら `STACK_RESET=1`）・`:76`（`down -v`）/ `tools/check-immutable.sh:10` / `tools/check-panic-log.sh:15` / `tools/verify-env.sh:19-20`（smoke の `down -v` の後に役割が無く `password authentication failed` で 371 本が落ちた実測）
- kind: technical
- 提案: 族の兄弟として B の問いを足す（推奨の例: 新しく作る見張りを `tools/` の 1 か所に置き、既存の 4 つの `down -v` もそれを通す）。あわせて Q2 の why「e2e・偽データ…は共有 DB に書く」を直す —— 検証の経路では `stack.sh` が毎回作り直しているので、e2e の残りが溜まるのは検証の外で起こしたときだけ
- 処置: fixed deep-questions.json （(2) として「既存の `down -v` の台本にも見送りを当てる」を選択肢に足した。推奨は入れない）。Q2 の why を「試験の外（偽データ・手で起こしたサーバ）」に直した

## R5. 作り直しの対象をどの worktree に限るかが無い。main の checkout と確認バッチ（`-verify`）の DB は試験だけの DB ではない
- 成果物: openspec/changes/ch05-test-db-bloat/deep-questions.json
- 根拠: `tools/ports.sh:6`（main の checkout はサーバ 18787「スマホから見る tailscale serve の先」）・`:9`（`-verify` は確認バッチの作業ツリー）/ `tools/stack.sh:48`（「本番のサーバが Tailscale の IP:18787 で動いている」）/ Q2 の why「作り直しは計算し直せば戻る（試験の DB の中身は試験が毎回作る）」は試験の worktree でしか成り立たない。main の DB の中身が何か（本人のデータを含むか）は確かめていない（main の compose project `ashiato2` はいまコンテナ 0 個）
- kind: irreversible
- loss: discarded
- 提案: 「作り直してよい worktree」を A で問うか、扉を開けたままの既定（C: `-st<NN>`・`-ch<NN>`・台帳の worktree だけ。main は作り直さない。`-verify` は確認バッチの始まりの前だけ）を design に置き、Q2 の context に書く。main の DB の中身は premises に 1 行足して確かめる
- 処置: escalated （第 1 回の問い Q5 にした。推奨は失わない側 —— Story・change の作業ツリーだけ。main の checkout は選択肢に置かない）

## R6. 「膨れの置き場」の族のうち、印の無い Docker の volume と消えた worktree のコンテナ・ネットワークが一覧に無い。volume を機械で消すなら、どの worktree のものか区別できない
- 成果物: openspec/changes/ch05-test-db-bloat/deep-questions.json
- 根拠: `docker volume ls -qf dangling=true | wc -l` = 65（全 94）。そのうち 62 個は `com.docker.compose.project` の印が空（`docker volume inspect` で集計）—— `docker-compose.yml` は名前つきの volume を持たず、`tools/stack.sh down`（39 行。`-v` なし）は使うたびに data の volume を 1 つ置き去りにする / worktree が消えているのに動いているコンテナ: `ashiato2-ch01`・`ch02`・`st13`・`st33`（`com.docker.compose.project.working_dir` が `git worktree list` に無い）/ コンテナ 0 個の compose のネットワークが 18 本（`ashiato2-st08_default` など）/ CH04 は Q4 でこれらを「何もしない」と答え済み（R1）
- kind: irreversible
- loss: discarded
- 提案: 範囲の問い（Q5 か新しい問い）に「消えた worktree の compose project（コンテナ・ネットワーク）を片付けるか」「印の無い volume をどうするか」を足す。印の無い volume は main の checkout のもの（R5）と区別できないので、消す選択肢には `loss: discarded` を付けて A にする。消さない側を既定にするなら C で design に残す
- 処置: escalated （第 1 回の問い Q7 の (1) にした。volume は消さないと明記し、消す側に irreversible を書いた。推奨は入れない・申し送り）

## R7. premises 行 8「走っている試験は `pg_stat_activity` の接続で見える」（確かさ 4）は誤りうる。CH04 は同じ点を「接続の数で判定しない」と決めている
- 成果物: openspec/changes/ch05-test-db-bloat/premises.md
- 根拠: `ashiato2-ch04/openspec/changes/ch04-test-db-growth/premises.md` の C6（「プールは試験ごとに作って捨てるので、走っている回でも接続が 0 本になる間がある」）/ `crates/server/src/testdb.rs:73-83`（`pool()` は試験ごとに作る）/ `cargo test` は試験の実行ファイルを順に起こすので、その間は接続が無い / 確かめ方「st29 で試験の最中に接続 2 本が見えた」は 1 時点の観察で、外れたら落ちる台本でも試験でもない（確かさ 4 の定義に合わない。2 が上限）/ Q2 の選択肢 1 は「ほかの接続が 1 本でもあれば作り直さない」をこの前提に載せ、検査から `DROP … WITH (FORCE)`（Q3 の選択肢 1）までの間に始まった試験も見ていない
- kind: premise
- 提案: 行 8 を直して確かさを 2 に下げる。Q2 の推奨を「CH04 の回の助言ロック（回の間ずっと持つ 1 本の接続）＋サーバ・e2e の接続」と、検査と作り直しを同じ錠の中でやる形に書き直す。CH04 が merge されるまでは接続の数では見えないことを context に書く
- 処置: fixed premises.md （行を直して確かさ 2 に下げた）。deep-questions.json の Q3（助言ロックと接続で見る問いを足し、判定と作り直しを同じ錠の中で行うことを推奨の detail に書いた）

## R8. 閾値で作り直すと、同じ項目の複数の検証コマンドの間で DB が消えうる（ST33 Q10 の型）
- 成果物: openspec/changes/ch05-test-db-bloat/deep-questions.json
- 根拠: `.harness2/scripts/evidence.py:341`（`run_one` は 1 コマンドごと）・`:378-388`（準備 `tools/verify-env.sh` をコマンドごとに毎回呼ぶ）/ `upstream_misses.py report` の ST33 Q10「DB を共有する連鎖の検証 … DB の作り直しを入口に入れるか」
- kind: technical
- 提案: Q2 の推奨の detail に、作り直しが 2 つ目以降のコマンドの前でも起きることと、前のコマンドが置いた行に頼る検証を書かない取り決め（あるいは作り直しを項目の最初のコマンドの前に限る）を足す。spec で「壊してはいけないもの」に入れる候補
- 処置: fixed deep-questions.json （推奨の detail に、コマンドの間でも作り直りうることと、CH04 の後の試験は `ashiato` の中身に頼らないことを書いた）

## R9. Q4 の「殺す」は 1 時点（行が入ったところ）だけ。殺す時点の兄弟が無い
- 成果物: openspec/changes/ch05-test-db-bloat/deep-questions.json
- 根拠: 体積の `INSERT … SELECT generate_series` は 1 文（`deletion_tests.rs:262-275`）なので、文の途中で殺すと巻き戻って行は残らない —— 残るのは文が終わった後に殺した回と、正常に終わった回 / CH04 の回は テンプレート（`_tpl`）の作成・移行・試験ごとの database の作成という別の時点を持つ（`ashiato2-ch04/crates/server/src/testdb.rs:12`・ch04 の DB の `ashiato_run_…_tpl`・`…_x_…` 370 MB）
- kind: technical
- 提案: Q4 の context に殺す時点を並べる（試験ごとの database を作った後・テンプレートを作っている途中・回の初めの片付けの途中）。推奨の判定は CH04 の `tools/check-test-db-growth.sh` に「殺してから 2 回目」を足す形と書けば、CH04 と別の台本を作らずに済む。また推奨は依頼の完了の条件「速さが変わらない」を量に置き換えるので、record で `CH05.md` の完了の条件を書き換えることを detail に書く
- 処置: fixed deep-questions.json （殺す時点を 2 つにし、完了の条件の書き方を直すことを detail に書いた）

## R10. Q3 は premise=A の根拠が無い。測ったのは「新しい project がネットワークを作れない」で、「作り直すと取り直せない」ではない。失うものも無い
- 成果物: openspec/changes/ch05-test-db-bloat/deep-questions.json
- 根拠: premises 行 10 の結果は `failed to create network ashiato2-up-ch05_default`（この worktree の**初めての** `up`）/ `down -v` は自分のネットワークを手放した直後に同じ大きさを取り直すので、負けるのは間に別の project が取ったときだけ / その `down -v` → `up` は既に smoke・stack・check-immutable・check-panic-log が検証のたびにやっている（R4。CH05 で新しく立つ危うさではない）/ 枯渇の原因はコンテナ 0 個のネットワーク 18 本（R6）/ 作り直しに負けた結果は `BLOCKED_INFRA`（`evidence.py:383-387`）で、試験の DB の中身は試験が作り直す —— 4 つの loss のどれも起きない
- kind: technical
- 提案: Q3 を B（`kind: open`、推奨はいまのまま）に落とし、context を測ったことに合わせて直す。依頼の逐語の手順と違う方を採るなら、record で `CH05.md` に「依頼の手順を覆した」と書くことを detail に残す。選択肢 1 の「中の database だけ」には、`ashiato` のほかに回の database・`*_tmp_*` も対象か、`postgres` database から落とすことを書く
- 処置: fixed deep-questions.json （B に落とし、context を測ったことに合わせた）

## R11. Q5 の (1) の直し方と (2) の見立てが実物と合わない
- 成果物: openspec/changes/ch05-test-db-bloat/deep-questions.json
- 根拠: (1) の detail「docker の設定（このマシン全体）に触る」—— 枯渇の原因は消えた・古い worktree のネットワーク 18 本と、worktree が消えたのに動くコンテナ 4 つ（R6）で、`default-address-pools` を変えなくても repo の台本から片付けられる / (2) の「膨れの症状だった可能性が高い」には出所が無い（premises に行が無い。確かさ 1）。同じ worktree で smoke・stack の `down -v` が試験の最中に走れば役割が消えて権限のエラーになる経路が既にある（`tools/verify-env.sh:19-20`・R4）
- kind: premise
- 提案: (1) に「消えた worktree の compose project を片付ける（docker の設定には触らない）」を足すか、detail を直す。(2) の context は見立ての出所を書くか、R4 の経路も候補として並べる
- 処置: fixed deep-questions.json （(1) を docker の設定ではなく使われていない資源の片付けに直し、(3) の見立てを出所つきの経路に直した）

## R12. Q2 の選択肢に、依頼が挙げた「下流の Task の開始前」が無い。verify-env だけでは agent が自分で走らせる試験は膨れた DB の上で走る
- 成果物: openspec/changes/ch05-test-db-bloat/deep-questions.json
- 根拠: `docs/changes/CH05.md` 依頼 2（「大きさの閾値・下流の Task の開始前など」）/ `AGENTS.md`（Task agent は `scripts/quiet-run` で `cargo test` を走らせる —— `verify-env.sh` を通らない）/ premises 行 7「harness2 への申し送りは要らない」は確かさ 2 で、verify-run の経路だけを見ている
- kind: technical
- 提案: Q2 に「Task の開始前（グラフの node。harness2 への申し送り）」の選択肢を足すか、verify-env だけにする理由を context に書く。B のままでよい
- 処置: fixed deep-questions.json （「下流の Task の開始前（harness2 への申し送り）」を選択肢に足した）
