# 整合の確認（2026-10-13・ST33 の NFR-16 の改訂を、main（ST12 の archive・ST47）を取り込んだ木で）

- 基準: `32bd8ec337a0e578ada98b0ec77320dba4feb5b3`..`a7a8c70100f778af5a25bbb56a1ae5b1245140bf`
（基準の head は、main を取り込んだ merge commit `02c20914` を graph の prepare が rebase で一直線にしたときに同じ木になった commit。`02c20914` は rebase で origin から消え、CI が読めなかった）

対象は `git diff 32bd8ec3..02c20914 -- docs/requirements.md` の 2 項目。
ST33 の深掘りが戻した NFR-16 の改訂（★ 2026-09-29 訂正・★ 2026-10-12 の Q6〜Q8・★ 2026-10-12 の Q9「合言葉で暗号化した 1 つのファイル」）と、
main 側の FR-50 の ★ 2026-10-13（「位置を持つ」は `locationInfos` の欄に限る。地図で見た場所の URL の座標は含めない）。
重点は、ST33 の文書（`openspec/changes/st33-data-export/`・`docs/handoff/ST33.md`）が、同じ時期に main に入った
ST12 の archive（正典 `external-ingestion` / `collection-coverage`、FR-50 の ★ 2026-10-05 の 3 段と ★ 2026-10-13）と ST47 の追加を受けているか。
文書とコードは head（`02c20914`）の内容で読んだ。作業ツリーの未追跡ファイルは見ていない。
ST33 の 2 本の監査（`2026-10-12-st33-nfr16-consistency.md` / `2026-10-12-st33-consistency.md`）で処置済みの指摘は繰り返さない。

機械の入力:
- `req_impact.py --base 32bd8ec3`: 改訂 2 件（FR-50 / NFR-16）・扉 0 項。FR-50 の名指しは handoff ST12 / ST22 / ST23 / ST33・`docs/openapi.json`・`docs/production-prep.md`・INDEX / stories.json・ui-direction・走っている st08 / st17 / st21 / st25 / st33・正典 `browsing-views` `derived-records` `external-ingestion` `personal-entities` `record-deletion` `record-envelope`。NFR-16 の名指しは handoff ST33 / ST41・INDEX:56・stories.json:775・ui-direction:23・st33 の各文書・正典 `record-deletion`:60
- `check_chain.py`: 未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった（この監査を名指ししない改訂）: NFR-16」（この監査が受けるもの）

走っている change（archive 以外）: st08-browser-history / st17-daily-feeling / st21-place-registry / st25-day-timeline / st33-data-export（改訂した側）。
st33 以外に印を付けるものは無い（下の「触らなくてよい」）。

## C1. NFR-16 に戻した Q9（合言葉で暗号化した 1 つのファイル）を、ST33 の spec・design・tasks・申し送りが受けておらず、平文の JSON Lines をフォルダに置く形のまま
- 重大度: 高
- 文書: openspec/changes/st33-data-export/design.md:37
- 要件: NFR-16, PERM-10, PERM-11, PERM-15
- 根拠: NFR-16「**書き出しは、押すときに本人が入れる合言葉で暗号化した 1 つのファイルにする（合言葉は保存しない）。**」（docs/requirements.md:1142）。deep.md Q9 の効く先「spec『全記録を自宅 PC の決まったフォルダに書き出せる』と画面の Scenario（合言葉の入力）・design D2 / D3 / D13・tasks」（deep.md:312）。一方、design の Non-Goals「ダウンロードの口・**暗号化して受け取る口は作らない**」（design.md:37）、D7「表ごとの JSON Lines」と 10 本のファイル（design.md:206-219）、D3 / D16 はフォルダ単位（design.md:76-83, :341-354）。spec「書き出し 1 回ごとに新しく作る、書き出した日時を名前に持つフォルダへ置く」（spec.md:12）、「各ファイルを…JSON Lines、UTF-8 で書く」（spec.md:99）、Scenario「置き場の下に…フォルダが 1 つだけあり、その中に表ごとのファイルと説明書がある」（spec.md:33）。tasks に「暗号」「合言葉」は 0 件（`grep -n '暗号\|合言葉' tasks.md` は何も返さない）。deep.md の状態行は「第 3 回（…Q9）を本人に問うている（2026-10-12。**未回答**）」（deep.md:3）のままで、Q9 の答えは :309 にある。申し送りは「ST33 の仕様は本人が合意済み（`agreement.json`）なので spec / design / tasks は変えず」（docs/handoff/ST33.md:41）と書き、`agreement.json` は Q9 より前の head `e52dcf1d`・`deep-answers-4.txt` で accepted。下流がこの tasks から作ると、同じ PC の衛星（PERM-10）が平文を読めるフォルダができる —— Q9 が止めた `loss: exported` そのもの
- 直すところ: その文書（spec の置き場の Requirement・形式の Requirement・画面の Scenario（合言葉の入力）、design D2 / D3 / D7 / D13 / D16 と Non-Goals、tasks を Q9 の形に直し、仕様への合意を取り直す。deep.md:3 の状態行と handoff ST33:41 の「変えず」も直す）
- 行き先: docs（ST33 の spec の段をやり直して spec・design・tasks に写し、合意を取り直す。本人の決定 2026-10-13: #72 は merge せず spec の段から）
- 機械: 安く捕まえられる（`check_chain.py` か spec の段の検査で、deep.md の「効く先」に挙がった artifact が、その Q の答えの記録より古い commit のままなら warn。agreement.json の head が deep.md の最後の答えより前なら warn）

