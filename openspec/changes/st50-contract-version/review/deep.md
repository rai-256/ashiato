# ST50 深掘りの独立検査（deep-review）

対象: `openspec/changes/st50-contract-version/deep-questions.json`（検査の時点は Q1〜Q7。処置で Q8・Q9 を足した）と `premises.md`（検査の時点は P1〜P8・C1〜C9。処置で P9 を足した）。
手順 1〜5 は問いの一覧を見る前に、要件（FR-124 / FR-125 / FR-126 / FR-130 と、それが引く FR-8 / FR-9 / FR-10 / FR-105 / FR-119 / FR-121 / FR-122 / NFR-7・扉 32）と
コード（`crates/server`・`crates/collector-windows`・`collector-android`）でやり直した。

## 手順ごとの記録

- 手順 1（要件どうしの衝突）: 2 件。FR-125 の「上限を掛ける」× NFR-7 の「上限は C-01 だけ」（R10）。FR-130 の「最初の正式版までは版 1」× FR-105 のベータ版と、本人の環境を最初の正式版より前に配布の経路へ切り替えること（requirements.md:956）（R1）
- 手順 2（扉の幅）: ST50 の doors は空。ただし扉 32（requirements.md:1735-1740）が「送った版」を「作った時の版」に直した経緯（C7）が、Q1 の「送った契約の版」にそのまま当たる（R2）
- 手順 3（新たに立つ一方通行）: 契約の版を記録ごとに残すか（Q1。ただし R8）・知らない欄の扱い（Q2）・確かめる口の形と断りの状態符号（配った収集アプリに焼き込まれて外に出る。R7）・溜めを止める粒度（要求ごとか記録ごとか。R3）・既存の欄の型の変化が古い S-01 で `malformed` になり携帯が捨てる経路（R4）・原本の口の 409 の意味の衝突（R6）
- 手順 4（日常）: Q5・Q6 が受けている。足すのは、Q3 の「全部止める」で生存信号が止まると途絶の判定（`crates/server/src/outage_store.rs:103-110` が `core.heartbeat` を読む）と「サーバを更新してください」が同時に鳴りうることだけ（R5 に添えた）
- 手順 5（既存コードとの突き合わせ）: `IngestRequest`（`crates/server/src/ingest.rs:15`）に加えて `HeartbeatRequest`（`heartbeat.rs:11`）・`DropReportRequest`（`drops.rs:36`）・`MediaMissingRequest`（`media_missing.rs:30`）も `deny_unknown_fields` を持たない（`grep -n deny_unknown_fields crates/server/src/*.rs` は `lib.rs:1955/1983/2004` の 3 件だけ）。
  1 件の解釈の失敗はどの理由でも `malformed`（`lib.rs:1126-1131`）。携帯の `/ingest` の送り手だけが `malformed` を恒久として捨てる（`LocationService.kt:896` `dropPermanentlyRejected = true`、`Sender.kt:117-124`）。PC は 1 件も捨てない（`sender.rs:150-160`）。原本の口は 409 をどの本文でも `record_missing` と読み、7 日で欠けにして行を消す（`OriginalUploader.kt:287`・`:299-307`・`:267-270`）
- 手順 6（過去の取りこぼし）: 当てはまったのは「〜が解消するまで止める規則には、解消しないまま期限を越えたときを問う」（R3・R6・R9）、「他 Story の決定の波及・申し送り」（ST49 D3（仮）の反転条件。R9）、「推奨に失うものが無いと書く前に、版・鍵を使う箇所をすべて辿る」（R6・R9）。時計の族・保存場所の族は、ST50 の決定が時刻や手元の状態で一方向に動くものを持たないので該当なし（確かめた範囲: Q1〜Q7・C1〜C9 に時刻を入力にする規則は無い。C2 の「確かめた結果」を手元に覚えるかは Q4 が要求ごとの版で受けている）
- 手順 7（族の網羅）: Q2 の族「収集アプリと S-01 の版の組み合わせ」に、既存の欄の型・必須の変化（R4）、ベータ版・開発版の版（R1）、更新の前に溜めた古い形の未送信が新しい収集アプリの溜めに混ざる場合（R2・R3）が無い。原本の口が同じ断りを受ける場合（R6）も無い
- 手順 8（前提の確かさ）: R11・R12

