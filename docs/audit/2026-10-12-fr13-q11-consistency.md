# 整合の確認（2026-10-12・FR-13 の追加（ST08 の深掘り第 6 回 Q11））

- 基準: `09ef3fb9106d1c29703c3970108746285f218b7d`..`393c4c2838ff1708951c0e38b7da1ef2233e600b`
（基準は、もとの `ebe5e85e`..`4a1461e3` を graph の prepare と merge gate のための rebase で origin/main に載せ直したときの、同じ commit。もとの 2 つは rebase で枝から外れ、CI が読めない。`docs/requirements.md` の範囲の差分は載せ直す前と同じ）

対象: `docs/requirements.md` の FR-13 に足した ★ 2026-10-12 追加（Q11）「PC の時計が戻った後に書き換わった題名・滞在時間も、
その訪問について前に送った版より新しい版として届け、取りこぼさない」。本人の決定は `openspec/changes/st08-browser-history/deep.md` の第 6 回。
出所は前回の確認 `docs/audit/2026-10-12-consistency.md` の K2。文書とコードは head（`4a1461e3`）の内容で読んだ。

機械の入力:
- `req_impact.py --base ebe5e85e`: 改訂は FR-13 の 1 件、扉 0。名指しは `docs/collector-contract.md:401`・`docs/handoff/ST25.md:21`・`docs/stories/INDEX.md:31`・
  `stories.json:197`・`docs/ui-direction.md:83`・`openspec/changes/st08-browser-history/`（deep.md・design.md:110・proposal.md:22,96,97・spec.md 8 か所・review 2 本）・
  `openspec/changes/st25-day-timeline/`（spec.md:365 ほか）・`openspec/specs/collection-coverage/spec.md:537`・`openspec/specs/desktop-collection/spec.md:41`
- `check_chain.py`: 未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった: FR-13」で、この確認を求めている当のもの
- `check_scenarios.py`: OK（ST08 の delta は 81 本）。Q11 の振る舞いの Scenario は spec に無く、印も無い（K1）

走っている change の印: **st08-browser-history（この枝。改訂を起こした当の change。tasks.md は凍結）**、st25-day-timeline（上流 merge 済み・凍結。Q11 は効かない —— 末尾）、st21-place-registry（FR-13 と重ならない）。

**前提**: ST08 の spec・design D15・送信契約の履歴の節・コード（`history/contract.rs` / `history/fetch.rs` / `history/ledger.rs`）は Q11 の答えにまだ合っていない。
これは要件の誤りではなく、**ST08 自身の下流の回し直し（このブランチ）が直すもの**。下の C1〜C3・K1 はその直す範囲を名指しするために書く（差し戻しではない）。

## C1. ST08 の spec が「更新時刻には読んだ時刻を載せる」と言い切り、Q11（前に送った版より前にしない）を持たない
- 重大度: 高
- 文書: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md:167
- 要件: FR-13, FR-22
- 根拠: 要件 `docs/requirements.md:224`「**PC の時計が戻った後に書き換わった題名・滞在時間も、その訪問について前に送った版より新しい版として届け、取りこぼさない。**」。
  spec:167-168「THE SYSTEM SHALL ブラウザ履歴の記録の更新時刻として、その内容を読んだ時刻を載せ、それより新しい時刻に読んだ内容を、古い時刻に読んだ内容で書き戻さない。」、
  導出元 spec:173「design D6 / D7 / D8 / D15（**仮**）」に第 6 回 Q11 が無い。Scenario は spec:213「未送信の再送で古い題名が新しい題名を書き戻さない」だけで、
  時計が戻った後の更新が当たることを言う Scenario が無い（`grep -n "Q11" spec.md` 0 件）。deep.md 第 6 回の「効く先」は
  「spec の識別子の Requirement」を挙げ、「何が変わるか」は「時計が戻った後の題名・滞在時間の更新も取り込み口で当たる（`SkippedStale` にならない）ことを Runtime の層で固定する」。
  archive するとこの spec が正典になり、正典には「読んだ時刻を載せる」が残って Q11 が落ちる。正典どおりに作り直すと、戻った後の更新は取り込み口で捨てられる（loss: uncaptured。90 日で手元から消える）
- 直すところ: その文書（ST08 自身の下流の回し直しで、識別子の Requirement の SHALL を「読んだ時刻。ただし前に送った版より前にしない」にし、導出元に第 6 回 Q11 を足し、時計が戻った後の更新が当たる Scenario を 1 本足す）
- 機械: 安く捕まえられる（deep.md の「効く先」に spec と書いた答えの Q 番号が、その spec の導出元に出てこなければ warn。前回の C1 / C2 と同じ型）
- 行き先: fix（ST08 自身の下流の回し直し。このブランチ）

