# ST50 spec の独立検査（spec --draft。第 1 回の深掘りの推奨で先に書いた版）

対象: `openspec/changes/st50-contract-version/` の proposal.md・specs/{record-envelope,device-collection,desktop-collection,data-durability}/spec.md・design.md・tasks.md・provisional.jsonl、
docs/stories/ST50.md・docs/stories/INDEX.md の末尾の訂正・docs/handoff/ST49.md・docs/handoff/ST52.md の末尾。入力は deep.md（Q1〜Q9 は答え待ち・推奨）・deep-questions.json・premises.md・review/deep.md。

## 機械の検査（2026-10-28）

| コマンド | 結果 |
|---|---|
| `openspec validate st50-contract-version --strict` | rc=0（valid） |
| `python3 scripts/check_chain.py .` | rc=0（chain: OK。観点 8 の再生成との一致も通る） |
| `python3 scripts/check_scenarios.py . st50-contract-version` | rc=1。担保なし 63 件 = この change の Scenario 全部（実装前なので想定内。tasks 12.2 が 0 にする） |
| `python3 scripts/review_triage.py . st50-contract-version` | rc=1。31 件、全部 `review/deep.md` の `処置:` の字面（→ R5） |
| `python3 scripts/spec_overlap.py candidates . st50-contract-version` | `[]`（0 件） |
| `python3 scripts/provisional.py check …` | rc=1。D13・D18・D19 の provisional-review.md が無い（グラフの別の席が書くもので、この段の指摘にしない） |

数えたもの（tasks の下限の照合）: `cargo test -p ashiato-collector-windows --lib sender:: -- --list` = 11、`cargo test -p ashiato-server -- --list` のうち `export_` = 176・`hash_is_pinned` = 2、
`@Test` は SenderTest.kt 14・OriginalUploaderTest.kt 30・RetentionNotifierTest.kt 10・MainActivityTest.kt 13。tasks の下限（18・100・1・14・30・10・13）はどれも実物と合う。
spec の Scenario 63 本（record-envelope 34・device-collection 16・desktop-collection 11・data-durability 2）は、全部 tasks.md に一字一句同じ `Scenario: <名前>` で名指しされている（逆向きに tasks の名前が spec に無いものも 0）。
`git tag --list 'v[0-9]*'` は空（P7 のとおり）。

---

## R1. ST50 の `unknown_field` は ST49 D3（仮）の反転条件の形を立てる。加えて ST49 が後に入ると封筒の写しの作り直しが要るが、申し送りに書かれていない
- 成果物: docs/handoff/ST49.md / openspec/changes/st50-contract-version/proposal.md（Impact の ST49）
- 根拠: openspec/changes/st49-record-component-version/design.md:56（D3（仮）の反転条件「知らない欄を送り返したことで S-01 が要求を断る事例…が出たら、送る前に S-01 の契約の版で欄を落とす（ST50 の契約の版に任せる）」）。ST50 の design.md:42-48（D2）は S-01 の知らない最上位の欄を持つ 1 件を `unknown_field` で断る —— 条件の形が立つ。
  docs/handoff/ST49.md はこの扱い（欄を落とさない）を書いている。ただし書いていないものが 2 つある:
  (1) ST50 が先に main に入ると、ST49 が収集アプリ 2 本と S-01 の型に足す `client_version`（st49 design.md:114）で `docs/contract/envelope.json` が古くなり、ST50 の design.md:89（D7 の 1「写しが今のコードと違えば落とす」）で ST49 の PR の CI が落ちる。ST49 の tasks.md は `envelope` / `check-contract-version` を 1 度も名指さない（`grep -n "envelope\|check-contract" openspec/changes/st49-record-component-version/tasks.md` = 0 件）。
  (2) ST49 の C-01 の送り出しは「保った JSON をそのまま送る（型に通して作り直さない）」（st49 design.md:53）。ST50 の Task 10.1（tasks.md:172-176）は `Sender` が行の版を読んで版 ≤ A の行だけを載せる形で、行の版を型の欄（`contractVersion` 既定 1）から読む前提。後から入る側がどちらの読み方で行の版を取るかが申し送られていない
- kind: defer
- 提案: docs/handoff/ST49.md に (1)「後から入る側は `tools/check-contract-version.sh --write` で写しを作り直して commit する」、(2)「C-01 の保った JSON の行からも `contract_version`（無ければ 1）を読んで選ぶ」を足す。処置は followup ST49
- 処置: followup ST49 —— docs/handoff/ST49.md に (1) 後から入る側が `tools/check-contract-version.sh --write` で写しを作り直す、(2) C-01 の保った JSON の行からも `contract_version`（無ければ 1）を読んで選ぶ、を足した

