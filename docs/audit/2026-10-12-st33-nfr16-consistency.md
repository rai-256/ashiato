# 整合の確認 — NFR-16 ★ 2026-10-12 追加（ST33 の深掘り第 2 回 Q6〜Q8）

- 基準: `f8fcd356762fd5c017949443994f35f2fd0a468c`..`e44c267f6b6544bfcfd4f6754d4f8c7a334e6999`

> 基準の head は改訂を入れた commit `e44c267f`（監査を書いた時点では未 commit だったので `git stash create` の object を置き、commit 後に差し替えた）。

機械の入力（写し）:

- `python3 scripts/req_impact.py . --base f8fcd356…` —— 改訂した要件 1 件（NFR-16）・扉 0 項。名指ししている箇所:
  `docs/handoff/ST33.md:8` / `docs/stories/INDEX.md:55` / `docs/stories/stories.json:744` / `docs/ui-direction.md:23` /
  `openspec/changes/st33-data-export/{deep-questions.json,deep.md,design.md:255,proposal.md:5,53,review/deep.md,review/spec.md,specs/data-durability/spec.md}` /
  `openspec/specs/record-deletion/spec.md:60`
- `python3 scripts/check_chain.py .`（この監査を置く前）—— FAIL 1（整合の確認が無い）・warn 2（ST14 / ST24 の判断の日。今回の改訂とは別の要件）。
  warn 10 は「まだ change の無い Story」だけを見るので、change を持つ ST33 は対象外

走っている change（archive 以外）: st08-browser-history / st12-archive-ingestion / st21-place-registry / st25-day-timeline / st33-data-export（改訂した側）。
印を付けたのは ST08（C2）だけ。失われるもの（uncaptured / discarded / exported / rewrite-all）を ST08 の側で起こすものは無い（C2 は ST33 の design で閉じられる）。

## C1. 第 2 回の貼り戻し（deep-answers-2.txt）が、第 1 回の Q3・Q4 を記録と違う答えで返している
- 重大度: 高
- 文書: openspec/changes/st33-data-export/deep-answers-2.txt:4
- 要件: NFR-16
- 根拠: `deep-answers-2.txt:12`「Q4 [前に書き出したファイルをどうするか] -> 新しい書き出しで置き換える（1 つだけ置く）」/ `deep.md:118`「**本人の答え**: **毎回別のフォルダに残す**（本人が選んだ。**推奨を覆した**」。
  `deep-answers-2.txt:7`「押してから終わるまで: 確認をはさんでから、その場で進み具合」/ `specs/data-durability/spec.md:407`「確認をはさまずに書き出しを始め」。
  `deep-answers-2.txt:8`「「この書き出しの後に消した記録が N 件ある」と出す」/ `deep.md:103`「書き出した後に記録を消したとき: 知らせない。」・`spec.md:357` `:410`・design D11「書き出した後に消した記録の件数は数えない（Q3「知らせない」）」。
  `deep-answers-2.txt:6`「件数・期間 ＋「入らないもの」」/ `spec.md:406`（件数と期間だけ）。`deep-answers-2.txt:11`「proto の初期値から動かした軸: 軸 1・軸 2・軸 3。」/ `deep-answers-1.txt`「動かした軸: なし」。
  `deep.md:203` は「答えは 3 問とも推奨（除く側）だった（2026-10-12、`deep-answers-2.txt`）。spec と design の形を変える必要は無い」とだけ書き、同じファイルの Q1〜Q5 の段が違うことに触れていない
- 直すところ: その文書（本人に、第 2 回の貼り戻しの Q3・Q4 が意図した変更かを問う。変更なら deep.md の Q3・Q4、spec の Requirement「全記録を自宅 PC の決まったフォルダに書き出せる」「書き出す前の件数と、進み具合と、前回の書き出しを読み出せる」「設定画面に全記録の書き出しの節がある」と Scenario 4 本以上、design D3 / D11 / D13 / D16、tasks 5.2 / 6.3 / 9.2 / 9.4 が動く）
- 機械: 安く捕まえられる（record が貼り戻しの中の既決の Q を `deep.md` の「本人の答え」と突き合わせ、違えば止める。今回の検査には無い）
- 行き先: ask

