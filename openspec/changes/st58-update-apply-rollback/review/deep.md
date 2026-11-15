# ST58 深掘りの問いの独立検査（deep-review）

対象: `openspec/changes/st58-update-apply-rollback/deep-questions.json`（Q1〜Q8）・`premises.md`（P1〜P16）。
手順 1〜8 を一覧を見る前にやり直し、その後で突き合わせた。指摘は R1〜R15。

## 手順ごとの記録

- 手順 1（要件どうしの衝突）: FR-114 の「Windows のインストーラ」と PC の収集アプリ（zip の実行ファイル）→ Q2 が扱う。
  FR-119 の「戻して消える記録は無い」は「更新の間はサーバが止まっている」ことと「この回の退避を戻す」ことに依るが、どちらも今のコードでは成り立たない → R1・R3・R10。
  FR-120 の知らせと、凍結した更新の履歴（ST51。`outcome` の CHECK）→ R5
- 手順 2（扉の幅）: doors は空。要件本文の幅として「S-01 の更新の手順を開く」（Q4）、「Windows のインストーラ」（Q2）、「移行が失敗する」の範囲（移行が通った後に新しい版が起きない回を含むか）→ R4
- 手順 3（一方通行）: 戻しで DB を消して入れ直すこと（戻す退避を取り違えると記録を捨てる）→ R1・R3。「戻した」の行の形（追記のみ・書き出しに入る）→ R5。
  退避の写しを残す・消す → R6。同梱の PostgreSQL の入れ替え（DB の置き場の名前に大きい版を含む）→ R8。
  失敗した DB そのものは「退避 + 同じ新しい版の移行」で作り直せるので、捨てても失うものは無い（指摘しない）
- 手順 4（日常）: 退避の写しが更新のたびに増える容量 → R6。帯（Q5）、提供元不明のアプリの許可（Q3）は一覧にある
- 手順 5（既存コードと要件の突き合わせ）: インストーラは別の版のファイルを上書きせず止まる（P11 と逆）→ R2。走らせ直しは移行の前にサーバを止めない → R10。
  `migrate` の終了コード 1 は「この回の退避がある」を意味しない・退避の場所を呼び手に返さない → R1。携帯に `REQUEST_INSTALL_PACKAGES` と `MY_PACKAGE_REPLACED` の受け手が無いこと（AndroidManifest.xml）は Q1・Q3 の context が正しく書いている
- 手順 6（過去の取りこぼし）: 「削除が届かない D-01 の写し」→ R6。「書き直せない記録の形を新しく決めるときは問いにする」→ R5。
  「申し送りの全項目を突き合わせる」→ ST57 の最後の項目（戻したときの更新係）が一覧に無い → R7。
  「示すことを条件にした決定は、示せないときの向きを持つか」→ R11。「時刻を入力にする決定は時計が戻る 1 件を問う」→ R1（退避の名前が壁時計）
- 手順 7（族の網羅）: Q6 の族「更新の途中で止まる」に、移行が通った後に新しい版が起きない・終了コード 2・終了コード 1 でもこの回の退避が無い・止まったことを本人がどう知るか、が無い → R4。
  Q2 の族に「新しい版の PC の収集アプリが起きない」が無い → R12。Q1 の兄弟「端末の起動の後」は context で外しただけで行き先が無い → R15
- 手順 8（前提の確かさ）: P11（確かさ 3）がコードと合わない → R2。Q2 の推奨が確かさ 1 の P4 に乗る → R12。Q1 の推奨が確かさ 2 の P6・P7 に乗り、context の「裏の位置の権限をもう持っています」は誤り → R11。
  「終了コード 1 ならこの回の退避がある」（Q6 の族）と「戻した行を更新の履歴に書ける」（Q5）は premises に行が無い → R1・R5
- 構成の記号: `grep -nE '\b(S|C|V|D)-0[0-9]\b' deep-questions.json` は 0 件。該当なし
- 収集アプリを入れ替えた後の未送信と設定の引き継ぎ: 該当なし（確かめた範囲: PC の未送信は `LOCALAPPDATA` の置き場で実行ファイルの場所に依らない（`crates/collector-windows/src/config.rs:146`）。
  送信契約の版は積んだときに行ごとに焼き込まれて書き換えられない（`crates/collector-windows/src/contract.rs:401-403`・`outbox.rs:134`）ので、古い版が新しい行を古い契約の版で送ることは無い。
  携帯は同じ署名の更新でアプリのデータが残る。更新の確認を切った設定はインストーラの走らせ直しで上書きされない（`crates/installer/src/main.rs:210-218`）

