# CH06 deep-questions の検査（deep-review）

検査の対象: `openspec/changes/ch06-reset-verify-db/deep-questions.json`（Q1〜Q4、全部 open / B・推奨あり）と `premises.md`（P1〜P8・C1〜C4）。
手順 1〜8 は、問いの一覧を見る前に、依頼の逐語（`docs/changes/CH06.md`）・コード・Docker の実物（読むだけ）・走っている CH04（`ashiato2-ch04` の worktree を含む）から先にやり直した。
Docker の振る舞いは、使い捨ての compose project（`/tmp` の下・`network_mode: none` か明示の subnet。他の worktree の project には触っていない。終わった後に消し、volume の数が 95 のままであることを確かめた）で実測した。

## 手順ごとの記録

- 手順 1（衝突）: `satisfies: []`。依頼の完了の判定どうしと、C どうしを突き合わせた。C1（worktree でない木では作り直さない）と C4（判定 3 の検査を CI の rust job で走らせる）がぶつかる —— CI の checkout は `tools/ports.sh` では「worktree でない木」になる → R2
- 手順 2（扉の幅）: 「その worktree のテスト用 DB」が volume ごとか `ashiato` database だけかは Q1 が拾っている。「検証のたびに」は依頼の逐語で幅が無い → R5。作り直せなかったときに溜まった DB で進める案は、依頼が引くハーネスの約束（`scripts/evidence.py:28,374`）と食い違い、幅が無い → R6
- 手順 3（新たに立つ一方通行）: 該当なし（確かめた範囲: 消えるのは worktree の compose project の DB の中身で、試験の行・役割・CH04 の回の database・名前の規則の外の database（CH02 の計測など。生成の台本から作り直せる）。列の型・保存形式・鍵の入力は変えない。外に出すものは無い。失う側があるのは main の checkout の DB だけで、C1 が触らない側に倒している。ただし C1 の判定の仕方が CI と同じ形になる点は R2）。全問 B の分類は妥当
- 手順 4（日常）: 検証 1 回あたりの待ち（実測の代わりの計測を R8 に置いた）。Docker の容量 —— 作り直しの手段によっては匿名 volume が残り続ける（実測）→ R7。いまこの機械で DB を起動できない worktree がある（P4）→ R1
- 手順 5（既存コード）: `tools/verify-env.sh` が `HARNESS_VERIFY` を見ていないこと（`grep -rn HARNESS_VERIFY tools/` は `stack.sh:13,24` だけ）・`testdb.rs:86-102` が過程ごとに全部の移行を当てること・`scripts/evidence.py:374-388` が準備に印を渡し、準備が非 0 なら BLOCKED_INFRA にすることは、依頼と premises の記述どおり。合わないのは、ネットワークを手放すのが `stack.sh` だけという記述（R3）、P2 の行番号（R9）、Q1 の context の (3)（R4）
- 手順 6（過去の取りこぼし）: 「DB を共有する連鎖の検証は… DB の作り直しを入口に入れるか」（ST33 Q10）はこの change そのもの。逆向きの兄弟「前の検証コマンドの DB に頼る後ろの検証コマンド」を、走っている st23・st29・st41・ch03・ch04 の tasks.md の「検証:」で `psql`・`db.sh exec`・`compose exec`・`seed`・`archive-shape` を grep して探した —— 1 本のコマンドの中で閉じているものだけで、該当なし。「見積もり・根拠を実物で測っていない」（3 件）は Q1・Q4 の費用「数秒〜十数秒と見込む（測れていない）」に当たる → R8。「実物の形式・環境が未確認」は P4 の後半（確かさ 2）に当たる → R3 で実測した
- 手順 7（族の網羅）: Q2〜Q4 の族「作り直しが邪魔をする・できない場合」の兄弟を挙げた。(a) 繋いでいるものがある = Q2、(b) 作り直しが失敗 = Q3、(c) 頻度 = Q4、(d) **ネットワークがそもそも取れない（今この worktree で起きている）** = 一覧にも C にも無い → R1、(e) **検証コマンド自身が `down -v` で作り直してネットワークを手放す** = C2 が `stack.sh` しか見ていない → R3、(f) **worktree でない木 = CI** = C1 と C4 の間で決まっていない → R2、(g) 後ろの検証が前の検証の DB に頼る = 該当なし（手順 6）、(h) DB 以外の溜まる状態（`.export`）= 書き出しのファイル名は `export_id` で決まり（`export/dump.rs:223`）取り合わない。該当なし
- 手順 8（前提の確かさ）: `loss` のある問い・推奨は無い（全部 B）。C1 は失う側（main の checkout の DB）を避ける決定で、P7（確かさ 2）が誤っていても触らない側なので結論は変わらない。ただし「worktree でない木」の見分け方が CI にも当たることは premises に無い（R2）。P4 の後半（`down` がネットワークを消す。確かさ 2）は実測して正しかった（R3。4 に上げられる）。P5・P6（確かさ 2）は CH04 の実装が `ashiato2-ch04/crates/server/src/testdb.rs:12-18,155,196,212` にあり、読めば 3 以上に上げられる（R9）

