# ST17 実装タスク — 毎日 30 秒で「その日どう感じたか」を残す

読む順: `deep.md`（**最優先。本人が決めた 13 件と、聞かずに決めた既定 C1〜C10**）→ このファイル →
`specs/subjective-log/spec.md` / `specs/browsing-views/spec.md` → `design.md` → `review/spec.md`（処置の理由）→ `docs/stories/ST17.md` →
`docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d --wait db` と `.env`（ST28 の `tools/db-roles.sh`）が前提。

## Global Constraints

- **ST17 は ST16 の滞在と ST22 の削除の上に積み、ST25 の 1 日の画面に差す。** `/stays` と `core.stay_absorbed` は読むだけ。**`browsing-views` は ST25 が走っている間は触らない**
  （admission が ST25 の archive を待つ）。Task 1.1 が ST25 の archive を確かめる。無ければ始めない
- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript / Kotlin はコメント（`// Scenario: 快–不快だけで記録が 1 件入る`）、bash は `echo`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は全部 ADDED**（`subjective-log` と `browsing-views`）。全部をこのファイルのどこかの Task が名指しする
- **画面の Scenario（Task 8〜10・12.1 に挙げたもの）は、全部に `web/e2e`（本物のブラウザ・本物のサーバ）の印を置く**（AGENTS.md「画面の Scenario は web/e2e で担保する」）。
  jsdom の印は文字の組み立てを細かく撃つための**追加**で、e2e の代わりにならない。各画面の Task の「e2e」の項目は、同じ Task の jsdom の項目に挙げた Scenario の**全部**に印を置く
- **「人間の確認待ち」に逃がせる Scenario は無い。** 通知は Android の単体（判断・予約・再起動）と計測テスト（通知と Intent）で機械が確かめる。時刻は時計を差し替えて再現する
- **e2e と消せない主観の行**: 主観の行は DB の錠で消せない。手元の e2e は DB を作り直さない（`reuseExistingServer`）ので、
  **書き込む e2e は実行ごとに乱数で選んだ 2030 年以降の日を `page.clock` の「今日」にして書き、件数は書く前との差分で数える**。
  「未記入」「使い始める前」「記入あり」の見え方の assert は、偽データ（Task 6）が固定で持つ日（e2e が書き込まない日）に対してだけ行う
