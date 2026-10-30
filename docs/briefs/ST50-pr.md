# ST50 収集アプリとサーバの版が食い違っても記録が失われない

change: `openspec/changes/st50-contract-version/`。満たす要件は FR-124・FR-125・FR-126・FR-130。`requires: [ST04]`。

送信契約の版（`schema_version` とは別の整数。今の封筒の全部が版 1）を入れた。
サーバは `GET /contract` で受け付ける版を答え、収集アプリの要求の見出し `Ashiato-Contract-Version` が新しすぎれば 426 で断り、知らない欄を持つ 1 件は `unknown_field` で断る（どちらも収集アプリは捨てない）。
収集アプリ 2 本は、積む時に行ごとに版を焼き込み、送る契機の頭で確かめて、受け付けられる版の行だけを送る。それ以外の行は手元に残し、「サーバを更新してください」を出す（携帯は画面と通知、PC は通知）。
記録の表・前の版の表・書き出しに `contract_version` が残る。版の上げ忘れは `tools/check-contract-version.sh` が CI で落とす。

## 深掘りに積んだ未決（A）

なし（`deep.md` に `(未回答)` は 0 件）。第 1 回 Q1〜Q9・第 2 回 Q10 とも本人が答えた。

## 仮決め（B）と Ruling

**仮決め**（design の（仮）と反転条件。`provisional.jsonl`）:
- **D13（仮）** PC の未送信には上限を足さない（FR-125 の上限は C-01 に掛かる）
- **D18（仮）** 確かめは送る契機ごとに 1 回で、答えは覚えない。反転条件: 問い合わせの数が S-01 のログや携帯の電池で目に見える量になったら、答えを 1 時間覚える
- **D19（仮）** 外部サービス由来の更新では、契約の版も新しい中身の 1 件の版に動く
- **D23（仮）** `GET /contract` は今の規則のまま読み出しの記録に残す（携帯と PC で 1 日 576 行）
- **D24（仮）** 読めない 200 が 24 時間続いたら、「S-01 の更新を待っている」状態そのものに入る
- **D25（仮）**（finish で足した。code-verify R18）読めない 200 の数え始めが今より後（時計が戻った）なら今へ置き直す。置き直さないと、何日経っても促されないまま携帯が 90 日で捨てる。反転条件: 時計の戻りで促しが 24 時間遅れることが問題になったら `AgeClock` で数える。**別の席の確かめ（`provisional-review.md` の D25）はまだ無い** —— グラフの `provisional_review` が書く（このため 12.2 の `provisional.py check` は今は rc=1）
- D22 は仮ではなくなった（別の席が否 → 第 2 回 Q10 で本人が「24 時間続いたら促す」と決めた）

**Ruling**（全 Task の ledger の `Ruling:` 行。逐語）:
- Task 4: `Ruling: ST41 の移行（202610231200_event_end_time.sql）の書き換えを残す — 全版を 2 回流す試験が 10 本超（clock/end_time/feelings/media/satellite/export_excluded ほか）と check-immutable の 2 周目が前提で、ST50 が event_version_live に列を足し門の本体を変える以上、ST41 の再適用は md5 の受け入れと DROP VIEW 無しには落ちる。design D20 が「ST41 の md5 を確かめてから置き換える形に倣う」と書き、後から main に入る側が前の側を写す運用を採っている。試験側の当て直しを台帳経由へ変える案は影響が全試験に及び Task 4 の範囲を越える — 間違っていたときの費用: ST41 を当て済みの DB で起動のたびに「記録した指紋と中身が違う版がある（当て直さない）」の警告が出る（動作は変わらない。配った v* 印が無い）。PR 本文に明記する。`
- Task 4: `parked — F1 ST41 の移行（202610231200_event_end_time.sql）の書き換え — Ruling: 本人が受け入れた（2026-10-30）。これから作る DB には影響が無く、ST41 を当て済みの既存の DB で起動のたびに指紋の警告が出るだけ（動作は変わらない・配った v* 印が無い）。PR 本文に明記する。main に入った ST49 の移行にも同じ形が要る`
- final review R9: `parked — 携帯は待っている間、契機ごとに未送信を 1 回全走査する（76744589 で 2 回目は省いた）— Ruling: 費用は未送信の上限（90 日）・5 分の契機・S-01 が古い間だけに限られる。SegmentStore に区切りごとの版の最大を持たせる直しは dropHead / dropUntilBytes / remove の不変条件に触るので fix wave で入れない。誤りだった場合の代償: 古い S-01 の後ろで何週間も止まった携帯の電池`
- final re-review Minor: `parked — 携帯の nextBatch は A が下がって probe の版が upTo を超えると probe を未送信に無いと見て手放す — Ruling: D9 は受け付ける版が下がらない前提（S-01 を古い版へ戻した時だけ）。誤りだった場合の代償: S-01 を戻した間に版で断られた行の印が 1 つ早く外れ、待ちを 1 契機早く抜ける`
- finish の scoped re-review Minor: `parked — PC の send_held は 401 も到達できないのも error=transient で、ログで見分けられない — Ruling: 携帯の send_held と同じ粒度（D11 どおり 401・403・5xx・到達できないを一時的な失敗にまとめる）。分けるには答えの型に理由を持たせ、PC・携帯の両方と D11 を変える。誤りだった場合の代償: 合言葉の誤りに気づくのが、ログでなく稼働状況の途絶からになる`