---

## R1. 「正式版」の境界（ベータ版・開発版）が未決のまま、C3 は印 `v*` を「配った」と読み、FR-124 の「受け付け続ける」がベータ版の契約の版に及ぶかが決まっていない
- 成果物: openspec/changes/st50-contract-version/premises.md（C1・C3）/ openspec/changes/st50-contract-version/deep-questions.json（一覧に無い）
- 根拠: docs/requirements.md:1038（FR-124「**正式版として**出した送信契約の版」）・:1060-1063（FR-130「最初の**正式版**を出すまでの足し方は版 1 に含める」）・:966（FR-105 ベータ版は先行公開の印）・:956（「配る相手の前に、本人の環境をこの経路に切り替える」—— 本人の記録が最初の正式版より前のベータ版・開発版を通る）。C3 が倣う `tools/check-released-migrations.sh:18` は `git tag --list 'v[0-9]*'` で、ベータ版の印（例 `v0.1.0-beta.1`）も当たる（`echo v0.1.0-beta.1 | grep -c '^v[0-9]'` = 1）
- kind: irreversible
- loss: discarded
- 提案: 問いを 1 つ足す。「ベータ版で出した契約の版も FR-124 の『受け付け続ける』に入れるか / 版 1 を閉じるのは最初のベータ版か最初の正式版か」。入れない側を選ぶと、ベータ版の収集アプリが溜めた未送信が、その版を外した後の S-01 に断られ続け、携帯は 90 日で捨てる（FR-8）。閉じるのを正式版にする側では、ベータ版どうしの間で欄が増えても版が上がらず、守りは Q2 の答えだけになる。C3 の「印 `v*`」もこの答えに合わせて書き直す
- 処置: escalated —— Q8 を足した（ベータ版を「正式版」に含めるか。A・discarded）。C3 に Q8 への参照を足した

## R2. Q1 は「送った」契約の版を残すと問うが、欄の意味を読み分けるのに要るのは「作った時」の版。扉 32 が同じ取り違えを C7 で直している
- 成果物: openspec/changes/st50-contract-version/deep-questions.json（Q1）
- 根拠: Q1 の問い「収集アプリが**送った**送信契約の版…を記録 1 件ごとに残しますか」と why「古い意味か新しい意味かが要る」。docs/requirements.md:1736-1737（扉 32「C-01 / C-02 の記録は**作った時**の収集アプリの版」、★C7「『送った』版を『作った時』の版に」）・:1021-1027（FR-122 の★C7「未送信は更新の後に送られるので、送った時の版では、取り違えた版が作った記録に直した版が付く」）。Q4 の推奨は「各要求にも版を載せる」で、要求の版は送った収集アプリの版になる。ST49 の design D3（仮）（`openspec/changes/st49-record-component-version/design.md:51-56`）は、溜めに残った古い形の行をそのまま新しい収集アプリが送る
- kind: irreversible
- loss: uncaptured
- 提案: Q1 を「残すなら、その記録を**作った時**の契約の版（溜めの 1 行ごとに持つ）か、**送った時**の要求の版か」の選択に直す。送った時の版を残すと、更新の前に作った記録に新しい意味の印が付き、Q1 の目的（意味の読み分け）が逆に誤る。R8 と合わせて見直す
- 処置: escalated —— Q1 を「作った時 / 送った時 / 残さない」の 3 択に直し、推奨を作った時の版にした

