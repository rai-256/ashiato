# ST48 deep-questions の独立検査（deep-review）

※ 処置の後、問いは 5 問に組み直した（旧 Q1+Q2 → Q1、旧 Q3〜Q6 → Q2〜Q5）。下の R の本文は組み直す前の番号で書かれている。

検査対象: `openspec/changes/st48-db-version-guard/deep-questions.json`（Q1〜Q6。全部 `kind: open`・`recommended` あり）。
手順 1〜8 は問いの一覧を読む前に、Story・要件（FR-104〜FR-130）・C4・コード・走っている change の枝から先に行い、後で突き合わせた。

## 手順ごとの記録

- 手順 1（要件どうしの衝突）: FR-117 / FR-118 / FR-129 どうしは同じ向き。衝突は **FR-105 の「チャンネルを移るとき DB は戻さない、正式版が追いつくのを待つ」と「正式版の枝の緊急修正」** と、「知らない移行があれば止める」の組で出る（R4）。もう 1 つは Story の「壊してはいけないもの」（全版を当て直せる形の移行が当たり続けること）と Q1 / Q2 の推奨の組（R1）
- 手順 2（扉の幅）: 該当なし（確かめた範囲: ST48.md の `doors: []`。satisfies の 3 要件はどの扉の関与要件にも出てこない。Q4 が引く扉 #15 は既定の向きの根拠として使っているだけ）
- 手順 3（新たに立つ一方通行）: 台帳の形と置き場、無いときの扱い、終了コードは**最初に配る版に焼き付く**（R2・R3）。台帳に残す「当てたアプリの版」は、いまの組み立てではどれも 0.1.0（R7）
- 手順 4（日常に影響する選択）: 確認バッチの DB は作り直されない。止まるのは人間が止まる場所（確認バッチ）で、DB には電話から届いた記録が入りうる（R5）
- 手順 5（既存コードとの突き合わせ）: `migrate()`（`crates/server/src/lib.rs:186-197`）は全版を当て直し、台帳は無い。移行を当てる経路が `migrate()` の外にも 2 つある（`testdb.rs:93-96`・`tools/check-immutable.sh:21-34`）（R9）。アプリの役割は `core` の全表を書ける（`grants.sql`）（R8）。`tools/db-roles.sh` は移行の前に管理者で DB を変える（R10）
- 手順 6（過去の取りこぼし）: 当てはまった観点は 4 つ。「書き直せない記録の形の凍結」→ R3、「〜まで待つ規則が解けないまま」→ R4、「保存場所・手がかりが丸ごと無いとき」→ R2、「他 Story の決定の波及」→ R1。時計の観点は当てはまらない（版の比べ方に時刻を使わない。premises.md の 1 行目で ORDER が時刻順でないことは確かめてある）
- 手順 7（族の網羅）: Q3〜Q6 の族「DB と版の食い違い」には兄弟が足りない（R4・R9）
- 手順 8（前提の確かさ）: R11

不要: 該当なし（Q1〜Q6 はどれも、要件か扉で既に決まっているものではない）。
文字で決めさせている画面の問い: 該当なし（画面を持たない）。
既定の無い仮の問い: 該当なし（6 問とも `recommended` がある）。

---

