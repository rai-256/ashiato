# ST49 deep-questions.json の独立検査（deep-review）

問いの一覧（Q1〜Q3）と premises.md の C1〜C9 を見る前に、手順 1〜8 を `docs/stories/ST49.md`・`docs/requirements.md`（FR-26 / FR-30 / FR-31 / FR-92 / FR-104〜FR-107 / FR-119 / FR-121〜FR-125 / FR-130 / NFR-25 / 扉 32・33）・
`docs/audit/2026-10-25-consistency.md` の C7 / C8・`crates/server/src/ingest.rs`・`crates/server/src/lib.rs`（`authorize_quiet` / `ingest_one` / `ingest_request` / `apply_external_update` / `store_one`）・
`crates/server/src/stay_store.rs`・`crates/server/src/export.rs`・`migrations/202609081618_envelope.sql`・`202609120943_version_and_ledger.sql`・`202609120944_gates.sql`・`202609151546_drop_reports.sql`・`202610181200_media_capture.sql`・
`202609271716_deletion_ledger.sql`・`crates/collector-windows/src/{contract.rs,outbox.rs,history/contract.rs}`・`collector-android/.../{IngestRequest.kt,Outbox.kt,SegmentStore.kt,DropReport.kt,MediaScanner.kt}`・
`web/src/{attributes.ts,feelings.ts,places.ts}`・走っている ST48（design D8・`feat/st48-db-version-guard` の `crates/server/build.rs`）・ST29（design D8）・ST41（design）・ST23（design D7 / D8）・ST50 / ST51 の Story からやり直し、その後で突き合わせた。

※ 処置の後、問いは 4 問に組み直した（旧 Q3 を外し、R2 → 新 Q3、R3 → 新 Q4 を足した）。下の R の本文は組み直す前の番号で書かれている。

## 手順の記録（指摘ではない）

- 手順 1（要件どうしの衝突）: FR-122 と FR-22 / FR-30（外部サービス由来の更新で中身が入れ替わる収集した記録の「作った時の版」はどれか）→ R1。
  FR-123 の★（衛星を外す理由「サーバが確かめられない」）と FR-122（収集アプリの版も申告で、サーバは確かめられない）・FR-92（書き手は内容でなく資格情報から決める）→ R11。
  FR-122 と FR-130（ST50。新しい欄が古い S-01 で黙って落ちる例に FR-122 の版が名指しされている）→ R6。
  FR-123 と FR-31（派生は同じ識別子のまま作り直される）→ R9
- 手順 2（扉の幅）: 扉 32 の「記録」がどの表を指すか（`core.event` 以外に収集アプリ・画面が作る追記のみの表がある）→ R2。
  「書いた本体の部品」が V-01 の記録で S-01 を指すか、原文を組んだ画面の JS を指すか → R3。「作った時」が更新された記録で何を指すか → R1
- 手順 3（新たに立つ一方通行）: 版の値の中身と、その値が組み立てのたびに正しく更新されるか（凍結した列に古い commit が入る）→ R4 / R5。
  版の列を凍結するか（`device_id` の先例と逆）→ R10。内容の鍵に混ぜない（C3）は正しい（`content_hash_of` は `logical_source`・`event_time`・`raw` だけ。`ingest.rs:209-227`）。
  破棄の報告の鍵は `logical_source + raw`（`drop_reports.sql:25`）で、版を `raw` に入れると鍵が動く → R2 に含めた。書き出しの列（C6）は外に出るが、列を足すだけで今の列は変えない
- 手順 4（日常に影響する選択）: 該当なし（確かめた範囲: 1 件あたり 20〜50 バイトの文字列が増える。C-02 のウィンドウは 1 日数千件（`outbox.rs:7-9` の注記）で 1 日 250 KB 未満、
  90 日でも数十 MB で、未送信の上限（2 GB）に対して無視できる。常時通知・電池・毎日目に入るものは増えない。組み立ての時間は R5 の費用として書いた）