## R3. 溜めを止める粒度（要求ごとか、記録ごとの版か）が問われていない。要求ごとに止めると、古い S-01 でも受け付けられる記録まで溜めに残り、携帯の上限で捨てられる
- 成果物: openspec/changes/st50-contract-version/deep-questions.json（一覧に無い。Q4 の推奨が暗に要求ごとを選んでいる）
- 根拠: FR-125（docs/requirements.md:1042-1047）「手元に残した記録には FR-8 / FR-9 の上限」。携帯の溜めは送り先ごとに 1 つ（`LocationService.kt:892-900` の `/ingest` の `Sender`）で、収集アプリを更新すると、更新の前の版で作った記録（古い S-01 も知っている形）と更新の後の記録が同じ溜めに並ぶ。Q4 の推奨「送る前に問い合わせ…受け付ける版に無い…どれでも送らずに手元に残す」は溜め全体を止める
- kind: irreversible
- loss: discarded
- 提案: 問いを 1 つ足す（または Q4 に選択肢を足し kind を改める）。「収集アプリだけ更新して S-01 が古いとき、更新の前に作った記録（作った時の契約の版を S-01 が受け付ける）だけは送るか、溜め全体を止めるか」。止める側の失うもの（携帯で 90 日を超えた分の破棄）を選択肢に書く。R2 の「作った時の版を 1 行ごとに持つ」が前提になる
- 処置: escalated —— Q9 を足した（溜め全体か記録ごとか。A・discarded。Q1 の作った時の版が前提）

## R4. 既存の欄の型・必須の変化は、古い S-01 で 1 件ごとの `malformed` になり、携帯はそれを捨てる。Q2 の族に無く、Q7 の「欄の追加だけを落とす」はこれを CI で通す（Q7 は open ではない）
- 成果物: openspec/changes/st50-contract-version/deep-questions.json（Q2 の family・Q7）
- 根拠: `crates/server/src/lib.rs:1126-1131`（`serde_json::from_value::<IngestRequest>` の失敗はどれも `IngestError::Malformed`）、`collector-android/.../Sender.kt:117-124`（`PERMANENT_ERRORS` に `malformed`）、`LocationService.kt:896`（`/ingest` は `dropPermanentlyRejected = true`）、`SenderTest.kt:146`（恒久の 6 種は捨てることを固定）。Q7 の選択肢 2 は「型の変化や欄の削除は通る」と書くだけで、通った先で記録が消えることを書いていない
- kind: irreversible
- loss: discarded
- 提案: Q2 の family の siblings に「版を上げずに既存の欄の型を変えた・必須の欄を外した（古い S-01 が `malformed` で断り、携帯が捨てる）」を足し、`decided` を Q7 にする。Q7 を `kind: irreversible`・`loss: discarded` にし、選択肢 2 の irreversible に「型を変えた・欄を外した要求を古い S-01 に送ると、携帯は記録ごと捨てる」と書く
- 処置: escalated —— Q2 の族に型の変化・必須の欄を外した場合を足し（decided: Q7）、Q7 を irreversible・discarded にして選択肢 2 に失うものを書いた

## R5. Q3 は open だが、「記録の取り込みだけを止める」側は、ほかの口に足した欄を古い S-01 で黙って失う（問いの why 自身がそう書いている）
- 成果物: openspec/changes/st50-contract-version/deep-questions.json（Q3）
- 根拠: Q3 の why「取り込みだけを止めると、ほかの口に足した欄は古い S-01 に黙って捨てられる」。`crates/server/src/heartbeat.rs:11`・`drops.rs:36`・`media_missing.rs:30` はどれも `deny_unknown_fields` を持たない。足される欄は既に決まっている: FR-122 の★（docs/requirements.md:1025-1026）は破棄の報告・原本の欠けにも収集アプリの版を持たせ（ST49）、FR-121 の★（:1019-1020）は生存信号に動いている版を持たせる（ST51）。
  日常の面: 全部を止めると生存信号も止まり、途絶の判定（`crates/server/src/outage_store.rs:103-110` が `core.heartbeat` を読む）による途絶の知らせと「サーバを更新してください」が同時に出うる
- kind: irreversible
- loss: uncaptured
- 提案: Q3 を `kind: irreversible`・`loss: uncaptured` にし、選択肢 2 に `irreversible`「生存信号・破棄の報告・原本の欠けに足した欄（収集アプリの版・動いている版）は、古い S-01 に届くと失われる」を書く。選択肢 1 の detail に、途絶の知らせも同時に出ることを添える
- 処置: escalated —— Q3 を irreversible・uncaptured にし、選択肢 2 に失うもの、選択肢 1 に途絶の知らせを書いた