## R1. Q1 の推奨（全部を毎回当て直す）と Q2 の推奨（main の移行の中身を CI で止める）は両立しない。走っている ST23・ST41 が、まさに main の移行の中身を直している
- 成果物: openspec/changes/st48-db-version-guard/deep-questions.json
- 根拠: `git -C ../ashiato2-st23 diff main...HEAD -- migrations/202609271716_deletion_ledger.sql`（b051f808。本文:「起動のたびに全版を当てるため、列を足した後のビューへ古い定義を当てると落ちる」ので、main にある `CREATE OR REPLACE VIEW core.stay_erased` を「無いときだけ作る」に書き換えた）/ `git -C ../ashiato2-st41 diff main...HEAD -- migrations/202609120943_version_and_ledger.sql`（3983a055。`core.event_version_live` について同じ書き換え）/ openspec/changes/st41-event-end-time/design.md:201 / Q2 の context「以後は Story のブランチの中で、main に入る前にだけ直している」/ docs/stories/ST48.md の「壊してはいけないもの」1 行目
- kind: conflict
- 提案: 全部を当て直す形のままだと、後の版がビューの列を足すたびに、前の版のファイル（main にあるもの）を書き換えないと 2 回目の当て直しが落ちる。新しい版を足しても直らない（古いファイルが先に走って落ちる）。なので「Q1 = 全部を当て直す」と「Q2 = main の中身を止める」は一緒には選べない。Q2 の context の前提を直し、ST23・ST41 の 2 本を実例として書く。そのうえで Q1 と Q2 を組で問うか、Q2 の選択肢にこの組み合わせの帰結を書く（どちらの側が ST23 / ST41 の作り直しを求めるかも書く）
- 処置: escalated —— 旧 Q1（当て方）と旧 Q2（直すことを止める）を Q1 の 1 問にまとめ、組でしか選べないことと ST23・ST41 の実例を why / context に書いた。推奨は「まだの分だけ当てる + 配った（リリースの印より前の）移行の中身と名前を CI が止める」。印は ST52 から付くので、走っている ST23・ST41 の書き換えは止めない（差し戻さない）

## R2. Q3 の初回の規則は「1 回きり」ではない。配った版すべてに残り、台帳が見つからない DB に出会うたびに働く。推奨の側には失うものがある
- 成果物: openspec/changes/st48-db-version-guard/deep-questions.json
- 根拠: Q3 の why「初回の扱いは 1 回きり」/ 推奨の「いつもどおり全部を当て、全部を『当てた』と記録する」/ docs/requirements.md の「配布と更新」冒頭「FR-117 / FR-118 / FR-129 は、最初に配る版に入れる」/ FR-129 の★（古い版の移行が新しい D-01 に当たると門が古い定義に戻る）/ `migrations/202609111111_coverage_rebuild.sql:18-28`（形を見て表を DROP する移行が現にある）
- kind: irreversible
- loss: discarded
- 提案: 後の版で台帳の置き場か形が変わったとき、または台帳が消えたときを考える。古い版は、新しい DB を「記録の無い ST48 前の DB」と読む。推奨の側だと自分の知る全版をその DB に当て、自分の版を記録する。その後の起動の検査（FR-117）も通ってしまうので、FR-117 / FR-129 が防ぎたい形そのものになる。why をこの事実に直し、失わない側を選択肢に書く（台帳の置き場と形を永久に固定する、`core` があるのに台帳が無い DB は止めて本人の DB だけ 1 回手で通す、など）。B（仮）のままにするか A に上げるかは、失わない側の費用を書いてから判定する
- 処置: escalated —— Q2 の why を「配った全部の版に残る規則」に直し、選択肢 2 に「記録が壊された DB にも当て直さない」を足した。失わない側（記録の置き場と形を凍結の試験で固定する）は premises.md の C1 にした。費用が小さく範囲内なので B（推奨あり）のまま本人に見せる

## R3. 台帳の形・読み方・止まったときの終了コードは、最初に配る版に焼き付く。凍結の試験と「読めないときは止める」を決める問い（または design の D）が無い
- 成果物: openspec/changes/st48-db-version-guard/deep-questions.json
- 根拠: docs/requirements.md の「配布と更新」冒頭（FR-117 / FR-118 / FR-129 を最初に配る版に入れる）/ Q5 の why「終了コードは後から変えられる」/ docs/stories/ST58.md（FR-119。更新係が終わり方を読み、「前の版の S-01」を起こし直す）/ `python3 scripts/upstream_misses.py report` の観点「書き直せない記録の形を新しく決めるときは、形の凍結と読めない形の出し方を問いにする」（ST33 Q12）
- kind: irreversible
- loss: exported
- 提案: 配った S-01 は、台帳を読む問い合わせと判定を持ったまま本人の手元に残る。後から台帳の形を変えたり終了コードの意味を変えたりしても、配った版の動きは変えられない（ST58 の更新係は、配った全部の版の終了コードを読むことになる）。1 問足すか、Q5 の why を直す。足す問いの中身は 2 つ。(a) 台帳の形を固定の試験で凍結し、形の版を持たせるか。(b) 台帳が読めない・知らない形のとき、止める側に倒すか。失わない側（凍結して、読めないときは止める）は安いので、推奨を付けた B でよい
- 処置: escalated —— Q4 の why の「後から変えられる」を「配った版に残るが、区別する側は扉を開けたまま」に直した。記録の形の凍結と「読めない形は止める」は premises.md の C1 にした

