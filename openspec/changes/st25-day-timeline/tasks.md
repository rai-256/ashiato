# ST25 実装タスク — 1 日を時刻順に見る

読む順: `deep.md`（**最優先。本人が決めた 6 件と、聞かずに決めた既定 C1〜C11**）→ このファイル →
`specs/browsing-views/spec.md` → `design.md` → `review/spec.md`（処置の理由）→ `docs/stories/ST25.md` →
`docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は足さない**（design D3）。**`/events`・`/stays`・`/stays/detail`・`/ingest` の応答、`coverage.rs` の判定、`stay_store::day_view` の行の判定、
`core.event_folded`、PC の収集（`crates/collector-windows`）には触らない**（design D14）。並走中の st05 / st06 / st08 / st12 / st28 の change のファイルも触らない。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d --wait db` が前提（ST28 が merge されていれば `.env` と `tools/db-roles.sh` も）。

## Global Constraints

- **ST25 は ST22 の上に積む**（`requires: [ST01, ST22]`）。ST25 の画面は ST22 の「開ける行」（`DayView.tsx` の `openId`・`GET /stays/detail`・「この滞在を消す」）を広げる。
  Task 1.1 が ST22 のコードが main にあることを確かめる。無ければ始めない
- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript はコメント（`// Scenario: 10 分以上の離席は行になる`）、bash は `echo`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 86 本**（`browsing-views` の ADDED のみ。うち 2 本 —— `日付を含むアドレスでその日の一覧が開く` / `前の日へ移ると前の日の滞在が出る` —— は ST16 の印がそのまま使える）。
  REMOVED した要件の Scenario `稼働状況の画面の入口は変わらない` の印（ST22 の後は `web/src/__tests__/DayView.test.tsx`）は、**試験ごと** `ルートを開くと今日の 1 日の画面が出る` に直す（印を残すと嘘の試験が残る）
- **画面の文字を言う Scenario は、サーバの試験（数と規則）と jsdom（文字の組み立て）の両方に印を置く**（spec-review R15）。
  サーバの試験は `/day` の JSON までしか撃てない。「行がある」「文字が添えてある」は jsdom が描いた文字列で確かめる
