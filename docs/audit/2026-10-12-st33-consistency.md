# 整合の確認（2026-10-12・ST33 の深掘りが NFR-16 を改訂した枝を、main の上へ rebase した後）

- 基準: `929aede45f24f4ef0ce729630bd88fb867e2aa63`..`6932395c077a4e67d1d4f8dd496233c3154c3d0c`

対象は ST33（`openspec/changes/st33-data-export/`）の深掘りが要件へ戻した NFR-16 の改訂（★ 2026-09-29 訂正・★ 2026-10-12 追加）と、
同じ枝で直した `docs/stories/ST33.md` `stories.json` `INDEX.md`（requires に ST22、layer 1 → 3）。
改訂は main の「衛星の基盤」の改訂（2026-10-11）と UI の表面の改訂（2026-10-12）、ST17 の改訂の上へ rebase された。**main 側で新しくなった要件・Story（ST37〜ST46）・正典・申し送り（`docs/handoff/ST33.md` の「2026-10-11 の整合の確認」の節 = C33）との食い違いを重点に見た。**
文書は head（`6932395c`）の内容で読んだ。作業ツリーの未追跡ファイル（`deep-answers-4.txt`）は見ていない。ST33 の内側の食い違い（rebase の前）は `docs/audit/2026-10-12-st33-nfr16-consistency.md` が見ているので、ここでは繰り返さない。

機械の入力:
- `req_impact.py --base 929aede4`: 改訂 1 件（NFR-16）・扉 0 項。名指しは `docs/handoff/ST33.md:8,23,27,34` / `docs/stories/INDEX.md:56` / `stories.json:775` / `docs/ui-direction.md:23` / ST33 の change の各文書 / 正典 `openspec/specs/record-deletion/spec.md:60`
- `check_chain.py`: 未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった: NFR-16」（この監査が受けるもの）
- `check_scenarios.py`: 担保なし 85 件は全部 ST33 の change の Scenario（下流が未着手）。正典 `openspec/specs/` の担保なしは 0

走っている change（archive 以外）: st08-browser-history / st12-archive-ingestion / st17-daily-feeling / st21-place-registry / st25-day-timeline / st33-data-export（改訂した側）。
印を付けたのは ST17（C1）。ST17 の側で失われるもの（uncaptured / discarded / exported / rewrite-all）は起きない（C1 は除き過ぎの側）。

## C1. ST17 の主観の記録は、消した滞在に付いたもの・消した滞在のある日の「日」の記録が、書き出しの規則で除かれる。ST17 の spec は「書き出しからは見える」を Q10 の前提にしている
- 重大度: 高
- 文書: openspec/changes/st33-data-export/design.md:162
- 要件: NFR-16, FR-36, FR-37
- 根拠: ST33 design D5a(1)「緯度経度を持たないと決まったソースの一覧: `c01-app-usage` / … / `s01-attribute`。**一覧に無いソースは緯度経度を持つとみなす**」、D5b（:152）「`SPAN_RULES` で『時刻 1 点』と決まっていない**すべてのソース**（既定）」は次の記録まで。ST17 の design（`openspec/changes/st17-daily-feeling/design.md:47`）は `s01-feeling` の `event_time` を「日は `<date>T00:00:00+09:00`、滞在は原文の `target.start`」、`device_id` を NULL（:49）と置く。そのため、消した滞在に付けた気分は `event_time` が消した時間の始まりと一致して除かれ、「日」の記録は次の主観の記録までの幅を持ち、その間に消した滞在があれば除かれる。ST17 の spec（`openspec/changes/st17-daily-feeling/specs/subjective-log/spec.md:507`）は「本人が消した滞在の記録は消えていない（ST22 は主観に触らない）—— 隠すのはこの読み出しの滞在ごとの欄だけで、記録は検索・AI・書き出しからは見える（Q10）」と書く。ST33 の design:24「まだ main に無いもの: ST12…、ST21…、ST25」は ST17（と ST08）を挙げておらず、`s01-feeling` をどちらの一覧にも置いていない。ST17 の表（`core.feeling_target_kind` `core.feeling_reminder`）は D8 の分類の検査が拾うが、ソースの分類（D5a(1) / D5b）には同じ検査が無い
- 直すところ: その文書（ST33 の design D5a(1) と D5b に `s01-feeling` を置く —— 本人が書いた記録で座標の項目を持たず、幅は紐づけ先の `target`）か、凍結した Story への申し送り（ST17 の下流が `NO_COORD_SOURCES` と `SPAN_RULES` に足す。どちらが先に archive されても入る形で）。ST17 は走っている change なので印だけ。除き過ぎの側で、出したものは無いので失われるものは無い
- 機械: 安く捕まえられる（D8 と同じ形で、`core.source` の全ソースが「緯度経度を持たない / 持つ」と「幅の読み方」のどちらかに明示で置かれていることを試験にすれば、ST17・ST21・衛星のソースが入った時点で名前を挙げて落ちる。いまは「一覧に無い＝持つ」の既定なので黙って除く）
- 行き先: handoff ST33（どちらが先に archive されても `s01-feeling` が D5a(1) と D5b に置かれる形を、ST33 の下流と ST17 の申し送りに書いた。除き過ぎの側で失われるものは無い。仕様は合意済みなので ST33 の design は直さない）

