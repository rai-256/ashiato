# ST23 spec review（`spec --draft`。推奨の答えで書いた仕様を、推奨が採られたとして見る）

見たもの: proposal.md / specs/record-deletion/spec.md / specs/browsing-views/spec.md / design.md / tasks.md / deep.md（Q1〜Q6 は答え待ち）/
defaults.md（C1〜C16）/ premises.md / review/deep.md / deep-questions.json / docs/stories/ST23.md / INDEX.md / 正典（record-deletion・browsing-views）/
既存コード（deletion.rs・stay_store.rs・lib.rs・export.rs・export/excluded.rs・archive/worker.rs・migrations）/ 走っている change（st09-media-capture・st13-scheduled-account-fetch）。

## 機械の検査

```
openspec validate st23-record-erasure --strict   → Change 'st23-record-erasure' is valid（rc=0）
python3 scripts/check_chain.py .                  → chain: OK（0 件 / 未回収 0 件 / warn 0 件。観点 8 の再生成との一致も通過）
python3 scripts/check_scenarios.py . st23-record-erasure
                                                  → FAIL（担保なし 71 件 / 名無しの確認待ち 0 件）rc=1
                                                    71 件はすべてこの change が足した Scenario（record-deletion 68・browsing-views 3）。実装前なので想定どおり。
                                                    warn 15 件（spec に無い Scenario を指す印）は collector-windows・archive_flow_tests・migrate_backup・st12_delta_diff の既存のもので、ST23 とは無関係
python3 scripts/review_triage.py . st23-record-erasure
                                                  → triage: OK（指摘 18 件 / 仮決め 0 件 / 要件へ戻すもの 0 件）
```

triage の「仮決め 0・要件へ戻すもの 0」は、design に（仮）が 8 本（D10〜D17）、proposal に「要件へ戻すもの」が 3 件あるのに 0。
deep.md にまだ写っていない（record の段の前）ためで、R21 に書いた。

Scenario → tasks の割り当て: 新しい Scenario 71 本はすべていずれかの Task に名指しされている（スクリプトで突合。漏れ 0）。MODIFIED で写した既存の 22 本は名前を変えていない。

---

## R1. D8 の `settle_arrival` を置く取り込みのまとまりは書庫の格納と共有で、D9・Q3 推奨の detail・spec と食い違う
- 成果物: openspec/changes/st23-record-erasure/design.md（D8 / D9）・specs/record-deletion/spec.md:271-272
- 根拠: design.md:146「`ingest_one` のトランザクションで…`settle_arrival(tx, &stored)` を呼ぶ」。しかしトランザクションを開くのは `ingest_one` ではなく `ingest_request`（crates/server/src/lib.rs:1182 `.begin()`）で、
  書庫の取り込み器もここを通る: `archive/worker.rs:56-72 store_requests → sink.store` → `lib.rs:782-790 PgSink::store → store_with` → `lib.rs:814 ingest_request`。
  そのまま置くと書庫の記録も**格納と同じまとまり**で本文が消え、D9（「格納の後の別のまとまり」・見回りで消し直す）、spec:272、Q3 推奨の detail（「書庫から入るものは格納の後の別のまとまりで消す…その間は本文が DB にある」）、
  Risks（design.md:235）のどれかが成り立たなくなる。さらに錠の費用（design.md:154）は `/ingest` の 1 件ごとしか見ていないが、実際には書庫の 1 件ごと（百万件級。lib.rs:999 のコメント）に `pg_advisory_xact_lock` を取ることになる
- kind: conflict
- 処置: fixed D8 仮 —— 後着を同じまとまりで片付けるのは取り込みの口（HTTP）だけにした。`ingest_request` に `Settle::InTx` / `Settle::Later` の引数を足し、書庫（`store_with`）は `Later`。書庫の後着は D9 の格納の後のまとまりで消す（spec の 2 文と Q3 の detail のまま）。錠の費用は位置の取り込みだけに絞り、Risks を直した。反転条件は D8
- 提案: `settle_arrival` を呼ぶ場所を「HTTP の口（`ingest_one`）だけ」か「格納の共通の口（書庫も同じまとまり）」のどちらかに決めて D8 / D9 に書く。後者なら D9 の見回りと spec:272 を「同じまとまり」に揃え（Q3 の detail より守りが強い側なので問いは変わらない）、書庫の取り込みの錠の費用を Risks に足す。

