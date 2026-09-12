# ST16 specs / design / tasks の独立レビュー

対象: `openspec/changes/st16-stay-derivation/`（HEAD c2fce56、ブランチ docs/st16-upstream）。
**成果物は触っていない。** 指摘の番号は `review/deep.md`（R1〜R10）の続き（R11〜）。
同じ番号にすると `review_triage.py` の `escalated` 検査（deep.md に R<n> があるか）が、
深掘りレビューの R1〜R10 に当たって素通りするため。

## 機械の検査（最初に写す）

| コマンド | 結果 |
|---|---|
| `openspec validate st16-stay-derivation --strict` | `Change 'st16-stay-derivation' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0）。観点 8（再生成との一致）を含めて通過 |
| `python3 scripts/check_scenarios.py . st16-stay-derivation` | `scenarios: FAIL (担保なし 33 件)`（rc=1）。この change の 37 本 = 担保なし 33 + 人間の確認待ち 4。上流の段階では想定どおり（テストは下流が書く）。既存テストの `[warn] spec に無い Scenario を指す印` 多数は ST02 / ST03 が archive 待ちで正典に無いため |
| `python3 scripts/review_triage.py . st16-stay-derivation` | `triage: OK`（rc=0。この時点では review/deep.md の 10 件のみ） |

---

## R11. D3（仮）の形は、本人が選んだ Q1 の第 1 選択肢ではなく、選ばれなかった第 3 選択肢の説明文そのもの

- 成果物: openspec/changes/st16-stay-derivation/design.md（D3）/ specs/derived-records/spec.md（Requirement「作り直しても同じ滞在は同じ識別子を保つ」）
- 根拠:
  - deep-questions.json Q1 の選択肢（本人に見せた文言）:
    - 第 1（**本人が選んだ**）: 「滞在に安定な鍵を与え、区切りが変わっても「同じ滞在」を指せるようにする」/
      detail「「その日・そのあたりの場所」から鍵を作り、外部識別子の欄に置く。**区切りが動いても鍵が同じなら紐づけはそのまま残る**」/
      irreversible「**鍵の作り方は一度決めると変えられない**」
    - 第 3（選ばれていない）: 「区切りは動かし、紐づけは「**時間の重なりがいちばん大きい新しい滞在**」へ移す」/
      detail「機械的に移す。**1 件が 3 件に割れたら、いちばん長く重なる 1 件に集まる**」
  - design.md:78-100（D3）「(新しい滞在, 既存の滞在) の組を**時間の重なりの大きい順**に…割り当て」
  - specs/derived-records/spec.md:145「時間の重なりがいちばん大きい既存の滞在の識別子を引き継がせる」、
    :160-164 Scenario「割れた滞在は重なりのいちばん大きい 1 件が識別子を継ぐ」＝第 3 選択肢の detail の逐語の言い換え
  - Q1 の why（本人に問うた中身）が「**割れた 3 件のどれに、元の 1 件に付いていた「気分 +1」を移すのか**」。
    第 1 選択肢の説明文の形（割れた 3 件が同じ鍵を持つ）なら答えは「3 件すべてが指す」、D3 なら「重なりの最大の 1 件だけ。残り 2 件は新しい識別子で何も付いていない」。
    **問いの核心への答えが、本人の選択と逆の側に変わっている**
  - deep.md:23-24 / :220-222 は「性質は本人の答えのまま」「紐づけが 0 件のうちは戻せる」として D3 を（仮）にしている。
    しかし (a) 第 1 選択肢の irreversible 欄が「鍵の作り方は変えられない」と明示しており、本人は「作り方」ごと読んで選んでいる。
    (b) 性質「区切りが変わっても同じ滞在を指す」の**「同じ滞在」の定義**（1 対 1 か 1 対多か、場所を見るか時間だけか）が D3 で新たに決まっており、
    これは B（計算し直せば戻る）ではなく、ST17 の最初の紐づけで閉じる A（loss: discarded）の中身そのもの
  - D3 は時間の重なりしか見ない（design.md:89、反転条件 :108 で「時間と距離」を後回し）。
    基準を広げて移動の区間が隣の滞在とつながった場合など、**別の場所の滞在へ識別子が移る**ことを spec も design も禁じていない
  - 実装側の指摘自体（計算した鍵は割れると重複する / 格子の境目で変わる）は正しい。第 1 選択肢の説明文の前提が事実と違っていた型（premise）
- kind: premise
- loss: discarded
- 提案: Q1 を deep.md に第 2 回の問いとして立て直す（A）。「第 1 選択肢の説明文の形は成り立たない（理由 2 つ）」を context に置き、
  選択肢を少なくとも「重なり最大の 1 件だけが継ぐ（D3。割れた残りは何も付かない）」「割れた全件が元の識別子を参照できる（1 対多。例: 継いだ 1 件 + 残りに『分かれ元』を持たせる）」
  「時間に加えて距離でも同じと判定する」に分けて本人に返す。答えが戻るまで D3 の（仮）と spec の該当 Requirement は保留扱い
- 処置: escalated
  - **人間に返した**（深掘り第 2 回 Q10。`deep-questions-r2.json` / `docs/briefs/ST16-deep-r2.html`）。指摘は正しい —— D3 の形は第 3 選択肢の説明文そのもので、割れたときの答えが本人の選択と逆の側に変わっていた。実装者が仮で閉じてよいものではなかった。距離を見るか（後半）は B として Q11 に分けた。答えが戻るまで design D3 と spec の該当 Requirement に「答え待ち」と明記し、tasks 3.2 は答えを確かめてから始めると書いた。

## R12. D10 の台帳 2 表に利用者識別子の列が無い（FR-29「すべてのテーブル」に反する）。追記のみの錠を掛けた後では足し直しが重い

- 成果物: openspec/changes/st16-stay-derivation/design.md（D10）/ tasks.md 1.1
- 根拠:
  - docs/requirements.md:209「**FR-29**: THE SYSTEM SHALL すべてのテーブルに利用者識別子の列を持たせる」
  - design.md:204 `core.stay_criteria(id, radius_m, min_minutes, gap_minutes, sources, created_at)` / :207 `core.stay_absorbed(event_id, into_event_id, criteria_id, at)` —— どちらにも `user_id` が無い
  - 既存の追記のみの表は持っている: migrations/202609120943_version_and_ledger.sql:20（`event_version.user_id NOT NULL`、コメント「FR-18 / FR-29 の適用範囲がここにも及ぶ」）/ :62（`erasure_ledger.user_id NOT NULL`）
  - design.md:208 で両表に UPDATE / DELETE / TRUNCATE を拒むトリガを掛ける。後から列を足すと、既存行を埋める UPDATE がそのトリガに拒まれる（トリガを外す移行が要る）
  - 基準が利用者をまたいで 1 本（「いまの基準は `id` が最大の行」design.md:206）なので、D5「利用者ごとに作り直す」と基準の粒度が合っていない
- kind: technical
- 提案: 両表に `user_id uuid NOT NULL` を持たせ、「いまの基準」を利用者ごとの最大 `id` にする。spec「滞在はどの基準で作られたかを持つ」に利用者ごとであることを 1 文足す
- 処置: fixed D10
  - 両表に `user_id uuid NOT NULL`。基準は利用者ごと、いまの基準はその利用者の最大 `id`、行が無ければコードの既定。移行は行を入れない（利用者の一覧を知らないため）。spec「判定の基準は利用者ごとに版として残る」と Scenario「基準は利用者ごとに分かれる」を足した。

## R13. 基準の台帳は開発 DB で共有・追記のみなので、テストが基準を変えると開発 DB と確認バッチの「いまの基準」が戻せなくなる

- 成果物: openspec/changes/st16-stay-derivation/design.md（D10 / D5）/ tasks.md 3.2・3.4・3.5・4.2・5.1・7.1
- 根拠:
  - crates/server/src/testdb.rs:14-15 / :24 —— テストの接続先の既定は開発 DB そのもの（`postgres://ashiato:ashiato@127.0.0.1:55432/ashiato`）。隔離は「テストごとに固有の論理ソース」（:48-52）で取っている
  - 滞在は `logical_source` が固定（`s01-stay`）で、基準の `sources` も固定（`c01-location`）なので、この隔離の手段が使えない
  - design.md:206「いまの基準は `id` が最大の行」、:208 台帳は UPDATE / DELETE を拒む
  - Scenario「基準を変えても前の基準は残る」「半径を変えて作り直すと…」「基準を戻すと吸収された滞在が同じ識別子で戻る」「基準を変えて割れても…」「基準を変えて作り直すと一覧の基準の表示が変わる」のテストは、どれも基準の台帳に 50 m などの行を足す
  - 帰結: (a) 並列に走る他の滞在テストの「いまの基準」が途中で変わる（不安定）。(b) `cargo test` の後、開発 DB の「いまの基準」が最後に走ったテストの値のまま残り、追記のみなので消せない。
    tools/verify-prep.sh:61 も同じ URL を既定にしているので、tasks 7.1（「既定の基準で 9 件 / 15 件」）と確認バッチの手順書が別の基準で動く
