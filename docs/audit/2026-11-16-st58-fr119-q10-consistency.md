# 整合の確認 — FR-119 に ST58 の深掘り第 3 回の答え（Q10）を足した（2026-11-16）

- 基準: `a0da6284f5df3f67855b69f087966ce7f3d543b5`..`58b9d48aa232e28b7922347048744ee456f55e8d`

## 機械の入力（写し）

`python3 scripts/req_impact.py . --base a0da6284f5df3f67855b69f087966ce7f3d543b5`（改訂した要件 1 件・扉 0 項）:

- FR-119（改訂）: `docs/handoff/ST51.md:15`・`docs/handoff/ST58.md:14,23,52`・`docs/handoff/ST60.md:19,21`・`docs/stories/INDEX.md:82`・`docs/stories/stories.json:1396`・
  `openspec/changes/st58-update-apply-rollback/deep-questions.json:129`・`deep.md:33,50,117`・`premises.md:22`・`proposal.md:6,24,34`・
  `review/deep.md:9,61,79,104,109,112,149`・`review/spec.md:275,278,279`・`specs/distribution/spec.md:238,239,290,325,357,381,534,560,613`

`python3 scripts/check_chain.py .`（chain: FAIL 1 件 / 未回収 0 件 / warn 0 件）:

- [FAIL] 整合の確認: 整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-119（この監査がそれに当たる。★ 2026-11-16 追加はこのファイル名を名指ししている）
- 検査 10（判断の日より新しい訂正）は何も出していない。ST58 は satisfies に FR-119 を持ち、判断の日は 2026-11-15（stories.json:1419）で ★ は 2026-11-16 だが、検査 10 は「まだ change の無い Story だけ」（`scripts/check_chain.py:209`）を見るので、走っている change を持つ ST58 は対象外（下の C1）

走っている change（`openspec/changes/` の archive 以外）は `st58-update-apply-rollback` の 1 本だけ。tasks.md は 11.1 まで `[x]` で、古い D23（仮）（落ちた DB は次に更新が通ったときに消す）の形でコードがもう書かれている（K2）。
失われるもの: なし —— 配る版はまだ出ておらず（12.1・12.2 が `[ ]`）、`main` に `ashiato_failed` は 1 件も無い（`git grep -n ashiato_failed main` は 0 件）ので、古い形で残った落ちた DB がある PC は無い。書き直すのは 4.1・5.1・6.1 の一部で、全部ではない。

## 1. 改訂後の要件と、ほかの文書

## C1. ST58 の完了の判定に Q10（戻した後、前の版が待ち受けを開いたら落ちた DB が残っていない）が無く、検査 10 も走っている change を持つ ST58 を見ない
- 重大度: 中
- 文書: docs/stories/stories.json:1414
- 要件: FR-119
- 根拠: requirements.md:1052「**戻した回の落ちた D-01（入れ替えて退けた写し）は、前の版の S-01 が待ち受けを開いたら直ちに消す。**」。stories.json:1414「サーバの更新で移行が失敗するか、新しい版が待ち受けを開く前に落ちると、この回の退避から DB が戻り、前の版で起動し直す。…」・:1415「更新が通って新しい版が待ち受けを開いたら、移行の前の退避はその回の 1 つだけが残る」・:1419「"judged": "2026-11-15"」—— 落ちた DB を消す時点は判定のどこにも無い。Q10 は loss: discarded / uncaptured（deep.md の Q10）で、守るもの（本人が消した記録が見えない写しに残らない）は判定から確かめられない。前例（`docs/audit/2026-11-15-st58-fr121-q9-consistency.md` C1）は Q9 の形を done に足した
- 直すところ: stories（ST58 の done に「移行が落ちて戻した後、前の版のサーバが待ち受けを開くと、落ちた DB は残っていない」を足す → make_story.py で ST58.md を貼り直す）
- 機械: 安く捕まえられる（check_chain.py の検査 10 を、走っている change を持つ Story にも warn で広げる。いまは `check_chain.py:209` が change の無い Story に限る）
- 行き先: stories（ST58 の done に「移行が落ちて戻した後、前の版のサーバが待ち受けを開くと、落ちた DB は残っていない」を足し、make_story.py で ST58.md を貼り直した）