- **「人間の確認待ち」に逃がせる Scenario は無い。** 実寸・フォーカス・URL・明暗は e2e（本物のブラウザ）、数と規則はサーバの試験と jsdom
- **件数つき検証**: このファイルで
  **`CT <名前>`** は `bash -o pipefail -c 'set -a; [ -f .env ] && . ./.env; set +a; cargo test -p ashiato-server <名前> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log`、
  **`VT <ファイル>`** は `bash -o pipefail -c 'cd web && npx vitest run src/__tests__/<ファイル> 2>&1 | tee /tmp/vt.log' && grep -Eq 'Tests +[1-9][0-9]* passed' /tmp/vt.log`（**このタスクで新しく作るファイル**を名指しする）、
  **`ET <ファイル>`** は `bash -o pipefail -c 'set -a; [ -f .env ] && . ./.env; set +a; cd web && npx playwright test e2e/<ファイル> 2>&1 | tee /tmp/et.log' && grep -Eq '[1-9][0-9]* passed' /tmp/et.log`
  が rc=0 になることを指す（`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く）
- **試験の名前の接頭辞は Task ごとに重ならない**（spec-review R16。前方一致で別の Task の試験に当たらない）:
  `st25_allow_`（1.2）/ `st25_fold_`（1.3）/ `st25_range_`（1.4）/ `st25_api_records_`（2.1）/ `st25_api_unchanged_`（2.2）/ `st25_usage_`（3.1）/ `st25_summary_`（3.2）/ `st25_other_`（3.3）/ `st25_api_day_`（3.4）/
  `st25_gaprow_`（4.1）/ `st25_browser_`（4.2）/ `st25_reasons_`（5.1）/ `st25_nostay_`（5.2）。
  作ったら `cargo test -p ashiato-server st25_ -- --list` にこの change の試験しか出ないことと、各接頭辞が他の接頭辞の前方一致になっていないことを確かめる
- **本人の決定（下流は変えない）**: 行の単位は滞在・閉じた行にアプリ上位 3 とサイト上位 3・開くと同じアプリの連続を畳んだ時刻順・取れていない時間は行（離席は 10 分以上）（Q1）/
  2 台の PC の同じ訪問だけ畳み、書庫と手元の組・書庫どうしの組は両方出す（Q2）/ 記録なしに事情を添え、判定は変えない（Q3）/ ルートは今日の 1 日の画面（Q4）/
  明暗はこの Story の画面だけ（Q5）/ 滞在以外を消す口は足さない（Q6）
- **本人の決定の値は定数にして試験が名指しで固定する**（`docs/testing.md` §3）: `SUMMARY_TOP = 3`・`AWAY_ROW_MIN_MINUTES = 10`。画面の側も同じ値を定数に持ち、jsdom の試験が固定する
- **D4 / D5 / D6 / D7 の一部 / D8 の一部 / D10 / D11 の並び / D15 は（仮）決め。** 反転条件は `design.md` にある。変えたらその D 番号を書き直す
- **種類は文字で出し、色で分けない**（C1）。新しい色を `tokens.ts` に足さない
- 記録の値（題名・URL・アプリ名・座標）を**ログに出さない**（製造準備 A-2）。出すのは件数・日付・種別・利用者だけ

## Task 1: 前提の確認と、記録の正規化（許可リスト・畳み・日の範囲）

`crates/server/src/day_records.rs`（新規）。試験は `crates/server/src/day_records_tests.rs`。design D2 / D3 / D4 / D5。

- [ ] 1.1 前提を確かめる: ST22 のコードが main にある。ST06 / ST08 / ST12 のうち main に入ったものは、`docs/collector-contract.md` の payload の鍵名を design D2 の表と突き合わせ、違えば D2 の表を直してから進む。
  あわせて、**登録簿に実在する `c03-%` の論理ソースがどれも D2（活動）か D11（位置の点）の表にある**ことを試験にする（表に無い名前を列挙して落ちる）。
  検証: `bash -c 'test -f crates/server/src/deletion.rs && grep -q "\"erased\"" web/src/stays.ts'`、`CT st25_allow_registry_covers_archive`
- [ ] 1.2 許可リスト（D2 の表）を 1 つの定数の表にし、`core.event` の行を 2 段で正規化する（1 段目はスマホの `background` を持つ内部の形、2 段目が `/day/records` に出す形）。
  除外は本文を写さない。離席は leave だけ（今日に限り、leave の無い enter をいままでの離席にする）。感度で外さない。表の行ごとに試験を置く（偽の記録を 1 件ずつ）。
  Scenario: `許可リストに無い種別の記録は返らない` / `除外の記録は件数を持つ` / `除外の記録は本文を持たない` / `地域の違う記録はその地域を持って返る` / `感度の高い記録も返る`。
  検証: `CT st25_allow_`
- [ ] 1.3 読み出しの SQL（D3）: `core.event_live` を日付で先に絞り、`DISTINCT ON (logical_source, content_hash)` を `ingest_time, id` の順で畳む。続けて訪問を `(event_time, url)` で畳む（D4）。
  Scenario: `削除済みの記録は返らない` / `内容が同じ記録は 1 件として返る` / `2 台の PC から届いた同じ訪問は 1 件になる` / `書庫と手元の同じ出来事は両方返る` / `書庫どうしの同じ出来事も両方返る`。
  検証: `CT st25_fold_`
- [ ] 1.4 日の範囲（D5）: 点は出来事の時刻、区間は重なりで判定（SQL は ±40 日で索引に当て、重なりは Rust）。`from` / `to` も同じ判定。
  Scenario: `日の区切りは Asia/Tokyo で決まる` / `前の日から続く区間はその日にも返る` / `他の日の記録は返らない`。
  検証: `CT st25_range_`

## Task 2: `GET /day/records`（記録を 1 件ずつ読む口）

design D1。ハンドラは `lib.rs`、組み立ては `day_records.rs`。

- [ ] 2.1 `GET /day/records?date=&from=&to=&user_id=` を足す。出来事の時刻順。認証・利用者・日付の解釈は `/stays` と同じ。OpenAPI に足す（`utoipa`）。
  Scenario: `その日の記録が出来事の時刻順に 1 件ずつ返る` / `時刻の範囲を指定するとその範囲の記録だけが返る`（範囲の外から始まって中で終わる離席を含める） /
  `日付として読めない求めは断られる` / `資格情報の無い 1 日の記録の求めは断られる`。
  検証: `CT st25_api_records_`、`tools/check-openapi.sh` rc=0
- [ ] 2.2 既存の口の形が変わらないこと。`/events` と `/stays` の応答の鍵の集合と並びを、口を足す前と同じ入力で固定する（`/events` は `payload` を返さないまま）。
  Scenario: `記録の読み出しの形は変わらない`。検証: `CT st25_api_unchanged_`

## Task 3: 使った時間・要約・ほかの記録の行と、`GET /day`

`crates/server/src/day.rs`（新規）。試験は `crates/server/src/day_tests.rs`。design D1 / D6 / D7。

- [ ] 3.1 使った時間（D6）: PC は次の前景の変化（次の前景・除外の始まり）まで、間の離席（10 分未満も）・除外・停止を差し引く。
  スマホは次の別のアプリの前景か同じアプリの `background` まで、1 回 10 分で打ち切る。行の範囲の外は数えない。
  Scenario: `離席の時間はアプリを使った時間に数えない` / `10 分に満たない離席も使った時間から差し引く` / `スマホの使った時間は 1 回 10 分で打ち切る`。
  検証: `CT st25_usage_`
- [ ] 3.2 要約（D7）: 行の `[start, end)` ごとに PC / スマホ（時間と上位 3）・ブラウザ（件数とドメインの上位 3）・書庫（表示名ごとの件数）。同点は名前の昇順、ドメインは小文字で `www.` を外す。
  記録の無いソースは鍵ごと省く。「消した」の行には付けない。`SUMMARY_TOP` を名指しで固定する試験を置く。
  Scenario（サーバ側の数）: `滞在の行に PC のアプリの上位 3 つと時間が出る` / `同じ時間のアプリは名前の順に並ぶ` / `ドメインは www を外して小文字で束ねる` /
  `ブラウザは見たサイトのドメインの上位 3 つと件数が出る` / `スマホのアプリの上位 3 つと時間が出る` / `書庫の記録は表示名ごとの件数で出る` /
  `記録の無いソースの要約は出ない` / `記録なしの行にもその時間の要約が出る` / `消した行には要約が出ない` / `日付をまたぐ行の要約は両方の日で同じ`。
  検証: `CT st25_summary_`
- [ ] 3.3 ほかの記録の行（D7）: `day_view` の行の隙間（消した時間・今日の最後の位置からいままで）に位置以外の記録があれば `kind: "other"` の行を足し、要約を付ける。`day_view` は変えない。
  Scenario（サーバ側）: `どの行にも入らない時間の記録はほかの記録の行に出る` / `今日の最後の位置の後の記録はほかの記録の行に出る`。
  検証: `CT st25_other_`
- [ ] 3.4 `GET /day?date=&user_id=` を足す。`entries` は `stay_store::day_view` の結果（ST22 の「消した」を含む）に `summary` とほかの記録の行を足したもの。OpenAPI にも足す。
  検証: `CT st25_api_day_`、`tools/check-openapi.sh` rc=0

## Task 4: 取れていない時間とブラウザの「最後に届いた時刻」

`day.rs`。design D9 / D10。

- [ ] 4.1 `gaps`: 離席（`AWAY_ROW_MIN_MINUTES` 以上だけ。定数を名指しで固定する試験と、ちょうど 10 分・9 分 59 秒の境界の試験を置く。今日はいま離席中も）・PC の停止・
  除外（次の前景の記録まで。続く除外の記録を 1 行にまとめて件数を足す）・スマホ（`reason` を写す）。日の端といまで切る。
  Scenario（サーバ側）: `10 分以上の離席は行になる` / `10 分に満たない離席は行にならない` / `いま離席中の時間は今日の行になる` / `PC の停止は行になる` /
  `除外は件数だけの行になる` / `送る契機で区切られた除外は 1 行にまとまる` / `スマホの取得元に残っていなかった期間は行になる` / `時計のずれで取り直さなかった期間は別の文字の行になる`。
  検証: `CT st25_gaprow_`
- [ ] 4.2 `browser_through`（D10）: `c02-browser-history` の `ingest_time` の最大（削除済みを含む）。生存信号は使わない。収集を始めた日より前の日には出さない。
  Scenario（サーバ側）: `まだ届いていないブラウザ履歴は行になる` / `届いた日にはまだ届いていない行は出ない` / `ブラウザ履歴を集めていない日にはまだ届いていない行は出ない`。
  検証: `CT st25_browser_`

## Task 5: 記録なしの事情

`day.rs`。design D11。**`coverage.rs` を呼ばない・書き換えない**（読む SQL を別に書く）。

- [ ] 5.1 記録なしの行ごとに `reasons`（導入前・止めていた・破棄（件数）・書庫に位置（件数。タイムラインと移行前の両方を数える））を D11 の順で付ける。行の始まりと終わりは変えない。
  止めていた時間は `core.coverage_span` に試験から直接行を入れて確かめる（`testdb.rs` と同じ）。
  Scenario（サーバ側）: `収集を始める前の日の記録なしにはそう添えられる` / `端末が破棄した時間の記録なしには件数が添えられる` /
  `書庫に位置がある時間の記録なしには件数が添えられる` / `本人が止めていた時間の記録なしにはそう添えられる` /
  `事情が重なると並べて添えられる` / `事情の無い記録なしには何も添えない` / `事情は記録なしの行の範囲を変えない`。
  検証: `CT st25_reasons_`
- [ ] 5.2 書庫の位置から滞在を作らないことを固定する（書庫の位置の点だけがある時間に、`/stays` の滞在が増えない）。
  Scenario: `書庫の位置からは滞在を作らない`。検証: `CT st25_nostay_`

## Task 6: 偽データ

`tools/seed.sh`。design D13。

- [ ] 6.1 `normal` の 2026-09-07 に、PC の前景（同じアプリの連続を含む）・離席 4 回（うち 1 回は 10 分未満）・除外 1 回（件数 3）・PC の停止 23:40〜・スマホの前景と後ろに下がったイベントを足す。
  ブラウザの訪問（2 台目の PC からの同じ訪問を含む）は、**`c02-browser-history` の `external_id_kind` が `record` のとき（ST08 の移行が済んでいるとき）だけ**入れ、そうでなければ入れないと出して続ける（design D13。spec-review R18）。
  書庫は入れない。登録簿に無い論理ソースは先頭で `ON CONFLICT DO NOTHING` で足す。固定の乱数で作り、2 回入れても件数が変わらない（再送として畳まれる）。
  検証: `bash -c 'tools/stack.sh up && tools/seed.sh normal && tools/seed.sh normal'`、
  `bash -c 'set -a; . ./.env 2>/dev/null; set +a; curl -sf -H "authorization: Bearer ${API_TOKEN:-dev-token-0123456789abcdef}" "http://127.0.0.1:18787/day?date=2026-09-07" | jq -e "([.gaps[] | select(.kind==\"away\")] | length) == 3 and ([.gaps[] | select(.kind==\"excluded\")] | length) == 1 and ([.entries[] | select(.summary.pc != null)] | length) >= 1 and ([.entries[] | select(.summary.phone != null)] | length) >= 1"'`

## Task 7: 画面 —— ルート・行き先・明暗

`web/src/Root.tsx` / `App.tsx` / `DayView.tsx`（`nav` と見出し）/ `docs/screens.md`。design D12。

- [ ] 7.1 ルート（空・`#/`・解釈できないアドレス）を今日の 1 日の画面に、`#/coverage` を稼働状況にする。1 日の画面の上に「稼働状況」「マスタ管理」の行き先、稼働状況に「1 日の画面へ」とマスタ管理への行き先。見出しを日付にする。
  REMOVED した Scenario の試験（`DayView.test.tsx` の `稼働状況の画面の入口は変わらない`）を直す。
  Scenario: `ルートを開くと今日の 1 日の画面が出る` / `稼働状況は専用のアドレスで開く` / `稼働状況から 1 日の画面へ移れる` /
  `解釈できないアドレスでは今日の 1 日の画面が出る` / `1 日の画面の見出しは日付`。
  検証: `VT st25-routes.test.tsx`、`bash -c '! grep -rn "Scenario: 稼働状況の画面の入口は変わらない" web crates'`
