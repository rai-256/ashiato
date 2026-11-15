# 整合の確認 — FR-121 を ST58 の深掘り第 2 回の答え（Q9）で訂正した（2026-11-15）

- 基準: `bf5635892cf1c6420c52f8cb41b2e7455c87b450`..`c7edb7c2e79617c94fc107da544ec88be90bc089`

## 機械の入力（写し）

`python3 scripts/req_impact.py . --base bf5635892cf1c6420c52f8cb41b2e7455c87b450`（改訂した要件 1 件・扉 0 項）:

- FR-121（改訂）: `docs/handoff/ST51.md:6,14,16,23,27`・`docs/stories/INDEX.md:75,506`・`docs/stories/stories.json:1206`・
  `openspec/changes/st58-update-apply-rollback/base/data-durability.md:12`・`base/distribution.md:220`・`deep.md:53,87`・`review/spec.md:267`・
  `specs/data-durability/spec.md:12`・`specs/distribution/spec.md:460,842`・
  `openspec/specs/collection-coverage/spec.md:1925,1969,1971,2052`・`openspec/specs/data-durability/spec.md:960`・`openspec/specs/desktop-collection/spec.md:1132`・
  `openspec/specs/device-collection/spec.md:2534`・`openspec/specs/distribution/spec.md:323,324,476,502`

`python3 scripts/check_chain.py .`（chain: FAIL 1 件 / 未回収 0 件 / warn 0 件）:

- [FAIL] 整合の確認: 整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-121（この監査がそれに当たる。★ 2026-11-15 訂正はこのファイル名を名指ししている）
- 検査 10（判断の日より新しい訂正）は FR-121 について何も出していない —— FR-121 を satisfies に持つ ST51 は archive 済みの change を持ち、Q9 を実装する ST58 は satisfies に FR-121 を持たず、しかも走っている change を持つ（下の C1 はその穴）

走っている change（`openspec/changes/` の archive 以外）は `st58-update-apply-rollback` の 1 本だけ。tasks.md は 15 項とも `[ ]` で、`record-rollback` も `MY_PACKAGE_REPLACED` の受け手もまだコードに無い。
本人の答え（`deep-answers-2.txt`）は Q8・Q9 とも推奨と同じ側で、design の D10・D26 は「答えが出るまで下の形で作らない」と止めてある。失われるもの: なし。

## 1. 改訂後の要件と、ほかの文書

## C1. Q9 の ★ が決めた更新の履歴の形は ST58 が作るのに、ST58 の satisfies・完了の判定・INDEX に FR-121 が無く、FR-121 を持つ ST51 は archive 済み —— 改訂がどの生きた Story の判定にも届かない
- 重大度: 中
- 文書: docs/stories/stories.json:1394
- 要件: FR-121, FR-120
- 根拠: stories.json:1394-1398「"satisfies": [ "FR-114", "FR-119", "FR-120" ]」・:1413「サーバの更新で移行が失敗するか、新しい版が待ち受けを開く前に落ちると、この回の退避から DB が戻り、前の版で起動し直す。戻したこと・入らなかったこと…が、インストーラの窓と画面の帯（次に更新が通るまで）に知らされる」（更新の履歴の行の形と「前の版」の読みは判定に無い）・:1418「"judged": "2026-11-15"」。INDEX.md:82「| [ST58](ST58.md) | … | FR-114, FR-119, FR-120 |」。ST58.md:4「satisfies: [FR-114, FR-119, FR-120]」—— ST58.md が引く要件は FR-120 の ★ の「戻したことは更新の履歴（FR-121）にも `rolled_back` として残す」（:32）までで、requirements.md:1068「**待ち受けを開かずに戻した回は、当てた結果の行を書き直さず、`rolled_back` の 1 行に当てた版の名前を持たせる**…戻した後の次の更新の「前の版」は戻した先の版のまま」は写らない。FR-121 を satisfies に持つのは ST51（stories.json:1205-1207）だけで、その change は `openspec/changes/archive/2026-11-01-st51-update-history`。Q9 は loss: exported（書き出しに出た行は直せない）
- 直すところ: stories（ST58 の satisfies に FR-121 を足すか、done に「待ち受けを開かずに戻した後も、更新の履歴の前の版は戻した先の版のままで、戻した行が当てた版の名前を持つ」を足す → make_story.py で ST58.md を貼り直す）
- 機械: 安く捕まえられる（check_chain.py の検査 10 を、改訂した要件の ★ が名指しする Story（「ST58 の深掘り Q9」）にも warn で広げる。今は satisfies でしか引かない）
- 行き先: stories（ST58 の done に「待ち受けを開かずに戻した後も、更新の履歴の前の版は戻した先の版のままで、戻した行が当てた版の名前を持つ」を足し、make_story.py で ST58.md を貼り直した。satisfies は変えない —— FR-121 の本体は ST51 が作り済みで、ST58 が作るのは戻した回の行の形だけ）

