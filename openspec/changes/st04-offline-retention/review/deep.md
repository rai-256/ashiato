# ST04 深掘りの独立レビュー

対象: `openspec/changes/st04-offline-retention/deep-questions.json`（Q1〜Q5。A 3 / B 2）と、
その下書き `deep.md`（C1〜C7・「確かめたが問わなかったこと」）、Q3 の `proto.html`。
schema の `deep` 手順 1〜5 を、一覧と下書きを開く前に独立にやり直し、突き合わせた差分だけを書く。
**問いの JSON は触っていない。**

実施日: 2026-09-14。ブランチ `docs/st04-upstream`（clean）。

## 実測・確認の方法

| | やったこと | 結果 |
|---|---|---|
| 実験 1 | `coverage_span_range` と同じ CHECK を持つ一時表（`ashiato2-db-1` の TEMP TABLE、`ROLLBACK`）に `started_at = ended_at` の行を入れた | `ERROR: new row ... violates check constraint "coverage_span_range"` |
| 実験 2 | `LocationFix.toIngestRequest` と同じ欄で JSONL 1 行を組み立て、バイト数を数えた（python） | 1 行 724 B / 1 日 約 1.04 MB / 90 日 約 94 MB（129,600 件）/ 2 GB は約 1,918 日 |
| grep | `crates/server/src/lib.rs:1177-1182` の route | `/ingest` `/heartbeat` `/events` `/coverage` `/coverage/achievement` のみ。破棄を受ける口は無い |
| grep | `migrations/*.sql` の `coverage_span` | 表の定義（`202609111111_coverage_rebuild.sql:91-103`）だけ。トリガ・一意索引・`device_id`・受信時刻・原文の列は無い |
| grep | `web/src/coverage.ts` の `DayCell` | `event_count`（記録の件数）だけ。破棄の件数を運ぶ欄は無い |

## 手順ごとの記録（一覧を見る前）

- **手順 1（要件どうしの衝突）**: FR-8 / FR-9 / NFR-7 と扉 #14 の関与要件（FR-33, FR-34, FR-54, FR-35, FR-61, FR-80, FR-81, FR-82）に、
  FR-10 / FR-78 / NFR-1 / NFR-13 / FR-24 を突き合わせた。
  「90 日**かつ** 2 GB」と「90 日**または** 2 GB を超える」は同じこと（衝突なし。下書きと一致）。
  見つけた衝突: FR-8「到達できない間」× FR-10 の一時的な失敗（→ Q1 にある）、
  FR-8 が圏外を「保持して後で送る」正常な状態にしたこと × FR-78 / NFR-13 の「接続」を含む取得可否（→ **R10、一覧に無い**）、
  NFR-1（到達できる間は 1 時間以内）× 復旧後の溜まった分の送り方（→ C6 にあるが分類違い。R6）
- **手順 2（扉の幅）**: 扉 #14「バッファから破棄されたのか」。この Story の文脈で残る幅は
  (a) 丸ごと覆わない破棄の見え方（→ Q3）、(b) 破棄の担い手が 1 本の範囲で足りるか（→ R3 / R5）、
  (c) 上限による破棄以外の消え方（壊れた行・書けなかった記録）を同じ区別に入れるか（→ C5 と **R8**）、
  (d) 端末が 2 台あるとき、どの端末の破棄か（→ **R1 / R11**）
- **手順 3（新たに立つ一方通行）**: 捨てること（FR-9 が決定済）。そのうえで、
  **捨てた時点でしか取れない事実**（端末・理由・日ごとの件数）、**破棄の報告の鍵の入力**、
  **範囲の表し方（列の型と CHECK）**、捨てる判定に使う時計（→ R1 / R2 / R3 / R4 / R9）
- **手順 4（日常）**: 端末の容量（位置だけなら 90 日で約 94 MB、2 GB には約 5 年）、
  復旧後の送信の電池と通信量（→ R6）、捨てる前の通知（→ Q5）、稼働状況の画面（→ Q3）
