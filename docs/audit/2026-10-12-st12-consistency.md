# 整合の確認（2026-10-12・ST12 の深掘り第 4〜6 回を FR-50 へ戻した改訂）

- 基準: `380febd3118cf6ee986b907f12b347b5a2c9c6f0`..`32bd8ec337a0e578ada98b0ec77320dba4feb5b3`

対象は ST12（`openspec/changes/st12-archive-ingestion/`）の深掘りの決定を FR-50 へ戻した改訂。
`git diff 380febd3..32bd8ec3 -- docs/requirements.md` の差分は FR-50 の 3 段（★ 2026-10-05。第 4 回 Q13「書庫から入る位置」・第 5 回 Q14「位置を持つマイアクティビティの項目」・第 6 回 Q15「位置を持つ YouTube の履歴の項目。ソースでなく項目で決める」）で、
3 段とも基準より後の commit（9a331a7a / e519a112 / c819c795）なので、Q15 の段だけでなく 3 段を見た。
文書とコードは head（`32bd8ec3`）の内容で読んだ。作業ツリーの未追跡ファイルは見ていない。

機械の入力:
- `req_impact.py --base 380febd3`: 改訂 1 件（FR-50）・扉 0 項。名指しは `docs/handoff/ST12.md` `ST22.md` `ST23.md` `ST33.md`・`docs/openapi.json`・`docs/production-prep.md`・`docs/stories/INDEX.md` `stories.json`・`docs/ui-direction.md`・走っている st08 / st12 / st17 / st21 / st25 の文書・正典 `browsing-views` `derived-records` `personal-entities` `record-deletion` `record-envelope`
- `check_chain.py`: 未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった: FR-50」（この監査が受けるもの）
- `check_scenarios.py . st12-archive-ingestion`: OK（Q15 の Scenario 4 本はすべて `crates/server/src/archive_flow_tests.rs:3668` `:3669` `:3722` `:3723` に印がある）

## C1. FR-88 は「種類をまたぐデータは写真の位置だけ」と置くが、FR-50 の改訂は検索・視聴の履歴の項目が座標を持つことを前提にしている
- 重大度: 中
- 文書: docs/requirements.md:729
- 要件: FR-50, FR-88, FR-102, PERM-6, PERM-15
- 根拠: FR-88 ★「種類をまたぐデータは、当面『写真の位置』1 つだけを別の種類にする（写真は承認・位置は不承認の衛星には、撮影地を抜いて渡す）」（:729-730）。表では「検索・視聴の履歴（YouTube の検索と視聴・マイアクティビティ・Takeout の Chrome の履歴）」を位置と別の種類に置いている（:720）。改訂後の FR-50 は「書庫のマイアクティビティの項目が位置（緯度経度）を持つ」（:571）・「位置を持つ書庫の項目は、マイアクティビティに限らず YouTube の視聴・検索の履歴も含めて」（:574）と、この種類の項目が座標を持ちうることを要件の前提にした。ST12 の design D22-b は「payload と内容の鍵は変えない（捨てない・原文から欄を外さない）」（`openspec/changes/st12-archive-ingestion/design.md:496-497`）ので、座標は `raw` の `locationInfos` に残る。FR-50 が手当てしたのは**消した時間帯**だけで、消していない時間の座標は「検索・視聴の履歴」の種類として出る。位置を承認せず検索・視聴の履歴を承認した衛星には、位置が原文ごと渡る（FR-88 が写真で避けた型そのもの）。初期の承認の表（外部 AI は検索・視聴が未決、ローカル AI は位置も承認）では起きないので、いま外へ出ているものは無い。`docs/handoff/ST38.md` `ST45.md` に位置の扱いは書かれていない（`grep -n 位置` 0 件）
- 直すところ: requirements（FR-88 の★に、書庫の検索・視聴の項目が座標を持ちうることと、その扱いを未決として置く）と、凍結した Story への申し送り（ST38 / ST45。読み出しの門を作る Story の深掘りで問う）
- 行き先: handoff ST38（承認の表を決める深掘りで、検索・視聴の項目の座標の扱いを問う。FR-88 は決まるまで未決のまま）
- 機械: できない（写真の位置と同じく抜くか、種類を割るかは人間が決める。決まった後は ST45 の読み出しの試験で `locationInfos` を持つ項目を見れば捕まえられる）