## C2. ★ の「退避の控え」は新しい呼び名で、spec・design の「状態の置き場の控え（`history-pending.jsonl`）」と合わず、移行の前の退避（DB の写し）の控えと読める
- 重大度: 低
- 文書: docs/requirements.md:1068
- 要件: FR-121, FR-119
- 根拠: requirements.md:1068「（**退避の控え**から写す回も同じ）」。requirements.md:1049 の FR-119 で「退避」は「移行の前の退避（FR-66 とは別。`ashiato-server migrate` が当てる前に毎回取る）」= DB の写し。spec（changes/st58-update-apply-rollback/specs/distribution/spec.md:458）「2 行を**状態の置き場の控え**（`<状態の置き場>\update\history-pending.jsonl`）に書き」・design.md:149「控え（`history-pending.jsonl`）」・tasks.md:24「`history-pending.jsonl`」。「退避の控え」は repo の中でこの 1 行だけ（`grep -rn '退避の控え' docs openspec`）。requirements.md:1843-1844 の「控え」は鍵の USB の写しの意味
- 直すところ: requirements（「退避の控え」を「状態の置き場の控え（前の版の表が `rolled_back` を受け付けない回）」に）
- 機械: できない
- 行き先: requirements（★ の「退避の控え」を「状態の置き場の控え（前の版の表が `rolled_back` を受け付けない回）」に直した）

## C3. ST60 への申し送り（Linux の更新と戻し）は FR-119・FR-120 の ★ だけを挙げ、FR-121 の Q9 の ★（戻した回の更新の履歴の行の形）を名指ししない
- 重大度: 低
- 文書: docs/handoff/ST60.md:21
- 要件: FR-121, FR-131
- 根拠: ST60.md:19「Linux のサーバの更新と戻し（FR-114・FR-119・FR-120）も…扱う。手順の段・印・戻す退避の特定（回ごとの空の置き場）・**更新の記録の形**は ST58 の正典…と design D1〜D3・D14（仮）・D20（仮）にそろえる」・:21「FR-119・FR-120 に ★ 2026-11-15 が入った…ST60 の satisfies は FR-131 だけなので、Linux の更新と戻しもこの ★ にそろえる」。「更新の記録」は spec ではインストーラの `outcomes.jsonl`（changes/…/specs/distribution/spec.md:426）で、更新の履歴（`core.update_history`）の「戻した」の行の形は design D18（仮）・D26 にあり、申し送りの挙げる D の中に無い。stories.json:1446-1448 の ST60 の satisfies は `FR-131` だけ・:1461「移行が失敗すると前の版に戻る」。Linux で戻した回も requirements.md:1068 の形（結果の行を書き直さず `rolled_back` に当てた版の名前）で書かないと、Linux の書き出しだけ「前の版」がずれる
- 直すところ: 凍結した Story への申し送り（ST60.md の 2026-11-15 の段に「FR-121 の ★ 2026-11-15 訂正（Q9）の更新の履歴の行の形も。design D18・D26」を足す）
- 機械: 安く捕まえられる（C1 と同じ。ST60 の satisfies に FR-121 を足せば検査 10 が引く）
- 行き先: handoff ST60

## C4. 正典の ST51 の「試みの後に結果の行が無ければ、戻した回と読める」は、ST58 の `rolled_back`（きっかけ `migrate`）で成り立たなくなり、Q9 で待ち受けを開かずに戻した回は `rolled_back` の 1 行がそのまま「その migrate の結果」に読まれるが、change はこの Requirement を MODIFIED にしていない
- 重大度: 低
- 文書: openspec/specs/distribution/spec.md:319
- 要件: FR-121
- 根拠: distribution/spec.md:319「試みの行の後に、同じ migrate の結果の行（`via` が `migrate` で移行の結果が `attempted` でない行）が、次の試みの行より前に無いことで、その回が結果を書く前に終わった（**退避から D-01 を戻した**・途中で止まった）と読めるようにする」。changes/…/specs/distribution/spec.md:455「「戻した」（**きっかけ `migrate`**…移行の結果 `rolled_back`）」・:864「`via` が `migrate`・`outcome` が `rolled_back` の行」。requirements.md:1068「当てた結果の行を書き直さず、`rolled_back` の 1 行に当てた版の名前を持たせる」—— 待ち受けを開かずに戻した回の行は「試み → `rolled_back`（`applied_versions` あり）」になり、:319 の読み方では `rolled_back` が同じ migrate の結果で、当てた版を持つので「当てた」と読める。当てる版の無い回（design.md:194「多くの版はこれ」）は `rolled_back`（当てた版は空）だけで、移行が落ちた回（`failed` → `rolled_back`）とは直前の `failed` の有無でしか分かれない（FR-121 本文 requirements.md:1054「（S-01 なら）移行が通ったか」）。change の MODIFIED は distribution:831 の表の形だけ（changes/…/specs/distribution/spec.md:558-831 の見出し）。読み方が `rolled_back` を前提にしていないのは Q6（`failed` と `rolled_back`）からだが、Q9 で `rolled_back` が回の唯一の行になった
- 直すところ: その文書（change の spec に、:319 の Requirement を MODIFIED で「`rolled_back` は戻した行で、直前に `failed` が無ければ移行は通っていた」と読めるように書き、書き出しの説明書にも同じ読み方を書く。spec --answers の段）
- 機械: 安く捕まえられる（Q9 の Scenario「待ち受けを開かずに戻すと…」の THEN に「書き出しの説明書に `rolled_back` の前に `failed` が無い回の読み方がある」を足す）
- 行き先: leave: 走っている change の spec を書くのは次の spec --answers の段。deep.md の「spec の段へ渡すもの（第 2 回の整合の確認）」に C4 として置いた