## C2. 書き出しは全記録の平文を自宅 PC のフォルダに置くが、同じ PC で動く衛星（AI を含む）がそれを読めるかを、要件も ST33 も決めていない
- 重大度: 高
- 文書: docs/requirements.md:1121
- 要件: NFR-16, NFR-15, PERM-8, PERM-10, PERM-11, PERM-15, FR-63, FR-77
- 根拠: NFR-16「書き出したファイルは本システムが自宅 PC の決まったフォルダにだけ置き、画面から端末へは渡さない」。ST33 の design D3（:76-80）の既定は `%LOCALAPPDATA%\ashiato\export` で、考えた危険は同期されるフォルダだけ（C16）。一方 main 側の要件と製造準備は衛星が同じ PC で動くことを前提にしている —— PERM-10 ★（requirements.md:862-863）「同じ PC の中から呼ぶものを何も止めない。第三者製のプラグインは同じ PC で動き」、`docs/production-prep.md:192`「第三者製衛星は同じ PC で動くので、素通しだと PERM-8 … が空文になる」。衛星が持ち出してよいのは「承認した種類（PERM-11）の記録」（NFR-15 ★ 2026-10-11、requirements.md:1117）だけで、印の付いた記録は「どの承認があっても」返さない（PERM-15）。書き出しのフォルダは主観・人物・写真の位置・印の付いた記録を区別せずに平文で持つ（spec:103「感度で絞らずにすべての感度の記録を出す」）ので、ファイルを読める衛星やローカル AI の道具には、承認の門を通らずに全部が渡る。ST33 の change の 5 文書（deep / proposal / design / tasks / spec）に「衛星」は 1 度も出てこない（`grep -c 衛星` が 0）。NFR-16 の改訂は 2026-09-29、衛星の基盤は 2026-10-11 で、ST33 の深掘りは後者を知らずに置き場を決めた
- 直すところ: requirements（NFR-16 の置き場に「衛星・AI の道具から読めない」を課すか、課さないことを ★ で決める。決めるのは本人 —— 外へ出たものは戻らない `loss: exported` の問いとして ST33 か衛星の基盤の Story の深掘りに立てる）
- 機械: できない（OS の上で同じ利用者のプロセスが読めるかは、本システムの試験の外）
- 行き先: ask（`loss: exported`。ST33 の深掘り 第 3 回の Q9 として deep.md に (未回答) で積み、`deep-questions-r2.json` と `docs/briefs/ST33-deep-r2.html` を作った）