## C2. ST33 の spec・design・proposal・deep.md は「書庫の位置には削除の印が付かない」を前提に Scenario を組んでいるが、main の ST12（FR-50 ★ 2026-10-05）は書庫の位置にも連鎖で印を付ける。spec の「後に消した記録の件数」の Scenario は 1 件多く数える
- 重大度: 高
- 文書: openspec/changes/st33-data-export/specs/data-durability/spec.md:460
- 要件: NFR-16, FR-50
- 根拠: ST33 spec「その時間に、同じ書き出しに入っていた**書庫の位置（印が付かない別のソース）**が 1 件ある」→「THEN 後に消した記録の件数は、消した滞在と連鎖で印が付いた記録の数に **1 を足した数**である」（spec.md:458-461）。同じ前提が spec.md:179「削除の印を付けるのは滞在と基準のソースだけで、書庫の位置・写真の位置…」、:191-194「印の無い別のソースの位置（書庫の位置）」、proposal.md:97「書庫の位置は別のソースで入り、ST22 の印が付かない（ST12 への申し送り R6）」、deep.md:23 にある。ST33 の spec を書いた基準（`929aede4`）には FR-50 の ★ 2026-10-05（9a331a7a / e519a112 / c819c795）が無かった。main の正典は「本人が滞在を消すとき、その時間帯と重なる書庫の位置の記録にも削除済みの印を付ける」（openspec/specs/external-ingestion/spec.md:316-318）で、コードは連鎖に書庫の位置 7 本を足している（`crates/server/src/deletion.rs:274` `stay_store::with_archive_sources`、`crates/server/src/archive/mod.rs:18-26`）。:458 の材料では書庫の位置が連鎖で印を受けて「連鎖で印が付いた記録」に入るので、THEN の「+1」は二重に数えた値になり、下流の試験は落ちるか、書庫の位置を連鎖に掛からない形で入れて曲げることになる。:191 も同じで、書庫の位置は D5 の時間の判定ではなく印で除かれ、Scenario が確かめたいはずの「印の無い行を時間と緯度経度で除く」経路を通らない。main で「印の無い・緯度経度を持つ・消した時間の中」の行になるのは、写真の位置（`c01-photo`）・生きている滞在の前の版・URL にだけ座標を持つマイアクティビティと Chrome の項目（FR-50 ★ 2026-10-13）・ウィンドウとブラウザの履歴
- 直すところ: その文書（ST33 の spec :191 / :458 の材料を、印の付かない行（写真の位置か、URL にだけ座標を持つ書庫の項目）に差し替え、:179・proposal:97・deep.md:23 の前提を ST12 の連鎖を踏まえた文に直す）。合意済みの spec を直さないなら、凍結した Story への申し送り（`docs/handoff/ST33.md` に、試験の材料を差し替えることを書く）
- 行き先: docs（ST33 の spec の段をやり直して spec・design・tasks に写し、合意を取り直す。本人の決定 2026-10-13: #72 は merge せず spec の段から）
- 機械: 安く捕まえられる（下流の試験で捕まる —— 材料を `deletion::erase` の連鎖に通せば :458 の THEN が落ちる。上流の段では、spec の Scenario が「印が付かない」と名指しする論理ソースを、`deletion.rs` の連鎖の並び（`with_archive_sources`）と突き合わせる検査は作れるが、文から論理ソースを引くのは粗い）