- kind: technical
- 提案: R12 の `user_id` を入れ、テストは `testdb::user()` ごとに基準を持たせて隔離する（「いまの基準」を利用者ごとにすれば開発 DB の既定利用者 `00000000-…` に波及しない）。tasks 0 の規律に「滞在のテストは基準を利用者で隔離する」を書く
- 処置: fixed D10
  - 滞在のテストは `testdb::user()` で利用者を毎回作り、その利用者の基準だけを変える（D10 と tasks 0 の規律）。R12 で基準が利用者ごとになったので、開発 DB の既定利用者にも確認バッチにも波及しない。

## R14. D4 の「本人が消した滞在」の述語は `deleted_by` が NULL の削除を取りこぼす（実測）。区別の約束が design にしか無く、ST22 を縛らない

- 成果物: openspec/changes/st16-stay-derivation/design.md（D4）/ specs/derived-records/spec.md（Requirement「作り直しは本人が消した時間帯に滞在を戻さない」）
- 根拠:
  - migrations/202609081618_envelope.sql:33-34 `deleted_at timestamptz, deleted_by text` —— `deleted_by` は NULL 可・CHECK なし
  - design.md:117「`deleted_at IS NOT NULL` かつ `deleted_by` が `'rebuild:'` で始まらない滞在」
  - 実測（migrations 12 本を当てた使い捨て DB `st16_review_tmp`）: `UPDATE core.event SET deleted_at=now(), deleted_by=NULL` の後、
    `WHERE deleted_at IS NOT NULL AND deleted_by NOT LIKE 'rebuild:%'` は **0 件**（NULL の三値論理）、`coalesce(deleted_by,'') NOT LIKE 'rebuild:%'` なら 1 件。
    取りこぼした行は D3 手順 2 の候補（「本人が消したものを除き」）に入り、割り当てで更新されうる
  - 滞在を消す操作を作るのは ST22（design.md:24、proposal.md:89「着手可」）。ST22 が `deleted_by` に何を書くかを縛るのは design.md:117 と :224-226「ST22 の上流はこの D3 / D4 を読むこと」だけで、
    **design は `openspec archive` で正典に入らない**。spec 側（spec.md:181）は「区別する」としか言わず、区別の手段が観測できる形で無い
  - 同じ実測で、派生の行は履歴を積まない内容の更新も DB が通す（`UPDATE … SET raw=…` が rc=0）。「前の版を履歴に残す」は DB ではなくコードだけが守る（tasks 3.2 のテストが唯一の担保）
- kind: technical
- 提案: 印を自由文字列の接頭辞で分けず、DB が区別を強制する形にする（例: `deleted_by` に CHECK を掛けて作り直しの印を列挙し、それ以外は非 NULL を要求する / 吸収は `core.stay_absorbed` の最新行の有無で判定する）。
  spec に「本人の削除として扱う印の条件」を Scenario で置き（NULL を含む）、ST22 が archive 後の正典から読めるようにする