## R2. 「配った印」の型 `v[0-9]*` を ST48 と ST50 の 2 つの検査が持つ。ST52 がタグの型を決めるときに両方を一緒に変える必要がある
- 成果物: docs/handoff/ST52.md / openspec/changes/st50-contract-version/design.md（D7・D8）
- 根拠: design.md:91-92（D7 の 3: `git tag --list 'v[0-9]*' --sort=-creatordate` の先頭）・design.md:106（D8: `-beta.N` を含む）、tools/check-released-migrations.sh:18（`git tag --list 'v[0-9]*'`）。ST52（配布）がタグの型を決める Story で、ST48 の申し送りが既に同じファイルにある（docs/handoff/ST52.md:5-7）。
  ST50 の版 1 を閉じるのは最初の `v*`（Q8 の推奨）なので、ST52 が最初の印を打つ時点が ST50 の版の規則の起点になる
- kind: defer
- 提案: docs/handoff/ST52.md の末尾の節（既に書かれている）で足りる。処置は followup ST52。Q8 の答えが「正式版だけ」になったら、この節の「ベータ版の印を含む」を `spec --answers` が書き直す
- 処置: followup ST52 —— docs/handoff/ST52.md の末尾の節のまま

## R3. D20 の「前の版の表を錠を外さずに埋め戻す」は成り立たない。`reject_version_rewrite` は列を見るかに関係なく、消去でない UPDATE を全部止める
- 成果物: openspec/changes/st50-contract-version/design.md（D20）/ tasks.md（4.1）/ specs/record-envelope/spec.md（Scenario「ST50 より前に収集アプリから入った記録は版 1 になる」）
- 根拠: design.md:184-185「前の版の表の錠を一時的に外さずに済むよう、列を足した直後に `UPDATE` する。錠の関数は列を足す前の本体なので新しい列を見ない」。
  実物: migrations/202609120944_gates.sql:196-198 `CREATE TRIGGER event_version_immutable BEFORE UPDATE ON core.event_version … EXECUTE FUNCTION core.reject_version_rewrite()`（列の指定なし）。
  その今の本体 migrations/202610231200_event_end_time.sql:161-191 は、名指しの列が変わっていなくても `IF OLD.raw = '' THEN RAISE`（:186-187）と `IF NEW.raw <> '' OR NEW.payload <> '{}'::jsonb THEN RAISE '履歴の書き換えは本文の消去…だけが通る'`（:190-191）に落ちる。
  つまり `UPDATE core.event_version SET contract_version = 1 …` は、消去済みの行も消去されていない行も全部例外になる。これまでの移行は `core.event_version` を UPDATE で埋め戻したことが無い（`grep -n "UPDATE core.event_version" migrations/*.sql` は 0 件。列は `ADD COLUMN … DEFAULT` か空で足してきた。202609120943_version_and_ledger.sql:44-48）。
  加えて、移行は 1 つのまとまりで当たる（crates/server/src/migration_ledger.rs:448-461 `conn.begin()` → `raw_sql` → `commit`）。`core.event` の門 `event_requires_version` は `DEFERRABLE INITIALLY DEFERRED`（gates.sql:150-153）なので、埋め戻しの UPDATE の検査は COMMIT の時に走り、その時点の本体 —— 同じ移行で `contract_version` を足した後の本体（D20・spec の「収集した記録の契約の版の書き換えを、前の版を同じまとまりに積むときだけ通す」）—— で判定される。足した後の本体が `contract_version` の変化を中身の変化に数えるなら、`core.event` の埋め戻しも COMMIT で落ちる
- kind: technical
- 提案: D20 に埋め戻しの手順を明記して直す（例: 同じ移行の中で `event_version_immutable` を一時的に外して埋め戻し、張り直す。`core.event` は埋め戻しの直後に `SET CONSTRAINTS core.event_requires_version IMMEDIATE` で古い本体のうちに検査を済ませてから本体を置き換える、など）。
  張り直したことを `tools/check-immutable.d/<作成時刻>_st50-contract-version.sh` で撃って確かめることを tasks 4.3 に足す。Task 4.3 の `st50_migration_backfill` はこれを捉えるはずなので、試験はそのままでよい
- 処置: fixed D20 —— 埋め戻しを `DISABLE TRIGGER`（event_version_immutable・event_requires_version）→ UPDATE → `ENABLE TRIGGER` → 本体の置き換えの順に直した。tasks 4.1 に手順、4.3 の check-immutable.d の節に張り直しの確かめ（tgenabled）を足した

