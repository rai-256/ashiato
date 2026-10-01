# ST21 実装タスク — 場所を登録し、識別子を変えない

読む順: `deep.md`（**最優先。本人が決めた 4 件と、聞かずに決めた C1〜C13**）→ このファイル →
`specs/personal-entities/spec.md` → `design.md` → `review/spec.md`（処置の理由）→ `docs/stories/ST21.md` →
`docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D16）。**名前は作成時刻 `YYYYMMDDHHMM_places.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。
**`record-envelope` / `derived-records` / `browsing-views` の要件、取り込みの口の応答の形、滞在の作り方（`stay.rs` / `stay_store.rs`）、
ST03 / ST19 の錠と門の関数には触らない**（design D17）。並走中の st05 / st06 / st08 / st12 / st22 / st28 の change のファイルも触らない。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d db` が前提。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript はコメント（`// Scenario: 渡した識別子で場所の器ができる`）、bash は `echo`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**（空白は無視される）
- **この change の Scenario は 156 本**（`personal-entities` の ADDED のみ）。そのうち **10 本は ST19 の印がそのまま当たる**
  （「マスタ管理の画面は個人属性と場所のタブを持ち、個人属性は積んだ主張を全部見せる」のうち、名前も本文も変えていない 10 本。`web/src/__tests__/master-view.test.tsx` ほか。design D20（仮））。
  **この change の Scenario の名前は、正典の他の Requirement の Scenario と重ならない**（重なると別の画面の印が偽の担保になる。spec-review R1）。Task 11.2 で確かめる。
  **ST19 の印「人物と場所のタブは無い」は spec から消えた**ので、そのテストを「タブは個人属性と場所の 2 つで人物のタブは無い」に**直す**（印を移すだけにしない。Task 8）
- **この change が新しく足す画面の Scenario の印は `web/e2e`（本物のブラウザ）に置く**（`AGENTS.md`）。jsdom（`web/src/__tests__`）は「指定と勘定」を固定する補助で、新しい印は置かない。
  例外は 2 本だけ —— `場所の画面は確定した色だけを使う`（コードの色を集める静的な検査）/ `同じ内容の 2 つの場所の記録は別々の乱数を持つ`（原文の組み立ての単体）。
  **振る舞いを変えていない ST19 の画面の 10 本は ST19 の jsdom の印のまま**（design D20（仮）。OpenSpec の都合で ADDED に入っただけ）