## C2. ST08 の design D15 が「（仮）読んだ時刻を載せる」「本人に返している」のままで、Q11 の答えを受けていない
- 重大度: 中
- 文書: openspec/changes/st08-browser-history/design.md:297
- 要件: FR-13
- 根拠: design:297「### D15.（仮）`source_updated_at` には、その内容を読んだ時刻を載せる」、:300「読んだ時刻は同じ訪問について単調に進む**版**として働く」、
  :309「扱いを本人に返している（deep.md 第 6 回 Q11）」、:311-312 の反転条件「帳面に持つ単調な版番号に替える」、
  Open Questions :332「例外は D15 の時計が戻った後の更新の扱いで、本人に返している」、Context :10「深掘りで本人が決めたこと（第 1 回の 3 件、第 2〜5 回の Q4〜Q10）」。
  deep.md 第 6 回は「回答済み」で、「当初案を覆したもの」に「design D15 の『時計が戻った幅の間の更新は取り込み口で捨てられる』を覆す」とある。
  D15 は（仮）の決めで、Q11 の答え（前の版 + 1 マイクロ秒）は本人が決めた側に移ったので、（仮）と反転条件の扱いも変わる
- 直すところ: その文書（ST08 自身の下流の回し直しで、D15 を Q11 の答えの形に書き直し、Open Questions と Context の深掘りの範囲を第 6 回まで延ばす）
- 機械: 安く捕まえられる（deep.md の節の見出しが「回答済み」なのに、design に「本人に返している」が残っていれば warn）
- 行き先: fix（ST08 自身の下流の回し直し。このブランチ）

## C3. 送信契約が「読んだ時刻は同じ訪問について単調に進む版として働く」と言い、Q11 で事実と違うと分かった守りを書いている
- 重大度: 中
- 文書: docs/collector-contract.md:425
- 要件: FR-13, FR-22
- 根拠: 契約:425-426「履歴は**その内容を読んだ時刻**を `source_updated_at` に置く。履歴 DB は更新時刻を持たないが、読んだ時刻は同じ訪問について単調に進む版として働き、
  未送信の再送で古い題名の到着が新しい題名を書き戻すのを防ぐ」。要件 :225-226「版を読んだ時刻のまま送ると、時計が戻った後の版は前の版より古く見えて捨てられ」。
  この行はこの枝で入った（`git log -S` の最初の commit は main に無い。tasks 3.3）。契約は C-02 の外の読み手（取り込み口・ST25 など）も読む文書で、
  「単調に進む」を前提に版を読むと、時計が戻った後の版の扱いを誤る。deep.md 第 6 回の「効く先」（design D15・spec）にこの文書が入っていない
- 直すところ: その文書（ST08 自身の下流の回し直しで、`source_updated_at` を「読んだ時刻。ただしその訪問について前に送った版より前にしない（前の版 + 1 マイクロ秒）」に書き直し、「単調に進む」を外す）
- 機械: できない（語の一致では「単調」の主張の真偽は取れない）
- 行き先: fix（ST08 自身の下流の回し直し。このブランチ）

## C4. 要件は「取りこぼさない」と言い切るが、Q11 で採った形は「帳面に前の版が無ければ読んだ時刻のまま」で、帳面を退避した後は取りこぼす
- 重大度: 高
- 文書: openspec/changes/st08-browser-history/deep.md:449
- 要件: FR-13
- 根拠: 要件 :224「前に送った版より新しい版として届け、取りこぼさない」（条件を付けていない）。deep.md:449-450「帳面の各訪問に前に送った版（`source_updated_at`）を持ち…
  前の版の帳面に欄が無ければ読んだ時刻のまま」、推奨 :434「（前の版の帳面に無ければ読んだ時刻のまま）」。
  コード: 壊れた帳面は退避して空の帳面から始める（`history/ledger.rs:140` のテスト `history_ledger_broken_is_quarantined`、:146 `ledger().visits.is_empty()`）。
  空の帳面では全訪問が「前の版が無い」になり、読んだ時刻で積まれる。時計が取り込み口の水位（`crates/server/src/lib.rs:1170-1191` の `advance_watermark` が保つ最大）より戻っている間は、
  内容の変わった訪問は `incoming < known` で `SkippedStale`（`lib.rs:1215-1216`）、応答は `accepted: true`（`lib.rs:1131-1133`「どれも `accepted: true` / `duplicate: true` で返る」）で、
  収集側は未送信から取り除く（`sender.rs:153`）。帳面にはそのときの読んだ時刻が前の版として入るので、その後の更新も「前の版 + 1 マイクロ秒」が水位を越えるまで捨てられ続ける。
  時計が戻った幅が 90 日を超えれば、その間の最終の題名・滞在時間は後から取れない（loss: uncaptured）。起きるのは「時計が戻った」と「帳面が壊れた」が重なったときだけ。
  「前の版の帳面に欄が無い」のもう 1 つの場合（Q11 の前の形式の帳面）は、ST08 が main に入っていないので確認の機械の上にしか無い