## C2. 正典 `record-deletion` の「位置の記録」は ST22 の範囲のままで、FR-50 が広げた「その時間の位置の記録」と同じ語で別の範囲を指す
- 重大度: 中
- 文書: openspec/specs/record-deletion/spec.md:79
- 要件: FR-50, FR-31, FR-51
- 根拠: 正典「WHEN 位置の記録に削除済みの印が付いた THE SYSTEM SHALL その記録の日（Asia/Tokyo）ごとに 1 回、その日の滞在を作り直す」（:79）、「滞在を消すと、その時間の位置の記録も削除済みになる」（:53）、「消す操作を押したとき、一緒に消える位置の記録の件数を含む確認」（:264）。改訂後の FR-50 では「その時間の位置の記録」に書庫の位置 7 本と、位置を持つマイアクティビティ・YouTube（・Chrome。design D22-d）の項目が入る（:567-576）。ST12 はこれを `external-ingestion` の ADDED だけに書き（`openspec/changes/st12-archive-ingestion/specs/external-ingestion/spec.md:312-321`）、`record-deletion` を MODIFIED しない（change の specs は `external-ingestion` と `collection-coverage` だけ）。広い読みで正典 :79 を当てると、書庫の印付けのたびに作り直しが要るが、ST12 は「滞在の作り直しを呼ばない・待たない」（design.md:468、`stay_store.rs:755-757`）。狭い読みなら :53 / :264 が書庫の位置を落とす。ST12 の archive 後、正典の `record-deletion` だけを読む Story（ST23 の物理削除・ST33 の書き出し・ST14 / ST15 の稼働状況）は、どちらの範囲かを文面から決められない
- 直すところ: その文書（ST12 の change に `record-deletion` の MODIFIED を足し、「位置の記録」の範囲を書庫の位置と位置を持つ項目まで書く。作り直しの要件は基準のソースに限ると書く）か、新しい Story（ST12 の merge 後の `fix/`）
- 行き先: fix（ST12 の merge 後の `fix/` で、`record-deletion` の「位置の記録」の範囲と、作り直しは基準のソースに限ることを書く）
- 機械: できない（語の範囲の食い違いで、文字列の照合では拾えない）

## C3. ST12 は滞在の削除の連鎖（`deletion.rs`）を変えるが、INDEX の capability の表と「ST12 が満たす条項」の表に FR-50 / `record-deletion` が無い
- 重大度: 中
- 文書: docs/stories/INDEX.md:87
- 要件: FR-50
- 根拠: INDEX「`record-deletion` | 削除 | ST22, ST23」（:87）。「satisfies に無い条項を ST12 が満たす」の表（:205-212）は NFR-12 / NFR-3 / FR-78 / FR-35・FR-61 だけで FR-50 が無い。`stories.json` の ST12 は `satisfies` が FR-14 / FR-16 / FR-17 / FR-55、`breaks` は ST03 の門と ST02 / ST04 の稼働状況だけで、ST22 の滞在の削除（連鎖・戻す）が無い（:279-297）。一方 ST12 の spec は導出元に FR-50 を挙げ（spec.md:323）、コードは ST22 の連鎖そのものを広げている（`crates/server/src/deletion.rs:274-280`「基準のソース ∪ 書庫の位置の論理ソース」「位置を持つ項目…も連鎖の対象」）。admission は capability の表で重なりを見るので、`record-deletion` を持つ ST23 は ST12 と並べて走れ、同じ `deletion::erase` を 2 本が触りうる（CLAUDE.md の「ST02 と ST03 が登録簿を共有し、12 時間で 5 往復」と同じ型）。確認の手順書が `breaks` から回帰を選ぶなら、ST22 の「消すと位置も消える・戻すと戻る」が回帰の候補に上がらない
- 直すところ: その文書（INDEX の「ST12 が満たす条項」に FR-50 の 3 段を足す。`record-deletion` の行に ST12 を足すか、足さない理由を書く。`stories.json` の ST12 の `breaks` に ST22 の削除の連鎖を足して `make_story.py` で再生成）
- 行き先: handoff ST23（ST23 の上流は ST12 の merge の後に始める。INDEX と stories.json の表は fix で足す）
- 機械: 安く捕まえられる（`check_chain.py` で、change の spec の「導出元」に出る FR が、その Story の `satisfies` か INDEX の「満たす条項」の表に無ければ warn にする。ST12 の spec.md:323 の FR-50 が拾える）

