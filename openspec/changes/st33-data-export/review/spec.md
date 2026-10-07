# ST33 spec レビュー（spec-review）— st33-data-export

対象: `proposal.md` / `specs/data-durability/spec.md` / `design.md` / `tasks.md`（`deep.md`・`deep-answers-1.txt`・`review/deep.md`・`proto.html` と突き合わせ）。
正典: `openspec/specs/record-deletion/spec.md`。要件: `docs/requirements.md` NFR-16。Story: `docs/stories/ST33.md`。

## 機械の検査（2026-10-08 実行）

| コマンド | 結果 |
|---|---|
| `openspec validate st33-data-export --strict` | `Change 'st33-data-export' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0。再生成との一致を含む） |
| `python3 scripts/check_scenarios.py . st33-data-export` | `FAIL (担保なし 57 件)`（rc=1）。57 本すべてが この change の Scenario で、テストが未実装のため。tasks の宣言（57 本 = サーバ 45 + 画面 12）と件数は合う。閉じるのは Task 10.1 |
| `python3 scripts/review_triage.py . st33-data-export` | `triage: OK`（指摘 12 件 / 要件へ戻すもの 1 件。`review/deep.md` だけを数えた時点） |

---

## R1. 除く範囲の「時刻の幅」を `payload.end` だけで読むので、書庫の滞在・移動（`end_time`）が消した時間をまたいでも出る
- 分類: 観点 1（Q5 の担保）/ 観点 2
- 成果物: openspec/changes/st33-data-export/design.md（D5）、specs/data-durability/spec.md
- 根拠: design.md:111 は幅を `[event_time, coalesce(core.try_timestamptz(payload->>'end'), event_time)]` と定める。spec.md:112 は「時刻に幅を持つ記録（滞在など）は、その幅が消した滞在の時間と重なるとき『時間に入る』」と言う。一方 ST12 の design（openspec/changes/st12-archive-ingestion/design.md:157-163）では `c03-timeline-visit` / `c03-timeline-activity` / `c03-legacy-visit` / `c03-legacy-activity` は出来事の時刻が `startTime` で、終わりは `end_time`。この 4 つは緯度経度を持つ（visit は `lat` / `lng`）。`end` という鍵が無いので D5 では点の扱いになり、消した 01:00〜02:00 をまたぐ 00:30〜03:00 の訪問（event_time 00:30）は除かれない。proposal.md:102-103 は「ST33 は印でなく時間と緯度経度で除くので、ST12 の実装を変えずに成り立つ」と書くが、ここで成り立たない。spec の Scenario（spec.md:128-136）には、event_time が消した時間より前で、幅だけが重なる「滞在でないソース」の行が無い
- kind: technical
- loss: exported
- 提案: 幅の終わりの鍵をソースごとに分類へ持たせる（例: D5a と同じ定数に `end` / `end_time` を足す）。分類の無いソースは D8 と同じく「分類が無ければ書き出しを始めない」にするか、知っている鍵（`end` / `end_time` / `endTime`）を全部見る。spec に Scenario を 1 本足す: 「消した時間より前に始まり、中で終わる・またぐ、緯度経度を持つ別のソースの区間の記録は出ない」
- 処置: escalated — deep.md 第 2 回 Q6（deep-questions-r1.json）。spec と design D5 は推奨（幅で重ねて除く。`END_KEYS`）の形で書いてあり、Scenario「消した時間をまたぐ、別のソースの幅を持つ緯度経度つきの記録は出ない」を足した

## R2. 除く範囲を「同じ `user_id`」に絞っているが、`user_id` は収集側ごとに設定する値で、食い違うと書庫・PC の行が漏れる
- 分類: 観点 2 / 観点 3（観測可能な振る舞いが design にだけある）
- 成果物: design.md（D5 / D12）、specs/data-durability/spec.md
- 根拠: design.md:110 は「同じ `user_id` の本人が消した滞在 `s` があり」と書く。spec.md:111 は利用者に触れない（「ソースを問わず」とだけ書く）。`user_id` はクライアントが送る値で、収集側ごとに別に設定する: C-02 は `ASHIATO_USER_ID`（crates/collector-windows/src/config.rs:110）、書庫は `ASHIATO_ARCHIVE_USER_ID`（st12 design.md:59）、Android は `userId`（LocationFix.kt:66）。サーバはこれらが一致することを確かめない。書き出しは全利用者を 1 つのフォルダにまとめる（D12、design.md:232）ので、利用者で絞っても他人の行が出ない保証にはならず、絞ると漏れる経路だけが増える
- kind: technical
- loss: exported
- 提案: 既定を厳しい側にして、利用者を問わず消した時間で除く（1 つのフォルダに全利用者を書く D12 と矛盾しない）。利用者で絞るなら、spec に「別の `user_id` で入った同じ時間の緯度経度つきの行」の Scenario を置き、どちらに倒すかを書く。どちらに決めても spec の Requirement に明記する
- 処置: escalated — deep.md 第 2 回 Q7。spec と design D5 / D12 は推奨（利用者を問わず除く）の形で書いてあり、Scenario「別の利用者の識別子で入った同じ時間の位置も出ない」を足した

## R3. 親が Q5 で除かれた前の版が、自分の時刻が外なら出てしまい、spec の「親が入るものだけ」と「食い違わない」に反する
- 分類: 観点 2 / 観点 3
- 成果物: design.md（D5 / D7）
- 根拠: spec.md:62「前の版を、親の記録が書き出しに入るものだけ出す」、spec.md:239「前の版のファイルのどの行も、記録のファイルにある親を指し」。design.md:108-112 の除外条件は、前の版 `v` に (1) `core.event_version_live` に無い (2) `v` 自身の時刻と緯度経度 だけを当てる。design.md:145 の元も「`core.event_version_live`（D5 で除いた残り）」。**親が Q5 の条件で除かれたとき（親は印が無く生きている）に前の版も落とす条件が無い**。例: いまの版が消した時間に重なり、前の版は外にある滞在（D6 の「残る穴」の間、作り直しがまだ隠していないもの）や、書庫の再取り込みで時刻が動いた記録
- kind: technical
- 提案: D5 に「前の版は、親が (1) か (2) で出ないなら出さない」を足し、`event_versions.jsonl` を `events` の残りと結合して作る。spec の「前の版は親が書き出しに入るものだけ出る」（spec.md:93-96）に、親が Q5 で除かれた場合の 2 本目の WHEN を足す
- 処置: fixed D5 — 前の版は親が除かれたら除く（親の 1 / 2 のどちらでも）。spec の Requirement に括弧書きと Scenario「親が消した場面の規則で除かれると、前の版も出ない」を足した

## R4. 「緯度経度を持たない」と決めた一覧に、URL を持つソース（`c02-window` / `c02-browser-history`）が入っている。地図の URL は座標を文字列で持つ
- 分類: 観点 1（Q5 の前提）
- 成果物: design.md（D5a）
- 根拠: design.md:123 の一覧に `c02-window` と `c02-browser-history` がある。design.md:126 は「足すのは、そのソースの payload に緯度経度が無いことを…確かめた Story」と規律を置いている。しかし ST08 の訪問の payload は `url` を「DB の文字列そのまま」で持ち（openspec/changes/st08-browser-history/design.md:107）、`c02-window` もアドレスバーの URL を持つ（CLAUDE.md の構成表「ブラウザのアドレスバー（`platform.rs` `browsers.rs`）」）。地図サービスの URL（`/@35.68,139.76,15z` の形）は、消した場面の緯度経度を文字列で持つ。2 段目（design.md:127）は最上位の鍵しか見ないので拾えない。Q5 の答えは「緯度経度を持つ行も除く」で、「緯度経度の鍵を持つ行」ではない
- kind: premise
- loss: exported
- 提案: (a) 既定を厳しい側にして、URL を持つ 2 ソースを一覧から外す（消した時間の中のブラウザの行が出なくなる。出さない側なので後から広げられる）、または (b) URL・題名の中の座標の形を 2 段目で見る。どちらを採るかは「緯度経度を持つ」の読み方なので、(a) を既定にして D 番号に残し、(b) へ広げるときに本人に問う
- 処置: escalated — deep.md 第 2 回 Q8（第 1 回 Q5 の context の前提の誤り）。spec と design D5a は推奨（項目名をどの深さでも見る・原文の中の座標の並びも数える）の形で書いてあり、Scenario 2 本を足した。並びの形は D5a（仮）

## R5. spec の「応答に載せてよいのは件数・大きさ・時刻・置き場のパスだけ」に、design の応答（ソース名・表示名・`export_id`・失敗の種別）が入らない
- 分類: 観点 2 / 観点 3
- 成果物: specs/data-durability/spec.md、design.md（D1）
- 根拠: spec.md:14「応答に載せてよいのは件数・大きさ・時刻・置き場のパスだけとする」、Scenario spec.md:36「置き場のパス・件数・大きさ・時刻だけが返る」。design.md:49-51 の応答には `export_id`、`logical_source`、`display_name`、`last_failure.kind`、`running` / `interrupted` がある。`GET /export/summary` はソースごとの件数の口なのでソース名を返すしかない。spec の文面のままだと、Scenario の「だけ」は design の形で必ず偽になる
- kind: technical
- 提案: spec の禁止を、許す欄の列挙から「記録の値（原文・内容・緯度経度・本文）を載せない」へ書き換える。Scenario の THEN は「取り込んだ記録の原文・緯度経度・本文のどれも含まれない」の 1 主張にする（tasks 6.4 の検証はすでにこの形）
- 処置: fixed specs/data-durability/spec.md — 禁止を「記録の値（原文・内容・緯度経度・本文）を載せない」に書き換え、Scenario の THEN を 1 主張にした。D1 にも同じ言葉

## R6. 「書き出しの名前」と、入らない表のファイル名が spec で決まっていないので、2 本の Scenario が spec だけでは真偽を決められない
- 分類: 観点 2
- 成果物: specs/data-durability/spec.md
- 根拠: (a) spec.md:15 / spec.md:38-41「書き途中のフォルダは…名前で見分けられ」。spec が決めているのは「書き出した日時を名前に持つ」（spec.md:12）だけ。design.md:79 の `.partial-ashiato-export-YYYYMMDD-HHMMSS` も日時を名前に持つので、spec の言葉では両方とも「書き出しの名前」に当たる。見分け方は design にしか無い。(b) spec.md:177「削除の台帳・消去の台帳・書き出しの台帳・読み出しの記録・日ごとの件数のファイルが無い」。入らない表のファイル名はどこにも決まっていないので、どんな名前で書いても「無い」と判定できてしまう
- kind: technical
- 提案: (a) spec に「書き終えた書き出しのフォルダの名前は `.` で始まらない / 書き途中は印の前置きを持つ」の形で区別を 1 行書く（具体の綴りは design のままでよい）。(b) THEN を含める側で書く: 「フォルダのファイルは、記録・前の版・辞書・稼働の元データのファイルと説明書・目録だけで、それ以外は無い」
- 処置: fixed specs/data-durability/spec.md — (a) 書き終えたフォルダは `.` で始まらず、書き途中は `.` で始まる前置き、を Requirement と Scenario に書いた。(b) 「台帳と導出の表は書き出しに入らない」の THEN を含める側の一覧で書いた

## R7. 置き場の既定（C16）が spec に無い。archive で正典から落ちる
- 分類: 観点 1 / 観点 3
- 成果物: specs/data-durability/spec.md、design.md（D3）
- 根拠: deep.md:82 は Q1 の効く先を「specs `data-durability`（書き出しの置き場の Scenario）」とする。C16（deep.md:152）は既定を `%LOCALAPPDATA%\ashiato\export` に決め、同期されうるフォルダを既定にしない。spec.md:20 は「置き場の既定が同期されうるフォルダでないことは design に置く」と自分で外へ出している。design.md:75-77 には、既定の決め方・環境変数で変えられること・どれも無いときは失敗にして起動は止めないこと、がある。どれも観測できる振る舞い（どこにファイルができるか）で、`openspec archive` は design を正典へ運ばない
- kind: technical
- 提案: spec に Requirement の 1 文と Scenario を 2 本足す: 「置き場の指定が無ければ、利用者のローカルのアプリのデータのフォルダの下（同期されるドキュメント・デスクトップでない）に置く」/「置き場を環境で指定すればそこに置く」。解決の順と Linux の代わりの置き場は design に残してよい
- 処置: fixed specs/data-durability/spec.md — 置き場の既定（ローカルのアプリのデータのフォルダの下・同期されうるフォルダを既定にしない）と環境での指定を Requirement に入れ、Scenario 2 本を足した。決める順と Linux の代わりは D3 に残した。tasks 2.1

## R8. 何を「緯度経度を持つ」とみなすか（ソースの一覧・鍵の名前）が design にしかない。出すか出さないかを決める規則なのに正典から落ちる
- 分類: 観点 3
- 成果物: design.md（D5 / D5a）、specs/data-durability/spec.md
- 根拠: design.md:112 は鍵を `lat`/`lon`/`lng`/`latitude`/`longitude` とし、design.md:123 で 6 ソースを「持たない」と決めている。spec.md:114-116 が言うのは「緯度か経度の項目」「PC のウィンドウなど」だけ。Scenario（spec.md:153-156）は `lat` と `lon` を両方持つ形しか測らないので、`lng` だけ・`latitude` だけを見落とす実装でも緑になる。一覧のソースを 1 つ足すと出る行が変わる（外に出て戻らない）のに、その規則は正典に残らない
- kind: technical
- 提案: spec に「緯度経度を持たないと決まったソース」の一覧（ソース名ではなく種類でよい: ウィンドウ・ブラウザ・アプリの使用・時計・属性）と、鍵の名前の一覧を Requirement として置く。Scenario は `lng` だけ・`latitude` だけを持つ行を 1 本ずつ足すか、1 本の中で鍵ごとに 1 行ずつ確かめる形にする
- 処置: fixed specs/data-durability/spec.md — 「緯度経度を持つ」の判定 (1) ソースの種類 (2) 項目名の一覧 (3) 原文の座標の並び を Requirement に置き、Scenario を `lng` だけ・入れ子の `latitude`・`LatitudeE7` の 3 形にした

## R9. 画面が書き出しの失敗と「途中で止まった」をどう出すかが、design にしかない
- 分類: 観点 3
- 成果物: design.md（D13 / D11）、specs/data-durability/spec.md
- 根拠: design.md:241「失敗は節の中に 1 行（『書き出しに失敗しました（<種別>）』）。押し直せる」。spec の画面の Requirement（spec.md:338-347）と Scenario 12 本には、失敗・途中で止まった（`interrupted`）の見え方が 1 つも無い。サーバ側は spec.md:288-289 で状態として返すと約束しているのに、画面がそれを出すかは正典に無い。e2e（Task 9）にもこの場面が無い
- kind: technical
- 提案: spec の画面の Requirement に「最後の書き出しが失敗した・途中で止まったときは、節の中にそのことを出し、もう一度押せる」を足し、Scenario を 1〜2 本置く（e2e は 9.3 と同じく `page.route` で `last_failure` / `interrupted` を返して確かめられる）
- 処置: fixed specs/data-durability/spec.md — 画面の Requirement に失敗と途中で止まったの 1 行と押し直しを足し、Scenario を 1 本置いた（e2e は 9.3 の `page.route`）。D13 に文言

## R10. 前の版のファイルの列が Scenario で測られていない。ビューにしか無い親の 3 列（`sensitivity` / `origin` / `external_id`）を出すかも決まっていない
- 分類: 観点 2 / 観点 3
- 成果物: specs/data-durability/spec.md、design.md（D7 / D8 / D9）
- 根拠: spec.md:58 は「前の版の行には、前の版の表が持つ列のうち…トランザクションの番号を除くすべての列」と言うが、これを測る Scenario が無い。列の Scenario は記録の側だけ（spec.md:68-71）。`core.event_version_live`（migrations/202609120943_version_and_ledger.sql:78-90）は `txid` を持たず、代わりに親の `sensitivity` / `origin` / `external_id` を足している。D8 の `columns` と D9 の突き合わせは表（`core.event_version`）の列を基準にするので、この 3 列は分類の外になる。出すなら README の列の意味（D8 から引く）が無く、出さないなら spec の「表の列と同じ名前」の形と合う。どちらかが書かれていない。`txid` を外すこと自体は spec・D7・ビューの 3 か所で揃っている
- kind: technical
- 提案: Scenario を 1 本足す: 「前の版のファイルの行は、前の版の表の列から内部の番号を除いた列を持ち、それ以外の列を持たない」。親の 3 列は D7 / D8 で「出さない（親は `events.jsonl` にある）」か「出す（意味を CATALOG に書く）」のどちらかに決めて書く。heartbeat の除外列（`attempts`/`successes`/`raw`/`content_hash`）は spec.md:63・spec.md:106・D7 で揃っている（指摘なし）
- 処置: fixed D7 — 親の 3 列は出さない（表の列ではない。親は `events.jsonl` にある）。spec の Requirement に「それ以外の列（親の記録の列）を持たせない」と Scenario「前の版の行は前の版の表の列から内部の番号を除いた列を持つ」を足した。tasks 5.1

## R11. Requirement の本文だけにあって Scenario が測らない主張
- 分類: 観点 2
- 成果物: specs/data-durability/spec.md
- 根拠:
  - spec.md:202 説明書と目録の「写しの時点・書き出した時刻・記録の形の版」→ Scenario spec.md:211 は列・行の数・入らないもの・畳む規則しか見ない
  - spec.md:169 出さない列を説明書の「入らないもの」に名前で挙げる → Scenario が無い（heartbeat の回数を外したことが説明書から読めないままでも緑）
  - spec.md:60 時刻を UTC で書く → Scenario spec.md:78-81 は 1 列の時刻しか見ない（`ingest_time` / `source_updated_at` / 前の版の `superseded_at` / 稼働の元データの時刻は測られない）
  - spec.md:226 写しの時点の後に**付いた削除の印**を反映しない → Scenario spec.md:231-234 は取り込みだけ（書き出しの途中で消した滞在がその書き出しに残る形は測られない）
  - spec.md:262 台帳に記録の中身を持たせない、spec.md:260 経路（自宅 PC のフォルダ）を持つ → Scenario が無い
  - spec.md:165 の除外の一覧に `web_session`（ログインの印の SHA-256）が無い。D8（design.md:168）だけが `Exclude` にしている
  - C9（deep.md:145。期間・ソースで絞る口を作らない）が spec の文に無い（spec.md:19 の導出元に番号があるだけ）
- kind: technical
- 提案: 上の各項目に 1 本ずつ Scenario を足す（どれも Rust のテストで測れる）。`web_session` は spec.md:165 の一覧に「画面のログインの印」として足す。C9 は「書き出しの求めは範囲を受け付けない（期間・ソースの指定は無視しないで 400 で断る、または指定する手段が無い）」のどちらかで 1 文にする
- 処置: fixed specs/data-durability/spec.md — Scenario を足した: 説明書の写しの時点・時刻・版（既存の Scenario の THEN に足した）/「出さない列は説明書の入らないものに名前で挙がる」/「どのファイルのどの時刻の列も UTC で書かれる」/「書き出しの途中で付いた削除の印はその書き出しに反映しない」/「台帳の行は経路を持ち、記録の中身を持たない」/「範囲を指定した書き出しの求めは断られる」（C9。D1 に 400）。`web_session` は除く一覧に「画面のログインの印」として足した

## R12. 遅着の印（D6）の範囲が半開区間なので、最後の消した滞在のちょうど終わりの時刻の位置に印が付かない
- 分類: 観点 2（Scenario「書き出しの前に印が付く」）
- 成果物: design.md（D6）
- 根拠: design.md:133 は「`mark_late_arrivals` を、本人が消した滞在の最小の始まり〜最大の終わりの範囲で 1 回呼ぶ」。crates/server/src/stay_store.rs:654 は候補を `e.event_time >= $3 AND e.event_time < $4` で絞る（終わりを含まない）。滞在との重なりの判定（stay_store.rs:651）は `<=`（終わりを含む）。最大の終わりを `$4` に渡すと、その時刻ちょうどの位置は候補に入らない。D5 は端を含んで除くのでファイルには出ないが、spec.md:117 / spec.md:158-161 の「印を付ける」は成り立たない
- kind: technical
- 提案: D6 に「上端は最大の終わり＋1 マイクロ秒（または翌日の 0 時）で渡す」と書き、Task 4.1 の検証に、終わりちょうどの時刻の位置を 1 件入れる
- 処置: fixed D6 — 上端を最大の終わり＋1 マイクロ秒で渡す。tasks 4.1 の検証に終わりちょうどの位置を足した

## R13. 台帳の始まりの行と遅着の印（D6）の順が決まっていない。`late_mark` で落ちた書き出しが台帳に残らないことがある
- 分類: 観点 2 / 観点 4
- 成果物: design.md（D4 / D6 / D10）、tasks.md（6.1 / 6.2）
- 根拠: spec.md:273-276「失敗した書き出しも台帳に残る」（始まりと失敗の終わりの行）。design.md:54 は失敗の種別に `late_mark` を持つ。design.md:88 の擬似コードは D6 を写しの前に置き、design.md:100 は始まりの行を「この前に別のトランザクションでコミット」とする。「この前」が D6 の前か写しの前か読めない。tasks.md:107 は「本体の前に…`mark_late_for_user`」、tasks.md:112 は「写しの前に `start` の行をコミット」で、2 つの順は書かれていない。D6 が先なら、`late_mark` の失敗は始まりの行を持たない
- kind: technical
- 提案: D4 の擬似コードの先頭を「始まりの行をコミット → D6 → 写し」に直し、Task 6.2 の「失敗した書き出しも台帳に残る」を、`late_mark` の段の失敗と `catalog` の段の失敗の 2 つで確かめる
- 処置: fixed D4 — 順を「台帳の始まりの行 → 遅着の印 → 写し」に決めた。spec の「失敗した書き出しも台帳に残る」を `late_mark` の段と分類の段の 2 つにし、tasks 6.1 / 6.2 に書いた

## R14. Task 5 が Task 6 で作る差し替え口と `GET /export` に依存している（順が逆）
- 分類: 観点 4
- 成果物: tasks.md
- 根拠: tasks.md:88「`書き終えていないフォルダは書き出しの名前を持たない`（D2 の差し替え口で書く途中に止めて…）」、tasks.md:99「5.5 写しの時点。D2 の差し替え口で…」。この差し替え口を足すのは tasks.md:106 の 6.1（「`App` に `exports`（D2）と、テスト用の置き場・段ごとの失敗の差し替え口を足す」）。tasks.md:94 の 5.3「除いた行の件数はどこにも出ない」は「`GET /export` の応答」も見るが、それを作るのは 6.3（tasks.md:114）。Task は 1 本ずつ別の agent が回すので、Task 5 の agent は無い口を前提に書くことになる
- kind: technical
- 提案: 差し替え口（置き場・段ごとの失敗・遅延）を Task 5 の前の独立した項（例: 5.0。`export.rs` の本体が受け取る設定の型）へ移す。5.3 の `GET /export` の側は 6.4 へ移す（6.4 はすでに応答に中身が無いことを見ている）
- 処置: fixed tasks.md — 設定の型・段ごとの差し替え口・遅延を Task 2（2.1）へ移した。応答の側の確かめは 6.4 へ移した

## R15. `CT` の件数の下限が 1 本なので、1 項目が 7〜8 本の Scenario を持っていても 1 本で通る
- 分類: 観点 4
- 成果物: tasks.md
- 根拠: tasks.md:19-21 は `test result: ok\. [1-9][0-9]* passed`（1 本以上）。3.1 は 7 本（tasks.md:61-65）、5.1 は 8 本（tasks.md:82-84）、6.3 は 6 本（tasks.md:115-118）の Scenario を 1 つの `CT` で検証する。`check_scenarios.py` は印があることだけを見て、その印のテストがその絞り込みで走ったかは見ない。Task 9.2〜9.5 も同じ `npm run test:e2e -- settings-export` で、ファイルがあれば通る
- kind: technical
- 提案: `CT` に下限の引数を足す（例: `CT export_excluded 7` は `[7-9]|[1-9][0-9]+ passed`）。e2e は `--grep` を Scenario 名で渡すか、`--list` の件数を下限と比べる
- 処置: fixed tasks.md — `CT <絞り込み> <N>` に本数の下限を持たせ、e2e も `ET <grep> <N>`（`--list` の件数）にした

## R16. 台帳と状態と表の件数は全テストで共有なのに、テストの隔離の規律が利用者の行しか見ていない
- 分類: 観点 4
- 成果物: tasks.md（Global Constraints / 6.2 / 6.3 / 5.1）、design.md（D10 / D11）
- 根拠: tasks.md:23-24 の隔離は「そのテストの利用者の行だけを見る」。しかし `core.export_ledger` は `user_id` を持たず（design.md:218）、`GET /export` の `last` / `last_failure` / `interrupted` は台帳の最後の行から読む（design.md:223-227）。`cargo test` は同じバイナリのテストを並列に走らせ、testdb に直列化の錠は無い（crates/server/src/testdb.rs に `Mutex` / serial の類が無い）。そのため次が他のテストの書き出しで揺れる: 「失敗しても前回の書き出しの値は変わらない」（間に他のテストの `done` が入る）、「書き出した後に記録を消しても状態は変わらない」、「途中で止まった書き出し」（他の `App` が走らせている書き出しは、この `App` から見ると終わりの無い始まりの行＝`interrupted`）、「読むための辞書と稼働の元データが出る」（件数を表の行の数と比べる間に、他のテストが生存信号や登録簿の行を足す）
- kind: technical
- 提案: 書き出しのテストを 1 つの錠（`tokio::sync::Mutex` の static、または `#[serial]`）で直列にすることを Global Constraints に書く。表の件数の比較は、書き出しと同じ写しで数えた値（目録の `rows`）と比べる形にする
- 処置: fixed tasks.md — Global Constraints に書き出しのテストの直列化（static の `tokio::sync::Mutex`）と、表の件数は目録の `rows`（同じ写しで数えた値）と比べる規律を足した