---

## R1. [前提の誤り・抜け] 終了コード 1 は「この回の退避がある」を意味しない。戻す退避をどう特定するかの問いが無い
- 種別: 前提の誤り（Q6 の族）＋ 抜け
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: crates/server/src/migration_ledger.rs:528-547（錠を取れないと退避の前に `MigrateError::Db`）/ crates/server/src/migration_ledger.rs:110-116（`Before`（退避の失敗）は `failure_line` を持たず、`main` の `Err` で終了コード 1）/ crates/server/src/lib.rs:400-407（接続の失敗も退避の前に `?` で終了コード 1）/ crates/server/src/lib.rs:321-326（退避の完了の文は置き場の定数だけで、この回のファイルの名前を出さない）/ crates/server/src/lib.rs:411（ファイルの名前は壁時計の UTC）/ crates/server/src/migrate_backup.rs:113・166-169（同じ秒なら `truncate(true)` で上書き）/ deep-questions.json Q6 の family「移行が終了コード 1 で落ちた（この回の退避がある）」/ docs/handoff/ST58.md の st48 R21「終了コード 1 … こちらはその回の退避がある」
- kind: premise
- loss: discarded
- 提案: Q6 の族の 1 行目を「終了コード 1 でも、この回の退避が無い回がある（接続・錠の失敗、退避の失敗）」に直す。戻すのは「この回に取った退避」と特定できたときだけにする。特定の仕方（`migrate` が退避の場所を呼び手に返す・回ごとに空の置き場を渡す・更新の途中の印に書く）を問いとして足す。いちばん新しい名前のファイルを選ぶ形は、時計が戻ったときと 2 回目の試み（1 回目で半分当たった DB を退避する）で、前の退避か半分当たった退避を戻してしまう。失わない側（特定できなければ戻さずに止める）は安いので、P として台帳に載せてよい。ただし一覧には挙げる
- 処置: escalated —— Q5 の族と context に「この回の退避と特定できたときだけ戻す」を置いた。既定は defaults.md C2・design D2

## R2. [前提の誤り] P11「インストーラは今、プログラムの置き場を新しい版で上書きする」は逆。別の版のファイルがあれば何も替えずに止まる
- 種別: 前提の誤り（premise）
- 成果物: openspec/changes/st58-update-apply-rollback/premises.md
- 根拠: crates/installer/src/main.rs:883-904（`program_file_plan` は置き場に無いファイルだけを写し、中身が違えば `CopyPlanError::Differs`）/ crates/installer/src/main.rs:840-861（`Differs` で `program_version_mismatch_message` を返して止まる）/ crates/installer/src/commands.rs:245-248（「版を替える口はまだ無いので、何も替えずに止めます」）/ docs/install.md:52 / premises.md P11（確かさ 3）・P10（「新しい版のサーバは `kind=db_behind` で起動しない」。実際は main.rs:1240 の写しの段で止まり、移行の段（main.rs:1303）にも、サーバの起動にも届かない）
- kind: premise
- 提案: P11 を「別の版のファイルがあれば止まる（上書きも退けもしない）」に直し、P10 の帰結を「写しの段で止まる」に直す。Q6 の context の「前の版の実行ファイルを置き場に戻せない」は、どの順で入れ替えるか（新しい版の `migrate` を展開した zip から先に走らせ、通ってから置き場を替える／前の版を退けてから替える）に依る。順序の決定を design の D に置き、Q6 がそれを前提にしていることを書く
- 処置: fixed premises.md P10・P11（別の版があれば写しの段で止まる）。入れ替えの順序は defaults.md C1