- 手順 5（既存コードが要件を満たしていない箇所）: FR-122 / FR-123 はどちらも未実装。`grep -rn "CARGO_PKG_VERSION\|VERSION_NAME\|BuildConfig.VERSION" crates collector-android/app/src/main web/src` は 0 件で、
  どの部品も自分の版を実行時に読んでいない。滞在は `schema_version` を `1` に固定して書いている（`stay_store.rs:1008`）。
  前の版の履歴は列を名指しで写す（`lib.rs:1625-1651`・`stay_store.rs:927-944`・ビュー `version_and_ledger.sql:78-88`）ので、版の列は 3 か所に足さないと更新のたびに前の値が消える → R1 / R9。
  `core.event_live` は `SELECT *` で、列を足したら作り直す → R13
- 手順 6（過去の取りこぼし）: 当てはまったのは 3 つ。「ある欄を持つものの洗い出しは形が同じものすべてで」（ST12 Q15）→ R2（収集アプリが作る表は `core.event` だけではない）。
  「書き直せない記録の形を新しく決めるときは形の凍結と読めない形の出し方を問う」（ST33 Q12）→ R4（値の形が ST48 の印と合っていない）。
  「推奨に『失うものが無い』と書く前に、版・鍵を使う箇所をすべて辿る」（ST08 Q11）→ R1（C5 の「収集アプリの版は変わらない」が `apply_external_update` の経路を辿っていない）。
  時計の観点は版の決定には入力として入らないが、Q2 の「時刻から推定できる」の前提には効く → R8
- 手順 7（族の網羅）: Q1 の族（同じ版番号の組み立てが 2 つ以上）に「差分の組み立てで印が古いまま」「同じ commit の debug / release」が無い → R4 / R5。
  Q2 に族が書かれていない。版を付ける場面の族（初めて入る・外部サービス由来の更新・滞在の作り直し・消去・画面のタブが更新をまたぐ）のうち、更新と作り直しと画面が C で失う側に決まっている → R1 / R3 / R9。
  「版の欄が途中で落ちる」族（新しい収集アプリ → 古い S-01、古い収集アプリが新しい未送信を読む）は片方だけが C9 にあり、失うことが書かれていない → R6
- 手順 8（前提の確かさ）: 失われるもののある決定（Q1・Q2・C4）が、確かさ 2〜3 の前提に載り、4 への引き上げを tasks に送っている → R7。
  Q2 の context の推定の前提はコードと要件に合わない → R8
- 確かめた前提（合っていた）: 1（C-01 `Outbox.add` は要求の形で積む・`SegmentStore.kt:410-427` が `ingestJson` で読み書きする）、2（`ingestJson` は `ignoreUnknownKeys = true`。`IngestRequest.kt:91-94`）、
  3（サーバの `IngestRequest` に `deny_unknown_fields` は無い。あるのは `lib.rs:1833`・`:1861`・`:1882` の別の型）、6（`lib.rs:506-522`）、7（`places.ts:349`・`feelings.ts:247`・`attributes.ts:195`）、
  8（`archive/worker.rs:56-75` は `RecordSink` 経由で `store_one` → `ingest_request`。`lib.rs:852-857`）、Q2 の why の「S-01 が手を入れる」（`ingest.rs:201-227` の鍵・`to_nfc`・単位の既定・`lib.rs:940-955` の識別子の振り分け）
- 不要な問い: Q3 → R12

## R1. C5 の「収集アプリの版は一度入ったら変わらない」は、外部サービス由来の更新で中身を入れ替えた収集アプリの版を捨てる（失う側を聞かずに決めている）
- 成果物: openspec/changes/st49-record-component-version/premises.md
- 根拠: premises.md の C5 / `crates/collector-windows/src/history/contract.rs:307-334`（C-02 のブラウザ履歴は `external_id` と `source_updated_at` を持って送り、読み直すたびに新しい題名・滞在を送る）/
  `crates/server/src/lib.rs:1406-1411`（内容の鍵が違えば `apply_external_update`）/ `lib.rs:1660-1672`（「`schema_version` も新しい内容を読むための欄なので同じ —— 据え置くと、古い版の宣言で新しい payload を読むことになる。前の値は履歴に残る」）/
  `lib.rs:1625-1651`（前の版へは列を名指しで写す）