- [ ] 7.2 `DayView` の読み出し先を `/api/stays` から `/api/day` へ付け替え、**`/api/stays?date=` の呼び出しを固定している既存の試験**（ST22 の後の `DayView.test.tsx` / `DayView-erase-erased-row-keyboard.test.tsx` など）を
  `/api/day` に付け替える（ST16 / ST22 の印は残す）。暦に無い日付の画面を `useScheme()` にする（C10）。
  検証: `bash -c 'cd web && npx vitest run'`（既存の試験が全部緑）、`bash -c '! grep -rn "api/stays?date" web/src/DayView.tsx'`
- [ ] 7.3 `docs/screens.md` の表を直す（S-1 は `/#/coverage`、S-2 は `/`）。`web/e2e/stack.spec.ts` と `coverage-year.spec.ts` は `/#/coverage` を開く。
  検証: `bash -c 'grep -q "#/coverage" docs/screens.md'`、`ET stack.spec.ts`、`ET coverage-year.spec.ts`

## Task 8: 画面 —— 要約・取れていない時間の行・記録なしの事情・ほかの記録の行

`web/src/DayView.tsx` / `web/src/timeline.ts`（新規。型・形の検査・`placeGaps`）。design D7 / D9 / D11。

- [ ] 8.1 各行に要約を文字で出す（「PC 2 時間 45 分 — VS Code 1:30 · …」「ブラウザ 32 件 — github.com 18 · …」「書庫: <表示名> 12 件」）。ほかの記録の行（「ほかの記録 始まり – 終わり」）を描く。
  形が違う応答は失敗として出す（`isDayView` と同じ規律）。固定の `/api/day` の応答を描いて文字列を確かめる。
  Scenario（画面の文字）: `滞在の行に PC のアプリの上位 3 つと時間が出る` / `ブラウザは見たサイトのドメインの上位 3 つと件数が出る` / `スマホのアプリの上位 3 つと時間が出る` /
  `書庫の記録は表示名ごとの件数で出る` / `記録の無いソースの要約は出ない` / `記録なしの行にもその時間の要約が出る` / `ソースの種類は文字で出る` / `消した行には要約が出ない` /
  `どの行にも入らない時間の記録はほかの記録の行に出る` / `今日の最後の位置の後の記録はほかの記録の行に出る`。
  検証: `VT st25-summary.test.tsx`