## C3. 書き出しのフォルダとファイルの権限が決まっていない。main は同じ「全記録の写し」である移行前の退避を本人だけが読める権限で作ると決めた
- 重大度: 中
- 文書: openspec/changes/st33-data-export/design.md:76
- 要件: NFR-16, NFR-15
- 根拠: ST33 design D3「決め方: `ASHIATO_EXPORT_DIR` → `%LOCALAPPDATA%\ashiato\export`（Windows）→ `$XDG_DATA_HOME/ashiato/export` → `$HOME/.local/share/ashiato/export`」。`.partial-…` を作って `rename` する手順（:82-83）にも、spec の Requirement「全記録を自宅 PC の決まったフォルダに書き出せる」（spec:9-23）にも、権限の文が無い。main は 2026-10-11 に移行前の退避で同じ問題を直している —— `cb2bac39`「退避は実データの全部の写しなのに、既定の umask のまま（-rw-rw-r--）で置かれていた（コミット時のセキュリティレビューの指摘）。置き場を 0700、ファイルを … 0600 で作る」、`crates/server/src/migrate_backup.rs:138`（`from_mode(0o700)`）・`:150`（`o.mode(0o600)`）・`:239-248`（試験）。Linux の代わりの場所（`$HOME/.local/share`）では、既定の umask で同じ PC の別の利用者が読める
- 直すところ: その文書（design D3 に、置き場と書き出しのフォルダを 0700、ファイルを 0600 で作る（Windows は利用者のプロファイルの既定の ACL）と書き、tasks に試験を足す）
- 機械: 安く捕まえられる（`migrate_backup.rs:239-248` と同じ形で、書き出したフォルダとファイルの mode を unix で確かめる試験 1 本）
- 行き先: handoff ST33（下流が design D3 の置き場とフォルダを 0700・ファイルを 0600 で作り、`migrate_backup.rs` と同じ形の試験を置く。既定は厳しい側（C）で、Q9 の答えに依らず要る）

## C4. 書き出しの口は「読み出しの口と同じ資格情報」で、衛星の資格情報を断る決まりが無い。ST29 の全 route の試験は「記録を返す口」だけを数えるので、この口を拾わない
- 重大度: 中
- 文書: openspec/changes/st33-data-export/specs/data-durability/spec.md:23
- 要件: PERM-10, PERM-11, PERM-15, FR-63
- 根拠: spec:11「WHEN 資格情報を持つ呼び出し元が書き出しを求める THE SYSTEM SHALL 書き出しを背景で始め」、spec:23「資格情報の扱いは記録の読み出しの口と同じとする」、design D1（:54）「資格情報は `authorize` をそのまま使う（C11。`GET /events` と同じ扱い）」。`GET /export/summary` は全ソースの件数・いちばん古い時刻・いちばん新しい時刻（design:52）を、`GET /export` は置き場の絶対パス（design:51 `folder`）を返す。deep.md C11（:163）は「呼び出し元ごとの本決めは ST29（PERM-10）」と書くが、main の改訂で収集アプリと画面の資格情報は ST44、衛星の資格情報は ST29、衛星の読み出しの門は ST45 に分かれた。ST29 への申し送り（`docs/handoff/ST29.md:27`）の試験は「router の全 route を衛星の資格情報で呼んで、**記録を返す口が 0**」で、記録を返さない書き出しの 3 本はこれを通る。衛星の資格情報で書き出しを始め、置き場のパスと、承認していない種類（主観・人物）を含むソースごとの件数と期間を得られる（C2 と合わせると、パスを知った衛星が平文を読める）
- 直すところ: その文書（spec の書き出し・件数・状態の口の呼び出し元を「画面のログイン（と API の合言葉）」に限り、衛星の資格情報は断ると書く。C11 の「ST29」を ST44 / ST29 に直す）と、凍結した Story への申し送り（ST29 の全 route の試験を「衛星の資格情報で 2xx を返す口が、衛星向けに作った口だけ」に広げる）
- 機械: 安く捕まえられる（ST29 の全 route の試験の判定を「記録を返す」から「2xx を返す」に変えれば、書き出しの 3 本も後から足した口も拾う）
- 行き先: handoff ST29（衛星の資格情報では書き出しの 3 本の口も断る。全 route の試験を「衛星の資格情報で 2xx を返す口が、衛星向けの口だけ」に広げる。ST29 の前に衛星の資格情報は無く、失われるものは無い）