- **「人間の確認待ち」に逃がせる Scenario は 1 本も無い**（このファイルの末尾）
- **件数つき検証**: `cargo test <絞り込み>` は一致するテストが 0 本でも rc=0 になる。このファイルで **`CT <絞り込み>`** と書いたものは、次のコマンドが rc=0 になることを指す:
  `bash -o pipefail -c 'cargo test -p ashiato-server <絞り込み> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log`。
  **`VT <絞り込み>`** は `bash -o pipefail -c 'cd web && npx vitest run <絞り込み> 2>&1 | tee /tmp/vt.log' && grep -Eq 'Tests +[1-9][0-9]* passed' /tmp/vt.log`、
  **`ET <絞り込み>`** は `bash -o pipefail -c 'cd web && npx playwright test <絞り込み> 2>&1 | tee /tmp/et.log' && grep -Eq '[1-9][0-9]* passed' /tmp/et.log` が rc=0
  （`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く）
- **試験の名前の接頭辞はこの change だけのもの**（既存の試験に部分一致させない）: Rust は `place_`（`place_container_` / `place_ingest_` / `place_lock_` / `place_view_` / `place_window_` / `place_match_` / `place_candidates_` / `place_sensitivity_`）、
  web は `places-`（`web/src/__tests__/places-*.test.ts(x)`）と `web/e2e/places*.spec.ts`。作ったら `cargo test -p ashiato-server place_ -- --list` にこの change の試験しか出ないことを確かめる
- **テストは利用者で隔離する**（`testdb::user()` で毎回新しい利用者を作る）。器の表は追記のみで消せない
- **本人の決定（下流は変えない）**: 画面の構造（Q1: 登録した場所のカードだけ / 帯と合計 / 上位 10 件 ＋ 残り / 最近居た順 / 居た所から選ぶだけ / 前の値は押したときだけ / 広さはカードに出して変えられる）/
  座標を変えるたびに「直す」か「移った（いつから）」かを聞く（Q2）/ 場所の記録の既定の感度は**外部 AI に出してよい**（Q3。推奨と違う側）/
  照合は代表点が広さの中・重なれば近いほう、S-2 は触らない（Q4）
- **D4 / D6 / D7 / D8 / D9 / D10 / D11 / D14 は（仮）決め。** 反転条件は `design.md` にある。変えたらその D 番号を書き直す
- **値は定数にし、テストは名指しで固定する**（`docs/testing.md` §3）: `places::PLACE_DEFAULT_RADIUS_M`（100）/ `places::CANDIDATE_RADIUS_M`（100）/ `places::PLACE_RADIUS_MIN_M`（10）/
  `places::PLACE_RADIUS_MAX_M`（5,000）/ `places::DEFAULT_SENSITIVITY`（1）/ 画面の上位件数（10）と広さの選択肢（50 / 100 / 200 / 300）
- 記録の値（名前・座標・補足）を**ログに出さない**（製造準備 A-2）。出すのは件数・種別・利用者だけ
- **並走中の ST28 が `testdb.rs`（DB の役割）と画面のログインを変えている。** 先に merge されていたら、DB の試験はその形（所有者の接続・`.env`）で書き、e2e は既定の `storageState` で通す。既存の試験の本体は書き換えない

## Task 1: 移行 —— 器の表・場所の記録の錠・登録簿（design D1 / D5 / D16）

- [x] 1.1 移行 `migrations/YYYYMMDDHHMM_places.sql` と `.down.sql` を足す —— `core.place`（D1 の列・`UNIQUE (id, user_id)`・`(user_id, seq)` の索引）と、
  UPDATE / DELETE を拒む行トリガと TRUNCATE を拒む文トリガ（**この移行専用の関数** `core.reject_place_change()`）、
  D5 の 3 関数（`core.reject_place_record_rewrite()` / `core.require_place_erasure_ledger()` / `core.reject_place_record_delete()`）とトリガ、
  登録簿の 1 行 `('s01-place', '場所', 86400, 'none')`。当て直せる形。`MIGRATIONS` 配列の末尾に足す。
  `.down.sql` は場所の記録か器の行が残れば登録簿の行と器の表を残す（D16）。
  検証: `tools/check-migrations.sh` rc=0、`CT place_lock_migration_applies_twice`（全版を 2 回当てて落ちない）、
  `CT place_lock_down_keeps_rows`（器の行を 1 つ入れて `.down.sql` を当て、器の表と登録簿の行が残る）
- [x] 1.2 器の表が追記のみであること。
  Scenario: `器の表は書き換えも削除も切り詰めもできない`。検証: `CT place_container_append_only`
- [x] 1.3 場所の記録の錠（`s01-place` の行を SQL で直接入れて撃つ）。
  Scenario: `場所の記録の座標を書き換える文は拒まれる` / `場所の記録の書いた日時は書き換えられない` / `場所の記録の利用者は書き換えられない` /
  `場所の記録の行は削除できない` / `他の記録を場所の記録へ付け替えられない` / `場所の記録に削除の印を付けられる` / `場所の記録の感度を変えられる` /
  `台帳のある場所の記録の消去は通る` / `台帳の無い場所の記録の消去は拒まれる` / `別の記録の台帳の行では場所の記録の消去は通らない` /
  `台帳の行があっても消去の形でない場所の記録の書き換えは拒まれる` / `場所の錠を足しても主張の錠は変わらない`。
  検証: `CT place_lock_`
- [x] 1.4 `tools/check-immutable.sh` に足す —— (a) `core.place` への UPDATE / DELETE / TRUNCATE が psql から拒まれる、
  (b) `s01-place` の行の書き換え・行の削除・台帳の無い消去が拒まれ、削除の印と台帳つきの消去は通る、(c) `…_places.down.sql` を戻しの逆順の先頭で当てて当て直せる。
  ST19 の「本人が書いた記録は書き換えられる」の段が、場所の行がある DB でも通ること（`logical_source = 'immutable-check'` に絞られている）を確かめる。
  検証: `bash -o pipefail -c 'tools/check-immutable.sh | tee /tmp/ci.log' && grep -q "OK place" /tmp/ci.log`

## Task 2: 器の口 `POST /places`（design D1 / D15）

- [x] 2.1 `POST /places`（`{id, user_id}` → `{id}`）を足す。`INSERT … ON CONFLICT (id) DO NOTHING` の後で行の利用者を読み、同じなら 200、違えば 400 `{"error":"place_id_taken"}`。資格情報なしは 401。
  Scenario: `渡した識別子で場所の器ができる` / `同じ識別子で器を 2 回作っても 1 つ` / `別の利用者の器の識別子では作れない`。
  検証: `CT place_container_endpoint`

## Task 3: 場所の記録の取り込み（design D2 / D3 / D4 / D11。`crates/server/src/places.rs`）

- [x] 3.1 `places.rs` に原文の解釈（`parse_place_record(raw, id)`）と形の検査を置き、`ingest_one` が `logical_source = 's01-place'` のときだけ呼ぶ。
  `valid_from` は `attributes.rs` の解釈を呼ぶ（同じ規則を 2 か所に書かない）。`payload` は原文から `nonce` を除いて組み直し、文字列を NFC にする。
  理由の種別 10 個を `IngestError` に足す（spec の表）。座標の記録では先頭で場所ごとの錠（D4）。
  Scenario: `無い器を指す場所の記録は受け付けない` / `別の利用者の器を指す場所の記録は受け付けない` / `空の名前は受け付けない` / `範囲の外の緯度は受け付けない` /
  `WGS84 でない座標は受け付けない` / `座標を持つ場所に初めての座標は書けない` / `座標の無い場所は移れない` / `直す先の無い直す記録は受け付けない` /
  `座標をすべて消した場所にも初めての座標は書けない` / `移ったの精度と日付が合わなければ受け付けない` / `別の場所の座標は直せない` / `座標でない記録は直せない` / `消した座標の記録を直す先に指せる` /
  `広すぎる広さは受け付けない` / `何を書くものか分からない場所の記録は受け付けない` / `乱数が短い場所の記録は受け付けない` /
  `本人が書いたでない場所の記録は受け付けない` / `外部識別子を持つ場所の記録は受け付けない` / `場所の記録の拒否の応答に値が含まれない`。
  検証: `CT place_ingest_rejects`
- [x] 3.2 受け付けた記録の保存の形。
  Scenario: `同じ場所の記録の再送は増えない` / `場所の記録の原文が 1 バイトも変わらずに残る` / `原文と食い違う解析済みを送っても原文の座標で格納される` /
  `場所の記録の乱数は解析済みに写らない` / `消去後に残る列と正しい座標から場所の記録の鍵を作り直せない`。
  検証: `CT place_ingest_stores`
- [x] 3.3 同時に `first` を 2 本送っても座標の記録が 1 本だけ入ること（D4 の錠）。印は置かない（`座標を持つ場所に初めての座標は書けない` の裏側）。
  検証: `CT place_ingest_first_is_serialized`
- [x] 3.4 既定の感度（D11（仮））。`default_sensitivity` に `places::SOURCE` の枝を足し、`places::DEFAULT_SENSITIVITY = 1` を名指しで固定する。
  Scenario: `場所の記録は外部 AI に出してよいで格納される` / `場所を足しても主張の既定の感度は変わらない`。
  検証: `CT place_sensitivity`
- [x] 3.5 範囲の定数を名指しで固定する（`PLACE_RADIUS_MIN_M = 10` / `PLACE_RADIUS_MAX_M = 5000`。10 と 5,000 は通り、9 と 5,001 は断る）。
  検証: `CT place_ingest_radius_bounds`

## Task 4: いまの値と前の値、座標の版（design D6 / D7 / D15。`GET /places`）

- [x] 4.1 `places.rs` に純粋な関数 `view` と `coord_windows` を置き、単体テストで D6 / D7 の表を固定する（印は 4.2 / 5.2 の結合に置く）。
  **今日と現在時刻を差し込める形**（`App::at()` か引数）をここで作る —— 4.2 の Scenario（「2026-10-01 に読み出す」・未来の移転）が使う（spec-review R11）。
  `PLACE_DEFAULT_RADIUS_M = 100` を名指しで固定し、`stay::Criteria::default_values().radius_m` と別の定数であることも見る。
  検証: `CT place_view_unit`、`CT place_window_unit`
- [x] 4.2 `GET /places?user_id=` を足す（D15 の形。滞在の項はこの時点では 0 でよい —— Task 5 で埋める）。資格情報なしは 401。
  Scenario: `名前を 2 回変えると 3 つの名前の記録が残る` / `座標の記録は変え方を持つ` / `直す記録は直した座標の記録を指す` / `移ったのいつからは精度のまま残る` / `座標は丸めずに残る` /
  `場所の記録の書いた日時と D-01 に入った時刻が別々に入る` / `同じ名前に変え直しても 1 件増える` / `場所の名前は合成済みで読み出される` / `同じ名前の場所を 2 つ持てる` /
  `名前と座標と広さを変えても識別子が変わらない` /
  `名前を変えるといまの名前が変わり前の名前が残る` / `広さの記録の無い場所は 100 m` / `広さを変えるといまの広さが変わる` / `補足なしを書くと補足が消える` /
  `消したことにした名前の記録の前の名前がいまの名前に戻る` / `本文を消去した名前の記録は使わない` / `名前の記録が全部消えた場所は返らない` / `座標の記録の無い器は返らない` /
  `直した前の座標は直したものとして返る` / `移る前の座標は移る前のものとして返る` / `未来のいつからの移転はいまの座標を変えない` / `いまの座標の記録の識別子が返る` /
  `感度で場所の記録を絞らない` / `別の利用者の場所は読み出せない`。
  検証: `CT place_view_endpoint`
- [x] 4.3 4.1 の差し込み口で、`Asia/Tokyo` の日付で「いまの座標」が切り替わることを確かめる（UTC で 2026-03-31T15:00 に「いつから 2026-04-01」の移転がいまの座標になる）。
  検証: `CT place_view_today_is_tokyo`

## Task 5: 照合と、場所ごとの合計（design D7 / D8 / D9）

- [x] 5.1 `places.rs` に純粋な関数 `assign`（滞在 → 場所）と集計（件数・合計・最後に居た日・24 時間の帯・並び）を置き、`GET /places` の滞在の項を埋める。
  滞在は `core.event_live` の `s01-stay`・`origin='derived'`。**滞在にも場所の記録にも書き込まない。**
  試験の滞在は `/ingest` で `s01-stay` の派生を入れるか、位置の記録から作り直して作る（どちらでも代表点と時刻を試験が決められる形にする）。
  Scenario: `広さの中の滞在はその場所に当たる` / `広さの外の滞在は当たらない` / `広さを広げると外にあった滞在も当たる` / `2 つの場所に入る滞在は近いほうに当たる` /
  `同じ距離なら先に作った場所に当たる` / `照合は滞在に書き込まない` / `消した滞在は場所に当たらない`。
  検証: `CT place_match_`
- [x] 5.2 座標の版ごとの期間を、滞在の当たり方で確かめる（D7）。
  Scenario: `直すと全期間が新しい座標で照らされる` / `直すと前の座標の近くの滞在は外れる` / `移ったなら前の座標で居た時間もこの場所` /
  `移ったより前の新しい座標の滞在は当たらない` / `移ったより後の前の座標の滞在は当たらない` / `年だけの移ったはその年の初めから当てる` /
  `いつから分からない移転は書いた日まで両方の座標で当てる` / `後から書いた古いいつからの移転が後を占める` / `直す記録を消すと直す前の座標に戻る`。
  検証: `CT place_window_`
- [x] 5.3 集計と並び。
  Scenario: `場所の滞在の件数と合計が返る` / `最後に居た日が返る` / `24 時間の帯は時刻ごとの居た分を持つ` / `日をまたぐ滞在は両方の日の時刻に分かれる` / `場所は最近居た順に返る`。
  検証: `CT place_view_stays`

## Task 6: 名前の無い、よく居た所（design D10 / D15。`GET /places/candidates`）

- [x] 6.1 `places.rs` に純粋な関数 `candidates` を置き、`CANDIDATE_RADIUS_M = 100` を名指しで固定する。`GET /places/candidates?user_id=` を足す（全部返す）。資格情報なしは 401。
  Scenario: `名前の無い所は場所に当たらない滞在から作られる` / `100 m より離れた滞在は別の名前の無い所になる` /
  `中心から 90 m の滞在は同じ名前の無い所に入る` / `中心から 110 m の滞在は別の名前の無い所になる` / `名前の無い所は最近居た順に返る` /
  `登録すると名前の無い所から消える` / `消した滞在は名前の無い所に入らない`。
  検証: `CT place_candidates_`

## Task 7: 契約・縦串・偽データ（design D15 / D18）

- [x] 7.1 OpenAPI に 3 本の口と理由の種別を載せ、`docs/openapi.json` を再生成する。検証: `tools/check-openapi.sh` rc=0
- [x] 7.2 `tools/smoke.sh` に足す —— 位置の記録から滞在を作り、`/places/candidates` の中心で器と記録を送って登録 → 名前を変える → 座標を「間違いを直す」で変える。
  その後 `GET /places` の場所が 1 つで識別子が登録のときと同じこと（Story の完了の判定。D18）を `echo` で出して確かめる。
  検証: `bash -o pipefail -c 'tools/smoke.sh | tee /tmp/smoke.log' && grep -q "OK place id unchanged" /tmp/smoke.log`
- [x] 7.3 `tools/seed.sh`（`normal`）に場所を 2 つ足す（偽データの滞在のうち 2 か所に名前を付け、1 つは名前を 1 回変えておく）。名前の無い居た所が 1 つ以上残るようにする。
  `tools/smoke.sh` の**最後の段**に、同じサーバ（`BIND` / `API_TOKEN` を渡す）へ `tools/seed.sh normal` を当て、
  `curl -fsS "${AUTH[@]}" "http://$BIND/places"` の場所が 2 つ以上・`/places/candidates` の居た所が 1 つ以上なら `echo "OK seed places"` を出す段を足す
  （smoke は起動と後始末を自分で持つので、待ち続ける `tools/stack.sh up` を検証に使わない。spec-review R10）。
  検証: `bash -o pipefail -c 'tools/smoke.sh | tee /tmp/smoke.log' && grep -q "OK seed places" /tmp/smoke.log`

## Task 8: 画面 —— タブと場所のカード（design D12。`web/src/`。jsdom で指定と勘定）

- [x] 8.1 `places.ts` —— `GET /places` / `GET /places/candidates` の型と形の検査（**形が違えば失敗として出す**）、時間・日付・座標の書き方、原文の組み立て（D13）。
  原文の乱数は記録ごとに 128 bit（`crypto.getRandomValues`）で、識別子と一致しない。
  Scenario: `同じ内容の 2 つの場所の記録は別々の乱数を持つ`。
  検証: `VT places-model`
- [x] 8.2 `MasterView.tsx` のタブを「個人属性」（`#/master`）・「場所」（`#/master/places`）の 2 つにし、`Root.tsx` に `#/master/places` を足す。`#/master` は個人属性を開く。
  ST19 の `master-view.test.tsx` の「人物と場所のタブは無い」の試験を、2 つのタブと人物のタブが無いことを見る形に**直す**（印は置かない。印は 10.1 の e2e）。
  検証: `VT master-view`、`VT places-tabs`
