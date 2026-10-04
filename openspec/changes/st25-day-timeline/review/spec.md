# ST25 成果物の独立レビュー（proposal / specs / design / tasks / ST25.md）

書いた文脈を持たない目で、`deep.md` の決定（Q1〜Q6・C1〜C11）→ `specs/` → `design.md` → `tasks.md` の写りと、
正典・要件・既存コード・並走中の change と枝との整合だけを見た。**成果物は 1 つも触っていない。**

確かめた範囲: `openspec/changes/st25-day-timeline/**`（proposal / deep / deep-answers-1.txt / proto.html / review/deep.md / specs/browsing-views/spec.md / design / tasks）、
`openspec/specs/browsing-views/spec.md`、`openspec/specs/desktop-collection/spec.md`（除外）、
`openspec/changes/st22-record-deletion/specs/{browsing-views,record-deletion}/spec.md`・`deep.md`、`openspec/changes/st08-browser-history/design.md`、`openspec/changes/st12-archive-ingestion/design.md`、
`docs/requirements.md`（FR-13 / FR-56 / FR-81〜85 / NFR-20）、`docs/stories/ST25.md` / `INDEX.md` / ST22.md / ST15.md、`docs/handoff/ST14.md`、`docs/screens.md`、`docs/testing.md`、
`migrations/*.sql`、`crates/server/src/{lib.rs,stay_store.rs,coverage.rs}`、`crates/collector-windows/src/{contract.rs,engine.rs,marker.rs,runtime.rs}`、
`web/{package.json,playwright.config.ts,vite.config.ts,src/Root.tsx,src/DayView.tsx,src/stays.ts,src/tokens.ts,src/__tests__/,e2e/}`、`tools/{seed.sh,stack.sh}`、
枝 `feat/st06-app-usage`（`docs/collector-contract.md`・migrations）/ `feat/st08-browser-history`（`docs/collector-contract.md`・`history/contract.rs`・`heartbeat.rs`・tasks.md・migrations）/
`feat/st12-archive-ingestion`（migrations・`archive/{timeline,legacy,youtube,myactivity}.rs`）/ `feat/st22-record-deletion`（`main...` との差分・`DayView.tsx`・`stays.ts`・`__tests__`）、
openspec 1.12.0 の `dist/core/specs-apply.js`（MODIFIED の Scenario 落ちの検査）。

## 機械の検査（人間の目より先に）

```
$ openspec validate st25-day-timeline --strict
Change 'st25-day-timeline' is valid                                      rc=0

$ python3 scripts/check_chain.py .
要件 118 件 / Story 36 本 / 扉 26 項
[ok] どれかの Story に拾われた要件: 116/118 件
chain: OK (0 件 / 未回収 0 件 / warn 0 件)                                rc=0   ← 観点 8（stories.json からの再生成との一致）も通っている

$ python3 scripts/check_scenarios.py . st25-day-timeline
Scenario 490 件 / 印 518 個 / 担保あり 433 / 人間の確認待ち 0
scenarios: FAIL (担保なし 57 件 / 名無しの確認待ち 0 件)                   rc=1

$ python3 scripts/review_triage.py . st25-day-timeline
指摘 13 件 / 仮決め 1 件 / 要件へ戻すもの 1 件（review/deep.md のぶん）
triage: OK                                                               rc=0
```

`check_scenarios` の 57 件は**上流では正常**（テストがまだ無い）。この change の Scenario は
`grep -c '^#### Scenario:' specs/browsing-views/spec.md` = **59**、うち 2 本（`日付を含むアドレスでその日の一覧が開く` / `前の日へ移ると前の日の滞在が出る`）は
ST16 の印が既にあるので 59 − 2 = 57 で、`tasks.md:19` の自己申告と一致した。
59 本すべての名前が `tasks.md` のどこかに一字一句で現れることも機械で確かめた（取りこぼし 0。`ルートを開くと今日の 1 日の画面が出る` だけ 2 か所）。

「日を移れる」の REMOVED + ADDED の理由（MODIFIED では既存の Scenario を落とせない）は事実だった —— openspec 1.12.0 の
`specs-apply.js:345-347` が「current spec contains scenario(s) not present in the modified block」で archive を止める。
REMOVED した要件の名前を指すものは正典・他の change に無い（`grep -rn "日を移れる"` は ST25 自身と ST16 の archive だけ）。
ST22 の delta とは要件の名前が 1 つも重ならず、ST22 の MODIFIED（5 本）も ST25 の ADDED（7 本）も互いの本文を前提にしないので、**archive の順はどちらでも正典の文は壊れない**。
ただし本文の参照が先に立つ（R19）。

---

## R1. D6 の「使った時間」の数え方が、spec の要件の文と Scenario の数値の両方と食い違う（PC は 25 分のはずが 15 分、スマホは 10 分で打ち切る）
- 成果物: openspec/changes/st25-day-timeline/design.md:114-121（D6）・specs/browsing-views/spec.md:143・:157-160
- 根拠: spec.md:143「アプリを使った時間を、前景に出た時刻から**次の前景の変化まで**とし、その間に入る離席・除外・PC の停止の時間を差し引く」/
  spec.md:159-160（10:00 VS Code → 10:15〜10:40 離席 → 10:50 次の窓。THEN **25 分** = 50 − 25）/
  design.md:116「前景の時刻から、**次の PC の記録（前景・離席の始まり・停止の始まり・除外の始まり）まで**…その間に入る離席…を差し引く」→
  次の PC の記録は離席の始まり 10:15（`leave_record` の `at` は区間の始まり。`crates/collector-windows/src/engine.rs:572-578`）なので **15 分**になり、差し引く区間はもう無い。
  復帰後（10:40〜10:50）は前景が変わらないので記録が出ず（`engine.rs:396-410` は前景が同じなら何も出さない）、D6 ではどのアプリにも数えられない /
  design.md:118（スマホは「次の前景か同じアプリが下がるまで、**10 分で打ち切る**」）は spec.md:143 の「次の前景の変化まで」と違う規則で、spec の Scenario（:167-170）は数値を持たないので打ち切りの有無を撃たない