---

## R1. いまこの機械では、CH06 の下流の worktree の DB が起動できない見込みが高い。原因の大半は消えた worktree が持ったままのネットワークで、その片付けを誰がするかが一覧にも C にも無い
- 成果物: openspec/changes/ch06-reset-verify-db/deep-questions.json / openspec/changes/ch06-reset-verify-db/premises.md
- 根拠: premises.md P4（この worktree の `tools/verify-env.sh` が `all predefined address pools have been fully subnetted` で落ちた）/ `docker network ls` の `ashiato2*_default` 25 個のうち、`/home/yosis/dev/<project>` が無い（`git worktree list` に無い）ものが 15 個: ch01・ch02・st08・st12・st13・st14・st16・st19・st21・st22・st22-ra・st25・st28・st33・st47。うち ch01・ch02・st13・st33 は DB のコンテナが今も `Up`（`docker ps -a`）/ 明示の subnet（`ipam.config.subnet` に私設の /24 を 1 つ）を持つ使い捨ての compose project は、空きが尽きた今の機械でもネットワークを作れた（実測）/ 依頼の完了の判定 1「ほかの worktree の DB には触らない」（docs/changes/CH06.md:31-32）/ CH04 の Q4「使われていない volume は何もしない」（ch04-test-db-growth/deep.md）
- kind: defer
- 提案: 族 (d)「ネットワークがそもそも取れない」の問いを足す。CH06 の判定 2・3 の証跡は DB が起動しないと取れない（BLOCKED_INFRA）ので、下流の入口より前に決める必要がある。選択肢の例: (1) CH06 の外で、消えた worktree の compose project を 1 回片付ける（依頼の「ほかの worktree の DB には触らない」は生きている worktree の話と読む）、(2) CH06 の範囲に、worktree ごとの subnet を台帳（`tools/ports.sh` の `ashiato-ports`）で割り当てる形を足す（被らない割り当て。依頼の範囲を越える）、(3) Docker の `default-address-pools` を広げる（OS の設定。人間の作業）。(1) と (2) は依頼の本文に触るので、仮で進めずに問いにする
- 処置: fixed deep-questions.json —— 問い Q3（Docker のネットワークの空きが尽きている。族「作り直しが邪魔をする・できない場合」）を足した。推奨は worktree ごとの subnet を台帳の番号から割り当てる（被らない割り当て）。premises.md に P10 を足した