- [x] 8.3 `PlacesView.tsx` —— カード（名前・合計・最後に居た日 / まだ居たことが無い・広さ・24 区分の帯・座標・「前の名前・座標 N」）、
  前の座標の「直した」「移る前（〜YYYY-MM）」「予定（YYYY-MM から）」の文字、場所が無いとき・読み出しの失敗。地図・`navigator.geolocation` を使わない。
  色は `tokens.ts` からだけ引く。
  Scenario: `場所の画面は確定した色だけを使う`。
  検証: `VT places-view`、`VT places-colors`

## Task 9: 画面 —— 足す・変える（design D13 / D14。jsdom で指定と勘定）

- [x] 9.1 「場所を足す」→ 名前の無い居た所（上位 10 件と「残り N か所」）→「名前を付ける」のフォーム（名前・書き換えられない座標・広さ 50 / 100 / 200 / 300（最初は 100）・補足）→「登録する」。
  器 → 記録の束の順で送り、押し直しは同じ器の識別子と同じ原文。送っている間は押せない。受理でフォームを閉じて読み直す。断られた・届かなかったの文は D13 の表。
  検証: `VT places-add`
- [x] 9.2 「名前を変える」「広さを変える」（いまの広さを最初に選ぶ）「座標を変える」（居た所から選ぶ・直すか移ったかを必ず選ぶ・移ったなら精度を先に・「前の座標で居た時間も「<名前>」のまま」）。
  「前の座標が間違っていた」はいまの座標の記録を直す先にする。
  検証: `VT places-change`

