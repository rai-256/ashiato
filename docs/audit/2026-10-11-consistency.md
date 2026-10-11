# 整合の確認（2026-10-11）

要件を「衛星の基盤」へ改訂した後、`/stories` の前に、ほかの文書とコードとの食い違いを独立の検査 2 本で洗った。
指摘は計 51 件（要件と他の文書 C1〜C34、文書とコード K1〜K17）。原文の報告は末尾の 2 節。

## 本人の決定

- **C6（衛星の読み出しをどこまで細かく残すか）→ A**（2026-10-11、本人「a で良いので進めてください」）。
  衛星の読み出しは既存の読み出しの記録（ST28）と別の台帳に分け、記録の識別子まで残す（PERM-13）。

## 行き先（1 件ずつ）

| 行き先 | 指摘 |
|---|---|
| 要件（このブランチで直した） | C6・C8・C10・C11（要件の分）・C23・C28・K1（EXT-J）・K5（PERM-10 の本文） |
| 他の文書（このブランチで直した） | C9・C15・C16・C17・C18・C20・C25・C29・C30・C33・K1〜K3（注記）・K6〜K17（文書の側） |
| `/stories`（次の工程） | C1〜C5・C7・C11（ST12 の残件の宛先）・C12〜C14・C19・C21・C22・C24・C27・C31・C34・K3（強制するかの判断）・K4（`user_id` の列）・K5（`record-envelope` の spec の MODIFIED）・PERM-10 の収集側の配り方の宛先 |
| コードの `fix/` | K1（`lib.rs:2400` の OpenAPI の説明文と `docs/openapi.json`）・K2（移行の前の退避）・K7（偽データの取得エラー）・K8（文言の lint。範囲を狭めるかは本人）・K12（既定の port。`net_guard.rs` / `tools/verify-prep.sh:96` / `crates/collector-windows/README.md`）・K13（API の版の定数）・K15（`ci.yml:116` の「86 本」の注記） |
| harness2（整合を保つ仕組み） | C32（check_chain が対象外の節の後ろまで読む）と、各指摘の「機械で捕まえられたか」 |
| 触らない | C26（★の中の経緯の引用。生きた宛先だった PERM-10 の★の「引き続き ST29」は直した）。凍結中の change（st08 / st12 / st21 / st25）には差し戻さない（失われるもの A は無かった） |

## 機械・AI・人間の分け方（この回の実測）

- 機械で捕まえられた / 安く捕まえられる: C5・C11・C14・C19・C24・C26・C27・C30・C31・K1・K3・K4・K5・K8・K9・K12・K17 など（各報告の「機械」の欄）
- AI でしか捕まらなかった（意味の衝突）: C1〜C4・C6〜C10・C16〜C18・C20・C23・K2・K6・K7
- 人間に聞いたもの: C6 の 1 件だけ
- 人間に問う残り（下流の深掘りで問う）: FR-101 の名前の形（扉 #30）・検索・視聴の履歴の外部 AI への承認（FR-88）・
  K8 の文言の分離の範囲・K3 の強制・C17 の RLS の理由（後の 2 つは AI の推奨つき）
- 独立レビュー（2026-10-11）の 17 件はすべて反映した（PERM-10 と FR-92 の矛盾、FR-101 を AI が決めていた件、例外の口の数など）


---

## 報告 1: 改訂後の要件と他の文書


対象: `/home/yosis/dev/ashiato2-req`（branch `docs/requirements-satellite`、HEAD 3cac9111）。repo のファイルは触っていない。
基準: `git diff origin/main -- docs/requirements.md` と `docs/audit/2026-10-09-satellite-foundation.md`。

実測:
- `python3 scripts/check_chain.py` → **FAIL**。未回収 20 件（FR-86〜FR-99, NFR-24, PERM-11〜PERM-15）。ST01 / ST07 / ST20 / ST24 / ST27 / ST28 / ST29 の 7 本が再生成結果と食い違う
- scratchpad に `make_story.py` で再生成して差分を見た。引用の部分は直るが、**stories.json が持つ判断（表題・価値・完了の判定・requires）は直らない**（下の C1〜C4）

件数: **高 9 / 中 15 / 低 10（計 34）**

凡例: 「機械」= 機械の検査で捕まえられたか。

---

## 高

### C1 ST24 の前提が要件から消えた
- 文書: `docs/stories/stories.json:527-`（ST24）、`docs/stories/ST24.md:3,64-66`、`INDEX.md:46`
- 要件: PERM-2 / PERM-15 / PERM-3〜PERM-6 / PERM-9 / 扉 #15
- 食い違い: ST24 の表題は「記録に感度を持たせ、既定で守る」で、完了の判定は「新しく入った収集記録の感度が既定で『外部 AI に出してよい』」「主観と写真が既定で『ローカル AI まで』」になっている。改訂後の PERM-2 は「すべての記録に『どの衛星にも出さない』の印を付けられるようにする」で、★ は「既にある記録は、すべて印なしとして始める」。PERM-3/4/6 はもう記録の値ではなく、「外部 AI」「ローカル AI」の 2 つの衛星への最初の承認を定めている。ST24 が satisfies に持つ PERM-3/4/6 は、衛星の登録簿（ST29 側）が無いと満たせない。requires の ST09 / ST17 も、印を付けるだけなら要らない
- 直すところ: stories.json（ST24 を「記録ごとの『出さない』の印 + 既存の感度の値を読み替える」に縮める。PERM-3/4/6 は AI の衛星の Story へ移す）→ 貼り直す
- 機械: 一部できる。check_chain が見ているのは引用が再生成と一致するかだけ。「satisfies の要件に、Story の判断より新しい ★ 訂正がある」を検査すれば見つけられた

### C2 ST29 の表題・価値・完了の判定が改訂後の FR-62 / FR-65 と逆
- 文書: `stories.json:650-`、`ST29.md:3,58-61`、`INDEX.md:51`
- 要件: FR-62 / FR-63 / FR-64 / FR-65 / FR-77 / PERM-8。新しく足した FR-90 / FR-92 / FR-94 / FR-96 / FR-97 と PERM-10★ / PERM-11〜PERM-14
- 食い違い:
  - 完了の判定は「読める感度の上限と対象ソースの宣言を求められる」。FR-62 は「読む種類と書く種類（FR-88）、書く種類ごとの分類の宣言」を求める
  - 完了の判定は「有効化に必要なのは S-01 の再起動だけ」。FR-65 は「登録と承認のみで有効化」で、★ に「本体の再起動は要らない」とある
  - 表題は「プラグイン」のまま。requires は ST24（感度）を指している
  - 承認・資格情報・取り外し・読み出しの台帳（PERM-11〜PERM-14・FR-90/96/97）を受け持つ Story が無い