## R6. Q4 の推奨「受け付けない版を 409 で断る」は、Q3 の推奨（原本の口も止める）と組むと、携帯の原本の送り手が 409 を `record_missing` と読み、7 日で原本を欠けにして送り待ちから消す
- 成果物: openspec/changes/st50-contract-version/deep-questions.json（Q4・Q3）
- 根拠: `crates/server/src/media.rs:326`（`RecordMissing` → `StatusCode::CONFLICT`）、`collector-android/.../OriginalUploader.kt:287`（`HTTP_CONFLICT -> recordMissing(row)`。本文を見ない）・`:299-307`（最初の 409 から 7 日で `settle`）・`:267-270`（`settle` は欠けを報告して行を消す）。Q4 の why「確かめ方と状態符号は、後から作り直せる（仮でよい）」
- kind: irreversible
- loss: discarded
- 提案: Q4 の推奨の状態符号を「口ごとに、その口でまだ意味を持たない符号」に限ると書くか、原本の口だけ別の符号にする選択肢を足す。少なくとも context に「原本の口の 409 は既に `record_missing`（7 日で欠け）」を書き、Q4 を open のままにするなら、409 を選んだ場合の失うもの（原本）が起きない理由を書く
- 処置: escalated —— Q4 の推奨の状態符号を 426 に直し、context に原本の 409 = record_missing を書いた。premises に P9 を足した

## R7. 確かめる口の形と断りの符号は、最初に配った収集アプリに焼き込まれて外に出る。Q4 の「後から作り直せる」は最初の正式版までしか成り立たない
- 成果物: openspec/changes/st50-contract-version/deep-questions.json（Q4）/ premises.md（C2・C4）
- 根拠: C2「答えが受け付ける版として読めないとき（404・読めない本文・版が無い）は、受け付けないものとして送らずに手元に残す」。配った収集アプリはこの読み方のまま残る（premises.md の冒頭「古い側として残り、直せない」）ので、後の S-01 が口の道・答えの形（C4 の「整数 1 つ」）を変えると、配った収集アプリは全部が「受け付けない」と読んで送るのを止め、携帯は 90 日で捨てる（FR-8。docs/requirements.md:181-186）。P7 のとおり今は 0 個なので、決めるのは最初の正式版に入れる形
- kind: irreversible
- loss: exported
- 提案: Q4 の why を「最初の正式版までは作り直せる。正式版に入った口の道・答えの形・断りの符号は、以後の S-01 が永久に保つ（FR-124 と同じ扱い）」に直し、選択肢の irreversible に書く。答えの形（整数 1 つか、受け付ける版の一覧か）を C4 のまま聞かずに置くなら、その形を FR-124 と同じく変えないと design に書く
- 処置: escalated —— Q4 を irreversible・exported にし、why と選択肢に「正式版に入った後は変えられない」を書いた。答えの形（整数 1 つ）を context に書いた

## R8. Q1 の loss は大半が再計算で戻る。ST49 より前の記録は C1 で全部が版 1、ST49 の後の記録は収集アプリの組み立ての印（commit 40 桁）から契約の版を引ける
- 成果物: openspec/changes/st50-contract-version/deep-questions.json（Q1）
- 根拠: Q1 の選択肢 2 の irreversible「ST49 の印が無い・不明の記録は、どの版の契約で届いたかを永久に分けられない」。だが印の無い記録は ST49 より前 = 最初の正式版より前で、C1（premises.md）と FR-130 の★（docs/requirements.md:1063）により全部が版 1。ST49 の印は `<版番号>+<commit 40 桁>[.dirty][.debug]`（`openspec/changes/st49-record-component-version/design.md:73-78`）で、作った時の版（扉 32）なので、その commit の契約の版の定数を読めば作った時の契約の版が出る。戻らないのは commit が `unknown`（同 :75）か、配布元に無い手元だけの commit で組んだ記録に限られる
- kind: irreversible
- loss: uncaptured
- 提案: Q1 の why と選択肢 2 の irreversible を「戻らないのは commit が `unknown`・手元だけの commit の記録だけ」に絞って書き直すか、それが配った版では起きない（配る組み立ては commit を持つ）なら `loss` を外して B（仮）に落とし、推奨を既定にする。残す場合は R2 のとおり作った時の版にする
- 処置: escalated —— Q1 の why・context・選択肢 3 の失うものを「commit が unknown・手元だけの commit の記録」に絞った。作った時の版を残す側を推奨にしたので A のまま（Q9 の前提にもなる）