## R2. D2 の「重なる滞在」が Q2 推奨より広く、別に消した滞在を巻き込んで、戻せず本文も消せない記録を作る
- 成果物: openspec/changes/st23-record-erasure/design.md（D2 の 3）・specs/record-deletion/spec.md:111
- 根拠: design.md:68「時間が…**端でも**重なるもの。**印は問わない**（`rebuild:absorbed`・`rebuild:erased-range`・**別の本人の削除**・印なし）」。Q2 推奨の detail（deep-questions.json）は
  「消した位置から作られていた滞在（作り直しで吸収された滞在も）」で、本人が別に消した滞在は入っていない。spec:111 は「重なる滞在（…を含む）」で範囲を閉じていない。
  帰結: (a) browsing-views spec:91-94 の Scenario（10:00–11:00 を本文ごと消し、11:00–11:30 は消しただけ）と、11:00–11:30 を先に消していた場合に食い違う（端の 11:00 で重なるので一緒に本文が消える）。
  (b) 巻き込まれた滞在 B の連鎖の位置（原因 = B）は D2 の 2 に入らず本文が残るが、B は `raw = ''` になるので、B を戻すと 409（D6・spec:503）、B を本文ごと消すと「すでに本文ごと消した」で 0 件（design.md:57）——
  **本文を持ったまま、戻すことも本文を消すこともできない記録**が残る。D2 は（仮）でもない（design.md:32）
- kind: irreversible
- 処置: fixed D2 仮 —— 失わない側に倒した。重なる滞在は作り直しの印のものだけ・端が触れるだけは入れない・本人が別に消した滞在は入れない。spec の Requirement に 3 文と Scenario「別に消した滞在は重なっていても本文が残る」、tasks 2.1 に端の試験を足した。provisional.jsonl に載せた
- 提案: 失わない側（本人が別に消した滞在は範囲に入れない・端だけの接触は重なりにしない）は安く、範囲を越えず、要件を変えないので、そちらを P として D2 に書き、spec:111 に「本人が別の操作で消した滞在は含めない」を足す。広い側を残すなら Q2 の問いに載せて A で聞く。

## R3. 戻す口の 409 の規則が、既存の「別の原因で消えたままの記録は戻さない」と重なり、隣の「消した」を戻せなくする
- 成果物: openspec/changes/st23-record-erasure/specs/record-deletion/spec.md:503・design.md:131
- 根拠: ST22 の `erase` は、すでに別の滞在で消えている位置にも原因の行を追記する（crates/server/src/deletion.rs:305-321）。`restore` はその位置を `blocked_by_other_cause` で戻さずに飛ばす（deletion.rs:133-146・:172-174）。
  spec:503 は「戻すことになる記録（指定した滞在と、それを原因として消えた記録）に本文を消した記録が 1 件でもあれば 409」で、別の原因（本文ごと消した A）で消えたままになる位置も数える。
  10:00–11:00 を本文ごと消し 11:00–11:30 を消した日に 11:00 ちょうどの位置が 1 件あれば、後者の「戻す」は 409 になり、browsing-views spec:94（「消した」の行が戻すための識別子を持つ）が戻せない識別子を出す
- kind: conflict
- 処置: fixed D6 仮 —— 戻す口の 409 の数から、別の原因で消えたまま残る記録（いまの `blocked_by_other_cause`）を除いた。spec の MODIFIED に 2 文と Scenario「本文ごと消した時間と境界で位置を共有しても隣の消した滞在は戻せる」を足し、tasks 4.1 に割り当てた
- 提案: spec:503 と D6 の「戻すことになる記録」から、別の原因で消えたまま残る記録（いまの `blocked_by_other_cause`）を除く。境界で位置を共有する Scenario を 1 本足す。

## R4. D13（仮）の「和」は自ら失う側を書いているのに問いに出ておらず、緩和の「範囲を見せる」が spec にも文面にも無い
- 成果物: openspec/changes/st23-record-erasure/design.md（D13 / D1 / D16）・specs/record-deletion/spec.md:244 / :51-58・specs/browsing-views/spec.md:51
- 根拠: design.md:190「**失う側**: 指定した滞在の時間の外へはみ出した分の後着の本文も消える。確認に出す範囲（D1 の `range`）は和で出し、本人が見て押す」。
  しかし spec の確認の Requirement（:53-58）は範囲を返すと言わず、D16 の文面（design.md:213）にも範囲が無い。Q3 の問い・context・選択肢は「本文ごと消した滞在の時間帯」としか言わず、和には触れていない（deep-questions.json Q3）。
  加えて spec:189「本文を消した滞在の時間帯に、削除されていない滞在を作らない」が和に効くので、はみ出した時間にかかる生きた滞在も次の作り直しで隠れる（隠れた滞在は `rebuild:` の印なので本文ごと消す口は 409・原因の滞在は戻せない）。
  はみ出しを試す Scenario は 1 本も無い（指定 10:00–11:00＋吸収 10:30–11:30 → 行が 10:00–11:30・11:15 の後着の本文が消える、のような形）
- kind: irreversible
- 処置: fixed D13 仮 —— 失わない側に倒した。本文ごと消した時間帯は指定した滞在の時間だけ（和にしない）。`user:purged` はビューに出さず（D5 / D6）、はみ出しの後着は生きた記録として入る。spec に「一緒に本文を消した滞在のはみ出しは時間帯に入らない」、確認の `range` も指定した滞在の時間（D1）、D16 の文面に範囲の時刻を出した。provisional.jsonl に載せた
- 提案: P の条件（失わない側が高くつかず・範囲を越えず・要件を変えない）は、失わない側（指定した滞在の時間だけ）にはみ出しの後着が「戻せない滞在が原因の消した」で残る費用があり、自明には満たさない。
  A として Q3 の context に「時間帯は一緒に本文を消した滞在の時間の和」を書いて本人に見せるか、P で進めるなら和の範囲を確認の応答と文面（spec:53-58・D16）に載せ、はみ出しの Scenario を足す。