## R4. 「正式版が追いつくのを待つ」が永久に解けない場合と、正式版の枝の緊急修正に移行が入る場合を、族も問いも扱っていない
- 成果物: openspec/changes/st48-db-version-guard/deep-questions.json
- 根拠: FR-105（「チャンネルを移るとき DB は戻さない —— 正式版が追いつくのを待つ」「正式版にだけ緊急の修正を当てるときに限り、その版の枝を一時的に切る」）/ `crates/server/build.rs:26-55`（ORDER とファイルが合うかしか見ない。ファイルと ORDER の行を一緒に消せば通る）/ Q2 は中身の書き換えしか扱っていない / upstream_misses の観点「〜が解消するまで待つ規則には、解消しないまま期限を越えたときを問う」
- kind: conflict
- 提案: 族「DB と版の食い違い」に 2 つの兄弟を足す。(a) ベータ版・開発版に載った移行を、幹で取り消したか名前を変えた。その DB は以後どの版でも開けない（待っても追いつかない）。(b) 正式版の枝の緊急修正で移行を足した。幹がその移行を取り込むまで、正式版からベータ版へ進むのも止まる（FR-117 は「新しい」ことを求めているが、比べ方は「知らない名前があれば止める」）。問いの中身は「配った移行の名前を ORDER から消す・変えることを機械で止めるか」と「緊急修正の移行は先に幹へ入れることを求めるか」。どちらも、名前を戻せば DB は開けるので B（仮）でよい
- 処置: escalated —— 族に 2 つの兄弟を足した（Q1〜Q5 の family）。配った移行の削除・名前の変更は Q1 の推奨の検査が落とす。緊急修正の移行は同じ名前で幹へ入れる（premises.md の C2）

## R5. Q6 と Story の前提「確認バッチの DB は使い捨て（MIGRATE_BACKUP=off）」が誤り。確認バッチの DB は作り直されず、電話から届いた記録が入りうる
- 成果物: openspec/changes/st48-db-version-guard/deep-questions.json
- 根拠: tools/verify-prep.sh:72-80（`run.sh` は `STACK_RESET` を付けずに `tools/stack.sh up` を呼ぶ）/ tools/stack.sh:24・75-76・83（作り直しと `MIGRATE_BACKUP=off` は `STACK_RESET=1` か `HARNESS_VERIFY=1` のときだけ）/ tools/ports.sh:9-10（`-verify` の DB は 55499 に固定。「確認の間だけ立てるサーバは、スマホから届く 18787 のまま」）/ docs/stories/ST48.md の「壊してはいけないもの」3 行目 / Q6 の context「使い捨ての DB は `STACK_RESET=1` で作り直せる」
- kind: daily
- loss: discarded
- 提案: 確認バッチは回ごとに入れる feat の組が違う。前の回に入っていて今回外れた feat の移行は DB に残ったままになるので、ST48 の後は `run.sh` が止まる。止まるのは、人間が止まる 3 か所の 1 つ。推奨の「止まったら作り直す」をこの DB でやると、確認の間に電話が送った記録も捨てることになる（電話は送り終えた分を手元から消す）。Q6 の context に確認バッチの DB を名指しで足し、「作り直す」側で失うものを書く。選択肢には確認バッチの扱い（回ごとに DB を分ける・`run.sh` を作り直す形にする、など）を加える
- 処置: escalated —— 旧 Q6 を Q5（確認バッチと worktree の DB）に直し、context に `run.sh` が DB を残すこと・電話の記録が入ることを書いた。推奨を失わない側（前の DB を残して新しい DB で立て直す）にし、作り直す側に失うものを書いた

