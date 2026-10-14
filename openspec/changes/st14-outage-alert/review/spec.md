# ST14 spec レビュー（独立）

対象: `openspec/changes/st14-outage-alert/` の proposal.md / specs/collection-coverage/spec.md / design.md / tasks.md、`docs/stories/ST14.md`、`docs/handoff/ST47.md`。
突き合わせの正本: deep.md（Q1〜Q9、C1〜C13、「整合の確認から spec の段へ渡すもの」）、`docs/requirements.md`（FR-35 / FR-54 / FR-79 / NFR-13 / NFR-17）、
`docs/stories/INDEX.md`、`openspec/specs/collection-coverage/spec.md`（正典）、`openspec/specs/external-ingestion/spec.md`、`docs/handoff/ST02.md` / `ST14.md`、
既存コード（`crates/server` / `crates/collector-windows` / `collector-android` / `web/src` / `migrations`）、並走中の change（st08 / st17 / st21 / st25）と `feat/st08-browser-history` の差分。**成果物は触っていない。**

実施日: 2026-10-13。ブランチ `docs/st14-upstream`（HEAD f34d0246）。

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate st14-outage-alert --strict` | `Change 'st14-outage-alert' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0。観点 8 の再生成との一致を含む） |
| `python3 scripts/check_scenarios.py . st14-outage-alert` | `scenarios: FAIL (担保なし 88 件 / 名無しの確認待ち 0 件)`（rc=1）。88 件はこの change が足した Scenario（ADDED 77 本と、MODIFIED に足した 11 本）で、上流では想定どおり（tasks の Global Constraints が同じ数を書いている）。warn 8 件は他の change の印 |
| `python3 scripts/review_triage.py . st14-outage-alert` | `triage: OK`（rc=0。このファイルを書く前の状態。review/deep.md の 8 件） |

手で確かめたこと:

