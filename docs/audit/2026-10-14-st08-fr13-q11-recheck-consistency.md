# 整合の確認（2026-10-14・FR-13 の ★ 2026-10-12 追加（ST08 の深掘り第 6 回 Q11）を、Q11 を実装した木で見直す）

- 基準: `e32f33a926d9fe0b1fc66697c33bce7b4f0bc0cc`..`1244318aedcd1044c3e67c92985f63e88774a3f0`

対象: 前回の確認 `docs/audit/2026-10-12-fr13-q11-consistency.md`（指摘 7 件）が名指した、ST08 自身の下流の回し直しで直す範囲（C1〜C5・K1）と、
触らないとした C6。base は `git merge-base origin/main HEAD`、head は Q11 を実装して文書を合わせた commit（`9f0e4f37` / `df3fe8c5` / `714c626e` / `1244318a`）の最後。
文書とコードは head の内容で読んだ。`docs/requirements.md` は base..head で変わっていない（FR-13 の ★ 2026-10-12 追加は前回の確認の対象のまま）。

出所: whole-branch review `openspec/changes/st08-browser-history/review/code.md` の R77（Q11 が未実装）・R78（`check_chain.py` が「整合の確認の後に要件が変わった: FR-13」で FAIL）。
rebase で入った main の `2026-10-14-st14-merge-consistency.md` は基準の head が ST08 の FR-13 ★ 2026-10-12 を持たないので、この確認が ST08 の木での最新になる。

機械の入力:
- `cargo test -p ashiato-collector-windows`: 261 本 OK。Q11 の振る舞いは `runtime::tests::history_runtime_clock_back_update_has_newer_version`、
  C4 は `runtime::tests::history_runtime_clock_back_update_after_quarantine_has_newer_version`（どちらも Runtime の層。送る本体の `source_updated_at` を比べる）
- `check_scenarios.py . st08-browser-history`: OK（spec に足した Scenario 2 本に印がある）
- `openspec validate st08-browser-history --strict`: valid

## 前回の指摘がどう閉じたか

- **C1（spec が「読んだ時刻を載せる」と言い切る）**: 閉じた。`openspec/changes/st08-browser-history/specs/desktop-collection/spec.md:169-170` に
  「THE SYSTEM SHALL その更新時刻を、その訪問について前に送った版より前にしない（PC の時計が戻った後に読んだ内容も、手元の帳面を作り直した後も、前に送った版より新しい版として届ける）」、
  導出元 :174-175 に第 6 回 Q11 と D15 / D16（仮）、Scenario :222「時計が戻った後に書き換わった題名も、前に送った版より新しい版で届く」と
  :227「帳面を作り直した後も、時計が戻った後の更新は前に送った版より新しい版で届く」。印は `crates/collector-windows/src/runtime.rs:2423` / `:2435`
- **C2（design D15 が「（仮）」「本人に返している」のまま）**: 閉じた。`design.md:297` の D15 は「（仮）」を外し、版 = max(読んだ時刻, 前に送った版 + 1 マイクロ秒) を
  本人の決定（第 6 回 Q11）として書く。Context :10 は「第 2〜6 回の Q4〜Q11」、Open Questions :359-361 は「第 6 回 Q11 で本人が決めた」。`grep -n "本人に返している" design.md` は 0 件
- **C3（送信契約の「単調に進む版」）**: 閉じた。`docs/collector-contract.md:425-429` は「読んだ時刻。ただし、その訪問について前に送った版より前にしない（前の版 + 1 マイクロ秒）」と、
  「読んだ時刻そのものではない —— 時計が戻った後は読んだ時刻より先の値になる」「読む側はこの値を『いつ読んだか』として使わない（届いた時刻は `ingest_time`）」。
  契約の履歴の節に「単調に進む」は残っていない
- **C4（帳面を退避した後は取りこぼす）**: 名指しの場合（帳面の退避・帳面が無い）は閉じた。`design.md:319` の D16（仮）—— 送った版の最大値を帳面とは別の
  `browser-history/sent_version.json` に、積む前に書き、版をそれより前にしない（`crates/collector-windows/src/history/collect.rs:639` `Planner::version`）。
  退避は `*.ledger` を `*.broken.ledger` に動かすだけで、このファイルは残る。テストは帳面を `{broken` に壊してから時計を戻し、戻った後の版が前に送った版より新しいことを見る。
  残るもの（置き場ごと消えた後）は下の C1