## R6. Q5 の選択肢 2 の「DB は変わっていないので害は無い」は成り立たない。推奨の並び（退避を取る前に止める）と組み合わせると、ST58 は前の回の退避で戻しうる
- 成果物: openspec/changes/st48-db-version-guard/deep-questions.json
- 根拠: Q5 の選択肢 1 の label「退避も取る前に止める」/ 選択肢 2 の detail「ST58 が戻しを走らせても、DB は変わっていないので害は無い」/ crates/server/src/migrate_backup.rs（退避は `MIGRATE_BACKUP_DIR` に `pre-migrate-<時刻>.dump` として積まれる）/ FR-119（「移行の前の退避から D-01 を戻し」）
- kind: irreversible
- loss: discarded
- 提案: 止めるのが退避の前なら、この回の退避は無い。終わり方が移行の失敗と同じだと、ST58 は置き場にある直近の退避（前の更新の分）で戻しうる。その場合、その後に入った記録を捨てる。選択肢 2 の detail をこの事実に直し、「退避の前に止めるか」と「終わり方を分けるか」が組であることを書く。推奨の側に失うものは無いので、B のままでよい
- 処置: escalated —— Q4 の選択肢 2 の detail を「前の回の退避で戻し、その後の記録を捨てうる」に直し、退避の前に止めることを context に書いた

## R7. 「どちらの版か」を示す材料の前提が誤り。いまの組み立ては全部 0.1.0 なので、台帳に残す「当てたアプリの版番号」で区別できない
- 成果物: openspec/changes/st48-db-version-guard/deep-questions.json
- 根拠: Q5 の context「DB の版と自分の版（当てたアプリの版番号と、自分の知らない移行の名前）を示す」/ Cargo.toml:6 `version = "0.1.0"` / docs/audit/2026-10-25-consistency.md:223-229（K3。版は 3 か所に手書き）/ docs/stories/ST52.md（版の出所を 1 つにする Story。`requires: [ST48, ...]` なので ST48 より後）
- kind: premise
- loss: uncaptured
- 提案: ST48 から ST52 までの間に当てた行は、どのブランチ・どの組み立てで当てても「0.1.0」になる。どの組み立てが当てたかは残らず、後から取り戻せない。FR-117 の「どちらの版か」も、その間は移行の名前でしか示せない。台帳に組み立ての印（commit など）を一緒に残すかを、B の問いか design の D で決める。失わない側（一緒に残す）は安い
- 処置: escalated —— Q4 の context に、示す材料として組み立ての印（commit）を残すことを書いた（premises.md の C3）。premises.md に 0.1.0 の行を足した

## R8. アプリの役割は `core` の全表を書け、後から作る表にも既定で書ける。台帳を `core` に置くと、守る相手のサーバが台帳を書き換えられる
- 成果物: openspec/changes/st48-db-version-guard/deep-questions.json
- 根拠: crates/server/src/grants.sql（`GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA core TO ashiato_app` と `ALTER DEFAULT PRIVILEGES … GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ashiato_app`）/ FR-118
- kind: technical
- 提案: 人間に問わない。design の D で決める（台帳を `core` の外に置く、またはアプリの役割から読み取り以外の権限を外して書き換え禁止の門を付ける）。FR-117 の起動の検査はアプリの役割で台帳を読むので、読み取りの権限は要る
- 処置: fixed premises.md —— 問わない。premises.md の C4（記録の表は `core` の外に置き、アプリの役割は読むだけ）にした。spec の段で design の D にする