- kind: technical
- 提案: D6 を spec の規則（次の**前景**の変化まで、間の離席・除外・停止を差し引く）に揃える。スマホの打ち切りを残すなら spec.md:143 の文に足し、数値の Scenario（例: 22:00 に前に出て次の前景が 07:00 → 10 分）を置く。
- 処置: fixed D6 仮 —— PC は spec の文のとおり「次の前景の変化（次の前景・除外の始まり）まで、間の離席・除外・停止を差し引く」に揃えた（離席の始まりで区切ると戻った後の時間が落ちる）。スマホの 10 分の打ち切りは spec の文に足し、Scenario `スマホの使った時間は 1 回 10 分で打ち切る` と `10 分に満たない離席も使った時間から差し引く` を置いた。tasks 3.1

## R2. D2 の許可リストはスマホの「後ろに下がった」イベント（2 / 23）を落とすが、D6 はそれを使い、D7 は「同じ正規化を通す」と言う
- 成果物: openspec/changes/st25-day-timeline/design.md:69・:73・:118・:142
- 根拠: design.md:69（`c01-app-usage` は `event_type = 1` だけを `foreground` にする）・:73（「スマホの前景以外のイベント」は表に無いので出さない）/
  :118（スマホの使った時間は「同じアプリが後ろに下がった時刻（`event_type` 2 / 23）まで」）/ :142「要約の計算は D2〜D6 の正規化を**そのまま**使う。`GET /day/records` と同じ関数を通す」→
  正規化の後には 2 / 23 が残らないので、D6 のスマホの区切りは計算できない
- kind: technical
- 提案: 正規化（内部の形）は 2 / 23 を持ち、`/day/records` に出すかどうかを別の段で決める、と D2 / D7 に書く。あるいは D6 から 2 / 23 を外す（R1 の揃え方と一緒に決める）。
- 処置: fixed D2 —— 正規化を 2 段にした（1 段目の内部の形はスマホの `background`（2 / 23）を持ち、2 段目で `/day/records` に出すものを選ぶ）。D2 の表に `background` の行を足し、要約（D7）は 1 段目を読むと書いた。spec の読み出しの要件にも「後ろに下がった記録は返さない」を置いた。tasks 1.2

## R3. 書庫の論理ソース名の前提が ST12 の枝の実装と違う（`c03-timeline-activity` / `-path` → 実装は `c03-timeline-move` / `-route`）。許可リストは閉じる側なので、黙って 0 件になる
- 成果物: openspec/changes/st25-day-timeline/design.md:18・:71・:183・tasks.md:42
- 根拠: design.md:71（許可リストの書庫の行に `c03-timeline-activity`）・:183（記録なしの事情 `archive` は `c03-timeline-path` を数える）・:18 /
  `git show feat/st12-archive-ingestion:migrations/202609181600_archive_ingestion.sql`:149-158 が登録するのは `c03-timeline-visit` / **`c03-timeline-move`** / **`c03-timeline-route`** / `c03-timeline-signal` / … で、
  `-activity` / `-path` は無い（`archive/timeline.rs:20-35` も同じ名前で積む）。ST12 の design.md:76・:158-159 は `-activity` / `-path` と書いており、**ST12 の中で design と実装が食い違っている** /
  tasks.md:42（Task 1.1）が突き合わせるのは `docs/collector-contract.md` の **payload の鍵名**だけで、書庫は収集の契約に載らない（サーバ側の取り込み）ので、名前の違いはどの検査にも掛からない /
  許可リストは「表に無いものは出さない」（design.md:73）ので、名前が違えばタイムラインの移動は 1 日の画面に出ず、経路の点は「書庫に位置がある」の件数に入らない —— どちらも試験は緑のまま
- kind: technical
- 提案: Task 1.1 に「`core.source` に実在する `c03-%` の名前（ST12 の移行・取り込み器が登録するもの）を D2 / D11 の表と突き合わせる」を足し、検証を `psql` の 1 行か Rust の試験（登録簿の `c03-%` がどれも表にあるか、表に無いものを列挙する）にする。ST12 の design と実装の食い違いは ST12 側の話なので、見つけたことは `docs/handoff/ST12.md` へ。
- 処置: fixed D2 —— 書庫の名前を枝の移行が登録する `c03-timeline-move` / `c03-timeline-route` にした（D2 の表・D11・Context）。Task 1.1 に「登録簿の `c03-%` がどれも D2 か D11 の表にある」試験（`st25_allow_registry_covers_archive`）を足した。ST12 の中の design と実装の食い違いは `docs/handoff/ST12.md` に置いた（走っている Story へは差し戻さない）