ほかの Task（1〜3・5〜12）の ledger に `Ruling:` は無い。

**確認バッチで見てほしいこと**: ST41・ST49 の移行の門の本体を写し直したため、ST50 より前に ST41・ST49 を当てた DB（確認バッチの DB を含む）は `migrate` のたびに
`記録した指紋と中身が違う版がある（当て直さない）` の警告を出し続ける（データは無害。消すには `STACK_RESET=1` で作り直す）。

## レビューの処置（`review/code.md`）

- final review: 16 件すべて fixed（R1〜R16）。scoped re-review で全件 ADDRESSED
- final の後の 3 席（13 件）:
  - code-verify: R17・R19 は fixed、R18 は D25（仮）で fixed、R20 は rejected（9.2 の「本物の Windows の CI が緑」は merge gate が head の CI で見る。`collector-windows-runtime` は `crates/` の変更で必ず走る）
  - security review: 指摘なし
  - `/code-review`: R21・R22・R25・R28・R29 は fixed、R23・R24・R26・R27 は rejected（R23 は park 済みの Minor と同じ、R24 は D18（仮）の費用、R26 は移行が起動の外で当たる、R27 は網羅的な match で型の検査がずれを止める）
- finish の fix（b65703b3）の scoped re-review: 8 件すべて ADDRESSED・新しい Critical / Important なし

## Task

12 Task・18 項目すべて `[x]`。9.2 の終了条件（CI の `collector-windows-runtime` が緑）は、この PR の CI で満たされる。

## 通した検証（`evidence.jsonl`。各項目の最後の実行）

| 項目 | コマンド | 終了コード |
|---|---|---|
| 1.1 | `cargo test -p ashiato-server st50_contract_` / `access_log` / `./tools/check-openapi.sh` | 0 / 0 / 0 |
| 2.1 | `cargo test -p ashiato-server st50_header_` | 0 |
| 3.1 | `cargo test -p ashiato-server st50_unknown_field_` / `st50_item_version_` / `./tools/check-openapi.sh` | 0 / 0 / 0 |
| 3.2 | `docs/collector-contract.md` の grep | 0 |
| 4.1 | `./tools/check-migrations.sh` / `cargo build -p ashiato-server` | 0 / 0 |
| 4.2 | `cargo test -p ashiato-server st50_record_version_` / `hash_is_pinned` | 0 / 0 |
| 4.3 | `cargo test -p ashiato-server st50_migration_` / `st50_lock_` / `tools/check-immutable.sh` | 0 / 0 / 0 |
| 5.1 | `cargo test -p ashiato-server st50_released_` | 0 |
| 6.1 | `cargo test -p ashiato-server st50_export_` / `export_` | 0 / 0 |
| 7.2 | `tools/check-contract-version.sh --self-test` / 本体 / `St50EnvelopeShapeTest` / `--lib st50_envelope_shape` | 0（4 本とも） |
| 8.1 | `cargo test -p ashiato-collector-windows --lib st50_sender_` / `sender::` / `check-contract-version.sh` / `st50_envelope_shape` | 0（4 本とも） |
| 9.1 | `cargo test -p ashiato-collector-windows --lib st50_notice_` / `cargo clippy -p ashiato-collector-windows --all-targets -- -D warnings` | 0 / 0 |
| 9.2 | `runtime_windows.rs` の grep（終了条件は CI） | 0 |
| 10.1 | `St50SenderTest` / `OriginalUploaderTest` / `SenderTest` ほか / `check-contract-version.sh` | 0（5 本とも） |
| 11.1 | `St50ContractNoticeTest` / `RetentionNotifierTest` / `MainActivityTest` / detekt | 0（4 本とも） |
| 12.1 | `tools/smoke.sh` | 0 |
| 12.2 | `check_scenarios.py` / `provisional.py check` | 0 / **1**（D25 の別の席の確かめ待ち。グラフの `provisional_review` が書く） |

finish の fix の後（b65703b3）に、全体も手元で通した: `cargo test -p ashiato-server` 1775 passed・3 ignored /
`cargo test -p ashiato-collector-windows --lib` 324 passed / `:app:testDebugUnitTest` 744 本 failures 0 /
`cargo clippy -p ashiato-collector-windows -p ashiato-server --all-targets -- -D warnings` / `cargo fmt --all -- --check` / detekt（どれも rc=0）。

## 未処置の申し送り

なし。ST49 からの申し送り（`docs/handoff/ST50.md`。版の脱落する古い組み合わせを止める）は、この Story の送信契約の版（FR-125 / FR-130）で受けた。