- kind: irreversible
- loss: uncaptured
- 提案: 収集アプリの版も、`schema_version` と同じく中身と一緒に動かし、前の値を前の版の表（`core.event_version`・`event_version_live`・書き出しの `EVENT_VERSION_COLUMNS`）に残す形に C5 を直す。
  失わない側は列を 1 つ写すだけで、範囲も要件も変えないので仮（P）でよい。凍結のままにするなら、更新を作った版が残らないことを代償として問いに出す
- 処置: fixed D1 仮 —— C5 を「版は今の中身を作った部品の版。中身が入れ替わるときは一緒に動かし、前の値を前の版の表に写す（`schema_version` と同じ）」に直し、provisional.jsonl に D1 で載せた（design の D1（仮）は spec の段で書く）。前提 11 を足した

## R2. 扉 32 の「記録」の範囲が `core.event` だけになっていて、収集アプリと画面が作る追記のみの表（破棄の報告・原本の欠け・削除の台帳など）が問いにも C にも無い
- 成果物: openspec/changes/st49-record-component-version/deep-questions.json
- 根拠: `migrations/202609151546_drop_reports.sql:13-29`・`:55-79`（C-01 が作る破棄の報告。追記のみのトリガで後から列を埋められない。鍵は `logical_source + raw`（:25））/
  `migrations/202610181200_media_capture.sql:63-74`・`:105-107`（C-01 が作る原本の欠け `core.media_missing`。追記のみ）/ `migrations/202609271716_deletion_ledger.sql:38-47`（画面から本人が消した・戻した操作。追記のみ）/
  `migrations/202610141000_subjective_log.sql:61-66`（`core.feeling_reminder`。追記のみ）/ `collector-android/.../DropReport.kt:58-60`・`:210`（破棄の下書きは保存され、続けて足すので、更新をまたいで 1 本の報告に 2 つの版の破棄が入りうる）/
  premises.md の 8（書く道を `core.event` の 2 つだけ挙げている）/ ブリーフの capability は record-envelope・device-collection・desktop-collection だけ
- kind: irreversible
- loss: uncaptured
- 提案: 「版を持たせる記録の表」を問いにする（A）。少なくとも C-01 の `core.drop_report`・`core.media_missing` を入れるか外すかと、外す理由（「取り違えた版の破棄の報告を選び出せない」が代償）を選択肢に書く。
  入れるなら、版を `raw` に入れず別の欄にする（鍵が動く）ことと、更新をまたぐ下書きの版（作り始めた版か、凍結した版か）を C に足す。生存信号は ST51 が動いている版を申告する（FR-121）ので外してよいと明記する
- 処置: escalated —— Q3（版を持たせる表の範囲）を足した。推奨は「破棄の報告・原本の欠けに収集アプリの版と受け取った S-01 の版、画面の操作の台帳に S-01 の版」。版を鍵と `raw` に入れないことは C3 に、更新をまたぐ下書きを閉じることは Q3 の context に、生存信号を外すことは C13 に書いた。前提 12 を足した

## R3. C8（V-01 の組み立ての版を持たない）は、期限の後も「更新をまたいで開いたままのタブ」が作った記録を新しい S-01 の版で残す（失う側を聞かずに決めている）
- 成果物: openspec/changes/st49-record-component-version/premises.md
- 根拠: premises.md の C8・C7 / `web/src/attributes.ts:195-209`・`web/src/feelings.ts:247-262`（記録の原文・出来事の時刻・TZ は画面の JS が組む。`tz_id` はブラウザの `Intl` から取る `attributes.ts:190`）/
  FR-104 の★（「いまは `vite preview` の別のプロセス」）/ 扉 32（期限の後は「配った版の不具合を追うのに要る」）