## R4. スマホの取りこぼし（`kind: gap`）を全部「取得元に残っていない」と出すが、ST06 の契約は `reason` を 2 つ持ち「混ぜない」と書いている
- 成果物: openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:254・:285-288・design.md:17・:70
- 根拠: `git show feat/st06-app-usage:docs/collector-contract.md`:189-216 —— gap は `reason` が `retention`（**問い合わせた**が無かった）と `clock_skew_abandoned`（**問い合わせていない**。取得元にはまだ残っていたかもしれない）で、
  :205「**`reason` は 2 つあり、混ぜない。** 扉 #14 の『データが無い』を…見分ける材料」/
  spec.md:254 は「スマホの取得元に残っていなかった期間（『スマホ 取得元に残っていない 始まり – 終わり』）」の 1 つの文言、design.md:17・:70 は `reason` を読まない /
  FR-85（requirements.md:106-108）が名指しするのは `retention` の側だけ
- kind: conflict
- 提案: `reason` ごとに文言を分ける（例: `clock_skew_abandoned` は「スマホ 時計のずれで取り直した（取れていない）」）か、spec を「取れていない」の中立な語にし、`reason` を行に添える。Scenario を `reason` ごとに 1 本ずつ。表示の規則なので仮で閉じられる。
- 処置: fixed D21 仮 —— `reason` を写し、`retention` は「スマホ 取得元に残っていない」、`clock_skew_abandoned` は「スマホ 時計のずれで取れていない」と文字を分けた（ST06 の契約「混ぜない」）。spec の文と Scenario `時計のずれで取り直さなかった期間は別の文字の行になる` を足した。反転条件は D9 の表示の規則（文言が読みにくければ変える。記録は残る）

## R5. 除外の行の「終わり」を `range_end` から取るが、PC の収集は除外の区間を送信の契機（5 分）で切り、続きは数えが 0 なら記録にしない —— 「除外 16:05 – 16:25（3 件）」の 1 行は作れない
- 成果物: openspec/changes/st25-day-timeline/design.md:15・:67・:157-161・specs/browsing-views/spec.md:280-283
- 根拠: `crates/collector-windows/src/runtime.rs:41`（`SEND_INTERVAL_SEC = 300`）・:451-458（送信の契機ごとに `engine.flush`）/
  `engine.rs:489-490` の `flush` は `close_excluded(now, keep_open = true)` で、数えが 1 以上なら `[since, now]` を 1 件出して**数えを 0 から数え直す**（:509-520）。
  同じ窓を見続けて数えが増えなければ、次の吐き出しも除外から出たときも**記録を作らない**（:506 `emit = span.count > 0 && …`、:521-523）——
  `engine.rs` の試験 `excluded_count_is_not_inflated_by_flush`（30 分の除外 → 記録は最初の 5 分の 1 件）がこの形を固定している /
  design.md:67 は除外の終わりを `range_end` とし、D9 は隣り合う除外の記録をまとめない →
  20 分の除外は「16:05 – 16:10（n 件）」などに割れ、末尾の時間はどの行にも入らず、FR-83 の本文が避けたい「何も無い時間」に戻る（spec.md:263）
- kind: technical
- 提案: D9 に「除外の行は、除外の記録の始まりから**次の前景の記録まで**（`range_end` は下限）とし、続く除外の記録は 1 行にまとめて件数を足す」と書き、5 分の契機をまたぐ除外の Scenario（偽の記録 2 件 + 次の前景）を spec に足す。収集側の形を変えるなら ST25 の範囲ではないので `docs/handoff/` へ。
- 処置: fixed D9 —— 除外の行の終わりを次の前景の記録まで（`range_end` は下限）にし、続く除外の記録を 1 行にまとめて件数を足す、を spec の SHALL と Scenario `送る契機で区切られた除外は 1 行にまとまる` に置いた。既存の Scenario `除外は件数だけの行になる` の WHEN を「16:25 に次の前景」に直した。収集側（`desktop-collection`）は変えない。tasks 4.1 / 8.2

## R6. ブラウザ履歴の「最後の取得」（D10）は ST08 の生存信号の数え方と合わない。spec の側はその語を観測できる形で定義していない
- 成果物: openspec/changes/st25-day-timeline/design.md:163-172（D10（仮））・specs/browsing-views/spec.md:256-257・:290-298
- 根拠: design.md:167（`core.heartbeat` の `c02-browser-history` で `successes > 0` の `received_at` の最大を「最後の取得」の近似にする）/
  ST08 design.md:229-237（D12）—— 生存信号は 86,400 秒ごと、「区間に読みが 1 回も無いとき（起動直後の信号・24 時間経たない起動）は…**写しを取って開けるか（`SELECT 1` まで）を確かめ…確かめも 1 試行と数える**」→
  行を 1 つも読んでいない起動直後の信号が `successes > 0` で届く。D10 の反転条件（:171-172）は「取得しても `successes` が 0」「信号が取得の前」だけで、**取得していないのに `successes > 0`** の場合を挙げていない（その場合「まだ届いていない」の行が消え、届いていない訪問が「見ていなかった」に見える）/
  spec.md:256・:292 の「ブラウザ履歴の最後の取得」はサーバに印が無い（design.md:16・:165）ので、Scenario の WHEN を試験で作るには D10 の近似を知っている必要がある
- kind: technical
- 提案: D10 から生存信号を外して `ingest_time` の最大（削除済みを含む）だけにするか、`successes > 0` かつ同じ区間に `visit` / `vanished` の到着がある信号に限る。spec の WHEN は「ブラウザ履歴の記録が最後に届いた時刻」の形にして、観測できる語にする。
- 処置: fixed D10 仮 —— 生存信号を外し、`c02-browser-history` の `ingest_time` の最大（削除済みを含む）だけにした。spec の語を「ブラウザ履歴の記録が最後に届いた時刻」にした（観測できる語）。代償（新しい訪問の無い取得では進まない）と反転条件を D10 に書いた。tasks 4.2