- 処置: fixed D4
  - 述語を `coalesce(deleted_by, '') NOT LIKE 'rebuild:%'` にし、`NULL` を本人の削除に数えた。区別の約束を design だけに置かず、spec の Scenario「削除した者の欄が空の削除も本人が消したものとして扱う」と `docs/handoff/ST22.md`（ST22 は `rebuild:` で始まる値を書かない）に置いた。DB の CHECK は `core.event` への変更で `record-envelope` に触れるので採らなかった。「前の版を履歴に残す」はコードだけが守る点は、tasks 3.2 のテストが担保。

## R15. D5 の範囲の広げ方では、まとめ送りが 0 時をまたいで分かれると日付をまたぐ滞在が重なって 2 件になる

- 成果物: openspec/changes/st16-stay-derivation/design.md（D5）/ specs/derived-records/spec.md（「日付をまたぐ滞在は 1 件のまま」「位置が届かなかった日の滞在は変わらない」）
- 根拠:
  - design.md:138「範囲は その日の 00:00〜24:00 を、**範囲と重なる**読み出しに出ている滞在の始まりと終わりまで広げたもの」、:139「前後 `gap_minutes` ぶん余分に読む」、:87「範囲と時間が重なる既存の滞在を集める」、:93「割り当てられなかった新しい滞在は、新しい UUID」
  - collector-android LocationFix.kt:16 `SEND_INTERVAL_MS = 300_000` / :30 `MAX_BATCH = 200`、Sender.kt:104 `take(MAX_BATCH)` —— まとめ送りは 5 分ごとに時刻順の塊で、塊の境目は 0 時と無関係
  - 手で追った経過（自宅に 20:00 から翌 08:00 まで居る、半径・最短は既定）:
    1. 23:59 までの塊が届く → 9/12 を作り直し → 滞在 S = 20:00〜23:59
    2. 00:00〜00:04 だけの塊が届く → 9/13 だけを作り直す。S は 23:59 に終わるので範囲 [9/13 00:00, 24:00) と重ならず、範囲は広がらない。
       位置は 23:50 から読む → 23:50〜00:04 の集まり（14 分 ≥ 10 分）ができる。範囲と重なる既存の滞在は無い → **新しい UUID で S′ を足す**
    3. 以後の塊は 9/13 だけ → S′ が伸びる。9/12 は二度と作り直されない → **S（20:00〜23:59）と S′（23:50〜08:00）が重なって並ぶ**
  - 塊の境目が 0 時の前後数分に落ちるのは 5 分おきの送信で毎日起こりうる。圏外の後の 200 件ずつの送り直しでも同じ
  - tasks 2.1 は「日付をまたぐ滞在は 1 件のまま」を DB に触らない純粋な関数（`stay::`）で確かめるので、この経路（日ごとの作り直し）を踏まない
  - spec.md:123「位置の記録が届かなかった日の滞在を、この経路では変えない」は、日をまたぐ滞在を「どの日の滞在」と数えるかを定めておらず、広げ方（D5）と文面が衝突する（9/13 の記録で 9/12 に始まる滞在が伸びるのは「届かなかった日の滞在を変えた」か）
- kind: technical
- 提案: 範囲の端で集まりが閉じていなければ（間隔 `gap_minutes` 未満で端に接していれば）閉じるまで範囲を広げ、候補も「範囲の端から `gap_minutes` 以内の既存の滞在」まで含める形に D5 を直す。
  spec に「0 時の前後で別々に届いても日付をまたぐ滞在は 1 件のまま」を**自動の作り直しの経路で** Scenario として足し、tasks 4.1 に結ぶ。「その日の滞在」を「その日と時間が重なる滞在」と定義する
- 処置: fixed D5
  - 範囲の広げ方を「端から `gap_minutes` 以内に端を持つ滞在」「端をまたいで続くとどまり」の 2 条件を変わらなくなるまで繰り返す形に直した。spec に Scenario「0 時の前後で別々に届いても日付をまたぐ滞在は 1 件のまま」を自動の経路で足し、tasks 4.1 で `/ingest` を 2 回叩く DB のテストに結んだ。「その日の滞在」は「その日と時間が重なる滞在」と spec に書いた。

## R16. 作り直しに排他が無い。同じ日の作り直しが並ぶと滞在が別 UUID で二重に入り、履歴の版番号が衝突する

- 成果物: openspec/changes/st16-stay-derivation/design.md（D3 / D5）
- 根拠:
  - design.md:93 割り当てのない新しい滞在は新しい UUID。`event_dedup_ext`（migrations/202609120942_dedup_indexes.sql:35-36）は `external_id` = 新 UUID なので当たらず、`event_dedup_hash`（:46-47）は `WHERE external_id IS NULL` なので当たらない —— **DB に二重を止めるものが無い**
  - `core.event_version` は `UNIQUE (event_id, version_no)`（202609120943_version_and_ledger.sql:39）。design.md:92「`version_no` は続き番号」を 2 本が同時に読むと同じ番号を積もうとして落ちる
  - 並びうる経路: (a) 手の全期間の作り直し（`POST /stays/rebuild`、design.md:144-146）が走っている間に端末の 5 分おきのまとめ送りが自動の作り直しを起こす。
    (b) collector-android HttpTransport.kt:32 `readTimeout = 15_000` —— design.md:148 が自ら挙げる「応答が遅れる」状態では、端末が読み取り上限で諦めて再送し、同じ日の作り直しがサーバで重なる
  - spec にも design にも、ロック・直列化・「同時でも二重にならない」の記述が無い（`grep -n "ロック\|lock\|排他\|直列" design.md` 0 件）