- 直すところ: stories.json（ST29 を衛星の登録・承認・資格情報に書き直す。大きければ割る）と新しい Story
- 機械: 一部できる（C1 と同じ検査）。「再起動」は FR-65 の現行本文と完了の判定を照合すれば文字列で捕まる

### C3 ST27（AI）の完了の判定と依存が古い
- 文書: `stories.json:604-622`、`ST27.md:53`、`INDEX.md:49`
- 要件: FR-59★ / FR-60 / FR-88 / PERM-3〜PERM-6 / PERM-14
- 食い違い: 完了の判定は「『AI に出さない』の記録が返らない」。FR-60 は「その資格情報の衛星に承認された種類（PERM-11）の記録のうち、『出さない』の印が無いものだけを返す」。AI は FR-88 の 2 つの衛星になったので、ST27 は衛星の登録・承認・資格情報（PERM-14 で外部かローカルかを選ばせる）を前提にする。requires は ST24 のままで、ST29（か新しい衛星の基盤の Story）を指していない
- 直すところ: stories.json（完了の判定、requires を ST29 か新しい Story へ）
- 機械: 一部できる（C1 と同じ検査）

### C4 ST20 はいま着手できる状態なのに、完了の判定が消えた前提のまま
- 文書: `stories.json:445-462`、`ST20.md:41`
- 要件: PERM-5 / PERM-2★ / 扉 #4
- 食い違い: 完了の判定は「登録した人物の感度が、何もしなくても『AI に出さない』になっている」。PERM-5 は「人物の種類の読み取りを、最初はどの衛星にも承認しない」に変わり、PERM-2★ には「旧『AI に出さない』の人物を印と読むと、PERM-5 の承認で読めなくなる」とある。人物を出さない仕組みは承認であって、記録の値ではない。ST20 の requires（ST16）は archive 済みなので admission を通る。このまま回すと、消えた前提でテストを固定する
- 直すところ: stories.json（「人物の種類はどの衛星にも承認されていない」に書き換える。承認の器が無い間は、記録に印を付けないことを判定にする）
- 機械: 一部できる（C1 と同じ検査）

### C5 新しい要件 20 件を受け持つ Story が無い。期限付きの扉を止める仕掛けも無い
- 文書: `docs/stories/INDEX.md`、`stories.json`
- 要件: FR-86〜FR-99、PERM-11〜PERM-15、NFR-24。扉 #13（書き手。期限は衛星が書き始める日）/ #27（読み出しの台帳。期限は衛星が読み始める日）/ #28（終わりの時刻）/ #29（衛星の保管場所）
- 食い違い: どの Story の satisfies にも無い（check_chain が FAIL）。#13 / #27 / #28 は loss=uncaptured で、期限が「衛星が書き始める / 読み始める日」になっている。しかし衛星をつなぐ Story（V-02 など）が無いので、期限を盤面で止めるものが無い。audit §5 の新しい Story の候補 (1)〜(8) も stories.json に入っていない
- 直すところ: `/stories` を回し直して新しい Story を立てる（API の版、資格情報と承認、書き手の列と FR-92、読み出しの台帳、汎用の台帳と衛星の保管場所、区間の終わり、NFR-24 の試験）。ST29 / ST27 の requires をそこへ向ける
- 機械: **できた**（check_chain が既に FAIL を出している）

### C6 正典 `data-sensitivity` が、読み出しの記録に識別子を入れることを禁じている
- 文書: `openspec/specs/data-sensitivity/spec.md:295-296`（ST28 で archive 済み）、`migrations/202609290900_access_control.sql:19`
- 要件: PERM-13（★「件数でなく記録の識別子まで残す」）
- 食い違い: 正典は、行の中身を「時刻・経路・資格情報の種類（画面のログイン / API の合言葉 / 無し）・口の名前・結果**だけ**」とし、「記録の中身・求めの引数・…・**識別子の文字列を持たせない**」と定めている。PERM-13 は「どの衛星が・いつ・どの記録（識別子）を読んだか」を追記のみの台帳に残せと言う。既存の `core.access_log` を広げる実装はどちらかに違反する。要件の側も、PERM-13 の台帳が access_log とは別物かを書いていない
- 直すところ: requirements（PERM-13 は別の台帳であり、access_log の禁止とは別だと明記する）と、新しい Story の `data-sensitivity` の MODIFIED
- 機械: できない（意味の衝突）

### C7 書き手の列と FR-92 が、正典と送信契約の両方に無い
- 文書: `openspec/specs/record-envelope/spec.md:73-85`（「どの論理ソース・どの端末が生成したか」。書き手が無い）、`docs/collector-contract.md:13,22`（`authorization: Bearer <API_TOKEN>` の共有、`user_id` を送り手が送る）、`crates/server/src/ingest.rs:18`、`stories.json:29`（ST01 の完了の判定「ソースと端末」）
- 要件: FR-24（書き手）/ FR-92（書き手と利用者識別子を資格情報から決める）/ PERM-1★ / PERM-10★ / 扉 #13
- 食い違い: 要件は「利用者識別子と書き手はサーバが資格情報から決める」。正典と契約では、利用者識別子は封筒の自己申告で、書き手を入れる欄が無い。ST01 は archive 済みで、受け持つ Story も無い（C5）。衛星が書き始めた日から、書き手が記録されない（uncaptured）
- 直すところ: 新しい Story（`record-envelope` の MODIFIED、移行、契約の改訂）。ST01 へは戻さない
- 機械: 一部できる（check_chain が ST01 の引用の食い違いだけを出す）

### C8 PERM-14「資格情報は必ずどれか 1 つの衛星に結び付く」が、収集アプリと画面を勘定に入れていない
- 文書: `docs/requirements.md:843`（PERM-14）。関連: `docs/handoff/ST29.md`（収集側の合言葉の範囲は ST29 の Q4）、`docs/network.md:33`
- 要件: PERM-10（すべての API 要求に資格情報）/ PERM-14 / FR-99★（衛星のソースは NFR-13 の分母と FR-35 から外す）/ PERM-8
- 食い違い: C-01 / C-02 / V-01 は本体であって衛星ではない。それなのに資格情報を持つ（API_TOKEN と画面の合言葉）。PERM-14 を文字どおりに読むと、収集アプリの資格情報も衛星に結び付く。そうなると、新しい衛星は何も承認されない（PERM-8）し、衛星のソースは成功条件 1 の分母から外れる（FR-99★）ので、Must の 5 ソースの判定が崩れる。ST29 の深掘りが handoff の Q4（収集側の資格情報の範囲）を決めるとき、この幅で迷う
- 直すところ: requirements（収集アプリと画面の資格情報は衛星の外に置く、と PERM-14 か PERM-10 に書く。書き手（FR-24）の値の取り方も揃える）
- 機械: できない