## R7. 「時刻の範囲を指定する」の Scenario が、同じ要件の区間の記録の規則と食い違う
- 成果物: openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:51・:54・:71-74
- 根拠: spec.md:54「区間を持つ記録（離席・PC の停止・除外・取得元に残っていなかった期間）は、その区間がその日（**または指定した範囲**）と重なれば、始まりがその日の外でも返す」/
  spec.md:74（THEN「**出来事の時刻が**その範囲に入る記録**だけ**が返る」）→ 9:50〜10:20 の離席は前者なら返り、後者なら返らない。
  `c01-app-usage` の gap は `event_time` が**終わり**（ST06 の契約 :193）なので、「出来事の時刻」がどちらの端かもソースで変わる（design.md:109 が認めている）
- kind: technical
- 提案: Scenario を「点の記録は時刻が範囲に入るものだけ、区間の記録は範囲と重なるもの」に直し、範囲の外から始まって中で終わる離席を WHEN に入れる。
- 処置: fixed specs/browsing-views/spec.md —— Scenario `時刻の範囲を指定するとその範囲の記録だけが返る` を、点の記録（9:30 / 10:30 の前景）と範囲と重なる区間（9:50 – 10:20 の離席）を WHEN に入れ、THEN を「10:30 の前景と離席が返り、9:30 は返らない」にした。tasks 2.1

## R8. Q2 の（1）（3）—— 書庫どうしの 2 経路の組を両方出す —— が spec に無い。spec は「書庫と手元」だけを言う
- 成果物: openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:58・:101-104
- 根拠: deep.md:104-106（Q2 の本人の答え:「（4）だけ 1 件に畳み、（1）〜（3）は両方出す」。（1）移行前のロケーション履歴と Timeline.json、（3）YouTube の視聴履歴とマイアクティビティは**どちらも書庫**）/
  spec.md:58「書庫から入った記録と手元で集めた記録は、同じ出来事でも別の記録として返す」・Scenario は（2）の Chrome だけ（:101-104）→
  実装が書庫どうしの組を畳んでも spec は落ちない（いまは D3 / D4 が論理ソースごとなので畳まれないが、正典が持っていない）
- kind: daily
- 提案: :58 の文を「論理ソースが違う記録は、同じ出来事でも別の記録として返す（2 台の PC の訪問を除く）」にし、YouTube の視聴履歴とマイアクティビティの組の Scenario を 1 本足す。（1）は位置なので、記録なしの事情の件数（D11）で両方を数えるかを書く。
- 処置: fixed D16 仮 —— spec の文を「論理ソースが違う記録は、同じ出来事でも別の記録として返す（書庫と手元の組・書庫どうしの組を畳まない）」にし、Scenario `書庫どうしの同じ出来事も両方返る`（YouTube の視聴履歴とマイアクティビティ）を足した。（1）の位置は D11 と spec の事情の要件に「タイムラインと移行前の両方を数える」と書いた。反転条件は D16

## R9. 本人の決めた「10 分」が Scenario の値で固定されていない（17 分と 6 分なので、7〜17 分のどの閾値でも緑）
- 成果物: openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:265-273
- 根拠: deep.md:95（本人が動かした軸 4「10 分以上の離席…を 1 行ずつ」）/ spec.md:267（10:15 – 10:32 = 17 分 → 行）・:272（11:00 – 11:06 = 6 分 → 行なし）/
  tasks.md:32・:86 は定数 `AWAY_ROW_MIN_MINUTES` を試験が名指しするとしているが、それは design / tasks の側で、spec は値を持たない
- kind: technical
- 提案: 境界の Scenario に替える（ちょうど 10 分の離席は行になる / 9 分 59 秒は行にならない）。
- 処置: fixed specs/browsing-views/spec.md —— 境界の Scenario にした（ちょうど 10 分 10:15 – 10:25 は行になる / 11:00:00 – 11:09:59 は行にならない）。tasks 4.1 に境界の試験と定数の固定を書いた

## R10. Q1 の「e2e の実寸の基準」（4 つの日・閉じた状態の高さ）が spec / tasks に写っていない。偽データの書庫は proposal と design で食い違う
- 成果物: openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:357-373・design.md:197-205・tasks.md:111-114・:159-161・proposal.md:82
- 根拠: deep.md:99「下流の e2e は**同じ構成の偽データ**で、**閉じた行の高さ**・360 px 幅で 640 px に見える行・24 px を割る対象が無いことをアサートする」（4 つの日: 18 行 1,614 px / 34 行 3,172 px / 18 行 2,053 px・見える行 4 / 11 行 921 px）/
  spec.md:360・:372 は「いつもの日（`normal`）で 6 行以上」だけ（高さ・他の 3 日は無い）/
  proposal.md:82「偽データ: 2026-09-07 に PC・ブラウザ・スマホ・**書庫**の記録と…」⇔ design.md:199-205（D13）・tasks.md:111 には書庫が無い /
  本人の測った「書庫を置いた日」は開いてすぐ見える行が **4**（< 6）—— 偽データの `normal` に書庫を足せば spec.md:373 が落ちうる
- kind: technical
- 提案: proposal と D13 のどちらに揃えるかを決める（書庫を入れるなら別の日付にする）。高さの上限（いつもの日の閉じた状態の高さ）を spec に置くか、置かない理由を design に書く。`max`（滞在 15 件）の日を e2e に入れるかも同じ所で決める。
- 処置: fixed D15 仮 —— 偽データに書庫は入れない（D13 に揃え、proposal の Impact を直した。書庫を置いた日は本人の測定で見える行が 4 で、いつもの日と同じ日に置けない）。e2e の実寸の基準を D15（仮）に新設し、日の高さの合計は撃たず「行の数・見える行 6・1 行 200 px 以下・横にはみ出さない」を spec の Scenario にした（`閉じた行は 200 px を超えない` / `幅 360 px で横にはみ出さない`）。滞在 15 件の日は e2e に入れない（D15 の理由）