- **手順 5（既存コードと要件）**: 上限も破棄も無い（`Outbox.kt:5`、`outbox.rs:4`）。
  破棄を受ける口が無い（grep）。1 件の破棄は表の CHECK が拒む（実験 1。→ R4）。
  件数は画面にも応答にも無い（grep。→ Q3 にある）。Android の置き場は壊れた行を次の書き直しで黙って消す（→ R8）。
  90 日ぶんを溜めた置き場の読み戻しは全件をメモリに載せる（→ R13）

---

## R1. 破棄の報告に「どの端末か・いつ届いたか・原文・なぜ捨てたか」を持つかが、問いにも C にも無い
- 成果物: openspec/changes/st04-offline-retention/deep-questions.json
- 根拠: migrations/202609111111_coverage_rebuild.sql:91-101（`core.coverage_span` の列は id / user_id / logical_source / kind / started_at / ended_at / event_count / note だけ。`device_id`・受信時刻・原文・理由の列が無い）/ `grep -n coverage_span migrations/*.sql` でトリガも一意索引も無い / crates/server/src/coverage.rs:611-616（破棄は `logical_source` だけで引かれ、端末で分けない）/ docs/requirements.md:853（扉 #13 provenance）/ docs/requirements.md:283-288（FR-78: 生存信号は「証拠」として書き換え禁止・冪等キー・原文の素通し・受信時刻を時刻で残す）
- kind: irreversible
- loss: uncaptured
- 提案: 破棄の報告は扉 #14 の「バッファから破棄された」の唯一の証拠で、生存信号（FR-78）と同じ理由で保護が要る。報告の列（端末識別子・受信時刻・原文・理由 = 90 日 / 2 GB / 書けなかった / 壊れていた）を C として明記するか、取捨に幅があるなら A の問いに立てる。**捨てた時点を過ぎると端末にも残らない**ので後から足せない。
- 処置: escalated — 問いにはしない（持つ側は何も失わず費用が小さい = 扉を開けたままにする既定）。deep.md の C8 に R1 つきで記録し、本人には「聞かないで決めるもの」として HTML の外（報告と PR 本文）で見せ、異論は番号で受ける

## R2. 破棄の報告の冪等キーの入力が決まっておらず、C3「範囲を伸ばす」と C2「再送しても 1 件」が噛み合わない
- 成果物: openspec/changes/st04-offline-retention/deep.md
- 根拠: deep.md の C2（冪等）と C3（連続して押し出した分は 1 つの範囲に伸ばす）/ crates/server/src/heartbeat.rs:70-80（生存信号の鍵は `logical_source` + `emitted_at` + `raw`）/ migrations/202609111111_coverage_rebuild.sql:92（`coverage_span` の主キーは `id uuid` だけ）/ collector-android/.../Sender.kt:148-151（200 / 400 以外は一時的な失敗で未送信に残す＝再送は常態）
- kind: irreversible
- loss: rewrite-all
- 提案: 範囲・件数を鍵に入れると、報告を送った後に範囲が伸びるたびに**別の行**になり件数が二重に数えられる。鍵の入力（例: ソース + 端末 + 破棄した最初と最後の記録の id、または報告ごとの不変の id）と「伸ばす」を「不変の行を足す」に直すかを C に明記する。鍵は凍結した行に焼き込まれるので、後から変えると全行の計算し直しになる。
- 処置: escalated — 問いにはしない。deep.md の C3 を「不変の行を足す。送り始めた報告は書き換えない」に直した（台帳は追記のみ）。本人へは C の一覧で見せる

