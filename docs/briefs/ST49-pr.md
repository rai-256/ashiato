# ST49 記録ごとに、作った部品の版が残る（上流）

change: `openspec/changes/st49-record-component-version/`。既存の 8 capability に ADDED（`record-envelope` 7・`device-collection` 2・`desktop-collection` 1・`derived-records` 2・`data-durability` 1・`record-deletion` 1・`subjective-log` 1・`personal-entities` 1。Scenario 66 本）。
満たす要件は FR-122・FR-123（扉 #32）。**`requires: [ST01, ST48]`**（この段で ST48 を足した。layer 1 → 2、ST52〜ST58 も 1 つずつ下がる）。

> **第 1 回の深掘りの答えを待たずに、推奨の答えで先に書いた**（2026-10-20 の流れ）。下の Q1〜Q4 は**本人の答えではなく推奨**で、`deep.md` は `（答え待ち・推奨: …）` のまま。
> 4 問とも A（失われるものがある）で、本人は答えを省略できない。答えが推奨と違えば `spec --answers` が仕様に写し直す。

## 深掘り（第 1 回。推奨で書いた。どれも A）

| Q | 推奨（仕様はこれで書いた） | どこに効くか |
|---|---|---|
| Q1 記録に残す「版」の中身 | 版番号・commit（40 桁）・その部品のソースの未 commit の印（`.dirty`）・組み立ての種類（`.debug`）。例 `0.1.0+<commit>.dirty.debug` | record-envelope の「組み立ての印は…」、design D5・D6、tasks 1・8.1・9.1・10 |
| Q2 収集アプリと衛星の記録にも受け取った S-01 の版 | すべての記録に S-01 の版（衛星の記録も。衛星自身の版は付けない） | record-envelope の 1 つ目、design D10、tasks 3 |
| Q3 版を持たせる表の範囲 | 破棄の報告・原本の欠けに収集アプリの版と S-01 の版、画面の操作の台帳（削除・気分の通知・滞在の基準・個人属性の種類の名前）に S-01 の版 | device-collection・record-deletion・subjective-log・derived-records・personal-entities・data-durability、design D7・D14・D15、tasks 5・6・7・9.3 |
| Q4 画面の組み立ての版 | 残す（画面の記録は画面の版と S-01 の版の 2 つ） | record-envelope の「部品の版は…資格情報で振り分ける」「画面は…」、design D9・D20、tasks 3・10 |

**要件との関係（record の段へ）**: 推奨は FR-122 / FR-123 と Story の完了の判定「衛星が書いた記録には版を付けない」を広げる（Q2・Q3・Q4）。`deep.md` 末尾の「record の段で要件に戻すかを決めるもの」に挙げてあり、
答えを受けた record が「要件へ戻すもの」に移して `docs/requirements.md` に ★ を入れ、ST49.md を貼り直す（review/spec.md R3）。

聞かずに決めたもの（`premises.md` の C1〜C14 → design の D）: 版は送信要求の 1 件ごとの欄で、積む時点で焼き込む（D13）/ 値は検査せず断らない（D8）/ 鍵・原文に混ぜない（D11）/ 版の無い記録を推定で埋めない（D12）/
収集アプリの版を信じるのは API の合言葉の要求だけ（D9）/ 印は組み立てのたびに作り直す（D6）/ 印の形は ST48 の台帳とそろえる（D5）/ 選び出す口・生存信号の版・ST50 / ST51 の分は作らない（D18）/ 走っている change と同じ箇所の触り方（D19）。

## 深掘り（第 2 回。本人の答え）

| Q | 本人の答え | どこに効くか |
|---|---|---|
| Q5 書き出しの版の列の説明の時点（仮の D4 が否・A / exported） | 今の中身を作った部品の版と書く（D1 に合わせる。推奨と同じ） | data-durability の書き出しの Requirement と Scenario「書き出しの説明書は、版を今の中身を作った部品の版と説明する」、design D4（D1（仮）に乗る。D1 を反転するなら説明も変える）、tasks 7.1・7.2 |