## C3. NFR-16（Q8）は URL や題名の中の座標を「緯度経度」に数えて書き出しから除くが、FR-50 の ★ 2026-10-13 は同じ座標を「そこにいた記録ではない」として消した場面に含めない。「本人が消した場面」が 2 つの要件で別の広さになっている
- 重大度: 中
- 文書: docs/requirements.md:577
- 要件: FR-50, NFR-16
- 根拠: FR-50「**ここで『位置を持つ』は、その時に端末がいた位置（原文の `locationInfos` の欄）を持つことを指す。** 地図で見た場所（URL の中の座標）は『そこにいた』記録ではないので含めない」（:577-580。マップのマイアクティビティ 10,495 件・Chrome の履歴 3,849 件・マイアクティビティの Chrome 359 件が URL にだけ座標を持つ）。NFR-16「D-01 の全記録（**本人が消した場面を除く**）…URL や題名の中に書かれた座標も緯度経度に数える。…座標の名前の項目だけで読むと、同じ場面の緯度経度が外へ出て戻らない」（:1133, :1140-1142。★ 2026-10-12、ST33 第 2 回 Q8）。ST33 の design D5a (3) は URL の座標の並びを拾い（design.md:176-191）、書庫の `c03-*` は `NO_COORD_SOURCES` に無いので丸ごと「緯度経度を持つ」側（design.md:162-163）。結果、消した滞在の時間に地図で見た場所の項目は、FR-50 では生きた記録として読み出し・画面・（ST45 の後の）衛星に出て、NFR-16 では書き出しからだけ除かれる。Q8 の理由（同じ場面の緯度経度）は FR-50 ★ 2026-10-13 の理由（そこにいた記録ではない）と逆で、Q8 は 10-12、★ 2026-10-13 はその翌日の決定。書き出しの側が厳しいのは「除き過ぎは戻る」で失われるものは無いが、どちらの理由が本人のいまの考えかは文書から読めない。Q8 を狭める向きに直すと外へ出るものが増える（`loss: exported`）
- 直すところ: requirements（NFR-16 か FR-50 の ★ に、書き出しは FR-50 より広く除くことを意図して残すと書く。狭めるなら ST33 の深掘りで本人に `premise` として問う）
- 行き先: leave: 書き出し（NFR-16 の Q8）は除く側に広い。外へ出るものは増えず、除き過ぎは書き出し直せば戻る。Q8 を FR-50 の狭さに揃えると外へ出るものが増える（loss: exported）ので、揃えるなら本人に問う
- 機械: できない（2 つの要件の語の範囲の食い違いで、文字列の照合では拾えない）

## C4. ST47 は YouTube のマイアクティビティを「視聴・検索・その他」に分けるが、FR-50 の連鎖が見る項目のソースは固定の並びで、新しい論理ソースが並びに入ったことを確かめる試験が無い
- 重大度: 中
- 文書: docs/stories/ST47.md:52
- 要件: FR-50, FR-14
- 根拠: ST47「FR-50 の滞在の削除の連鎖（位置を持つ YouTube の項目に印が付くこと）」（ST47.md:52）、「それ以外の項目は視聴にも検索にも入らない」（:57）。正典「**位置の 7 本以外で書庫が項目を入れる論理ソースはすべて同じ判定に掛ける**」（openspec/specs/external-ingestion/spec.md:322）。コードの判定の範囲は `ITEM_SOURCES` の固定 3 本（`c03-youtube-watch` / `c03-youtube-search` / `c03-chrome-history`。`crates/server/src/archive/mod.rs:32-36`）と `c03-myactivity-` の前置き（`crates/server/src/stay_store.rs:664-683`）。`archive/mod.rs` の試験は `LOCATION_SOURCES` の 7 本と `INTERVAL_SOURCES` の包含だけを固定し（:62-80）、書庫が出しうる論理ソースが位置の 7 本・`ITEM_SOURCES`・前置きのどれかに入ることは見ない。ST47 が「その他」を `c03-youtube-` の新しい論理ソースに入れると、`locationInfos` を持つ項目でも消した時間に印が付かず、生きた記録として読み出しに出る（書き出しは ST33 の既定で除かれる）。INDEX で ST47 の capability は `external-ingestion` だけで（INDEX.md:83）、完了の判定が触る稼働状況の格子（ST47.md:58）の `collection-coverage` にも名が無い
- 直すところ: 新しい Story への申し送り（`docs/handoff/ST47.md` を作り、新しい論理ソースを `ITEM_SOURCES` に足すことと、書庫の解析器が出しうる論理ソースがどれかの並びに入ることを試験で固定することを書く。INDEX の `collection-coverage` に ST47 を足すかは、格子が登録簿から自動で並ぶなら不要）
- 行き先: handoff ST47（「その他」を新しい論理ソースに入れるなら、FR-50 の連鎖の並びに入ることを試験で固定する）
- 機械: 安く捕まえられる（`archive` の試験で、解析器の各 `Item.source` が `LOCATION_SOURCES` ∪ `ITEM_SOURCES` ∪ `c03-myactivity-*` ∪ 位置を持たないと明示した並びのどれかに入ることを見る）