## R11. C5「感度で隠さない」が spec のどこにも無い
- 成果物: openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:48-134（記録の読み出し）・:136-190（要約）
- 根拠: deep.md:63（C5「1 日の画面は…**感度で隠さない**」）/ `grep -n "感度" specs/browsing-views/spec.md` → 0 件 /
  `core.event.sensitivity`（`migrations/202609081618_envelope.sql:28-29`）は 0〜3 を持つので、感度の高い記録を読み出しから外す実装でも全 Scenario が通る。
  ST24（感度）が後から同じ capability の読み出しに手を入れるとき、正典に「隠さない」が無いと、隠す側への変更が差分として見えない
- kind: daily
- 提案: 読み出しの要件に「感度の値で記録を外さない」を 1 文と、感度 3 の PC のウィンドウの記録が返る Scenario を 1 本足す。
- 処置: fixed D17 仮 —— 読み出しの要件に「感度の値で記録を外さない」を置き、Scenario `感度の高い記録も返る` を足した。反転条件: ST24 が 1 日の画面にも感度の扱いを入れると決めたら、ST24 が同じ capability で MODIFIED する（正典に「外さない」があるので差分として見える）。tasks 1.2

## R12. Requirement の本文だけが言い、Scenario が撃たない主張がある
- 成果物: openspec/changes/st25-day-timeline/specs/browsing-views/spec.md
- 根拠: 次の文はどの Scenario の WHEN / THEN にも現れない（実装が守らなくても spec は緑）:
  - :145・:259「色だけで区別しない」（要約のソースと、取れていない時間の行）。ST22 は同じ型に Scenario `消した行は文字で区別される` を置いている（ST22 spec :165-168）
  - :147「『消した』の行には要約を出さない」
  - :199「畳んだ行も一度に 1 つだけ開く」（:235-238 は行の方だけ）
  - :255 括弧「（要約の使った時間からは差し引く）」—— 10 分未満の離席を差し引くこと
  - :260「今日の画面では、いまの時刻より後を取れていない時間の行にしない」
  - :377「（取得できないときはダーク）」
  - :380「意味を担う非テキスト…3:1 以上」
- kind: technical
- 提案: それぞれに Scenario を 1 本ずつ置くか、Scenario にしないものは本文から外して design に移す。
- 処置: fixed specs/browsing-views/spec.md —— それぞれ Scenario を置いた: `ソースの種類は文字で出る` / `取れていない時間の行は文字で区別される`（色だけで区別しない）/ `消した行には要約が出ない` / `畳んだ行も一度に 1 つ` / `10 分に満たない離席も使った時間から差し引く` / `いま離席中の時間は今日の行になる`（いまより後を出さない）/ `OS の明暗が取得できないときはダーク`。3:1 の文は「行の種類を枠や線の色で担わせず文字で担わせる」に直した（意味を担う非テキストを足さない）

## R13. Scenario の言葉が観測できない・1 本に主張が 2 つ以上ある・数え方が 2 通りに読める
- 成果物: openspec/changes/st25-day-timeline/specs/browsing-views/spec.md
- 根拠:
  - :134「足す前と**同じ意味**の応答」—— 何を比べれば真偽が決まるかが無い（tasks.md:63 は鍵の集合と並びで比べるので、それを THEN に書けばよい）
  - :212-213「題名が **23 回変わり**」→「23 件を持つ 1 行」—— 最初に前に出た 1 件を含めると 24 件。どちらの数え方かで試験の期待値が変わる
  - :365-368 の THEN（行の数 = 滞在・移動・記録なし・消した・取れていない時間の行の数の和）は、tasks.md:159 が `/api/day` の応答から数える —— 応答が記録を行として混ぜても両辺が一緒に増えて緑。「PC の記録 1,000 件以上」の WHEN が THEN に効いていない
  - 主張を束ねている: :39-40（稼働状況が出る **かつ** そこから 1 日へ移れる）/ :44-45（稼働状況 **と** マスタ管理）/ :68-69（順序・持つ項目・他の日を返さない）/ :113-114（件数 **と** 本文なし）/ :334-335（件数 **と** 滞在が無い）/ :242-243（開く **と** 輪郭 —— :405-408 と重複）
- kind: technical
- 提案: :134 は「同じ鍵の集合と並び」に、:212 は「VS Code の前景の記録が 23 件」に直す。:368 は「閉じた行に、出来事 1 件の題名が出ない」など応答に依らない観測にする。束ねたものは分ける。
- 処置: fixed specs/browsing-views/spec.md —— `記録の読み出しの形は変わらない` の THEN を「同じ鍵の集合と同じ並び」に、畳みの WHEN を「VS Code の前景の記録が 23 件続き」に、`閉じた画面に記録 1 件ごとの行は出ない` の THEN を「PC の前景の題名が 1 つも出ず、行の数は 100 未満」（応答に依らない観測）にした。束ねた Scenario を分けた（稼働状況の行き先 3 本・記録の順と他の日・除外の件数と本文・書庫の件数と滞在）。開く／輪郭の重複は `畳んだ行はキーボードで開ける` から輪郭を外した