### C9 製造準備の「読みは PostgREST で自動生成」が、承認・印・台帳・版と両立しない
- 文書: `docs/production-prep.md:28`（「PostgREST でスキーマから自動生成。テーブルを足せば読み取り口が出る」）、`docs/openapi.json:5` と `crates/server/src/lib.rs:2400`（「読みは PostgREST が別に自動生成する」）
- 要件: PERM-8 / PERM-11 / PERM-15 / PERM-13 / FR-63 / FR-89 / NFR-24（汎用の読み出し）
- 食い違い: 衛星の読み出しはすべて「承認された種類か → 『出さない』の印が無いか → 台帳に残す」を通らなければならない。表を足せば読み口が出る方式では、承認の前に読める口が生まれ（PERM-8 に反する）、表の変更がそのまま API の壊す変更になる（FR-89 に反する）。audit §3 も「OpenAPI の『読みは PostgREST』は実在しない」と書いている。汎用の読み出し API を作る Story が、この決定に従うと誤る
- 直すところ: production-prep の A-1（読みも自前の口にし、承認の門を 1 か所に集める）と、openapi の説明文
- 機械: 一部できる（`PostgREST` を grep し、要件の決定と突き合わせる）

---

## 中

### C10 衛星が自分の「収集した」記録を変えることを、FR-30 が禁じている
- 文書: `docs/requirements.md:337-343`（FR-30）と :640（FR-64）、:329（FR-25★）
- 要件: FR-64「その衛星が書いた記録の追加・**変更**・削除」と FR-30「『収集した』記録を後から書き換えない（例外は FR-22 の履歴と FR-51 の台帳だけ）」。FR-25★ は、睡眠の機器から取ったものを「収集した」に分類する
- 食い違い: 衛星の「変更」が FR-22（外部識別子による更新と履歴）を通るのかを書いていない。正典 `record-envelope` の DB の錠（「収集した記録は書き換えられない」）にぶつかる
- 直すところ: requirements（FR-64 の「変更」は FR-22 の経路だと明記する）
- 機械: できない

### C11 FR-88 の種類の表が、既存の論理ソースを覆っていない。ST12 が ST24 へ送った件の行き先も消えた
- 文書: `docs/requirements.md:672-693`（FR-88）。`openspec/changes/st12-archive-ingestion/deep.md:45-47,321-322`、`proposal.md:84,95`（「ST24 へ: 検索語を持つソースの既定の感度」）
- 要件: FR-88 / PERM-3★ / PERM-11
- 食い違い:
  - 表に無いもの: `c03-youtube-search` / `c03-youtube-watch` / `c03-myactivity-*`（検索語・視聴）、動画（FR-5）、端末時計の測定（FR-7、`c01-clock`）、離席・PC の停止・除外（FR-81〜FR-83）、`c03-timeline-*`（位置か滞在か）
  - 論理ソースから種類への写像の規則も無い
  - ST12 は検索語の扱いを「ST24 で締める」として送った。しかし handoff ファイル（`docs/handoff/ST24.md`）が無く、ST24 の前提も消えた（C1）。外部 AI の衛星に検索語が最初から承認されるかどうかが決まらない（ST27 の後は exported）
- 直すところ: requirements（種類と論理ソースの写像の規則、表に足す行）。`docs/handoff/` に ST12 の残件を新しい宛先で置く。ST12 は凍結中なので、tasks には戻さない
- 機械: **できる**（登録簿の `logical_source` の一覧と FR-88 の表の写像を突き合わせる検査）

### C12 4 段階の既定の値を、正典の Scenario が 4 つの capability で固定している
- 文書: `openspec/specs/derived-records/spec.md:313-323`、`desktop-collection/spec.md:252-274`、`personal-entities/spec.md:480-495`（と :382-388）、`record-envelope/spec.md:487-497`。走っている change: `st08-browser-history/specs/desktop-collection/spec.md:511-542`、`st08 tasks.md:163-168`、`st21-place-registry/tasks.md:89-91`
- 要件: PERM-2★（既存の値は印ではない。読み替える）/ PERM-3〜PERM-6
- 食い違い: 「その滞在の感度は『外部 AI に出してよい』である」などが正典の Scenario になっている。読み替える Story（C1）は、4〜5 本の capability を MODIFIED しなければならない。`personal-entities` は ST21 が走っている（ST20 も控えている）ので、admission で衝突する。st08 / st21 は凍結中で、書いている値（1）自体は無害
- 直すところ: 書き直した ST24（か新しい Story）の capability の割り当てを INDEX に明記する。凍結中の change には触らない（leave）
- 機械: 一部できる（正典の `感度` の Scenario を PERM-2 の版と照合する）

### C13 INDEX の capability の表と訂正の表が古い
- 文書: `docs/stories/INDEX.md:77`（`data-sensitivity` = 「感度・アクセス制御・プラグイン権限」）、:129 / :161 / :232（「PERM-3/4 の本体の Story は ST24。感度の操作は ST24」）
- 要件: PERM-3〜PERM-6（AI の衛星の承認）/ FR-86〜FR-89
- 食い違い: 衛星の基盤（登録・承認・汎用の台帳・API の版）を置く capability が無い。capability の名前は変えられないので、新しい capability を立てるか、`data-sensitivity` に積むかを決める必要がある。訂正の表は、PERM-3/4 の担い手を古いまま ST24 にしている
- 直すところ: INDEX（`/stories` の改訂と一緒に）
- 機械: 一部できる

### C14 INDEX の「着手前に閉じる必要がある扉」に新しい扉が無く、#26 は古い
- 文書: `docs/stories/INDEX.md:262-268`
- 要件: 扉 #13（★ の期限は衛星が書き始める日）/ #27 / #28 / #29。扉 #26（要件では 2026-09-07 に決定済）
- 食い違い: 期限付きの uncaptured の扉が表に無い。#26 は「現在未了」のまま
- 直すところ: INDEX
- 機械: **できる**（要件 §5 の「期限」と INDEX の表を突き合わせる）