## K1. ST33 の design は ST12 を「まだ main に無いもの」とし、`c03-*` の幅を「`end_time` で (a)、点は ST12 が (d) に足すまで (c)」と書くが、ST12 は archive 済みで (d) に足す係が無く、移行前の `end_time` はミリ秒の数字で (a) では読めない
- 重大度: 中
- 文書: openspec/changes/st33-data-export/design.md:157
- 要件: NFR-16
- 根拠: design「**まだ main に無いもの**: ST12（`c03-*`・`core.archive_*`）」（:24）、「ST12 の `c03-*` は `end_time` で (a)、持たないもの（`c03-timeline-route` の点など）は ST12 が (d) に足すまで (c)」（:157）、(a) は「内容の鍵のうち最初に `core.try_timestamptz` で読めるもの」（:150）。ST12 は `openspec/changes/archive/2026-10-13-st12-archive-ingestion/` に移り、`SPAN_RULES` は ST33 が作る定数なので ST12 の側から足す経路が無い。ST12 のコードは点と区間を既に決めている（`crates/server/src/archive/mod.rs:41-46` の `INTERVAL_SOURCES` 4 本、それ以外は始まりの時刻 1 点。`stay_store.rs:650-662`）。移行前の訪問・移動の `end_time` は `endTimestampMs`（`worker.rs:656-660`）のミリ秒の数字で、ST12 は `core.try_timestamptz` とは別に `'^[0-9]{1,15}$'` を `to_timestamp(…/1000)` で読む（`stay_store.rs:633-641`。試験の注記「終わりをミリ秒として読まないと終わりが始まりに倒れ」`archive_flow_tests.rs:3243-3245`）。`core.try_timestamptz` は `value::timestamptz` が例外なら NULL を返す（`migrations/202609271716_deletion_ledger.sql:28-34`）ので、design の (a) は移行前の 2 本を (c)「次の記録まで」に落とす。ほとんどは除き過ぎだが、同じソースの次の記録が本当の終わりより前に始まる行は、幅が本当より短くなり、消した時間と重なる部分が出る
- 直すところ: その文書（design :24 を「ST12 は main に入った」に、:157 を「区間は `archive::INTERVAL_SOURCES` を (a) とし、移行前のミリ秒の終わりは `stay_store::event_end_sql` と同じ読み方。点は ST12 のソースの並びから (d) に置くかを ST33 の下流が決める」に直す）。合意済みで直さないなら handoff ST33 へ
- 行き先: docs（ST33 の spec の段をやり直して spec・design・tasks に写し、合意を取り直す。本人の決定 2026-10-13: #72 は merge せず spec の段から）
- 機械: 安く捕まえられる（ST33 の下流の除く範囲の試験に、`endTimestampMs` の移行前の訪問を材料として足す）