- **件数つき検証**: このファイルで
  **`CT <名前>`** は `bash -o pipefail -c 'set -a; [ -f .env ] && . ./.env; set +a; cargo test -p ashiato-server <名前> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log`、
  **`VT <ファイル>`** は `bash -o pipefail -c 'cd web && npx vitest run src/__tests__/<ファイル> 2>&1 | tee /tmp/vt.log' && grep -Eq 'Tests +[1-9][0-9]* passed' /tmp/vt.log`、
  **`ET <ファイル>`** は `bash -o pipefail -c 'set -a; [ -f .env ] && . ./.env; set +a; cd web && npx playwright test e2e/<ファイル> 2>&1 | tee /tmp/et.log' && grep -Eq '[1-9][0-9]* passed' /tmp/et.log`、
  **`GT <クラス>`** は `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*<クラス>*'` rc=0 かつ
  `grep -Eq 'tests="[1-9]' collector-android/app/build/test-results/testDebugUnitTest/TEST-*<クラス>*.xml`、
  **`AT <クラス>`** は `scripts/quiet-run instr -- tools/android-emulator.sh` rc=0 かつ
  `find collector-android/app/build/outputs/androidTest-results -name 'TEST-*<クラス>*.xml' -exec grep -lE 'tests="[1-9]' {} + | grep -q .`（計測テストの正式な入口。`connectedDebugAndroidTest` は `--tests` を受けない。0 本で通らないよう結果の XML を見る）
  が rc=0 になることを指す（`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く）
- **試験の名前の接頭辞は Task ごとに重ならない**: `st17_lock_`（1）/ `st17_ingest_`（2）/ `st17_reject_`（3）/ `st17_read_`（4）/ `st17_remind_`（5）。
  2.2 / 2.3 は `st17_ingest_` の下の `nonce_` / `notcov_` で切り、2.1 の試験はそれらの前方一致にならない名前にする
- **本人の決定（下流は変えない）**: 押したら保存・ひとことと元気さは常に出す・見出し＋‹ ›・未記入は S-2 の頭と S-3 の暦の両方・滞在の気分を書く口は S-3 の下（Q1）/
  ひとこと 1 欄（Q2）/ 元気さ 1 本（Q3）/ 滞在の口を作る（Q4）/ 未来の日も書ける・前後は機械（Q5）/ 出来事の時刻は対象の時刻（Q12）/
  携帯が通知して Web を開く（Q6）/ 21:00（Q7）/ 書いた日は鳴らさない（Q8）/ **夜中も暦の今日**（Q9）/ 吸収先に出す・消した滞在の気分は隠す（Q10）/ 最後の 1 件と履歴（Q11）/ 未記入の数え方（Q13）/
  S-2 の行の中に主観を出す（ST16 の Q4。読むだけの 1 行）
- **本人の決定と仮決めの値は定数にして試験が名指しで固定する**（`docs/testing.md` §3）: `DEFAULT_REMINDER_AT = 21:00`・`NOTE_MAX_CHARS = 2000`（コードポイント）・`RANGE_MAX_DAYS = 62`・
  `REMINDER_WINDOW = 15 分`・`REMINDER_REFRESH_AT = 12:00`・`CALENDAR_DAYS = 30`・`ABSORB_HOPS_MAX = 16`・尺度の表（`valence` / `energy` 版 1、-2〜+2）。画面と携帯も同じ値を定数に持ち、それぞれの試験が固定する
- **D3 / D6 / D7 / D10 / D12 / D13 / D14 / D19 / D20 は（仮）決め。** 反転条件は `design.md` にある。変えたらその D 番号を書き直す
- **快–不快は数値と言葉で出し、色で分けない**（C1）。新しい色を `tokens.ts` に足さない
- **主観の値・ひとことをログに出さない**（製造準備 A-2）。サーバ・画面・携帯のどれも。出すのは件数・日付・種別・利用者だけ。`tools/check-log-private.sh` rc=0
- **携帯の通知は収集の経路と別のファイルに置く**（design D10）。`LocationService.kt` は予約を呼ぶ 1 行だけ

## Task 1: 前提の確認と移行（登録簿・錠・紐づけ先の種類・通知の時刻の台帳）

`migrations/YYYYMMDDHHMM_subjective_log.sql` と `.down.sql`（作成時刻）、`crates/server/src/lib.rs` の `MIGRATIONS` の末尾、`grants.sql`。試験は `crates/server/src/feelings_tests.rs`（新規）。design D2 / D9 / D15 / D16。

- [ ] 1.1 前提を確かめる: ST25 が archive 済み、ST22 の `deletion.rs` と `core.stay_absorbed` がある。
  検証: `bash -c 'ls -d openspec/changes/archive/*-st25-day-timeline && test -f crates/server/src/deletion.rs && grep -q stay_absorbed migrations/*_stays.sql'`
- [ ] 1.2 移行を書く: 登録簿 `('s01-feeling','主観',86400,'none')`、`core.feeling_target_kind`（日・滞在の 2 行。追記のみ）、`core.feeling_reminder`（追記のみ）、
  主観の行の 3 つの錠（`core.reject_feeling_rewrite` / `core.require_feeling_erasure_ledger` / `core.reject_feeling_delete`）。当て直せる形。`.down.sql` は行が残っていれば表と登録簿を残す。
  Scenario: `主観の値を書き換える文は拒まれる` / `主観の紐づけ先は書き換えられない` / `主観の出来事の時刻は書き換えられない` / `主観の行は削除できない` /
  `他の記録を主観へ付け替えられない` / `主観に削除の印を付けられる` / `主観の感度を変えられる` / `台帳のある主観の消去は通る` / `台帳の無い主観の消去は拒まれる` /
  `別の記録の台帳の行では主観の消去は通らない` / `台帳の行があっても消去の形でない主観の書き換えは拒まれる` / `主観以外の本人が書いた記録は従来どおりの規則で扱われる`。
  検証: `tools/check-migrations.sh` rc=0、`CT st17_lock_`（全版を 2 回当てて落ちない試験・2 つの台帳が追記のみの試験・`.down.sql` が行を残す試験を含む）
- [ ] 1.3 `tools/check-immutable.sh` に主観の節を足す（1.2 の Scenario を psql で 1 本ずつ。`echo "Scenario: …"`）。既存の `immutable-check` の段は変えず、主観の行がある DB で通ることを確かめる。
  検証: `tools/check-immutable.sh` rc=0

## Task 2: 主観を積む（取り込み口の解釈・感度・乱数・稼働状況の外）

`crates/server/src/feelings.rs`（新規）と `lib.rs` の `ingest_one` / `default_sensitivity`。design D1 / D3 / D4 / D5 / D19。

- [ ] 2.1 原文の解釈（`parse_feeling`）と、`payload` を原文から組み直す（NFC・`nonce` を除く・前/後を持たない）。`ingest_one` は `s01-feeling` のときだけ呼ぶ。`default_sensitivity` に `s01-feeling` → 2。
  Scenario: `快–不快だけで記録が 1 件入る` / `元気さとひとことを添えた記録が入る` / `尺度は識別子と版とともに残る` / `日の記録の出来事の時刻はその日の 0 時` /
  `記入の日時は出来事の時刻と別に残る` / `滞在の記録の出来事の時刻は書いた時点の滞在の始まり` / `滞在の時刻が作り直しで動いても記録の時刻は動かない` / `昨日の分を後から書ける` /
  `未来の日を紐づけ先にする記録を受け付ける` / `同じ値を書き直しても 1 件増える` / `同じ記録の再送は増えない` / `主観の原文が 1 バイトも変わらずに残る` /
  `合成済みでないひとことは合成済みで読み出される` / `原文と食い違う解析済みの主観を送っても原文の値で格納される` / `前後は格納した記録に持たない` /
  `主観はローカル AI までで格納される` / `主観以外の既定は主観を足しても変わらない`。
  検証: `CT st17_ingest_`
- [ ] 2.2 乱数（D4）: 解析済みと読み出しに写らないこと、乱数を知らずに鍵を作り直せないことを結合テストで。
  Scenario: `主観の乱数は解析済みに写らない` / `消去後に残る列と正しい値から主観の鍵を作り直せない`。検証: `CT st17_ingest_nonce_`
- [ ] 2.3 主観のソースが稼働状況に入らないことを固定する（`/coverage` の応答のソースと途絶の日に `s01-feeling` が無い）。
  Scenario: `主観のソースは稼働状況に含まれない`。検証: `CT st17_ingest_notcov_`

## Task 3: 形の合わない主観を断る

`feelings.rs` と `IngestError` の種別。design D5。

- [ ] 3.1 spec の表の 6 種別を表の順に確かめる。ひとことの上限 `NOTE_MAX_CHARS = 2000` をコードポイントで数えることを名指しで固定する試験（2,000 は通る・2,001 は断る・絵文字 2,000 個は通る）を置く。断った応答に値・ひとことが無いこと。
  Scenario: `快–不快の無い記録は受け付けない` / `範囲の外の快–不快は受け付けない` / `整数でない尺度の値は受け付けない` / `知らない尺度は受け付けない` /
  `同じ尺度が 2 つある記録は受け付けない` / `長すぎるひとことは切り詰めずに断る` / `上限ちょうどのひとことは受け付ける` / `ひとことの上限はコードポイントで数える` /
  `空のひとことはひとこと無しとして受け付ける` / `記入の日時の読めない記録は受け付けない` / `記録の地域が東京でない主観は受け付けない` / `乱数が短い主観は受け付けない` /
  `本人が書いたでない主観は受け付けない` / `端末識別子を持つ主観は受け付けない` / `外部識別子を持つ主観は受け付けない` / `2 つに当たる記録は表の上の種別で断られる` /
  `主観の拒否の応答に値が含まれない`。
  検証: `CT st17_reject_shape_`
- [ ] 3.2 紐づけ先の検査（登録簿 → 暦 → その利用者の滞在（削除の印は見ない）→ 区間の重なり → 出来事の時刻との一致）。
  Scenario: `登録簿に無い紐づけ先の種類は受け付けない` / `暦に無い日は紐づけ先にできない` / `無い滞在は紐づけ先にできない` / `別の利用者の滞在は紐づけ先にできない` /
  `滞在の区間と重ならない始まりと終わりは受け付けない` / `作り直しで少し動いた滞在の区間でも受け付ける` / `出来事の時刻が対象の時刻と違う記録は受け付けない` /
  `消した滞在を紐づけ先にできる` / `吸収された滞在を紐づけ先にできる`。
  検証: `CT st17_reject_target_`、`tools/check-openapi.sh` rc=0（理由の種別の値）

## Task 4: 読み出し（前/後・日の状態・吸収と削除）と `GET /feelings`

`feelings.rs`（純粋な `day_feelings`）と `lib.rs` のハンドラ。design D6 / D7 / D8。

- [ ] 4.1 前/後と日の状態を純粋な関数で（DB を持たない単体テスト）。`Asia/Tokyo` の日の境界と「使い始める前」「これから」。
  Scenario: `翌朝に書いた昨日の記録は後` / `当日の夜に書いたその日の記録は前` / `未来の日に書いた記録は前` / `滞在が終わった後に書いた記録は後` / `日付が変わった直後に書いた前の日の記録は後` /
  `日の記録がある日は記入あり` / `滞在にだけ書いた日も記入あり` / `前だけ書いた日も記入あり` / `書いていない日は未記入` / `最初の記録より前の日は使い始める前` /
  `未来の日は未記入に数えない` / `今日まだ書いていなければ今日は未記入` / `日をまたぐ滞在の記録は始まりの日に数える` / `主観の無い日の今日は Asia/Tokyo の日付で決まる`。
  検証: `CT st17_read_rules_`
- [ ] 4.2 `GET /feelings?from=&to=&user_id=`。最新は（紐づけ先・前/後）ごと、滞在は `stay_store::Mark` の 4 種で分け（吸収は `stay_absorbed` を `ABSORB_HOPS_MAX` 回までたどる）、
  表示しない滞在の記録は件数だけ、記録のある滞在の数。`RANGE_MAX_DAYS = 62` を名指しで固定。OpenAPI に足す。
  Scenario: `同じ日に 2 回書くと最新は後に書いたほう` / `前と後の最新は別々に返る` / `滞在ごとの記録は滞在ごとに返る` / `吸収された滞在の気分は吸収先に出る` /
  `消した滞在の気分は滞在ごとの欄に出ない` / `消した滞在の気分は数だけが返る` / `記録のある滞在の数は書き直しを 1 と数える` / `範囲を消して隠れた滞在の気分は滞在ごとの欄に出ない` /
  `消したことにした主観は読み出しに出ない` / `本文を消去した主観は読み出しに出ない` / `消したことにした記録しか無い日は未記入` / `本文を消去した記録しか無い日は未記入` /
  `読み出しは記入の日時と D-01 に入った時刻を別々に返す` / `感度で主観を絞らない` / `別の利用者の主観は読み出せない` / `長すぎる範囲は断られる` /
  `解釈できない日付の主観の求めは断られる` / `資格情報の無い主観の求めは断られる`。
  検証: `CT st17_read_api_`、`tools/check-openapi.sh` rc=0

## Task 5: 通知の時刻の台帳と通知の判断の口

`feelings.rs` と `lib.rs`。design D9。

- [ ] 5.1 `POST /feelings/reminder`（台帳に 1 行）と `GET /feelings/reminder?date=`（`at` と `recorded` の 2 欄だけ）。`DEFAULT_REMINDER_AT = 21:00` を名指しで固定。応答の鍵の集合を固定する試験を置く。OpenAPI に足す。
  Scenario: `通知の時刻の既定は 21 時` / `通知の時刻を変えると次の判断から効く` / `通知の時刻の変更は台帳に積まれる` / `書いた日の判断は記録ありになる` /
  `書いていない日の判断は記録なしになる` / `通知の判断は主観の中身を返さない` / `時刻でない通知の時刻は断られる`。
  検証: `CT st17_remind_`、`tools/check-openapi.sh` rc=0

## Task 6: 偽データ（e2e と確認バッチの前提）と縦串

`tools/seed.sh`・`tools/smoke.sh`。**画面の Task より先に置く**（e2e の見え方の assert はこの日を使う）。

- [ ] 6.1 `tools/seed.sh` の `normal` に、固定の 30 日（2026-09-04〜10-03）のうち 9 日が未記入の主観（日・滞在・前・後を含む。1 日は 3 回書き直し・1 日は前と後の両方）と、
  前の日から続く滞在・吸収された滞在・本人が消した滞在に付いた気分を足す（proto の量）。滞在のために要る位置の記録と作り直しも足す。
  **原文を固定し**（識別子・乱数・記入の日時を定数にする）、2 回流しても主観が増えない（冪等で畳まれる）ことを `tools/seed.sh` 自身が数えて確かめ、違えば非 0 で終える。
  検証: `bash -c 'tools/seed.sh normal && tools/seed.sh normal'` rc=0
- [ ] 6.2 `tools/smoke.sh` に、取り込み口から主観を 1 件入れて `GET /feelings` と `GET /feelings/reminder` で読む段を足す（`echo "Scenario: 快–不快だけで記録が 1 件入る"`）。
  検証: `scripts/quiet-run smoke -- tools/smoke.sh` rc=0

## Task 7: 画面の型と原文の組み立て（`feelings.ts`）

`web/src/feelings.ts`（新規）。試験は `web/src/__tests__/feelings.test.ts`。design D4 / D12 / D14。

- [ ] 7.1 型・原文の組み立て（`buildFeeling(target, scales, note, now)`。識別子・乱数・記入の日時と地域を決める）・ひとことの数え方（コードポイント）・前/後の判定（いま書けば付くほう）・
  日の状態の表示の文字・暦の 30 日の並び・定数（`NOTE_MAX_CHARS` / `CALENDAR_DAYS` など）。
  Scenario: `同じ内容の 2 つの主観は別々の乱数を持つ`（乱数が 128 bit・識別子と一致しない・同じ入力でも異なる）。
  検証: `VT feelings.test.ts`、`cd web && npm run lint` rc=0

## Task 8: 主観入力の画面（S-3）—— 押した瞬間に保存

`web/src/FeelView.tsx`（新規）、`Root.tsx` に `#/feel` と `#/feel/YYYY-MM-DD`。jsdom は `web/src/__tests__/FeelView.test.tsx`、e2e は `web/e2e/feel.spec.ts`（新規）。design D12。

