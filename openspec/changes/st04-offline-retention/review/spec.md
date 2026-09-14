# ST04 spec レビュー（独立）

対象: `openspec/changes/st04-offline-retention/` の proposal.md / specs/device-collection/spec.md /
specs/collection-coverage/spec.md / design.md / tasks.md、`docs/stories/ST04.md`。
突き合わせの正本: deep.md（Q1 / Q3 / Q4 / Q5 / Q6、C1〜C13、要件へ戻すもの）、`docs/requirements.md`、
`docs/stories/INDEX.md`、`openspec/specs/`（正典）。**成果物は触っていない。**

実施日: 2026-09-15。ブランチ `docs/st04-upstream`。

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate st04-offline-retention --strict` | `Change 'st04-offline-retention' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0。観点 8 の再生成との一致を含む） |
| `python3 scripts/check_scenarios.py . st04-offline-retention` | `scenarios: FAIL (担保なし 48 件)`（rc=1）。48 件はすべてこの change が足した Scenario（device-collection 28 / collection-coverage 20）で、上流の段階ではテストが無いので想定どおり。tasks 11.2 が回収する |
| `python3 scripts/review_triage.py . st04-offline-retention` | `triage: OK`（rc=0。このファイルを書く前の状態。review/deep.md の 13 件） |

手で確かめたこと:

- **MODIFIED が正典の Scenario を落としていないか**: 5 本の MODIFIED Requirement について、正典（`openspec/specs/{device-collection,collection-coverage}/spec.md`）の本文と Scenario を change 側と行ごとに比べた。
  **落ちた SHALL 行・Scenario は 0**。変わったのは SHALL 行の追加、注記の追加、「一時的な失敗では捨てない」の WHEN に「保持の上限を超えていない状態で」を足した 1 か所だけ