### C15 ui-direction の S-7 設定の中身が、承認の画面を表していない
- 文書: `docs/ui-direction.md:23`（「S-7 設定 | ソース・停止・感度・プラグイン・書き出し | ST13, ST15, ST24, ST29, ST33 | … PERM-2〜9, FR-62〜65」）、`docs/screens.md`
- 要件: PERM-11（衛星 × 種類 × 読む / 書く）/ PERM-12（「取り消しても戻らない」を同じ画面に出す）/ PERM-14（資格情報の発行で衛星を選ばせ、名前を出す）/ FR-90 / FR-96 / FR-97（増えた種類だけ承認し直す）/ PERM-2・PERM-9（記録ごとの「出さない」とその確認）
- 食い違い: S-7 は「感度・プラグイン」の 1 行だけ。承認の行列、資格情報の発行、衛星の取り外しが面として立っていない。記録ごとの「出さない」の印は 1 件ごとの操作なので、S-5（記録の詳細）に置くのが自然だが、S-5 の触れる要件に PERM-2 が無い。proto を作る判定（`story_brief.py --screens`）は、この表で動く
- 直すところ: ui-direction（面の表の行。ui-review を通す）
- 機械: 一部できる（面の表の「触れる要件」と、Story の satisfies の画面要件を照合する）

### C16 製造準備のセキュリティ指摘の行き先が、消えた Story を指している
- 文書: `docs/production-prep.md` B ブロックの「自動セキュリティレビューの指摘」の表（「呼び出し元ごとの権限が無い → 感度による出し分け（PERM-2〜6 / FR-60）とプラグインの書き込み範囲の制限（FR-64）は ST24 / ST27 / ST29」「`user_id` を自由に名乗れる → ST29 の深掘り」）
- 要件: FR-92 / PERM-10★ / PERM-11
- 食い違い: `user_id` の自己申告を塞ぐのは FR-92 で、受け持つ Story が無い（C5）。出し分けも感度でなく承認になった
- 直すところ: production-prep（行き先を新しい Story に）
- 機械: できない

### C17 RLS を採らない理由（「守る対象が無い」）が成り立たなくなった
- 文書: `docs/production-prep.md` 「やらないと決めたもの」の RLS（「利用者は各自の PC で 1 人のまま。守る対象が無いので入れない」「衛星とプラグインで稼ぐ」）
- 要件: PERM-1★（権限は衛星の間で分かれる）/ PERM-11 / FR-64 / FR-77。FR-30 / FR-44 は「同じ PC の psql やプラグインが素通りする」ので錠を DB に置いている
- 食い違い: 衛星の間で分ける対象ができた。衛星は HTTP 越しだけ（FR-77）なので、結論（RLS は採らない）は保てるかもしれない。しかし、承認の門をアプリ層に置くか DB に置くかを要件も製造準備も書いていない
- 直すところ: production-prep（理由を「衛星は DB に直に触れず、承認の門は API の 1 か所」に差し替えるか、決め直す）
- 機械: できない

### C18 製造準備の「書きは取り込み口 1 本」に、新しい書き込みの器が入らない
- 文書: `docs/production-prep.md:29-30`（`logical_source` + `external_id` + `payload` + `raw` の 1 本に集める）
- 要件: FR-86 / FR-93（汎用の台帳と形の検査）/ FR-87（衛星の保管場所）/ FR-91（終わりの時刻）/ FR-92
- 食い違い: モノの台帳と衛星の保管場所は記録（`core.event`）ではない。1 本に集めるのか、口を足すのかが決まっていない。終わりの時刻の欄も封筒に無い
- 直すところ: production-prep の A-1（新しい Story の上流で決め、ここに戻す）
- 機械: できない

### C19 API に版が無く、壊す変更を見る検査も無い。契約のライセンスも未整理
- 文書: `docs/openapi.json`（`info.version` が `0.1.0`、パスに版が無い、`info.license` が `AGPL-3.0-only`）、`tools/check-openapi.sh`（コードとのずれを見るだけ）
- 要件: FR-89 / FR-95（壊す変更は版の差分を機械で見て判定し、前の版を 6 か月並べる）、§商用化の方針（衛星は有料でよい。ライセンスの細部は別途）
- 食い違い: 版ごとの契約も、壊す変更の判定も無い。契約そのものを AGPL で出すと有料の衛星が派生物と読まれる余地がある（audit U4）が、openapi は AGPL を宣言している
- 直すところ: 新しい Story（FR-89）。ライセンスはライセンスの整理（2026-10-09 の HTML）に回し、いまは leave
- 機械: **できる**（oasdiff などの openapi の差分検査）

### C20 送信契約が古く、衛星向けの契約が無い
- 文書: `docs/collector-contract.md:373-376`（「感度の欄は無い。`sensitivity = 1` = 外部 AI に出してよい」）と表 :20-37（終わりの時刻・書き手が無い）
- 要件: FR-99（衛星にも FR-20 / FR-21 / FR-23 / FR-27 を課す）、§5（衛星は溜めて再送する）、FR-91、FR-92、PERM-2
- 食い違い: 契約は収集アプリ 2 本のためのもので、衛星が従う契約が無い。感度の説明は読み替える前の値のまま
- 直すところ: 新しい Story で衛星の契約を作る。この文書は ST24 を書き直すときに直す
- 機械: できない

### C21 正典 `data-sensitivity` の Purpose と NFR-15 の写しが古い
- 文書: `openspec/specs/data-sensitivity/spec.md:6`（「記録ごとの感度とプラグインの権限も、この capability に積む」）、:119 / :124（「API の合言葉の読める範囲は変えない。呼び出し元ごとの範囲は PERM-10 の配り方として後で決める」）、:409（「感度で許した外部 AI の問い合わせ」）
- 要件: NFR-15★（承認した種類を衛星が持ち出すことは、拠点外に写しを置くことに当たらない）/ PERM-10★（資格情報は衛星ごと）/ PERM-11
- 食い違い: 「拠点外に写しを置かない」の例外が、要件では承認した衛星の持ち出しまで広がった。正典は AI だけを挙げている
- 直すところ: 新しい Story の MODIFIED
- 機械: 一部できる（Requirement の導出元の要件 ID の ★ の日付が、正典の日付より新しいものを列挙する）

### C22 NFR-24 の自動テストの置き場が、testing.md にも CI にも無い
- 文書: `docs/testing.md`（言及なし）、`.github/workflows/ci.yml`
- 要件: NFR-24（「判定は自動テストで行い、本体の移行が加わるたびに走らせる」）
- 食い違い: 層・置き場・job が無い。移行の検査（`check-migrations.sh`）にもつながっていない
- 直すところ: 新しい Story と testing.md の表
- 機械: できる（作れば、移行の差分で起動できる）