- [ ] 8.2 取れていない時間の行と事情: `placeGaps`（D9）で `gaps` を始まりを含む行の直後に始まりの時刻順で差し込む（どれにも入らなければ始まりより前で最も近い行の直後。ブラウザの行は並びの先頭）。
  行の文字（「離席」「PC 停止」「除外 …（N 件。本文なし）」「スマホ 取得元に残っていない」「スマホ 時計のずれで取れていない」「ブラウザ …」）と、記録なしの事情の文字を描く。
  画面側の `AWAY_ROW_MIN_MINUTES` を名指しで固定する。
  Scenario（画面の文字と位置）: `10 分以上の離席は行になる` / `10 分に満たない離席は行にならない` / `いま離席中の時間は今日の行になる` / `PC の停止は行になる` /
  `除外は件数だけの行になる` / `除外の行に本文は出ない` / `送る契機で区切られた除外は 1 行にまとまる` / `スマホの取得元に残っていなかった期間は行になる` /
  `時計のずれで取り直さなかった期間は別の文字の行になる` / `まだ届いていないブラウザ履歴は行になる` / `届いた日にはまだ届いていない行は出ない` /
  `ブラウザ履歴を集めていない日にはまだ届いていない行は出ない` / `取れていない時間の行は始まりの時刻の位置に並ぶ` / `取れていない時間の行は文字で区別される` /
  `収集を始める前の日の記録なしにはそう添えられる` / `端末が破棄した時間の記録なしには件数が添えられる` / `書庫に位置がある時間の記録なしには件数が添えられる` /
  `本人が止めていた時間の記録なしにはそう添えられる` / `事情が重なると並べて添えられる` / `事情の無い記録なしには何も添えない` / `事情は記録なしの行の範囲を変えない`。
  検証: `VT st25-gaps.test.tsx`
