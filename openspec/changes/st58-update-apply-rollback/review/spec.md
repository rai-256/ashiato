# ST58 の spec の検査（spec --draft・第 1 回の先読み）

## 機械の検査（2026-11-15、docs/st58-upstream の 5255ac795 の上）

- `openspec validate st58-update-apply-rollback --strict` → `Change 'st58-update-apply-rollback' is valid`
- `python3 scripts/check_chain.py .` → `chain: FAIL (60 件 / 未回収 0 件 / warn 0 件)`。FAIL の行は ST01〜ST30 の「再生成した結果と一致しない」だけで、ST58 は無い（観点 8 は ST58 について通っている）。
  このブランチの差分（`git diff --stat origin/main..HEAD`）は `docs/stories` と `docs/requirements.md` に触れていないので、この change が起こした落ちではない
- `python3 scripts/check_scenarios.py . st58-update-apply-rollback` → `scenarios: FAIL (担保なし 64 件 / 名無しの確認待ち 0 件)`。64 件は ADDED の 56 本と MODIFIED で足した 8 本で、tasks.md の Global Constraints の数と合う。下流の前なので担保が無いのは予定どおり。
  spec の 100 本の Scenario のうち tasks.md が名指ししない 31 本は、どれも MODIFIED で文を変えずに持ち越した既存の Scenario だった（突き合わせの python で確かめた）
- `python3 scripts/review_triage.py . st58-update-apply-rollback` → `triage: FAIL (18 件)`。review/deep.md の R1・R3・R4・R5・R6・R10・R11・R12・R15 が「loss つきなのに fixed」で落ちる（R21 を見る）。`要件へ戻すもの 0 件`（R28 を見る）
- `python3 scripts/spec_overlap.py candidates . st58-update-apply-rollback` → `[]`
- 参考: `python3 scripts/provisional.py pending openspec/changes/st58-update-apply-rollback` → `["D10"]`（R20 を見る）

## R1. 待ち受けを開いた後に止まった更新を走らせ直すと、D20（仮）は DB を戻して、新しい版が受け取った記録を捨てる
- 成果物: openspec/changes/st58-update-apply-rollback/design.md・specs/distribution/spec.md
- 根拠: design.md:38 では、待ち受けを待つ段が `starting` で、待ち受けを確かめた後の段が無い。design.md:39 の 8 は「開始の種類を自動」→「アプリと機能」→ 更新係 → 保持 → `applied` → 控えの写し →「印を消す」の順で、印を消すのは最後。
  design.md:154 と spec.md:306 は「印が残っていれば段に依らずまず前の版へ戻し終える（この回の退避があれば DB も戻す）」と書き、待ち受けを開いた後の例外を持たない。
  そのため、8 の途中で電源が落ちると、次の起動で開始の種類が「自動」のサーバ（新しい版）が起き、記録を受け取る（収集アプリは未送信から消す）。その後に本人が走らせ直すと、退避から DB が戻り、受け取った記録が消える。
  これは Q5 の推奨（deep-questions.json:133「待ち受けを開いた後に落ちたときは戻さず（受け取った記録を捨てないため）」）と spec.md:286 に反する
- kind: irreversible
- loss: discarded
- 提案: 待ち受けを確かめた直後に、段 `listening` を印に書く（開始の種類を「自動」へ戻すより前）。印の段が `listening` のときは、走らせ直しも `rollback` の口も DB を戻さず、8 の残りを前へ進める。Scenario を足す:「待ち受けを開いた後に止まった更新は、走らせ直しても DB を戻さない」（差し込み `ASHIATO_SETUP_TEST_STOP_AT=after-listen`）
- 処置: escalated —— Q5（推奨の detail に「新しい版が待ち受けを開いた後に止まった回は戻さず、残りを進めます」を足した）。design D1 の 8 に段 `listening`・D20（仮）・spec「更新や戻しが途中で止まっても…」に例外と Scenario「待ち受けを開いた後に止まった更新は、走らせ直しても DB を戻さない」を足した

## R2. Q5 の推奨の文は「続きからやり直す」だが、spec と D20（仮）は「前の版へ戻し終えてから初めからやり直す」
- 成果物: openspec/changes/st58-update-apply-rollback/deep-questions.json・deep.md・proposal.md・design.md
- 根拠: 本人が選ぶ文は deep-questions.json:133「どちらの版のインストーラを走らせ直しても、印を見て続きからやり直します」と deep.md の Q5「印を見て続きからやり直す」。proposal.md:22 も「続きからやり直す」と書く。
  一方 spec.md:306 と design.md:152-155 は「まず前の版へ戻し終え、…更新の手順を初めからやり直す」で、design.md:155 は「続きから前へ進める形は採らない」と明記している。
  本人は、書いてある仕様と違う手順に答えることになる。D20 は（仮）なので、推奨のまま答えても合意は D20 に及ばない
- kind: conflict
- 提案: 本人が答える前に、Q5 の推奨の detail と deep.md の推奨の文を「印を見て、まず前の版へ戻し終えてから、走らせた版で初めからやり直す」に直す。proposal.md:22 も合わせる。D20 の「戻し終える」部分は Q5 の答えに含まれるので、（仮）を外して Q5 に依る形（`follows: Q5`）にする
- 処置: escalated —— Q5 の推奨の label・detail と deep.md の推奨と proposal.md を「まず前の版へ戻し終えてから、走らせた版で初めからやり直す」に直した（本人が答える前）

## R3. 待ち受けを開かずに戻した回（`server_not_listening`）に、更新の履歴へ書く 2 行が決まっていない
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md
- 根拠: spec.md:285 と design.md:47 では、移行が通った後に新しい版が待ち受けを開かなければ、退避から戻す。
  ところが、戻した後に書く「落ちた試みの結果」の行は、spec.md:379 が `outcome` を「`failed` か `backup_failed`・落ちた版の名前と SQLSTATE」に限っている。この回は移行が通っているので、落ちた版も SQLSTATE も無く、どちらの値も当たらない。
  `backup_failed` の回は D2 では DB を戻さないので、この 2 行の場面には現れない。
  戻した DB の中では `applied` の結果の行が消え、試みの行だけが残る。書き出しに出る行の形は一度出ると直せない（Q6 の rewrite-all）。spec.md:391 の Scenario も移行が落ちた回だけを見る
- kind: irreversible
- loss: rewrite-all
- 提案: 待ち受けを開かずに戻した回の 2 行を決め、spec に書く。たとえば「落ちた試みの結果」は `applied`（当てた版を持つ）にし、「戻した」は `rolled_back` にする。または「落ちた試みの結果」を書かず、`rolled_back` の 1 行だけにする。そのうえで Scenario「待ち受けを開かずに戻すと、更新の履歴に <決めた行> が足される」を足す。`backup_failed` を 2 行の候補から外すかどうかも決める
- 処置: escalated —— deep.md Q9（第 2 回）。D26 を仮で進めたが、別の席が「失わない」を否とした（provisional-review.md D26）ので台帳から外して本人に返した

## R4. 終了コードが 0〜4 以外（落ちた・止められた）で、この回の退避がある回に DB を戻さないと、どちらの版も起きない
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md・design.md
- 根拠: spec.md:259 と design.md:48 は「0・1・2・3・4 以外」でも DB を戻さず、前の版で起こし直す。
  しかし移行は版ごとに当たって残る（P13・spec.md:233「途中の版で落ちると、それより前の版は当たったまま…前の版のサーバは『DB が新しい』で起きない」）。退避の後に panic や kill で終わると、DB は前の版より進んでいて、前の版のサーバは起きない。
  同じ場面でインストーラの方が死んだときは、D20（design.md:154）が「段が `migrating` 以後で、この回の退避があれば DB も戻す」ので、どちらが死んだかで扱いが逆になる。
  終了コード 2 と「それ以外」の Scenario も無い（spec.md:265-281 は 3・4・1 の退避の前だけ）
- kind: technical
- 提案: DB を戻すかどうかの判定を「終了コード 1」でなく「この回の退避がある・かつ終了コードが 0 でない、かつ 2・3・4 でない」に直す。3・4 が退避より前に止まることは申し送り（st48 R21・st54 D3）が確かめている。2 は `crates/server/src/lib.rs:402-403` の接続先の検査が、接続より前に止める。
  Scenario「移行が途中で止められると（終了コードが 0〜4 以外）、この回の退避から戻して前の版が起きる」と「接続先が手元でないと（終了コード 2）、戻さずに前の版のまま起きる」を足す
- 処置: fixed D2 —— 終了コード 0・2・3・4 以外で、この回の取り終えた退避があれば戻す。spec の 2 つの Requirement を直した