### C23 要件に、衛星の汎用の読み出しと、ソース・種類の登録の経路が無い
- 文書: `docs/requirements.md` の §3 衛星・API（audit §3「汎用の読み出し API」「`logical_source` の名前空間規則」、§5 の候補 (5)(8)）
- 要件: NFR-24（「読み出せること」）/ FR-63（返さない条件だけ）/ FR-65（「登録簿への 1 行」を誰がどう足すか）
- 食い違い: 読み出しの形（期間・種類・ページング）を定める SHALL が無い。衛星が自分の論理ソースや種類を登録する口（FR-94 は種類だけ）と、名前の衝突の規則も無い。成功条件 3 の判定が、何を読めれば通るかを決められない
- 直すところ: requirements
- 機械: できない

### C24 走っている change の口が、利用者識別子を求めの引数から取っている
- 文書: `openspec/changes/st21-place-registry/tasks.md:68,101,132`（`POST /places {id, user_id}`・`GET /places?user_id=`）、`st25-day-timeline/design.md:43-50`・`tasks.md:63,87`（`/day?user_id=`）
- 要件: FR-92 / PERM-1★（「利用者識別子は送られた内容から取らず、資格情報から決める」）
- 食い違い: どちらも画面（V-01）の口で、利用者は 1 名なので、いま失われるものは無い。しかし口を足すたびに型が広がり、FR-92 の Story が直す範囲が増える
- 直すところ: leave（凍結中）。FR-92 を受け持つ新しい Story が、既存の口の全部を対象にする（その Story の tasks に列挙する）
- 機械: できる（OpenAPI の各操作に `user_id` の引数があるかを数える）

---

## 低

### C25 README の位置づけが古い
- `README.md:3`（「全記録を…AI と衛星アプリに読ませる」）と :14（「S-01 … プラグイン基盤」）。要件 §1.1 / §1.3 は「衛星の基盤が主、AI は衛星の 1 つ」。README を直す。機械: できない

### C26 要件の本文に「プラグイン」が残っている
- `docs/requirements.md:342`（FR-30）、:489（FR-44 付近）、:813（PERM-10「第三者製のプラグインは同じ PC で動き」）、:818（「収集側・プラグインの配り方」）。§1.3★ で衛星に統合済み。:1115 は経緯の引用なのでそのままでよい。requirements を直す。機械: できる（★ の統合以降の本文で「プラグイン」を grep する）

### C27 正典の理由に「第三者製プラグイン」が残っている
- `record-envelope/spec.md:229,344`、`personal-entities/spec.md:204`。次に MODIFIED する Story がついでに直す。機械: できる（grep）

### C28 「衛星」が 2 つの意味で使われている
- `device-collection/spec.md:25,969,1058,1073,1146` と `collector-contract.md:242-252` の「衛星の時刻 / 衛星の時計」は GNSS のこと。要件の「衛星」は衛星アプリ。要件の §1.3 に、衛星（アプリ）と GNSS を区別する用語の注を置く。機械: できない

### C29 network.md と handoff/ST29.md は、ST29 の範囲が変わった後の宛先が曖昧
- `docs/network.md:33`（「収集側の合言葉は ST29 まで全読みのまま」）、`docs/handoff/ST29.md`（Q4）。中身は有効。ST29 を書き直したり割ったりしたら、宛先を付け替える（C8 と一緒に）。機械: できない

### C30 製造準備に「プラグイン」が残っている
- `docs/production-prep.md:38`（「プラグインが別プロセス」）、:39（「プラグイン / 衛星の境界 … 既存プラグイン」）、RLS の行（「衛星とプラグインで稼ぐ」）、C の「プラグイン境界の検査」。production-prep を直す。機械: できる（grep）

### C31 INDEX の satisfies 列が stories.json とずれている（改訂の前から）
- `INDEX.md:30`（ST01 に PERM-10 が無い）と :51（ST29 に FR-77 が無い）。stories.json には両方ある。check_chain は INDEX の表と stories.json を比べていない。INDEX を貼り直す。機械: **できる**

### C32 check_chain が「Story の対象外」の節の後ろに続く訂正の注記まで読んでいる
- check_chain の出力では、対象外が `FR-1, FR-53, NFR-11, NFR-20, NFR-8`。INDEX が対象外に挙げているのは NFR-8 / NFR-11 だけ。FR-1 / FR-53 / NFR-20 は、その節の後に足された ST05 / ST25 の訂正の注記から拾われている。いまは 3 件とも別の Story が受け持っているので害は無い。しかし新しい要件の ID がそこに書かれると、黙って「未回収」から外れる。check_chain（harness2 側）か、INDEX の節の配置を直す。機械: 検査そのものの穴

### C33 書き出し（ST33）の対象に、新しい器が挙がっていない
- `docs/handoff/ST33.md`、ST33 の完了の判定「全記録を書き出し」。NFR-16 の「D-01 の全記録」に、FR-86 の台帳・FR-87 の衛星の保管場所・書き手の列・「出さない」の印が入るかを、どこも書いていない。ST33 は未着手なので、handoff に 1 項を足せば済む。機械: できない

### C34 日の境界のタイムゾーンを要件が扱っていない
- audit §4 / §5 の候補 (7)（衛星が海外で書くと日の境界がずれる。`coverage.rs:14` と NFR-13 は `Asia/Tokyo` 固定。production-prep A-2 は「日の境界は記録時点の `tz_id` で切る」）。FR-91 は足したが、境界の規則は足していない。導出なので戻せる（B 段）。新しい Story の上流で決める。機械: できない

---

## 触らなくてよいと判断したもの（参考）

- `HANDOFF.md:113-122`（「プラグイン機構」）—— 2026-09-07 の引き継ぎの記録だと冒頭に明記してある
- `docs/CLA.md` / `CONTRIBUTING.md` —— 友人は別の D-01 の利用者で、当面は個人での開発（ライセンスの L3 / L5 は保留）。食い違いは無い
- `docs/testing.md` / `docs/flow-gates.md` —— NFR-24（C22）のほかに食い違いは無い
- ST21 の FR-98 —— ST21 の座標の記録は「直す / 移った」を必須にしているので、衛星に開ける日にもそのまま使える
- st25 の C5 / D17（「1 日の画面は感度で隠さない」）—— V-01 は本人の画面で衛星ではないので、改訂後も成り立つ

---

## 報告 2: 文書とコード