## R4. `ingest_one` / `ingest_request` は呼び手（資格情報の種別）を受け取らない。D1・D2 は 1 件ごとに呼び手で分けるので、proposal の「引数は増やさない」と食い違い、ST23 と同じ継ぎ目に触る
- 成果物: openspec/changes/st50-contract-version/proposal.md（Impact の ST23）/ design.md（D1・D2）/ tasks.md（3.1・4.2）
- 根拠: proposal.md:67「ST50 は HTTP の入口（`ingest` の前）で見出しを見るので引数は増やさない」。一方 design.md:47（D2「掛けるのは `Caller::ApiToken` の要求だけ」）と tasks.md:85（4.2「`Caller::ApiToken` の 1 件は欄の値か 1、それ以外は NULL」）は、1 件ごとの判定の中で呼び手が要る。
  実物: crates/server/src/lib.rs:1830 `authorize(&app, &headers).await?;`（戻り値の `Caller` を捨てる）・:1846 `ingest_one(&app, item)`・:1116-1119 `ingest_one(app, item)`・:1141-1144 `ingest_request(app, req)` のどれも呼び手を持たない。
  書庫の取り込みは `store_with → ingest_request`（:942-944）で同じ関数を通るので、「欄が無ければ 1」（収集アプリ）と「NULL」（書庫）を同じ関数の中で呼び手により分けることになる。
  ST23 は同じ `ingest_request` に `Settle::InTx` / `Settle::Later` の引数を足す（openspec/changes/st23-record-erasure/design.md:172-173）—— ST50 も同じ関数の署名を変えるので、proposal の「ほかは触る表が重ならない」は継ぎ目では成り立たない
- kind: technical
- 提案: proposal の Impact を「`ingest_one` / `ingest_request` に呼び手（または『収集アプリの 1 件か』の印）を渡す。ST23 の `Settle` と同じ署名に触るので、後から入る側が rebase で両方の引数を合わせる」に直す。
  design D1 に「書庫（`store_with`）は NULL を渡す」を書き、tasks 4.2 の名指しの場所に `store_with` を足す
- 処置: fixed D1 —— 呼び手（収集アプリの 1 件か）の印を ingest_one / ingest_request へ渡す引数を足し、store_with と滞在・画面は偽と書いた。proposal の Impact の ST23 の行と tasks 4.2 を直した

## R5. review_triage が 31 件で落ちている。`review/deep.md` の `処置:` が文法に無い形（`fixed: Q8 を足した` など）
- 成果物: openspec/changes/st50-contract-version/review/deep.md / deep.md
- 根拠: `python3 scripts/review_triage.py . st50-contract-version` rc=1（31 件）。R1〜R9 は loss つき（discarded / uncaptured / exported）で `fixed:` は通らない（「人間に返すもの。fixed で閉じられない」）。R10 は「仮決めの指す先は design.md の D<n>」、R11・R12 は「fixed の指す先が D<n> でも <x.y> でも change 直下のファイル名でもない」。
  同じ形の ST49 は `escalated —— Q3 を足した…` / `fixed D1 仮 —— …` / `fixed premises.md —— …`（openspec/changes/st49-record-component-version/review/deep.md:51・63・115）で通っている（`review_triage.py . st49-record-component-version` = OK）。
  `escalated` は deep.md に R<n> が書かれていることを求めるが、今の deep.md が名指すのは R10 だけ（`grep -no "R[0-9]\+" deep.md` → 53 行目の R10 のみ）。
  あわせて review/deep.md:3 の対象「Q1〜Q7」「P1〜P8」は、Q8・Q9・P9 を足した後も古いまま。premises.md の表は P9 が P8 の前に並ぶ
- kind: technical
- 提案: R1〜R9 を `escalated —— Q<n> を足した / 直した`、R10 を `fixed D13 仮 —— …`（provisional.jsonl の D13 の source は既に `review/deep.md#R10`）、R11 を `fixed premises.md`、R12 を `fixed deep-questions.json` に書き換え、
  deep.md の各 Q の見出しか本文に由来の R<n>（Q1 ← R2・R8、Q2 ← R4・R9、Q3 ← R5、Q4 ← R6・R7、Q7 ← R4、Q8 ← R1、Q9 ← R3）を書く
- 処置: fixed deep.md —— review/deep.md の処置を R1〜R9 escalated・R10 fixed D13 仮・R11 fixed premises.md・R12 fixed deep-questions.json に書き換え、対象の行（Q8・Q9・P9 を足したこと）を直した。deep.md の各 Q に「由来: review/deep.md R<n>」を足した