## R2. C1（worktree でない木では作り直さない）と C4（判定 3 の検査を CI の rust job で走らせる）がぶつかる。CI の checkout は「worktree でない木」なので、CI では作り直しが起きず、検査が落ちる
- 成果物: openspec/changes/ch06-reset-verify-db/premises.md
- 根拠: `tools/ports.sh:48-50`（git-dir と共通ディレクトリが同じ木 = main の checkout。注記 `ports.sh:6` が「CI もここ」と書く）/ `.github/workflows/ci.yml:62-68`（`actions/checkout` の普通の clone、`docker compose up -d --wait db`）/ `tools/ci-db-env.sh`（CI の URL は 55432）/ さらに CI には `.env` が無いので、`tools/verify-env.sh:12` の `tools/env-init.sh` が乱数の合言葉で `.env` を作り（`env-init.sh:12-28`）、`tools/db-roles.sh:22-23` はその `.env` を読んで役割の合言葉を `$GITHUB_ENV` と違う値に揃える —— 同じ job の後ろの段（`$GITHUB_ENV` の `DATABASE_OWNER_URL` で繋ぐもの）が認証で落ちうる
- kind: conflict
- 提案: 問いにはしない（失う側は main の checkout の DB だけで、どちらを選んでも C1 の「触らない」は保てる）。premises.md の C1 に見分け方を書き直す: 例として、作り直しの道を検査から通すために検査が使い捨ての `git worktree add` の中で `HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ、または CI を見分けて作り直す側に倒す。C4 の検査は「worktree でない木では作り直さない（行が残る）」も同じ台本で見る（C1 の取り違えが main の DB を消す側に倒れないことを機械で固定する）。CI で `.env` を作らない・`$GITHUB_ENV` と食い違わないことも C4 に書く
- 処置: fixed D2 仮 —— design D2（仮）に判定の表と反転条件を書いた（spec の段）。questions の段では premises.md の C1 を「main の checkout では作り直さない。ただし CI（`CI=true`）は作り直す側」に直し、C4 に CI でない main の checkout で行が残ることと、CI の合言葉と食い違わないことを spec で決めると書いた

## R3. C2 の前提（ネットワークを手放すのは `tools/stack.sh` の `down -v` だけ）が誤り。検証コマンドとして走る `smoke.sh`・`check-immutable.sh`・`check-panic-log.sh` も毎回 `down -v` で手放すので、C2 だけでは「次の `up` で取れない」を防げない
- 成果物: openspec/changes/ch06-reset-verify-db/premises.md
- 根拠: `tools/smoke.sh:31,38`・`tools/check-immutable.sh:10`・`tools/check-panic-log.sh:15` の `docker compose down -v` / これらは verify-run の検証コマンドとして走る（`ashiato2-st23/openspec/changes/st23-record-erasure/tasks.md:51` の `tools/check-immutable.sh` など。CI では `.github/workflows/ci.yml:271-274`）/ 実測（使い捨て project）: `docker compose rm -s -f -v db` の後もネットワークは残り、`docker compose down -v` の後は消えた —— P4 の後半「`down` がネットワークを消す」は正しく、確かさは 4 に上げられる
- kind: premise
- 提案: C2 の文を「CH06 の作り直しはネットワークを手放さない。ただし `stack.sh`・`smoke.sh`・`check-immutable.sh`・`check-panic-log.sh` の `down -v` は手放す（この change では触らない）」に直し、Q3 の context の「今の起動と同じ条件」の後ろに、これらの検証の直後は毎回ネットワークを取り直すことを足す。R1 の問いで (2)（subnet の割り当て）を選ばない限り、この危うさは残ると proposal に書く
- 処置: fixed premises.md —— P4 に `smoke.sh`・`check-immutable.sh`・`check-panic-log.sh` の `down -v` を足し確かさを 4 に上げた。C2 に「これらは触らない・Q3 で subnet を選ばない限り危うさが残る」と書いた。Q3 の context にも書いた

## R4. Q1 の context の「CH06 だけが足すもの」の (3) が誤解を招く。`tools/verify-env.sh` も各 worktree の木から読まれるので、CH06 は CH04 と同じく rebase した worktree にしか効かない
- 成果物: openspec/changes/ch06-reset-verify-db/deep-questions.json
- 根拠: `scripts/evidence.py:377`（`prep = root / VERIFY_ENV`。走らせるのは検証している worktree の中の台本）/ CH04 の `testdb.rs` も同じく worktree の木から組まれる / CH04 の下流は Task 10 の途中（`ashiato2-ch04/openspec/changes/ch04-test-db-growth/tasks.md` は `[x]` 26・残りは 10.1〜10.4）で、CH06 の上流は始まったばかり —— CH06 が CH04 より先に merge される見込みは低い
- kind: premise
- 提案: (3) を「CH04 より先に merge されたときだけ、先に rebase した worktree に効く（どちらも取り込んだ worktree にしか効かない）」に直すか、消す。Q1 は本人が目を通す唯一の問い（note の 2 つ目）なので、推奨の理由に効く記述を正しくする
- 処置: fixed deep-questions.json —— Q1 の context の (3) を「どちらも rebase した worktree にしか効かない。CH04 が先に merge される見込みが高い」に直した

## R5. Q4（作り直す頻度）は依頼で決まっていて幅が無い
- 成果物: openspec/changes/ch06-reset-verify-db/deep-questions.json
- 根拠: docs/changes/CH06.md:15（依頼「検証（`scripts/verify-run`）のたびに、その worktree のテスト用 DB を空から作り直す」）/ 同 :31（完了の判定「`HARNESS_VERIFY=1` のとき … 作り直してから」）/ `scripts/evidence.py:374-382`（印はどの検証の前にも渡る）/ 案 2（10 分より古いときだけ）は「その間の検証は前の検証の残りの上で走る」で、依頼の逐語と判定 1 に反する
- kind: defer
- 提案: Q4 を外し、C に「毎回作り直す（依頼の逐語）」を足す。費用は R8 の実測を C の理由に書く
- 処置: fixed deep-questions.json —— Q4 を外し、premises.md に C5（毎回作り直す。費用は P9）を足した

## R6. Q3（作り直せなかったとき）の案 2 はハーネスの約束と依頼の理由に反し、幅が無い
- 成果物: openspec/changes/ch06-reset-verify-db/deep-questions.json
- 根拠: `scripts/evidence.py:28-31`（「証跡は溜まった状態に頼らない」）・`:374`（印の意味）・`:384-388`（準備が非 0 なら BLOCKED_INFRA で検証を走らせない）/ docs/changes/CH06.md:26-27（依頼はこの約束を理由に挙げている）/ `tools/verify-env.sh:6`（準備できなければ非 0 で終わる、が今の取り決め）
- kind: defer
- 提案: Q3 を外し、C に「作り直せなければ非 0（BLOCKED_INFRA）」を足す。人間に見せる価値が残るのは、R1 のネットワークが取れない場合の扱いのほうなので、そちらの問いに寄せる
- 処置: fixed deep-questions.json —— 旧 Q3（作り直せなかったとき）を外し、premises.md に C6（非 0 で BLOCKED_INFRA）を足した。ネットワークの問いは新しい Q3 に寄せた

## R7. 作り直しの手段によっては、古い匿名 volume が消えずに残る（実測）。Q4 の「毎回」と掛け合わせると容量に効くのに、手段も検査も決まっていない
- 成果物: openspec/changes/ch06-reset-verify-db/premises.md
- 根拠: `docker-compose.yml` は volume を宣言しない（`postgres:17-alpine` の `VOLUME` による匿名 volume）/ 実測（使い捨て project）: `up -d --force-recreate -V` は古い volume を残した。一度 `up --force-recreate`（設定の変化・rebase で compose が作り直すときと同じ）を経たコンテナは、`rm -s -f -v db` でも `down -v` でも volume が残った。作り直しを経ていないコンテナは `rm -s -f -v db` で消えた / いま `docker volume ls -qf dangling=true` は 66 個（CH04 の P5 の「62 個」と同じ現象）
- kind: daily
- 提案: 問いにはしない。C2 に手段を書き足す: 消す前に `docker inspect` で DB のコンテナの volume の名前を取り、コンテナを消した後にその volume を名指しで `docker volume rm` する（`-V` は使わない）。C4 の検査に「作り直しの前後で、この project の volume の数が増えない」を足す
- 処置: fixed D1 仮 —— design D1（仮）に手順と反転条件を書いた（spec の段）。questions の段では premises.md の C2 に volume の名前を `docker inspect` で取り名指しで消す手段を書き、C4 に volume の数が増えないことを足した。P11 を足した

## R8. Q1・Q4 の費用（作り直し 1 回「数秒〜十数秒と見込む（測れていない）」）は、ネットワークの無い使い捨て project で測れる。測った値と、検証の回数を context に置く
- 成果物: openspec/changes/ch06-reset-verify-db/deep-questions.json / openspec/changes/ch06-reset-verify-db/premises.md
- 根拠: 実測（`network_mode: none`・同じ image・同じ `max_connections=300`・同じ healthcheck）: `rm -s -f -v db` → `up -d --wait db` が 5.1 / 5.5 / 5.5 秒（役割の `db-roles.sh` と port の公開は含まない）/ `ashiato2*/openspec/changes/*/evidence.jsonl` の 2026-10-17〜10-24 の検証は 602 回 → 作り直しの合計は 1 週間で約 1 時間。依頼の server の cargo test の 53.5h（docs/changes/CH06.md:25）と並べられる
- kind: premise
- 提案: premises.md に費用の行を足し（確かさ 3: 同じ image を手元で走らせたが、port の公開と役割は含まない）、Q1 の context の「測れなかった」を置き換える
- 処置: fixed premises.md —— P9（約 5 秒・602 回・週に約 1 時間。確かさ 3）を足し、Q1 の context の「測れなかった」を置き換えた

## R9. P2 の行番号の誤りと、P5・P6 の確かさの上げ漏れ
- 成果物: openspec/changes/ch06-reset-verify-db/premises.md
- 根拠: P2 は `tools/stack.sh:36-37` を挙げるが、そこは `WEB_PORT` の行と空行。作り直しは `tools/stack.sh:75-76` / CH04 の回の database と助言ロックは `ashiato2-ch04/crates/server/src/testdb.rs:12-18`（回の id と作る database）・`:155`（回の助言ロック）・`:196`（片付けの `pg_try_advisory_lock`）・`:212`（`_tmp_`）に実装がある（P5・P6 は design を読んだだけの確かさ 2）
- kind: premise
- 提案: P2 の確かめ方を `tools/stack.sh:24,75-76` に直す。P5・P6 の確かめ方に CH04 の worktree の実装を足し、確かさを上げる（どちらも B の Q1・Q2 の土台なので、Q の結論は変わらない）
- 処置: fixed premises.md —— P2 を `tools/stack.sh:24`・`:75-76` に直し、P5・P6 の確かめ方に `ashiato2-ch04` の `testdb.rs` を足した（P6 は確かさ 3）