## C5. `docs/handoff/ST33.md` の「2026-10-11 の整合の確認」の節（C33）を、ST33 の深掘りも tasks も受けていない。節の「その Story の後に形を確かめる」は ST33 の順では起きない
- 重大度: 中
- 文書: openspec/changes/st33-data-export/deep.md:9
- 要件: NFR-16, FR-24, FR-86, FR-87, PERM-2, PERM-15
- 根拠: deep.md:9-14「この Story に宛てられたもの」は `st22-record-deletion R2` の 1 件だけで、「別の change のレビューの `処置: deferred ST33` は無い（ブリーフの『この Story に宛てられたもの』は申し送り 1 件だけ）」。rebase 後の `docs/handoff/ST33.md:21-34` は「書き出しの対象に…モノの台帳（FR-86）・衛星の保管場所（FR-87）・書き手の列（FR-24）・記録ごとの『出さない』の印（PERM-2）を入れる」「印の付いた記録も書き出し、印そのものも書き出す。別の扱いにするなら ST33 の深掘りで本人に問う」「印は親の記録に従う…書き出しでも親と束ねて出す」「ST33 の上流は**その Story の後に**形を確かめる」と書く。tasks 10.3（tasks.md:220-222）は申し送りを PR の前に読み直すが、検証は `grep -c 'st22-record-deletion' docs/handoff/ST33.md` で R2 の節しか見ない。ST33 は layer 3（requires ST01 / ST22）、器を作る ST40 / ST46 は layer 6 で、ST33 の上流は「その Story の後」にならない。器の側は ST40（stories.json:954）・ST46（:1090）が breaks に「ST33 の書き出し」を持つが、書き手の列の ST37 と印の ST24 は持たない（D8 / D9 の検査が、列を足した時点で分類と `core.event_live` の作り直しを求めるので、黙って落ちはしない）
- 直すところ: その文書（deep.md の「宛てられたもの」に 2026-10-11 の節を足し、印を出すことを C5（感度で絞らない）と同じ側として受ける。tasks 10.3 の grep に節の数を足す）と、`docs/handoff/ST33.md` の 2026-10-11 の節（「ST33 の上流がその Story の後に確かめる」を「器を作る ST24 / ST37 / ST40 / ST46 が ST33 の分類（`export::CATALOG`）に足す」に書き直す）
- 機械: 安く捕まえられる（`story_brief.py` の「宛てられたもの」は `docs/handoff/ST<NN>.md` の節を数えられるので、deep.md の記載と数を突き合わせられる。tasks 10.3 の grep が節ごとの見出しを数えれば読み落としで落ちる）
- 行き先: docs（deep.md の「この Story に宛てられたもの」に 2026-10-11 の節を足して受け、`docs/handoff/ST33.md` の「その Story の後に形を確かめる」を、器を作る Story が ST33 の分類に足す形に書き直した。tasks は合意済みなので 10.3 の grep は直さず、申し送りに書いた）