## R6. tasks の依存順: 封筒の写しを作るのが 7.2 で、7.1 の試験と、後の 8.1・10.1 の欄の追加が写しを古くする。CI の置き場も矛盾している
- 成果物: openspec/changes/st50-contract-version/tasks.md（7.1・7.2・8.1・9.2・10.1）
- 根拠: 下の 6 点（tasks.md と .github/workflows/ci.yml の行を名指す）
  - tasks.md:123-127（7.1）の試験は「`ASHIATO_ENVELOPE_OUT` が無ければ `docs/contract/envelope.json` の `android` と比べる」が、その写しを作る `--write` は 7.2（tasks.md:128-140）。7.1 の検証 `GT St50EnvelopeShapeTest` / `CW st50_envelope_shape` は 7.2 の前には通らない
  - 8.1（tasks.md:144「`contract.rs` の `IngestRequest` と `HeartbeatRequest` に `contract_version: u32` を足す」）と 10.1（tasks.md:172「`IngestRequest.kt`・`Heartbeat.kt`・`DropReport.kt`・原本の欠けの 1 件に `contractVersion` を足す」）は Task 7 の**後**で直列化の欄を増やす。写しの作り直しの指示が無いので、8・10 の後に `st50_envelope_shape` と `tools/check-contract-version.sh` が落ちる
  - 定数を作る指示が 3 か所にある: 1.1（tasks.md:41 S-01 の `CONTRACT_VERSION`）・7.1（tasks.md:126「3 つの定数（D12）を足す」）・10.1（tasks.md:172 携帯の `const val CONTRACT_VERSION = 1`）
  - 7.2 は「`.github/workflows/` の `check-released-migrations.sh` の並びに足す」と「`android` と `rust` の旗の job の後に走る job に置く」を同時に言う（tasks.md:138-139）。前者は rust の job の中（.github/workflows/ci.yml:91-92）。別の job に置くなら、ci.yml:83-90 の「印の取り寄せ」（checkout は浅くタグが無い）を同じ job に置かないと `git tag --list` が空になり、D7 の 3・4 は「印が 0 個」として黙って通る
  - 9.2 の検証（tasks.md:168）は関数名の `grep -c` だけで、試験を走らせない（走るのは CI の `collector-windows-runtime`）。終了条件として「CI のその job が緑」を書いていない
  - 10.1 の `GT SenderTest`（tasks.md:189）は `--tests '*SenderTest*'` と `TEST-*SenderTest*.xml` が `St50SenderTest` にも当たるので、下限 14 は既存の `SenderTest` の 14 本が減っても満たせる
- kind: technical
- 提案: Task 7（写しと検査）を Task 8・10 の後へ移すか、8.1・10.1 の検証に `tools/check-contract-version.sh --write` の後の `tools/check-contract-version.sh` rc=0 を足す。定数を作るのは 1 か所の Task に寄せる。
  7.2 の CI の置き場を 1 つに決め、別の job なら印の取り寄せの手順を同じ job に置くことを書く。9.2 に「CI の `collector-windows-runtime` job が緑」を終了条件として書く。`GT SenderTest` は `--tests 'dev.ashiato.collector.SenderTest'` と `TEST-dev.ashiato.collector.SenderTest.xml` に絞る
- 処置: fixed 7.2 —— 7.1 の試験の検証を写しを作る 7.2 の後へ移し、8.1・10.1 に `--write` と検査・形の試験を足した。定数は 1.1（S-01）と 7.1（PC・携帯）だけにした。CI の置き場を rust の job の check-released-migrations の直後に決め（印の取り寄せがある）、収集アプリのずれは 7.1 の試験が各 job で見る形にした（D7）。9.2 の終了条件に collector-windows-runtime の job を書いた。SenderTest を完全な名前で絞った

## R7. 「200 で本文が読めない」を「受け付けない」と読むと、携帯が captive portal に当たっただけで「サーバを更新してください」の通知が出る
- 成果物: openspec/changes/st50-contract-version/design.md（D11・D5・D6）/ specs/device-collection/spec.md・specs/desktop-collection/spec.md（「読めない答え」の IF と知らせの WHEN）
- 根拠: design.md:132（D11「受け付けない（止まった状態に入る）: 404・`200` で本文が JSON でない・…」）、design.md:72・80（止まった状態に入った契機で通知を 1 回）、specs/device-collection/spec.md:11・80-82（読めない答えも「契約の版で手元に残している」に入り、通知を 1 回出す）。
  公衆の Wi-Fi の captive portal は 200 で HTML を返す。携帯の収集アプリ自身がこの事故を名指している（collector-android/…/IngestRequest.kt:76「API ゲートウェイや captive portal が 200 で別の配列を返し」）。design の Risks（design.md:213）は「404 を返す別のもの」しか挙げていない。
  送らない側に倒すのは FR-125 どおりで失うものは無い。誤るのは知らせの文言で、S-01 を更新しても直らない（更新済みの S-01 でも captive portal の下では同じことが起きる）