## R3. [分類違い] Q6 は「失うものは無い」ではない。戻しの途中で止まった DB にサーバのサービスが自動で起き、受け取った記録を続きの戻しが捨てる
- 種別: 分類違い（`open` → loss あり）＋ 前提の誤り（`repair` の口）
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: openspec/specs/distribution/spec.md:1435-1439（サーバと DB のサービスは開始の種類「自動」。異常に終わればサービスの回復で起こし直す）/ crates/installer/src/commands.rs:958-976（`restart/10000` を 3 回、`failureflag 1`）/ openspec/specs/distribution/spec.md:154 以下（起動の検査は版の記録だけを見る。`pg_restore` は表の中身を入れてから制約とトリガを作るので、版の記録は戻ったが中身や門がまだの DB で検査を通りうる）/ deep-questions.json Q6 の why「退避のファイルと前の版の実行ファイルを消さない限り、後から何でもやり直せます」/ Q6 の推奨の detail「インストーラの `repair` の口を示して止まる」と docs/install.md:44・spec.md:1759（`repair` は DB はあるのに秘密が無いときの口で、記録に触らない。半分戻った DB を戻す口ではない）
- kind: irreversible
- loss: discarded
- 提案: Q6 に「更新・戻しの間、サーバのサービスが自分で起きないようにするか（開始の種類を手動にして最後に戻す・別の名前の DB に戻してから入れ替える など）」を族の 1 行として足し、loss を付ける。途中で電源が落ちると、PC の起動のときにサーバが半分戻った DB で起き、記録を受け取って 200 を返す（収集アプリは未送信から消す）。その後に「印を見て続きから」の戻しが DB を入れ直すと、それらの記録が捨てられる。推奨の detail の `repair` は「戻しをやり直す口」に直す
- 処置: escalated —— Q5。loss: discarded の A に直した

## R4. [抜け（族）] Q6 の族に、移行が通った後に新しい版が起きない回・終了コード 2・止まったことを本人が知る経路が無い
- 種別: 抜け（族の網羅）
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: crates/installer/src/main.rs:1340-1350（移行の後にサーバを起こし、180 秒で待ち受けが開かなければ「サーバが起きない」で止まるだけ）/ docs/requirements.md:1047（FR-119 の条件は「移行が失敗する」だけ）/ crates/server/src/net_guard.rs:89-93・crates/server/src/lib.rs:403（DB の接続先が手元でなければ終了コード 2。退避も書き込みも無い）/ deep-questions.json Q6 の family（終了コード 1・3・4 と途中の停止だけ）/ docs/requirements.md:1049（FR-120 は FR-119 で戻したときだけ。終了コード 3・4・退避の失敗で入らなかった回の知らせは決まっていない）
- kind: irreversible
- loss: discarded
- 提案: 族に 3 行を足す。(a) 移行は通ったが新しい版のサーバが起きない・すぐ落ちる。DB は新しい版なので前の版は `db_ahead` で起きない。戻すかどうかと、戻すなら新しい版が 1 件でも受け取った後に戻すと捨てることになる点を問う（受け取る前＝待ち受けを開く前に落ちた回だけ戻す、など）。(b) 終了コード 2 と、0・1・3・4 以外（落ちた・殺された）の扱い。(c) 入らなかった・途中で止まったことを、窓を閉じた後に本人がどう知るか（サーバが止まっていれば画面の帯は出ない）。Q5 の族として、終了コード 3・4・退避の失敗で「入らなかった」ことの知らせも並べる
- 処置: escalated —— Q5 の族・Q4 の族

## R5. [抜け・前提が誤りうる] 「戻した」を更新の履歴にどう書くか（凍結した表の形）の問いが無い。Q5 の「戻した記録は更新の履歴に残る」は、ST58 より前の版へ戻す回には成り立たない
- 種別: 抜け（一方通行: 書き直せない記録の形）＋ 前提が誤りうる（Q5）
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: migrations/202610300900_update_history.sql:10・13（`via IN ('migrate','startup')`・`outcome` は 5 つだけ）/ openspec/specs/distribution/spec.md:465-479（形は版をまたいで変えない。足すだけ）/ openspec/specs/distribution/spec.md:495-500（書き換えも消しもできない）/ docs/handoff/ST58.md の st51 R1（「列か値を足す」。CHECK も作り替える）/ deep-questions.json Q5 の why「戻した記録は更新の履歴に残る」・context「書き出しに入る」
- kind: irreversible
- loss: rewrite-all
- 提案: 問いを足す。「戻した」の表し方（`outcome` に値を足す／`via` に `rollback` を足す／列を足す）、書き直す「落ちた試みの結果」の行の `via` と `at`（`migrate` が書いたように見せるか、戻しの時刻か）。行は追記のみで書き出しに出るので、一度書いた形は書き直せない。あわせて premises に 1 行足す。戻した DB は前の版の形なので、「戻した」の値を足す移行を持たない版（ST58 を初めて入れる回の失敗を含む）へ戻すと、CHECK が断って書けない。そのとき FR-120 の知らせの元をどこに置くか（状態の置き場のファイルなど）が決まらないと、窓を閉じた後に戻したことの跡が残らない。Q5 を B のまま置くかは、この前提を確かめてから決める
- 処置: escalated —— Q6。loss: rewrite-all