## C6. 正典になる data-durability の Scenario がファイルの閉じた一覧と「感度の段」を固定するが、そこへ足す・読み替える Story は capability の表で data-durability を持たない
- 重大度: 中
- 文書: openspec/changes/st33-data-export/specs/data-durability/spec.md:280
- 要件: NFR-16, PERM-2, FR-86, FR-87
- 根拠: spec:280「2 度目のフォルダのファイルは、記録・前の版・ソースの登録簿・個人属性の種類と名前・滞在の基準・生存信号・停止と破棄の期間・破棄の報告（日ごと時間ごと）のファイルと、説明書と目録だけで、**それ以外のファイルは無い**」。spec:103「感度で絞らずにすべての感度の記録を出す」、Scenario `どの感度の記録も出る`（spec:140-143「感度の段がそれぞれ違う記録を取り込んでから書き出す → 各行が感度の列を持つ」）。改訂後の PERM-2（requirements.md:796-）は 4 段階を記録ごとの「出さない」の印に縮め、「書き込み時に入れていた既定の感度…を印と読まない」。`docs/stories/INDEX.md:98-99`「ST24 は 6 つの capability を MODIFIED する（`data-sensitivity` のほか、4 段階の既定の値を Scenario に固定している `record-envelope` / `derived-records` / `desktop-collection` / `personal-entities` / `subjective-log`）」に data-durability は無い。モノの台帳と保管場所を書き出しに足す ST40 / ST46（breaks「ST33 の書き出し（…も対象になる）」）は `satellite-platform` で、`INDEX.md:91`「`data-durability` | … | ST10, ST30, ST31, ST32, ST33」に入っていない。ST33 が先に archive されると、閉じた一覧と感度の段が正典に残り、ST24 / ST40 / ST46 はそれを MODIFIED しないまま書き出しを変えることになる。admission は capability が重ならないので並べて走らせる
- 直すところ: stories（INDEX の `data-durability` に ST24 / ST40 / ST46 を足し、ST24 の「6 つ」に data-durability を足す）か、その文書（spec:280 の一覧を「分類（D8）が Include とした表のファイルと説明書と目録だけ」の開いた形にし、感度の Scenario を「記録の表のどの列でも絞らない」に言い直す。仕様は合意済みなので、直すなら合意の範囲で）
- 機械: 安く捕まえられる（`check_chain.py` で、`breaks` が「ST<NN> の…」を名指しする Story が、名指しされた Story と capability を 1 つも共有しなければ warn。ST40 / ST46 → ST33 がいま当たる）
- 行き先: stories（INDEX の `data-durability` に ST24 / ST40 / ST46 を足し、ST24 が MODIFIED する capability に `data-durability` を足した。admission が ST33 の archive を待たせる）

## C7. FR-91 の終わりの時刻の列（ST41）を、除く範囲の幅（D5b）と消した時間（`erased_spans`）が読まない
- 重大度: 中
- 文書: openspec/changes/st33-data-export/design.md:150
- 要件: NFR-16, FR-91
- 根拠: D5b (a)「内容の鍵のうち最初に `core.try_timestamptz` で読めるもの | `export::END_KEYS` = `end` / `end_time` / …」は `payload` の鍵だけを見る。消した時間も、前の版は `coalesce(core.try_timestamptz(v.payload->>'end'), v.event_time)`（design:117）、いまの版は `core.stay_erased` の `coalesce(core.try_timestamptz(payload->>'end'), …)`（`migrations/202609271716_deletion_ledger.sql:60`）。FR-91（requirements.md:290-）は「幅のある出来事の記録…に、終わりの時刻を持たせられるようにする」★「payload に入れると、期間の重なりで探せない」で、終わりを**列**に置く。ST41（layer 1、requires ST01）は ST33 より先に入りうるが、breaks（「ST01 の記録の骨格と送信契約」「収集アプリの送信」）に ST33 は無い。列を足せば D8 / D9 が分類を求めるので列は書き出しに入るが、除く判定は列を読まず (c)「次の記録まで」に落ちる。同じソースの記録が重なるとき（読書のひと続きの中に別の読書）は次の記録の始まりが終わりより前になり、消した時間と重なる部分が幅から外れる。滞在の終わりを列へ移す Story が出れば、前の版の広い時間（C19）は点に縮み、消した場面の緯度経度が出る（`loss: exported`）
- 直すところ: その文書（design D5b の (a) を「FR-91 の終わりの列があれば先にそれ、無ければ内容の鍵」と書き、`erased_spans` も同じ読み方にする）と、ST41 への申し送り（書き出しの除く判定と `core.stay_erased` が終わりを読む場所）
- 機械: 安く捕まえられる（ST41 が列を足すときに、終わりを列に持つ緯度経度つきの記録が消した時間と重なる試験を `export_tests.rs` に 1 本足せば落ちる。ST33 の側で `END_KEYS` の隣に「列」を置けば先回りできる）
- 行き先: handoff ST41（終わりの列を足すときに、ST33 の幅の読み方 `END_KEYS` と `core.stay_erased` が列を先に読むようにし、試験を 1 本置く）