- 直すところ: その文書（ST08 自身の下流の回し直しで、design D15 にこの場合の扱いを決める。閉じる手が取れれば B（仮）で D 番号に書き、閉じられないなら要件の「取りこぼさない」に例外を足すかを本人に返す）
- 機械: 安く捕まえられる（帳面を退避した直後に、水位より前の読んだ時刻で内容の違う訪問を送り、取り込み口の行が新しい題名を持つかを見る Runtime のテスト）
- 行き先: fix（ST08 自身の下流の回し直しで D15 に扱いを決める。閉じられなければ ask）

## C5. ST08 の proposal が FR-13 の ★ 2026-10-12（2 つ）と第 6 回 Q11 を持たない
- 重大度: 低
- 文書: openspec/changes/st08-browser-history/proposal.md:96
- 要件: FR-13
- 根拠: proposal「### 要件の改訂」:96-98 は「FR-13 に追加（★ 2026-10-05 を 2 回）」までで、★ 2026-10-12 訂正（URL と題名の無い訪問。前回の C6）と ★ 2026-10-12 追加（Q11）が無い。
  「深掘りで本人が決めたこと」の表 :47-55 は「第 3〜5 回（2026-10-05。5 問）」の Q6〜Q10 までで、Q11 の行が無い
- 直すところ: その文書（proposal の要件の改訂と深掘りの表に第 6 回を足す）
- 機械: 安く捕まえられる（requirements.md の項目の ★ の数・最新の日付と、その項目を satisfies に持つ走っている change の proposal に出る日付を比べる。前回の C5 と同じ型）
- 行き先: fix（ST08 自身の下流の回し直しで proposal も直す。このブランチ）

## C6. 凍結した ST08 の tasks.md が「`source_updated_at` は読んだ時刻」を完了の済みとして持ち、Q11 を受ける Task が無い
- 重大度: 低
- 文書: openspec/changes/st08-browser-history/tasks.md:57
- 要件: FR-13
- 根拠: tasks:56-57「- [x] 3.1 … `source_updated_at` は読んだ時刻」、:65「- [x] 3.3 `docs/collector-contract.md` に … `source_updated_at` に読んだ時刻を載せる理由を追記する」。
  どの Task も第 6 回 Q11 を名指ししない（`grep -n "Q11\|第 6 回" tasks.md` 0 件）。Task agent は tasks.md を唯一の入力として回るので、
  Q11 の直しは Task の外（前回の C1 / C2 / K1 / K4 と同じ `fix`）で入れることになる
- 直すところ: leave（tasks.md は凍結。Q11 の直しは spec・design・契約・コードと、その検証で持つ）
- 機械: できない（凍結した文書の古さを止める検査は凍結と両立しない）
- 行き先: leave: tasks.md は issue ができた時点で凍結（CLAUDE.md「走っている Story へは差し戻さない」）。Q11 の振る舞いは C1 の Scenario とその印で機械が見る

## K1. コードは Q11 を持たない —— 帳面に前に送った版が無く、時計が戻った後の更新は取り込み口で捨てられて二度と送られない
- 重大度: 高
- 文書: docs/requirements.md:224
- 要件: FR-13, FR-22
- 根拠: 要件 :224「前に送った版より新しい版として届け、取りこぼさない」。コード: `crates/collector-windows/src/history/contract.rs:330` `source_updated_at: Some(micros(collected_at))`（読んだ時刻そのまま。
  注釈 :310-311「読んだ時刻は同じ訪問について単調に進む版として働き」、`crates/collector-windows/src/contract.rs:315`「取得元の内容を読んだ時刻」）。
  帳面の訪問 `history/ledger.rs:12-21` の `LedgerVisit` は `content_hash` / `at` / `foreign` / `excluded` / `slot` だけで、前に送った版の欄が無い。
  `history/fetch.rs:263-272` `select_new_or_changed` はハッシュが同じ訪問を積まず、`fetch.rs:280` `mark_queued` は積んだ時点でハッシュを記録する。
  取り込み口は `crates/server/src/lib.rs:1215-1216` `if incoming < known { return Ok(Update::SkippedStale); }`、その応答は `accepted: true`（:1131-1133）で、
  `sender.rs:153` が未送信から取り除く。したがって前回の K2 のとおり、時計が戻った後の題名・滞在時間の版は捨てられ、再送されない。
  Q11 の振る舞いのテストも Scenario の印も無い（`grep "時計が戻" history/*.rs runtime.rs` は取得の契機（Q7）の `fetch.rs:231,313` だけ）