- kind: daily
- 提案: 送らずに残す条件はそのままにし、「サーバを更新してください」を出す条件を分ける（例: 404 と、版を持つ答えで受け付けない版の行がある・426 のときだけ促す。200 で読めない答えは一時的な失敗と同じく促さない、または数契機続いたときだけ促す）。どちらにするかを D11 に書き、Scenario「到達できないだけでは更新を促さない」の隣に「読めない 200 では更新を促さない（または N 回続いたら促す）」を足す
- 処置: escalated —— 深掘り第 2 回 Q10 で本人に返した（2026-10-29。deep.md Q10・deep-questions-r2.json。答え待ち）。経緯: はじめは仮の D22 として、送らずに残す条件と「S-01 の更新を待っている」（知らせを出す）条件を分け、200 の読めない答えでは促さないことを D9・D11 と device / desktop の Requirement・Scenario（「読めない 200 の答えだけでは更新を促さない」）に書いた。provisional.jsonl に載せたが、別の席の否（provisional-review.md D22。携帯は 90 日で捨てる）で外した

## R8. `GET /contract` を 5 分ごとに引くと、読み出しの記録（data-sensitivity）に 1 日あたり端末ごとに 288 行が積もる。D18（仮）の費用に無く、data-sensitivity の決めた理由と食い違う
- 成果物: openspec/changes/st50-contract-version/design.md（D18（仮）・D15）/ provisional.jsonl（D18 の cost）
- 根拠: crates/server/src/access_log.rs:140-142 `is_ingest` は `/ingest` `/heartbeat` `/drops` だけ。:163-165 で資格情報が認められた取り込みだけを書かずに通し、それ以外の口は全部 1 行書く（`route_layer` で全経路に掛かる。lib.rs:3792-3795）。
  openspec/specs/data-sensitivity/spec.md:294・306「資格情報が認められた取り込み…を、読み出しの記録に残さない」「取り込みを残さないのは『誰がいつ読んだか』を残すためで、収集は数分ごとに来る」。
  D18（design.md:167-172）と provisional.jsonl の D18 の cost は「5 分に 1 回の小さな GET」だけで、記録の行を数えていない。携帯と PC の 2 台で 1 日 576 行が、人が読む読み出しの記録に混ざる
- kind: daily
- 提案: `/contract` を読み出しの記録に残すかを design に決めて書く。残さないなら data-sensitivity の Requirement（取り込みを残さない）を MODIFIED で広げる（capability が増えるので proposal と INDEX の訂正の節にも書く）。残すなら D18 の cost と provisional.jsonl の D18 の `cost` に行の数を書く
- 処置: fixed D23 仮 —— 失わない側（今の規則のまま 1 回 1 行を残す）を仮で採り、1 日 576 行を D18・D23 の費用に書いた。反転条件は「読み出しの記録を読みにくくする事例が出たら残さない形（data-sensitivity を MODIFIED）に変える」

## R9. Scenario「契約の版はスキーマ版と別に動く」は、記録の版を要求の見出しから取るように読める。Requirement（1 件の欄から取る・無ければ 1）と食い違う
- 成果物: openspec/changes/st50-contract-version/specs/record-envelope/spec.md
- 根拠: spec.md:25-28「`schema_version` が 1 の記録を版 2 の要求で送る → …契約の版は 2 になる」。WHEN は 1 件の `contract_version` に触れていない。同じ spec の Requirement は spec.md:128-129「その 1 件の欄 `contract_version` … を記録の表に残す」「欄の無い 1 件の契約の版を 1 とする」で、欄の無い 1 件なら版は 1 になる。
  この Scenario のとおりに試験を書くと、見出しの版を記録に入れる実装（Q1 の推奨が退けた「送った時の版」。design.md:40）が緑になる。THEN も「`schema_version` は 1 のまま」と「契約の版は 2」の 2 つを束ねている