## R5. D17（仮）は一覧に出ている生きた滞在の本文を消す側を選んでいる。失わない側が安いので P の条件に合わない
- 成果物: openspec/changes/st23-record-erasure/design.md（D17）・specs/record-deletion/spec.md:111
- 根拠: design.md:221「作り直しが落ちると…滞在が印なしで残る…D2 の 3 に入れ、`user:purged` を付けて本文を消す」。本人が「消した」にしていない、一覧に見えている滞在が、確認の「滞在 N 件」に混ざって戻せなくなる。
  Q2 推奨の detail は「消した位置から作られていた滞在」で、作り直しが落ちた生きた滞在を名指していない。D17 と R2 の (a) を合わせると、端だけで接する生きた滞在も対象になる。
  失わない側（印の無い滞在は範囲に入れない）を選んでも、本人がその滞在を ST22 の口で消してから本文ごと消せば済む（安い・範囲内・要件を変えない）
- kind: irreversible
- 処置: fixed D17 仮 —— 失わない側に倒した。印の無い重なる滞在は範囲に入れない。spec の Requirement の 1 文と Scenario「一覧に出ている滞在の本文は消えない」で固定。provisional.jsonl に載せた
- 提案: D17 を失わない側に倒して（仮）のままにするか、今の側を残すなら A として Q2 に載せる。どちらでも spec:111 に「印の無い滞在を含めるか」を書く（design に置くと正典から落ちる）。

## R6. Task 3 の `grep` 検証は、設計どおりに直しても 0 件にならない
- 成果物: openspec/changes/st23-record-erasure/tasks.md:69・design.md:111-124（D5）
- 根拠: `grep -rn "payload->>'end'" crates/server/src --include=*.rs | grep -v _tests.rs | grep -v 'fn span_of' | wc -l` は現在 7。
  `grep -v 'fn span_of'` は何も除かない（`span_of` は `payload.get("end")` で読み、その文字列を含まない。stay_store.rs:295-306）。残る 7 件のうち stay_store.rs:311 はコメント、:325（`load_existing`）と :1241（`day_view`）は生きた滞在を索引で粗く絞る式、
  lib.rs:2548（`stays_detail_get`。`event_live` 越しの生きた滞在）、export/excluded.rs:202 は前の版の終わりで、D5 の表は「`core.erasure_span` か**本文**」とこの式を残す。
  D5 の表には `stays_detail_get` と粗い絞り 2 か所が無い（proposal の Impact:136 は `stays_detail_get` を挙げている）
- kind: technical
- 処置: fixed 3.1 —— `grep` の検証をやめ、`payload->>'end'` の出現を `file:関数` で許可の一覧と突き合わせる試験（`purge_range_reader_allowlist`）にした。D5 の表に `stays_detail_get` と生きた滞在の粗い絞り 2 か所を「変えない」で足した
- 提案: 検証を「消した時間帯を読む式（`core.stay_erased` / `user_erased_stay_sql` / `deleted_at IS NOT NULL` の滞在を読む SQL）の中に `payload->>'end'` が無い」に絞るか、残してよい行（生きた滞在の粗い絞り・`stays_detail_get`・前の版の本文の補い）を明示した許可の一覧と突き合わせる形にする。D5 の表に残す読み手と理由を足す。

## R7. D14（仮）の「その日の 00:00〜24:00」は観測できる振る舞いで、Scenario が無い。Migration Plan の「down 後は D14 で読む」も成り立たない
- 成果物: openspec/changes/st23-record-erasure/design.md（D14・Migration Plan）・specs/record-deletion/spec.md:247
- 根拠: design.md:195-196 は、範囲の行が無い本文を消した滞在について、1 日の一覧の行・書き出しの除外・作り直しを 1 日全体にし、後着は印だけにする。spec:247 は「厳しい側」としか言わず、Scenario が無い。
  Task 3（tasks.md:67）は「D14 の 2 本」を試験に書くが、Scenario の名前が無いので check_scenarios が追えず、archive で正典から落ちる。
  Migration Plan（design.md:243）「本文を消した後に戻すと span が無くなり、D14 の倒し方で読む」は、`.down.sql` が元のビュー（migrations/202609271716_deletion_ledger.sql:60。終わりは `payload->>'end'`、無ければ +1µs）に戻すので、D14 ではなく 1µs に縮む（premises の実測と同じ形）