## R5. 「この回の置き場に退避のファイルがある」では、書きかけの退避と取り終えた退避を区別できない
- 成果物: openspec/changes/st58-update-apply-rollback/design.md（D2）・specs/distribution/spec.md
- 根拠: `crates/server/src/migrate_backup.rs:113-116` は、`pg_dump` を起こす前に空の `pre-migrate-<時刻>.dump` を作る。ファイルを消すのは、`pg_dump` が失敗を返したときだけ（:139）。
  `pg_dump` の途中で migrate かインストーラが kill される（電源・窓を閉じる）と、書きかけのファイルが残る。D2（design.md:47）は「`pre-migrate-*.dump` が 1 つあれば『この回の退避がある』」とみなす。
  D20 の走らせ直しはこれを戻しに使い、`pg_restore --exit-on-error` が落ちる。すると「戻しそのものが落ちた」になり、サーバは「無効」のまま止まり、`rollback` の口でも同じに落ち続ける。
  この回は移行を当てていない（退避の後に当てる）ので、DB を戻す必要はそもそも無い
- kind: technical
- 提案: 退避を取り終えた印を、取り終えた後にだけ置く。たとえば migrate が `pg_dump` の成功の後に `.dump.part` から `.dump` へ名前を変える、または完了の印のファイルを書く。D2 は「取り終えた退避」だけを数える。
  Scenario「退避の途中で止まった更新は、走らせ直すと DB を戻さずに前の版へ戻す」を足す
- 処置: fixed D2 —— `.dump.part` → `.dump`。Task 2.1・Scenario「書きかけの退避は戻さない」

## R6. `record-rollback` を「新しい版の実行ファイル」で走らせるが、戻しの時にはその実行ファイルが置き場に無い
- 成果物: openspec/changes/st58-update-apply-rollback/design.md・tasks.md
- 根拠: design.md:52 の D3 は「前の版のプログラムを置き場に戻し、DB のサービスを起こしてから」`pg_restore` する。tasks.md:72 は、戻した後に `ashiato-server update-history record-rollback`（新しい版の実行ファイル）を走らせる。
  しかし前の版を置き場へ戻した時点で、新しい版の実行ファイルは置き場から外れている。前の版が ST58 より前なら、`update-history` の口そのものを持たない。
  D1（design.md:43）は、zip の中から実行ファイルを走らせる形を、置き場の指定が食い違うとして退けている。どこから新しい版を起こすかが、どの成果物にも無い