- kind: technical
- 提案: 作り直しを利用者ごとに直列化する（例: `pg_advisory_xact_lock` を利用者の鍵で取り、1 日ぶんを 1 トランザクションにする。testdb.rs:35 が既に同じ手を使っている）。
  spec に「同じ日の作り直しが同時に 2 回走っても滞在は二重にならない」を Scenario で足し、tasks 3 に結ぶ
- 処置: fixed D5
  - 利用者ごとに `pg_advisory_xact_lock` で直列化し、1 日ぶんを 1 トランザクションにした（`version_no` もその中で読む）。spec に Scenario「同じ日の作り直しが同時に 2 回走っても滞在は二重にならない」を足し、tasks 3.6 に結んだ。

## R17. 取り込みの口で `s01-stay` を断るとき、どの拒否の種類で返すかが決まっていない。Scenario の「400」はまとめ送りでは成り立たない

- 成果物: openspec/changes/st16-stay-derivation/specs/derived-records/spec.md:220-223 / design.md（D2）/ proposal.md:34
- 根拠:
  - crates/server/src/lib.rs:791-792, 822-826 —— 「**400 は 1 件も受け付けなかったこと**」。他の正しい記録と一緒に送れば 200 で、断りは 1 件ごとの `IngestError` に載る
  - Scenario「その記録は 400 で断られ」は、単独で送ったときしか真にならない（入力の条件が書かれていない）
  - 既存の `IngestError` に「このソースは外から書けない」に当たる種類が無い。新設すれば OpenAPI の列挙と `docs/collector-contract.md` §状態符号が変わり、proposal.md:34「取り込みの契約…は変えない」と食い違う。
    既存の `unknown_source` を流用すれば「登録簿にある」事実と矛盾する
  - collector-android Sender.kt:79 `PERMANENT_ERRORS` は拒否の種類ごとに「恒久的 → 未送信から捨てる」を分けている。種類の選び方が端末側の挙動を決める
- kind: technical
- 提案: 断りの種類（新設か流用か）を D2 で決め、spec の Scenario を「滞在用のソースの記録は 1 件ごとの結果で <種類> として断られ、滞在は増えない」＋「同じまとめ送りの位置の記録は受け入れられる」に分ける。新設なら proposal の「契約は変えない」を直す
- 処置: fixed D2
  - R18 の処置で**取り込みの口では断らない**ことにしたので、拒否の種類は要らなくなった（取り込みの契約も変えない）。spec の「滞在はサーバの作り直しだけが書く」を消し、Scenario「取り込みの口から送った滞在は作り直しで置き換わる」にした。

## R18. 登録簿にある `s01-stay` を取り込みの口で断るのは `record-envelope` の正典の変更（MODIFIED）だが、proposal は「変えない」と書き、盤面の衝突の判定からも漏れている

- 成果物: openspec/changes/st16-stay-derivation/proposal.md（Capabilities / Modified: なし）/ design.md（D2）/ specs/derived-records/spec.md:213-223
- 根拠:
  - openspec/specs/record-envelope/spec.md:127-143（正典）「THE SYSTEM SHALL 登録簿に登録された論理ソースからの記録だけを受け付ける」
    「THE SYSTEM SHALL 新しいソースの追加を、登録簿への 1 行の追加だけで完了させ、API の変更を要求しない」/
    Scenario「登録するだけで受け付けられる」—— THEN「API のコードを変更せずに、そのソースからの記録が受け付けられる」
  - D2 は `s01-stay` を登録簿に 1 行足し（design.md:67）、**コードで**取り込みの口から断る（:72）。登録されているのに受け付けない論理ソースが正典の文面の例外として初めて立つ
  - proposal.md:64-66「Modified Capabilities: なし。`record-envelope` … の要件は変えない」、:34「取り込みの契約…登録簿の形は変えない」
  - `record-envelope` は ST03 が archive 待ちで握っている（`python3 scripts/board.py` の出力: ST05 が「`record-envelope` を ST03 が触っている」で衝突待ち）。
    ST16 は proposal に書かないことで、同じ capability を触るのに衝突待ちの判定を受けていない
  - design.md:73-74 は「`origin='derived'` 全体は断らない —— 断ると `record-envelope` の振る舞いを変える」と自ら理由にしているが、`s01-stay` だけを断るのも同じ capability の振る舞いの変更
- kind: conflict
- 提案: どちらかに決める。(a) `record-envelope` への MODIFIED として書き（「サーバだけが書くと宣言したソースは取り込みの口で断る」、登録簿に宣言の列を足す形なら API コードの例外にならない）、ST03 の archive を待つ。
  (b) 断らず、偽の滞在が入る代償を D2 に書く（deep.md C12 の「既定は厳しい側」を覆すので理由が要る）
- 処置: fixed D2 仮
  - (b) を採った。`s01-stay` を断ると `record-envelope`（ST03 が archive 待ち）への MODIFIED になり ST16 自身が衝突待ちで止まるため。作り直しの候補と読み出しを `origin='derived'` だけにし、取り込みの口から入った派生の滞在は作り直しで置き換える。代償（`origin='collected'` を名乗った行が `core.event_live` に残る）は Risks に、反転条件（ST03 の archive 後に宣言の列を MODIFIED で足す）は D2 に書いた。deep.md C12 を覆したことを「当初案を覆したもの」に書いた。

## R19. D4 の「時間帯に重なれば削除済みとして作る」は、基準を広げたとき本人が消していない時間まで隠し、印が外れる経路も定まっていない。FR-50 の ★ は仮決めを要件の文面に焼いている