対象: `/home/yosis/dev/ashiato2-req`（branch `docs/requirements-satellite`。コードは origin/main と同じ）。
2026-10-11。リポジトリのファイルは編集していない。

集計: **高 3 / 中 9 / 低 5**（計 17 件）

機械で全体を見たもの:
- 正典の spec（`openspec/specs/*/spec.md`）にある `#### Scenario:` 646 件すべてについて、`crates` `web/src` `web/e2e` `collector-android/app/src` `tools` に `Scenario: <名前>` の印があるかを確かめた（空白は無視）→ **印の無いものは 0 件**。
- `docs/screens.md` と `web/src/Root.tsx` の行き先（`/`・`#/day/<日付>`・`#/day/`・`#/master`）、ログインの期限なし（`WEB_SESSION_MAX_AGE_DAYS` 未設定なら 0）、port 5180 → **食い違いなし**。
- `docs/collector-contract.md` の送る形の欄（`ingest.rs:16-46`）・冪等キー（`ingest.rs:178-193`。SHA-256、長さを前置）・200/400/401 の意味（`lib.rs:1283-1312`）・裸のオブジェクトも受ける・`external_id_kind` を宣言していなければ record に倒れる・5 分間隔（`LocationFix.kt:17` / `runtime.rs:45`）・想定間隔の初期値（6h×4 / 24h）・drops の理由 4 種とエラー → **一致**。

---

## 高

### K1 読み出しは「PostgREST が自動で作る」と書いてあるが、PostgREST はどこにも無い
- 文書: `docs/production-prep.md:28`（API 層の方式（読み）**PostgREST でスキーマから自動生成**）、`:30`（FR-61 は読み側だけ仕組みで守れる）、`docs/requirements.md:1143` EXT-J（「FR-61（読み取り API の実装手段）」）、`docs/openapi.json:5`
- コード: `docker-compose.yml` は `db` しか持たない。Cargo・`tools/`・CI のどこにも PostgREST は出てこない。読み出し口は axum の手書きのハンドラ（`crates/server/src/lib.rs:2221-2230` の `/events` `/coverage` `/stays*` `/attributes*`）。同じ文言がコードにもある: `lib.rs:2400` の `description = "…読みは PostgREST が別に自動生成する。"` → それが `docs/openapi.json` に生成されている
- 何が違うか: 「テーブルを足せば読み取り口が出る」は成り立っていない。新しいソースや種類を読むには、ハンドラを手で書く必要がある。衛星の読み取り API（PERM-11 / FR-88）を「PostgREST の上に足す」前提で Story を書くと、その土台が無い
- 直す場所: 文書（production-prep A-1 の 2 行と、EXT-J の「実装手段」）**と**コード（`lib.rs:2400` の description を直して `docs/openapi.json` を作り直す）。FR-61 は書き込み側（登録簿に 1 行）では守れているので、要件の本文はそのままでよい
- 機械の検査: いまある検査では捕まらない（`check-openapi.sh` はコードと json の一致しか見ないので、両方とも間違っていれば通る）。安い手として、`docs/` と `crates/` を `PostgREST` で grep し、`docker-compose.yml` に service が無ければ落とす 1 行を足せる

### K2 「移行を当てる直前に自動でバックアップを取る」が実装されていない
- 文書: `docs/production-prep.md:43`（DB マイグレーション: 「**適用の直前に自動でバックアップを取る**」。可逆性: 低 —— 個人のデータは復旧できない）
- コード: `crates/server/src/lib.rs:241-254` の `run_migrate()` は接続して `migrate()` を呼ぶだけ。`tools/stack.sh:72` も `"$server" migrate` を呼ぶだけ。`pg_dump` が出てくるのは `tools/smoke.sh:107`（縦串で戻せるかを試す所）だけで、`tools/` `scripts/` には他に無い。`docs/network.md:51` は本人が手で `pg_dump` すると書いている
- 何が違うか: 文書は守りがあると書いているが、コードには無い。実データの入った DB で `ashiato-server migrate` を叩いても、写しは取られない
- 直す場所: コード（`run_migrate` か、それを包む台本で先に退避する）か、文書（「手で退避する」に直し、network.md §6 を正にする）。安全側に倒すならコード
- 機械の検査: 無い。smoke で `migrate` の後に退避ファイルができたかを見る 1 手を足せば捕まえられる

### K3 「論理削除はビュー越しでしか読めない／素のテーブルを直接引かせない」が強制されていない
- 文書: `docs/production-prep.md:73`（A-3。「論理削除はビュー越しでしか読めない形にし、素のテーブルを直接引かせない。後から入れると全クエリを書き直す」）
- コード: `crates/server/src/grants.sql`。`ashiato_app` に `GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA core` を出しているので、アプリの役割は `core.event` を直に読める。本番のコードも素の表を読んでいる: `coverage.rs:631`（意図して `core.event`）、`deletion.rs:114,299,371,388`、`stay_store.rs:322,634,706,865`、`attributes_store.rs:491`、`lib.rs:853,891,1090`
- 何が違うか: 文書は構造で守ると書いているが、実際は規約で守っているだけ。`/events` は `core.event_live` を読む（`lib.rs:2167`）ものの、それを強制するものは無い。感度や承認で絞る読み出し（ST24 / ST27 / ST29、衛星）を「ビューを通せば済む」前提で設計すると、素の表を読む経路が残る
- 直す場所: 文書を実態（規約。書き込み・稼働状況は素の表を読む）に合わせるのが最小。強制したいならコード（読みだけの役割を分け、その役割にはビューにだけ SELECT を出す）
- 機械の検査: 無い。安いのは「`FROM core.event\b` を許してよいファイルの一覧」を持つ grep（`check-boundaries.sh` と同じ作り）。強いのは `app_role_tests.rs` に「読みの役割は `core.event` を SELECT できない」を足すこと

---

## 中

### K4 FR-29「すべてのテーブルに利用者識別子の列」を持たない表がある
- 文書: `docs/requirements.md:333`（FR-29: THE SYSTEM SHALL すべてのテーブルに利用者識別子の列を持たせる）、`docs/production-prep.md:114`（「`user_id` を全テーブルに持つ決定は残る」）
- コード: `migrations/202609290900_access_control.sql` の `core.access_log` と `core.web_session`、`migrations/202609151546_drop_reports.sql` の `core.drop_report_hour` に `user_id` が無い（migrations を走査して確かめた。残りの 13 表は持っている）
- 何が違うか: SHALL が満たされていない。PERM-13（どの衛星が・いつ・どの記録を読んだか）は読み出しの記録の上に立つ見込みで、利用者ごとに分ける列が無い
- 直す場所: 要件（子の表・認証の表を例外として書く）か、コード（列を足す）。`access_log` は衛星の Story の前に決めておく
- 機械の検査: 無い。`registry_tests.rs` と同じ形で、`information_schema.columns` から「`core` の表のうち `user_id` を持たないもの ⊆ 許した一覧」を確かめる試験が安く書ける