## C2. ★ の理由（「本人が消した記録を、本人に見えず消す口も無い写しに残さない」「失敗の原因を調べる D-01 の写しは残らず」）が、同じ FR-119 が残すと決めた移行の前の退避と食い違う
- 重大度: 中
- 文書: docs/requirements.md:1052
- 要件: FR-119, FR-50, FR-51
- 根拠: requirements.md:1052「本人が消した記録を、本人に見えず消す口も無い写しに残さない（NFR-15 の側）。失敗の原因を調べる D-01 の写しは残らず」。同じ FR の :1051「移行の前の退避は、更新が通って新しい版が待ち受けを開いたら、その回の 1 つだけを残して古いものを消す」—— 退避は DB 全体の `pg_dump`（premises P12）で、通った後も 1 つ残る。change の spec（specs/distribution/spec.md:531）「THE SYSTEM SHALL 戻した回・入らなかった回には、退避を消さない」・Scenario「戻した回は退避を消さない」（:543-547「前の 2 つの退避と、この回の退避が残っている」）—— 戻した回の退避は次に更新が通るまで残り、試すたびに増える。退避を消すのは `retain_update_artifacts`（`crates/installer/src/main.rs:1757-1776`）だけで、呼ぶのは通った 2 か所（:1344・:1734）。`docs/install.md` にも画面にも退避を消す口は無い。失敗を調べる材料についても、review/code.md:198 の提案「調べたいなら、すでにある移行の前の退避で足りる」・go.json:2300・:2343 の選択肢「調べる材料は退避に残る」と逆を言っている（退避は移行の前の D-01 で、そこから同じ移行を当て直せる）
- 直すところ: requirements（★ の理由を「落ちた D-01 は」に限り、「移行の前の退避は Q7 と spec の保持のとおり残り、消した記録もそこには残る（族の兄弟。C3）」「失敗した状態の D-01 の写しは残らないが、移行の前の D-01 はこの回の退避に残る」と書く）
- 機械: できない
- 行き先: requirements（★ の理由を落ちた D-01 に限り、移行の前の D-01 は退避に残り消した記録もそこに残る（IMP-02 で受け入れ済み）と書いた）

## C3. 族の兄弟「戻した回の退避」を「ST54 からの移行前退避 → 範囲外・本人に返す」としたが、その保持は ST58 自身の spec が決めたもので、本人に返す宛先がどこにも無い
- 重大度: 中
- 文書: openspec/changes/st58-update-apply-rollback/deep.md:122
- 要件: FR-119
- 根拠: deep.md:113「ST54 からの移行前退避 → この Story の範囲外（同じ保持期間をあわせて決める必要がある）」・:122「族の兄弟: ST54 からの移行前の退避は、この Story の範囲外のまま（Q10 の答えと同じ保持の考え方で揃えるかは、本人に返す）」。deep-questions-r3.json:10「"ST54 からの移行前退避", "decided": "範囲外。Q10 の答えと整合させる必要がある"」。出所の review/code.md:208「移行の前の退避（`backups\pre-migrate\*.dump`。ST54 から）も…戻した回の退避は次に通るまで残る —— R11 と同じ扱いを決めるときに併せて見る」。移行の前の退避は 2026-10-11 の fix（`git log --diff-filter=A -- crates/server/src/migrate_backup.rs` → `5897e72ef 2026-10-11`。docs/production-prep.md:44）からで、ST54 由来ではない。戻した回の退避を残すのは ST58 の spec（specs/distribution/spec.md:531）と design D23（design.md:192「戻した回・入らなかった回は消さない」）。「本人に返す」は第 1 回の X1 のような「範囲の外として本人に返すもの」の節にも、change・申し送り・確認バッチにも置かれていない（deep.md の第 3 回には「当初案を覆したもの」の箇条の中にしか無い）
- 直すところ: その文書（deep.md に「範囲の外として本人に返すもの（第 3 回）」を立て、呼び名を「戻した回の移行の前の退避（ST58 の spec「戻した回は退避を消さない」）」に直す。範囲の外でなく ST58 の決めたことなので、深掘りの問いとして返すか、spec の段で Q10 にそろえるかを決める）
- 機械: 安く捕まえられる（deep.md の「本人に返す」を含む行が「範囲の外として本人に返すもの」の節の外にあれば落とす検査）
- 行き先: docs（deep.md の族の兄弟の呼び名を「戻した回の移行の前の退避」に直し、本人が IMP-02 で受け入れ済みなので聞き直さないと書いた）