## R9. 族「DB と版の食い違い」の兄弟が足りない。移行は `migrate()` の外でも当たるので、「当たっているのに記録が無い・欠けている」はふだんの worktree で毎回起きる
- 成果物: openspec/changes/st48-db-version-guard/deep-questions.json
- 根拠: crates/server/src/testdb.rs:93-96（`MIGRATIONS` と `GRANTS` を直に当てる。`migrate()` を通らない）/ tools/ports.sh（試験と `tools/stack.sh` は worktree ごとに同じ DB の port を使う）/ tools/check-immutable.sh:21-34（psql で直に当てる）/ openspec/changes/ch04-test-db-growth/design.md:43-46（雛形の database にも `crate::MIGRATIONS` を直に当てる）/ docs/production-prep.md:44（「各版に戻し手順を必ず書く」。`.down.sql` を手で当てうる）
- kind: technical
- 提案: 族に 3 つの兄弟を足す。(a) 台帳はあるが、一部の版が記録無しで当たっている（試験の足場や psql の経路）。(b) `.down.sql` を手で当て、記録は残っているのに中身が戻っている。(c) 台帳が読めない・形が違う（R3）。(a) と (b) は Q1 の答えで扱いが変わる（まだの分だけ当てる形だと (b) は当て直されない）ので、Q1 の選択肢 2 の detail に書く。どの経路で記録を書くかは design の D でよい。版の拒否を試す試験は、worktree で共有する DB に知らない版の行を書くと、その worktree の stack と試験を全部止める。CH04 の使い捨ての database（design.md:46）を使うことを proposal の Impact に書く
- 処置: fixed premises.md —— 族に兄弟（`migrate()` の外で当たった版・`.down.sql` を手で当てた DB）を足し、記録の書き手と拒否の試験の DB を premises.md の C5 にした

## R10. `tools/db-roles.sh` は移行より前に管理者で DB を書き換える。FR-129 の止めは `migrate` だけに効く
- 成果物: openspec/changes/st48-db-version-guard/deep-questions.json
- 根拠: tools/stack.sh:80（db-roles → :85 migrate）/ tools/dev.sh:20・tools/smoke.sh:40（同じ並び）/ tools/db-roles.sh（`ashiato_app` の役割の付与を全部外す・`core` の所有を移す・合言葉を揃える）
- kind: technical
- 提案: 古い版の台本が新しい DB に当たると、版の検査より先に db-roles が走る。いまの中身は版に依らない操作だけなので害は小さいが、後の版が役割や schema を足すと古い db-roles が先に触る。問いにはしない。design の D に「db-roles は版に依らない操作だけにする（または同じ検査を先に通す）」と書き、proposal の Impact に挙げる
- 処置: fixed premises.md —— 問わない。premises.md の C6（db-roles は版に依らない操作だけにする）にした。proposal の Impact に挙げる

## R11. premises.md の行が、Q1 の推奨を支えるには足りない。この Story の決定が依る事実のうち 3 つが表に無い
- 成果物: openspec/changes/st48-db-version-guard/premises.md
- 根拠: premises.md の行「全部の移行は 2 回当てても落ちない」（確かさ 4、出所「既存のテスト材料」。確かめ方は「CI で走る」で、自分で走らせた記録が無い）/ R1 の ST23・ST41 の枝（後の版がビューの列を足すと、前の版の当て直しが落ちる）/ R5・R7 の根拠
- kind: premise
- 提案: 「2 回当てても落ちない」は main にある版だけで見た性質で、後の版を足しても保たれるとは言えない。行の「何が依るか」から「全部当て直すを続けられるか」を外すか、ST23 / ST41 の実例を結果に書く。足す行は 3 つ。(1) 確認バッチの DB は作り直されず、電話が届く（tools/verify-prep.sh:80・tools/ports.sh:10）。(2) 組み立ての版はどれも 0.1.0（Cargo.toml:6）。(3) 移行は `migrate()` の外の経路でも当たる（testdb.rs:93）
- 処置: fixed premises.md —— 「2 回当てても落ちない」の行を確かさ 3 に下げて ST23・ST41 の実例を書き、確認バッチの DB・組み立ての版・`migrate()` の外の経路の 3 行を足した