### K5 「すべての API 要求は資格情報を要求する」のに `/healthz` は資格情報なしで通る
- 文書: `openspec/specs/record-envelope/spec.md:221-224`（THE SYSTEM SHALL すべての API 要求に…資格情報を要求する）、`docs/production-prep.md:179`（「合言葉を全経路に要求」）
- コード: `crates/server/src/lib.rs:2217` の `.route("/healthz", get(|| async { "ok" }))` は `authorize` を通らない。`crates/collector-windows/src/clock.rs:204` は「**合言葉を要しない口**を使う」として、`/healthz` の `Date` を時計の基準にしている（`tools/smoke.sh:744` もこの口を見ている）。`POST /session`（ログイン）も、性質上は資格情報なしの要求
- 何が違うか: 正典の spec と実装・設計（ST05 / ST07 D17）が食い違っている。spec には例外が書かれていない
- 直す場所: spec（生存確認の口とログインの口を例外として書く）と production-prep:179
- 機械の検査: 無い。`router()` の route 一覧と「資格情報なしで通してよい口」の一覧を突き合わせる試験が `api_tests.rs` に安く書ける

### K6 縦串の説明が古い（「サーバ起動時に移行を当てる」・手順の数）
- 文書: `docs/production-prep.md:164-171`（「8 手順で通す」と書いて 9 手順を挙げ、手順 2 は「サーバ起動時にマイグレーションを当てる」）、`:141`（9 手順）
- コード: `crates/server/src/lib.rs` の `run()` には「移行は当てない（`ashiato-server migrate`。design D5）」とある。`tools/smoke.sh:29` は「2. 移行を当ててからサーバを起動（サーバは移行を当てない…）」。smoke の手順はいま 1〜42 と枝番（2b / 9b / 20b-d / 21b / 40a-b）がある
- 何が違うか: 手順 2 の意味が逆になっている（ST28 で移行とサーバの起動を分けた）。手順の数も 3 通りの値が書かれている
- 直す場所: 文書。「縦串が実際に確かめていること」を現在形で書き続けるなら、当時の記録と分ける
- 機械の検査: 無い（散文）

### K7 偽データは「4 状態」と書いてあるが、取得エラーの状態が無い
- 文書: `docs/production-prep.md:79`（A-4: 通常（滞在 9 件）・要件上限（15 件）・空・**取得エラー**の 4 状態を再現できること）
- コード: `tools/seed.sh:5-25` は `normal|max|empty` の 3 つだけで、それ以外は使い方を出して `exit 2`
- 何が違うか: 取得エラー（生存信号の `capturable=false` や破棄の報告など）を再現する偽データが無い。e2e で稼働状況の③④を見るときに、Story ごとに fixture を作ることになる（A-4 が避けようとしたこと）
- 直す場所: コード（`seed.sh error` を足す）か、文書（3 状態にして、エラーは Story 側へ送る）
- 機械の検査: 無い

### K8 i18n「文言をコードから分離する」が守られていない
- 文書: `docs/production-prep.md:47`（日本語のみ。**ただし文言をコードから分離し、後から言語を足せる形にする**）
- コード: 画面の文言は JSX に直書きされている。例: `web/src/MasterView.tsx:99`「マスタ管理」・`:122`「読み込み中…」・`:33-36`、`web/src/Root.tsx:32-34`、`session.tsx`、`DayView.tsx`。一部（`stays.ts` `coverage.ts` `attributes.ts`）は分けてあるが、全部ではない
- 何が違うか: 「後から入れると全部直すことになるもの」として決めた事項が、Story ごとに崩れている
- 直す場所: 文書（範囲を「API の語彙だけ」に狭める）かコード（文言を外へ出す）。決めるのは本人
- 機械の検査: 無い。`eslint-plugin-i18next` の `no-literal-string`（JSX だけ）を `npm run lint` に足せば機械で止められる

### K9 `docs/testing.md` §8「CI と同じ」の手順では `cargo test` が通らない
- 文書: `docs/testing.md:128-142`（`./tools/db.sh up -d --wait db && cargo test --workspace` …）
- コード: `crates/server/src/testdb.rs:81,113` は役割が無いと「`tools/db-roles.sh` を先に実行する」と panic する。CI の rust job（`.github/workflows/ci.yml:67-69`）は `ci-db-env.sh` → `docker compose up` → `db-roles.sh` の順。§8 にはさらに、CI が走らせている `check-offsite.sh`（2 本）・`check-exposure.sh --self-test`・`check-private.sh`（2 本）・`check-log-private.sh`（2 本）・`check-no-time-server.sh`（2 本）・`check-db-secret.sh` が無い
- 何が違うか: 「送る前に走らせるもの（CI と同じ）」をそのとおりに打つと `cargo test` が落ちる。通ったとしても、CI でだけ落ちる検査が 9 本残る
- 直す場所: 文書
- 機械の検査: 無い。§8 のコードブロックと ci.yml の `run:` 行を突き合わせるスクリプトは安く書ける

### K10 `docs/testing.md` §1 の job の説明が CI と違う
- 文書: `docs/testing.md:27-30`（`rust`（fmt / clippy / test + **検査 4 本**）… `chain`（**token があるとき**））
- コード: `.github/workflows/ci.yml:75-85` の rust job は検査を 10 本走らせる（migrations / boundaries / openapi / offsite ×2 / exposure / private / no-time-server ×2）。`chain` job（`:257-274`）は token が無くても**毎回**走り、`check-private.sh`・`check-log-private.sh`（2 本）・`check-db-secret.sh` を回す。token の有無で変わるのは `check_chain.py` だけ
- 何が違うか: 私的データの検査（網の名前・ログ・DB の合言葉）がどの job で効いているかを読み違える。「token が無いから chain は飛ぶ」と読むと、これらの検査も飛ぶと誤解する
- 直す場所: 文書
- 機械の検査: 無い