- **tasks と Scenario の対応**: 新しい 48 本は、名前が一字一句一致する形でどれかのタスクに書かれている。既存の Scenario の印は `SenderTest.kt` / `OutboxStoreTest.kt` / `spans.rs` / `smoke.sh` に残っている
- **tasks が名指しする道具と名前**: `tools/{check-migrations,check-immutable,check-openapi,check-boundaries,android-emulator,smoke,seed}.sh`、crate `ashiato-server`、`:app:testDebugUnitTest`、package `dev.ashiato.collector`、`MAX_BATCH`、`dropPermanentlyRejected`、`Config.overrideForTest`、`LocationService.notification()`、`FileOutboxStore` / `salvage`、`dropped_full`、`DayCell`、`WeekRow` / `WeekDetail`、`contrast.ts`、`__tests__/one-scroll.test.tsx`、`TEXT.muted`、`202609111112_immutable_heartbeat.sql`、CI の `android-instrumented` は**すべて実在**した。食い違いは R13 の 2 点だけ
- **要件へ戻すもの**: FR-8 / FR-9 / FR-10 / FR-54 / NFR-7 に ★ 2026-09-14 が入っている（`docs/requirements.md:97` `:106` `:118` `:439` `:572`）
- **proposal の Capabilities と specs/**: `device-collection` / `collection-coverage` の 2 つで一致。`collection-coverage` の前倒しは `INDEX.md:131-144` に理由がある

---

## R1. 「置き場に書けなかった」の報告は、design の作り（64 バイトの数え）では受け手の検査に通らず、しかも端末が捨てないので永久に残る
- 成果物: openspec/changes/st04-offline-retention/design.md（D5）/ specs/device-collection/spec.md / specs/collection-coverage/spec.md
- 根拠: design.md:106 は `write-failed.bin`（64 バイト）に「件数・最初と最後の出来事の時刻」だけを書く。一方 device-collection spec.md:218-221 は、読めなかった行を除くすべての報告に「1 時間ごとの件数」と「終わりが始まりより後の範囲」を求め、collection-coverage spec.md:355-356 と design.md:125 は、`unreadable` 以外で範囲と時間ごとの件数を欠く報告・件数の合計が合わない報告を拒む。1 件だけ書けなかったときは「最初 = 最後」で、R4 と同じく範囲が空になる。拒まれた報告は design.md:197（D13。`dropPermanentlyRejected = false`）で捨てないので、毎回送り直されて一度も格納されない。C5（deep.md:116-118）が残したかった「失ったこと」が稼働記録に届かない
- kind: conflict
- 提案: spec に「書けなかった」の報告が何を持つかを書く（時間ごとの件数を持たせるなら固定の置き場の形を変える。持たせないなら受け手の例外を `unreadable` と同じ扱いに広げ、Scenario を足す）。1 件のときの範囲の終わりも決める
- 処置: fixed D5 仮 — 書けなかった記録は固定長 4 KiB の枠に「ソース × UTC の時間 × 件数」で数え、範囲と `hourly` を持つ報告にする。枠のあふれは範囲と `hourly` を持たない `write_failed` の報告（受け手が `unreadable` と同じく許す）。1 件の範囲は最後の直後で終わる。spec 2 本に SHALL と Scenario 3 本（書けなかった記録の報告は時間ごとの件数を持つ / 1 件でも範囲の終わりは始まりより後 / 受け手の許す条件）。反転条件は D5

## R2. 「破棄の報告は恒久的に断られても端末から取り除かない」が device-collection の SHALL に無く、collection-coverage の理由の文と Scenario が互いに逆を言っている
- 成果物: openspec/changes/st04-offline-retention/specs/device-collection/spec.md / specs/collection-coverage/spec.md
- 根拠: device-collection spec.md:17-19 は恒久的に断られた「記録」を未送信から取り除くとし、:226 は破棄の報告を「記録と同じ未送信の仕組みに乗せて」再送するとしている。取り除かない規則は design.md:197（D13）と、別の capability の理由の文（collection-coverage spec.md:362）にしか無い。さらに collection-coverage spec.md:362-363 は「断る条件は『どう直しても通らない形の不正』に限る」と言うが、:381-383 の Scenario と design.md:125 は、登録簿に無いソースを断る。これは ST03 が「受け手側の設定で変わりうる」（device-collection spec.md:20-21）と分類した理由そのもの
- kind: technical
- 提案: device-collection の「端末から失われた記録を破棄として報告する」に「破棄の報告は、断られた理由を問わず未送信から取り除かない」を足し、Scenario を 1 本置く。collection-coverage の理由の文は「登録簿に無いソースも断る（端末は捨てずに送り直す）」に直す
- 処置: fixed D4 — device-collection に「破棄の報告を、断られた理由を問わず未送信から取り除かない」の SHALL と Scenario「断られた破棄の報告も未送信から取り除かれない」を足した。collection-coverage の理由の文を「形の不正と、登録簿に無いソース（直せば次の送り直しで通る）に限る」に直した

## R3. 観測できる振る舞いが design にだけあり、archive で正典から落ちる（D9 の応答の形・D12 の種類ごとの送信・D6 の退避ファイルを消さない）
- 成果物: openspec/changes/st04-offline-retention/design.md（D9 / D12 / D6）
- 根拠: deep.md:67 は Q3 の効く先に「specs `collection-coverage`（…稼働状況の応答に日ごとの破棄の件数と時刻の範囲）」を挙げたが、collection-coverage spec に `/coverage` の応答についての Requirement は無い。応答の欄・時刻を `Asia/Tokyo` の `HH:MM` で出すこと・日の終わりを `24:00` にすること・区間の端の時間をどちら側で数えるかは design.md:151-161 にしか無い。画面の Scenario（spec.md:328-331「うち 180 件を破棄（10:00〜13:00）」）はタイムゾーンを書いていない。同じく「記録・生存信号・破棄の報告をそれぞれ送る（記録の溜まりが生存信号と報告を待たせない）」は design.md:190、「退避したファイルは上限の対象外で、端末から消さない」は design.md:119 にしか無い
- kind: technical
- 提案: collection-coverage に「稼働状況の応答は日ごとの破棄の件数と、その日で切った時刻の範囲（`Asia/Tokyo`）を持つ」の Requirement と Scenario（日をまたぐ範囲の切り方を含む）を足す。D12 と D6 の上の 2 点は device-collection に SHALL と Scenario を足す
- 処置: fixed D9 — collection-coverage に Requirement「稼働状況の応答は日ごとの破棄の件数と時刻の範囲を持つ」（`Asia/Tokyo`・`24:00`・区間ごとの件数・範囲なしは数えない）と Scenario 3 本を足し、D9 は形だけに絞った。device-collection に「種類ごとに送る」「退避した行を消さない」の SHALL と Scenario を足した

## R4. 再起動をまたぐ 30 日の規則が、spec（超える分だけ数えない）と design（差を丸ごと数えない）で違い、Scenario はどちらの読みでも通る
- 成果物: openspec/changes/st04-offline-retention/specs/device-collection/spec.md / design.md（D2）
- 根拠: spec.md:148「30 日を超える分を 90 日の経過に数えない」（30 日までは数える）と、design.md:59「差が 30 日を超えたら、その差は数えない」（全部数えない）。spec.md:197-200 の Scenario（積んでから 10 日 + 空白 100 日）は、spec の読みで 40 日、design の読みで 10 日となり、どちらでも取り除かれない。**「再起動をまたいでも短い空白は数える」を確かめる Scenario が無い**ので、再起動をまたぐ時間を一切数えない実装でも緑になる（毎日再起動する端末は 90 日の側で捨てられなくなる）。時計が再起動をまたいで戻ったときの扱いも spec に無い
- kind: technical
- 提案: どちらの読みか 1 つに揃える。「積んでから 80 日 + 再起動をまたぐ空白 20 日 → 取り除かれる」のように、30 日以内の空白を数えることを確かめる Scenario を足す。30 日ちょうど前後の境界の Scenario も足す
- 処置: fixed D2 — spec の読み（30 日までは数え、超える分は数えない。戻ったら 0）に design を揃えた。Scenario「再起動をまたぐ 30 日以内の空白は 90 日に数える」「再起動をまたいで時計が戻っても経過は減らない」を足した

## R5. 「溜まっている間は続けて送る」の終わる条件が、断られ続ける未送信で終わらない
- 成果物: openspec/changes/st04-offline-retention/specs/device-collection/spec.md / design.md（D12）
- 根拠: spec.md:8-9 は「1 回に載る件数より多く溜まっている間は続け、件数以下になるか一時的な失敗で終わったら間隔に戻る」。登録簿に無いソースの記録が 201 件以上溜まると、応答は 200 / 400 で 1 件ごとに断られ（一時的な失敗ではない）、未送信に残る（spec.md:20-21）ので、この規則だと 90 日の上限まで間を置かずに送り続ける。design.md:188 の「まだ飛ばしていない未送信」も、`Sender.kt:103-106` は飛ばした分が尽きると `skipped.clear()` で全件を当たり直すので、同じく終わらない。Q6 の反転条件は「復旧後の電池・通信量」（deep.md:98）で、これが一番効く経路。3 本の Scenario（spec.md:100-113）はどれもこの場合を見ていない
- kind: daily
- 提案: 終わる条件に「1 回の送信で未送信から 1 件も取り除けなかったとき」を足すか、1 回の続けての間に各項目を最大 1 回だけ送ると書く。「断られ続ける未送信が 1 回に載る件数を超えて溜まっていても、一定の間隔に戻る」の Scenario を足す
- 処置: fixed D12 仮 — 続けて送るのを止める条件に「1 回の送信で 1 件も取り除けなかった」を足した（spec の SHALL と D12）。Scenario「断られ続ける未送信だけが溜まっていても一定の間隔に戻る」を足した。`skipped.clear()` の経路を D12 に書いた

## R6. Scenario「溜まっている間は間隔を待たずに続けて送る」の「3 回」が、spec 本文とも design とも合わない
- 成果物: openspec/changes/st04-offline-retention/specs/device-collection/spec.md
- 根拠: spec.md:102-103「3 倍の未送信 … 3 回の送信で未送信が 1 回に載る件数以下になる」。600 件は 2 回送ると 200 件で、spec.md:9 の「件数以下になったら間隔に戻る」により、そこで続けるのをやめる。design.md:188 の「200 件を超えて残っていれば次」でも、続けて送るのは 2 回目まで。3 回目は一定の間隔の契機になる。「間隔を待たずに」も、何を測れば真偽が決まるか（2 回の送信の間隔が一定の間隔より短いこと）が書かれていない
- kind: technical
- 提案: THEN を「2 回目の送信が一定の間隔を待たずに行われ、その後に残った 1 回に載る件数ぶんは一定の間隔の契機で送られる」のように、回数と時間の間隔で観測できる形に直す
- 処置: fixed specs/device-collection/spec.md — Scenario を「1 回目の直後に間隔を待たずに 2 回目」と「2 回目の後に残った 1 回ぶんは一定の間隔の契機」の 2 本に分け、回数と時間の間隔で観測できる形にした

## R7. 破棄の報告を分ける条件（ソース × 理由ごと・1 時間を超えて離れる・戻る）が design にだけあり、Scenario「送る前の報告には続けて起きた破棄が足される」と食い違う
- 成果物: openspec/changes/st04-offline-retention/specs/device-collection/spec.md / design.md（D4）
- 根拠: spec.md:269-272 は、送る前なら続けて捨てても「報告は 1 本のまま」と条件なしで言い切る。design.md:92 は「ソース × 理由ごとに 1 本」、:97 は「1 時間を超えて離れる、または戻るなら新しい報告」とする。後者は「過去の写真で何年もの範囲が 1 本になり、格子を丸ごと『破棄』に塗るのを防ぐ」ためで、画面に効く観測できる振る舞い。範囲の終わりが「残った最も古い記録」から 1 時間を超えて離れたときの値（最後 + 1 ms。design.md:95-96）と、その記録が「同じソース」であることも spec.md:222-223 に無い。**報告の時間ごとの件数がすべて範囲の内側にある**という前提も spec に無く、受け手も確かめない（design.md:124-125）。外れると `dropped_count` と `dropped_ranges`（D9）が画面で食い違う
- kind: technical
- 提案: 分ける条件と終わりの規則（同じソース・1 時間を超えたときの値）を SHALL にし、「理由が違う破棄は別の報告になる」「出来事の時刻が 1 時間を超えて離れた破棄は別の報告になる」の Scenario を足す。時間ごとの件数が範囲の外にある報告を受け手が断る行と Scenario も足す
- 処置: fixed D4 — ソース × 理由ごとに分ける / 1 時間を超えて離れる・戻るなら新しい報告 / 終わりは同じソースで残った最も古い記録（1 時間以内）か最後の 1 ms 後 / 時間ごとの件数は範囲の内側、を device-collection の SHALL にし、Scenario 4 本を足した。受け手が範囲の外の時間を断る SHALL と Scenario を collection-coverage に足した

## R8. deep の C3 は範囲の終わりを「最後に捨てた記録の直後」とするが、spec は「残った最も古い記録の時刻」にした。覆したことが deep に記録されていない
- 成果物: openspec/changes/st04-offline-retention/specs/device-collection/spec.md / deep.md
- 根拠: deep.md:112「範囲は半開区間（最初に捨てた記録の時刻から、最後に捨てた記録の直後まで）」。spec.md:222-223 は「破棄せずに残った最も古い記録の出来事の時刻（1 時間以内のとき）」で、最大 1 時間ぶん後ろへ伸びる。これは丸ごと覆うかの判定（C13）に効く（23:30 で最後に捨て、次に残るのが翌 00:20 なら、その日は丸ごと覆われる）。deep.md:191-196「当初案を覆したもの」に、この変更が載っていない
- kind: technical
- 提案: deep.md の C3 に、spec で終わりの規則を変えたことと理由（2 本に割れた範囲をつなぐため）を追記する。成果物の整合だけの話なので、本人に聞く必要は無い
- 処置: fixed deep.md — C3 に ★ で終わりの規則を詰めたことと理由（2 本の範囲をつなぐ）を追記し、「当初案を覆したもの」に「specs で詰めたもの」を足した

## R9. 通知の「未送信の日数」が数える対象に、捨てられない生存信号・破棄の報告が入るかが決まっていない。spec と design で日数を出す条件も違う
- 成果物: openspec/changes/st04-offline-retention/specs/device-collection/spec.md / design.md（D11）
- 根拠: spec.md:297 は「WHILE 送れていない記録がある」あいだ日数を出す。design.md:178 は「1 日以上なら」出し、未満なら出さない。spec.md:297-300 は「未送信の最も古い記録」、design.md:178-180 は「最も古い未送信」と書く。生存信号は捨てない（C1。ST03 D14 で恒久的に断られても残る）ので、それを数えると日数は増え続け、83 日の音が鳴っても何も捨てられない。音はそのまま鳴り直さない（「83 日を下回る」が来ない）
- kind: daily
- 提案: 数える対象を「保持の上限の対象になる記録」と spec に明記し、「断られ続ける生存信号だけが 83 日を超えても鳴らない」の Scenario を足す。1 日未満の表示は spec と design のどちらかに揃える
- 処置: fixed D11 仮 — 数える対象を「保持の上限の対象になる記録」（生存信号と破棄の報告を含まない）と spec に明記し、日数は 1 日以上のときだけ出すに揃えた。Scenario「1 日に満たない未送信では日数が出ない」「生存信号だけが古くても鳴らない」を足した

## R10. 置き場の作りの Requirement（全件をメモリに載せない・全件を書き直さない）が Scenario を持たず、2 GB 側も確かめていない
- 成果物: openspec/changes/st04-offline-retention/specs/device-collection/spec.md
- 根拠: spec.md:327-328 の 2 行は Requirement の本文だけにある。Scenario（spec.md:336-344）は「129,600 件（約 94 MB）で落ちずに起動して送り切る」だけで、エミュレータのヒープに 94 MB が載れば、全件を読む実装でも緑になる（C7 の R13 はヒープを未実測としている。deep.md:121）。本文の「上限に近い量」（spec.md:326）は 2 GB 側も含むが、2 GB に近い量の Scenario は無い。tasks 5.1 の `SegmentStoreTest` は確かめるが、Scenario の印が無いので正典の検証から外れる
- kind: technical
- 提案: 「読み戻しでメモリに載る件数が 1 回に載る件数を超えない」「送れた分を取り除いても、残りの置き場のバイトが書き直されない」の Scenario を足し、tasks 5.1 の印にする
- 処置: fixed 5.1 — Scenario「読み戻しでメモリに載る件数は 1 回に載る件数を超えない」「送れた分を取り除いても残りは書き直されない」「2 GB に近い量でも収集は起動する」を足し、tasks 5.1 / 10.2b の印にした

## R11. 受け手が端末識別子の無い報告や、4 つ以外の理由の報告を断らない。検査の行の一部は Scenario を持たない
- 成果物: openspec/changes/st04-offline-retention/specs/collection-coverage/spec.md / design.md（D7）
- 根拠: C8（deep.md:123-125）は「端末識別子と理由は捨てた時点を過ぎると残らず、後から足せない」とした。collection-coverage spec.md:352-356 は理由の無い報告は断るが、端末識別子の無い報告と理由の値が 4 つ以外の報告は断らない。design.md:124-125 の検査も `device_id` を見ない。書き換えを拒む表（D7）に識別子の無い行が入ると、直せない。本文の「件数が 0 以下を断る」（:353）と、「読めなかった行以外で範囲を欠く報告を断る」（:356 の裏）は Scenario が無い
- kind: technical
- 提案: 「端末識別子を持たない」「理由が 4 つのどれでもない」報告を断る SHALL を足す。件数 0・範囲を欠く `90 日を超えた` の報告を含め、拒否の Scenario を足す
- 処置: fixed D7 — 端末識別子の無い報告・4 つ以外の理由・範囲を欠く報告を断る SHALL と、拒否の Scenario 5 本（端末識別子 / 知らない理由 / 件数 0 / 範囲を欠く 90 日 / 範囲の外の時間）を足した。D7 の検査に `device_id` を足した

## R12. 既存の Scenario「破棄が期間と件数で残る」の印は、ST04 の後は書き手のいない表への直接 INSERT を試験しており、tasks が付け替えない
- 成果物: openspec/changes/st04-offline-retention/tasks.md
- 根拠: `crates/server/src/coverage/tests/spans.rs:45-60` は `core.coverage_span` に `kind='dropped'` を直接 INSERT して読み戻す。design.md:132 は「`coverage_span` の `dropped` は残す（書き手はいない）」とし、破棄の実際の経路は `/drops` →（新しい表）になる。tasks.md:19 は「既存の印をそのまま生かす」とし、この Scenario をどのタスクにも挙げていない。**実際の経路が壊れても、この Scenario は緑のまま**
- kind: technical
- 提案: tasks 2.2 に、この Scenario の印を `/drops` 経由の結合テストへ付け替える（または足す）ことを書く
- 処置: fixed 2.2 — `drop_is_stored_with_count` の Scenario の印を `/drops` 経由の結合テストにも置くことを tasks 2.2 に書き、印が spans.rs 以外にもあることを grep で確かめる検証を足した

## R13. tasks の検証の一部が、新しいテストが 0 本でも rc=0 になるか、実物と合わない
- 成果物: openspec/changes/st04-offline-retention/tasks.md
- 根拠: 10.1 / 10.2 の `./tools/android-emulator.sh` は `connectedDebugAndroidTest` を全部回すだけ（`tools/android-emulator.sh:46`）で、`RetentionInstrumentedTest` が無くても rc=0。4.1 / 4.2 の `cd web && npm run test`（`vitest run`）も同じ。4.2 の「`one-scroll.test.tsx` が変更なしで通る」は、変更なしを判定するコマンドが無い。10.4 の `jq -e '.… .dropped_count == 180'` は `…` が埋まっておらずコマンドになっていない。さらに `tools/seed.sh` が登録簿に入れるのは `seed-location`（`tools/seed.sh:25`）で、`c01-location` を入れるのは `smoke.sh:130` だけなので、seed だけで `c01-location` の報告を送ると登録簿に無いソースとして断られる（collection-coverage spec.md:381-383）
- kind: technical
- 提案: 10.1 / 10.2 に `RetentionInstrumentedTest` が走ったことの確認（`tools/android-emulator.sh:47` が示す `collector-android/app/build/reports/androidTests/connected/` か、結果の XML にそのクラスがあり失敗が 0）、4.x に `vitest run <ファイル>` の件数の grep、4.2 に `git diff --exit-code web/src/__tests__/one-scroll.test.tsx` を足す。10.4 は jq の式を埋め、登録簿にソースを入れる段を足す
- 処置: fixed 10.4 — 計測テストは `AT <クラス>`（結果の XML にそのクラスがあり失敗 0）、画面は `VT <ファイル>`（件数の grep）を 0 章に定義して 4.x / 10.x を置き換えた。4.2 に `git diff --exit-code` を足した。10.4 の jq の式を埋めた。**登録簿の件は事実が違った** —— `c01-location` は移行 `202609111111_coverage_rebuild.sql` が登録簿に入れている（`seed.sh` が入れなくても断られない）。tasks 10.4 にその出所を書いた

## R14. satisfies に無い FR-10 / FR-54 / NFR-1 を specs が改訂・担保しているが、INDEX の訂正の節は capability の割り当てしか書いていない
- 成果物: docs/stories/ST04.md / docs/stories/INDEX.md
- 根拠: `ST04.md:4` は `satisfies: [FR-8, FR-9, NFR-7]`。specs は FR-10 ★（Q6。device-collection spec.md:8-9）と FR-54 ★（Q3。collection-coverage spec.md:172-177）を満たし、NFR-1 の Scenario を足している（device-collection spec.md:129-132）。`INDEX.md:131-144` は `collection-coverage` を足した理由を書くが、要件の側は書いていない。ST03 / ST16 の訂正の節は「本体の Story / この Story が満たす条項」の表で書いている（`INDEX.md:112` ほか）
- kind: technical
- 提案: INDEX.md の 2026-09-14 の訂正の節に、同じ形の表（FR-10 本体 ST01 / ST04 が満たす条項「溜まっている間は続けて送る」、FR-54 本体 ST02 / 「破棄の印と週の詳細の文字」、NFR-1 本体 ST01 / 「溜まった分を送る間も 1 時間」）を足す
- 処置: fixed proposal.md — `docs/stories/INDEX.md` の 2026-09-14 の訂正の節に、ST03 / ST16 と同じ形の表（FR-10 本体 ST01 / FR-54 本体 ST02 / NFR-1 本体 ST01 と、ST04 が満たす条項）を足した。proposal の Impact の要件の行から参照する

## R15. AND で 2 つの主張を束ねた Scenario が 6 本あり、片方だけを確かめたテストでも印が付く
- 成果物: openspec/changes/st04-offline-retention/specs/device-collection/spec.md / specs/collection-coverage/spec.md
- 根拠: device-collection spec.md:207-211（全部が届く AND 報告が作られない）、:284-288（退避される AND 報告が積まれる）、:313-317（1 回鳴る AND 84・85 日で鳴らない）、:264-267（最初の報告が変わらない、後の破棄は新しい報告）、:202-205（生存信号 と 破棄の報告の 2 つが対象）。collection-coverage spec.md:318-321（丸ごと覆う日 と 破棄の無い日の 2 つの前提）
- kind: technical
- 提案: 独立に壊れうるもの（「1 時間の圏外では破棄の報告が作られない」「84 日で再び鳴らない」「丸ごと覆う破棄の日に印が付かない」）を別の Scenario に分ける
- 処置: fixed specs/device-collection/spec.md — 独立に壊れうる主張を分けた: 1 時間の圏外（届く / 報告が作られない）、読めない行（退避される / 件数が報告される）、通知（鳴る / 鳴り直さない）、書き換え（1 バイトも変わらない / 新しい報告になる）、捨てないもの（生存信号 / 破棄の報告）、印（丸ごとの日 / 破棄の無い日。collection-coverage）

## R16. 破棄の報告の「原文をそのまま残す」が、何の文字列をどう比べれば真偽が決まるかを書いていない
- 成果物: openspec/changes/st04-offline-retention/specs/collection-coverage/spec.md
- 根拠: spec.md:405 と :424-427「端末から受け取った原文がそのまま残り」。design.md:88 では、原文は要求の本文ではなく、報告の中の `raw` 欄（「上の欄を組んだ JSON の文字列」）で、冪等キーもそこから作る（design.md:134）。spec からは、要求の本文の 1 要素を指すのか `raw` 欄を指すのかも、比べ方（バイト一致か、JSON として等しいか）も決まらない。`raw` と他の欄が食い違う報告の扱いも無い
- kind: technical
- 提案: 「端末が報告に載せた原文の文字列が、1 バイトも変わらずに読み戻せる」と比べ方まで書く。原文と他の欄が食い違う報告を断るかどうかを 1 行足す
- 処置: fixed D7 — 「端末が報告に載せた原文の文字列を 1 バイトも変えずに読み戻せる」と比べ方を書き、Scenario をバイト一致で観測できる形（キーの順序・空白・数値の表記を含む原文）にした。原文と欄の食い違いは受け付けの条件にしない（生存信号と同じ。欄を集計に使い、原文は証拠）と SHALL にした

## R17. specs の理由の文に実装の名前（`muted`、`OutOfMemoryError`）が入った
- 成果物: openspec/changes/st04-offline-retention/specs/collection-coverage/spec.md / specs/device-collection/spec.md
- 根拠: collection-coverage spec.md:201「`muted` の明るさ」（`web/src/tokens.ts:41` の `TEXT.muted`）、device-collection spec.md:333「読み戻しの `OutOfMemoryError`」。どちらも SHALL ではなく理由の文だが、archive で正典に入る
- kind: technical
- 提案: 「文字の控えめな明るさ（72）」「読み戻しでメモリが尽きると」のように、振る舞いの言葉に置き換える
- 処置: fixed specs/collection-coverage/spec.md — 「`muted` の明るさ」を「控えめな文字の明るさ」、「読み戻しの `OutOfMemoryError`」を「読み戻しでメモリが尽きたときの失敗」に置き換えた

---

観点 1（deep の決定が正典に写っているか）: Q1 / Q3 / Q4 / Q5 / Q6 の答えは、それぞれ対応する Scenario がある（device-collection spec.md:182-185, 187-190, 313-322, 100-113、collection-coverage spec.md:313-341）。数値（90 日・2 GB・83 日・右下の三角・3:1・180 件・10:00〜13:00）も Scenario にある。要件へ戻すもの 5 件は ★ つきで入っている。ずれは R1（C5）/ R4（C10）/ R8（C3）/ R3（Q3 の効く先の応答）/ R11（C8）に書いた。
観点 2: R6 / R10 / R11 / R15 / R16 に書いた。
観点 3: R3 / R17 に書いた。proposal と specs/ と INDEX の capability は一致。
観点 4: 依存順（移行 → 受け口 → 稼働状況 → 画面、置き場 → 経過の数え方 → 上限 → 報告 → 送信 → 知らせ → 完了の判定）は、鍵に当たる経過の数え方（6.1）が上限（6.2）より前にあり、問題なし。人間の確認待ちは「違和感」1 問で、2026-09-14 の決定に沿っている。指摘は R12 / R13。
観点 5: check_chain の観点 8（再生成との一致）は OK。ST04.md の「価値」「完了の判定」3 行は deep の決定（捨てる・知らせる・送り切る）と矛盾しない。指摘は R14。

## 処置のまとめ（呼び出し元が付けた）

17 件すべてに処置を付けた。**人間へ返したもの（escalated）は 0 件** —— `loss` の付いた指摘は無く、conflict / daily の 3 件は仮決めで閉じた。

| 処置 | 件数 | 指摘 |
|---|---|---|
| 仮決め（design に（仮）と反転条件） | 3 | R1（D5）/ R5（D12）/ R9（D11） |
| spec / design / tasks / deep / INDEX を直した | 14 | R2 R3 R4 R6 R7 R8 R10 R11 R12 R13 R14 R15 R16 R17 |

新しく足した Scenario は 33 本（48 → 81 本）。R13 のうち「`seed.sh` は `c01-location` を登録簿に入れない」は、移行が入れているので事実が違った（10.4 に出所を書いた）。