- [ ] 8.1 快–不快の 5 つ（数値と言葉）・ひとこと・元気さ・見出しと ‹ ›・暦に無い日付のアドレス・送り方（D12）・結果の読み方・送り直し・保存の知らせ・最新の選択ともう一方の 1 行・書き直した履歴。
  Scenario: `通知から開いて 1 回押すと保存される` / `日付の無いアドレスは夜中も暦の今日を開く` / `暦に無い日付のアドレスは書く操作を出さない` / `保存したら対象の日が出る` /
  `昨日の分は前の日へ 1 回で書ける` / `次の日へ移って未来の日に書ける` / `快不快のボタンは数値と言葉を持つ` / `ひとことと元気さは押さずに見える` /
  `ひとことを足すと記録がもう 1 件積まれる` / `元気さを押すと今の快不快と一緒に積まれる` / `快不快を選ぶ前は元気さだけを送らない` / `届かなかった記録は同じ原文で送り直す` /
  `届かなかったとき選んだ値とひとことが残る` / `断られたとき理由の文が出る` / `開くと最新の記録の値が選ばれている` / `もう一方の前後の最新は 1 行で添えられる` /
  `書き直した履歴を開くと全部が出る` / `前後を選ぶ操作が無い` / `主観を書き換える操作が無い`。
  検証: `VT FeelView.test.tsx`