- kind: technical
- 提案: WHEN を「`schema_version` が 1 で `contract_version` が 2 の 1 件を、版 2 を名乗る要求で送る」に直す。必要なら「欄の無い 1 件を版 2 の要求で送ると記録の版は 1」を別の Scenario にして、見出しで記録の版が決まらないことを固定する
- 処置: fixed 4.2 —— Scenario を「contract_version が 2 の 1 件を版 2 の要求で送る」に直し、「要求の見出しの版は記録の契約の版を決めない」を足した（tasks 4.2 の st50_record_version_not_from_header）

## R10. deep の推奨の数語が Scenario に無い（Q7 の「削除」・Q8 の「ベータ版も配った印」）
- 成果物: openspec/changes/st50-contract-version/specs/record-envelope/spec.md
- 根拠: Q7 の推奨（deep.md:38）「欄の追加・削除・型の変化・必須と任意の入れ替わり」。Requirement（spec.md:218）は 4 つを言うが、Scenario は追加（:227-230）と型・必須（:232-235）だけで、欄の削除を落とす Scenario が無い。
  Q8 の推奨（deep.md:43）「ベータ版も含める（印 `v*` のどれか）」。Requirement（spec.md:190・218）は「ベータ版の印を含む」と言うが、どの Scenario の WHEN も「配った印」とだけ書き、印が `-beta.N` であることを言わない。tasks 7.2（tasks.md:130）が自己試験で `v0.1.0-beta.1` を使うのは tasks の選び方で、spec は固定しない。
  印の型を `v[0-9]*` から `-beta` を除く形へ変える実装でも、今の Scenario は全部緑のまま（Q8 の答えが試験で守られない）
- kind: technical
- 提案: 「配った印の後に S-01 の型から欄を 1 つ消し、版を上げずに検査すると落ちる」を足す。「ベータ版の印（`v0.1.0-beta.1`）しか無い木で欄を足し、版を上げずに検査すると落ちる」を足す（Q8 の答えが「正式版だけ」なら `spec --answers` がこれを「通る」に書き直す）
- 処置: fixed 7.2 —— 「欄を消して版を上げないと CI が落ちる」「ベータ版の印も配った印として比べる」を足し、7.2 の自己試験の印にした

## R11. Requirement の本文だけが言い、Scenario が言っていないもの（record-envelope）
- 成果物: openspec/changes/st50-contract-version/specs/record-envelope/spec.md / design.md（D4・D7・D15）
- 根拠: 下の 5 点（spec.md・design.md・tasks.md と crates/server/src の行を名指す）
  - spec.md:9「問い合わせの口の道・答えの形・答えの意味を、最初の配った印の後は変えない」に Scenario が無い。design.md:64（D4）は「道・答えの形・**見出しの名前・426** を変えない…spec の Requirement に書いた」と言うが、Requirement が凍結するのは道・答えの形・意味だけで、見出しの名前と 426 は入っていない
  - spec.md:39-40 の「生存信号・**破棄の報告・原本の欠け**の 1 件の `contract_version` が新しすぎれば `contract_unsupported`」に対し、Scenario は生存信号だけ（:77-80）。3 つの型は断りの列挙が別々（crates/server/src/lib.rs:2763 `HeartbeatError`・drops.rs:178 `DropError`・media_missing.rs:52 `MediaMissingError`）なので、1 つが通っても残りは通らない
  - spec.md:221「版の履歴に今の版の行が無ければ CI を落とす」、design.md:93「印はあるのに写しを取り出せなければ落とす」に Scenario が無い（tasks.md:137 は印の無い一文「落ちること」だけ）
  - design.md:155（D15）「画面のログインと衛星は 403」は spec に無く、tasks.md:45 が Scenario「合言葉の無い問い合わせは断られる」（THEN は 401 だけ）の試験の中で 403 を見ている。応答の状態符号の意味は観測できる振る舞いで、design にあると archive で正典から落ちる
  - spec.md:35「426 で断り、本文 `{"error": …}` を返し」を 6 つの口すべてに掛けているが、`HEAD /media/originals/{sha256}` の応答は本文を持てない（HTTP の HEAD）。Scenario（:57-60）も PUT と HEAD を束ねている
- kind: technical
- 提案: 凍結の Requirement に見出しの名前と 426 を足す。凍結は配った印の後の検査なので、Scenario は「配った封筒の例の試験が見出しの名前と 426 を固定する」の形で足す（tasks 5.1 の例の試験に名指しを足す）。
  破棄の報告・原本の欠けの 1 件の `contract_unsupported` の Scenario を足す。「版の履歴の行が無いと CI が落ちる」「印はあるのに写しを取り出せないと落ちる」の Scenario を足し、tasks 7.2 の印にする。
  D15 の 403 を spec の Requirement と Scenario（「画面のログインで問い合わせると 403」）へ移す。426 の本文の文を「HEAD は本文を持たない」と書き分け、Scenario を PUT と HEAD の 2 つに分ける