- **MODIFIED 3 本と正典の全文の突き合わせ**（`diff` で取った）: 「ソース × 日 の状態は 8 つのいずれかに決まる」「ソースごとに収集を開始した日が残る」「成功条件 1 の達成日はソースの主語で分けて数える」は、正典の本文・理由・Scenario（11 / 11 / 16 本）を 1 本も落とさずに写し、足した行だけが違う。正典の Scenario 名も変えていない
- **新しい Scenario 88 本は、どれも tasks.md の Task が名指ししている**（名前を逐語で突き合わせた。名指しの漏れ 0、名前の重複 0）
- **deep の答え → Scenario**: Q1（両方の収集アプリ・外の push 無し）/ Q2 / Q3（端末の時刻・最後の書庫）/ Q4 / Q5 / Q6 / Q7 / Q8 / Q9（75 日。数値も Scenario にある）は、どれも Scenario がある。C1〜C7 / C9〜C12 も Scenario か Requirement の本文に写っている。C8 は R8、C13 は画面の構造を変えないので Scenario は要らない
- **要件へ戻すもの**: FR-35 / FR-54 / FR-79 に ★ 2026-10-13 がある（`docs/requirements.md:412` `:656` `:488`）。Story の完了の判定に Q9 / Q2 の 2 行が足されている（`docs/stories/ST14.md:47-48`）。ST14.md の FR-35 の引用は要件の現在の本文と一致する
- **proposal の Capabilities と specs/**: `collection-coverage` の 1 つで一致。INDEX の割当（`INDEX.md:84`。収集の稼働状況・通知・停止）とも一致。FR-54 / FR-79 / NFR-17 / NFR-13 を満たす条項は INDEX に理由がある（`INDEX.md:219-227`）
- **tasks が名指しする道具・ファイル・関数**: `tools/{check-log-private,usage-volume,check-openapi,check-migrations,check-immutable,smoke,seed}.sh`、`web/e2e/{archive-layout,surface}.spec.ts`、
  `coverage/tests/{started_on,achievement}.rs`、`testdb::source`（`testdb.rs:130`）、crate `ashiato-server` / `ashiato-collector-windows`、`DEVICE_SUBJECT`（`coverage.rs:28`）、`touch_started_on`（:240）、`facts`（:606）、`active_days`（:889）、`decide`（:940）、
  `archive_ledger_source`（`migrations/202609181600_archive_ingestion.sql:53`）、通知 ID 1・2（`LocationService.kt:912,916`）は**すべて実在**した。食い違いは R12 / R13 / R14
- **design の「いまの形」の主張**: 重複した記録でも `archive_ledger_source` の行が立つ（`worker.rs:290-329`。`Duplicate` を数えて行を足す）、`authorize()` は Bearer と cookie の両方を通す（`lib.rs:515-541`）、`GET /coverage` は配列を返す（`lib.rs:2381`）は合っている

---

## R1. ST02 への申し送り R38（PC を開かなかった日に①が立ちうる）を読まずに、格子で離席・止まっていた期間を①に数える側を決めている。理由の文は事実と合わず、同じ「観測ではない記録」という言葉が 2 つの Requirement で別の範囲を指す
- 成果物: openspec/changes/st14-outage-alert/specs/collection-coverage/spec.md（「観測ではない記録は稼働状況の記録として数えない」と MODIFIED「ソース × 日」）/ deep.md
- 根拠: docs/handoff/ST02.md:52-104（st07-active-window R10「PC の 2 ソースの分母」と R38「`idle` / `powered-off` / `excluded` / `clock-skew` が①を立てる」。R38 は「R10 と同じ change で」「達成日数が動くので本人に問う（premise）」と書き、:82-88 は R10 を R-e / R-k と「3 件まとめて見る」と書く）/
  ST14 は R10 を「ST07 の NFR-13 の但し書き」として実装する（spec.md:872、design D17）のに、`docs/handoff/ST02.md` は deep.md・proposal・design のどこからも引かれていない（`grep -n "R10\|R38\|handoff/ST02"` で 0 件）/
  spec.md:429「離席・止まっていた期間・除外の印は、PC がその日に前景を読めていた証拠でもある」—— 離席は入力の経過時間（FR-81。`contract.rs:27-28`、`docs/collector-contract.md:345-347`）から、止まっていた期間は起動の印（FR-82）から作られ、前景を読めるかどうかに依らない。前景が読めなくなった PC でも離席の出入りは毎日書かれる /
  spec.md:67-69 は離席・止まっていた期間・除外の印を「観測ではない記録」と呼び、spec.md:419-421 と :591 は同じ言葉で `gap` と `clock-skew` の 2 つだけを指す
- kind: conflict
- 提案: deep.md の「この Story に宛てられたもの」に ST02 の R10 / R38 を足し、R38 を本人に問う（B。格子は引き直せる）か、C9 の格子の除外に `idle` / `powered-off` を足すかを決める。どちらにしても spec.md:429 の理由の文は直す。2 つの集合には別の名前を付ける（例: 格子で数えない記録 / 途切れで数えない記録）
- 処置: fixed D22 仮 —— c02-window の達成日は生存信号で数える（`coverage.rs:1123-1126`）ので R38 の「達成日数が動く」は当たらず、格子の見え方だけの規則として D22（仮）に反転条件つきで置いた。spec の理由の文を「PC の収集がその日に動いていた証拠」に直し、集合の名前を「格子で数えない記録」と「途切れの判定で観測ではないとする記録（FR-35 の列挙）」に分けた。deep.md の宛てられたもの表に ST02 の R10 / R38 を足した

## R2. 「本人が書いた・導出した記録を 1 件でも持つソース」を外す規則では、収集のソースに 1 件の `authored` が入っただけで、そのソースが二度と鳴らなくなる
- 成果物: openspec/changes/st14-outage-alert/specs/collection-coverage/spec.md:8（Requirement「途切れの判定の対象は登録簿の印と記録の出どころで決まる」の (2)）と Scenario「本人が書いた記録のソースは印が無くても途切れにならない」（:32-35）/ design D3
- 根拠: crates/server/src/ingest.rs:13,505-516（`/ingest` はどの論理ソースにも `authored` / `derived` を端末識別子なしで受け付ける。出どころとソースの組を縛るのは `s01-attribute` だけ。`lib.rs:879-893`）/ 記録は書き換えも削除もできない（envelope の錠）ので、1 件入れば外れたままになる / deep.md:53 C1 の理由「印を付け忘れた新しい収集のソースが黙って外れない（既定は厳しい側）」/ FR-25 ★ 2026-10-11（衛星が本人の代わりに `authored` で書く）
- kind: technical
- 提案: 外す条件を「収集した記録を 1 件も持たず、本人が書いた・導出した記録を持つソース」にする（記録の無いソースは (4) で外れているので、厳しい側の既定は変わらない）。それに合わせて Scenario を「収集した記録と `authored` の記録が混ざったソースは、途切れとして判定される」の形で 1 本足す
- 処置: fixed D3 —— 外すのは「収集した記録を 1 件も持たず、本人が書いた・導出した記録を持つソース」だけにした。Scenario `収集した記録と本人が書いた記録が混ざったソースは判定の対象のまま` を足した。tasks 5.1

## R3. 携帯の問い合わせを「生存信号のスケジューラの emit() の後に 1 行」で足すと、ソースの数だけ問い合わせが走り、同じ途切れで音が 2 回以上鳴りうる
- 成果物: openspec/changes/st14-outage-alert/design.md D12（:184）/ tasks.md 8.3 / spec Scenario「携帯は生存信号の契機で問い合わせる」
- 根拠: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt:795-819（`for (running in sources)` で**ソースごとに** `newHeartbeatScheduler(...)` を作り、ソースごとに `emitter.emit()` を呼ぶ。起動時の 1 発もソースごと）/ D12「起動時の 1 発の後にも呼ぶ」/ D13 の決め方は `SharedPreferences` の `sounded` を読んでから書く。位置・アプリ利用・写真・集計の各スケジューラの `poll()` が並行に走ると、どれも「まだ鳴らしていない」を読んで鳴らす
- kind: technical
- 提案: 問い合わせは端末で 1 本のスケジューラにする（位置の生存信号のスケジューラだけに足す、または 6 時間の専用の契機を 1 つ持つ）。`decide` → 通知 → `sounded` の書き込みは端末内で直列にする。Scenario の THEN「1 回送られる」が、ソースが複数ある構成で試されるよう tasks 8.3 に書く
- 処置: fixed D12 —— 問い合わせは位置の生存信号のスケジューラだけに足し、poll → 決め方 → 通知 → 覚えの書き込みを単一スレッドの executor で直列にした。spec の SHALL を「端末で 1 回だけ」にし、Scenario `ソースが複数あっても携帯の問い合わせは契機ごとに 1 回` を足した。tasks 8.3

## R4. PC の差出人の登録に `windows` crate の `Win32_System_Registry` を使うと unsafe が要り、作業場の `unsafe_code = "forbid"` に当たる
- 成果物: openspec/changes/st14-outage-alert/design.md D11（:172-173）/ tasks.md 9.3
- 根拠: Cargo.toml:27（`unsafe_code = "forbid"`）/ crates/collector-windows/Cargo.toml:25-27（「unsafe を書かずに安全な wrapper に委ねる（design D15）」）/ `windows` crate の Win32 の関数（`RegCreateKeyExW` / `RegSetValueExW`）は `unsafe fn`
- kind: technical
- 提案: 登録簿への書き込みは unsafe を持たない wrapper（例: `winreg` crate）で行うと D11 に書く。WinRT の `UI_Notifications` の呼び出しは safe なので残せる。Cargo.toml の注記（全機能の `windows` crate でコンパイルが OOM した）に合わせて、足す機能を D11 の 3 つに絞ることも tasks 9.3 に書く
- 処置: fixed D11 —— 登録簿の書き込みは `winreg`（unsafe を書かない）にし、`windows` crate の機能を `UI_Notifications`・`Data_Xml_Dom`・`Foundation` の 3 つに絞った。tasks 9.3

## R5. Windows では、音なしのトーストでも差し替えるたびに画面の端に通知が出る。PC は 1 時間ごとに問い合わせるので、途切れが続く間は毎時通知が出る
- 成果物: openspec/changes/st14-outage-alert/design.md D11（:174「差し替えは同じ Tag / Group で出し直す」「音を鳴らさないときは `<audio silent="true"/>`」）/ spec Scenario「続く途切れでは音を鳴らさずに通知が残る」（:351-354）
- 根拠: Windows のトーストは同じ Tag / Group で出し直すと通知センターの中身を置き換えるが、`ToastNotification.SuppressPopup` を立てない限り画面の端の表示も毎回出る。`audio silent` が止めるのは音だけ / deep.md:93 Q4「続く間は音の無い通知を残し」/ C11（PC は 1 時間ごと）/ FR-35「鳴りすぎる通知は本人に切られ、切られた通知は 1 年の途中で気づく手段を失わせる」
- kind: daily
- 提案: 鳴らさない差し替えでは `SuppressPopup = true`（通知センターにだけ置く）と D11 に書く。Scenario の THEN に「画面の端に表示されない」を足すか、PC 用の Scenario を 1 本足す（実行時テストで `SuppressPopup` の値を読める）
- 処置: fixed D25 仮 —— 音なしの差し替えは `SuppressPopup = true` で通知の一覧だけを替える（D11 にも書いた）。spec の SHALL と Scenario `PC の音なしの差し替えは画面の端に出ない` を足した。tasks 9.3

## R6. 「鳴らした途切れを再起動しても覚えている」は design にしか無い。Q4 の「途切れ 1 回につき音は 1 回」を守る肝心の振る舞いなのに、Scenario が無い
- 成果物: openspec/changes/st14-outage-alert/design.md D11（:176「起動し直しても同じ途切れで鳴らない」）・D12（:186 `SharedPreferences`）/ spec Requirement「携帯と PC の収集アプリが途切れを OS の通知で知らせる」
- 根拠: spec.md:325-415 に、収集アプリを起動し直した後の Scenario が無い / Android は前景サービスでもプロセスを落とし、`START_STICKY` で起動し直す（`LocationService.kt:808-812` のコメント）。覚えを記憶の中だけに置く実装でも、spec の Scenario はすべて通る / deep.md:93 Q4
- kind: technical
- 提案: Scenario「収集アプリを起動し直しても、同じ途切れでは音が鳴らない」（携帯と PC）を足し、tasks 8.1 / 9.1 に名指しする
- 処置: fixed 8.1 —— spec に SHALL「音を鳴らした途切れを起動し直しても覚える」と Scenario `収集アプリを起動し直しても同じ途切れでは鳴らない` を足し、tasks 8.1 / 9.1 が名指しする

## R7. 正典の「記録も生存信号も来ない期間は途絶として残る」を MODIFIED にしていないので、archive 後に「時計の測定しか無い日は途絶になりうる」と食い違う
- 成果物: openspec/changes/st14-outage-alert/specs/collection-coverage/spec.md（MODIFIED の一覧）
- 根拠: openspec/specs/collection-coverage/spec.md:296-301（「想定間隔を超えて**記録も**生存信号も届かない THEN 途絶」。記録の種類を問わない）/ 変更後の spec.md:421（「途絶の判定の前後の記録」から観測ではない記録を除く）と Scenario「時計の測定しか無く生存信号も無い日は途絶になりうる」（:438-441。時計の測定の記録は毎時届いているのに途絶）/ `openspec archive` は MODIFIED に挙げた Requirement の本文しか置き換えない
- kind: conflict
- 提案: 「記録も生存信号も来ない期間は途絶として残る」を MODIFIED に加え、「記録」を「観測の記録（『観測ではない記録は稼働状況の記録として数えない』）」に直す。正典の Scenario 6 本はそのまま写す
- 処置: fixed D22 仮 —— 「記録も生存信号も来ない期間は途絶として残る」を MODIFIED に加え（正典の Scenario 6 本をそのまま写した）、記録を「観測の記録」に直した。Scenario `時計の測定だけが届く期間も途絶になる` を足し、tasks 2.1 が名指しする。格子で数えない記録の範囲が①と⑥の両方に効くことを D22（仮）に書いた

## R8. 観測できる振る舞いが design だけに置かれている。C8 は spec のどこにも無い
- 成果物: openspec/changes/st14-outage-alert/design.md / specs/collection-coverage/spec.md
- 根拠: design の行と、spec に無い振る舞い ——
  - D4（:100）: 途切れの判定は**削除済みの記録も**「最後に生きていた時刻」に数える
  - D4（:99）: 引き継ぎ先のソースは、1 件も届かないうちから鎖の根の収集開始日で「開始済み」と扱われ、届かなければ鳴る。spec の (4)「収集開始日を持たないソース」は、引き継いだ開始日をどう扱うかを言っていない
  - D5（:105）: 判定を待つ 30 分の間は台帳に書かない
  - D6（:123-124）: `device_id` は省略でき、省くと「届けた」を残さない / 一覧は表示名の昇順
  - D11（:173）・D12（:187）: 通知を出せない（差出人を登録できない・`POST_NOTIFICATIONS` が無い）ときも収集は止めない
  - deep.md:60 C8「破棄の報告（`/drops`）では鳴らさない」—— design の対応表（:264）は「D1 で材料に入れない」とするだけで、spec に Requirement の文も Scenario も無い
  - `openspec archive` は main specs しか更新しない（D の文は正典から落ちる）
- kind: technical
- 提案: 上の振る舞いを該当する Requirement の SHALL と Scenario に移す（特に削除済みの記録・引き継ぎ・C8）。D14（仮）の文言は仮のまま D に残してよいが、「押しても何も開かない」は Scenario にする
- 処置: fixed 5.2 —— 削除済みの記録・引き継ぎ（下限は後継の登録時刻）・破棄の報告（C8）・待つ間は台帳に書かない・端末識別子の省略・一覧の順・通知を出せないときも収集を続ける、をそれぞれ spec の SHALL と Scenario に移した。文面（押しても開かない）は反転条件を持つ仮決めなので design の D14 に残した。tasks 5.1 / 5.2 / 7.1 / 7.2 / 8.2 / 9.1

## R9. Requirement の本文にあって Scenario に無いもの、Requirement が決めていない場合
- 成果物: openspec/changes/st14-outage-alert/specs/collection-coverage/spec.md（Requirement「書庫のソースは最後の書庫から登録簿の日数で途切れとする」ほか）
- 根拠:
  - :157「書庫の作られた時刻を持たない書庫は、置き場で見つけた時刻で数える」—— Scenario が無い（`core.archive_ledger.created_at` は NULL を許す。`migrations/202609181600_archive_ingestion.sql:27-28`）
  - 記録はあるのに「そのソースを運んだ書庫」が 1 件も無い書庫のソースをどう判定するか（すぐ鳴る / 鳴らない）が決まっていない。台帳のソース別の行を残せないと、記録は格納済みのまま書き込みを諦める経路がある（`worker.rs:2240-2252`）
  - 「運んだ」に、その製品のファイルはあったが項目が 0 件だった書庫が入るかが決まっていない。ソース別の行は格納を試みた記録からしか作られない（`worker.rs:286-310`）ので、項目が 0 件なら行は立たず、「最後の書庫」は進まない（Q3 が避けようとした「使わなかっただけで鳴る」形）
  - Scenario「書庫のソースは 75 日で登録されている」（:205-209）は登録簿を読むだけで、取り込み器が実行時に足す製品のソース（`ensure_myactivity_source`）は、tasks 1.2 の括弧書き（「実行時に足すマイアクティビティの製品の行も含めて見る」）でしか試されない
- kind: technical
- 提案: 作られた時刻が NULL の書庫、運んだ書庫が無いソース、項目が 0 件のファイル、実行時に足されたソースについて、Scenario を 1 本ずつ足す（無い場合の扱いは厳しい側（鳴らす）を既定にし、D に書く）
- 処置: fixed D24 仮 —— 作られた時刻の無い書庫・運んだ書庫が台帳に無いソース（届いた時刻で測る）・項目が 0 件の書庫（数えない。D24（仮））・実行時に足された製品のソース、に SHALL と Scenario を 1 本ずつ足した。tasks 1.2 / 6.1

## R10. AND で 2 つ以上の主張を束ねた Scenario がある（片方だけ通っても緑になる）
- 成果物: openspec/changes/st14-outage-alert/specs/collection-coverage/spec.md
- 根拠: 「時計の測定と集計と滞在と個人属性は対象外で登録されている」（:42-46。対象外 4 本と、対象 7 種類以上）/ 「使わなかった製品は格子で途絶でも途切れにならない」（:194-198。途切れの判定と格子の状態という別の仕組み）/ 「書庫のソースは 75 日で登録されている」（:205-209）/ 「途切れたソースの表示名と最後に生きていた時刻が返る」（:233-237）/ 「戻ると台帳に戻りが 1 行残る」（:299-303。台帳と応答）/ 「同時に途切れたソースは 1 通にまとまる」（:356-360。1 通と音 1 回）
- kind: technical
- 提案: 少なくとも :194-198（判定と格子）と :299-303（台帳と応答）は 2 本に割る。残りは、THEN の各主張を試験が別々に assert することを tasks の該当行に書く
- 処置: fixed 6.2 —— 「使わなかった製品は…」を判定と格子の 2 本に、「戻ると台帳に戻りが 1 行残る」を台帳と応答の 2 本に割った。残りは tasks の Global Constraints に「主張ごとに別々に assert する」と書いた

## R11. 取りこぼしの期間の Scenario の時刻にタイムゾーンが無く、境界の Scenario の真偽が読み方で変わる
- 成果物: openspec/changes/st14-outage-alert/specs/collection-coverage/spec.md（:475-504）
- 根拠: 「終わりの時刻ちょうどの日には重ならない」（:491-494）の `2026-09-03 00:00` は、`Asia/Tokyo` なら 09-03 に重ならないが、UTC（= 09:00 JST）なら重なる。取りこぼしの期間の `begin` / `end` は端末の RFC 3339 で届く（`docs/collector-contract.md:199-225`）。Requirement（:461）は日の判定を `Asia/Tokyo` とするが、Scenario の時刻がどのタイムゾーンかは書いていない
- kind: technical
- 提案: :477 / :482 / :493 の時刻に `+09:00` を付ける（例: `2026-09-03T00:00+09:00`）。境界の Scenario は、UTC の真夜中で終わる期間についても 1 本足す
- 処置: fixed 2.2 —— 取りこぼしの Scenario の時刻に `+09:00` を付け、UTC の真夜中に終わる期間の Scenario `UTC の真夜中に終わる取りこぼしは日本時間のその日に重なる` を足した

## R12. Task 2.3 / 3.3 / 4.2 の検証の絞り込みでは、「緑のまま通る」と書いた正典の Scenario の試験の大半が走らない
- 成果物: openspec/changes/st14-outage-alert/tasks.md:72-73, :85, :96
- 根拠: `CT coverage_states` は `coverage/tests/states.rs:17` の 1 本にしか当たらない。`ソース × 日` の 11 本の印は `states.rs:72,73,112,815`・`retired.rs:46,47`・`drops.rs:26,61` にもある /
  `CT started_on` は `coverage/tests/started_on.rs` の 7 本だけ。`開始日` の印は `api_tests.rs:146,147,185`・`states.rs:13,279`・`achievement.rs:344,489`・`retired.rs:11` にもある /
  `CT achievement` は `retired.rs:89,124,184`・`states.rs:302` の `達成` の印に当たらない（印の場所は `grep -rn "Scenario: <名前>"` で確かめた）
- kind: technical
- 提案: 絞り込みを `CT coverage::tests::` と `CT api_tests` にする（どちらも 1 本以上）。tasks の「11 本」「16 本」を言うなら、印のあるファイルを列挙する
- 処置: fixed 2.3 —— 2.3 / 3.3 / 4.2 の絞り込みを `CT coverage::tests::` と `CT api_tests` にした

## R13. Task 10.1 の検証コマンドに `...` があり、そのままでは走らない
- 成果物: openspec/changes/st14-outage-alert/tasks.md:186
- 根拠: `bash -c '... | jq -e ".[] | select(.logical_source==\"c01-app-usage-rollup\")"'` —— `curl` の宛先・資格情報（`authorize()` が 401 を返す。`lib.rs:515-541`）・期間の引数が書かれていない。「`gaps` を持つ日があること」は jq の式に入っていない
- kind: technical
- 提案: st25 の tasks.md:124 と同じ形で、`curl -sf -H "authorization: Bearer ${API_TOKEN:-…}" "http://127.0.0.1:18787/coverage?from=…&to=…"` と、集計の行と `gaps` が空でない日の両方を見る jq の式を書く
- 処置: fixed 10.1 —— `curl`（Bearer・期間）と、集計の行と `gaps` が空でない日の両方を見る `jq` の式を書いた

## R14. Task 9.2 / 11.3 の clippy は「CI と同じ」と書いてあるが、CI と違って `--all-targets` が無い
- 成果物: openspec/changes/st14-outage-alert/tasks.md:174, :204
- 根拠: .github/workflows/ci.yml:100（`cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings`）・:73（`cargo clippy --workspace --all-targets -- -D warnings`）/ tasks の 2 か所は `--all-targets` を持たないので、新しい試験（`tests/runtime_windows.rs` の追加分・`*_tests.rs`）の lint が手元で通っても CI で落ちうる
- kind: technical
- 提案: 2 つのコマンドに `--all-targets` を足す
- 処置: fixed 9.2 —— 9.2 と 11.3 の clippy に `--all-targets` を足した

## R15. proposal の Impact は ST08 との重なりを `runtime.rs` だけとしているが、ST08 の下流は collector-windows の 20 ファイルを変えていて、ST14 が触るファイルとさらに 4 つ重なる
- 成果物: openspec/changes/st14-outage-alert/proposal.md:94-96
- 根拠: `git diff main...feat/st08-browser-history --stat -- crates/collector-windows`: `Cargo.toml`（ST08 は `rusqlite` を足す。ST14 は `windows` を足す）・`src/lib.rs`（モジュールの宣言。ST14 は `outage_notice` / `toast` を足す）・`src/contract.rs`（ST14 は端末識別子を読む。D6）・`src/sender.rs`（ST14 の問い合わせは `HttpTransport` を使う）・`tests/runtime_windows.rs`（ST14 の Task 9.3 が足す）・`src/runtime.rs`（+1,645 行。`maybe_measure_skew` の呼び出しの周りも動いている）
- kind: defer —— 他 Story の担当（ST08。差し戻しではなく、後から merge する側の追従の見積り）
- 提案: Impact の重なりを上のファイルに直し、「後から merge する側が rebase で追従する」範囲を書き直す。Task 9 の前置きに、ST08 が先に merge されていれば `runtime.rs` の契機の場所を ST08 の後の形で読み直すと書く
- 処置: fixed proposal.md —— Impact の ST08 との重なりを 6 ファイルに直し、design D11 に ST08 が先に merge されたときの読み直しを書いた（ST08 には差し戻さない）

## R16. 取りこぼしの理由の文言が、並走中の ST25 の 1 日の画面と違う
- 成果物: openspec/changes/st14-outage-alert/specs/collection-coverage/spec.md:462-465 / design D18
- 根拠: openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:354-355（「スマホ 取得元に残っていない 始まり – 終わり」「スマホ 時計のずれで取れていない 始まり – 終わり」）/ ST14 は「取得元に残っていなかった（期間）」「時計のずれで問い合わせなかった（期間）」で、期間の区切りも `〜` と ` – ` で違う。同じ取りこぼしが S-1 と 1 日の画面で別の言葉になる
- kind: daily
- 提案: どちらかに揃えるか、言葉を分けることを D（仮）に書く（ST25 には差し戻さない。ST14 の文言は `messages.ts` の 1 か所で変えられる）
- 処置: fixed D23 仮 —— 文言を ST25 の 1 日の画面にそろえた（「取得元に残っていない」「時計のずれで取れていない」）。区切りは S-1 の破棄と同じ `〜`

## R17. 書庫のソースの格子の並びを決めている `external-ingestion` の条項に、集計の格子が割り込むことが ST47 への申し送りに入っていない
- 成果物: docs/handoff/ST47.md / openspec/changes/st14-outage-alert/design.md D10（仮）
- 根拠: openspec/specs/external-ingestion/spec.md:676（書庫のソースの格子は「Must の 5 ソースより後ろ、退役したソースより前」）・:709-712（Scenario「格子は上から Must の 5 ソース・書庫のソース・退役したソースの順に並ぶ」）/ ST14 は Must と書庫の間に集計を置く（spec.md:509、D10）/ docs/stories/INDEX.md:202（「collection-coverage の画面の Requirement を後から書き換える Story（ST14 / ST15 など）は、external-ingestion の並びの条項も見る」）—— proposal・design・ST47 への申し送りのどこにもこの条項への言及が無い。いまの `orderCoverageWithArchives` と `archive-order.test.tsx` は集計を入れても壊れない（`archives.ts:100-107`）ので、食い違うのは正典の文だけ
- kind: defer —— 他 Story の担当（ST47）
- 提案: `docs/handoff/ST47.md` の D20 の項に「書庫のソースの並びの Scenario に、Must と書庫の間に集計（`collection-coverage`）が入ることを書き足す」を加える。D10（仮）にこの条項を確かめた旨を 1 行書く
- 処置: deferred ST47 —— `docs/handoff/ST47.md` に R17 の項を足した（並びの文と Scenario に集計を書き足す）。design D10（仮）にこの条項を確かめた旨を書いた

---

## 観点ごとの該当なし

- 観点 3 の「specs に実装の名前」: 該当なし（確かめた範囲: spec.md の全文。出てくる識別子は論理ソース名と、収集側との契約の値（`gap` / `clock-skew` / `retention` / `clock_skew_abandoned`。`docs/collector-contract.md`）だけで、関数名・crate 名・列名は無い）
- 観点 5 の「Story と要件の一致」: 該当なし（確かめた範囲: `check_chain.py` の観点 8 が OK。ST14.md の「価値」と「完了の判定」5 行は deep の Q2 / Q9 / C2 と矛盾しない。75 日でも「2 か月周期では来ない」は成り立つ。`satisfies` に無い FR-54 / FR-79 / NFR-17 / NFR-13 は INDEX:219-227 に理由がある）
- deep の「当初案を覆したもの」: 無い（deep.md:112-115）ので、古い案が Scenario に残っていないかの確認は対象外