## R3. Q3 の proto が見せる「日ごとの破棄件数」は、C3 の形（範囲 1 本 + 総件数）からは作れない
- 成果物: openspec/changes/st04-offline-retention/proto.html
- 根拠: proto.html:188（`count:(to-from)*src.perMin` —— 範囲の分数に「1 分 1 件」を掛けて日ごとの件数を作っている）/ openspec/changes/archive/2026-09-10-st01-location-ingest/deep.md:224（実測の取得率 67 %、Doze の空き 14 分〜）/ deep.md の C3「細かい粒度の事実は件数として持つ」（総件数 1 つ）
- kind: irreversible
- loss: uncaptured
- 提案: 取得は一様でないので、範囲と総件数からは日ごとの件数を割り戻せない。Q3 で「週の詳細に『うち N 件を破棄』」を選べる以上、端末が**捨てる時点で**日（`Asia/Tokyo`）ごとの件数を報告に持つことを C に足す（扉を開けたままにする既定「細かい粒度で持つ」）。proto の注記にも「件数は報告が持つ値」と書く。
- 処置: escalated — 問いにはしない。deep.md の C12（端末が捨てる時点で UTC の 1 時間ごとに数える）に足し、Q3 の context と proto.html に「件数は報告が持つ値」と書いた。本人へは C の一覧で見せる

## R4. 1 件だけ（または同じ時刻の記録だけ）を捨てた報告は、既存の表の CHECK に拒まれ、未送信に居座る
- 成果物: openspec/changes/st04-offline-retention/deep.md
- 根拠: migrations/202609111111_coverage_rebuild.sql:100（`CHECK (ended_at IS NULL OR started_at < ended_at)`）/ 実験 1（`started_at = ended_at` で `violates check constraint "coverage_span_range"`）/ crates/server/src/lib.rs:321-323（DB の制約に任せると 500、500 はまとめ送り全体を落とす）/ Sender.kt:148-151（500 は一時的な失敗として 1 件も取り除かない）/ deep.md の C2（報告は送れるまで捨てない）
- kind: technical
- 提案: 範囲を「捨てた最初と最後の記録の出来事の時刻」で表すと、1 件の破棄（完了の判定 2 で最初に起きる形）が 500 になり、C2 により永久に再送される。範囲の表し方（半開区間にして終わりを最後の記録の直後にする等）と受け口での検査を C に足す。
- 処置: fixed deep.md — C3 に「範囲は半開区間で持つ」を足した。design の D 番号と、1 件の破棄を受け口が 500 にしない試験に落とす

## R5. C3 で範囲を 1 本に伸ばしても、報告を送った後に続く破棄で範囲が割れ、日を丸ごと覆わなくなる
- 成果物: openspec/changes/st04-offline-retention/deep.md
- 根拠: crates/server/src/coverage.rs:611-616（`EXISTS (SELECT 1 FROM core.coverage_span s ... started_at <= 日の始まり AND ended_at >= 翌日の始まり)` —— **1 本の範囲**が丸ごと覆うかだけを見る）/ deep.md の C3 の理由（1 分ごとの範囲だと⑤が立たない）/ Q1 の選択肢 1（到達できても送れない間も捨てる＝報告が途中で届きうる）
- kind: technical
- 提案: 端末が報告を 1 回送った後も押し出しが続けば、2 本目の範囲が日の途中から始まり、C3 の目的（⑤を立てる）が崩れる。「隣接・重なる破棄の範囲を合わせてから丸ごと判定する」を導出の規則として design に仮で置く（範囲が残っていれば引き直せる）。
- 処置: fixed deep.md — C13（受け手が隣り合う・重なる破棄の範囲を合わせてから丸ごと判定する。導出の規則なので仮）に足した。design で（仮）と反転条件を付ける