## Task 10: e2e（本物のブラウザ。`web/e2e/places.spec.ts`）

**画面の Scenario を「人間の確認待ち」へ逃がさない。** アサートするのは**数値と経路**（実寸・可視・URL・送った求めの本文と宛先）。スクリーンショット比較は使わない。
**場所は消せない**ので、手元で何度走らせても通る形にする —— 試験は走りごとに違う座標（偽データの居た所と重ならない範囲の乱数）で
**画面と同じ経路（`/api/ingest`）に位置の記録を送って自分の居た所を作り**（10 分以上・100 m 以内。作り直しは取り込みの後に走る）、件数は**同じ試験の中で読んだ `/api/places` と `/api/places/candidates` の値と突き合わせる**。
場所が 0 件・居た所が 0 件・登録 6 の量・読み出しの失敗・届かない・`place_id_taken` は `page.route` で応答を差し替えて作る。時刻の差し込みは `page.clock`。
**「届いたが応答が返らなかった」は `route.fetch()` でサーバへ通してから応答を捨てて作る**（`route.abort()` は届かない側。spec-review R9）。送った本文は `page.on('request')` で集めて比べる。

- [ ] 10.1 タブと一覧。
  Scenario: `タブは個人属性と場所の 2 つで人物のタブは無い` / `場所のタブを押すと場所の画面に移る` / `場所のタブには登録した場所のカードだけが出る` /
  `場所のカードは最近居た順に並ぶ`（並びを `/api/places` の順と突き合わせる）/ `場所のカードに合計と最後に居た日と広さと帯と座標が出る`（帯の区分が 24）/
  `滞在の当たらない場所はまだ居たことが無いと出る` / `前の名前と座標は押したときだけ出る` / `前の座標は直したか移ったかが文字で出る` / `前の座標の直したは文字で出る` /
  `予定の移転は予定の文字で出る` / `場所がまだ無いと出る` / `場所の画面は外へ求めを送らない`（`page.on('request')` で宛先の origin を全部集める）/
  `登録 6 の量で 1 画面目に場所が 3 枚見える`（design D21（仮）。`boundingBox().y < 640` のカードを数える）/
  `場所の識別子は画面に出ない`（`/api/places` の全識別子が `document.body.innerText` に無い）。
  検証: `ET places.spec.ts`