## R6. [抜け] 移行の前の退避をいつまで残すかの問いが無い。更新のたびに DB 全体の写しが増え、記録の削除が届かない
- 種別: 抜け（日常: 容量 ＋ 一方通行: 写しを捨てる）
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: crates/server/src/migrate_backup.rs:102-150（毎回 `pg_dump --format=custom` で DB 全体を 1 ファイル。消す処理は無い。`grep -n remove_file crates/server/src/migrate_backup.rs` は失敗の時の 139 行だけ）/ docs/requirements.md:948（FR-70 の 30 日の保持は定時のバックアップだけ。FR-119 の退避は「FR-66 とは別」で保持の定めが無い）/ docs/requirements.md:721-726（削除の確認に出すのは書庫の写しだけで、移行の前の退避は無い）/ openspec/specs/distribution/spec.md:1683（`migrate` が走るのはインストールと更新のとき。ST58 から更新のたびに走る）/ `upstream_misses.py report` の「DB や写しを増やす決定の問いに、その写しに本物の記録が入るか・削除が届くかを確かめる項目を足す」
- kind: daily
- loss: discarded
- 提案: 問いを足す。退避を何個・何日残すか（戻しが要るのはこの回の分だけ）。消せば戻せなくなる（discarded）。残せば、更新のたびに DB 全体の大きさの分だけ空きが減り、本人が消した記録の本文も写しに残る。開発版を毎日入れる場合も挙げる。削除の確認に「移行の前の退避 N 個にも入っている」を出すかも並べる
- 処置: escalated —— Q7。loss: discarded

## R7. [抜け（申し送り）] ST57 の申し送り「前の版へ戻したときに更新係のサービスをどう扱うか」が一覧に無い
- 種別: 抜け（申し送りの取りこぼし）
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: docs/handoff/ST58.md の「ST57 — 更新の確認を引き継ぐ事項」の最後の項目 / crates/installer/src/main.rs:852-861（更新係の実行ファイルも同じ写しの規則）/ openspec/specs/distribution/spec.md:1436（更新係は「自動」のサービス）/ openspec/specs/distribution/spec.md:2872-2875（サーバは更新係が書いた結果を読むだけ）
- kind: technical
- 提案: B の問いとして足す（推奨つき）。更新の間に更新係を止めるか、戻した後に更新係はどちらの版で動くか（新しい版の更新係が書いた結果を前の版のサーバが読めるか）、戻した後も同じ版を「更新あり」に出し続けるか。切った更新の確認（`enabled: false`）が戻しでも切ったまま、を Scenario にする
- 処置: fixed defaults.md C4

## R8. [抜け] 同梱の PostgreSQL の大きい版が違う配布物で更新したときの扱いが無い（DB の置き場の名前が `db\17`）
- 種別: 抜け（入れ替えるものの範囲）
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: crates/installer/src/layout.rs:23（`db_dir: C:\ProgramData\ashiato\db\17`）/ crates/installer/src/commands.rs:896-903（写すものに `pgsql` を含む）/ tools/release/pgsql.lock:2（`version=17.6-1`）/ crates/installer/src/state.rs:39-41（DB が無ければ `InitDatabase`）/ crates/installer/src/main.rs:1303（移行の完了の印があれば移行を飛ばす）
- kind: technical
- 提案: B の問いとして足す（推奨つき）。コードを辿った限りでは、大きい版が違う配布物で ST58 の更新の道を通すと、`db\<新しい版>` に空の DB を作りうる（記録のある DB は `db\17` に残ったまま、サーバは空の DB に繋ぐ）。ST58 では「同梱の PostgreSQL の大きい版が違う更新は止める」とし、`pg_upgrade` は版を上げる Story で扱う、を推奨にできる。小さい版の入れ替え（DB のサービスを止めて `pgsql` を替える）と、戻すときに前の `pgsql` も戻すかも同じ問いに含める
- 処置: fixed defaults.md C5（大きい版が違う更新は止める）