- [ ] 8.2 e2e: 8.1 の Scenario の**全部**に印を置く。幅 360 px・暗で `#/feel` を開いて 1 回押すと保存の応答が返り、DB の件数が 1 増える（ページを開いてから保存の応答まで 30 秒を下回る）。
  夜中は `page.clock` を `Asia/Tokyo` の 00:30 に置いて見出しを測る。届かない・断られるは `page.route` で応答を差し替える（送り直しの原文は要求の本文を比べる）。
  前/後の 1 行と履歴は、書き込み用の日に `page.clock` を動かして前と後を書いてから開く。
  検証: `ET feel.spec.ts`

## Task 9: S-3 の滞在の行・暦・通知の時刻の設定

`FeelView.tsx`。jsdom は `web/src/__tests__/FeelViewParts.test.tsx`、e2e は `web/e2e/feel-parts.spec.ts`（新規）。design D13 / D14。

- [ ] 9.1 滞在の行（`/stays` の `kind: "stay"`。44 px の 5 つ。後があれば後の最新・前の日から・行ごとの履歴・消した滞在の数）・暦（対象の日で終わる 30 日。未記入は破線と「未」）・通知の時刻の欄。
  Scenario: `その日の滞在が始まりの順に並ぶ` / `滞在の行を押すとその滞在に保存される` / `滞在の行に最新の気分が出る` / `前の日から続く滞在の行は前の日からと出る` /
  `前の日から続く滞在に書いた気分はその行に出る` / `滞在の書き直した履歴を開ける` / `消した滞在は気分の行に並ばない` / `消した滞在の気分は数だけ出る` /
  `吸収された滞在の気分は吸収先の行に出る` / `滞在の無い日は滞在が無いと出る` / `滞在の読み出しの失敗は滞在が無いと出ない` /
  `暦に未記入の日が破線と未で出る` / `暦に記入ありの日の値が出る` / `前と後がある日の暦は後の値を出す` / `使い始める前の日は未記入と出ない` /
  `これからの日は暦で未記入と出ない` / `暦は対象の日で終わる 30 日` / `暦の日を押すとその日を書ける` /
  `画面で通知の時刻を変えられる` / `画面で変えた通知の時刻が通知の判断に効く` / `通知の時刻を保存できなかったとき入れた値が残る` / `読み出しの失敗と未記入を区別する`。
  検証: `VT FeelViewParts.test.tsx`
