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
- 処置: fixed D26 仮

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