## R6. C6「復旧後に溜まった分を続けて送る」は C ではない —— 本人が決めた送信の間隔を覆し、電池と通信量に効く
- 成果物: openspec/changes/st04-offline-retention/deep.md
- 根拠: openspec/changes/archive/2026-09-10-st01-location-ingest/design.md:126-132（D9「送信は 5 分間隔でまとめる（**深掘りで本人が決定**）」、理由は電池と通信量）/ collector-android/.../LocationService.kt:148（「design D9 が決めた 5 分。本人が決めた値なので、ここをリテラルに書き換えない」）/ 実験 2（90 日ぶん約 94 MB、5 分 200 件だと純減 195 件で約 55 時間）
- kind: daily
- 提案: C6 を B の問い（kind: daily、推奨つき）に上げ、「D9 を覆す」ことを明示する。選択肢の例: 続けて送る / Wi-Fi のときだけ続けて送る / 新しい記録を先に送り溜まった分は 5 分ごとのまま。どれも記録は失われない（NFR-1 を超える時間と、モバイル回線の通信量が変わる）。
- 処置: escalated — Q6 として問いに足した（kind: daily、推奨は「溜まっているうちは続けて送る」）。当初の C6 は削除し、ST01 の D9 を覆すことを why に明記した

## R7. Q1 の選択肢 2 の不可逆の記述「稼働記録にも残らない」が C5 と矛盾し、C5 は空きが尽きたときの報告の置き場を書いていない
- 成果物: openspec/changes/st04-offline-retention/deep-questions.json
- 根拠: deep-questions.json Q1 の選択肢 2 `irreversible`「端末の空きが尽きた後に生まれる新しい記録（稼働記録にも残らない）」/ deep.md の C5「書けなかった記録も、失ったことを範囲と件数で残す」/ collector-android/.../Outbox.kt:20-24（書けなくてもメモリには積み、失敗を返すだけ）/ OutboxStore.kt:147-153（追記の失敗はログ 1 行）
- kind: irreversible
- loss: discarded
- 提案: C5 を採るなら選択肢 2 の記述は誤り、C5 が空きの尽きた端末で成り立たないなら C5 が誤り。どちらかに揃える。C5 には「報告そのものが書けないとき」（報告用に空きを予約する / メモリだけに持ち立て直しで消える、のどちらか）を書き、選択肢 2 の不可逆の記述をそれに合わせる。
- 処置: escalated — Q1 の選択肢 2 の irreversible を「空きが完全に無いと、失ったことの報告も書けず、立て直しで消えることがある」に直し、deep.md の C5 に「数えは小さな固定の置き場に先に書く / そこにも書けなければメモリに持つ」を書き足して揃えた

## R8. Android の置き場は壊れた行を次の書き直しで黙って消し、読めずに退避したファイルは送られも数えられもしない —— 下書きの「確かめたこと」に無い
- 成果物: openspec/changes/st04-offline-retention/deep.md
- 根拠: collector-android/.../OutboxStore.kt:95-106（壊れた行は `broken++` とログだけで読み飛ばし、読めた分だけを返す）→ Outbox.kt:43（次の `remove` が `store.save(pending)` で全件を書き直し、壊れた行はファイルから消える）/ OutboxStore.kt:84・:131-146（全行が読めないときは `.unreadable.<時刻>` へ退避し、以後読まれない）/ 対比: crates/collector-windows/src/outbox.rs:53-67（PC 側は `broken.jsonl` へ追記で退避）
- kind: irreversible
- loss: discarded
- 提案: 上限による破棄だけを稼働記録に残すと、同じ「端末で失われた」がこの 2 経路では格子に出ず、扉 #14 の区別が欠ける。C に「壊れた行・退避したファイルは捨てずに退避し、件数を理由つきで破棄の報告に載せる。退避分を 2 GB に数えるか」を足す（扉を開けたままにする既定「捨てるより印を付けて入れる」）。
- 処置: escalated — 問いにはしない（捨てるより印を付けて入れる）。deep.md の C9 に足した。本人へは C の一覧で見せる