- 成果物: openspec/changes/st16-stay-derivation/design.md（D4）/ specs/derived-records/spec.md:177-199 / docs/requirements.md FR-50
- 根拠:
  - design.md:120-121「新しく計算した滞在のうち、本人が消した滞在の `[start, end]` と時間が**重なるもの**は…削除済みとして入れる」—— 重なりの長さを問わない
  - 経過: 本人が 10:00〜11:00 を消す → 半径を広げて作り直すと 08:00〜12:00 が 1 件に統合される → 1 時間の重なりで **4 時間の滞在が丸ごと削除済み**になり、一覧（spec browsing-views:13）から 08:00〜10:00 と 11:00〜12:00 が消える。
    本人は 3 時間ぶんを消していない。spec にこの形（部分的な重なり）の Scenario が無い（3 本とも、消した範囲に内側から収まる滞在か、吸収だけ）
  - design.md:122-123「`'rebuild:erased-range'` の滞在は D3 の割り当ての対象にする」「印は外れない」。しかし基準を戻すと、その行を継いだ新しい滞在（08:00〜09:59）は消した範囲と重ならない。
    D3 手順 3（design.md:91-92）が外すのは「吸収の印」だけで、`erased-range` の印を外すかが書かれていない —— 外さなければ、重ならない滞在が作り直しのたびに削除済みのまま残る
  - deep-questions.json Q2 第 1 選択肢の irreversible 欄「**印の掛け方（何に対して消したのか）は後から変えられない** —— 時刻の範囲に掛けるか、場所に掛けるか、Q1 の鍵に掛けるかで、消える範囲が変わる」。本人の答えはここを開いたまま（deep.md:46）
  - docs/requirements.md:369-370（★ 2026-09-13）「本人が消した派生の**時間帯**に、削除されていない派生を作らない」—— D4（仮）の選択が要件の本文に入っている。D4 の反転条件（design.md:129-130「ST22 の深掘りで本人が…望めば変える」）を使うと要件の改訂が要るが、そのことが ST22 側のどこにも書かれていない（docs/handoff/ は README.md と ST02.md だけ）
- kind: daily
- 提案: 部分的な重なりの扱い（丸ごと隠す / 重なった部分だけ隠す）と、重ならなくなった `erased-range` の印を外すかを D4 に書き、spec に Scenario を 2 本足す（「消した範囲より長い滞在に統合されたとき」「基準を戻して重ならなくなったとき」）。
  FR-50 の ★ を「時間帯」と書くなら D4 の（仮）を外すか、ST22 への申し送り（docs/handoff/ST22.md）に反転条件と要件の改訂を書く
- 処置: fixed D4 仮
  - 部分的な重なりは**丸ごと隠す**を仮に置き（既定は厳しい側）、**深掘り第 2 回 Q12（B。推奨は丸ごと隠す）**として本人にも見せた。重ならなくなった `rebuild:erased-range` の印は外すと D4 に書き、spec に Scenario「消した範囲より長い滞在に統合されると丸ごと隠れる」「基準を戻して重ならなくなった断片は戻る」を足した。FR-50 の ★ の文面と反転条件は `docs/handoff/ST22.md` に書いた。

## R20. design Risks の反転案「終わりだけが伸びた更新は版を積まない」は、spec の Scenario「区切りが伸びても識別子は変わらない」の AND と正面から逆

- 成果物: openspec/changes/st16-stay-derivation/design.md:221-223 / specs/derived-records/spec.md:154-158
- 根拠:
  - spec.md:156-158 WHEN「09:00〜09:20 の滞在がある状態で、同じ場所の 09:20〜10:00 の位置の記録が届く」THEN「識別子は同じ」**AND「09:00〜09:20 の内容が前の版として残っている」**
  - design.md:222「多すぎると判ったら、**終わりだけが伸びた更新は版を積まない**に変える」—— まさにこの Scenario の入力（終わりだけが伸びる）で AND が偽になる
  - 1 Scenario に主張が 2 つ（識別子の保持 / 版の保持）束ねてあるので、反転を採ったとき「識別子は保つ」側まで一緒に赤になるか、テストの AND 側だけが黙って消される
- kind: technical
- 提案: Scenario を 2 本に分け（識別子 / 版）、版の側は Risks の反転を採るなら書き換える対象だと design に明記する。どちらを正とするかは年 10 万行（design.md:221）の見積りで今決めてもよい
- 処置: fixed D3
  - Scenario を「区切りが伸びても識別子は変わらない」と「区切りが伸びると前の版が残る」に分けた。版は終わりだけ伸びても積むことに**今決めた**（年 10 万行・50 MB 未満）。Risks の反転案は消した。

## R21. Scenario の検証性 —— 観測の手段が無いもの・入力が足りないもの・主張を束ねたもの

- 成果物: openspec/changes/st16-stay-derivation/specs/derived-records/spec.md / specs/browsing-views/spec.md
- 根拠:
  - **観測の手段が無い** —— derived-records:90-93「基準を変えても前の基準は残る」THEN「半径 100 m の基準も、変えた時刻とともに**引ける**」。spec にも design にも基準の一覧を読む API が無い（design.md:144-146 は `POST /stays/rebuild`、:183 の `GET /stays` の `criteria` は時刻を持たず、その日の滞在の基準しか返さない）。DB を直に引く以外に真偽が決まらない
  - **入力が足りない** —— derived-records:103-107「半径を変えて作り直すと区切りが変わり…」WHEN「1 日ぶんの位置の記録」THEN「件数**または**始まりと終わりが変わる」。入力の形が無いので、互いに 30 m 以内の点だけの 1 日なら 50 m にしても変わらず偽になる。人間の確認待ち（tasks.md:152-155）もデータを指定していない
  - **「同じ基準」が 2 通りに読める** —— derived-records:109-113「基準を変えずに作り直しても…前の版は増えない」。design.md:56 は `raw` に `criteria.id` を入れ、:144-145 は本文に基準を渡すと台帳に**新しい版**を足す。同じ値の基準を本文付きで 2 回叩くと `criteria.id` が変わり、全滞在の `raw` が変わって版が積まれる。「本文を省く」のか「同じ値を渡す」のかを WHEN が決めていない
  - **失敗の起こし方が無い** —— derived-records:138-141「作り直しが失敗する状態で」。tasks.md:91「吸収の台帳を読めなくして作る」は、テストが開発 DB を共有する（testdb.rs:14-15）ので他のテストの作り直しも一緒に落とす。権限を剥がすなら接続ロールが表の所有者だと効かない
  - **移動の行の前提が抜けている** —— browsing-views:41-44「08:40 に滞在が終わり、09:22 に次の滞在が始まる」THEN「移動 42 分」。間に 10 分以上の欠けがあれば「記録なし」になる（同 spec:46-50）ので、WHEN に「その間の位置の記録が 10 分以上途切れない」が要る
  - **束ねた主張** —— derived-records:65-69（割れない AND 記録が残る）、:103-107（区切りが変わる AND 位置の記録が変わらない）、:185-189、browsing-views:18-22。片方だけ通っても緑になる
  - **件数が無い** —— browsing-views:18-22「自宅・職場・昼の店・職場・自宅」THEN に滞在の件数（5 件）と移動の行が無い。tasks 6.1（tasks.md:113-119）は応答を固定して描画だけを見るので、サーバが何件返すべきかをどこも確かめない