- [ ] 8.3 新しく使う文字と背景の色の組を、ライトとダークの両方で 4.5:1 以上に固定する（`web/src/contrast.ts` と同じ形）。行の種類を枠や線の色で担わせない。
  Scenario: `要約と中身の文字はライトでもダークでも 4.5:1 を下回らない`。検証: `VT st25-contrast.test.ts`

## Task 9: 画面 —— 開いた中身と畳み

`web/src/DayView.tsx` / `web/src/timeline.ts`（`foldRuns`）。design D8。

- [ ] 9.1 滞在・移動・記録なし・ほかの記録の行を開ける行にする（ST22 の `openId` 1 つのまま。一度に 1 つ）。開くと `/api/day/records?date=&from=&to=` を 1 回読み、時刻順に出す。
  滞在の行は ST22 の件数（場所 N 件）と「この滞在を消す」を末尾に残す。地域が Asia/Tokyo でない記録に地域の時刻を添える。
  Scenario: `行を開くとその時間の記録が時刻順に出る` / `位置の点は件数で出る` / `地域の違う記録にはその地域の時刻が添えられる` /
  `開ける行は一度に 1 つ` / `滞在の詳細の消す操作は残る`。
  検証: `VT st25-open.test.tsx`
- [ ] 9.2 `foldRuns`: 同じ `source` の同じアプリ（ブラウザは同じ `browser`）の連続を 1 行に畳み、始まり・終わり・件数。取れていない時間は畳まない。畳んだ行は一度に 1 つ開く。
  Scenario: `同じアプリの連続は 1 行に畳まれる` / `別のアプリが挟まると別の行になる` / `畳んだ行を開くと中の全件が時刻順に出る` / `畳んだ行も一度に 1 つ`。
  検証: `VT st25-fold.test.ts`