## C8. 衛星が登録するソースは、除く判定の一覧（Rust の定数）に載る経路が無く、既定の「緯度経度を持つ・次の記録まで」で書き出しから除かれ続ける
- 重大度: 中
- 文書: openspec/changes/st33-data-export/design.md:164
- 要件: NFR-16, FR-61, FR-62, NFR-24
- 根拠: design:164「一覧は Rust の定数（`export::NO_COORD_SOURCES`）。足すのは、そのソースの payload に座標の項目が無いことを収集側の型で確かめた Story」、D5b（:155）「『時刻 1 点』へ移すのは、そのソースの記録が出来事 1 点であることを収集側の型で確かめた Story」、Risks（:389）「除き過ぎは書き出し直せば戻る」。main の改訂では、ソースは衛星が登録するもので本体のコードは変えない —— FR-61「API の変更なしに、そのソースからの記録を受け付ける」、FR-62「衛星が読む種類と書く種類（FR-88）、書く種類ごとの分類（FR-25）の宣言を要求する」（緯度経度・幅の宣言は無い）、NFR-24「登録簿の行・種類と中身の形の登録…だけで」。衛星のソース（家計簿・読書ログ）は書く間隔が長いので (c) の幅が日の単位になり、その間に本人が消した滞在が 1 つあれば記録ごと書き出しから落ちる。書き出し直しても戻らず、戻すには衛星ごとに本体のコードを変えることになる
- 直すところ: 新しい Story か、凍結した Story への申し送り（ST38 / ST39 の登録の宣言に「緯度経度を持つか・幅の読み方（終わりの列 / 時刻 1 点）」を足し、ST33 の判定が宣言を読む。宣言の無い衛星は今の既定（除く側）のまま）
- 機械: 安く捕まえられる（ST43 の 3 つの衛星の試験に「消した滞在のある期間に書いた衛星の記録が書き出しに入る」を 1 本足せば落ちる）
- 行き先: handoff ST38（登録の宣言に「緯度経度を持つか・幅の読み方」を足し、ST33 の判定が宣言を読む。宣言の無いソースは今の既定（除く側）。除き過ぎは書き出し直しで戻る）

## C9. ST33 の spec と deep.md は、S-7 に後から足す節を「感度（ST24）・プラグイン（ST29）」と書く。main の改訂で印は S-5、プラグインは衛星、承認は ST38 になった
- 重大度: 低
- 文書: openspec/changes/st33-data-export/specs/data-durability/spec.md:497
- 要件: PERM-2, PERM-11, FR-77
- 根拠: spec:497「ソース・停止（ST15）・定期取得（ST13）・感度（ST24）・プラグイン（ST29）の節は、それぞれの Story が同じ画面に足す」、deep.md:100 / :114 も同じ、deep.md:157（C5）「AI（ST27）やプラグイン（ST29）の出口とは別」。`docs/ui-direction.md:39`「1 件ごとの『出さない』の印は 1 件を見る S-5 に置く」、:23 の S-7「衛星の承認（衛星 × 種類 × 読む / 書く）・資格情報の発行・衛星の取り外し・書き出し」、ST38 の完了の判定（stories.json:903）「設定の画面で衛星ごと・種類ごとに読む / 書くを承認でき」
- 直すところ: その文書（spec:497 を「ソース（ST15）・定期取得（ST13）・衛星の登録と資格情報（ST29）・承認（ST38）」に。経緯ではなく本文なので「プラグイン」を残さない）
- 機械: 安く捕まえられる（走っている change と正典の本文で、改名済みの語（`プラグイン`）を ★ の引用の外から grep で数える）
- 行き先: handoff ST33（合意済みの spec は変えず、下流が画面の文言と説明書で新しい呼び名を使う。正典の言い回しは ST24 が直す）