## R17. 分類・列・台帳が空、の Scenario を関数の単体テストで代えているが、Scenario の THEN（書き出しが失敗して終わり、フォルダができない）を測っていない
- 分類: 観点 4
- 成果物: tasks.md（2.1 / 2.2 / 6.3）
- 根拠: spec.md:186-187 / spec.md:250-251 の THEN は「書き出しは失敗として終わり、書き終えた書き出しのフォルダはできず、失敗の理由に…名前が出る」。tasks.md:51 / tasks.md:54 は「突き合わせの関数に表と列の一覧を渡す単体テストで起こす」。これで測れるのは関数が名前を返すことまでで、書き出しが始まらないこと・フォルダが無いこと・台帳の `reason` に名前があることは測らない。6.3 の「1 度も書き出していなければ」（tasks.md:116）も、口ではなく関数を測る
- kind: technical
- 提案: D2 の段ごとの差し替え口で、分類の段に「分類の無い表 `x`」の一覧を注入して本体を走らせ、フォルダが無いことと台帳の `reason` を確かめるテストを 2.1 / 2.2 に足す（共有の DB に表を足さずに済む）
- 処置: fixed tasks.md — 分類の無い表・列と欠けた列の 3 本は、差し替え口で一覧を注入して本体を走らせ、THEN（失敗・フォルダが無い・理由に名前）を全部確かめる形にした（2.2 / 2.3。台帳の `reason` は 6.2）。「1 度も書き出していなければ」だけは rejected の部分あり: 台帳は追記のみで共有のテスト DB では空にできないので、状態を作る純関数に「行が無い」を渡して固定し、口がその関数を通すことを 6.3 の他のテストで確かめる（6.3 に理由を書いた）