## R9. Q2 の「契約に無い欄」が、要求が名乗る版の契約か、S-01 が知る欄かで決まっていない。名乗る版で断る読みでは、収集アプリを戻した場合（族の未決の兄弟）に、最新の S-01 が断り続け「サーバを更新してください」が誤った案内になる。推奨の irreversible「なし」は成り立たない
- 成果物: openspec/changes/st50-contract-version/deep-questions.json（Q2）
- 根拠: Q2 の family の兄弟「収集アプリを古い版へ戻した（…ST49 の D3（仮）は知らない欄を捨てずにそのまま送る）」は `decided: 未決 Q2` だが、選択肢はこの場合の帰結を書いていない。ST49 の design D3（仮）の反転条件（`openspec/changes/st49-record-component-version/design.md:56`「知らない欄を送り返したことで S-01 が要求を断る事例…が出たら、送る前に S-01 の契約の版で欄を落とす（ST50 の契約の版に任せる）」）は、Q2 の推奨が断る側を選ぶと立つ —— 立つと欄は収集アプリの側で落ちる（uncaptured）。名乗る版で断る読みでは、戻した収集アプリ（版 N）が溜めの新しい形の行を送るたびに、最新の S-01 は「版 N に無い欄」で断る。S-01 を更新しても直らず（直るのは収集アプリを上げ直すこと）、携帯は 90 日で捨てる
- kind: irreversible
- loss: discarded
- 提案: Q2 の問いに「契約に無い」を「S-01 が知らない欄」と「要求が名乗る版に無い欄」のどちらで判定するかを足すか、選択肢を分ける。選択肢 1 の irreversible に、戻した収集アプリの場合の帰結と、ST49 D3（仮）の反転条件が立つことを書く。FR-126 の文言（「サーバを更新してください」）がこの場合に誤ることも context に書く
- 処置: escalated —— Q2 の判定を「S-01 が知らない欄」にし、名乗る版で判定しない理由・ST49 D3（仮）の反転条件の扱いを context に、推奨の失うもの（携帯の上限）を書いた

## R10. FR-125 は手元に残した C-01 / C-02 の記録に 90 日・2 GB を掛けると書き、NFR-7 は PC に上限を置かないと書く。C5 が聞かずに後者を採っている
- 成果物: openspec/changes/st50-contract-version/premises.md（C5）
- 根拠: docs/requirements.md:1042-1045（FR-125「IF C-01 / C-02 の…」「**手元に残した記録には FR-8 / FR-9 の上限（90 日・2 GB）を掛ける**」）・:1220-1221（NFR-7「**上限は C-01 だけ**で、C-02 の未送信には置かない」）・:181（FR-8 は C-01 だけを主語にする）。`crates/collector-windows/src/outbox.rs:4`「上限も破棄も持たない」。docs/stories/ST50.md の完了の判定「携帯の画面と PC の通知に…出る（上限を超えた分だけが捨てられる）」は PC にも上限があるように読める
- kind: conflict
- 提案: 失わない側（PC は上限なし）なので問いにはしない。代わりに C5 を仮の決定として台帳に載せ（rests_on: NFR-7・FR-8 の主語）、FR-125 の★に「上限は FR-8 の主語（C-01）に掛かる」と 1 行足す要件の訂正を申し送る。Story の完了の判定の括弧も「携帯は上限を超えた分だけ」に直す
- 処置: fixed D13 仮 —— premises の C5 に食い違いを書き、spec の段で D<n>（仮）として台帳に載せ、FR-125 の★の訂正を「要件へ戻すもの」に挙げると書いた（問いにしない）