## C4. ST23 への申し送りは消去する位置を `c01-location` として書いており、FR-50 が広げた位置（書庫の位置・位置を持つ項目）を ST23 に送った文書が無い
- 重大度: 中
- 文書: docs/handoff/ST23.md:22
- 要件: FR-50, FR-51, FR-52
- 根拠: `docs/handoff/ST23.md:22`「位置の記録（`c01-location`）の本文を消去する（FR-51）とき、その時間と重なる滞在…も消去の対象に含めるか」。:7「消した時間帯への後着位置を…`user:late` を付ける」も ST22 の範囲で書かれている。改訂後の FR-50 では、滞在を消すと書庫の位置 7 本と、位置を持つマイアクティビティ・YouTube・Chrome の項目にも `user:cascade` / `user:late` が付く（`deletion.rs:286-289`、`stay_store.rs:808-859`）。ST12 の deep.md の「後続へ送るもの」が ST23 に送ったのは写しの消去だけ（`openspec/changes/st12-archive-ingestion/deep.md` 「ST23（本文を本当に消せる）へ —— 書庫の写しも消去の対象にするか」）で、ST12 の handoff への追記（`docs/handoff/ST23.md:88` R23）も写しの経路だけ。ST23 の上流がこの handoff から消去の範囲を組むと、本人が「この場面を本文ごと消す」と指示しても、書庫の位置と検索・視聴の項目の `raw` に同じ場面の座標が残る
- 直すところ: その文書（上流が始まっていない ST23 への申し送り `docs/handoff/ST23.md` に、FR-50 の「その時間の位置の記録」が書庫の位置と位置を持つ項目まで及ぶこと、台帳の原因（`cause_event_id`）で範囲を引けることを足す）
- 行き先: handoff ST23
- 機械: できない（ST23 の上流の深掘りが範囲を決める）

## C5. FR-50 は「位置（緯度経度）を持つか」で決めると書くが、spec・design・コードの判定は `locationInfos` の欄の有無だけで、13.1 の確かめ方も欄の名前しか見ない
- 重大度: 中
- 文書: openspec/changes/st12-archive-ingestion/specs/external-ingestion/spec.md:320
- 要件: FR-50
- 根拠: FR-50「書庫のマイアクティビティの項目が位置（緯度経度）を持つときも」（:571）・「どのソースかではなく、項目が位置を持つかで決める」（:574）。spec「項目のうち**原文に位置（`locationInfos`）を持つもの**」（spec.md:318 / :320）、design D22-b「欄の有無だけで判定し、中身は解析しない」（design.md:500）。コードは `"locationInfos"` の鍵だけを見る（`crates/server/src/stay_store.rs:694-713`、SQL も `raw::jsonb->'locationInfos'`、:729）。13.1 は「`field_names` に `locationInfos` があるか」を見る（tasks.md:259）が、`field_names` は項目の鍵の名前だけで値を出さない（`crates/server/src/archive/worker.rs:1159-1174`「値（題名・URL・検索語・座標・時刻）は出さない」）。座標が別の欄（たとえば地図の製品のマイアクティビティの `titleUrl` の URL）に入る形なら、FR-50 の文では対象、コードでは対象外で、13.1 でも見つからない。実物がそういう形かは確かめていない（下の「未確認」）
- 直すところ: その文書（spec の Requirement に「位置の判定は `locationInfos` の欄に限る」と範囲を明記し、FR-50 の「位置（緯度経度）を持つ」との差を design の反転条件に置く）か、requirements（FR-50 の括弧を「位置の欄（`locationInfos`）」に揃える）。どちらを採るかは改訂した側
- 行き先: requirements（本人の決定 2026-10-13: 「位置を持つ」は `locationInfos` の欄に限る。地図で見た場所は含めない。FR-50 の★に書いた）
- 機械: できない（実物の書庫の値が要る。`tools/archive-shape.sh` は値を出さない設計）

## K1. 滞在を消す確認の件数は `c01-location` だけを数え、FR-50 の改訂で一緒に消えるようになった書庫の位置と位置を持つ項目を数えない
- 重大度: 中
- 文書: openspec/specs/record-deletion/spec.md:264
- 要件: FR-50, NFR-19
- 根拠: 正典「消す操作を押したとき、一緒に消える位置の記録の件数を含む確認をその場に出し」（:264）、`browsing-views` の正典「件数は、消すときに何が一緒に消えるかを消す前に見るためのもの」（`openspec/specs/browsing-views/spec.md:237`）。画面は `web/src/DayView.tsx:311`「`m.eraseConfirm(counts.find((count) => count.logical_source === "c01-location")?.count ?? 0)`」で `c01-location` の件数だけを出す。消す側は書庫の位置 7 本と、位置を持つマイアクティビティ・YouTube・Chrome の項目まで印を付ける（`crates/server/src/deletion.rs:274-289`）。詳細の件数（`crates/server/src/lib.rs:1903-1918`）は `event_time BETWEEN start AND end` で数えるが、連鎖は区間のソース 4 本を「少しでも重なれば」で消す（`stay_store.rs:650-662`）ので、滞在の前から続く訪問・移動は件数に出ずに消える。また詳細の件数は YouTube の項目を位置の有無で分けないので、どれが一緒に消えるかを画面から読めない。消す・戻すは台帳で戻せるので失われるものは無い
- 直すところ: コード（確認の件数を、消す側と同じ集合・同じ重なりの判定で数える口にする）。ST12 の merge 後の `fix/` か、ST22 の画面を触る Story へ
- 行き先: fix（ST12 の merge 後の `fix/` で、確認の件数を消す側と同じ集合・同じ重なりで数える）
- 機械: 安く捕まえられる（`record-deletion` の Scenario `確認の文面に一緒に消える位置の件数が出る` の試験に、書庫の位置と位置を持つ YouTube の項目を混ぜた材料を足し、`deletion::erase` が返す `locations` と確認の数が一致することを見る）