- 処置: fixed 5.1 —— 凍結に見出しの名前と 426 と本文の鍵を足し、口の形の例 wire.json（tasks 5.1・D8）に S-01 が従う Scenario を足した。3 つの型の contract_unsupported を 1 つの Scenario にまとめ直し（3.1）、版の履歴と写しを取り出せないときの Scenario（7.2）、画面のログインの 403（1.1）、HEAD は 426 だけ（2.1）を足した

## R12. Requirement の本文だけが言い、Scenario が言っていないもの（device-collection・desktop-collection）。D14 は device の Requirement に無い
- 成果物: openspec/changes/st50-contract-version/specs/device-collection/spec.md・specs/desktop-collection/spec.md / design.md（D14）
- 根拠: 下の 6 点（device / desktop の spec.md・design.md・tasks.md・Sender.kt の行を名指す）
  - design.md:151（D14）「止まった状態を抜けた契機で、PC の `Sender` と**携帯の `Sender`** の『飛ばす』印を捨てる」。desktop の Requirement（desktop spec.md:14）は書くが、device の Requirement（device spec.md:3-16）には無い。携帯の `Sender` は飛ばす印を持つ（collector-android/…/Sender.kt:135・170-174・302）。tasks.md:176（10.1）は実装するが、spec が言わないので archive で正典から落ちる。desktop でも、Scenario「S-01 を更新すると PC に残っていた記録がそのまま入る」（:45-48）の THEN は印に触れず、tasks.md:152 が試験の中で見るだけ
  - device spec.md:9 / desktop spec.md:8「見出しに載せた行の版の一番大きいもの」。Scenario（device :23-26・desktop :20-23）は答えが 1・送る行が全部版 1 なので、見出しが「載せた行の最大」か「S-01 の答え」か「定数」かを区別できない
  - device spec.md:81 は画面に「手元に残している行の最も古いものを積んでからの日数」を出すと言うが、画面の Scenario（:90-93）は文言だけを見る
  - desktop の知らせの Requirement（desktop spec.md:52-56）は「契約の版で行を手元に残している」の中身（404・読めない答え・426・`unknown_field` / `contract_unsupported`）を書かない。device は書いている（device spec.md:80）
  - D5・D6（design.md:73・80）「前に出した時刻を手元に持つ（再起動のたびに鳴らないように）」は観測できる振る舞い（再起動しても 24 時間以内に出し直さない）だが spec に無い
  - desktop Scenario「PC の通知は止まっている間 1 日に 1 回出直す」（:65-68）の THEN「日数の文言が 0・1・2 日と進む」は、日数の起点が「最も古い行を積んだ時」（:52）なので、WHEN に「最も古い行を止まった時に積んだ」が無いと値が決まらない。desktop Scenario「PC は積んだ時の版を 1 行ごとに持ち、版を持たない古い行は版 1 として送る」（:25-28）は名前どおり主張が 2 つ
- kind: technical
- 提案: device の Requirement と Scenario に「抜けた契機で飛ばす印を捨て、受け付けるようになった 1 件が先頭で待たされない」を足し、desktop の同じ Scenario の THEN にも書く。見出しの Scenario を「答えが 2・未送信に版 1 の行だけ → 見出しは 1」に変えるか足す。
  画面の日数・再起動しても出し直さない（両方）の Scenario を足す。desktop の知らせの Requirement に「手元に残している」の中身を書く。日数の Scenario の WHEN に最も古い行を積んだ時を書く。主張 2 つの Scenario を分ける
- 処置: fixed 10.1 —— device の Requirement に飛ばす印を捨てることと Scenario を足し、見出しが載せた行の最大になる Scenario（携帯・PC）、画面の日数、再起動しても出し直さない（携帯・PC）を足した。desktop の知らせの Requirement に「更新を待っている」の中身を書き、日数の Scenario の WHEN に起点を書き、主張 2 つの Scenario を分けた（tasks 8.1・9.1・10.1・11.1）