## C4. ★ が「NFR-15 の側」と引くが、NFR-15 は拠点の外へ出ないことの要件で、同じ PC の写しに消した記録が残ることは FR-50・FR-51 の範囲
- 重大度: 低
- 文書: docs/requirements.md:1052
- 要件: FR-119, NFR-15, FR-51
- 根拠: requirements.md:1052「本人が消した記録を、本人に見えず消す口も無い写しに残さない（NFR-15 の側）」。requirements.md:1496「**NFR-15**: すべてのデータは自宅内に存在し、拠点外に出るのは暗号化されたバックアップのみとする」・:1497-1498「「拠点外に出る」は、本システムが記録の写しを拠点外に置くことを指す」。落ちた DB はサーバの PC の中にある（review/code.md:196「PC の外へは出ないが」）。消した記録の写しの扱いを決めているのは FR-51（requirements.md:721-722「ここで消す「本文」は D-01 の記録（と前の版）の原文と解析済みを指し、ashiato が残している書庫の写し…を含めない。写しには消した記録の本文が入ったまま残るので、確認（FR-52）に「書庫の写し N 個にも入っているかもしれない」と出す」）
- 直すところ: requirements（「（NFR-15 の側）」を「（FR-50・FR-51 の側。書庫の写しと違い、本人に見えず数も出せない）」に）
- 機械: できない
- 行き先: requirements（「NFR-15 の側」を「FR-50・FR-51 の側」に直した）

## C5. ST60 への申し送り（Linux の更新と戻し）が FR-119 の ★ 2026-11-16（Q10）を受けていない
- 重大度: 低
- 文書: docs/handoff/ST60.md:21
- 要件: FR-119, FR-131
- 根拠: ST60.md:19「Linux のサーバの更新と戻し（FR-114・FR-119・FR-120）も…扱う」・:21「FR-119・FR-120 に ★ 2026-11-15 が入った…Linux の更新と戻しもこの ★ にそろえる」・:23（Q9 の段）。Q10 の段は無い。stories.json の ST60 の satisfies は `FR-131` だけで、検査 10 も引かない。Linux で戻した回も落ちた DB を残すと、Linux だけ消した記録が写しに残る
- 直すところ: 凍結した Story への申し送り（ST60.md に「2026-11-16（この監査 C5）: FR-119 に ★ 2026-11-16 が入った（Q10）。Linux で戻した回も、前の版のサーバが待ち受けを開いたら落ちた DB を消す」を足す）
- 機械: 安く捕まえられる（C1 と同じ。ST60 の satisfies に FR-119 を足せば検査 10 が引く）
- 行き先: handoff ST60