## C10. `docs/ui-direction.md` の S-7 の Story の欄に、承認の画面を作る ST38 が無い
- 重大度: 低
- 文書: docs/ui-direction.md:23
- 要件: PERM-11, PERM-12, FR-97
- 根拠: ui-direction:23「S-7 設定 | … 衛星の承認（衛星 × 種類 × 読む / 書く）・資格情報の発行・衛星の取り外し・書き出し（★ 2026-10-11） | ST13, ST15, ST29, ST33 | … PERM-11, PERM-12, …」、:40「S-7 の ST29 は衛星の登録・承認 … に書き直す見込みで、確定は `/stories`」。`/stories` は済み、承認は ST38（stories.json:903「設定の画面で…承認でき、…『取り消しても、持ち出された写しは戻らない』と出ている」= PERM-12）。`story_brief.py --screens` は Story の欄を読むので、ST38 のブリーフに S-7 が出ない
- 直すところ: その文書（ui-direction:23 の Story に ST38 を足し、:40 の「見込み」を確定に直す）
- 機械: 安く捕まえられる（ui-direction の表の要件の列に挙がる ID を satisfies に持つ Story が、Story の欄に無ければ warn。PERM-11 / PERM-12 → ST38 が当たる）
- 行き先: docs（ui-direction の S-7 の Story の欄に ST38 を足した）

## C11. 「後に消した記録の件数」は本人の削除を `rebuild:` 以外の印で読む。衛星が自分の記録に付ける削除の印（FR-64）も本人が消したものとして数える
- 重大度: 低
- 文書: openspec/changes/st33-data-export/design.md:364
- 要件: NFR-16, FR-64, FR-50
- 根拠: design D17「本人の削除の印がある（`deleted_by IS NULL OR deleted_by NOT LIKE 'rebuild:%'`。`Mark::of` と同じ読み方）」、spec:409「いまは本人が消した場面に当たる記録の件数」。FR-64 ★ 2026-10-11「衛星の書き込みを…その衛星が書いた記録の…削除（FR-50 の印）に限定する」で、ST39 の完了の判定「消したことにもできる」。衛星が付けた印の `deleted_by` の形は決まっておらず、`rebuild:` でなければ「この書き出しの後に消した記録が N 件」に入る。除く判定（NFR-16「削除の印の付いた記録」）は書き手を問わないので、件数の文だけがずれる
- 直すところ: 凍結した Story への申し送り（ST39 が衛星の削除の印の `deleted_by` を決めるときに、D17 の読み方に足すかを決める）
- 機械: 安く捕まえられる（ST39 の試験で衛星が消した記録が `erased_after` に入らないことを 1 本）
- 行き先: handoff ST39（衛星の削除の印の `deleted_by` を決めるときに、ST33 の D17 の数え方から外すかを決める）

## K1. design の「いまの形」の環境変数の一覧に、main が足した移行の退避の変数が無い
- 重大度: 低
- 文書: openspec/changes/st33-data-export/design.md:20
- 要件: —
- 根拠: design:20「環境変数 | `DATABASE_URL` / `API_TOKEN` / `WEB_PASSWORD` / `WEB_SESSION_MAX_AGE_DAYS` / `BIND`、`ASHIATO_SELFTEST_PANIC`」。コードには `DATABASE_OWNER_URL`（`crates/server/src/lib.rs:244`）・`MIGRATE_BACKUP` / `MIGRATE_BACKUP_DIR` / `PG_DUMP`（`crates/server/src/migrate_backup.rs:27-33`。2026-10-11）がある。サーバの起動（`run`）が読むのは一覧のとおりで、足りないのは `migrate` の側
- 直すところ: leave（書き出しは `run` の側で、`ASHIATO_EXPORT_DIR` の並べ方の参考にするだけ。C3 を直すときに `migrate_backup.rs` を参照に足せば足りる）
- 機械: できない
- 行き先: leave: 書き出しは `run` の側で、一覧は `ASHIATO_EXPORT_DIR` の並べ方の参考にすぎない。権限は C3 の申し送りで `migrate_backup.rs` を参照に足した