- [ ] 10.2 足す。
  Scenario: `場所を足すを押すと名前の無い居た所が上位 10 件出る`（居た所を 13 以上にしてから、出た件数 10 と「残り <API の件数 - 10> か所」）/ `残りを押すと全部出る` /
  `名前の無い居た所に手がかりが出る` / `名前を付けるフォームに緯度経度の欄が無い` / `広さの最初は 100 m` / `名前を付けて登録するとカードが増える` /
  `登録で送る座標は居た所の中心`（送った本文の座標と `/api/places/candidates` の中心が一致）/ `登録の書いた日時は入力させない` /
  `登録を 2 回押しても場所は 1 つ`（1 回目は `route.fetch()` で届けて応答を捨てる）/ `器の識別子が取られていたら押し直しで識別子を作り直す` /
  `登録を送っている間は登録するを押せない`（応答を遅らせて `toBeDisabled`）/ `登録が受理されるとフォームが閉じる` /
  `名前が空だと断られて入力が残る` / `場所の登録が届かなかったとき入力が残り届かなかったと出る` / `居た所がまだ無いと出る`。
  検証: `ET places.spec.ts`
- [ ] 10.3 変える。
  Scenario: `名前を変えるとカードの名前が変わる` / `広さを変えるといまの広さが選ばれている` / `広さを変えるとカードの広さが変わる` / `座標を変えるには直すか移ったかを選ぶ` /
  `座標の新しい値は居た所から選ぶ` / `間違いを直すといまの座標の記録を直す先に送る` / `移ったを選ぶといつからを精度から入れる` /
  `移ったを選ぶと前の時間もこの場所のままと出る` / `移ったを送ると移ったの記録が送られる` / `座標を変える先の居た所が無いと送れない` /
  `変えるを送っている間は押せない` / `変えるを 2 回押しても同じ原文を送る` / `変えるが受理されるとフォームが閉じる` /
  `変えるが届かなかったとき入力が残り届かなかったと出る` / `座標を変えて断られると入力が残る`。
  同じ試験の最後に、登録 → 名前を変える → 座標を直す の後で `/api/places` のその場所の識別子が登録のときと同じで、カードが 1 枚であることを見る（Story の完了の判定。D18）。
  検証: `ET places.spec.ts`