## C6. 走っている change の中の文書が、Q10 の前の形（落ちた DB は次に更新が通ったときに消す）のまま —— spec の段へ渡すもの
- 重大度: 低
- 文書: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md:285
- 要件: FR-119
- 根拠: spec.md:285「THE SYSTEM SHALL 落ちた DB を、元の DB と入れ替えた後に `ashiato_failed_<時刻>` の名前で残し、次に更新が通ったときに消す」・:297（Scenario「更新の移行が落ちると、退避から DB を戻し、前の版のサーバが起きる」の THEN「`ashiato_failed_` で始まる名前の DB が 1 つある」）・:532「前の回に残した落ちた DB（`ashiato_failed_<時刻>`）を、更新が通ったときに消す」・:549-553（Scenario「更新が通ると、前の回に残した落ちた DB を消す」）。design.md:57（D3「`failed\` は次に更新が通ったときに消す」）・:60「落ちた DB は `ashiato_failed_<時刻>` で残し、次に更新が通ったときに消す（D23（仮））」・:190-192（D23（仮）。provisional.jsonl に D23 の行は無い）。defaults.md:38。tasks.md:76（5.1）・:89（6.1 の Scenario の並び）。go.json の IMP-01（:2286「落ちた DB は次に更新が通るまで消さず」）・IMP-02（:2332-2338）・:840・:1104・:1354・:1433・:2010・:2876、agreement.json:16-18（IMP-02 を受け入れた題「消した記録が、落ちた DB と残した退避に次の更新まで残る」）。tools/release/update-apply-install.sh:222（戻した後に `ashiato_failed_|failed_db=1` を求める）。deep.md:122 は直す範囲を spec の 285・297・549-553 まで挙げているが、design の D3・D23・defaults・tasks・go の IMP-01／IMP-02・通しの台本は挙げていない
- 直すところ: その文書（spec --answers Q10 の段。spec は上の 4 か所、design は D3 と D23（「戻した回は消さない」から落ちた DB を外し、（仮）の扱いを台帳に合わせる）、tasks 5.1・6.1、go の IMP-01（戻すたびに増える分から落ちた DB が外れる）と IMP-02（落ちた DB の側は閉じ、退避の側は残る —— C2・C3）、通しの台本 :222 の向きを反す）
- 機械: 捕まえられる（spec の Scenario を直せば、`tools/release/update-apply-install.sh` の判定と `11.1` の印の突き合わせが食い違いを出す）。go と agreement の文は、できない
- 行き先: leave: 走っている change の文書を書くのは次の spec --answers Q10 の段。deep.md の「spec の段へ渡すもの（第 3 回の整合の確認）」に C6 として名指しで置いた

## C7. deep.md が spec の段に「ST58 より前の版が残したもの」の掃除を考えさせるが、`ashiato_failed_*` を作る版は ST58 より前に無い
- 重大度: 低
- 文書: openspec/changes/st58-update-apply-rollback/deep.md:121
- 要件: FR-119
- 根拠: deep.md:121「「更新が通ると、前の回に残した落ちた DB を消す」（前の回の写しは残らなくなる。ST58 より前の版が残したもの・消し損ねたものの掃除として残すかを含めて）を直す」。`git grep -n ashiato_failed main` は 0 件で、`ashiato_failed_` の名前を作るのはこの change の `restore_database`（`crates/installer/src/main.rs:1674-1678`）だけ。配る版も出ていない（tasks 12.1・12.2 が `[ ]`）。掃除の対象になりうるのは「ST58 以後の版が戻した後、消す前に止まった・前の版が待ち受けを開かなかった回」だけ
- 直すところ: その文書（deep.md:121 の「ST58 より前の版が残したもの」を消し、「ST58 以後の版が、消す前に止まった回・前の版が待ち受けを開かなかった回に残したもの」にする）
- 機械: できない
- 行き先: docs（deep.md の「ST58 より前の版が残したもの」を「ST58 以後の版が、消す前に止まった回・前の版が待ち受けを開かなかった回に残したもの」に直した）

## 2. 文書とコード

## K1. ★ は「失敗した版と理由は…更新の履歴（FR-121）に残る」と言うが、インストーラは移行が落ちた回も `--kind startup-failed` でしか `record-rollback` を呼ばず、落ちた版と SQLSTATE を持つ `failed` の行は戻した DB に書かれない。その行が残るのは落ちた DB だけで、Q10 はそれを直ちに消す
- 重大度: 高
- 文書: docs/requirements.md:1052
- 要件: FR-119, FR-120, FR-121
- 根拠: requirements.md:1052「失敗した版と理由は FR-120 の知らせと更新の履歴（FR-121）に残る」。spec（changes/…/specs/distribution/spec.md:473）「### Requirement: 戻したことを、更新の履歴に「落ちた試みの結果」と「戻した」の 2 行で残す」・:476「移行の結果は `failed`・落ちた版の名前と SQLSTATE」。コード: `crates/installer/src/main.rs:1467-1475`（`rollback`）と `:1532-1538`（`finish_restored`）はどちらも `"startup-failed"` を渡す。`crates/installer/src/commands.rs:1121-1143` の `update_history_record_rollback_command` は `--failed-version`・`--sqlstate` を組めない。サーバ側（`crates/server/src/lib.rs:480-496`）は `migration-failed` のときだけ `failed` の行を足す（`crates/server/src/update_history.rs:212-240`）。`migrate` が書いた `failed` の行（`crates/server/src/migration_ledger.rs:594-599`）は退避より後に書くので、入れ替えで `ashiato_failed_<時刻>` 側に移る（`main.rs:1678`）。さらに `startup-failed` の `rolled_back` の行は、`MIGRATIONS` にあって戻した DB の版の記録に無い版を「当てた版」として持つ（`update_history.rs:182-195`）ので、移行が落ちた回は当てていない版の名前が書き出しに出る（exported）。spec の印 `// Scenario: 戻すと、更新の履歴に落ちた試みの結果と戻したの 2 行が足される`（`crates/server/src/st58_tests.rs:39`）はサーバの関数に `MigrationFailed` を直に渡して確かめており、インストーラの経路を通らない。review/code.md:302 の R24（「戻しの履歴の種別が常に startup-failed…画面の理由は outcomes 側で別に持つので影響は小さい」。Minor）と同じ根で、Q10 の前は落ちた DB に `failed` の行が次に更新が通るまで残っていた
- 直すところ: コード（インストーラが `update_failure_reason`（`main.rs:1143`）で読んだ落ちた版と SQLSTATE を、移行が落ちた回は `--kind migration-failed --failed-version … --sqlstate …` で渡す。落ちた DB を消すのは `record-rollback` が通るか控えに書けた後にする）
- 機械: 安く捕まえられる（`crates/installer` の命令を組む純粋な関数の試験で、移行が落ちた回の `record-rollback` の引数に `migration-failed` と落ちた版があることを見る。今の `crates/installer/src/lib.rs:216` の試験 `st58_plan_rollback_history_command_is_traced_as_failed_program` は `startup-failed` を固定している）
- 行き先: leave: ST58 の下流のコード（review/code.md R24 と同じ根）。★ から更新の履歴に残るという主張を外し、deep.md の「spec の段へ渡すもの（第 3 回）」に K1 として置いた —— spec --answers Q10 の段が tasks に写す