- kind: technical
- 処置: fixed D14 —— D14 の振る舞いを spec の Requirement（2 文）と Scenario 2 本（「範囲の行が無い…」）に移し、tasks 3.1 / 5.1 に割り当てた。Migration Plan を「down の後は 1µs に縮む。本文を消した滞在があれば `.down.sql` が止める」に直し、tasks 1.1 に止まり方の試験を足した
- 提案: D14 の振る舞い（1 日全体として読む・後着は印だけ）を spec:247 の Requirement と Scenario（2 本）に移す。Migration Plan は「down の後は範囲が 1µs に縮む（本文を消した滞在があれば down しない）」と正しく書く。

## R8. 取り込みと本文ごと消す操作を直列にする錠（D8）は観測できる振る舞いなのに、spec の「同時に走らせない」に取り込みが無い
- 成果物: openspec/changes/st23-record-erasure/specs/record-deletion/spec.md:346・design.md:149
- 根拠: design.md:149「錠を取る（…取らないと、消す操作の commit と取り込みの commit が互いに見えずに印が漏れる）」。spec:346 は「派生の作り直し・消す操作・戻す操作と同時に走らせない」で、取り込みを挙げていない。
  確認の後に届く Scenario（spec:97-100）は順に起きる場合だけで、同時に届いた位置が本文を持ったまま残らないことを試す Scenario が無い
- kind: technical
- 処置: fixed specs/record-deletion/spec.md —— 「同時に走らせない」に取り込みの口からの位置の格納を足し、Scenario「本文ごと消す操作と同じ時間帯の取り込みが同時に走っても位置の本文は残らない」を足した（tasks 5.1）
- 提案: spec:346 に「その時間帯に届いた記録の格納」を足し、「本文ごと消す操作と同じ時間帯の位置の取り込みが同時に走っても、その位置の本文は残らない」の Scenario を足す。

## R9. すでに本文ごと消した滞在へ、確認の件数を運んで求めたときに 200 と 409 のどちらになるかが決まっていない
- 成果物: openspec/changes/st23-record-erasure/specs/record-deletion/spec.md:11 / :90-92 / :46-49・design.md:56-57
- 根拠: spec:11「すでに本文ごと消した滞在を指定する求めを、消した件数 0 とともに 200」と、spec:90「運んだ件数が消す時点の件数と 1 つでも違えば…409」。2 度押し（1 回目が成功した後、同じ件数を運んだ 2 回目）は両方に当たる。
  design.md:56（錠→数え直し→比べて 409）と :57（`raw = ''` は 200）の順も書かれていない。Scenario（spec:46-49）は運ぶ件数を言っていない。すでに本文ごと消した滞在の**確認**の応答も決まっていない
- kind: technical
- 処置: fixed D1 —— 消す口の順を D1 に書いた（錠 → 本文ごと消した滞在なら件数を比べずに 200・0 件 → 断り → 数え直し）。spec の 1 文、Scenario の WHEN（同じ確認の件数を添えて）、確認の口の応答（0 件と本文ごと消した印）と Scenario「本文ごと消した滞在の確認は 0 件を返す」を足した
- 提案: 「本文ごと消した滞在なら件数を比べずに 200・0 件」を spec:11 に明記し、Scenario の WHEN に「確認で見せた件数を添えて」を足す。確認の口が本文ごと消した滞在に何を返すかも 1 行書く。

## R10. 派生の門（D7）は消去の形でない `raw = ''` を通すが、D5 / D6 は `raw = ''` を「本文を消した」の印として読む
- 成果物: openspec/changes/st23-record-erasure/design.md:137-139・specs/record-deletion/spec.md:313-317
- 根拠: D7 は「消去の形 → 台帳を要る」「それ以外（作り直しの書き換え…）→ 通す」。`UPDATE … SET raw = '', payload = '{"x":1}'` は消去の形でない（payload が空でない）ので通る。
  一方で `core.stay_erased.purged`（D5:113 `raw = ''`）・`load_existing` の候補外し（D6:130）・戻す口の 409（D6:131）は `raw = ''` だけで本文を消した滞在と読む。
  収集した記録の門はこの形を R95 で塞いでいる（migrations/202609120944_gates.sql:101-110 のコメント）。spec:317「取り込み口の外から加えられた操作にも適用する」の守りがここで抜ける
- kind: technical
- 処置: fixed D7 —— 派生の門に「原文を空にする書き換えで消去の形でないものは拒む」の枝を足した。spec の 1 文と Scenario「解析済みを残して原文だけ空にする書き換えは拒まれる」（tasks 1.1）
- 提案: spec:313-315 に「派生の行の原文を空にする書き換えは、消去の形で台帳の行があるときだけ通す」を足し、D7 の判定に `NEW.raw = '' AND OLD.raw <> ''` で消去の形でないものを拒む枝を足す。Scenario を 1 本（payload を残して原文だけ空にする書き換えは拒まれる）。