- kind: irreversible
- loss: uncaptured
- 提案: C8 を問いに上げるか、失わない側で仮にする。失わない側の例: 画面が自分の組み立ての印（vite の define で版と commit）を要求に載せ、S-01 はログインの印の要求に限ってそれを「作った部品の版」として持つ（C7 の「画面の要求の版の欄は使わない」と一緒に直す）。
  「タブは開き直せば揃う」は本人の操作に頼っていて、期限の後も起きる（更新の手順に画面の再読み込みが無い）ことを代償として書く
- 処置: escalated —— Q4（画面の組み立ての版）を足した。推奨は「残す（ログインの印の要求に限って画面の版の列に持つ）」。「開き直せば揃う」の C8 は消し、前提 13 を足した

## R4. Q1 の推奨「ST48 の印と同じ」は、差分の組み立てで commit が古いまま残る印で、凍結した記録には永久に誤った commit が入る（選択肢の「不可逆」の記述が誤り）
- 成果物: openspec/changes/st49-record-component-version/deep-questions.json
- 根拠: `openspec/changes/st48-db-version-guard/design.md:130-132`（「git の HEAD の変化では作り直さない（差分のビルドでは commit が古いまま残りうる。診断の材料なので許す）」）/
  `git show feat/st48-db-version-guard:crates/server/build.rs` の 20-24 行（`rerun-if-changed` は `migrations/ORDER` と `migrations/` と環境変数だけ）/
  Q1 の選択肢 1 の irreversible「混ざって読める形なので困らない」/ 例 `0.1.0+1a2b3c4d`（8 桁）と D8 の `git rev-parse HEAD`（40 桁）の食い違い / C5（列を凍結する）
- kind: irreversible
- loss: uncaptured
- 提案: Q1 の族に「差分の組み立てで印が古いまま（commit を足した後に移行を触らずに `cargo build` した S-01・C-02、gradle の構成キャッシュに乗った C-01）」を足し、選択肢 1・2 の不可逆に「古い印が付いた記録は誤った commit のまま直せない」と書く。
  新しい印を保つ手順（`.git/HEAD` と参照先を `rerun-if-changed` に足す等）とその費用を選択肢に書く。値の形（全桁か短縮か、`+` の後ろ）を ST48 の台帳と一字一句そろえるかも C に置く
- 処置: escalated —— Q1 の族に「差分の組み立てで印が古いまま」を足し、聞かない側に置いた: 組み立てのたびに印を作り直す（C10。ST48 の D8 と違えて古い印を許さない理由も書いた）。例を 40 桁の形に直し（C11 で ST48 の台帳とそろえる）、選択肢の「混ざって読める」は消した

## R5. Q1 は失わない側（commit と未 commit の印）が安く、範囲も要件も変えないのに、推奨が失う側（commit だけ）で、A にする理由（失わない側の費用）が書かれていない
- 成果物: openspec/changes/st49-record-component-version/deep-questions.json
- 根拠: Q1 の選択肢 2 の irreversible（「印を付けなかった期間の手元の組み立ては、commit 済みのものと分けられない」）/ premises.md の 4（配布の経路に切り替える日まで全部が手元の組み立て）/
  `CLAUDE.md`「失わない側が高くつかず・範囲を越えず・要件を変えないなら、AI が仮で進む（P）」/ `collector-android/app/build.gradle.kts:26-27`（C-01 は debug の鍵で入っている。扉 34。同じ commit の debug と release は縮小の有無で振る舞いが変わりうる）
- kind: irreversible
- loss: uncaptured
- 提案: 失わない側の費用（R4 の「組み立てのたびに印を作り直す」で差分の組み立てが遅くなる、など）を実測か見積もりで選択肢に書き、それが高いなら A のまま、安いなら最も多く残す側で仮（P）にする。
  族に「同じ commit の debug / release の組み立て」を足し、組み立ての種類を印に入れるかを決める