## K2. コードは古い D23（仮）のまま —— 戻した後に落ちた DB を消さず、前の版の待ち受けも確かめない
- 重大度: 中
- 文書: docs/requirements.md:1052
- 要件: FR-119
- 根拠: requirements.md:1052「前の版の S-01 が待ち受けを開いたら直ちに消す」。`cleanup_failed_databases`（`crates/installer/src/main.rs:1778-1822`）を呼ぶのは、更新が通った `update_install_attempt`（:1345）と `finish_update`（:1735）だけ。戻しの `rollback`（:1366-1485）と `finish_restored`（:1522-1562）は `start_previous_services`（:1507-1520。`sc config`・`sc start`・更新係だけで、`wait_for_port` を呼ばない）で終わる。Windows の通し `tools/release/update-apply-install.sh:222` は戻した後に `ashiato_failed_` があることを求める。tasks.md:76（5.1）は `[x]`
- 直すところ: コード（spec の段が C6 を直した後に、戻した後に前の版の待ち受けを確かめてから `ashiato_failed_*` を消す。`rollback` と `finish_restored` の両方と、走らせ直し（`install` の :1858-1871）から入る戻し）
- 機械: 安く捕まえられる（spec の Scenario の THEN を「`ashiato_failed_` で始まる DB は無い」に直せば、Windows の通しの判定が落とす。ただし台本の判定は R2・R23 のとおりまだ実状態を見ていない）
- 行き先: leave: ST58 の下流のコード。deep.md の「spec の段へ渡すもの（第 3 回）」に K2 として置いた