## K2. ST33 の design D6 は「既存の `mark_late_arrivals` を範囲で 1 回呼ぶだけ」と書くが、ST12 の後この関数は論理ソースの並びを引数に取り、作り直しは書庫の位置を足した並びを渡している
- 重大度: 低
- 文書: openspec/changes/st33-data-export/design.md:197
- 要件: NFR-16, FR-50
- 根拠: design D6「中身は `rebuild_day` と同じ順（トランザクション → `lock(LOCK_KEY, user)` → 基準を読む）で、**既存の `mark_late_arrivals` を、本人が消した滞在の最小の始まり〜（最大の終わり＋1 マイクロ秒）の範囲で 1 回呼ぶ**だけ」（:196-197）。spec「作り直しと同じ判定で印を付ける」（spec.md:176）。いまの関数は `mark_late_arrivals(tx, user, sources: &[String], (from, to))`（`crates/server/src/stay_store.rs:796-801`）で、`rebuild_day` は `with_archive_sources(&c.sources)` を渡す（:568）。基準を読んで `c.sources` をそのまま渡すと、書庫の位置 7 本に `user:late` が付かず、spec の「作り直しと同じ判定」とずれる（書き出しのファイルには D5 の時間の判定で出ないので、外へ出るものは無い）
- 直すところ: その文書（D6 に「渡す並びは `rebuild_day` と同じ `with_archive_sources(基準のソース)`」と書く）か、handoff ST33
- 行き先: docs（ST33 の spec の段をやり直して spec・design・tasks に写し、合意を取り直す。本人の決定 2026-10-13: #72 は merge せず spec の段から）
- 機械: 安く捕まえられる（tasks 4.x の `mark_late_for_user` の試験に、書庫の位置を材料として 1 本足す）

## 未確認

- ST47 が「その他」の項目を新しい論理ソースに入れるのか、入れずに置くのか（ST47 の上流が未着手で、文書に決めが無い）。C4 の重さはこれで決まる
- `'1789182000000'::timestamptz` が PostgreSQL 17 で例外になるか（K1 は ST12 のコードと試験の注記から読んだ。DB で走らせてはいない）
- Q9 の「1 つのファイル」にしたとき、spec の「前回の書き出しのフォルダを消す」（Q4 第 2 回）と handoff ST33 の C3（フォルダ 0700・ファイル 0600）がどの形で残るか（C1 を直す側が決める）

## 触らなくてよいと判断したもの（理由つき）

- **ST33 の 2 本の監査で処置済みのもの** —— `2026-10-12-st33-nfr16-consistency.md` の C1〜C5・K1・K2、`2026-10-12-st33-consistency.md` の C1〜C11・K1。どれも行き先が付いている（C2 は Q9 として本人が答え、NFR-16 に戻した。その先の受け方が上の C1）
- **ST12 の監査の未確認「ST33 が書庫の写しを書き出すか」** —— 写しは DB の外のファイル（`core.archive_file.stored_path`。`migrations/202609181600_archive_ingestion.sql:69-79`）で、NFR-16 は D-01 の記録だけ。`core.archive_*` の表は proposal.md:99 が `Exclude` に足すと書き、D8 の分類の検査が名前で止める
- **NFR-16 の ★ 2026-09-29 の「削除の印を付けるのは滞在と位置のソースだけで、書庫の位置…」** —— 経緯の ★ の文で、決めた時点の事実として正しい。本文の除く範囲は ST12 の連鎖があっても変わらない（C2 は ST33 の spec の材料の話）
- **正典 `collection-coverage` の変更（「直近に置いた書庫」の箱の高さ）** —— 稼働状況の画面の予算の話で、ST33 の S-7 と書き出しに触れない
- **正典 `external-ingestion`:325「印を付けないと…読み出しと書き出しへ流れる」** —— ST33 は印に頼らず時間と緯度経度でも除くので、書き出しの側が厳しくなるだけで食い違わない
- **INDEX・stories.json・ST33.md の ST33 の行（requires に ST22、layer 3）と ST47 の行（layer 4、requires ST12）** —— 鎖は `check_chain.py` で通っている
- **走っている st25-day-timeline** —— FR-50 ★ 2026-10-13 で、消した時間に URL にだけ座標を持つ書庫の項目が生きた記録として残り、「ほかの記録」の行に出る。要約は題名と URL のホストだけで座標を出さない（ST12 の監査の判断と同じ）。失われるもの無し
- **走っている st08 / st17 / st21** —— Q9 の暗号化は本人が開けば同じ中身が読めるので、ST17 の「書き出しからは見える（Q10）」の前提は崩れない。FR-50 ★ 2026-10-13 は書庫の項目だけの話で、3 本の記録に触れない
- **扉 #24（バックアップ暗号鍵の保管方式）** —— Q9 の合言葉は保存しない書き出しの鍵で、拠点外へ送るバックアップの鍵ではない
- **zip の暗号化に要る crate** —— `crates/server/Cargo.toml:29` に `zip = "2"` が既にあり、`tools/check-offsite.sh` の一覧（送り手の crate）に当たらない