## R9. [抜け] DB を戻しても、D-01 の外の状態（設定・更新係の置き場・「アプリと機能」の版）は戻らない。どれを戻すかが無い
- 種別: 抜け
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: crates/server/src/service.rs:226-233（設定の値が文字列でなければ `Malformed` でサーバが起動しない。知らない鍵は通す）/ crates/installer/src/main.rs:1155-1166（走らせ直しは既にある設定を読み、書き直さない）/ crates/installer/src/main.rs:1351-1355（「アプリと機能」に `ASHIATO_VERSION` を書く）/ docs/requirements.md:1047（FR-119 が戻すのは D-01 と実行ファイルだけ）
- kind: technical
- 提案: B の問いとして足す（推奨つき）。新しい版が設定の形を変えたら前の版で起きない。どの版でも設定は「足すだけ・文字列だけ」と決めるか、戻しで設定の写しも戻すか。戻した後の「アプリと機能」の版表示をどうするかも並べる
- 処置: fixed defaults.md C6

## R10. [前提] 今の走らせ直しは、移行の前にサーバを止めない。FR-119 の「戻して消える記録は無い」はこれに依る
- 種別: premise（既存コードが要件の前提を満たしていない）
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: crates/installer/src/main.rs:1169-1175（サーバを止めるのは設定がまだ無い初回だけ）/ crates/installer/src/main.rs:1299-1307（移行）/ crates/installer/src/main.rs:1340-1345（移行の後で止めて起こし直す）/ docs/requirements.md:1048（FR-119 の ★「更新の間 S-01 は止まっていて…戻して消える記録は無い」）/ docs/handoff/ST58.md の st48 R21（「新しい版の migrate の前に古い S-01 を止める」）
- kind: premise
- loss: discarded
- 提案: 問いは足さない（申し送りで決まっている）。Q6 の族と context に「退避の前にサーバを止める（決まっている）」を 1 行置く。止めずに走らせると、退避の後に前の版が受け取った記録を戻しが捨てることを書き、design と試験（移行の間に `/healthz` が答えない）へ写す先を明記する
- 処置: escalated —— Q5 の context に「退避と移行の前にサーバを止める（申し送り st48 R21）」を置いた。design D1

## R11. [前提の誤り・分類] Q1 の「アプリは裏の位置の権限をもう持っています」は誤り。推奨の失敗時の通知も、出せないときがある。失わない側は安いので P の候補
- 種別: 前提の誤り ＋ 分類（A → P の候補）＋ 手順 8
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:172-176（裏の位置は「あるとよい」権限で、断られても収集を始める）/ MainActivity.kt:409-411（無いときは `no_background_location` で縮退して続ける）/ MainActivity.kt:177-179（通知の権限も「あるとよい」）/ premises.md P6・P7（確かさ 2）/ deep-questions.json Q1 の context「アプリは裏の位置の権限をもう持っています」・推奨の detail「再開できなかったとき…通知を 1 回出す」
- kind: premise
- loss: uncaptured
- 提案: context を「裏の位置の権限を持つ端末では」に直し、持たない端末で受け手が前景のサービスをどう起こすか（位置を除いた種別で起こす・起こさない）を推奨の detail に書く。再開に失敗したときの通知は、通知の権限が無い端末では出ない。そのときの向きを書く。失わない側（自分で再開する）は受け手 1 つで、範囲（st51 R10 の申し送り）の内にあり、要件を変えない。A に置く理由が無ければ P にして台帳に載せる。どちらでも P6・P7 は確かさ 2 のままなので、エミュレータで 4 に上げる task を P の確かめの条件にする
- 処置: fixed D10 仮（defaults.md P1 から台帳へ）