## 仮決め（design の（仮）と反転条件。`provisional.jsonl`）

- **D1（仮）** 外部サービス由来の更新では版が今の中身を作った版に動き、前の値は前の版に残る。反転条件: 最初の版で選び出したい事例が出たら本表を据え置き、前の版から読む
- **D2（仮）** 滞在の作り直しで原文が変わったら S-01 の版を作り直した版に替える。反転条件: 最初に書いた版で選び出したい事例
- **D3（仮）** 収集アプリの溜めは読み戻した行の知らない欄を捨てずに送る。入れ替えは S-01 を先に。代償: ST49 より前の版へ戻した間の版は落ちる（`docs/handoff/ST50.md` へ）
- ~~D4（仮）~~ → 別の席（`provisional-review.md`）が否とし、深掘り第 2 回 Q5 で本人が決めた（下の節）。仮ではなくなった
- **D21（仮）** 収集した記録の版の書き換えは本表の門の対象（前の版を積むときだけ直せる。直す前の値が残る）。spec レビュー R7 で premises の C8（門の外）を改めた。反転条件: 前の版を残さずに直したい量の直しが出たら門の外へ

## capability と requires

- INDEX.md は ST49 を `record-envelope`・`device-collection`・`desktop-collection` に割り当てていた。版を持たせる先が滞在と滞在の基準・書き出し・削除の台帳・気分の通知の台帳・個人属性の種類の名前に及ぶので、
  `derived-records`・`data-durability`・`record-deletion`・`subjective-log`・`personal-entities` に ST49 を足した（INDEX.md の訂正の節）。どれも ADDED で、走っている ST23・ST29・ST41 が MODIFIED している Requirement には触らない
- requires に ST48 を足した: S-01 の組み立ての印は ST48 の `crates/server/build.rs` の `build_commit()` に積み、移行は ST48 の D10（仮）の規則に従う。ST50 は requires に入れない（入れ替えの順で防ぎ、期限の後は ST50 が止める）

## tasks

11 Task（組み立ての印と S-01 / 移行と門 / 取り込み口と振り分け / 外部更新 / 滞在と基準 / 破棄の報告と台帳 / 書き出し / C-02 / C-01 / 画面（e2e）/ 申し送りと全体）。Scenario 66 本を全部名指し。
組み立て直しの確かめは `tools/check-build-stamp.sh`（repo を写した木で本物の部品を組み立て直す。節 `tool` / `server` / `windows` / `android`）。

## レビュー

- 深掘りの独立検査（review/deep.md）: 13 件 —— escalated 4（R2〜R5 → Q1・Q3・Q4）/ fixed 仮 4（D1〜D4）/ fixed 5
- spec の独立検査（review/spec.md）: 14 件 —— fixed 10 / fixed 仮 1（R7 → D21）/ escalated 2（R3 → Q2 と要件の戻し、R4 → Q3 の族に滞在の基準と個人属性の種類の名前を足した）/ followup ST23 1（R5 → `docs/handoff/ST23.md`。機械の止め `st49_ledger_inserts_name_server_version` も置いた）
- 機械の検査: `openspec validate --strict` OK / `check_chain.py` OK / `review_triage.py` OK / `spec_overlap.py check` OK（候補 0）/ `evidence.py lint` OK / `check_dep_waits.py` OK

## 走っている change との重なり（差し戻さない）

- ST48: requires。`build.rs` に積む
- ST41: 同じ `IngestRequest`・2 つの INSERT・`apply_external_update`・`push_version`・門と錠の関数・読み口・書き出しの一覧に列を足す。移行は当てる時点の本体を写し直す（D19）
- ST23: `ingest_request` の呼び出し元の引数（後から入る側が 1 つに畳む）と、削除の台帳の INSERT の `server_version`（申し送り）
- ST29: 衛星の資格情報は収集アプリの版にも画面の版にもしない（`docs/handoff/ST29.md` は下流の 11.1 で書く）