- **C5（proposal が第 6 回と ★ 2026-10-12 を持たない）**: 閉じた。`proposal.md:59-63` に第 6 回（Q11）の表、`:105-106` に FR-13 の ★ 2026-10-12 訂正（C6）と追加（Q11）。
  あわせて第 2 回の Q4 / Q5 の行を第 1 回の表（「2 巡・5 問」）へ戻した（review/code.md R81）
- **K1（コードは Q11 を持たない）**: 閉じた。`history/ledger.rs:24` `LedgerVisit::sent_version`（`#[serde(default)]`。前の形式の帳面を読むテスト
  `history_ledger_reads_format_without_sent_version`）、`history/fetch.rs:297` `next_version`（読んだ時刻をマイクロ秒に丸め、前に送った版 + 1 マイクロ秒より前にしない）、
  `:308` `mark_sent_version`、`history/contract.rs:331` `source_updated_at: Some(micros(version))`。注釈（`history/contract.rs:310-312`・`contract.rs:315`）も直した。
  取り込み口は `incoming < known` だけを捨てる（`crates/server/src/lib.rs:1529`）ので、前の版 + 1 マイクロ秒は当たる
- **C6（凍結した tasks.md の「読んだ時刻」）**: 前回どおり触らない（下の C2）

## C1. 置き場（`state_dir`）ごと消えた・入れ直した後に時計が戻っていると、D16 でも取りこぼす
- 重大度: 低
- 文書: openspec/changes/st08-browser-history/design.md:334
- 要件: FR-13
- 根拠: 要件 `docs/requirements.md:225`「前に送った版より新しい版として届け、取りこぼさない」（条件を付けていない）。design D16 :334-335「**閉じないもの**: 置き場（`state_dir`）ごと消えた・入れ直した後に
  時計が戻っていると、最大値も帳面も無いので D15 の前と同じく捨てられうる。取り込み口の水位を収集側が知る経路は無い」。
  起きるのは「置き場が消えた」と「取り込み口の水位より時計が戻っている」が重なったときだけ。置き場が消えると未送信（outbox）も消えるので、
  この Story の外でも同じときに失うものがある。閉じる手は取り込み口から訪問ごとの版を引くこと（D16 の反転条件 (2)）で、口が増える
- 直すところ: 本人に返す（要件の「取りこぼさない」に置き場ごと消えた場合の例外を足すか、取り込み口から版を引くか）
- 機械: できない（置き場が消えた後の取り込み口の水位は収集側から見えない）
- 行き先: ask

## C2. 凍結した ST08 の tasks.md が「`source_updated_at` は読んだ時刻」「D15 は（仮）決め」を持つ
- 重大度: 低
- 文書: openspec/changes/st08-browser-history/tasks.md:23
- 要件: FR-13
- 根拠: tasks:23「**D3 / D10 / D15 は（仮）決め**を含む」、:65「`source_updated_at` に読んだ時刻を載せる理由を追記する」。D15 は第 6 回 Q11 で本人の決定になり、
  契約は「読んだ時刻。ただし前に送った版より前にしない」に変わった。前回の C6 と同じ
- 直すところ: leave（tasks.md は凍結。Q11 の振る舞いは spec の Scenario 2 本とその印で機械が見る）
- 機械: できない（凍結した文書の古さを止める検査は凍結と両立しない）
- 行き先: leave: tasks.md は issue ができた時点で凍結（CLAUDE.md「走っている Story へは差し戻さない」）

## 未確認

- 本物の取り込み口（`crates/server`）に、時計が戻った後の版を通して `Applied` になることは走らせていない。Runtime の層のテストは送る本体の `source_updated_at` の大小を見る。
  取り込み口の比較は `incoming < known` で、同じ時刻は当てる側（`lib.rs:1529`）
- Windows の実行時テスト（`tests/runtime_windows.rs`）には足していない（時計を戻す形は Linux の Runtime の層で持つ）

## 触らなくてよいと判断したもの

- `docs/requirements.md`: FR-13 の ★ 2026-10-12 追加の文言（「前に送った版より新しい版として届け、取りこぼさない」）はコード・spec・契約と合う。base..head で変わっていない
- st25-day-timeline（凍結）と `docs/handoff/ST25.md`: ST25 の「最後に届いた時刻」は `ingest_time` の最大で `source_updated_at` を読まない（前回と同じ）。
  D16 で版が読んだ時刻より先になっても効かない。契約 :428-429 に「読む側はこの値を『いつ読んだか』として使わない」を書いた
- `openspec/specs/record-envelope/spec.md:291`「外部サービス側の更新時刻（または版）」: Q11 / D16 の値も「版」に当たる（前回と同じ）