## R9. 端末の時計が先へ飛ぶと 90 日の判定が溜まった分を一度に捨てる経路が、Q4 のどちらの選択肢にも残る
- 成果物: openspec/changes/st04-offline-retention/deep-questions.json
- 根拠: deep-questions.json Q4 の `why`「規則は後から変えても何も失われない」/ docs/requirements.md:821-824（扉 #5「常に正しい時刻を確保する」はオフライン時に必ず破れる）/ crates/server/src/coverage.rs:196-200（ST02 第 9 回 Q32: `emitted_at` は端末の時計そのもので狂えばそのまま入る、として閾値を掛けた）/ collector-android/.../Heartbeat.kt:259（`val at = now()`。本番は `Instant.now()`、LocationService.kt）/ FixCollector.kt:42（出来事の時刻は `location.time`）
- kind: irreversible
- loss: discarded
- 提案: 「積んでから」も「出来事の時刻から」も、今の時刻を端末の時計で読む限り、時計が数か月先へ飛んだ瞬間に全部が 90 日超になる。Q4 の `why` を直し、C に「時計の逆行・大きな前進を見たら 90 日側では捨てない（2 GB 側だけで守る）」などの既定を足すか、Q4 の選択肢に時計の扱いを含める。
- 処置: escalated — 問いにはしない。Q4 の why を直し（時計の飛びはどちらの規則でも残る）、deep.md の C10（単調時計と食い違う前進では 90 日の側で捨てない）に足した。本人へは C の一覧で見せる

## R10. 圏外の日の生存信号は「接続が無い＝取れない状態」になり、利用が主語のソースでは保持して後で届く日が達成日から落ちる —— FR-8 と FR-78 / NFR-13 の衝突が一覧に無い
- 成果物: openspec/changes/st04-offline-retention/deep-questions.json
- 根拠: collector-android/.../Heartbeat.kt:70-84（`Capability.of` は `network` が無いと `capturable=false`。根拠は「送れない期間の記録は端末内にしか無い（**ST04 の保持上限を超えれば破棄される**）」）/ docs/requirements.md:568-569（NFR-13: 利用が主語は「取得できる状態の生存信号があった日」）/ openspec/specs/collection-coverage/spec.md:379-380（全信号が取れない状態なら③）
- kind: conflict
- 提案: ST04 が破棄を明示の報告で残すなら、`network` を取得可否に混ぜた根拠（破棄されうるから）は ST04 自身が置き換える。写真（ST09。ST04 に依存）が入ると、圏外の日は撮った写真が後で届いても未達になる。`blockers` 配列が行に残るので引き直せる（loss なし）。B の問い（推奨つき）に立てるか、下書きの手順 1 に「問わない理由」を書く。
- 処置: deferred ST09 — 効くのは利用が主語の C-01 ソースで、最初は写真（ST09。tasks.md 無し）。位置は端末が主語でいまは影響せず、blockers が残るので引き直せる。deep.md の「後続へ送るもの」と proposal の Impact に書く

## R11. Q3 の context と proto の「1 件でも残った日は『記録あり』のまま」は、ST02 の判定順と合わない
- 成果物: openspec/changes/st04-offline-retention/deep-questions.json
- 根拠: deep-questions.json Q3 の context「丸ごと覆う破棄の日だけが『破棄された期間』になり、1 件でも残った日は『記録あり』のまま」/ proto.html:68 に同じ記述 / openspec/specs/collection-coverage/spec.md:376-378（(3) 丸ごと覆う破棄 は (5) 記録が 1 件以上 より**先**）/ crates/server/src/coverage.rs:789-797（`dropped_full` を `event_count > 0` より先に見る）
- kind: premise
- 提案: 正しくは「日を丸ごと覆う破棄があれば、記録が残っていても⑤」「丸ごと覆わない日は破棄が状態を決めない」。記録が残る丸ごとの日は、送れたのに取り除けなかった記録（Sender.kt:118-121 の `outbox_shrink_failed`）や、同じソースを別の端末が記録した日（R1）で起きる。context と proto の文を直す。
- 処置: fixed deep-questions.json — Q3 の context を「丸ごと覆えば記録が残っていても⑤、覆わなければ破棄は状態を決めない」に直し、proto.html の説明文も同じに直した