## R18. C17「1 回 1 行」を design D10 で 2 行にしたことは C17 の狙いと矛盾しないが、deep.md に記録が無い
- 分類: 観点 1
- 成果物: design.md（D10）、deep.md
- 根拠: deep.md:153 C17「1 回 1 行を追記のみの台帳に残す」。design.md:216-217 は 2 行にし、理由（途中で止まっても写しの時点と置き場が残る。1 行で持つと書き換えになり追記のみと両立しない）を書いている。C17 の中身（追記のみ・中身を残さない・書き出しに入れない・写しの時点と置き場と経路と表ごとの件数）は spec.md:258-281 にすべてある。行の数だけが変わった。ただし deep.md の「当初案を覆したもの」（deep.md:181-187）には無く、tasks.md:3 は deep.md を最優先で読めと書くので、下流がまず「1 回 1 行」を読む
- kind: technical
- 提案: deep.md の C17 の行に「→ design D10 で 2 行（始まり・終わり）に精密化。狙いは同じ」と 1 行足すか、「当初案を覆したもの」に加える
- 処置: fixed deep.md — 「spec で精密化したもの」の節を足し、C17 を 1 回 2 行にしたことと理由を書いた

## R19. e2e は手元の run.sh の stack を使い回すので、遅延と置き場が渡らない
- 分類: 観点 4
- 成果物: tasks.md（Task 9）
- 根拠: web/playwright.config.ts:40 `reuseExistingServer: !process.env.CI`。tasks.md:144-145 は `ASHIATO_EXPORT_DIR` と `ASHIATO_EXPORT_SELFTEST_DELAY_MS=400` を `tools/stack.sh` の e2e の起動だけに渡し、確認バッチの `run.sh` には遅延を渡さない。手元で run.sh の stack が立っていると、それが使い回される。そのとき遅延は 0 なので「押すとその場で進み具合が出る」「画面を離れて戻っても進み具合が出る」は書き出しが先に終わって揺れ、置き場は既定（`$HOME/.local/share/ashiato/export`）になる
- kind: technical
- 提案: 9.2 の e2e の先頭で、遅延が効いていることを確かめて、効いていなければ理由つきで落とす（例: 状態の口の `running` が 1 回は見えること）。または tasks に「e2e は `CI=1` か、run.sh を止めて走らせる」と書く
- 処置: fixed tasks.md — 9.1 の `beforeAll` で遅延が効いていること（`POST` 直後に `running` が見えること）を確かめ、効いていなければ「run.sh を止めるか CI=1 で」を出して落とす