- [ ] 9.2 e2e: 9.1 の Scenario の**全部**に印を置く。見え方は偽データの固定の日（Task 6）で、書き込み（滞在の行を押す・前の日から続く滞在に書く）は偽データの「前の日から続く滞在」を
  書き込み用の日に複製した位置で作る。未記入の枠は `border-style: dashed` と「未」の文字、`画面で変えた通知の時刻が通知の判断に効く` は `GET /feelings/reminder` の応答で、
  失敗は `page.route` で差し替える。**通知の時刻は最後に既定の 21:00 へ戻す**（台帳は追記のみなので、戻す行を足す）。
  検証: `ET feel-parts.spec.ts`

## Task 10: 1 日の画面（S-2）の頭の気分の欄と、滞在の行の気分

`web/src/DayFeeling.tsx`（新規）を `DayView.tsx` の見出しの下に 1 つ差し、滞在の行に 1 行添える（並びの組み立てには触らない）。
jsdom は `web/src/__tests__/DayFeeling.test.tsx`、e2e は `web/e2e/day-feeling.spec.ts`（新規）。design D14 / D20。

- [ ] 10.1 頭の欄（最新の数値と言葉 / 「気分 滞在ごとに N 件」/ 破線と「気分 未記入」と「書く」/「気分 使い始める前」/ これから / 失敗。1 行・40 px 以下）と、滞在の行の気分。並びの読み出しと別に `GET /feelings` を読む。
  Scenario: `書いた日の 1 日の画面の頭に気分が出る` / `書いていない日の 1 日の画面の頭に未記入が出る` / `1 日の画面の書くから主観入力の画面が開く` / `滞在にだけ書いた日は未記入と出ない` /
  `同じ滞在の書き直しは 1 と数える` / `使い始める前の日の 1 日の画面に未記入と出ない` / `これからの日の 1 日の画面に未記入と出ない` / `気分の読み出しの失敗は未記入と出ない` /
  `日を移ると気分の欄もその日のものになる` / `気分の欄は 1 行で 40 px 以下` /
  `気分を書いた滞在の行に気分が出る` / `滞在の行は後の気分を前より先に出す` / `1 日の画面で吸収された滞在の気分は吸収先の行に出る` / `気分の無い滞在の行には何も添えない`。
  検証: `VT DayFeeling.test.tsx`