- 処置: escalated —— 推奨を一番多く残す側（版番号・commit・未 commit の印・組み立ての種類）に変え、族に「同じ commit の debug / release」を足し、費用の見積もりを context に書いた。A のままにする理由（「版」が何を指すかを決め、ST48 の台帳・ST52 の配布の版と形をそろえる）を why に書いた

## R6. C9 の「今の S-01 との組み合わせを壊さない」は、新しい収集アプリが ST49 より前の S-01 に送ると版だけが 200 で黙って落ち、収集アプリは未送信から消す（失うことが書かれていない）。逆向き（古い収集アプリが新しい未送信を読む）も無い
- 成果物: openspec/changes/st49-record-component-version/premises.md
- 根拠: premises.md の 3（「記録は入る（版だけが落ちる）」）と C9 / FR-130 の★（「新しい欄（FR-122 の版・ST41 の終わりの時刻など）だけが黙って失われる」）/
  `crates/collector-windows/src/contract.rs:300-323`（`deny_unknown_fields` 無し。`grep -c deny_unknown_fields contract.rs` = 0）・`IngestRequest.kt:91-94`（`ignoreUnknownKeys = true`）—— 古い C-02 に戻すと新しい版が積んだ行の版の欄を読み捨てて送る /
  FR-105（チャンネルを移る）/ ST49.md の requires は `[ST01]` だけ
- kind: conflict
- loss: discarded
- 提案: C9 に「期限（扉 32）の前は、S-01 より先に収集アプリを入れ替えた間の記録と、収集アプリを古い版へ戻した間の記録の版が落ちる」と代償を書き、期限の後は ST50（FR-125 / FR-130）が止める前提だと申し送る。
  tasks に「S-01 を先に、収集アプリを後に入れ替える」順を置くか、ST50 を requires に入れるかを spec の段で決める
- 処置: fixed D3 仮 —— C9 に代償を書き、失わない側（収集アプリの溜めは読み戻した行の知らない欄を捨てずに送る・S-01 を先に入れ替える手順）を D3 で provisional.jsonl に載せた。ST49 より前の版へ戻した間の脱落は直せない（その版が欄を知らない）ことも代償に書いた

## R7. 失われるもののある決定（Q1・Q2・C4）が確かさ 2〜3 の前提に載り、4 への引き上げを tasks に送っている。premises.md の冒頭と表も食い違う
- 成果物: openspec/changes/st49-record-component-version/premises.md
- 根拠: premises.md の冒頭（「失われるもののある問い（Q1・Q2）が載る前提は、下の 1・2・6 で、確かさ 3 まで辿った。4 以上は spec の段で tasks の検証に置く」）と表の 4・5（「何が依るか: Q1」・確かさ 2）/
  `.harness2/skills/story-upstream/SKILL.md:118-120`（「失われるもののある決定は、確かさ 4 以上の前提にだけ載せる。2〜3 で止まったら、機械で確かめる」）/
  5 の出所の ST48 D8 は「（仮）」で、`provisional_review` が否にしうる
- kind: premise
- 提案: 1・2・3 は数分の試験で 4 に上げられる（版の欄の無い今の形の 1 行を、版の欄を足した型で C-02 の serde と C-01 の `ingestJson` で読み戻す・版の欄つきの要求を今の S-01 の `ingest_one` に通す）ので、問いを出す前に走らせて結果を表に書く。
  冒頭の「1・2・6」を表の「何が依るか」と合わせ（Q1 は 4・5 に依る）、5 は ST48 の D8 が仮であることと、否になったときに Q1 の推奨がどう動くかを書く