## R11. premises.md は「失われるものを持つ決定は P1 にだけ載せる」と書くが、Q2 の推奨の irreversible「なし」（今配っている収集アプリも捨てない）は P4・P5（確かさ 2）に載っている。P1 の出典の行も違う
- 成果物: openspec/changes/st50-contract-version/premises.md（P1・P4・P5 と表の下の段落）
- 根拠: Q2 の context「今の携帯の収集アプリは知らない種別を恒久とみなさず手元に残し（premises P4）、PC…（P5）ので、今配っている収集アプリも捨てない」。P4・P5 は「実物（コード）」で 2。走らせれば 4 にできる既存の試験がある: `collector-android/app/src/test/kotlin/dev/ashiato/collector/SenderTest.kt:125`（`brand_new_error_kind` を捨てない）・`:176`（403・429・5xx で取り除かない —— 409 は同じ分岐 `Sender.kt:255`）。
  P1 の「`lib.rs:1885` が `serde_json::from_value::<IngestRequest>` で読む」は滞在の作り直し（`rebuild_stays_after_ingest`）の行で、受け付けを決める読みは `lib.rs:1126`（`ingest_one`）。そこでは解釈の失敗が 1 件ごとの `malformed` になる（`:1128-1131`）ので、`deny_unknown_fields` を足すだけの実装は Q2 の選択肢 3 の「要求全体を `malformed`（400）」ではなく、1 件ごとの `malformed` として携帯に捨てられる
- kind: premise
- 提案: P4・P5 を既存の試験（携帯は `SenderTest` の 2 本、PC は `crates/collector-windows/src/sender.rs` の試験）を走らせて 4 に上げるか、表の下の「P1 にだけ載せる」を「Q2 の推奨は P4・P5 にも載る」に直す。P1 の出典を `lib.rs:1126-1131` に直し、Q2 の選択肢 3 の説明を「1 件ごとの `malformed`（`deny_unknown_fields` を足すだけの実装はこれになる）」に直す
- 処置: fixed premises.md —— SenderTest（携帯）と sender:: の 11 本（PC）を走らせ、P3・P4・P5 を確かさ 4 に上げた。P1 の出典を lib.rs:1126-1131 に直し、Q2 の選択肢 3 を 1 件ごとの malformed（deny_unknown_fields を足すだけの実装）に直した

## R12. Q7 の「比べるのは `docs/openapi.json` の、収集アプリが送る要求の型」は誤り。openapi は S-01 が受ける型で、収集アプリだけが足した欄（黙って捨てられる当の欄）はそこに出ない
- 成果物: openspec/changes/st50-contract-version/deep-questions.json（Q7 の context）
- 根拠: `docs/openapi.json` は `crates/server/src/bin/openapi.rs` が S-01 の型から作る（`tools/check-openapi.sh:6`）。`tools/check-openapi.sh:23-27` は「openapi の欄が Kotlin にあるか」の片方向だけで、Kotlin（`IngestRequest.kt`）にあって openapi に無い欄は見ない。PC の封筒（`crates/collector-windows/src/contract.rs:301` `IngestRequest`）はこの検査の対象外（`grep -n "collector-windows\|contract.rs" tools/check-openapi.sh` は 0 件）。ST49 の design D3（仮）は PC の型に知らない欄の置き場（`#[serde(flatten)]`）を足す
- kind: premise
- 提案: Q7 の context を「比べるのは S-01 の型（openapi）と、収集アプリ 2 本が実際に直列化する欄」に直す。収集アプリの側で欄が増えたのに版も S-01 の型も変わっていない場合を検査が落とすことを、選択肢 1 の detail に入れる
- 処置: fixed deep-questions.json —— Q7 の context を「S-01 の型と収集アプリ 2 本が直列化する欄の両方」に直し、選択肢 1 の detail に収集アプリだけが欄を足した場合を書いた

## 該当なしの確認

- 不要な問い: 該当なし（確かめた範囲: Q1〜Q7 のどれも、FR-124〜FR-126・FR-130 の本文で幅なく決まってはいない。Q6 の「画面だけ」は FR-126 の逐語だが、通知を足すかは要件が決めていない）
- 文字で決めさせている画面の問い: 該当なし（確かめた範囲: Q5・Q6 は通知の頻度と出し先で、格子・寸法・同時に見えるものを問うていない）
- 既定の無い仮の問い: 該当なし（確かめた範囲: loss の無い Q3〜Q7 はすべて `recommended: true` の選択肢を 1 つ持つ）