## R11. Requirement の本文だけが言って、Scenario が言っていないもの
- 成果物: openspec/changes/st23-record-erasure/specs/record-deletion/spec.md・specs/browsing-views/spec.md
- 根拠:
  - record-deletion:9 / :58 存在しない識別子の 404（消す口・確認の口とも Scenario は「滞在でない」だけ）、確認の口の「滞在でない」404
  - record-deletion:57「写しの数を少なく数えない」（Scenario :73-76 は写し 2 個がそのまま 2 と出るだけで、特定できない場合を試していない）
  - record-deletion:163「同じ書庫の読み直しを重複として扱う」（Scenario は取り込みの口の再送だけ）
  - record-deletion:214「本文を消した滞在それぞれと前の版の始まりと終わりを操作に結んで残す」（範囲の行の Scenario が無い。:252-255 は「消した時間帯を読む」だけ）
  - record-deletion:216「台帳の行を記録の読み出しと書き出しに出さない」（正典の ST22 には :115 の Scenario があった。新しい 2 表には無い）
  - record-deletion:346 の「消す操作・戻す操作と同時に走らせない」（Scenario は作り直しとの同時だけ）
  - design D15:208「どの組にも入らない論理ソースが範囲に入ったら、消す口は断る」（観測できる断りが design にだけある）
  - browsing-views:8 の「指定した滞在の識別子」と「後から届いて本文を消した件数」（読み出しの側の Scenario :25-28 は印と識別子の無さだけ）
  - browsing-views:50-51「重なる部分は本文ごと消したの行が持つ」「行の時間は一緒に本文を消した滞在の時間を合わせたもの」（隣り合う場合の Scenario だけで、重なる・和の Scenario が無い）
- kind: technical
- 処置: fixed specs/record-deletion/spec.md —— 挙がった 9 項目にそれぞれ Scenario を足した（知らない識別子 2 本・確認の滞在でない識別子・写しの数え方・書庫の読み直し・範囲の行・台帳を出さない・戻す操作との同時・組に入らないソースの断り（D15 の断りを Requirement に上げた）・browsing-views の指定した滞在と後着の件数・重なりの差し引き）。tasks に全部割り当てた（新しい Scenario 91 本・漏れ 0）
- 提案: 上の各項に Scenario を 1 本ずつ足す（D15 の断りは spec の Requirement に上げてから）。

## R12. 観測できない、または主張を束ねた THEN
- 成果物: openspec/changes/st23-record-erasure/specs/record-deletion/spec.md
- 根拠:
  - :286-289「端末から届いた位置は本文を持ったまま確定しない」の THEN「本文が空でない状態はどの時点にも見えない」は測れない。Task 5（tasks.md:87-88）が実際に見るのは「消去の台帳の `txid` が記録の `txid` と同じ」で、Scenario と試験が別のことを言う
  - :252-255「消した時間帯を読む」がどの読み手で読むのか書かれていない（1 日の並びか、書き出しの除外か、台帳か）
  - :97-100（409・数え直した件数・本文が残る）、:389-392（ファイルが替わる・成功が残る）、:399-402（1 晩目の失敗・2 晩目の書き直し）は 1 つの THEN に 2〜3 の主張を束ねている
- kind: technical
- 処置: fixed 5.1 —— 「本文を持ったまま確定しない」の THEN を「消去の台帳の行は記録の行と同じトランザクションで書かれている」にし、試験もそれを見る形にした。「消した時間帯を読む」の Scenario は読み手（1 日の並びの区間）を書いた。束ねた THEN は「主張を 1 つずつ確かめる」を Scenario に添え、Global Constraints の規律で 1 つずつ assert する
- 提案: :289 の THEN を「消去の台帳の行と記録の行が同じトランザクションで書かれている」にする。:254 に読み手（1 日の並びの区間）を書く。束ねた THEN は Scenario を分けるか、tasks の Global Constraints（:15）の「主張を 1 つずつ assert」に名指しで載せる。

## R13. Task 2 の Scenario は Task 3 の変更が無いと通らない（依存順）
- 成果物: openspec/changes/st23-record-erasure/tasks.md:53-59（Task 2）・:63（Task 3）
- 根拠: Task 2 の Scenario「本文ごと消す操作と作り直しが同時に走っても本文は戻らない」（THEN: どちらも成功・その時間帯に滞在の行は出ない）は、`load_existing` が `raw = ''` の滞在を候補から外す変更（D6・Task 3）が要る。
  それが無いと作り直しが本文を消した滞在に一致して `update_stay` で書き戻そうとし（stay_store.rs:963）、Task 1 の派生の門（D7）が拒んで作り直しが失敗する
- kind: technical
- 処置: fixed 3.1 —— 「本文ごと消す操作と作り直しが同時に走っても本文は戻らない」を Task 3 に移し、Task 3 が Task 2 の上に積むことを書いた
- 提案: その Scenario を Task 3 に移すか、Task 3 を Task 2 の前に置く。