## C2. 「時間に入る」を幅で読むと決めたのに、design D5 は幅を終わりの時刻の項目でしか読めず、長さで持つ幅・次の記録までの幅を点として出す
- 重大度: 高
- 文書: openspec/changes/st33-data-export/design.md:117
- 要件: NFR-16
- 根拠: `docs/requirements.md` NFR-16 ★ 2026-10-12「「時間に入る」は幅で読む（始まりが前でも、消した時間と重なれば除く）」。
  `design.md:117`「終わりは内容の `end` / `end_time` / `endTime` / `endTimestamp` / `end_at` のうち最初に時刻として読めるもの（`core.try_timestamptz`）、どれも無ければ `event_time`（点）」。
  (a) 走っている ST08 の訪問は幅を長さで持つ —— `openspec/changes/st08-browser-history/design.md:110`「`visit_duration_us` | `visit` | Chromium のみ」（時刻ではないので `END_KEYS` に足しても `try_timestamptz` で NULL になり点に落ちる）。
  `c02-browser-history` は D5a(1) の「緯度経度を持たない」一覧にあり、除かれるのは (3) の地図の URL に当たったときだけなので、消した時間の前に開いて中まで見ていた地図の訪問が出る。
  (b) `c02-window` の前景の記録は終わりを持たない（`crates/collector-windows/src/contract.rs:127`「範囲の終わり。`idle`（`leave` 側）と `powered-off` だけが持つ」）。前景は次の変化まで続くが、`title` / `url`（同 `:117` `:121`）に地図の座標を持つ記録が消した時間の前に始まれば出る。
  design 自身の Risks（`design.md:294`）は「足し忘れた」ソースだけを挙げ、D5 の形では表せない幅（長さ・次の記録まで）を挙げていない
- 直すところ: その文書（design D5 に長さの項目（`visit_duration_us` など）と「次の記録まで」の読み方を足すか、読めない幅を持つソースを「幅が分からない」として既定で厳しい側に倒す。spec「書き出しは本人が消した場面を出さない」の幅の文の例示にも足す）。ST08 は走っている change なので印だけ（ST08 の tasks は変えない）
- 機械: 安く捕まえられる（D8 の分類の検査と同じ形で、登録簿のソースごとに「幅の読み方」の分類を持たせ、無いソースで落とす）
- 行き先: docs

## C3. Q6〜Q8 が本人の決定であることが、proposal・design・tasks・spec の導出元のどこにも無い
- 重大度: 中
- 文書: openspec/changes/st33-data-export/tasks.md:34
- 要件: NFR-16
- 根拠: `tasks.md:34`「**Q1〜Q5 は本人の決定。下流は変えない**」/ `tasks.md:3`「本人が決めた 5 件」/ `design.md:5`「本人が決めた 5 件（Q1〜Q5）」/ `proposal.md:17`「深掘りは **1 巡・5 問**」・`proposal.md:53`「要件への戻し: **NFR-16 ★ 2026-09-29 訂正**」（★ 2026-10-12 が無い）。
  spec の導出元（`spec.md:149`）は「NFR-16（★ 2026-09-29 訂正）。深掘り Q5 … / C1 / C18」で、幅・利用者・URL の座標の根拠は `spec.md:152` `:153`・`design.md:110` `:129` `:145` とも spec-review の R1 / R2 / R4 を挙げ、Q6〜Q8 を挙げない。
  `rg 'Q6|Q7|Q8|2026-10-12'` は 4 ファイルとも 0 件。`scripts/task_context.py` は Task の本文が名指しした `Q<n>` だけを写すので、Task 3 の implementer と reviewer の写しに Q6〜Q8 は入らない。
  一方 `tasks.md:33` は D5a を「（仮）決め」と並べており、(2)(3) を仮の判断として狭めてよいと読める
- 直すところ: その文書（tasks の Global Constraints と読む順・design の Context・proposal の深掘りの表と要件への戻し・spec の導出元に Q6〜Q8 と ★ 2026-10-12 を足し、Task 3.1 / 5.4 の本文で Q6〜Q8 を名指しする）
- 機械: 安く捕まえられる（`deep.md` の「本人の答え」がある `Q<n>` が tasks の Global Constraints か名指しに 1 度も現れなければ warn）
- 行き先: docs