## R14. design の D 番号に、観測できる振る舞いが置かれている（`openspec archive` で正典から落ちる）。spec に実装の名前が 1 つある
- 成果物: openspec/changes/st25-day-timeline/design.md・specs/browsing-views/spec.md:360
- 根拠:
  - D2（:75）今日いま離席中（enter だけ）を「いままでの離席」として出す / D2（:80-81）ブラウザの `excluded` は行にしない（仮）
  - D7（:137-138）同点はアプリ名・ドメインの昇順 / ドメインは小文字にして先頭の `www.` を外す（spec.md:164 の `github.com` はこれに依存する）
  - D8（:149）位置の件数は滞在の行だけで、移動・記録なしの行には出さない / D8（:152）地域の時刻の形 `（London 8:02）`
  - D9（:160）消した行の中で始まる取れていない時間は直前の行の後に置く（spec.md:258 の規則はこの場合を決めていない）
  - D11（:176）事情の並び順（仮）
  - D12（:191）知らないアドレス（空・`#/` 以外）も今日の 1 日の画面 / :192 マスタ管理への入口を稼働状況にも残す / :194 見出しを「<日付>の滞在」から「<日付>」へ（仮）
  - spec.md:360「いつもの日（偽データの `normal`）」—— 偽データの名前（`tools/seed.sh` の引数）が正典に入る
- kind: technical
- 提案: 仮のものも含め、観測できるものは spec に Requirement の文か Scenario で置き、design には「なぜ」と反転条件だけを残す。:360 は偽データの名前を外し、行の構成（滞在 9 件など）で書く。
- 処置: fixed specs/browsing-views/spec.md —— 観測できるものを spec に上げた: 同点の並びと `www.` を外すドメイン（Scenario 2 本）/ いま離席中の今日の行 / ブラウザの訪問でない記録を返さない / 位置の件数は滞在の行の詳細だけ / 取れていない時間がどの行にも入らないときの位置 / 事情の並び順 / 解釈できないアドレス・稼働状況からマスタ管理への行き先・見出し（Scenario 2 本）。偽データの名前 `normal` を spec から外し、行の構成（滞在 9 件・PC とブラウザとスマホの記録がある日）で書いた。地域の時刻の書式（`（London 8:02）`）は表示の組み立てなので design に残す

## R15. 画面の文字を言う Scenario の多くが、サーバの試験だけに割り当てられている（印は付くが、画面に出ることは誰も撃たない）
- 成果物: openspec/changes/st25-day-timeline/tasks.md:71-78・:86-92・:98-103・:134-136
- 根拠: spec.md:152-190（要約。THEN「その滞在の行に『PC』の文字と…」）・:265-298（取れていない時間。THEN「『離席 10:15 – 10:32』の**行がある**」）・:322-355（事情。THEN「文字で添えてある」）は**画面**の Scenario /
  tasks.md:75-78（3.2 → `CT st25_summary`）・:87-89（4.1 → `CT st25_gaps`）・:91-92（4.2）・:100-103（5.1 → `CT st25_reasons`）はどれも Rust の試験で、撃てるのは `/day` の JSON まで /
  画面の側の 8.1（tasks.md:134-136 `VT day-summary`）は Scenario を 1 本も名指ししていない → `check_scenarios.py` は Rust の印で「担保あり」にするが、文言（「離席 10:15 – 10:32」「除外 …（3 件。本文なし）」「書庫: <表示名>」）が画面に出なくても緑 /
  `docs/testing.md:71-73` は「指定と勘定は jsdom」なので、文字の組み立ては jsdom に印が要る
- kind: technical
- 提案: 画面の文字を言う Scenario は、サーバの試験と jsdom（8.1 / 8.2）の**両方**に印を置くと tasks に書く。少なくとも文言を持つもの（離席・PC 停止・除外・スマホ・ブラウザの行、事情の添え書き、要約の 1 行の形）は jsdom で固定する。
- 処置: fixed 8.1 —— Global Constraints に「画面の文字を言う Scenario はサーバの試験と jsdom の両方に印を置く」を書き、8.1 / 8.2 に要約・取れていない時間の行・事情の Scenario を名指しで割り当てた（`VT st25-summary.test.tsx` / `st25-gaps.test.tsx`）

## R16. 検証コマンドの多くが、その Task で試験を 1 本も足さなくても rc=0 になる
- 成果物: openspec/changes/st25-day-timeline/tasks.md
- 根拠:
  - `CT` の絞り込みが前方一致で入れ子: 3.2 `st25_summary`（:78）は 3.1 の `st25_summary_usage` で、4.1 `st25_gaps`（:89）は 4.2 の `st25_gaps_browser` で、5.1 `st25_reasons`（:103）は 5.2 の `st25_reasons_no_stay_from_archive` で `1 passed` になる
  - 8.3 `VT text-contrast`（:140）は**既にある** `web/src/__tests__/text-contrast.test.ts` に当たる
  - 7.1 `VT day-view`（:123）は既にある `day-view.test.tsx` / `day-view-limits.test.tsx` に当たる（ST22 の後は R17）
  - 8.2 と 9.2 が同じ `VT timeline`（:138・:153）、7.2 と 10.1〜10.4 が同じ `ET day-timeline`（:126・:161-167）—— どれか 1 本が通れば全部の行が通る
  - 11.3 `grep -q "ST25" docs/handoff/ST14.md`（:176）は**いま既に真**（`docs/handoff/ST14.md:66`）
  - 6.1（:114）は rc だけで、本文の「`gaps` の数と、PC・ブラウザ・スマホの要約が 1 行以上あることを出す」「受け入れ件数が 0 で落ちない」を判定しない