## R14. Task 4 の契約の検証が `<生成物>` のままで、CI の関門（`tools/check-openapi.sh`）を通さない
- 成果物: openspec/changes/st23-record-erasure/tasks.md:79
- 根拠: `src/bin/openapi.rs` は標準出力に書くだけ（crates/server/src/bin/openapi.rs:7-9）。CI は `tools/check-openapi.sh` で `docs/openapi.json` との差分を見る（tools/check-openapi.sh:6-10。docs/testing.md:31）。
  tasks は `docs/openapi.json` の更新も `check-openapi.sh` も書いていないので、検証が緑でも CI が落ちる
- kind: technical
- 処置: fixed 4.1 —— `cargo run -q -p ashiato-server --bin openapi > docs/openapi.json` の後に `tools/check-openapi.sh` rc=0 と `jq -e` の 3 条件にした。触るファイルに `docs/openapi.json` を足した
- 提案: 「`cargo run -q -p ashiato-server --bin openapi > docs/openapi.json` の後に `tools/check-openapi.sh` rc=0、`jq -e '.paths["/stays/purge"] and .paths["/stays/purge/preview"]' docs/openapi.json` rc=0、`/stays/restore` に 409 がある」にする。

## R15. Task 7 の「実物の量」は `tools/seed.sh` の最大量（1 日分）で、D11 の反転条件を判定できない
- 成果物: openspec/changes/st23-record-erasure/tasks.md:104・design.md:178
- 根拠: `tools/seed.sh max` は「要件上限の 15 件」の 1 日（tools/seed.sh:7・:12）。D11 の反転条件「書き直しが 5 分を超える」が見る量は 1 年で約 1 GB（premises.md の確かさ 1 の行）。この量では 5 分は判定できず、PR に書く秒数は反転条件の証拠にならない。終了条件（コマンドと閾値）も無い
- kind: technical
- 処置: fixed 7.2 —— 量を作る台本（使い捨ての DB に `generate_series` で 1 年分）と、`seconds_total` が 300 以下の閾値（超えたら rc=1）を検証に書いた。`tools/seed.sh max` は使わない
- 提案: 量を作る手段（N 日分を `generate_series` で `core.event` に入れる SQL など）と、測るコマンド・5 分の閾値を検証に書く。作れないなら「人間の確認待ち（実データ）」と明記する。

## R16. Task 9.2 の e2e は共有の偽データを戻せない形で消し、偽データの作り方の根拠も無い
- 成果物: openspec/changes/st23-record-erasure/tasks.md:122-123
- 根拠: e2e は `tools/stack.sh up` の同じ DB を使い（docs/testing.md:108-109）、並列にしない共有の DB（:118）。「確認して本文ごと消す」の Scenario はその偽データの滞在を戻せない形で消すので、
  `STACK_RESET` なしで 2 回目を走らせると対象が無くなり、確認バッチの画面にも本文ごと消した状態が残る。
  また `tools/seed.d/` は MODE=normal のときだけ読まれ（docs/testing.md:92）、seed.sh が書庫の項目を置く手段は `/ingest` と台帳の直の INSERT（tools/seed.sh:414-423）で、「書庫の位置 3 件・前の版 20 件・残している写し 2 個（置き場のファイル）」を作れるかは tasks に書かれていない
- kind: technical
- 処置: fixed 9.2 —— 消す Scenario は試験の中で未使用の日付に `/ingest` で自分の滞在を作って消す形にし、前の版 20 件の作り方を書いた。seed.d は書庫の写し 2 個を置くだけ（読むだけ）。`STACK_RESET` なしで 2 回続けて通ることを検証に足した
- 提案: 消す Scenario は試験の中で API から滞在を作って消す（偽データの滞在は読むだけ）形にし、seed.d は「見るだけ」の材料に限る。写し 2 個と前の版 20 件の作り方（置き場へのファイルの置き方・作り直しの回数）を tasks に書く。

## R17. 走っている Story と触るファイルの重なりが、Impact と Global Constraints より広い
- 成果物: openspec/changes/st23-record-erasure/proposal.md:143-147・tasks.md:31
- 根拠:
  - ST09: tasks 9.1（openspec/changes/st09-media-capture/tasks.md:201）が `export/excluded.rs` の規則と `export.rs` の `CATALOG` に足す。ST23 も Task 1.1 で `CATALOG` に 3 表、Task 3.1 で `export/excluded.rs` の `erased_spans` を書き換える。proposal は ST09 を「触らない」とだけ書く
  - ST13: tasks 3.1（…/st13-scheduled-account-fetch/tasks.md:60）が `lib.rs` の起動で「取り込み器の隣」に spawn を足し、`bin/openapi.rs` を触る。ST23 の Task 7 も `run()` の「取り込み器の隣」、Task 4 も `bin/openapi.rs` と（R14 の直しで）`docs/openapi.json`
  - `archive/worker.rs` の `mark_archive_arrivals` の呼び出しは 2 か所ある（worker.rs:1693・:2466）。D9 / Task 6 の「隣の 1 行」はどちらかを決めていない