## R12. Q2（PC 側に上限を置くか）は要件で幅が閉じており、人間に聞かずに C に落とせる
- 成果物: openspec/changes/st04-offline-retention/deep-questions.json
- 根拠: docs/requirements.md:95-98・:550（FR-8 / FR-9 / NFR-7 は C-01 だけを名指し）/ openspec/specs/desktop-collection/spec.md:236-245（「保持の上限は ST04 が決める —— 捨てなければ何も失われない」は先送りの注記で、上限を求める要件ではない）/ deep-questions.json Q2 の context（1 年止まっても 0.5〜1 GB）
- kind: conflict
- 提案: 失われるものがあるのは「置く」側だけで、「置かない」は扉を開けたままにし費用も小さい（CLAUDE.md の C の条件）。C に「C-02 には上限を置かない。正典の注記を書き換える。反転条件: PC のディスクの空きが実際に尽きたとき」として移し、Q2 は外す。「置く」を選ぶには NFR-7 の改訂が要ることも C に書く。
- 処置: escalated — 指摘を容れて Q2 を問いから外し、deep.md の C11（C-02 には上限を置かない。反転条件: PC のディスクの空きが実際に尽きたとき）に移した。要件の矛盾ではない（要件は C-01 だけを名指しし、正典の文は先送りの注記）ことを C11 に書いた。本人へは C の一覧で見せる

## R13. いまの置き場は 90 日ぶんを溜めると、起動時の読み戻しでプロセスごと落ち続けうる（C7 の理由が性能だけになっている）
- 成果物: openspec/changes/st04-offline-retention/deep.md
- 根拠: collector-android/.../Outbox.kt:15（`ArrayDeque(store.load())` で全件をメモリへ）/ OutboxStore.kt:65-66（`file.readText()` でファイル全体を 1 つの文字列にし、`catch (e: IOException)` だけを捕まえる —— `OutOfMemoryError` は捕まらない）/ LocationService.kt:93（`onCreate` で読む）・:122（`START_STICKY`）/ 実験 2（90 日で約 94 MB・129,600 件）。**端末上のヒープは実測していない**（この環境に JVM が無い）
- kind: technical
- 提案: ファイル全体の文字列・行のリスト・デコード済みの 129,600 件が同時にヒープに載る。落ちると `START_STICKY` で立て直しと失敗を繰り返し、**収集そのものが止まる**。C7 の理由にこの失敗の形を足し、「90 日ぶん（約 94 MB）を置いた状態でエミュレータの計測テストが起動して送り切る」を完了の条件にする。
- 処置: fixed deep.md — C7 の理由に OutOfMemoryError での立て直しの繰り返しを足し、「90 日ぶんを置いた状態でエミュレータの計測テストが起動して送り切る」を完了の条件として tasks へ落とす

## 処置のまとめ（呼び出し元が付けた）

13 件すべてに処置を付けた。

| 処置 | 件数 | 指摘 |
|---|---|---|
| 問いに足した | 1 | R6（Q6） |
| 問いから外して C に移した | 1 | R12（旧 Q2 → C11） |
| 問いにせず C（扉を開けたままにする既定）として記録し、本人に一覧で見せる | 5 | R1（C8）/ R2（C3）/ R3（C12）/ R8（C9）/ R9（C10） |
| 問いの文面を直した | 2 | R7（Q1 と C5）/ R11（Q3 の context と proto） |
| deep.md に技術の既定として足した | 3 | R4（C3）/ R5（C13）/ R13（C7） |
| 後続へ | 1 | R10（ST09） |

`loss` 付きの 6 件（R1 / R2 / R3 / R7 / R8 / R9）は、`review_triage.py` の規則どおり `escalated` にした。
うち 5 件は**持つ側・捨てない側を選べば何も失われない**（指摘の `loss` は「決めずにおくと失われる」の意味）ので、
問いの数を増やさず、C として deep.md に R 番号つきで残し、本人には答えを貼り戻すときに番号で異論を受ける形で見せる。