- [ ] 10.4 下限（幅 360 CSS px）。
  Scenario: `場所の読み出しの失敗と場所が無いことを区別する` / `名前の無い居た所の読み出しの失敗を区別する` / `場所の画面は触れる対象の下限を満たす`（`boundingBox` の幅と高さが 24 以上）/
  `場所の画面は文字のコントラストの下限を満たす`（明と暗で、カードの名前・補助の文字・帯の時刻の文字の `getComputedStyle` の色と背景の色から比を計算。design D20（仮））/
  `場所の画面はフォーカスの輪郭が見える`（`Tab` だけで「場所を足す」へ。`document.activeElement` と、輪郭の色と隣の面の色のコントラスト比を `getComputedStyle` から計算）/
  `場所の画面は OS の明暗に追従する`（`colorScheme: 'light'` と `'no-preference'`）。
  検証: `ET places.spec.ts`

## Task 11: 仕上げ

- [ ] 11.1 `docs/handoff/` を PR の前にもう一度読む（開始時と合わせて 2 回）。ST21 が渡すもの（`docs/handoff/ST23.md` の場所を消す口 / `docs/handoff/ST25.md` の S-2 の見出し）が置かれていることを確かめ、
  **PR 本文の「後続」に、S-2 の見出しを場所の名前にする `fix/`（ST21・ST22・ST25 の archive 後。`docs/handoff/ST25.md`）を挙げる**（起こす係を残す。design D19 / spec-review R14）。
  検証: `bash -c 'grep -q "st21-place-registry" docs/handoff/ST23.md && grep -q "st21-place-registry" docs/handoff/ST25.md'`