## C5. 走っている change の中の文書が、Q8・Q9 の前の形（D10 の「起こさず通知だけ」・D26 の「当てた結果と戻したの 2 行」・IMP-07 の受け入れ）のまま
- 重大度: 低
- 文書: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md:451
- 要件: FR-121, FR-114
- 根拠: Q9: spec.md:451「### Requirement: 戻したことを、更新の履歴に「落ちた試みの結果」と「戻した」の 2 行で残す」・:454「待ち受けを開かずに…この回の移行の結果の行（…`applied`…`nothing_to_apply`）を時刻を戻した時にして足し」・:455「当てた版は空」・:480-484 の Scenario「待ち受けを開かずに戻すと、更新の履歴に当てた結果と戻したの 2 行が足される」。specs/data-durability/spec.md:8「移行の結果 `rolled_back` は、移行が落ちて退避から D-01 を戻した回で」（待ち受けを開かずに戻した回と当てた版の名前が無い）。design.md:190-196（D26）。tasks.md:40・:42・:83（`applied` と当てた版の名前を渡す・2 行の Scenario）。agreement.json:41-43・go.json:1219・:2367-2370・:2575（IMP-07 を残るリスクとして受け入れ）。review/spec.md R3。Q8: spec.md:53「裏の位置の権限が許されておらず…「…開いてください」の通知を 1 回出す」・design.md:89-95（D10）・tasks.md:111・:117・review/deep.md R11。requirements.md:1068「上の補足の「落ちた新しい版と読みうる（IMP-07）」は起きなくなる」・deep.md「当初案を覆したもの（第 2 回）」
- 直すところ: その文書（spec --answers が Q8・Q9 を spec・design・tasks に写し、agreement / go の IMP-07 を閉じる）
- 機械: 捕まえられた（gate が D10・D26 を FAIL にして deep.md に返した。`docs/briefs/ST58-gate.md:11-12`）
- 行き先: leave: 走っている change の中の文書で、次の spec --answers の段が deep.md の第 2 回の答えから写す。design の D10・D26 は「答えが出るまで下の形で作らない」と止めてあり、tasks は 15 項とも未着手でコードも無いので、古い形で作られたものは無い

## 2. 文書とコード

## K1. tasks.md 1.1 は ST51 の試験「決めていない値の行は入らない」がそのまま通ると書くが、その試験は `rolled_back` を「決めていない値」として断られることを確かめており、1.1 の移行で落ちる
- 重大度: 中
- 文書: openspec/changes/st58-update-apply-rollback/tasks.md:43
- 要件: FR-121
- 根拠: tasks.md:39「`core.update_history` の `outcome` の CHECK に `rolled_back` を足す」・:43「既にある `決めていない値の行は入らない`（`update_history_tests.rs`）と…**が通る**」。`crates/server/src/update_history_tests.rs:89-93` の `st51_form_rejects_unknown_via` は `("S-01", "migrate", "rolled_back")` を入れて `unwrap_err()` し、:106-110 で SQLSTATE `23514` を求める。CHECK に `rolled_back` が入るとこの 1 件は通ってしまい、`unwrap_err()` で落ちる。正典の Scenario（distribution/spec.md「決めていない値の行は入らない」）が言うのは `via` が `upgrade` の行だけで、`rolled_back` の組は試験が足したもの。Q9 の改訂とは別の、change の中の食い違い
- 直すところ: その文書（tasks.md 1.1 に「`st51_form_rejects_unknown_via` の `rolled_back` の組を、まだ決めていない別の値に替える。印は消さない」を足す）
- 機械: 捕まえられる（下流の 1.1 の検証 `cargo test -p ashiato-server update_history` が落とす）。上流では安く捕まえられない
- 行き先: leave: 走っている change の tasks を書くのは次の spec --answers の段。deep.md の「spec の段へ渡すもの（第 2 回の整合の確認）」に K1 として置いた