## C4. 座標の並びの形（D5a (3)）を「B（仮）」と分類しているが、design 自身が読み落としは戻らないと書いている
- 重大度: 中
- 文書: openspec/changes/st33-data-export/deep.md:203
- 要件: NFR-16
- 根拠: `deep.md:203`「D5a の「(3) の並びの形は仮」は仮のまま（Q8 は URL や題名の中も数えると決めただけで、並びの形は B）」/
  `design.md:146`「読み落とし（新しい座標の書き方）は戻らないので、見つけたら (3) に形を足す」/ `design.md:148` の反転条件は「形を狭める」向きも持つ。
  CLAUDE.md の B は「計算し直せば戻る」で、読み落としの側は `loss: exported`（NFR-16 ★ 2026-10-12「URL や題名の中に書かれた座標も緯度経度に数える」の読み落としそのもの）。実際に読み落とす形は K2
- 直すところ: その文書（(3) の形を「扉を開けたままにする既定」（既定は厳しい側。URL のデコード後に見る・小数の桁を緩める）で決めて C に置き、反転条件から「狭める」を外すか、狭める変更を本人に返す A として書く）
- 機械: できない（B と loss の対応を文から判定する検査は無い）
- 行き先: docs

## C5. 改訂の中身（幅・利用者・URL の座標）が ★ の段落にだけあり、ST33.md の逐語の引用で切れる
- 重大度: 低
- 文書: docs/stories/ST33.md:24
- 要件: NFR-16
- 根拠: `docs/stories/ST33.md:24`「> …（続きは requirements.md）」（`scripts/make_story.py:19` の `quote(lines, limit=6)` が 6 行で切る）。NFR-16 の本文（`docs/requirements.md:898-900`）は「本人が消した滞在の時間に入る緯度経度を持つ行」のままで、★ 2026-10-12 の 3 つの読み方は本文に無い。
  Story ブリーフ（`docs/briefs/ST33.html`）は ST33.md から引くので、同じく見えない
- 直すところ: leave
- 機械: できない
- 行き先: leave: 読み方を ★ の補足に置くのはこの要件書の慣行（NFR-15 の ★ 2026-09-29 補足と同じ）で、ST33 は上流の spec・design がすでに本文として持ち、下流は ST33.md を読まない。ブリーフは深掘りの前に読むもので、深掘りは済んでいる

## K1. design D5a (2) の jsonpath の式は、スカラーを持つ内容（実質すべての行）でエラーになる
- 重大度: 中
- 文書: openspec/changes/st33-data-export/design.md:140
- 要件: NFR-16
- 根拠: `design.md:140`「`jsonb_path_exists(payload, 'strict $.**.keyvalue() ? (@.key like_regex "^(lat|lon|lng|latitude|longitude)(e7)?$" flag "i")')` の形で、**どの深さでも**見る」。
  PostgreSQL 17.11（`ashiato2-integ-db-1`）で `{"a":1,"b":{"Latitude":2}}` に当てると `ERROR: jsonpath item method .keyvalue() can only be applied to an object`（`lax` でも同じ）。
  第 4 引数 `silent => true` で黙らせると NULL が返り、緯度経度を持つ行を「持たない」と読む（D5a(1) の一覧のソースで漏れる）。
  `strict $.** ? (@.type() == "object").keyvalue() ? (…)` なら `{"a":1,"b":{"LatitudeE7":2}}` で t・`{"a":1,"b":[{"x":1}]}` で f を返す
- 直すところ: その文書（design D5a (2) の式を、オブジェクトに絞ってから `keyvalue()` を当てる形に直し、silent で黙らせないことを書く）
- 機械: 捕まえられる（tasks 3.1 の `CT export_excluded_sql 11` と 5.4 が DB で当てれば落ちる。ただし silent で黙らせた実装は「緯度経度を持たないソースでも、内容に緯度経度があれば…出ない」の Scenario でしか捕まらない）
- 行き先: docs