- kind: technical
- 提案: D3 に順を書く。たとえば「新しい版の置き場を `<置き場>\failed\` へ退けてから前の版を戻し、`record-rollback` は `failed\` の新しい版で走らせる」。または「前の版を戻す前に、新しい版で `record-rollback` を走らせる」（ただし DB はその時点でまだ戻っていない）。どちらかを選び、tasks 5.1 を合わせる
- 処置: fixed D3 —— 新しい版を `failed\` へ退け、そこの `record-rollback` を走らせる。Task 5.1

## R7. 控えを写すのは誰か・いつかが、spec（migrate が写す）と design・tasks（インストーラが `replay` を走らせる）で食い違う
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md・design.md・tasks.md
- 根拠: spec.md:382 は「次に `rolled_back` を知る版の移行が通ったときに…表へ足し」と書き、Scenario の WHEN も spec.md:396「ST58 の移行を持つ組み立ての `migrate` が通る」。
  一方 design.md:144 は「写すのは `update-history replay`（インストーラが、更新が通った後に表の所有者で走らせる）」、design.md:39 は 8 の段（待ち受けを確かめた後）で写す。tasks.md:42 は、この Scenario の印を `replay` の試験に置く。
  そのため印の試験は Scenario の WHEN（migrate）を通らず、migrate が控えを写さない実装でも緑になる。
  控えの置き場はサービスのアカウントが書けない（spec.md:183）。migrate が写すなら、誰の権限で控えに印を書くかも決まっていない
- kind: technical
- 提案: どちらかに揃える。design のとおりにするなら、spec.md:382 を「更新の手順が、新しい版の待ち受けを確かめた後に、控えの行を…表へ足し」に直す。Scenario の WHEN も「控えを持つインストールで、`rolled_back` を知る版への更新が通る」に直し、install-windows（tasks 6.1）か `replay` を名指しする形にする
- 処置: fixed specs/distribution/spec.md —— 控えを写すのは更新の手順。Requirement と Scenario の WHEN を直した

## R8. 写した控えの行は `seq` で後ろに並ぶので、書き出しを読む側は「戻した」を、その後に通った更新より後に読む
- 成果物: openspec/changes/st58-update-apply-rollback/specs/data-durability/spec.md・specs/distribution/spec.md
- 根拠: 正典は「書いた順」を `seq` で決め、時刻では決めない（spec.md:746）。控えの 2 行は「控えた時刻のまま」、次に通った回で足される（spec.md:382）。
  そのため、`seq` の順では「applied 0.4.0→0.6.0」の後に「rolled_back 0.5.0→0.4.0」が並ぶ。版の変化の向きを `seq` の順で辿る読み手は、0.6.0 の後に 0.4.0 へ戻ったと読む。
  data-durability の説明書（data-durability/spec.md:8）は `rolled_back` の向きだけを説明し、写した行が `seq` と時刻で逆順になることを書かない。書き出しの行は直せない（Q6）
- kind: technical
- 提案: 説明書の `rolled_back` の説明に「前の版の表が受け付けなかった回は、次に通った更新の後に控えから写すので、`seq` の順では後ろに並ぶ（`at` は戻した時刻）」を足し、Scenario の AND の語句に入れる。または写した印の列か値（例: `via` は変えず `applied_versions` を空にする等）で、写した行だと分かる形を決める
- 処置: fixed specs/data-durability/spec.md —— 説明書に「控えから写すので seq の順では後ろ・at は戻した時」と AND の語句

## R9. PC の新しい版を端末の窓から起こすと、窓を閉じたときに一緒に止まりうる。その確かめも、入れ替えの途中で窓を閉じた場合の決まりも無い
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md・design.md（D12（仮）・D13（仮））
- 根拠: 窓は `setup` と同じ端末の窓で（design.md:113）、`crates/collector-windows/src/toast.rs:162-165` は `creation_flags(0x0000_0010)`（CREATE_NEW_CONSOLE）で起こす。D12 は新しい版を窓の子として起こし（design.md:106「窓が起こした子なので `Child::kill`」）、窓は Enter で閉じる（design.md:113）。
  子が端末を引き継ぐと、窓を閉じたときに新しい版（と、戻したときに起こし直した前の版）が一緒に止まりうる。そうなると次のログオンまで記録が取れない。
  spec.md:96 の Scenario は「1 分以内に動いている」しか見ず、窓を閉じた後を見ない。spec.md:76「同時に収集しない」には Scenario が無い。y と答えた後、落とす・待つの途中で本人が窓を閉じたときの扱い（前の版を止めた後なら、どちらも動いていない）も、どの D にも無い。
  端末の引き継ぎは Windows の上で確かめていない（前提として扱う）
- kind: premise
- loss: uncaptured
- 提案: premises に「窓が起こした子は、窓を閉じても動き続ける（起こし方の旗）」を足し、tasks 8.2 で確かさ 4 に上げる。Scenario を足す:「PC は窓を閉じた後も、入れ替えた新しい版が動き続ける」と「PC は入れ替えの間、前の版と新しい版が同時に記録を積まない」（受け手が受けた記録に同じ時刻の重なりが無い・プロセスが同時に 2 つでない）。
  D12 に、窓を閉じたときの決まりを書く（例: 前の版を止めた後に窓が閉じられたら、前の版を起こし直してから閉じる）
- 処置: escalated —— Q1 の context に P24 を書いた。spec に「窓を閉じても止まらない形で起こす」と Scenario 2 本、premises P24、design D12（仮）、Task 8.2

## R10. PC の自動の戻しは「新しい版の後に古い版を動かす」場面で、ST52 の申し送り（逆行判定の設計と検証）に答えていない
- 成果物: openspec/changes/st58-update-apply-rollback/defaults.md（C7）・design.md（D7・D12（仮））・specs/distribution/spec.md
- 根拠: defaults.md:55 の C7 と design.md:77 の D7 は「古い版を入れる場面は窓に現れない」として、問いから外した。
  しかし D12（design.md:106）の自動の戻しは、新しい版が起きて（送る仕組みを始めて未送信を積みうる。design.md:105）から前の版を起こし直すので、まさに新しい版の後に古い版が動く場面になる。
  申し送り docs/handoff/ST58.md の「ST52 D5（仮）・R14」は「未送信データを古い形で送らないための逆行判定も合わせて設計・検証する」と求めている。spec では spec.md:84 の導出元の説明文だけが「古い版へ戻しても古い形で送らない」と言い、Scenario は進む向き（spec.md:121）だけを見る。
  新しい版が手元の置き場（未送信の DB など）の形を変えた場合に、前の版がそれを読めるかも決まっていない
- kind: technical
- 提案: Scenario「新しい版が積んだ未送信は、前の版に戻しても消えず、前の版は自分の知らない送信契約の版の行を送らない」を足し、tasks 8.1 の Linux の試験に置く。D12 に「新しい版は、動き出したと書く（`update-alive.json`）前に、手元の置き場の形を変えない」か、変える場合の決まりを書く。C7 と D7 の「場面が現れない」を直す
- 処置: fixed D7 —— C7 を直し、新しい版は動き出す前に置き場の形を変えない。Scenario「PC は前の版に戻しても、新しい版が積んだ未送信を消さない」・Task 8.1

## R11. PC の版ごとの置き場の起点が、2 回目の更新で入れ子になる
- 成果物: openspec/changes/st58-update-apply-rollback/design.md（D12（仮）・D14（仮））
- 根拠: design.md:104・:122 は、落とし先を `<実行ファイルの置き場>\versions\<版>\` にする。1 回目の更新の後、動いている実行ファイルは `versions\0.5.0\` の中にある（自動起動もそこを指す。design.md:105）。
  そのため、2 回目に `current_exe()` の置き場を起点にすると `versions\0.5.0\versions\0.6.0\` になる。「前の版より古い版の置き場を消す」（design.md:105）も、どの `versions\` を見るかが決まらない。
  元の場所の実行ファイルはずっと残る（design.md:107）
- kind: technical
- 提案: 起点を固定する（例: 最初に置いた場所、または `LOCALAPPDATA` の下の決まった置き場）と D12・D14 に書く。tasks 8.1 の試験に「2 回続けて入れ替えても、置き場は 1 段で、残るのは今の版と前の版の 2 つ」を足す
- 処置: fixed D12 —— 版ごとの置き場の起点を `<状態の置き場>\versions\` に固定。Task 8.1

## R12. spec の導出元と premises は、PC の入れ替えが P4（確かさ 1）に依ると言うが、D12（仮）は P4 に依らない。本人に見せた Q1 の文は別の手順を説明している
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md・premises.md・deep-questions.json
- 根拠: spec.md:83「premises P4（確かさ 1。Windows の実行時の試験で 4 に上げる）」、premises.md:7「Q1 の推奨の手順が依る P4…下流の Windows の実行時の試験で 4 に上げる」。
  一方 design.md:107 は「動いている実行ファイルの名前を変えない（P4 に依らない）」で、tasks 8.2 に P4 を上げる段は無い。Q1 の context（deep-questions.json の Q1）は「名前を変えて退ける手順…確かめられなければ、自分を止めてから置き換えます」と説明している
- kind: technical
- 提案: spec.md:83 と premises.md:7 から「P4 に依る・4 に上げる」を外す（D12 の反転条件の先として残す）。Q1 の context を、本人が答える前に D12 の手順（版ごとの置き場に置いて起こす）に直す
- 処置: fixed specs/distribution/spec.md —— 導出元から P4 を外した。premises の冒頭と Q1 の context も D12 の手順に直した

## R13. 申し送り ST57「前の版へ戻したときの更新係」は、戻した後に更新係が動いていることを見る Scenario を持たない
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md・design.md（D4）
- 根拠: docs/handoff/ST58.md の ST57 の節は「前の版へ戻したときに更新係のサービスをどう扱うか…戻しの手順と合わせて検証すること」と求めている。
  spec.md:179 は、更新が通ったときだけ「更新係を（切ってなければ）起こし」と書く。戻しの道（spec.md:226・:259・:285）は更新係に触れない。D4（design.md:63）の「最後に（切ってなければ）起こす」も、戻しの道の最後を含むかが読めない。
  Scenario は、切った場合（spec.md:453）しか見ない。戻した後に更新係が止まったままだと、7 日後に「確かめられていない」の帯が出て、戻した版の「更新あり」も古くなる
- kind: technical
- 提案: 戻しの Requirement に「戻した後、切ってなければ更新係を前の版で起こす」を足す。Scenario「戻した後、切っていない更新係のサービスは実行中で、開始の種類は自動」を足し、tasks 6.1 に置く
- 処置: fixed specs/distribution/spec.md —— 戻した後に更新係を起こす文と Scenario「戻した後、切っていない更新係は動いている」。Task 5.1・6.1

## R14. 戻しの役割（表の所有者）を spec が固定しているのに、design は D3 を仮でなく、反転条件だけを Risks に置いている
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md・design.md
- 根拠: spec.md:229「戻しを、DB の管理者でない表の所有者の役割で行う」と、Scenario spec.md:252-255 は、役割を `ashiato_owner` に固定している。
  その根拠の P20 は確かさ 3 で（premises.md:P20「行える（文書と台本の上）」）、design.md:185 は「足りなければ…管理者として入る形に替える（D3 の反転条件）」と書く。しかし D3（design.md:50）は（仮）の印を持たず、台帳にも無い。
  反転したら spec の Requirement と Scenario が書き直しになるのに、確認バッチの一覧にも出ない。
  なお、所有者の権限で足りるかの読みは、repo の範囲では崩れていない。`tools/db-roles.sql:7`・`:11` が CREATEDB と DB の持ち主を与え、`migrations/` には `core` 以外の schema と拡張が無い
- kind: technical
- 提案: D3 の役割の部分を `D<n>（仮）` に分けて台帳に載せ、反転条件（Windows の CI で `pg_restore` が所有者で通らない）を書く。spec.md:229 は残してよい（反転したら spec を直すことを反転条件に書く）
- 処置: fixed D25 仮 —— D3 から役割を分けて台帳へ

## R15. 同梱の PostgreSQL の小さい版が違う更新で、戻しが前の版の `pg_restore` で新しい版の `pg_dump` の退避を読めるかの前提が無く、CI でも見ない
- 成果物: openspec/changes/st58-update-apply-rollback/premises.md・design.md（D5・D24（仮））・specs/distribution/spec.md
- 根拠: spec.md:434 は「小さい版が違う PostgreSQL は、プログラムの置き場と一緒に入れ替え、戻すときは一緒に前の版に戻す」と書き、Scenario を持たない。
  退避は新しい版の同梱の `pg_dump` で取る。一方、戻しは前の版の同梱の `pg_restore`（design.md:54）で入れ、DB の置き場も小さい版を下げて動かす。premises にこの組み合わせの前提が無い。
  D24（design.md:176）の 2 つの zip は、同じ commit から版番号だけを替えて組むので、`tools/release/pgsql.lock` が同じになる。そのため CI はこの組み合わせを通らない
- kind: premise
- 提案: premises に「同じ大きい版の中では、新しい小さい版の `pg_dump --format=custom` を古い小さい版の `pg_restore` が読め、データの置き場は小さい版を下げても起きる」を足し、出所（PostgreSQL の公式文書）を示す。
  確かめられなければ D3 を「戻しは新しい版の `pg_restore` で入れてから、前の版の PostgreSQL に替える」に直す。Scenario を置くなら、D24 の 2 つ目の組み立ての `pgsql.lock` の小さい版を替える差し込みが要る
- 処置: escalated —— Q5 の context に P25 を書いた。premises P25・design D5 と Risks・Task 10.1（docs/release.md）

## R16. 観測できる形（更新の記録と `last_update` の欄の名前と値・理由の短い説明・窓の文）が design にだけあり、archive で正典から落ちる
- 成果物: openspec/changes/st58-update-apply-rollback/design.md（D13（仮）・D14（仮）・D15（仮））・specs/distribution/spec.md
- 根拠: `GET /update-status` の `last_update` は、design.md:120-121 が欄の名前（`result`・`at`・`to_version`・`from_version`・`reason.kind`・`reason.failed_version`・`reason.sqlstate`）と `result` の値（`applied`・`rolled_back`・`not_applied`）を決める。spec.md:577 は「時刻・結果・新しい版・前の版・理由」と書くだけで、欄の名前と値の一覧を持たない。値は Scenario の文の中（spec.md:363・369・374）にだけ現れる。
  画面と窓に出る短い説明（design.md:128 の D15）も spec に無く、spec.md:352・363 の「その短い説明」は何と照らせばよいかが決まらない。
  窓の結果の文（design.md:113「0.5.0 に入れ替えました」）も spec に無い。どれも応答と画面の形で、`openspec archive` が写さない
- kind: technical
- 提案: spec.md:577 の `last_update` に欄の名前と `result` の 3 つの値を書き、Scenario「更新の状態の口は応答を残させない」の THEN に `last_update` の欄の名前を足す。D15 の種別と SQLSTATE の級の文の表を、spec の「更新が入らなかったことを…」の Requirement に移す（文は（仮）のまま、反転条件は design に残す）
- 処置: fixed specs/distribution/spec.md —— `last_update` の欄と値・更新の記録の欄・理由の短い説明の表を spec に置いた

## R17. Requirement の本文だけが言い、Scenario が言っていないことがある
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md
- 根拠: 次の主張は Scenario が無いので、実装が外しても spec が落ちない:
  - spec.md:76「入れ替えの間に、前の版と新しい版が同時に収集しない」（R9）
  - spec.md:137 の PC の側の「指紋が合わない配布物は入れない」（携帯は spec.md:27 にある。PC は指紋が無い場合（spec.md:157）だけ）
  - spec.md:9「インストール画面を閉じた後か、次にアプリを開いたときに APK を消す」のうち、入れ終えた後と、次に開いたとき（取り消し（spec.md:39）だけがある）
  - spec.md:140「宛先の差し替えを試験の組み立てだけで読む」
  - spec.md:184 の逆向き（正式版のインストーラは開発版のインストールを更新しない。spec.md:218 は開発版から正式版だけ）
  - spec.md:285 の「180 秒」と spec.md:581 の「1 MB」（どちらの数値も、値を変えても Scenario が落ちない。1 MB は tasks 2.1 の試験にはあるが、印の Scenario には無い）
  - spec.md:570「『戻した』『入らなかった』の行を押しても何もしない」（R19）
  - 走らせ直しと `rollback` の口で終わった回（spec.md:306-309。Scenario は spec.md:320-347）は、`outcomes.jsonl` の行・帯・更新の履歴の 2 行のどれにも触れない。印 `state.json`（design.md:32・:37）は終了コードしか持たず、落ちた版と SQLSTATE を持たない。そのため、途中で止まってから戻した回は FR-120 の「失敗した版・理由」を知らせられない
  - Q5 の推奨の「待ち受けを開いた後に落ちたときは…ログの場所を示します」（deep-questions.json:133）は、spec に文も Scenario も無い（spec.md:297-301 は rc=0 と DB だけを見る）
- kind: technical
- 提案: 少なくとも次の Scenario を足す。「PC は指紋の合わない配布物で入れ替えない」「携帯は入れ終えた後に APK を置き場に残さない」「正式版のインストーラは開発版のインストールを更新しない」「1 MB を超える更新の記録は画面に出さない」。
  ログの場所は、spec.md:286 に文を足すか、Q5 の detail から外すかを決める
- 処置: fixed specs/distribution/spec.md —— Scenario 4 本を足した。Task 2・6・8・9

## R18. 1 つの Scenario が多くの主張を束ねていて、どれか 1 つを外しても印の試験の名前が変わらない
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md
- 根拠: spec.md:194 は 7 つ（rc・`/healthz`・版・件数・退け先・開始の種類・印）、spec.md:336 は 5 つ、spec.md:363 は 5 つ（文・版・落ちた版・SQLSTATE と説明・本文が無い・記録の最後の行）、spec.md:443 は 5 つを束ねる。
  install-windows の確かめは `echo "== Scenario: …"` の段ごとに 1 つの印なので、段の中の確かめを 1 つ削っても印は残る。
  また spec.md:363 の「DB の誤りの本文（行の値）は無く」は、どの値を探せばよいかが GIVEN に無いので、真偽を決められない（23505 の本文は重なった鍵の値を含む）
- kind: technical
- 提案: 少なくとも spec.md:363 の GIVEN に「重なる値 `st58-secret-value` を持つ行を足して 23505 で落ちる移行」を置き、THEN を「出力・更新の記録・ログのどれにも `st58-secret-value` が無い」にする。束ねた THEN は、記録の件数（失わない）と開始の種類（起きない）の 2 本に分けるのを優先する
- 処置: fixed specs/distribution/spec.md —— `st58-secret-value` の GIVEN と AND に分けた

## R19. 書き替えた「帯を押しても何も起きない」は、中身は正しいが名前が広く、押しても何もしない 3 つの行のうち 1 つしか見ない
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md
- 根拠: spec.md:631-635 は GIVEN を「確かめられていない」の帯だけにした（判定: 正典の Scenario を残したまま意味を新しい本文に合わせる直し方として、中身は spec.md:570 と矛盾しない）。
  ただし名前は「帯を押しても」のままで、すぐ後の spec.md:637「帯を押すと、サーバの更新の手順が開く」と名前の上では逆を言う。spec.md:570 が「押しても何もしない」とする「戻した」「入らなかった」の行には Scenario が無い
- kind: technical
- 提案: 名前は正典の印（`web/e2e/st57-update-notice.spec.ts:55`）を保つためにこのままでよい。Scenario「戻した・入らなかったの行を押しても何も起きない」を足し、tasks 3.1 の e2e に置く
- 処置: fixed specs/distribution/spec.md —— Scenario「戻した・入らなかったの行を押しても何も起きない」。Task 3.1

## R20. （仮）の決定 D11〜D24 の 14 件が、仮の台帳に載っていない
- 成果物: openspec/changes/st58-update-apply-rollback/provisional.jsonl
- 根拠: `provisional.jsonl` は 1 行（D10）だけで、`python3 scripts/provisional.py pending …` → `["D10"]`。一方 design.md には D11〜D24 の（仮）が 14 件ある（design.md:94-177）。
  前例の `openspec/changes/archive/2026-11-14-st57-update-notice/provisional.jsonl` は 13 行だった。台帳に無い仮は、別の席（deep-review）の確かめにも確認バッチの一覧にも出ない
- kind: technical
- 提案: D11〜D24 を `provisional.py add` で載せる（`--keeps`/`--cost` か `--follows Q<n>`。D20 は Q5、D18 は Q6、D23 は Q7、D12・D13・D17 は Q1、D21・D22 は Q2 に依る）。R14 で分けた D3 の役割の部分も載せる
- 処置: rejected: 台帳は loss のある指摘を失わない側で閉じた仮のためのもので、B（計算し直せば戻る）の仮は design の（仮）と反転条件・PR 本文で足りる（`.claude/skills/story-upstream/SKILL.md:523`）。前例の ST57 も D11〜D22（仮）を台帳に載せていない（`openspec/changes/archive/2026-11-14-st57-update-notice/provisional.jsonl:1`）。loss のある D10・D25・D26 は載せた

## R21. review/deep.md の loss つきの R が `fixed` で閉じていて、triage が 18 件で落ちる
- 成果物: openspec/changes/st58-update-apply-rollback/review/deep.md
- 根拠: `python3 scripts/review_triage.py . st58-update-apply-rollback` → `[FAIL] deep.md R1: loss=discarded は人間に返すもの。fixed で閉じられない`（R1・R3・R4・R5・R6・R10・R11・R12・R15 の各 2 行）。
  処置の中身は、問いへ上げた（R3→Q5、R5→Q6、R6→Q7、R12→Q1、R4→Q4・Q5）か、仮にした（R11→P1→D10）か、本人に返した（R15→X1）もので、`fixed` の形だけが合わない
- kind: technical
- 提案: 問いへ上げたものは `escalated Q<n>`（deep.md の各 Q に `出所: review/deep.md R<n>` を足す。ST57 の review/spec.md R1 と同じ）、R11 は `fixed D10 仮`（台帳の D10）の形に直す。R15 は、本人に返す X1 を deep.md に名指しして `escalated` にする（X1 が deep.md に無ければ足す）。
  R1（C2）と R10（C1）は、聞かない既定で失わない側を選んだので、仮として台帳に載せるか、Q5 の族に出所を書いて `escalated Q5` にする
- 処置: fixed review/deep.md —— 問いへ上げたものを escalated に、R11 を fixed D10 仮に直し、deep.md の各 Q に出所を置いた。X1 を deep.md に名指しした

## R22. defaults.md の C4 は、D4 が直す前の「更新係のサービスは止めない」のまま残っている
- 成果物: openspec/changes/st58-update-apply-rollback/defaults.md
- 根拠: defaults.md:41-44 の C4 は「更新係のサービスは止めない」と書く。design.md:61-63 の D4 は、置き場を移せないので入れ替えの間だけ止めると直した。
  C1（defaults.md の 3 の「写して退ける」）も、D1（design.md:35「移す」）と違う。deep-questions.json の note は「AI が聞かずに決めたこと…は、GO 資料に出所つきで並びます」と言うので、GO 資料が defaults.md から読むと、本人は覆す前の文を見る
- kind: technical
- 提案: defaults.md の C4 に「★ spec の段で D4 に直した（入れ替えの間だけ止める）」を書き、C1 の 3 を「移す」に揃える。GO 資料が D の側を出すなら、そのことを確かめる
- 処置: fixed defaults.md —— C4 に D4 で直したことを書き、C1 の 3 を「移す」に揃えた

## R23. Task 1.1 の 2 つ目の検証コマンドは、cargo が引数を受け付けずに落ちる
- 成果物: openspec/changes/st58-update-apply-rollback/tasks.md
- 根拠: tasks.md:44 の `cargo test -p ashiato-server update_history export_update_history` を手元で走らせると `error: unexpected argument 'export_update_history' found`（`cargo test` が受ける名前は 1 つだけ）
- kind: technical
- 提案: 2 本に分ける（`cargo test -p ashiato-server update_history` と `cargo test -p ashiato-server export_update_history`）か、`cargo test -p ashiato-server -- update_history export_update_history` にする。どちらも `-- --list` で 1 本以上あることを見る
- 処置: fixed 1.1 —— 2 本に分け、--list の下限を足した

## R24. Task 6.1 の「`lint` の job」は CI に無い
- 成果物: openspec/changes/st58-update-apply-rollback/tasks.md
- 根拠: tasks.md:80「`lint` の job に `--self-test`」。`.github/workflows/ci.yml` の job は changes・vulns・rust・collector-windows・build-stamp・collector-windows-runtime・web・e2e・android・android-instrumented・smoke・package-linux・package-windows・install-windows・package-artifacts・chain で、`lint` は無い。
  ほかの道具の `--self-test` は `rust` の job（ci.yml:120-183）で走る
- kind: technical
- 提案: 「`rust` の job に `tools/release/update-apply-install.sh --self-test` を足す」に直し、検証に `bash -c 'grep -q "update-apply-install.sh --self-test" .github/workflows/ci.yml'` を足す
- 処置: fixed 6.1 —— `rust` の job に足す・grep で見る

## R25. Windows でだけ走る確かめ（Task 6.1・8.2）は、終了条件が自己試験と grep だけで、実地で 0 本走ってもチェックできる
- 成果物: openspec/changes/st58-update-apply-rollback/tasks.md
- 根拠: tasks.md:87 と :106 の検証は、自己試験・clippy・`ci.yml` の grep だけで、本物の Windows の通しの結果を条件にしない。
  前例の install-windows（ci.yml:503-523）は `install-check.sh --list` / `update-check-install.sh --list` と、走った `== Scenario:` の行を `comm` で突き合わせ、走らなかった Scenario があれば落とす。Task 6.1 の `update-apply-install.sh` には `--list` も突き合わせの段も書かれていない。
  そのため、段が分岐で飛ばされても CI は緑になる
- kind: technical
- 提案: Task 6.1 に、`update-apply-install.sh --list` と、install-windows で走った印との `comm` の突き合わせ（ST57 と同じ形）を足す。終了条件に「PR の CI の `install-windows` と `collector-windows-runtime` が緑」を書く（下流の手元で走らないことは今の注記どおり）
- 処置: fixed 6.1 —— `--list` と走った印の `comm` の突き合わせ・終了の条件に install-windows と collector-windows-runtime の緑。8.2 も

## R26. Task 9.2 の計測テストは、APK を入れ替えるとアプリのプロセスが落ちて gradle の XML に残らず、「4 本以上」の数えが満たせない見込み
- 成果物: openspec/changes/st58-update-apply-rollback/tasks.md
- 根拠: tools/android-emulator.sh:107-123 は「instrumentation が対象プロセス内で動く」ので、St56 の確かめを `am instrument` で直に呼び、gradle の XML を作らない。各段の XML は connected/ の同じ名前に上書きされ（:91-93）、ST57 は専用の置き場 `connected/st57-update` に写していた（:160-164）。
  Task 9.2 は `adb install -r` を外から行い、その後に「開かずに」収集が再開したかを見る。これは St56 と同じく、プロセスの外から入れ替えて `am instrument` で確かめる形になる。
  しかし検証（tasks.md:117）は `connected/*/TEST-*.xml` で St58 のクラスを 4 本以上数える。`am instrument` の結果はこの置き場に残らない
- kind: technical
- 提案: 段の手順（版違いの APK を 2 つ組む方法・`adb install -r`・`am instrument -e class …St58PackageReplacedInstrumentedTest#<段>`）を Task 9.2 に書く。終了条件は `am instrument` の出力の `OK (<n> tests)` を照らす形（St56 と同じ）か、`connected/st58-update` に写した XML を数える形にする
- 処置: fixed 9.2 —— 段の台本 `tools/android-st58-replace.sh` と `am instrument` の `OK` の照合

## R27. INDEX の capability 表の `data-durability` に ST58 が無い（理由は proposal にだけある）
- 成果物: docs/stories/INDEX.md・openspec/changes/st58-update-apply-rollback/proposal.md
- 根拠: docs/stories/INDEX.md:106 の `data-durability` の行は ST58 を持たない。proposal.md:51 は「INDEX の割当に無いが…説明の 1 文だけを MODIFIED で直す」と書く。
  前例は INDEX の側に訂正の注を置いている（INDEX.md:110・:131。ST29・ST41 を足した理由）。spec の「戻したことを、更新の履歴に…」（spec.md:384）は、ST51 の FR-121 も導出元に挙げる
- kind: technical
- 提案: INDEX.md の `data-durability` の行に ST58 を足し、`> **訂正（2026-11-15、ST58 の spec）**` の注で、`rolled_back` の値の説明の 1 文だけを直すこと（前倒しではないこと）を書く
- 処置: fixed proposal.md —— capability の照合の理由。docs/stories/INDEX.md にも `data-durability` に ST58 と訂正の注

## R28. Story の完了の判定と要件の本文が、推奨で書いた spec と食い違う。要件へ戻す 3 件は、機械が追う置き場に無い
- 成果物: docs/stories/ST58.md・openspec/changes/st58-update-apply-rollback/deep.md・proposal.md
- 根拠: docs/stories/ST58.md の完了の判定は「PC はインストーラ…が開き」。spec.md:72-81 と D12 は、PC にインストーラを作らず、収集アプリが自分で入れ替える。
  FR-119 に「待ち受けを開く前に落ちた」と「この回の退避がある回だけ」を、FR-120 に「入らなかった」を足すことも、proposal.md:31-34 の散文にだけある。
  triage は `要件へ戻すもの 0 件` を返すので、record の段で ★ が入らなくても、どの検査も落ちない（ST57 の review/spec.md R2 と同じ形）
- kind: conflict
- 提案: draft は止めない。record の段で、本人の答えに応じて deep.md に「要件へ戻すもの: FR-114（PC）・FR-119（待ち受けの前・この回の退避）・FR-120（入らなかった）」を置き、`docs/requirements.md` に ★ を入れ、`docs/stories/ST58.md` を作り直す。このことを、D の反転条件か tasks の前に置いて、機械の検査（triage の検査 4・check_chain の観点 8）が止められる形にする
- 処置: escalated —— 要件へ戻すもの（FR-114・FR-119・FR-120）は本人の答え（Q1・Q4・Q5）で決まる。proposal に挙げ、record の段が deep.md の「要件へ戻すもの」に置いて requirements.md に ★ を入れる（draft では要件を変えない）

## R29. 同梱の PostgreSQL の大きい版が違って止まった回は、更新の記録に残らず、帯にも設定にも出ない
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md
- 根拠: spec.md:433 は「状態の置き場に何も触れず」と書くので、`outcomes.jsonl` に行が無い。spec.md:572 の帯は、記録の最後の行だけを読む。
  Q4 の族（deep-questions.json の Q4 の family）は、移行が落ちた・移行の前に止まった・待ち受けの前に落ちたの 3 つで、この場面を持たない。窓には出るが、窓を閉じると本人の手元に何も残らない
- kind: daily
- 提案: 印と前の版の退け先は作らないまま、`outcomes.jsonl` にだけ `not_applied`（種別 `pg_major_mismatch`）の 1 行を足すか、窓だけでよいことを D5 に書く。足すなら spec.md:443 の THEN と D15 の種別の表に加える

## 観点ごとの該当なし

- 観点 6: 該当なし（`spec --draft` の第 1 回の先読みで、本人の答えはまだ無い。呼び出し元の指示どおり answers.md は書かない）

## 観点 7: 走っている change との前提関係

観点 7: 該当なし（spec_overlap.py candidates が 0 件）。`openspec/changes/` で archive でないのは st58-update-apply-rollback だけ。
- 処置: fixed D15 仮 —— 種別 `pg_major_mismatch` を表に足し、更新の記録に `not_applied` を 1 行。spec の大きい版の Requirement と Scenario・D5

---

# spec --answers Q8,Q9 の検査（第 2 回の答えの写し。2026-11-15）

## 機械の検査（docs/st58-upstream の 4f51b242f ＋ 作業ツリーの未 commit の差分の上）

- `openspec validate st58-update-apply-rollback --strict` → `Change 'st58-update-apply-rollback' is valid`
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（観点 8 も通る）
- `python3 scripts/check_scenarios.py . st58-update-apply-rollback` → `scenarios: FAIL (担保なし 82 件 / 名無しの確認待ち 0 件)`。82 件の名前を 1 つずつ tasks.md と突き合わせ、欠け 0（下流の前なので担保が無いのは予定どおり）。
  distribution の ADDED は 72 本（HEAD の 66 ＋ Q8 の 3 ＋ Q9 の 3）、MODIFIED は 64 本（HEAD の 42 ＋ 正典から写した「S-01 の版が変わると…」の 22。22 本は既に印があり、担保なしに出ない）。tasks の「ADDED 72・計 82」と合う
- `python3 scripts/review_triage.py . st58-update-apply-rollback` → `triage: OK`（指摘 44・仮決め 3・要件へ戻すもの 5）
- `python3 scripts/spec_overlap.py candidates . st58-update-apply-rollback` → `[]`
- MODIFIED「S-01 の版が変わると、更新の履歴に 1 行残る」と正典 `openspec/specs/distribution/spec.md` の塊の diff: 1 文の書き換え（結果の行の括弧に「でも `rolled_back` でもない」）・1 文の追加（`rolled_back` を結果の行に数えず、直前の `failed` の有無で 2 つの回を分けて読む）・★ 1 行。ほかは末尾の空行だけ。Scenario 22 本は字面が同じ。
  `base/distribution.md` の同じ塊は正典と同じ（末尾の空行だけ違う）。proposal・tasks の「読み方の 1 文だけ」は、実際には 1 文の書き換えと 1 文の追加
- `previous_version`（`crates/server/src/update_history.rs:128-136`）は `outcome IN ('applied','nothing_to_apply')` だけを読み、`startup_tx`（:256-300）と `write_in_tx`（:172-191）がそれを前の版にする。Scenario「待ち受けを開かずに戻した後も、前の版は戻した先の版のまま」の THEN（起動の行が足されない・次の試みの前の版が `0.4.0`）は、今のコードのまま成り立つ
- 終了コード 5 は `crates/server/src`・`crates/installer/src` のほかの所で使っていない。FR-121 の ★（`docs/requirements.md:1068`）と ST58.md の完了の判定（:57）は Q9 の答えと合う

## R30. D10 の「`onCreate` の前に読めるよう Intent で渡す」は、実物の `LocationService` では成り立たない
- 成果物: openspec/changes/st58-update-apply-rollback/design.md（D10）・tasks.md（9.1）
- 根拠: 種別を選んで `startForeground` を呼ぶのは引数の無い `onCreate()`（`collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt:397-403`）。Android は `onCreate` に起こした Intent を渡さず、Intent が届くのは `onStartCommand`（:899）だけ。
  `startForeground` を `onStartCommand` へ動かすと、:401 の「前景に上がってから置き場を開く」（review R29。前景に上がる期限 10 秒）に当たる。
  また `START_STICKY`（:928）の立て直しは Intent が `null` で来る。印が「付いていれば裏から」なので、裏の位置の権限が無い端末の立て直しは「画面から」と同じ扱いになり、位置の種別で裏から `startForeground` を呼ぶ。D10 の表は Intent が `null` の行を持たない
- kind: technical
- 提案: `onCreate` では Intent に頼らず、「位置の権限と裏の位置の権限が両方あれば `location`、そうでなければ `specialUse`」で選ぶ。画面（`MainActivity.kt:414`）が起こす Intent に「前景から起こした」の印を付け、`onStartCommand` でその印があるときだけ `location` へ上げる（印の向きを逆にする）。
  D10 の表に「立て直し（Intent が `null`）」の行を足し、9.1 の「`foregroundServiceType()` に『裏から起こした』の入力を足す」を直す
- 処置: fixed D10 —— Intent でなく 1 回きりの印のファイル（`noBackupFilesDir/update/replaced-start`）を受け手が置き、`onCreate` が読んで消す形にした。画面からの Intent に「前景から起こした」の印を付け、`onStartCommand` で上げる。立て直し（Intent が `null`）は今のまま（表に行を足した）。tasks 9.1 も直した

## R31. 「位置のソースを取得条件の欠けたソースとして扱う」を見る Scenario が無い。「位置の記録は積まれない」は、位置を取っていても真になる
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md
- 根拠: spec.md:54 の本文は「位置のソースを取得条件の欠けたソースとして扱って位置を取らず」と言う。正典 device-collection（`openspec/specs/device-collection/spec.md:234`）は、生存信号にソースごとの「取得できる状態か」を載せる。
  今の判定 `androidCapability`（`AndroidCapability.kt:27-46`）は `ACCESS_FINE_LOCATION` だけを見るので、手を入れなければ、この状態の生存信号は位置を「取れる」と言ったまま 0 件になる（深掘り（ST06）Q5 が塞いだ「壊れているのに動いていた」の形）。
  Scenario spec.md:67 の THEN「位置の記録は積まれない」は、30 秒の待ち（位置の間隔は 60 秒）で、エミュレータに位置を流し込まなければ、位置を取っていても真になる。本文だけが言い、Scenario が言っていない
- kind: technical
- 提案: Scenario を 1 本足す:「裏の位置の権限が無く位置を除いた種別で動く間、生存信号は位置のソースを取れない状態（満たされていないもの: 権限）と載せる」。THEN は生存信号の欄で見る。spec.md:67 の「位置の記録は積まれない」は、エミュレータに位置を流し込んだ上で見るか、外す。
  どの理由（権限・センサ・接続）で載せるかを D10 の表と spec の本文に書く
- 処置: fixed specs/distribution/spec.md —— 本文に「生存信号に位置のソースを取得できない状態・満たされていないもの「権限」」を足し、Scenario「携帯は裏の位置の権限が無くても…」の THEN を生存信号の欄で見る形に替えた（「位置の記録は積まれない」を外した）。D10 に `AndroidCapability` へ種別を入力として足すことを書き、9.1 に Robolectric の試験を足した

## R32. Task 9.2 の段の確かめは `am instrument` で始めるので、受け手が起こしたサービスを確かめの前に止めうる。「開くと位置の種別に上がる」は、上げ直しを通らずに緑になりうる
- 成果物: openspec/changes/st58-update-apply-rollback/tasks.md（9.2）
- 根拠: 9.2 は `adb install -r` の後に `adb shell am instrument -w -e class …#<段>` で確かめる。Android の ActivityManager は計測を始めるとき、`--no-restart`（API 31 以降）が無ければ対象のパッケージを止めてから起こし直す（AOSP `ActivityManagerService.startInstrumentation` の `forceStopPackageLocked`）。前例の `tools/android-emulator.sh:113-119`（St56）は、入れ替えた後のプロセスが生きていることを見ない段なので、この問題に当たらない。
  止められると、「開かずに 30 秒待つ」の段はサービスが動いていないので落ちる。Scenario spec.md:86（開くと上がる）は、新しく起きたプロセスが最初から `location` で立って緑になり、`onStartCommand` での上げ直しを通らない。
  加えて、`MainActivity` は求めた権限をメモリにだけ持つ（`MainActivity.kt:71`）。開くたびに、裏の位置の権限を求める画面（`niceToHave()` :172-180）が `start()`（:414）より先に出る。WHEN「本人がアプリを開く」の段は、この画面を越える手を書いていない。位置を流し込む手（`adb emu geo fix` など）も書いていない
- kind: technical
- 提案: 9.2 の段の確かめは `am instrument --no-restart` にする（その API 段のエミュレータで走らせる）。あるいは、計測を使わずに台本が `adb shell dumpsys activity services dev.ashiato.collector` とログの種別の行で見る。
  上げ直しの段は「同じプロセスの id のまま、種別の行が `specialUse` → `location` と続く」を THEN にする。権限を求める画面の答え方と、位置を流し込む手を 9.2 に書く
- 処置: fixed 9.2 —— `am instrument --no-restart`・確かめの前後でプロセスの id が同じことを見る・画面を `am start` で開き裏の位置の権限の画面を UiAutomator で断る・`adb emu geo fix` で位置を流し込む、を書いた。Scenario「…開くと位置の収集を再開する」の THEN を「同じプロセスのまま」に

## R33. 「動いているサービスの種別を `startForeground` の呼び直しで変えられる」が premises に無い
- 成果物: openspec/changes/st58-update-apply-rollback/design.md（D10）・premises.md
- 根拠: D10 は「Android は動いているサービスの種別を `startForeground` の呼び直しで変えられる。前景の画面から呼ぶので位置の型で始めてよい」と書き、Scenario spec.md:86 はこれに依る。`grep -n "呼び直" premises.md` は 0 件。
  P7 は「受け手から始めてよい」だけを持ち、9.2 は P6・P7 だけを 4 に上げる
- kind: premise
- 提案: premises に P を 1 つ足す（公式文書 `Service#startForeground(int, Notification, int)` と、前景から呼んだときの while-in-use の扱い。確かさ 2）。9.2 の段「開くと位置の収集を再開する」の結果で 4 に上げると書く。外れたときの先（サービスを止めて起こし直す）を D10 に書く
- 処置: rejected: 人間に返す前提ではない —— Q8 の答え（位置以外を取り続ける・位置は開くまで止まる）はこの前提に依らず、外れても開いたときの上げ方（サービスを止めて画面から起こし直す。design D10）が変わるだけで、失うもの（loss）が無い（openspec/changes/st58-update-apply-rollback/design.md:109）。前提は openspec/changes/st58-update-apply-rollback/premises.md:42 の P26（確かさ 2）に書き足し、9.2 の計測テストで 4 に上げる

## R34. 通知 ID 12「位置の収集が止まっています」を消す時が決まっていない
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md・design.md（D10）
- 根拠: spec.md:54 は「1 回出す」だけを言う。`grep -n "消す\|cancel" design.md` で、通知 ID 12 を消す記述は無い。開いて位置の種別に上がった後も、「止まっています」が通知の欄に残りうる
- kind: technical
- 提案: 「位置の種別に上がったら通知 ID 12 を消す」を spec の本文に足し、Scenario spec.md:86 の THEN に「通知 ID 12 は出ていない」を足すか、別の Scenario にする
- 処置: fixed specs/distribution/spec.md —— 上げたら通知を消すことを本文と Scenario の THEN に足した。D10 の表と 9.1 にも

## R35. 控えの 1 行を標準出力で渡す経路は、サーバの CLI の既定のログの出口（標準出力）と混ざる。終了コード 5 の経路をインストーラ側で通す確かめが無い
- 成果物: openspec/changes/st58-update-apply-rollback/design.md（D18（仮）・D26）・tasks.md（1.1・5.1・6.1）
- 根拠: D18（仮）は今回「書けなかった行を標準出力に 1 行 1 件の JSON で出し、インストーラはその行をそのまま控えに書く」を足した。サーバの CLI の口は `tracing_subscriber::fmt().with_env_filter("info").init()` で、書き先を指定していない（`crates/server/src/lib.rs:400`。`run_migrate`）。この形では、ログは標準出力に出る。
  `record-rollback` が同じ形で始まると、`warn_unwritten`（`update_history.rs:86-98`。表が無い・CHECK が断るときに出す）の行が JSON の行に混ざり、そのまま控えに入る。混ざった行は `replay` が読めない。
  5.1 の「終了コード 5 なら控えに書く」には試験が無い。6.1 の通しは 2 つの zip がどちらも `rolled_back` を知るので、終了コード 5 を通らない。1.1 の試験はサーバの側だけ
- kind: technical
- 提案: 控えの行は、インストーラが渡すファイルの場所（例: `--pending-out <path>`）に `record-rollback` が書く形にするか、`record-rollback` のログを標準エラーへ向けることを D18（仮）に書く。
  5.1 に、偽の `ashiato-server`（終了コード 5 と 1 行を出す）で控えの中身が JSON の行だけになることを見る `st58_plan_` の試験を足す
- 処置: fixed D18 仮 —— D26・tasks 1.1・5.1 もあわせて、標準出力でなく `--pending-out <控えのファイル>` に `record-rollback` が書く形にし、5.1 に偽の `ashiato-server` で控えが JSON の行だけになることを見る試験を足した

## R36. design D14 の名前の一覧が「戻した 2 行を書く」のまま
- 成果物: openspec/changes/st58-update-apply-rollback/design.md
- 根拠: design.md:137 は「`ashiato-server update-history record-rollback`（戻した 2 行を書く。…）」。D3 の 3.（:57）・D18（仮）（:161）・D26 は「移行が落ちた回は 2 行、待ち受けを開かずに戻した回は『戻した』の 1 行」に直した
- kind: technical
- 提案: :137 を「戻した行を書く（移行が落ちた回は 2 行、待ち受けを開かずに戻した回は 1 行）」に直す
- 処置: fixed design.md D14

## R37. spec の本文に実装の名前 `MIGRATIONS` が入った
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md
- 根拠: spec.md:477「新しい版の並び（`MIGRATIONS`）にあって、戻した DB の版の記録に無い版を、並びの順に」。`MIGRATIONS` は `crates/server/build.rs` が組む Rust の定数。正典の同じ並びは `migrations/ORDER` の順と書く（spec.md:916 の `applied` の説明）
- kind: technical
- 提案: 「新しい版が知っている版の並び（`migrations/ORDER` の順）にあって…」に直す。`MIGRATIONS` は design D26 と tasks 1.1 にだけ置く
- 処置: fixed specs/distribution/spec.md —— `migrations/ORDER` の順と書いた

## R38. MODIFIED した読み方は、控えから写した `failed` を、その後に通った回の 2 つ目の結果と読ませる
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md
- 根拠: spec.md:924 は結果の行を「`via` が `migrate` で `attempted` でも `rolled_back` でもない行」と定め、次の試みの行より前にあれば、その試みの結果と読ませる。
  移行が落ちて戻した先が `rolled_back` を受け付けない回は、`failed`・`rolled_back` の 2 行が、次に通った更新の `attempted`・`applied` の後に写る（spec.md:480・data-durability の説明書）。そのため、写した `failed` は、通った試みの後の 2 つ目の結果の行に読める。
  今回足した 1 文（spec.md:925）は `rolled_back` だけを外し、`failed` の扱いを持たない。説明書は時刻（`at`）で見分けると書くが、Requirement の読み方は `seq` だけで定める
- kind: technical
- 提案: spec.md:924 の読み方に「控えから写した行（時刻 `at` が直前の試みの行より前の行）は、どの試みの結果にも数えない」を足すか、説明書の文と同じ見分け方を Requirement に書く
- 処置: fixed specs/distribution/spec.md —— MODIFIED の読み方に「試みの結果の行は今の版が同じで、時刻が試みの行以後のもの。控えから写した `failed` と `rolled_back` は 2 行で 1 つの戻した回」を足した

## 観点ごとの該当なし（今回の差分）

- 観点 1: Q8 の族（裏の位置あり → spec.md の 1 本目の THEN を「位置の種別」に・裏の位置なし → :67・位置の権限なし → :74・通知の権限なし → :80・開いて上げ直す → :86）と Q9 の族（1 行・当てた版あり → :503・当てる版なし・前の版のまま・控えに 1 行）は、どれも Scenario がある。
  当初案（通知だけ・`applied` の書き直し）の文は spec・design・tasks・proposal に残っていない（`grep -n "当てた結果と戻したの\|起こさずに"` で、design の「仮の形」の説明だけ）。数値（30 秒・1 回）は Scenario にある。FR-121 の ★ はある。食い違いは R30〜R35 の噛み合いだけ
- 観点 4: 1.1 の `--list` の下限 8 は、1.1 が名指す 8 本と合う。9.2 の「7 つの段」は 9.2 が名指す 7 本と合う。K1（`st51_form_rejects_unknown_via`）は `update_history_tests.rs:87-93` のとおり `("S-01","migrate","rolled_back")` を断る組を持つ
- 観点 5: ST58.md の完了の判定（:57）は Q9 と合う。Q8 は要件の本文に効かない（deep.md）
- 観点 7: 該当なし（spec_overlap.py candidates が 0 件）

## 直しの確かめ（R30〜R38 の処置の後。2026-11-15）

- R30: D10（design.md:89-112）は印のファイル（`noBackupFilesDir/update/replaced-start`）を `onCreate` が読む形になり、`onCreate` に Intent が要らない。表に立て直し（Intent が `null`）の行がある。解けた。
  受け手から起こした後、開く前に OS が立て直したときは今の立て直しと同じ形になる（位置の型で裏から立つ）。これは design.md:108 が「この Story では変えない」と書く既存の形なので、指摘にしない
- R31: spec.md:54 の本文と Scenario「携帯は裏の位置の権限が無くても…」の THEN は、生存信号の「取得できない・権限」で見る形になった。解けた（ただし R39）
- R32: 9.2 に `--no-restart`（エミュレータは API 35。`tools/android-emulator.sh:21`）・`pidof` の前後の一致・`am start` と UiAutomator・`adb emu geo fix` が入った。解けた（ただし R42）
- R33: rejected。P26 は premises.md:42 にあり、9.2 が 4 に上げる（ただし R40）
- R34・R36・R37: 解けた（spec.md:56・design.md:140・spec の本文に `MIGRATIONS` が無い）
- R35: D18（仮）（design.md:164）・D26・tasks 1.1・5.1（:75・:78）が `--pending-out` で揃い、標準出力で行を渡す文は残っていない。5.1 に、偽の `ashiato-server` で控えが JSON の行だけになることを見る試験がある。解けた
- R38: spec.md:926 に読み方が足された（ただし R41）

## R39. 「`specialUse` の間は位置の権限があっても取れない」は、design と tasks が spec より広い。設定から後で許した端末の位置を止める
- 成果物: openspec/changes/st58-update-apply-rollback/design.md（D10）・tasks.md（9.1）
- 根拠: spec.md:54 が欠けと定めるのは「位置の権限は許されているが裏の位置の権限が許されておらず、位置を除いた種別で動いている」間だけ。
  design.md:106 と tasks.md:115 は「種別が `specialUse` の間は、位置の権限があっても『取得できない（権限）』」と書き、裏の位置の権限を条件に持たない。
  そのため、画面から位置の権限なしで `specialUse` で立ったサービス（D10 の表の 2 行目）の端末で、本人がアプリを開かずに端末の設定から位置と裏の位置の権限を両方許すと、今は次の取得契機から位置を取りにいく（`LocationService.kt` の `startTicking` の「取得条件を見ずに張る —— 後から許可した端末が次の契機で戻る」）。D10 の形では、開くまで位置が止まる。
  spec にこの振る舞いの変更は無く、Scenario も無い
- kind: technical
- 提案: design.md:106 と 9.1 の条件を spec に合わせて「種別が `specialUse` で、裏の位置の権限が無い間」にする。Robolectric の表の試験に「`specialUse`・位置と裏の位置の権限あり → 取得できる」の 1 本を足す
- 処置: fixed D10 —— 欠けにする条件を spec と同じ「位置の権限があり裏の位置の権限が無い間」にした。9.1 の文と Robolectric の試験（裏の位置の権限も許された後は取れる）を足し、下限を 22 にした

## R40. P26 が外れたときの代わりの形（止めて起こし直す）は、spec の「サービスを止めずに」と両立しない
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md・design.md（D10）
- 根拠: spec.md:56 は「サービスを止めずに位置の種別に上げ直し」と言う。design.md:109 は P26 が外れたら「`stopSelf` → `startForegroundService`」に替えると書く。R33 の処置は「外れても上げ方が変わるだけで失うものが無い」として rejected にしたが、替えると spec.md:56 の本文が偽になり、spec を直す（上流へ返す）ことになる
- kind: technical
- 提案: spec.md:56 から「サービスを止めずに」を外し、観測できる形（Scenario の「同じプロセスのまま」・通知を消す・位置の記録が積まれる）だけを残す。あるいは、D10 の外れたときの先に「spec.md:56 を直す（上流へ返す）」と書く
- 処置: fixed specs/distribution/spec.md —— 本文から「止めずに」を外した。Scenario の THEN（同じプロセスのまま・通知を消す・位置の記録）はそのまま

## R41. 試みの結果の行を時刻で絞る読み方は、正典の「時刻で決めない（S-01 の時計は戻りうる）」と食い違う
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md
- 根拠: R38 の処置で、spec.md:926 に「時刻がその試みの行の時刻以後のものに限る」が入った。同じ表の Requirement の spec.md:882 は「『書いた順』を `seq` で決め、時刻で決めない（S-01 の時計は戻りうる）」。
  試みと結果の間に時計が戻ると、本物の結果の行が「結果を書く前に終わった回」（spec.md:925）と読まれる。
  版の条件（今の版が同じ）だけでは、戻した後に同じ版で走らせ直して通った回が残る。この回は、写した `failed` が今の版も同じになり、時刻の条件に頼る
- kind: technical
- 提案: 時刻でなく `seq` で絞る。「試みの行の結果の行は、その試みの行の後で最初に現れる、`via` が `migrate` で `attempted` でも `rolled_back` でもない行 1 つだけ。2 つ目以降（控えから写した `failed`）は、すぐ後の `rolled_back` と 2 行で 1 つの戻した回と読む」。data-durability の説明書の文も同じ見分け方にそろえる
- 処置: fixed specs/distribution/spec.md —— 時刻の条件を外し、`seq` で「試みの後の最初の結果の行 1 つ」に限り、すぐ後に `rolled_back` がある 2 つ目以降の `failed` は戻した回の 2 行と読む形にした

## R42. Task 9.2 の手順の文は、先頭の命令に `--no-restart` が無いまま
- 成果物: openspec/changes/st58-update-apply-rollback/tasks.md（9.2）
- 根拠: tasks.md:120 の手順の文は `adb shell am instrument -w -e class …#<段>` で確かめると書く。同じ項目の後ろの括弧では「段の確かめは `am instrument --no-restart` で始め」と書く。先頭の命令をそのまま写すと、R32 の止まりが戻る
- kind: technical
- 提案: 先頭の命令を `adb shell am instrument --no-restart -w -e class …#<段>` に直す
- 処置: fixed 9.2 —— 先頭の命令を `am instrument --no-restart -w …` にした

## 直しの確かめ（R39〜R42 の処置の後。2026-11-15）

- R39: design.md:106 と tasks.md:115 の欠けの条件が、spec.md:54 と同じ「`specialUse` で、位置の権限があり裏の位置の権限が無い間」になった。9.1 の Robolectric に「裏の位置の権限も許された後は取得できる」が入り、下限は 22。解けた
- R40: spec.md:56 から「止めずに」が外れた。Scenario の THEN は観測できる形（同じプロセスのまま・通知を消す・位置の記録）のまま。P26 が外れたときの形（`stopSelf` → `startForegroundService`）でも、プロセスは残るので THEN は変わらない。解けた
- R41: spec.md:926 は時刻を使わず、`seq` で「試みの後の最初の結果の行 1 つ」に限る形になった。
  移行が落ちた回の `failed`（戻した直後に書く。最初の結果の行）と、控えから写した `failed`（通った回の結果の後。2 つ目以降）の両方を、spec.md:927 の `rolled_back` の読み方と矛盾なく読める。解けた
- R42: 9.2 の `am instrument` は 2 か所とも `--no-restart` になった。解けた

## R43. 欠けの条件と開いたときの上げ直しは、更新の後に限らず画面から立った `specialUse` にも効く。design は「画面からは今の振る舞いを変えない」と書く
- 成果物: openspec/changes/st58-update-apply-rollback/design.md（D10）・specs/distribution/spec.md
- 根拠: spec.md:54 の IF（「位置の権限は許されているが裏の位置の権限が許されておらず、位置を除いた種別で動いている」）と design.md:106 の欠けの条件は、起こし方を問わない。design.md:96 の上げ直し（「前景から起こした」の印があり、`specialUse` で位置の権限がある）も同じ。
  そのため、画面から位置の権限なしで `specialUse` で立った端末（D10 の表の 2 行目）が後で位置の権限だけを許すと、ST58 の後は、生存信号が位置を「取得できない・権限」と載せる。今は `androidCapability`（`AndroidCapability.kt:44-46`）が FINE だけを見て「取得できる」と載せる。また、開くと `location` に上がる。今は立て直すまで `specialUse` のまま。
  どちらも Q5 の形を直す向きの変更で、失うものは無い。ただ design.md:108 は「画面からと立て直しは今の振る舞いを変えない」と書き、D10 の表の 2 行目も「画面から開いた後 `specialUse`」だけを持つ。Scenario はどれも更新の後（`adb install -r`）の GIVEN だけ
- kind: technical
- 提案: design.md:108 を「種別の選び方（`onCreate`）は画面からと立て直しで今のまま。欠けの条件と開いたときの上げ直しは、起こし方を問わず効く」に直す。D10 の表に「画面から（位置の権限なしで立った後に、位置の権限だけを許した）」の行を足す。
  変えたくなければ、spec.md:54 と design.md:96・:106 の条件に「受け手から起こした（印のファイルを読んだ）間」を足す
- 処置: fixed D10 —— 欠けの扱い・開いたときの上げ直し・通知を「受け手から起こしたサービス」だけに限り（spec の本文 2 文・design・tasks 9.1）、画面からと立て直しは今の振る舞いのままにした

## 直しの確かめ（R43 の処置の後。2026-11-15）

- R43: spec.md:54・:56 は「更新の後に位置を除いた種別で起こしたサービス」に限った。:55 の通知の文は、もとから更新の後に限っている。
  design D10 の上げ直し・欠けの条件・「画面からと立て直しは今の振る舞いを変えない」の文と、tasks 9.1 も「受け手から起こしたサービス」に限った。D10 の表の 2 行目（画面から・`specialUse`・欠け）は今のまま。
  受け手から起こしたことは、印のファイルを `onCreate` が読んで消した後も、そのサービスが動いている間だけ持つ。後の立て直しには残らない（design D10）。spec の本文・design・tasks の 3 つで条件が揃う。解けた。新しい食い違いは無い