- kind: conflict
- 処置: fixed D9 仮 —— `archive/worker.rs` は触らない側に倒した（`mark_archive_arrivals` の中で消し、失敗は既存の読み直しが消し直す。2 か所の呼び出しはどちらもこの関数を通る）。反転条件は D9。ST09（`CATALOG`・`export/excluded.rs`）と ST13（`run()`・`bin/openapi.rs`・`docs/openapi.json`）の重なりを proposal の Impact と tasks の Global Constraints に書いた
- 提案: Impact の重なりに上の 3 点を足し、Global Constraints に「ST09 / ST13 が先に入ったら rebase で `CATALOG`・`run()`・`docs/openapi.json` を解く（生成し直す）」を書く。D9 に呼ぶ場所（どちらの見回りか、両方か）を書く。

## R18. INDEX の capability 表に ST23 の `browsing-views` が無く、FR-50 を動かす理由も INDEX に無い
- 成果物: docs/stories/INDEX.md:90・openspec/changes/st23-record-erasure/proposal.md:125-126
- 根拠: INDEX:90 の `browsing-views` の行は「ST16, ST17, ST22, ST25, ST26, ST36, ST37」で ST23 が無い。proposal:125 は「capability の前倒しは無い…ST22 の前倒しの続き」と書くが、ST22 のときは INDEX:240 に訂正を書いている。
  また ST23 の satisfies は FR-51 / FR-52（docs/stories/ST23.md:4）だが、spec は FR-50 ★ 2026-09-29 の振る舞い（後着の印を取り込みの直後に付ける。spec:550 / :585-588）を担い、Q3 推奨は FR-50 ★ を改訂する
- kind: premise
- 処置: escalated —— `docs/stories/INDEX.md` を直した（change の外なので `fixed <file>` で指せない。ST03 の R の前例）。capability 表の `browsing-views` に ST23 を足し、訂正の節（2026-10-22）に理由と FR-50 ★ 2026-09-29 を満たすことを書いた。`deep.md` の末尾に R18 を置いた。FR-50 ★ と FR-52 の読み替えは Q3 の答えを受けて record の段が要件へ戻す
- 提案: INDEX に ST23 の訂正（`browsing-views` を割り当てた理由、FR-50 の申し送り R8 と Q3 で FR-50 ★ を読み替えること）を足し、表の行に ST23 を加える（record の段で要件と一緒に）。

## R19. 書き出しの分類に足す表の数が design と tasks で違う
- 成果物: openspec/changes/st23-record-erasure/design.md:107・tasks.md:44
- 根拠: D4 は「2 表を `Exclude` で足す」、Task 1.1 は「新しい 3 表を `Exclude` で足す」。3 本目は D11 の `core.purge_compaction`（design.md:175）で、D4 にも D11 にも分類が書かれていない
- kind: technical
- 処置: fixed D4 —— 書き出しの分類に足すのは 3 表（`core.erasure_operation`・`core.erasure_span`・`core.purge_compaction`）と理由を D4 に書き、tasks 1.1 と揃えた
- 提案: D4（か D11）に `core.purge_compaction` の分類と理由を書き、数を揃える。

## R20. 細かな食い違い（実装の名前・文面の時刻・正典の台帳の Requirement・断りの理由）
- 成果物: openspec/changes/st23-record-erasure/specs/record-deletion/spec.md:186 / :192 / :320・design.md:53 / :176 / :213
- 根拠:
  - spec:186 の「`rebuild:` で始まる値」は DB の値（正典 record-deletion:13 は「作り直しが付ける印と区別できる形」で止めている）。spec:192 の `stay_store.rs` の `update_stay`、:320 の `migrations/202609120944_gates.sql:83-85` は実装の名前
  - 正典の「消す・戻すは追記のみの台帳に 1 操作 1 行で残る」（openspec/specs/record-deletion/spec.md:97）は「印を付ける・外すたびに」で、D6 の**付け替え**（`rebuild:*` → `user:purged`）の行はどちらでもない。spec:186 は付け替えを台帳に残すと言うが、正典の Requirement を MODIFIED していない
  - D16 の「ディスクの痕跡は今夜の書き直しで消えます」と D11 の 03:30: 00:00〜03:30 に消すと書き直しは同じ晩の早朝で「今夜」とずれる（文面は観測できる振る舞い）
  - design.md:53 の 409 の理由に `purged_restore` が並ぶが、これは戻す口の理由（D6:131）
- kind: technical
- 処置: fixed D16 —— 文面を「次の書き直し（毎晩 3:30）」と範囲の時刻にした。D1 の 409 の理由から `purged_restore` を外した（戻す口の理由は D6）。spec の実装の名前（`rebuild:`・`update_stay`・移行のファイル名）は外してある。付け替えの台帳の行は正典の「印を付ける」に当たる（D6 に書いた）ので MODIFIED にしない
- 提案: spec から実装の名前を外す（「作り直しが付ける印でない、本人の印」で足りる）。正典の台帳の Requirement を MODIFIED して付け替えの行を入れる。文面を「次の書き直し（毎晩 3:30）で消えます」にするか D11 に合わせる。D1 の理由の列挙から `purged_restore` を外す。