## K2. design D5a (3) の正規表現は、design が例に挙げた `ll=35.68,139.76`・`q=35.68,139.76` に当たらない
- 重大度: 中
- 文書: openspec/changes/st33-data-export/design.md:142
- 要件: NFR-16
- 根拠: `design.md:142`「`raw ~ '-?\d{1,2}\.\d{3,}\s*,\s*-?\d{1,3}\.\d{3,}'`（`@35.6812,139.7671`・`ll=35.68,139.76`・`q=35.68,139.76`）」。
  PostgreSQL 17.11 で `{"url":"https://maps.google.com/?ll=35.68,139.76"}` は f（小数 2 桁は `\d{3,}` に当たらない）、
  `{"url":"https://www.google.com/maps?q=35.68%2C139.76"}` も f（URL の中で符号化された `,`）。`@35.6812,139.7671` と `!3d35.6812!4d139.7671` は t。
  NFR-16 ★ 2026-10-12 は「URL や題名の中に書かれた座標も緯度経度に数える」で、桁や符号化で絞っていない
- 直すところ: その文書（design D5a (3) の例と式を揃え、符号化した区切りと短い小数の扱いを決める。C4 と同じ直し）
- 機械: 安く捕まえられる（spec の Scenario「原文に地図の座標を持つウィンドウの記録は、消した時間の中では出ない」に design の例の形を足せば tasks 5.4 が落とす）
- 行き先: docs

## 未確認

- `s01-attribute`（D5a(1) の一覧にある本人が書いた記録）の原文に、本人が座標を文字で書いた場合に (3) が当たるか（本人の入力の形を確かめていない）
- ST08 の訪問の `url` / `title` が実装後に原文（`raw`）へ入ること（ST08 の design D4 の記述だけを見た。コードはまだ無い）
- ST12 の `c03-*` の原文・内容のうち、`end_time` 以外の形で幅を持つもの（design の表 `st12-archive-ingestion/design.md:157-163` だけを見た）
- 消した滞在の前の版が今の版より広い時間を持つとき、その広い時間を「本人が消した滞在の時間」に数えるか（`core.stay_erased` は今の版の `start` / `end` だけを持つ。`migrations/202609271716_deletion_ledger.sql` のビュー）。NFR-16 の文面からは決まらない

## 触らなくてよいと判断したもの（理由つき）

- `openspec/specs/record-deletion/spec.md:60`（ST22 の正典）—— 「書き出し（NFR-16）ができたときに外へ出る」を理由に挙げるだけで、Q7 の「利用者を問わず」と食い違わない。ST22 の連鎖と遅着の印は同じ利用者の中で付くが、ST33 は印でなく自分の述語で除く（design D5 / D12）。`stay_store` の `mark_late_arrivals`（`crates/server/src/stay_store.rs:626-`）は design D6 の記述（非公開・`event_time < 上端`）どおり
- `design.md:109` の「本人が消した滞在」の読み方 —— `stay_store::Mark::of`（`crates/server/src/stay_store.rs:258-266`）と一致する（`NULL` も本人の削除）
- `docs/handoff/ST33.md` —— 宛先（ST33）は残っており、R2 の目的（消した場面の緯度経度を外へ出さない）は改訂で広がる側で、申し送りの文面と食い違わない
- `docs/stories/INDEX.md:55` `:222-224`・`docs/ui-direction.md:23`・`docs/stories/stories.json:744` —— 表題・価値・完了の判定・requires・S-7 の対応は改訂の前提を固定していない（「全記録」は NFR-16 の本文で読む、と deep.md が書いたとおり）
- `docs/production-prep.md` —— 書き出しも NFR-16 も名指ししていない（A-2 のログの規律は design D15 が受けている）
- spec の Scenario の本数（70 本。tasks の「サーバ 57・画面 13」と「除く範囲 11 + 遅着 1」）—— spec と一致。`check_scenarios.py` の担保なし 70 件は下流が未着手のため
- st12-archive-ingestion / st21-place-registry / st25-day-timeline —— 改訂の 3 つの読み方に接する持ち物が無い（ST12 の訪問・移動は `end_time` を持ち D5 の `END_KEYS` にある。ST21 の場所・ST25 の 1 日の画面は書き出しの範囲の判定に関わらない）
- `GET /events` が消した時間の中の印の無い緯度経度つきの行を返すこと —— NFR-16 は書き出しの条項で、改訂は読み出しに触れない（今回の改訂の前からある形）
- check_chain の warn（ST14 / ST24）—— 今回の改訂（NFR-16）とは別の要件の訂正