---

観点 5: 該当なし（確かめた範囲: `check_chain.py` が観点 8 を含めて OK。ST33.md の価値と完了の判定は、NFR-16 ★ 2026-09-29 訂正の本文（docs/requirements.md:898-）を逐語で写していて、deep の Q1 / Q5 と矛盾しない。`requires: [ST01, ST22]` は deep.md:175 の記録と一致する。specs が満たすのは NFR-16 だけで、NFR-17〜19 は画面の下限として引いていて別の要件を前倒しで満たしてはいない。capability `data-durability` は proposal.md:73・specs のディレクトリ・INDEX.md:80 の 3 か所で一致し、前倒しは無い）

観点 1 のうち、指摘しなかったもの（確かめた範囲）: Q1（spec.md:9-51、画面の spec.md:344・spec.md:387-390）、Q2（D7（仮）と反転条件）、Q3（spec.md:336-410。24 px・4.5:1・360 px の数値と文言「書き出し中 N / M 行」「まだ書き出していません。」は Scenario にある）、Q4（spec.md:28-31・D16（仮）。覆した後の内容だけが残っている）、Q5（spec.md:108-161。NFR-16 に ★ 2026-09-29 がある）、C1〜C8 / C10 / C11 / C13〜C15 / C18 は Scenario か D 番号にある。C9 / C16 / C17 は R11 / R7 / R18 で挙げた。