- kind: technical
- 提案: 基準の一覧を読む口（または `POST /stays/rebuild` の応答に前の基準を含める）を spec に置く / 入力の座標と件数を WHEN に書く / 「同じ基準」を「本文を省いた作り直し」と定める / 失敗は注入点（作り直しの関数を差し替える）で起こすと tasks に書く / AND を別 Scenario に分ける
- 処置: fixed specs/derived-records/spec.md
  - 基準の版の一覧を読む口（`GET /stays/criteria`）を spec に置いた / 区切りが変わる Scenario に入力（地点 A と 70 m 離れた地点 B）を書いた / 「同じ基準」を「基準を添えない、またはいまと同じ値を添える（版を足さない）」と定めた / 失敗は作り直しの関数を差し替えて起こすと design D5 と tasks 0 に書いた / 移動の行の前提（10 分以上途切れない）を足した / 束ねた主張を別 Scenario に分けた / 1 日歩き回った日に件数（滞在 5・移動 4）を書き、サーバの件数を tasks 5.2 で確かめる。

## R22. 観測可能な振る舞いが design にだけあり、archive で正典から落ちる

- 成果物: openspec/changes/st16-stay-derivation/design.md（D3 / D5 / D7 / D8 / D10）
- 根拠: 以下はどれも spec の Requirement / Scenario に無い
  - D8（design.md:178-191）: 一覧の置き場 `#/day/YYYY-MM-DD` と S-1 がルート `/` のまま、互いへの行き先 / `GET /stays?date=` の応答の形（`entries[].kind` が `stay` / `move` / `no-record`）/ `POST /stays/rebuild` の本文と応答（作り直した日数・前後の件数・時間。:144-146）。
    ST25 / ST26 が同じ `browsing-views` に足すとき、正典からは API の形が読めない。
    なお docs/ui-direction.md:34 は「S2[S-2 主表現 ★入口]」で、D8 の「S-1 がルートのまま」と食い違う（反転条件 :192 で ST25 に送っているが、spec に入口の定めが無い）
  - D10（design.md:205）: 基準の範囲 `radius_m` 1〜10,000 / `min_minutes`・`gap_minutes` 1〜1,440。範囲外を本文で渡したときに何が返るか（400 か 500 か）が spec に無い。CHECK 違反をそのまま返すと 500
  - D7（design.md:166, 168）: 消去済みの位置（`payload = '{}'`）を読み飛ばす / `acc_m` の無い位置を精度が良いものとして扱う。FR-51 と FR-76 ★ の境目の振る舞い
  - D3 / Risks（design.md:90, 219-220）: 重なりが同じときの割り当ての順（読み出しに出ている方 → 始まりの早い方）。tasks 3.2 がテストで固定すると言うが、Scenario が無いので check_scenarios.py の突合に乗らない
  - D5（design.md:141）: 作り直しの失敗を `kind = "stay.rebuild"` で残し、座標と時刻をログに出さない（製造準備 A-2）
- kind: technical
- 提案: 少なくとも API の形（`GET /stays` / `POST /stays/rebuild` の本文・応答・範囲外の断り方）と、消去済み・精度欠落の位置の扱い、同点の割り当て順を spec の Requirement / Scenario に移す
- 処置: fixed specs/browsing-views/spec.md
  - 1 日の並びの読み出しの形（種類・時刻・識別子・基準、400 / 401）、日付を含むアドレスと S-1 の入口を変えないことを browsing-views に、基準の範囲外を 400 で断ること・消去済みと精度欠落の位置の扱い・同点の割り当て順・作り直しの失敗を位置の値を含めずに残すことを derived-records に、それぞれ Requirement と Scenario として移した。

## R23. 今日の一覧で、まだ来ていない時間が「記録なし」になる。日の頭と尻の欠けの扱いも spec に無い

- 成果物: openspec/changes/st16-stay-derivation/specs/browsing-views/spec.md:46-62, 88-91 / design.md（D8）
- 根拠:
  - browsing-views:48「隣り合う位置の記録の間隔が…以上ある区間」を記録なしとし、:50「その日に 1 件も無いとき、その日全体を記録なし」、:62 THEN「00:00 – 24:00 の「記録なし」の行」
  - :91「開いた直後は今日（Asia/Tokyo）の一覧を出す」—— 既定で開くのは**今日**。今日の 09:00 に開くと、最後の位置から 24:00 までの未来が「記録なし」に数えられうる（「隣り合う記録の間」に当たらない尻の区間を記録なしにするか、spec が決めていない）。
    朝いちばん（その日の位置がまだ 0 件）なら :50 のとおり「記録なし 00:00 – 24:00」になり、**取れていないのではなく、まだ起きていない時間**を扱う（扉 #14 が分けた「データが無い」の意味と違う）
  - 日の頭（00:00 から最初の位置まで）と尻（最後の位置から 24:00 まで）が記録なしになるのか、前日から続く滞在がある場合はどうなるのかも spec に無い（design.md:183 の応答の形にも無い）