## K3. 戻した後も `<プログラムの置き場>\failed\` が残り、続けてもう 1 回更新が落ちると戻しが途中で落ち、以後 `rollback` の口でも戻せない。spec の「戻しをやり直す口で、前の版で起きる」の守りが無い
- 重大度: 高
- 文書: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md:378
- 要件: FR-119
- 根拠: spec.md:378「WHEN 本人が `rollback` の口を使う THE SYSTEM SHALL 管理者の確認を 1 回出し、印に従って前の版へ戻し終え、前の版のサーバで起こし直す」・:308（Scenario「戻した後の「アプリと機能」の版とプログラムの置き場は前の版」の THEN「プログラムの置き場の中身の指紋は、更新の前に控えた指紋と同じである」）。design.md:57（D3「`failed\` は次に更新が通ったときに消す」）。コード: `rollback` の主な道（`main.rs:1429-1484`）は新しい版を `failed\` へ退け（:1436）、`previous\` の中身を置き場へ戻す（:1443）が、`failed\` も空の `previous\` も消さない（消すのは `finish_restored` の :1558-1560 と通った 2 か所の :1346-1349・:1750-1753 だけ）。次の更新（`update_install_attempt`）は置き場の `previous` 以外を全部 `previous\` へ移す（:1255-1263）ので、`failed\` は `previous\failed` になる。その更新も落ちると、`rollback` は新しい `failed\` を作って新しい版を入れ（:1433-1438）、`previous\` の中身を置き場へ戻すときに `previous\failed` → `<置き場>\failed`（空でない）への `fs::rename` で落ちる（:1443。Rust の `std::fs::rename` は Windows で行き先が空でないディレクトリだと失敗する）。印の段は書き換わっていないので、次の `rollback` は :1430-1431「前回の failed\ が残っているため戻せない」で止まり続け、サーバは開始の種類「無効」のまま。Windows の本物では走らせていない（コードの読みと Rust の文書）。Q10 と同じ「戻した回の失敗した版をいつ消すか」の族で、Q10 は DB だけを決めた
- 直すところ: コード（戻して前の版が待ち受けを開いたら、`finish_restored` と同じく `failed\` と空の `previous\` を消す。spec の段で D3 の「`failed\` は次に更新が通ったときに消す」を Q10 にそろえる）
- 機械: 安く捕まえられる（Windows の通しに「移行が落ちる更新を 2 回続け、2 回目も前の版で起きる」の Scenario を足す。インストーラの単体なら、置き場の移し替えを一時ディレクトリで 2 回続けて回す試験）
- 行き先: leave: ST58 の下流のコード。deep.md の「spec の段へ渡すもの（第 3 回）」に K3 として置いた（design D3 と併せて直す）

## K4. docs/install.md の「サーバの更新」が、退避の置き場・戻すもの・残る退避の数をコードと spec と違えて書いている
- 重大度: 中
- 文書: docs/install.md:37
- 要件: FR-119
- 根拠: install.md:37-39「インストーラがその回の開始前に作った `C:\ProgramData\ashiato\update\backups\pre-migrate\<run_id>\` から DB と設定を戻し」。コード: 退避の置き場は `PathBuf::from(layout.state_dir).join("backups").join("pre-migrate").join(&run_id)`（`crates/installer/src/main.rs:1196-1199`。`state_dir` は `C:\ProgramData\ashiato`（`crates/installer/src/layout.rs:22`））で `update\` の下ではない（正典 openspec/specs/distribution/spec.md:1702 も `C:\ProgramData\ashiato\backups\pre-migrate`）。設定は戻さない（change の spec.md:284「設定と更新係の置き場は戻さない」。`rollback` が動かすのはプログラムの置き場と DB だけ —— main.rs:1429-1453）。install.md:50「退避は直前の 1 回分だけ残ります」は、spec.md:531「戻した回・入らなかった回には、退避を消さない」・Scenario「戻した回は退避を消さない」と合わない（戻した後は前の退避とその回の退避が並ぶ）
- 直すところ: その文書（install.md の :38 の置き場を `C:\ProgramData\ashiato\backups\pre-migrate\<run_id>\` に、「DB と設定を戻し」を「DB とプログラムを戻し（設定は戻さない）」に、:50 を「更新が通ると直前の 1 回分だけになる。戻した回の退避は次に更新が通るまで残る」に。Q10 を spec に写すときに、戻した回は落ちた DB を残さないことも書く）
- 機械: 安く捕まえられる（`docs/install.md` に書いたパスを `layout.rs` と `main.rs` の組み立てと突き合わせる検査。今は無い）
- 行き先: leave: ST58 の tasks が持つ文書。deep.md の「spec の段へ渡すもの（第 3 回）」に K4 として置いた

## 未確認

- 「待ち受けを開く前は、新しい版は記録を 1 件も受け取っていない」（change の spec.md:358）は、インストーラが 1 秒ごとに繋いで確かめる（`main.rs:266-276`）ことで判定している。新しい版が待ち受けを開いて記録を受け取り、次の確かめより前に落ちた回は「開かなかった」として戻され、その記録は落ちた DB にだけ残る。Q10 の前は次の更新までその DB に残っていたが、Q10 で直ちに消える。サーバが待ち受けを開いてから記録を受け取るまでの最短の時間と、サービスの立て直しの間隔は確かめていない
- K3 は Windows の本物で走らせていない（コードの読みと Rust の `std::fs::rename` の文書だけ）
- `pg_restore` で入れ直した DB が、元の DB の DB の水準の設定（持ち主・`GRANT CONNECT`・`ALTER DATABASE … SET`）を持たない場合に、前の版のサーバが待ち受けを開いても後で困るか。Q10 で落ちた DB を直ちに消すと、それを比べる元が無くなる（記録そのものはこの回の退避に残る）
- 正典の Scenario のテストの印は全件を機械で見ていない（改訂は正典を変えていない）

## 触らなくてよいと判断したもの

- `openspec/changes/st58-update-apply-rollback/agreement.json:5` の `"answers": "deep-answers-4.txt"`（合意は 2026-11-15、`deep-answers-4.txt` は今回の Q10 の答え）: `imp_ledger.verify`（`.harness2/scripts/imp_ledger.py:346-363`）がこのファイルを読むのは重大な IMP があるときだけで、agreement.json の IMP は medium と low だけ（`minor_accepted`）。ファイル名の重なりはハーネスの番号の付け方で、ashiato2 の文書の食い違いではない
- `docs/handoff/ST51.md:15`・`docs/handoff/ST58.md:14,23,52`・`docs/stories/INDEX.md:82`・`proposal.md:6,24,34`・`premises.md:22`: FR-119 の戻しの条件・終了コード・退避の特定を言っており、落ちた DB をいつ消すかに触れない
- 書き出し: 落ちた DB は書き出しに出ない —— `crates/server` は接続の URL の DB（`ashiato`）だけを読み、`export.rs` も同じ。Q10 で書き出しの形は変わらない
- `review/prose.src.md`・`review/prose.final.md`・`review/code.md` R11: レビューの席の記録で、R11 の処置 `escalated` は Q10 で答えが出た。書き直さない
- `docs/install.md` に落ちた DB の説明が無いこと（review/code.md:196「`grep -rn 'ashiato_failed' docs web/src` は 0 件」）: Q10 の後は落ちた DB が残らないので、残っていることの説明は要らない（戻した回の退避の数は K4）
- requirements.md:1051 の ★ 2026-11-15（退避はその回の 1 つだけを残す）: Q10 は退避の保持を変えていない（理由の書き方の食い違いは C2）