## 未確認

- Q9 の ★ が述べる振る舞い（`rolled_back` の行の `applied_versions`・`MIGRATIONS` と戻した DB の版の記録の差から取る当てた版の名前・控えの写し）と Q8（受け手から `specialUse` で起こす）は、まだコードが無いので照合していない。
  Android 14 以降で `ACTION_MY_PACKAGE_REPLACED` の受け手から `specialUse` の前景のサービスを起こせるか（premises P6・P7 は確かさ 2）も確かめていない
- 正典の Scenario のテストの印は全件を機械で見ていない（改訂は正典を変えていない）

## 触らなくてよいと判断したもの

- ★ の前提「`rolled_back` は「前の版」に数えない」: `crates/server/src/update_history.rs:127-136` の `previous_version` は `outcome IN ('applied', 'nothing_to_apply')` の最後の行だけを読み、正典 distribution/spec.md:313 も同じ。`rolled_back` の行を何行足しても前の版は変わらない。`applied_versions` は `migrations/202610300900_update_history.sql:14` に `text[] NOT NULL DEFAULT '{}'` としてあり、列を足さずに持てる
- requirements.md:1067 の ★ 2026-11-15 補足（C2）の「落ちた新しい版と読みうる（IMP-07。本人が受け入れ済み）」: 直後の :1068 の ★ が「起きなくなる」と名指しで覆している。★ を重ねて訂正する書き方で、本文に古い主張は残っていない
- requirements.md:1064 の ★ 2026-10-29「移行が通らなかった更新（FR-119 で退避から戻した）も「通らなかった」として 1 行残し」: 待ち受けを開かずに戻した回は :1067 の補足が「移行が通ったかに依らず `rolled_back`」と受けており、Q9 はその `rolled_back` の行の中身を決めただけ
- `docs/handoff/ST58.md:26`（ST51 R1「「落ちた試みの結果」…と「戻した」の 2 行を書き直す」）: 移行が落ちた回（`failed` / `backup_failed`）の話で、Q6 の形と合う。:29「戻した D-01 の最後の通った行は前の版なので…前の版で起動し直しても行は増えない」は、旧 D26（`applied` を書き直す）とは食い違っていたが、Q9 の形とは合う
- 正典 distribution/spec.md:324「落ちた版の名前と SQLSTATE は結果の行にだけあり、戻すと消える —— 戻した後に書き直すのは ST58（Q2）」: 落ちた版を持つのは移行が落ちた回だけで、待ち受けを開かずに戻した回には落ちた版が無い。Q9 の「書き直さない」と食い違わない
- `docs/handoff/ST51.md`（K1・C2・R14）と `docs/stories/stories.json:1206`（ST51 の satisfies）・:1221（ST51 の done「移行が通らずに退避から戻したときも「通らなかった」として 1 行残り」）: ST51 は archive 済みで、待ち受けを開かずに戻した回（移行は通った）は言っていない
- `docs/stories/INDEX.md:75`・`:506`・`:546`: ST51 の行と 2026-10-29 の経緯、ST58 が data-durability の説明書の 1 文を MODIFIED にする旨。Q9 で直す文が `rolled_back` の説明 1 つのままなので、INDEX の記述は変わらない
- 正典 collection-coverage（:1925・:1969・:1971・:2052）・desktop-collection:1132・device-collection:2534・data-durability:960: FR-121 の ★ 2026-10-25（収集アプリの版の変化を生存信号で知る）を引く導出元で、S-01 の更新の履歴の行の形に触れない。`crates/server` で `core.update_history` を読むのは `previous_version` と書き出し（`export.rs:373-387`）だけで、稼働状況の格子は読まない
- Q8 と要件・正典: requirements.md に裏の位置の権限を必須とする要件は無く、正典 device-collection にも収集のサービスを画面からだけ起こすとする Requirement は無い。`AndroidManifest.xml:117` の `LocationService` はすでに `location|specialUse` を宣言し（:29 `FOREGROUND_SERVICE_SPECIAL_USE`）、:134 の `BOOT_COMPLETED` は気分の通知の受け手だけ（端末の再起動の後に止まる件は deep.md 第 1 回の X1 のまま）
- `docs/briefs/ST58-pr.md:34`（D26（仮）の旧い形）・`docs/briefs/ST58-gate.md`: `.gitignore:34` で追跡しない生成物で、`scripts/pr_body.py` が組み直す