- 処置: fixed premises.md —— 冒頭を問いごとの依る前提（Q1 は 4・5、Q2 は 6・11、Q3 は 12、Q4 は 7・13）に直した。4 は grep を走らせて 4 に上げた。5 は ST48 の D8 が仮で、否になっても Q1 の推奨は動かない（形だけそろえる。C11）と書いた。6・7・11・12・13 は「どこで書くか」の前提で、外れても失う側の選択が変わらないので 2〜3 のまま tasks で 4 に上げる。1〜3 は C の実装の前提で、どの問いにも載らない

## R8. Q2 の context「更新の最中に入った記録で外れうる」はコードと要件に合わない。推定が外れる本当の場合（時計・版番号が動かない組み立て）が書かれていない
- 成果物: openspec/changes/st49-record-component-version/deep-questions.json
- 根拠: FR-119 の★（「更新の間 S-01 は止まっていて、新しい記録は収集アプリの手元に溜まる」—— 更新の最中に入る記録は無い。C-03 も S-01 の中で止まる）/
  `migrations/202609081618_envelope.sql:23`（`ingest_time` は DB の `now()` = PC の時計）/ premises.md の 4（配布の経路に切り替える日まで版番号は `0.1.0` のまま）/ ST51.md の完了の判定（「サーバを別の版にすると…1 行残る」—— 版番号が同じ組み立ての入れ替えを変化と数えるかは決まっていない）
- kind: premise
- 提案: context の推定の外れ方を「PC の時計を直した・戻したとき」「ST51 が版番号だけを見るなら、版番号の同じ組み立ての入れ替えは履歴に出ない（Q1 の答えに依る）」に直す。失われるもの（ST51 より前は推定もできない）は変わらない
- 処置: fixed deep-questions.json —— Q2 の context の推定の外れ方を「ST51 より前は履歴が無い・PC の時計を直した戻した・版番号の同じ組み立ての入れ替えは履歴に出ない（Q1 に依る）」に直した

## R9. C5 は S-01 の版が動く場合を「外部サービス由来の更新」だけにしていて、滞在の作り直し（同じ識別子のまま原文を書き換え、前の版を履歴へ積む）が抜けている
- 成果物: openspec/changes/st49-record-component-version/premises.md
- 根拠: `crates/server/src/stay_store.rs:594-601`（原文が変われば `push_version` してから `update_stay`。印だけの変化は履歴に積まずに書き換える）/ `stay_store.rs:927-944`（`push_version` は列を名指しで写す）/
  `stay_store.rs:1008`（新しい滞在は `schema_version` を `1` に固定）/ FR-123 の★（「作り直したときに新しい版が入るだけで困らない」）/ FR-31
- kind: technical
- loss: discarded
- 提案: C5 に「滞在の作り直しで原文が変わったときは作り直した S-01 の版に変え、前の値は `push_version` で前の版の表に写す。印だけの書き換えと変わらなかった滞在は元の版のまま」を足す。
  `push_version` に列を足し忘れると作り直しのたびに前の版が消えるので、前の版の表の列が本表と揃うことを試験で固定する
- 処置: fixed D2 仮 —— C5 に滞在の作り直し（原文が変わったら作り直した S-01 の版にし、前の値は `push_version` で写す）を足し、D2 で provisional.jsonl に載せた。前の版の表の列が本表と揃うことを試験で固定する

## R10. C5 が収集アプリの版を凍結するのは、収集アプリの申告値（`device_id`・`tz_*`）を「設定ミスを直せる余地を残す」ために凍結しなかった門の先例と逆で、理由が書かれていない
- 成果物: openspec/changes/st49-record-component-version/premises.md
- 根拠: `migrations/202609120944_gates.sql:66-70`（「`device_id` / `tz_id` / `tz_offset_min` は…収集側の設定ミスで誤った値が入ったとき、直せる余地を残す（どれも冪等キーの入力ではない）」）/ C5 / R4（古い commit が入りうる）
- kind: technical
- 提案: 凍結するなら、先例と違えて凍結する理由（版で選び出す目的には改ざんされないことが要る、など）と、誤った印（R4）が入ったときに直せない代償を C5 に書く。凍結しないなら門の外に置くと C5 を直す
- 処置: fixed premises.md —— C8 を「版の列は書き換えの門の外（`device_id` の先例と同じ。誤った印を直す余地を残す）。列を書き換えるのは C5 の経路だけ」にした