## 未確認

- 衛星の保管場所（FR-87）とモノの台帳（FR-86）の中身に座標と時刻が入ったとき（旅程など）、NFR-16 の「本人が消した滞在の時間に入る緯度経度を持つ行（ソースを問わず）」を記録の表の外の行にも当てるか。器がまだ無く、ST40 / ST46 の上流の形を見ていない
- `production-prep.md:74` の「ビューにだけ SELECT を出す」強制を衛星の読み出しの門の Story が採ったとき、ST33 が素の `core.event_version`（design:119）と削除済みの行の `id`（D17）を読めるか。どの Story が採るかが決まっていない
- Windows の `%LOCALAPPDATA%` の既定の ACL が、同じ PC の別の利用者から書き出しを読めなくしているか（Windows の環境で確かめていない）
- ST29 の資格情報の実装が `authorize` を広げる形になるか、別の関数になるか（ST29 は未着手。C4 の「同じ扱い」が衛星の資格情報を通すかはここで決まる）

## 触らなくてよいと判断したもの（理由つき）

- `docs/stories/stories.json:775`・`ST33.md`・`INDEX.md:14,56,248-250` —— requires に ST22・layer 3 は NFR-16 の改訂（消した滞在を読む）と合い、`check_chain.py` も通る。完了の判定「全記録を書き出し」は NFR-16 の本文で読む（前の監査と同じ）
- `openspec/specs/record-deletion/spec.md:60` —— 「書き出し（NFR-16）ができたときに外へ出る」は理由の文で、改訂は除く範囲を広げる側。食い違わない
- ST33 の design の「いまの形」（design:11-22）の事実 —— `GET /events` の 6 列と `fetch_all`（`lib.rs:2164-2186`）、`core.event_live` の最新の定義（`202609120941_event_columns.sql:21`）、`core.event_version_live` の列（`202609120943_version_and_ledger.sql:78-88`）、`core.stay_erased` の列（`202609271716_deletion_ledger.sql:56-66`）、`mark_late_arrivals` が非公開（`stay_store.rs:626`）、`LOCK_KEY`（:15）、`Mark::of`（:258）、`access_log::middleware`（`lib.rs:2260`）、本番のコードの `tokio::spawn` 0 件（試験だけ）、`grants.sql` の default privileges、`Root.tsx` の行き先、`MIN_TARGET_PX = 24`（`tokens.ts:185`）、`web/e2e/offsite.spec.ts` の `VIEWS`、`contract.rs` の `range_end` —— どれも head のコードと一致した
- 画面の文言の置き場（main の `56d45374`。`messages.ts` と eslint の `no-restricted-syntax`）を design D13 が挙げていないこと —— tasks の最後の検証が `npm run lint` を走らせるので、直書きは機械で落ちる
- UI の表面の改訂（ui-direction ★ 2026-10-12）—— ST33 の画面は色を `tokens.ts` の値だけで作る（tasks 8.3）ので、表面を直した `tokens.ts` にそのまま従う。「書き出す」は NFR-20 の挙げる操作（物理削除・収集の停止）ではないので 24 px のままでよい
- `check_scenarios.py` の担保なし 85 件 —— 全部 ST33 の change で、tasks.md:18 の「85 本」と一致する。下流が未着手のため
- ST08 / ST12 / ST21 / ST25 —— 前の監査（`2026-10-12-st33-nfr16-consistency.md`）の C2 は design D5b の `DURATION_KEYS`（`visit_duration_us`）と (c) で閉じている。ST21 の `s01-place` と ST12 の `c03-*` は design が名前を挙げて「緯度経度を持つ」側に置いている。ST25 は書き出しの範囲の判定に関わらない
- check_chain の FAIL（NFR-16 の改訂が整合の確認より新しい）—— この監査が受ける