## Task 10: e2e（本物のブラウザ。`web/e2e/day-timeline.spec.ts`）

偽データ（`SEED=normal`）の `#/day/2026-09-07` を使う。**アサートするのは数値と経路**（実寸・可視・フォーカス・URL・背景の色）。スクリーンショット比較は使わない。
各 Scenario は別の `test(...)` にし、`ET day-timeline.spec.ts` の通った件数がこの Task の Scenario の数（14）以上であることを見る。

- [ ] 10.1 幅 360 × 高さ 640: 閉じた画面に PC の前景の題名が 1 つも出ず行の数が 100 未満 / 画面の中に全体が入る行が 6 行以上 / 横スクロールが無い / 閉じた行の高さがどれも 200 以下。
  Scenario: `閉じた画面に記録 1 件ごとの行は出ない` / `幅 360 px で開いてすぐ 6 行以上見える` / `幅 360 px で横にはみ出さない` / `閉じた行は 200 px を超えない`。
- [ ] 10.2 行を開く操作・畳んだ行を開く操作・「稼働状況」「マスタ管理」の行き先の `boundingBox` が 24 以上。行き先を押すと URL が `#/coverage` / `#/master` になり、その画面が出る。
  時刻順の記録が 100 件以上ある滞在の行を開いて「この滞在を消す」が 44 以上。
  Scenario: `開く操作と行き先は 24 px を下回らない` / `1 日の画面から稼働状況へ移れる` / `1 日の画面からマスタ管理へ移れる` / `時刻順の記録を足しても消す操作は 44 px を下回らない`。
- [ ] 10.3 Tab で滞在の行へ移って Enter → 中の畳んだ行へ Tab → Enter で全件が出る。そのたびに `activeElement` に輪郭（`outline-style` が `none` でない）。
  Scenario: `畳んだ行はキーボードで開ける` / `キーボードで行を開くとフォーカスの位置が見える`。
- [ ] 10.4 明暗: `emulateMedia` でダーク → ライト → ダーク と切り替え、1 日の画面の背景の輝度が上がって戻る。`colorScheme: "no-preference"` で背景がダークの色。
  `colorScheme: "light"` で `#/day/2026-13-45` の画面の背景がライトの色。
  Scenario: `OS をライトに切り替えると 1 日の画面が明るくなる` / `OS の明暗が取得できないときはダーク` / `暦に無い日付の画面も OS の明暗に追従する`。
- 検証（10.1〜10.4 共通）: `ET day-timeline.spec.ts`、`bash -c 'grep -c "^test(" web/e2e/day-timeline.spec.ts | xargs test 14 -le'`

## Task 11: 仕上げ

- [ ] 11.1 検査。検証: `python3 scripts/check_scenarios.py . st25-day-timeline` rc=0、`python3 scripts/review_triage.py . st25-day-timeline` rc=0、`python3 scripts/check_chain.py .` rc=0
- [ ] 11.2 まとめて緑。検証: `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace` /
  `cd web && npm run lint && npm run test && npm run build && npm run test:e2e` / `tools/check-openapi.sh` / `tools/smoke.sh` が全部 rc=0
- [ ] 11.3 `docs/handoff/` を PR の前にもう一度読む（開始時と合わせて 2 回）。読んだ結果（ST25 宛ての申し送りの有無と処置）を `review/handoff.md` に書く
  （無ければ「無し」と、読んだ日時と `git log -1 --format=%h -- docs/handoff` を書く）。
  検証: `bash -c 'test -s openspec/changes/st25-day-timeline/review/handoff.md && grep -q "$(git log -1 --format=%h -- docs/handoff)" openspec/changes/st25-day-timeline/review/handoff.md'`

## 人間の確認待ち

**無し。** 画面の Scenario は Task 7 / 10 の e2e と、Task 8 / 9 の jsdom（指定と勘定）が担保し、サーバの Scenario は Rust の試験が担保する。
機械が再現できない物理（ロック・電池・本物の GPS・時間そのもの・実機）に当たるものが、この Story には無い。
確認バッチの手順書が Story ごとに 1 問聞く「触ってみて違和感は無かったか」だけが人間に渡る。