- [ ] 10.2 e2e: 10.1 の Scenario の**全部**に印を置く（見え方は偽データの固定の日）。あわせて **ST25 の 1 日の画面の e2e を走らせ、気分の欄と行の 1 行を足しても通る**ことを確かめる（開いてすぐ見える行数・行の高さ・横溢れ）。
  検証: `ET day-feeling.spec.ts`、`ET day-timeline.spec.ts`、`ET day-stays.spec.ts`

## Task 11: 携帯の通知（判断・予約・再起動・通知・開く先）

`collector-android` の新しいファイル（design D10 の表）。単体は `app/src/test/.../FeelingReminderPolicyTest.kt`・`FeelingReminderReceiverTest.kt`（Robolectric）、計測は `app/src/androidTest/.../FeelingReminderInstrumentedTest.kt`。design D10 / D11。

- [ ] 11.1 `FeelingReminderPolicy.decide`（純粋。時計と応答を差し替える）。`Asia/Tokyo` の日、1 日 1 回、時刻より前は待つ、得られなければ通知、時刻の後に起きたらその日のうちに通知、
  最後に得た時刻を使う、12:00 の読み直しでその日の時刻を変える。`DEFAULT_REMINDER_AT` と `REMINDER_REFRESH_AT` を名指しで固定。
  Scenario: `書いていない日は通知の時刻に通知が出る` / `書いた日は通知が出ない` / `網に届かないときは通知が出る` / `同じ日に 2 回は通知しない` /
  `通知の時刻より前には通知しない` / `通知の時刻はサーバの設定に従う` / `午前に早めた通知の時刻はその日から効く` / `届かないときは最後に得た通知の時刻を使う`。
  検証: `GT FeelingReminderPolicyTest`