- kind: technical
- 提案: 接頭辞を Task ごとに重ならない名前にする（`st25_summary_top_` / `st25_gaps_rows_` / `st25_reasons_attach_` など）か `-- --exact` を使う。VT / ET は新しいファイル名か `-t '<Scenario 名>'` で絞る。6.1 は `curl /day?date=2026-09-07 | jq` で件数を判定する。11.3 は handoff を読んだ証跡（`review/` の処置）を判定にする。
- 処置: fixed 1.2 —— 接頭辞を Task ごとに重ならない名前にした（`st25_allow_` / `st25_fold_` / `st25_range_` / `st25_api_records_` / `st25_api_unchanged_` / `st25_usage_` / `st25_summary_` / `st25_other_` / `st25_api_day_` / `st25_gaprow_` / `st25_browser_` / `st25_reasons_` / `st25_nostay_`）。VT / ET は新しく作るファイルを名指しする形にし、e2e は Scenario ごとに `test(...)` を分けて数を見る。6.1 は `/day` の応答を `jq -e` で判定、11.3 は `review/handoff.md` に読んだ証跡を書いて判定する

## R17. ST22 の merge 後には、tasks が名指しする試験のファイルが無い（`day-view.test.tsx` は `DayView.test.tsx` に名前が変わる）。画面の読み出し先の付け替えで直す既存の試験も名指しされていない
- 成果物: openspec/changes/st25-day-timeline/tasks.md:19-20・:120-123・:134-136
- 根拠: `git diff --name-status main...feat/st22-record-deletion -- web` → `R100 web/src/__tests__/day-view.test.tsx → web/src/__tests__/DayView.test.tsx`（`day-view-limits` も同じ）/
  tasks.md:15-16 は ST22 の merge を前提にしている（Task 1.1）のに、:20・:121 は旧い名前を指し、:123 の `VT day-view` は大文字小文字が違う `DayView` に当たらない（当たるファイルが 0 なら vitest は rc≠0）/
  ST22 の枝の `DayView.test.tsx:195-199` と `DayView-erase-erased-row-keyboard.test.tsx` は `/api/stays?date=` の呼び出しを固定していて、design.md:47（画面は `/stays` の代わりに `/day` を読む）でそれが落ちる。
  ST16 の印（`日付を含むアドレスでその日の一覧が開く` ほか）はその試験の中にある
- kind: technical
- 提案: tasks の名前を ST22 後のもの（`DayView.test.tsx`）にし、7.1 / 8.1 に「`/api/stays` を見ている既存の試験を `/api/day` に付け替える（印は残す）」を足す。
- 処置: fixed 7.2 —— 名前を ST22 の後の `DayView.test.tsx` にし、7.2 に「`/api/stays?date=` の呼び出しを固定している既存の試験を `/api/day` に付け替える（印は残す）」を足した

## R18. 偽データが登録簿と記録を先に入れると、ST08 の移行（`external_id_kind = 'record'`）が開発・確認バッチの DB で当たらなくなる
- 成果物: openspec/changes/st25-day-timeline/design.md:197-205（D13）・tasks.md:111-114・proposal.md:89
- 根拠: `git show feat/st08-browser-history:migrations/202609211400_browser_history_record_id.sql` は `c02-browser-history` に**記録が 1 件も無いときだけ** `external_id_kind` を `record` にする（ST08 design D7）/
  `c02-browser-history` は main で既に登録されている（`migrations/202609111111_coverage_rebuild.sql:132`。`external_id_kind` は既定）/
  D13 は 2026-09-07 にブラウザの訪問 約 190 件を入れる。ST25 が ST08 より先に merge されると、`tools/stack.sh` の DB（`STACK_RESET=1` でしか作り直さない。`stack.sh:59`）に訪問が入った後で ST08 の移行が走り、**その DB では二度と `record` にならない** ——
  同じ訪問の更新が版を積まず別の行になる（ST08 の D8 の前提が崩れる）。proposal.md:89「まだ main に無いソースは…偽データで先に作り」がこの順を想定している
- kind: technical
- 提案: 偽データの `c02-browser-history` の訪問は ST08 が main にあるときだけ入れる（`external_id_kind = 'record'` を確かめてから）か、ST08 と同じ識別子の形（`v1:visit:…`）で `external_id` を付け、Task 6.1 に「ST08 の前後どちらでも移行の結果が変わらない」を検証として置く。
- 処置: fixed D13 —— 偽データのブラウザの訪問は `c02-browser-history` の `external_id_kind` が `record`（ST08 の移行が済んでいる）のときだけ入れ、そうでなければ入れないと出して続ける。tasks 6.1

## R19. ST25 は ST22 のコードと spec の上に積むが、Story の `requires` は ST01 だけ。ST25 が先に archive されると、正典に無い capability と要件を指す
- 成果物: docs/stories/ST25.md:5（`requires: [ST01]`）・openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:201・tasks.md:15-16・:41-43
- 根拠: spec.md:201 は `record-deletion` と ST22 の ADDED「滞在の行は画面を移らずに開き、詳細にその時間の記録の件数が出る」を名指しするが、どちらも `openspec/specs/` に無い（`ls openspec/specs` に `record-deletion` が無い）/
  tasks.md:15-16「ST22 のコードが main に無ければ始めない」は**検査で落ちる**形で、admission の**待ち**にならない（`requires` に無いので admission は ST22 を待たない。CLAUDE.md の admission の (1)）/
  前例: INDEX.md:202-204 は ST22 が ST16 の上に積むとき `requires` に ST16 を足した