- 直すところ: コード（ST08 自身の下流の回し直しで、`LedgerVisit` に前に送った版を足し、`of_visit` に渡す版を「読んだ時刻と前の版 + 1 マイクロ秒の大きいほう」にし、`mark_queued` で版を記録する。注釈 2 か所も直す）
- 機械: 安く捕まえられる（deep.md 第 6 回の「Runtime の層で固定する」—— 先の時刻で 1 回送った後に時計を戻し、題名を変えて取得し、取り込み口の行が新しい題名を持つことを見るテスト。C1 の Scenario の印にする）
- 行き先: fix（ST08 自身の下流の回し直し。このブランチ）

## 未確認

- Q11 の形（前の版 + 1 マイクロ秒）で、時計が戻った後の版が実際に取り込み口で `Applied` になること。取り込み口は `incoming < known` だけで止め、同じ時刻は `>=` で当てる（`lib.rs:1212-1213`）ので通るはずだが、
  コードがまだ無いので走らせていない
- 時計が戻った幅と、取り込み口の水位の関係を実機で測っていない（どれだけ先へ進んだ時計で送った版が現に D-01 にあるか）。C4 の起きやすさはこれで決まる
- ST05（端末時計のずれを測って残す。main に入っている）の PC 側の測定が、Q11 の版の決め方に使えるか（たとえば戻った幅の検出）。見ていない
- Windows の実行時テスト（`tests/runtime_windows.rs`）が時計を戻す形を持てるか。手元の Windows では走らせていない

## 触らなくてよいと判断したもの

- `docs/stories/stories.json:197`・`docs/stories/INDEX.md:31`・`docs/stories/ST08.md`: ST08 の価値・完了の判定（「前日に見たページが翌日の取得で入っている」「2 回続けて取得しても行が増えない」）は Q11 の後も成り立つ。
  ST08.md の要件の引用は FR-13 の冒頭 6 行で切ってあり（「…（続きは requirements.md）」）、`check_chain.py` の再生成の一致も通っている
- `openspec/specs/record-envelope/spec.md:291`「外部サービス側の更新時刻（または版）を記録に持たせる」: Q11 の値（読んだ時刻と前の版 + 1 マイクロ秒の大きいほう）も「版」に当たり、正典の言葉と食い違わない
- 取り込み口（`crates/server/src/lib.rs` の `apply_external_update` / `advance_watermark`）: Q11 は「取り込み口・契約の形は変えない」（deep.md:434）で、水位を最大で保つ現行の振る舞いのまま当たる
- st25-day-timeline（凍結）と `docs/handoff/ST25.md`: ST25 の「最後に届いた時刻」は `ingest_time` の最大（ST25 design D10:175-178）で `source_updated_at` を読まない。Q11 で版の値が実時刻より先になることがあっても、ST25 には効かない。handoff ST25 の内容（URL の無い訪問）は前回の C3 で、Q11 とは別
- `docs/ui-direction.md:83`・`openspec/specs/desktop-collection/spec.md:41`・`openspec/specs/collection-coverage/spec.md:537`: FR-13 を件数の目安・例として引くだけで、版の扱いに触れない
- `docs/briefs/ST08-pr.md:17`・`docs/briefs/ST08-issue.md:39`（D15（仮）を挙げる）: 生成物で commit されていない（`git ls-files docs/briefs` に無い）。PR 本文は下流の回し直しの後に作り直される
- deep.md:8「3 問とも推奨と同じ答え。」: 第 1 回の時点の文で、この改訂（第 6 回の記録）で増えた古さではない
- st08 の `review/deep.md`・`review/spec.md`・`deep-questions*.json`: 経緯の記録で、FR-13 を名指しするのは当時の文言の引用
- st21-place-registry: FR-13 も `c02-browser-history` も扱わない
- 語の揺れ: 改訂は概念の名前を変えていない（「版」「前に送った版」「帳面」は deep.md 第 6 回・design D15 と同じ語）