## R13. 「要件へ戻すもの」の FR-125 の注記がまだ要件に無く、triage の検査 4 は前の ★ で空振りに通る。ST50.md の完了の判定も D13（仮）と食い違ったまま
- 成果物: deep.md（要件へ戻すもの）/ docs/requirements.md（FR-125）/ docs/stories/ST50.md（完了の判定）
- 根拠: deep.md:50-54 が FR-125 の★に「上限は FR-8 の主語（C-01）に掛かる。C-02 の未送信には置かない」を足すと言う。docs/requirements.md:1042-1047 の FR-125 にはまだ無い（最後の改訂の印は ★ 2026-10-25）。
  `review_triage.py` の検査 4 は「要件へ戻すもの」に挙がった要件に ★ と日付があるかを見るので、既にある ★ 2026-10-25 で通ってしまい、戻し漏れを捉えない。docs/stories/ST50.md の完了の判定「（上限を超えた分だけが捨てられる）」は PC にも上限があるように読め、design.md:141-146（D13（仮）。PC は捨てない）・desktop spec.md:13 と食い違う
- kind: technical
- 提案: deep.md の書くとおり record の段で直すことで足りるが、改訂の印は ★ 2026-10-28 と日付を新しくし、ST50.md の完了の判定は stories.json → `make_story.py` で直す（直した後に check_chain の観点 8 が通ることを見る）。機械が見ないので、record の段の作業に名指しで残す
- 処置: fixed deep.md —— 「要件へ戻すもの」を record の段の作業として名指しし、改訂の印の日付を新しくすることと ST50.md の完了の判定を stories.json → make_story.py で直すことを書いた（この段は要件を直さない）

## R14. proposal の「ST51 は `deferred ST51`」に指す先の R が無く、文も印の前後を分けていない
- 成果物: openspec/changes/st50-contract-version/proposal.md（Impact の申し送り）
- 根拠: proposal.md:68「ST51（生存信号に動いている版を足すなら契約の版を上げる。tasks がまだ無いので `deferred ST51`）」。`deferred` は review の R の処置で、review/deep.md にも、この spec の段の前にも ST51 を指す R は無い。
  中身も、FR-130 の★（docs/requirements.md:1063）と D8（design.md:107）では最初の配った印の前に足す欄は版 1 のままで、上げるのは印の後だけ。いずれにせよ ST51 は封筒の写し（`docs/contract/envelope.json`）の作り直しが要る（FR-121 の★ docs/requirements.md:1019 が生存信号に動いている版を足す。docs/stories/ST51.md:20）
- kind: defer
- 提案: 「ST51 が生存信号に欄を足すときは `tools/check-contract-version.sh --write` で写しを作り直し、最初の配った印の後なら版を上げる」と直し、ST51 の Story（tasks がまだ無い）へ deferred で渡す
- 処置: followup ST51 —— ST51 が tasks.md を持った（凍結）ので docs/handoff/ST51.md へ移した。proposal の申し送りを「欄を足すときは --write で写しを作り直し、最初の配った印の後なら版を上げる。印の前は版 1 のまま」に直した

## R15. spec に実装の置き場の名前が入っている
- 成果物: openspec/changes/st50-contract-version/specs/record-envelope/spec.md
- 根拠: spec.md:216「1 つの写し（`docs/contract/envelope.json`）に」。写しのファイルの置き場は実装の選び方で、観測できる振る舞いは「写しと今のコードが食い違えば CI が落ちる」。D7（design.md:87）に既にある
- kind: technical
- 提案: spec からファイル名を外し「1 つの写しに」とする（置き場は design D7・D21 に残す）
- 処置: fixed D21 —— spec からファイル名を外した（置き場は design D7・D21）

---

観点 6: 該当なし（`spec --answers` ではない。第 1 回の推奨で先に書いた `spec --draft` の版）

観点 7: 該当なし（spec_overlap.py candidates が 0 件）

確かめた範囲の補足:
- 観点 1: design 冒頭の対応表（design.md:3-6）の Q1→D1 … Q9→D9、C1→D10、C2→D11、C3→D7・D8、C4→D12、C5→D13（仮）、C6→D14、C7→D15、C8→D16、C9→D17 を 1 件ずつ中身と照らした。推奨と違うことを書いた D は無い。数値（5 分・24 時間・83 日・90 日・2 GB・426・版 1）は Scenario にある。落ちているのは R10・R12（D14）
- 観点 3: proposal の Capabilities（proposal.md:38-43）と specs/ のディレクトリ 4 つは一致し、INDEX.md:91-93・103 の表に ST50 がある。data-durability の追加は INDEX.md:472-477 の訂正の節に理由がある。collection-coverage を変えない理由も proposal.md:46-47 にある
- 観点 5: check_chain の観点 8 は通る（ST50.md の逐語は requirements.md:1038-1063 と一致）。satisfies に無い要件を specs が満たそうとしているものは無い（NFR-16 は data-durability の既存の規則の写し）
- 走っている change との重なり: candidates は 0 件だが、コードの継ぎ目では ST49（R1）・ST23（R4）と重なる