## R12. [手順 8・抜け（族）] Q2 の推奨は確かさ 1 の P4 に乗る。新しい版の PC の収集アプリが起きないときの向きが選択肢の不可逆の欄に無い
- 種別: 前提が誤りうる ＋ 抜け（族）＋ 不可逆の記述の不足
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: premises.md P4（「知識」・確かさ 1。動いている実行ファイルの名前を変えられる）/ deep-questions.json Q2 の推奨の detail「動いている自分を止め、前の版を残したまま入れ替え、自動起動の台本を新しい版に向けて起こし直す」・irreversible「止まるのは数秒」/ crates/collector-windows/src/autostart.rs:29-41（自動起動の台本は実行ファイルの場所を 1 つ持つ）
- kind: premise
- loss: uncaptured
- 提案: P4 を Windows の実行時の試験か手元の Windows で確かめ、確かさ 4 以上にしてから推奨にする。推奨の irreversible に「新しい版が起きない・すぐ落ちると、自動起動の台本は新しい版を指したままなので、本人が気づくまで窓・入力・アドレスバーの記録が取れない」を足す。残した前の版へ自分で戻すか（起きたことを確かめてから台本を書き換える、など）を detail に書く
- 処置: escalated —— Q1。新しい版が動き出さなければ前の版に戻す detail と irreversible を足した

## R13. [前提] Q7 の「入れようとしたら」が、Q2 の推奨の道には現れない（通知が出すのは新しい版だけ）
- 種別: 前提（Q2 に従う）
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: deep-questions.json Q2 の推奨（通知を押すと新しい版へ入れ替える）/ Q7 の question「入っている版より古い版を入れようとしたら」/ docs/handoff/ST58.md の ST52 D5（仮）・R14 / premises.md P15（PC の収集アプリにインストーラが無い）
- kind: technical
- 提案: Q7 に `follows: Q2` を置き、確かめの場所を書く（Q2 の 1・3 なら入れ替える役が版を比べる。2 なら手で置き換えるので確かめる場所が無く、起動のときに置き場の「最後に動いた版」と比べるしかない）。未送信を古い形で送らないことは、行ごとに焼いた契約の版が守っている（crates/collector-windows/src/contract.rs:401-403・outbox.rs:134）。そのため「失うものは無い」は成り立つ。context の根拠にこの 2 か所を足す
- 処置: fixed defaults.md C7（場面が推奨の道に現れないので問いから外した）

## R14. [不要] Q8（Linux のサーバの更新）は ST57 の D10 と ST60 への申し送りで決まっていて、幅が無い
- 種別: 不要
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: openspec/changes/archive/2026-11-14-st57-update-notice/design.md:105-111（Linux の配布物は変えない。Linux の更新係は ST60 へ）/ docs/handoff/ST60.md:13-15 / openspec/specs/distribution/spec.md:2961（Linux のサーバは帯を出さないので、FR-114 の「押す」が起きない）/ docs/stories/ST60.md:4（`satisfies: [FR-131]`）
- kind: defer
- 提案: Q8 を問いから外し、聞かない既定（C）として「Linux の更新と戻しは ST60 の範囲。ST58 は Windows だけ」と書く。ST60 の申し送りに「更新と戻し（FR-114・FR-119）も」を 1 行足す
- 処置: fixed defaults.md C8（問いから外した。ST60 の申し送りへの 1 行は spec の段）

## R15. [抜け（族）] 携帯の「端末の起動の後も開くまで収集が止まる」は Q1 の context で外しただけで、行き先が無い
- 種別: 抜け（族の兄弟の行き先）
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json
- 根拠: collector-android/app/src/main/AndroidManifest.xml:129-135（`BOOT_COMPLETED` の受け手は気分の通知だけ）/ MainActivity.kt:413（`LocationService` を起こすのは画面だけ）/ deep-questions.json Q1 の context「起動の後も扱うかは、別の change で決めます」/ docs/requirements.md NFR-12（Google 系以外のソースは手作業を要さない）
- kind: defer
- loss: uncaptured
- 提案: ST58 の範囲の外として外すのはよい。ただし「別の change で決めます」で終わらせず、`hx change` の依頼か `docs/handoff/` の行として行き先を作る。premises の「他の Story へ回すもの」に書き、Q1 の context にその名前を書く。Q1 の答えが「自分で再開する」なら、同じ受け手の形で起動の後にも広げられることを添える
- 処置: escalated —— 範囲の外として本人に返す X1。deep.md に名指し