- kind: daily
- 提案: 「今日の一覧では現在時刻より後を行にしない」と「日の頭と尻の欠けは、隣の日の記録を含めて間隔で判定する」を Requirement に足し、Scenario を 2 本置く
- 処置: fixed D8 仮
  - 今日はいまより後を行にしない / 日の頭と尻は前後の日の位置を含めて間隔で測る、を spec の Requirement に足し、Scenario を 2 本置いた。「位置の記録が無い日は丸ごと記録なし」は今日より前の日に限った。

## R24. Q4 の「作り直したことの見え方」は、本人が選んだラベル（行に出す）と proto の描画（一覧の上に 1 つ）が違い、spec は描画の側を採っている

- 成果物: openspec/changes/st16-stay-derivation/specs/browsing-views/spec.md:64-74 / design.md:187
- 根拠:
  - deep.md:80 本人の貼り戻し「作り直したことの見え方 - **行に**「作り直した基準」を出す」
  - proto.html:424 選択肢 `{v:"badge", t:"行に「作り直した基準」を出す", d:"150 m / 10 分。どの基準で出来た区切りか常に読める"}`
  - proto.html:545-548 —— `badge` を選んだときの描画は、見出しの下に 1 つだけ「この一覧は 半径 … m / … 分 で作った」。行ごとには出ない
  - spec browsing-views:66「一覧に…文字で出す」、design.md:187「一覧の上に」は描画の側。deep.md:92-96 の「読み方」はこの食い違いに触れていない
  - 影響は小さい（基準が混ざるのは作り直しの途中など限られる）が、spec:67「違う基準で作られた滞在が混ざるとき、それぞれの基準を出す」の置き場（行か上か）が決まらない
- kind: daily
- 提案: 描画の側を採るなら deep.md Q4 の「読み方」に 3 点目として書き、混ざったときの出し方（上に並べる / 該当行に出す）を spec に書く
- 処置: fixed D8 仮
  - 描画の側（一覧の上に 1 行）を採り、deep.md Q4 の読み方 3 に食い違いと採った理由を書いた。基準が混ざる日は、上にすべての基準を並べ、最初の基準と違う滞在の行にその基準を添える（spec と Scenario を足した）。

## R25. FR-50 と PERM-3 の ★ 補足を ST16 が実装するが、ST16 の `satisfies` と INDEX に理由が無い。doors も #7 のまま

- 成果物: docs/stories/ST16.md / docs/stories/INDEX.md / openspec/changes/st16-stay-derivation/deep.md（要件へ戻すもの）
- 根拠:
  - docs/stories/ST16.md:4 `satisfies: [FR-31, FR-76]`、:6 `doors: [7]`
  - specs/derived-records/spec.md:183「導出元: FR-50」（作り直しは本人が消した時間帯に滞在を戻さない）/ :205「導出元: PERM-3, PERM-2」
  - docs/requirements.md:369-370（FR-50 ★ 2026-09-13）/ :501（PERM-3 ★）/ :868-870（扉 #15 ★）はどれも「ST16 の深掘りの決定」
  - `python3 scripts/check_chain.py .` は FR-50 を ST22、PERM-3 を ST24 が拾っているので OK を返す。**鎖の上では FR-50 の派生の条項は ST22 の完了の判定になっている**が、ST22 はそれを知る手段が無い（docs/handoff/ に ST22.md が無い）
  - deep.md:174-175「stories.json の ST16 は判断が変わらないので直していない」—— capability の前倒し（INDEX.md:114-125）には理由があるが、要件の前倒しには無い
- kind: technical
- 提案: INDEX の訂正の節に「FR-50（派生の条項）と PERM-3（派生の条項）は ST16 が満たす」を足すか、stories.json の ST16 の `satisfies` / `doors`（#15）に加えて再生成する。ST22 への申し送りを docs/handoff/ST22.md に置く（R14 / R19 と同じ宛先）
- 処置: fixed proposal.md
  - proposal の Impact と `docs/stories/INDEX.md` の訂正の節に「FR-50 と PERM-3 の本体は ST22 / ST24、派生の条項だけを ST16 が満たす」を表で書いた。`satisfies` は変えていない（本体の Story を動かすと鎖と完了の判定の持ち主が変わる）。ST22 への申し送りを `docs/handoff/ST22.md` に置いた。doors の #15 は、`satisfies` に PERM-3 を足さない以上 `make_story.py` が引かないので据え置き。

## R26. tasks の検証コマンドのうち、1 本は必ず落ち、多くはテストが 0 本でも rc=0 になる

- 成果物: openspec/changes/st16-stay-derivation/tasks.md
- 根拠:
  - **必ず落ちる** —— tasks.md:99 `cargo test ingest_rejects_stay_source derived_rebuild_is_not_folded`。実測: `cargo test -p ashiato-server a_filter_x b_filter_y --no-run` → `error: unexpected argument 'b_filter_y' found`（`cargo test` の TESTNAME は 1 つだけ）
  - **0 本でも緑** —— `cargo test <名前の部分文字列>` は一致するテストが無くても `running 0 tests` で rc=0。
    Scenario に結ばれていない検証（tasks.md:32 `migrations_apply_twice`（既存に無い。`grep -rn migrations_apply_twice crates/` 0 件）/ :33 `stay_ledgers_are_append_only` / :35 `stay_source_is_registered` / :56 `stay_raw_is_pinned` / 3.2 の「重なりが同じ組の順を固定するテスト」）は、check_scenarios.py（8.2）の突合にも乗らないので、書かれなくても全タスクがチェックできる
  - **終了条件が無い** —— tasks.md:38 1.4「`EXPLAIN` の出力を PR 本文に貼る」（索引を使っているかの判定が人の目に委ねられ、rc が無い）/ :142-143 8.4（コマンドなし）/
    :132-133 7.1 は `<その日>` が未定（seed.sh の既存データは 2026-09-07 固定、tools/seed.sh:36）で、jq の式も rc も書かれていない。30 分の欠けを 1 つ入れたうえで「滞在 9 件」を保つ配置も指定が無い
  - **依存順** —— 1.4（`c01-location` の 1 日ぶんを引く文の EXPLAIN）は、その文を書く 3 / 4.1 より前に置かれている