## R11. FR-123 は衛星の記録を「サーバが確かめられない」で外すが、FR-122 の収集アプリの版も申告で、サーバは確かめられない。列と書き出しの説明が「確かめた版」と読める
- 成果物: openspec/changes/st49-record-component-version/premises.md
- 根拠: FR-123 の★ Q13 / FR-92（「書き手と利用者識別子を、送られてきた内容ではなく資格情報から決める」）/ premises.md の 6（「C-01 と C-02 は同じ合言葉を使う」）/ C2（「形を検査しない・断らない」）/ C6（書き出しに説明つきで足す）
- kind: conflict
- 提案: 要件を変える問いにはしない。C6 の書き出しの説明と spec の Requirement で、収集アプリの版は「収集アプリの申告（確かめていない）」、S-01 の版は「S-01 が自分で入れた」と分けて書く（外に出た説明は直せない）
- 処置: fixed D4 仮 —— C6 に、書き出しの説明を「部品の申告（S-01 は確かめていない）」と「S-01 が自分で入れた」に書き分けることを足した。design の D4（仮）に、反転条件（収集アプリの版を S-01 が確かめる手段 —— 署名など —— ができたら説明を「確かめた」に変える）と一緒に spec の段で書く

## R12. Q3（選び出す口を作るか）は完了の判定で決まっていて幅が無い
- 成果物: openspec/changes/st49-record-component-version/deep-questions.json
- 根拠: `docs/stories/ST49.md:46-50`（完了の判定は「版が残る」「付けない」だけで、選び出す口を求めていない）/ Q3 の why（「失うものは無い」）
- kind: defer
- 提案: Q3 を問いから外し、C に「選び出す口はこの Story では作らない（書き出しと D-01 の問い合わせで選べる）」と置く
- 処置: fixed deep-questions.json —— 旧 Q3 を問いから外し、C12 にした（選び出す口はこの Story では作らない）。新しい Q3・Q4 は R2・R3 の問い

## R13. 走っている ST41・ST23・ST48 と同じ箇所を触る（spec の段で並びを決める）
- 成果物: openspec/changes/st49-record-component-version/premises.md
- 根拠: `openspec/changes/st41-event-end-time/design.md:18`・`:194`（同じ `INSERT`・`IngestRequest`・`EVENT_COLUMNS` に列を足し、`core.event_live` の `SELECT *` を作り直す）/
  `openspec/changes/st23-record-erasure/design.md:171-173`（`ingest_request` に「HTTP か内部の格納か」の引数を足す —— ST49 が C-03（内部・資格情報なし）と収集アプリ（API の合言葉）と画面（ログインの印）を分ける場所と同じ）/
  `crates/server/src/lib.rs:537-540`（判定は `router()` を通る求めにだけある。書庫の `store_one` には無い）/ ST48 D2（当てた版を当て直さない）—— `envelope.sql:43` の `CREATE OR REPLACE VIEW core.event_live AS SELECT *` が当て直しで列を拾う今の形は ST48 の後は起きない
- kind: technical
- 提案: design に「版の判定は `ingest_request` の呼び出し元（HTTP の判定・内部の格納）から渡す」「ST49 の移行で `core.event_live` と `core.event_version_live` を作り直す」を置き、ST41 / ST23 と同じ関数を触る順を tasks に書く
- 処置: fixed premises.md —— C7 に「判定は `ingest_request` の呼び出し元から渡す」を、C14 に ST41・ST23・ST48 と同じ箇所を触ることと `core.event_live` などを作り直すことを書いた。触る順は spec の段で tasks と proposal の Impact に書く