## C6. ST12 の design は spec の Requirement を 14 と数えるが、spec には 15 ある
- 重大度: 低
- 文書: openspec/changes/st12-archive-ingestion/design.md:17
- 要件: —
- 根拠: design.md:17「specs の 14 の Requirement（第 4 回 Q13 で 1 つ足し…）」。`specs/external-ingestion/spec.md` の `### Requirement` は 15（:8〜:818）。commit `28e73b3e` の時点で spec 14・design 13 とずれており、Q13 で両方 1 つずつ増えた
- 直すところ: その文書（「15」。`collection-coverage` の MODIFIED を数えるなら、その旨）
- 行き先: docs（design.md の 14 を 15 に直した）
- 機械: 安く捕まえられる（`check_scenarios.py` が spec の見出しを数えるときに design の「specs の N の Requirement」と比べる）

## 未確認

- 本物の Takeout で、YouTube の視聴・検索とマイアクティビティの項目が `locationInfos` を持つか、座標を `locationInfos` 以外（`titleUrl` などの URL）に持つか（13.1 は未了。Q14 / Q15 の「事実」も検証者の知識と書いている）。C5 の重さはこれで決まる
- ST33（書き出し）が書庫の写し（design D9）を書き出しの対象に入れるか。入れるなら、消した場面の座標が写しの原文から外へ出る（`docs/handoff/ST33.md` の R2 は `core.event_live` を読むことだけを書き、写しに触れていない）
- `deletion::restore` が書庫の項目を戻したときに、その日の滞在の作り直しを呼ぶか（正典 `record-deletion` の「戻すと作り直す」との関係。restore の本体（`deletion.rs:99-215`）は読んだが、作り直しの呼び出しの条件までは追っていない）
- ST23 の上流が `docs/handoff/ST23.md` の ST12 の節（R23）と ST12 の deep.md の「後続へ送るもの」の両方を読むかどうか

## 触らなくてよいと判断したもの

- **走っている change への印**（差し戻しではない。どれも失われるもの無し）:
  - `st25-day-timeline` —— 消した時間の「ほかの記録」の行（`specs/browsing-views/spec.md:196`）は「位置以外の記録」を出すが、位置を持つ YouTube の項目は消した時間帯なら削除済みで `event_live` に出ないので、表示に食い違いは起きない。書庫の項目の要約は `title` と URL のホストだけで座標を出さない（design.md:72）
  - `st21-place-registry` / `st17-daily-feeling` / `st08-browser-history` —— FR-50 を名指しするのは各自の本人が書く記録・PC のブラウザ履歴の削除の印で、書庫の位置の範囲の改訂の影響を受けない（どれも `c03-` と書庫を名指ししていない）
- **`docs/production-prep.md:74` `:181` と `docs/openapi.json:326`（`/events`）** —— `core.event_live` 越しに読む話で、書庫の項目に付いた印も同じビューで効く
- **`docs/handoff/ST33.md` の R2** —— 書き出しは `core.event_live` を読むと送っており、書庫の位置・項目に付いた `user:cascade` / `user:late` も同じく除かれる（写しは上の「未確認」）
- **`docs/handoff/ST22.md` の FR-50 の名指し** —— ST22 は archive 済み（`openspec/changes/archive/2026-10-04-st22-record-deletion`）。書庫の範囲は ST12 側の連鎖の拡張で受けており、ST22 への申し送りは要らない
- **ST12 の tasks 16.2 の `myactivity_located_sql`**（tasks.md:246） —— コードは `item_located_sql` に改名済み（`stay_store.rs:723`）。`[x]` の付いた凍結した文面で、design D22-d とコードは一致している
- **ST12 の deep.md の「後続へ送るもの」の ST24 宛て** —— 宛先の前提（感度）が消えた件は、2026-10-11 の整合の確認の C11 で `docs/handoff/ST38.md` に移っている。今回の FR-50 の改訂とは別
- **格納の commit と印付けの間に、印の無い行が生きた記録として見える時間がある**（`archive/worker.rs:1553` `:2206`） —— FR-50 の ★ 2026-09-29（ST22 第 3 回 Q6）が「取り込みの直後に照合して付ける」を決めており、ST12 の design D22 (1) / D22-a もそれに従っている。要件との食い違いではない