## R21. 「推奨が採られたときに要件へ戻すもの」が proposal の散文にしかなく、triage が 0 と数える
- 成果物: openspec/changes/st23-record-erasure/proposal.md:86-90・deep.md
- 根拠: review_triage.py の結果「要件へ戻すもの 0 件」。proposal:86-90 は Q3（FR-50 ★・FR-52）・Q4（FR-51）・Q1（INDEX の宛先）の 3 件を挙げるが、deep.md の Q1 / Q3 / Q4 にも「効く先」の欄が無い
- kind: defer
- 処置: fixed defaults.md —— 末尾の「`record` の段へ」に、要件へ戻すもの 3 件と、record が各 Q の節に「要件へ戻すもの」の行を置くことを書いた
- 提案: record の段が答えを書くときに、各 Q に「要件へ戻すもの」の行を置く（triage が読めるように）。record が proposal を読む前提なら、その旨を defaults.md の末尾の「`record` の段へ」に 1 行足す。

---

## 観点ごとの確かめ

- **観点 1（deep の決定が正典に写っているか）**: Q1（spec:3-14・:112-113）・Q2（spec:107-114。ただし R2 / R5）・Q3（spec:267-309・MODIFIED :554。ただし R1 / R4）・Q4（spec:360-376・:55-57）・Q5（spec:433-491・browsing-views:46-58。44×44 px の数値は spec:435 / :453-456 にある）・Q6（spec:378-407）は推奨の形で Scenario がある。
  当初案を覆したものは第 1 回なので無い。要件へ戻すものは record の段の前で、requirements.md に ★ はまだ無い（R21）。
  C1〜C16 は D と spec に写っている（C1→spec:159-182 / C2→:184-207 / C3→:209-239 / C4→:342-358 / C5→:311-340 / C6・C15→:409-431 / C7→:88-105 / C8・C10→:112・:360-376 / C9→Task 10 / C11・C12→Non-Goals / C13→:241-265 / C14→:113・:149-152 / C16→:442・:488-491）。
  C3 の「読めない行は厳しい側」は spec:247 にあるが具体は D14 にしかない（R7）。
- **観点 2**: R11 / R12。
- **観点 3**: design の観測できる振る舞いは R4（D13 の範囲を見せる）・R7（D14）・R8（D8 の錠）・R11（D15 の断り）・R20（D16 の文面）。specs の実装の名前は R20。
  proposal の Capabilities（record-deletion / browsing-views）は `specs/` のディレクトリと一致。INDEX の表とは不一致（R18）。
- **観点 4**: R6・R13・R14・R15・R16。CT の絞り込み（`deletion_` `export_` `day_` `stay_` `api_` `archive_` `dedup_` `app_role` `export_excluded`）は既存の試験のモジュール名（crates/server/src/*_tests.rs）に当たり、`-- --list` の下限つきの定義（tasks.md:16-18）も形として成り立つ。
  `tools/check-immutable.d/`・`tools/smoke.d/`・`tools/seed.d/` は存在する（README あり）。`npm run test:e2e` は web/package.json:13 にある。人間の確認待ちは 0 本（実データの量を測る 7.1 の後段だけは R15）。
- **観点 5**: check_chain の観点 8 は通過。ST23.md の「価値」「完了の判定」は推奨の答えと矛盾しない（「本文が DB から消え」は SQL の見え方で判定し、ファイルの痕跡は Q6 の書き直しまで残ると spec:387 と確認に書いてある）。satisfies の外（FR-50）は R18。
- **観点 6**: 該当なし（`spec --draft`。`--answers` ではない）。
- **MODIFIED の写し**: 該当なし（確かめた範囲: record-deletion の「消したものを戻せる」「消した時間帯に後から届いた位置の記録は、削除済みとして入る」「「消した」の行から戻せる」、browsing-views の「1 日の並びを読み出せる」
  「日付を指定するとその日の滞在が時刻順に一覧で出る」の 5 本を正典と 1 行ずつ突き合わせた。正典の SHALL と Scenario（7 / 5 / 1 / 4 / 5 本）はどれも文言のまま残り、足しただけ）。
- **走っている Story**: R1（ST13 の取り込み器と同じ格納の口）・R17。ST09 の `deletion.rs` を変えない規律（st09 tasks.md:9 / :209 `git diff --quiet origin/main -- crates/server/src/deletion.rs`）は、ST23 が先に main に入ると ST09 の検証が落ちる（ST23 は `deletion.rs` に purge を足す）—— ST09 の検証は `origin/main` との差分なので、ST09 を rebase すれば通る。記録だけ。
- **新しく立った loss**: R2（別に消した滞在の巻き込み。D2 は仮でもない）・R4（D13 の和のはみ出し）・R5（D17 の生きた滞在）。どれも Q2 / Q3 の選択肢の文に無い失われ方。R2 と R5 は失わない側が安く P の条件に合うので、失わない側で仮にするのが筋。R4 は失わない側にも費用があり、A として問いに載せるのが筋。