- [ ] 11.2 予約・受け手・再起動の受け手・通知（Robolectric）: 偽の HTTP で応答を差し替え、`AlarmManager` に `REMINDER_WINDOW`（15 分）の不正確な窓で予約されること・
  `BOOT_COMPLETED` で予約し直し、時刻を過ぎていれば判断すること・通知の題と本文が定数で値を含まないこと・`Intent` が `<webUrl>/#/feel/<その日>` を開くこと・
  `webUrl` が空のビルドでは `Intent` を付けずに通知を出すこと・受け手が起きた前後で位置の記録の生成と送信の試験（既存の `LocationService` まわり）が緑のまま。
  マニフェストに正確な時刻の権限が無く、`RECEIVE_BOOT_COMPLETED` があることを試験で読む。**gradle でビルドを止めない**（CI・計測テスト・確認バッチは `ashiato.webUrl` を渡さない）。
  Scenario: `通知は 15 分の窓で予約される` / `通知の時刻の後に再起動した端末はその日のうちに通知する` / `通知の文面に主観の中身が出ない` / `通知の仕組みは正確な時刻の権限を求めない` /
  `通知を押すと今日の主観入力の画面が開く` / `画面のアドレスを持たないビルドでも通知は出る` / `通知の仕組みは位置の送信を止めない`。
  検証: `GT FeelingReminderReceiverTest`、`scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest` rc=0（既存の単体が全部緑）
- [ ] 11.3 計測テスト（エミュレータ）: 本物の通知の仕組みで、記録の無い日に受け手を起こすと通知が 1 件出て、押すと `ACTION_VIEW` の URI が S-3 のアドレスであること。記録のある日は出ないこと。
  `BOOT_COMPLETED` を送って予約が戻ること。
  Scenario: `通知を押すと今日の主観入力の画面が開く` / `書いていない日は通知の時刻に通知が出る` / `書いた日は通知が出ない` / `通知の時刻の後に再起動した端末はその日のうちに通知する`。
  検証: `AT FeelingReminderInstrumentedTest`

## Task 12: 画面の下限と全体の検査

e2e は `web/e2e/feel-limits.spec.ts`（新規）。

- [ ] 12.1 e2e で S-3 の下限を測る（明と暗のそれぞれ。幅 360 px）。コントラストは計算した比、実寸は `getBoundingClientRect`、フォーカスは `Tab` で移して輪郭の色。
  色の出所は、`tokens.ts` 以外の色の文字列が `FeelView.tsx` / `DayFeeling.tsx` に無いことを jsdom 側でも固定する。
  Scenario: `快不快のボタンは高さ 64 px 以上` / `快不快のボタンは 1 列に並ぶ` / `幅 360 px で横のスクロールが出ない` / `主観入力の画面は触れる対象の下限を満たす` /
  `主観入力の画面は文字のコントラストの下限を満たす` / `押された状態と未記入の破線は 3:1 以上` / `主観入力の画面はフォーカスの輪郭が見える` /
  `主観入力の画面は OS の明暗に追従する` / `主観入力の画面は確定した色だけを使う`。
  検証: `ET feel-limits.spec.ts`、`VT FeelViewParts.test.tsx`
- [ ] 12.2 全体の検査: `python3 scripts/check_scenarios.py . st17-daily-feeling` rc=0（担保なし 0）、`tools/check-log-private.sh` rc=0、`tools/check-openapi.sh` rc=0、
  `bash -c '! cargo test -p ashiato-server st17_ -- --list 2>/dev/null | grep ": test$" | grep -vq "st17_"'` rc=0（`st17_` で絞った一覧にこの change 以外の試験が出ない）