- kind: conflict
- 提案: `stories.json` の ST25 の `requires` に ST22 を足して再生成し（INDEX に訂正の段落）、Task 1.1 は確かめるだけにする。足さないなら、ST22 より先に archive したときの spec.md:201 の参照の扱いを proposal に書く。
- 処置: fixed D18 仮 —— `docs/stories/stories.json` の ST25 の `requires` に ST22 を足して再生成し（layer 1 → 3、壊してはいけないものに ST22 の詳細）、`docs/stories/INDEX.md` に訂正（2026-10-01）を書いた。admission が ST22 の archive を待たせる。proposal の Impact も直した。反転条件: ST22 が開ける行を作らずに閉じたら、ST25 が開ける行を作り、requires から外す

## R20. 「消した」時間の PC・ブラウザ・スマホの記録は、要約にも開いた中身にも出ない —— FR-56 ★「記録 1 件まで届く」と食い違う
- 成果物: openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:147・:194・design.md:140-141
- 根拠: requirements.md:554（FR-56 ★「**記録 1 件まで届くことは変えない**（開けば全件が時刻順で出る）」）/
  ST22 の deep.md:38（Q1 本人の答え: 滞在を消すと消えるのは**滞在と位置**）・ST22 spec `record-deletion`「同じ時間の PC のウィンドウの記録は消えない」→ その時間の PC の記録は生きている /
  spec.md:147（消した行には要約を出さない）・:194（開けるのは滞在・移動・記録なしの行）・ST22 spec（消した行に詳細を出さない）→ 消した時間の記録は 1 日の画面のどこからも届かない。
  design.md:140-141 は（仮）と反転条件を持つが、FR-56 の本文との食い違いは書いていない。同じ型が今日の「最後の位置からいままで」（`stay_store.rs` の day_view の説明「今日は最後の位置まで」）にもある
- kind: conflict
- 提案: どの行にも入らない時間の記録をどう届かせるかを決める（消した行も「その時間の記録」だけは開ける / 直前の行の中身に入れる など）。仮で閉じるなら、FR-56 ★ との関係を D7 の反転条件の所に書く。
- 処置: fixed D19 仮 —— どの行にも入らない時間（消した時間・今日の最後の位置からいままで）の記録を「ほかの記録 始まり – 終わり」の行に出し、要約を付けて開けるようにした（ST22 の「消した行に詳細を出さない」は変えない）。spec の SHALL と Scenario 2 本、D7 に反転条件、tasks 3.3 / 8.1

## R21. NFR-20 は ST25 だけが `satisfies` に持つが、ST25 の spec はどこでも NFR-20 を扱わない
- 成果物: docs/stories/ST25.md:4・openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:192-248・:375-408
- 根拠: `grep -ln "NFR-20" docs/stories/ST*.md` → ST25.md だけ（ST22.md:4 は FR-50、ST15.md:4 は FR-34 / FR-53）/
  deep.md:133（Q6: この Story の画面に取り返しの付かない操作は増えないので NFR-20 が新たに効く対象は無い）/ spec の導出元のどこにも NFR-20 が無い（`grep -n "NFR-20" spec.md` → 0 件）/
  それでいて ST25 は ST22 の詳細の中身を組み替える（spec.md:201・:245-248 は時刻順の記録を詳細と**同じ場所**に入れる）。44 px は ST22 の Scenario `詳細の末尾の消す操作は 44 px を下回らない`（ST22 `record-deletion` :275）だけが撃つ
- kind: conflict
- 提案: (a) 時刻順の記録を足した後も「この滞在を消す」が 44×44 を保つ Scenario を ST25 の spec に置き、導出元に NFR-20 を書く。(b) NFR-20 の宛先を ST22（消す）と ST15（止める）へ移し、INDEX に訂正を書く。どちらかに揃える。

---

## 観点ごとの該当なし

- 観点 1（deep → specs）: Q3 / Q4 / Q5 / Q6・C1〜C4・C7〜C10 は Scenario に写っていた。Q1 は 4 軸とも Requirement と Scenario がある（数値の 10 分は R9、e2e の基準は R10）。Q2 は R8、C5 は R11、C6 の代表の選び方は design（D3）だけだが、読み出しの結果に出る列（地域）の Scenario はある。当初案を覆したもの（Q1 の 2 軸）は、覆した後の形だけが spec にあり、古い形（「要約と見出しに文字で添える」）は残っていない。要件へ戻すもの（FR-56）は requirements.md:549-554 に ★ 2026-10-01 つきで入っている
- 観点 3 のうち proposal の Capabilities と `specs/` のディレクトリ: 一致（`browsing-views` だけ）。INDEX.md:78 の割当（`browsing-views`: ST16, ST22, **ST25**, ST26, ST36）とも一致。前倒しは無い
- 観点 5 のうち再生成との一致（`check_chain.py` 観点 8）: 通っている。ST25.md の「完了の判定」は Q1（滞在の行の中に要約、開くと時刻順）と Q5（「1 日の画面が」明るくなる）に揃っている。`satisfies` に無い要件を specs が満たそうとしている箇所は無かった（導出元の FR-20 / FR-81〜85 / FR-33 などは既存の要件を読む側として引いているだけで、満たす宛先を変えていない）
- 人間の確認待ち: 「無し」（tasks.md:178-182）は妥当。物理（ロック・電池・GPS・時間そのもの）に当たる Scenario は無かった
- 処置: fixed D20 仮 —— (a) を採った。開いた中身の要件に「この滞在を消すを 44×44 のまま詳細の末尾に置く」と導出元 NFR-20 を書き、Scenario `時刻順の記録を足しても消す操作は 44 px を下回らない` を置いた（tasks 10.2）。INDEX の訂正にも NFR-20 の満たし方を書いた