### K11 CLAUDE.md の構成の表が「削除（`drops.rs`）」と書いている
- 文書: `CLAUDE.md` の「構成」の表、`crates/server` の行（「属性・削除（`drops.rs`）」）
- コード: `crates/server/src/drops.rs:2` は「端末からの**破棄の報告**（ST04）」。滞在の削除は `deletion.rs`（ST22。`lib.rs:44`）。ST28 の `web_session.rs` / `access_log.rs` / `net_guard.rs` も表に無い
- 何が違うか: deep-review・iwakan・Task agent が「探索の入口」にしている表なので、削除の Story（FR-51 など）が別の機能のファイルから当たりを付けることになる
- 直す場所: CLAUDE.md
- 機械の検査: パスがあるかどうかは見られるが、意味の取り違えは捕まらない。表のパスが実在するかの検査も、いまは無い

### K12 API の既定の port が文書では 18787、コードでは 8787。C-02 の例は 8787 を指す
- 文書: `docs/network.md:8`（サーバは `BIND`（**既定 `127.0.0.1:18787`**））、`tools/verify-prep.sh:96`（同じ）
- コード: `crates/server/src/net_guard.rs:30` の `DEFAULT_BIND = "127.0.0.1:8787"`。18787 は `.env.example:14` の値。C-02 の起動例（`crates/collector-windows/README.md:11`・`src/main.rs:5`）は `http://127.0.0.1:8787`
- 何が違うか: 「既定」は `.env.example` の値で、コードの既定ではない。`.env` どおりに 18787 で立てたサーバに、README どおりの C-02 は繋がらない
- 直す場所: コードの既定を `.env.example` に揃えるか、文書を「`.env.example` の値」と書き直す。あわせて C-02 の README
- 機械の検査: 無い。`DEFAULT_BIND` と `.env.example` の `BIND` を突き合わせる grep 1 行で捕まえられる

---

## 低

### K13 OpenAPI の版が手書きの定数
- 文書: `docs/production-prep.md:46`（バージョンの単一情報源: build 設定から機械で取る。**手書きの定数を作らない**）
- コード: `crates/server/src/lib.rs:2399` の `version = "0.1.0"`（`Cargo.toml:6` の写し）
- 直す場所: コード（`version` を省くと utoipa が `CARGO_PKG_VERSION` を使う。または `env!`）
- 機械の検査: `check-openapi.sh` では捕まらない（コードと json が一致していれば通る）

### K14 `docs/testing.md` §1 の結合テストの置き場が古い・足りない
- 文書: `docs/testing.md:11`（`coverage/tests.rs`、結合は `api_tests.rs` / `dedup_tests.rs` / `coverage/tests.rs` / `registry_tests.rs`）
- コード: 実際は `crates/server/src/coverage/tests/mod.rs`。ほかに `attributes_tests.rs` `stay_tests.rs` `deletion_tests.rs` `drops_tests.rs` `web_session_tests.rs` `access_log_tests.rs` `access_migration_tests.rs` `app_role_tests.rs` `clock_tests.rs` がある
- 直す場所: 文書（列挙をやめて `src/*_tests.rs` と書く。CLAUDE.md と同じ書き方）

### K15 collector-windows の単体テストの本数
- 文書: `docs/testing.md:12`（ubuntu で走る 86 本）。`.github/workflows/ci.yml:116` の注記も 86 本
- コード: `cargo test -p ashiato-collector-windows --lib -- --list` は Linux で **133 本**
- 直す場所: 文書（本数は書かない）

### K16 生存信号の未送信が `heartbeat.jsonl` だと書いてある
- 文書: `docs/collector-contract.md` の「再送の扱い」（**ファイルは分ける（`heartbeat.jsonl`）**。追記 JSONL）
- コード: `collector-android/.../LocationService.kt:218-260` は区切りの置き場（`SegmentStore`、`store("records")` / `store("heartbeats")`）を使う。`heartbeat.jsonl` は ST01 / ST02 の古いファイルで、起動時に取り込まれるだけ（`:481-491`、`LegacyOutbox.kt`）
- 直す場所: 文書

### K17 送信契約の `error` の表が、サーバの列挙の一部しか持たない
- 文書: `docs/collector-contract.md:81-90`（8 種）、生存信号の表（`unknown_source` が無い）。冒頭（`:4`）で「この文書と `ingest.rs` が単一の情報源」と書いている
- コード: `crates/server/src/lib.rs:497-540` の `IngestError` は 15 種（個人属性の主張の 7 種がある）。`HeartbeatError`（`lib.rs:1795-1806`）は `unknown_source` を持つ
- 何が違うか: 収集側が受け取りうるのは 8 種だけだが、衛星が `/ingest` に書き始めると、残りの 7 種も契約の範囲に入る
- 直す場所: 文書（「主張だけに当たる 7 種は `personal-entities` の spec の表」と 1 行で指す）
- 機械の検査: `IngestError` を serde で並べたものと文書の表を突き合わせる単体テストが安く書ける

---

## 未確認（食い違いとは言えない、または確かめていない）

- `docs/production-prep.md` B 表の rc と件数（Rust 237 / Node 170 のライセンス数、`2 passed`、`BUILD SUCCESSFUL（35 タスク）`）は 2026-09-08 の実行記録なので、再実行していない
- `check-licenses.sh` / `check-migrations.sh` が「壊すと落ちる」かは、壊して試していない
- `android-instrumented` job の中身、Windows の実行時テスト（10 本の `#[test]`。CI は 7 本以上を求める）の実際の走り方
- `docs/requirements.md` の ★ のうち、ここで見たのは `default_sensitivity`（`lib.rs:567` = 1 / `attributes.rs:22` = 2 / `migrations/202609081618_envelope.sql:28` の DEFAULT 1 / `default_sensitivity_matches_the_column` 試験）、束ねた形の `max(sensitivity)`（`202609120943_version_and_ledger.sql:105`）、`Asia/Tokyo`（`coverage.rs:14`）、`expected_gap_sec / 86400`（`coverage.rs:991`）、C-02 が `/healthz` を叩くこと（`clock.rs:204`）。**どれも本当だった**。PERM-2 / FR-24 / FR-92 / 扉 #13 は「未実装」と正直に書かれていて、食い違いではない。それ以外の ★ のコードへの言及は見ていない
- 正典の spec は、印があることだけを全件で見た。印の先の試験が Scenario を本当に観測しているかは、record-deletion の「二度目の消す求め」（`deletion.rs:273-291` が `deleted_at IS NULL` で絞る）と personal-entities の既定の感度でだけ確かめた
- `docs/collector-contract.md` の C-02 の payload の欄（`contract.rs` の `WindowPayload`）と、C-01 の app-usage / clock の欄を 1 つずつ突き合わせることはしていない