- kind: technical
- 提案: 4.3 を 2 コマンドに分ける。Scenario に結ばれない検証は `cargo test -p ashiato-server <名前> -- --exact` にし、出力の `test result: ok. 1 passed` を終了条件に書く（または `--list` で 1 本以上あることを確かめる）。
  1.4 は「出力に `event_by_source_time` を含み `Seq Scan on event` を含まない」を条件にし、3 の後へ動かす。7.1 は日付と jq の式と rc を書く
- 処置: fixed tasks.md
  - 4.3 は 1 コマンドに分け直した。Scenario に結ばれない検証も含め、すべて「件数つき検証 `CT <絞り込み>`」（`test result: ok. N passed` の N≥1 を grep）にした（tasks 0）。1.4 の EXPLAIN は 3.7 に移し、`event_by_source_time` を含み `Seq Scan on event` を含まないことをテストの条件にした。7.1 は日付 2026-09-07・`jq -e` の式・rc を書いた。8.4 / 8.5 にコマンドを付けた。

---

## 観点ごとの確かめた範囲

- **観点 1（deep の決定が正典に写っているか）**: Q1 → R11（形が第 3 選択肢）。Q2 → Requirement と Scenario 3 本はあるが R14 / R19。
  Q3 → spec「滞在の既定の感度は収集した記録と同じ」と Scenario 1 本、PERM-3 ★ / 扉 #15 ★ あり（問題なし）。
  Q4 → 見出し・移動の行・記録なしの行・添える値は browsing-views にあり、R24 のみ。本文の置き場は docs/ui-direction.md ★ 2026-09-13 にあり。
  Q5 → proposal の Capabilities と INDEX.md の表・訂正の節が一致（R18 は別の capability の件）。
  Q6 → Requirement「新しい位置が届いた日の滞在を自動で作り直す」と「全期間」、R15 / R16。
  Q7 / Q8 / Q9 → FR-76 ★ と Scenario（「記録が欠けた区間の前後は別々の滞在になる」「精度の悪い 1 点…」「精度の悪い点しか無い区間…」「長い滞在は区切られない」）あり。数値（10 分・100 m・15 時間・150 m）は Scenario に入っている。
  「要件へ戻すもの」4 件はすべて ★ 2026-09-13 の印つきで本文に入っている（review_triage.py の検査 4 も通過）
- **観点 2**: R20 / R21 / R23
- **観点 3**: R22。specs に関数名・crate 名・列名は無い。例外は browsing-views:81 の導出元に `web/src/App.tsx` のファイル名が 1 か所（説明の出所で、振る舞いの主張ではない）。
  proposal の Capabilities（derived-records / browsing-views）と specs/ のディレクトリは一致
- **観点 4**: R26。Scenario 37 本（derived-records 25 / browsing-views 12）は全部、tasks の本文の Scenario 名か「人間の確認待ち」（4 本）に結ばれている（漏れ 0。`check_scenarios.py` の担保なし 33 + 確認待ち 4 = 37 と一致）。
  人間の確認待ちの書式（`- Scenario: <名前>` の裸の形）は check_scenarios.py が `[wait]` として読めている
- **観点 5**: `check_chain.py` 観点 8（再生成との一致）は OK。ST16.md の「価値」「完了の判定」2 件は deep の決定と矛盾しない（完了の判定 2 は spec の Scenario と人間の確認待ちに写っている）。R25
- **観点 6（既存コードとの食い違い。実測）**: 使い捨て DB `st16_review_tmp`（MIGRATIONS 12 本を当てた）で確かめた —
  派生の行は履歴を積んだ内容の更新・履歴なしの更新・`deleted_at` の付け外しがすべて rc=0 / 吸収の印で `core.event_live` から消え、外すと戻る /
  履歴のある派生の行は FK（`event_version_event_id_fkey`）で物理削除できない（D3「採らなかった案」の前提どおり）/ `deleted_by` NULL の述語の穴（R14）。
  コードで確かめたもの: `/ingest` は 1 件 1 トランザクション（lib.rs:296-298, 363-367。D5 の前提どおり）/ 400 の意味（R17）/ `coverage_get` と `achievement_get` は `must_sources()` だけを引く（lib.rs:1043-1047, 1072。C11 どおり）/
  `event_by_source_time (logical_source, event_time)` は 202609112113_source_lifecycle.sql:71-72 に実在 / collector-android の送信間隔・件数・読み取り上限は design の引用どおり（LocationFix.kt:16, 30、HttpTransport.kt:32）/ R12 / R13 / R15 / R16 / R26
- **観点 7（capability の割当と他 Story）**: R18（record-envelope）/ R14・R19・R25（ST22 への申し送りが無い）。
  ST25 は盤面で `衝突待ち`（`browsing-views` を ST16 が触っている）と出ており、INDEX の訂正と一致。ST17 / ST20 は requires: ST16 で待ち、紐づけの形は R11 の答えに依存。
  ST02（collection-coverage）とは、稼働状況が `must_sources()` だけを出すので重ならない。ST07 は移行を足さないので重ならない

---

## 処置のまとめ

16 件すべてに処置を付けた —— **escalated 1 件 / fixed 15 件（うち仮 4 件: R18 / R19 / R23 / R24）**。`rejected` / `deferred` / `followup` は 0 件。

- **R11 は人間に返した**（深掘り第 2 回 Q10。A）。第 1 回 Q1 の答えを実装に落とすところで、選ばれなかった選択肢の形を置いていた
- **R18 で deep.md C12（取り込みの口で断る）を覆した** —— 断ると `record-envelope` に触れ、ST16 が衝突待ちで止まる
- R19 は仮で閉じたうえで、第 2 回 Q12（B）として本人にも見せた。R11 の後半（距離）は Q11（B）
- Scenario は 37 本 → **64 本**（R15 / R16 / R19 / R20 / R21 / R22 / R23 / R24 で分割・追加）