- [ ] 11.2 検査 4 本。検証: `python3 scripts/check_scenarios.py . st21-place-registry` rc=0、`python3 scripts/review_triage.py . st21-place-registry` rc=0、`python3 scripts/check_chain.py .` rc=0、
  この change の Scenario の名前が正典の他の Requirement の Scenario と重ならないこと（重なってよいのは design D20（仮）の 10 本だけ）:
  `bash -c 'comm -12 <(grep -h "^#### Scenario:" openspec/specs/personal-entities/spec.md | sort -u) <(grep -h "^#### Scenario:" openspec/changes/st21-place-registry/specs/personal-entities/spec.md | sort -u) | wc -l | grep -qx 10'`
- [ ] 11.3 まとめて緑。検証: `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace` /
  `cd web && npm run lint && npm run test && npm run build && npm run test:e2e` / `tools/check-immutable.sh` / `tools/check-migrations.sh` / `tools/check-openapi.sh` / `tools/smoke.sh` が全部 rc=0

## 人間の確認待ち

**無し。** 画面の Scenario は Task 10 の e2e（本物のブラウザ）が担保し、サーバの Scenario は Rust のテストが担保する。
機械が再現できない物理（ロック・電池・本物の GPS・時間そのもの・実機）に当たるものが、この Story には無い
（場所の登録・照合・画面はサーバと画面の中で完結し、滞在は試験が位置の記録から作る）。確認バッチの手順書が Story ごとに 1 問聞く
「触ってみて違和感は無かったか」だけが人間に渡る。
