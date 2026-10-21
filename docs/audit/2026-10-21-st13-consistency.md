# 整合の確認（2026-10-21・ST13 の深掘り第 1 回を FR-15 へ戻した改訂）

- 基準: `08494ec3f869e00f3491d230dacd2ed912808750`..`848ecd5494ff723566e9dde9764ffb0c4f3dff83`

対象は ST13（`openspec/changes/st13-scheduled-account-fetch/`）の深掘り第 1 回の答え（Q1 / Q3 / Q6。`deep-answers-1.txt`）を要件へ戻した改訂。FR-15 の ★ 2026-10-21 と、
再生成した `docs/stories/ST13.md` の逐語、st13 の `deep.md` / `agreement.json` / `deep-answers-1.txt` を見た。文書は head（`848ecd54`）の内容で読んだ。

機械の入力:
- `req_impact.py --base 08494ec3`: 改訂 1 件（FR-15）・扉 0 項。名指しは `docs/stories/INDEX.md:36`・`docs/stories/stories.json:312`・`docs/ui-direction.md:23`・走っている st13 自身の文書
  （`deep-questions.json` `deep.md` `design.md` `premises.md` `proposal.md` `review/deep.md` `review/spec.md` `specs/external-ingestion/spec.md`）
- `check_chain.py`: 未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった: FR-15」（この監査が受けるもの）

走っている change は `st13-scheduled-account-fetch`（この改訂の出どころ）と `st09-media-capture`。st13 の食い違いは C9 に**印を付けるだけ**にした。
失われるもの（uncaptured / discarded / exported / rewrite-all）を伴うものは無い（Q3 で変わるのは表示の置き場だけ）。st09 はこの改訂の影響を受けない（末尾）。

## C1. 格子に取り方と間隔を添えると Must の 5 ソースの高さの予算に効くが、FR-15 ★ は予算に触れていない。予算は実寸で既に超えている
- 重大度: 高
- 文書: docs/requirements.md:312
- 要件: FR-15, FR-55, FR-54, NFR-13, NFR-19
- 根拠: FR-15 ★（:312-313）「画面では設定ではなく**稼働状況の格子の各行に読むだけで添える**（書庫のソースは間隔の代わりに「手作業・2 か月ごと」と通知までの残り日数）」—— 書庫だけでなく全ソースが対象で、Must の 5 本（位置・アプリ利用・写真・ウィンドウ・ブラウザ履歴）の格子にも文字が増える。
  一方 FR-55 ★（:743）「Must の 5 ソースのひとスクロールと開いた直後の 2 ソース（ST02 の深掘り 第 8 回 Q30）は、**箱の高さを除いて数える**（除く高さの上限 160 CSS px は仮。ST12 の design D12）」で、除いてよいのは「直近に置いた書庫」の箱だけ。
  正典 `collection-coverage`（spec.md:753-754）「Must の 5 ソースすべてが、**ひとスクロール以内**に見えるようにする」「この 2 つの高さを、…『直近に置いた書庫』の箱の高さ（160 CSS px を上限として）を除いて数える」。同じ予算を、集計の格子（spec.md:1742）と書庫の格子（`external-ingestion` spec.md:760）は「超えないようにする」と明示して受けているが、FR-15 ★ には同じ一文が無い。
  予算は実寸で既に割れている —— issue #75（OPEN）「Must の 5 本だけで 2 画面ぶん（1,280 CSS px）を超えている」「土台は約 1,317 px」、ST12 の design D20（`openspec/changes/archive/2026-10-13-st12-archive-ingestion/design.md:430-432`。2 本目の下端 846 px で 800 を超える）と D23（同 :443）。ST02 の design（`openspec/changes/archive/2026-09-14-st02-collection-coverage/design.md:735`）「**いまの余地は 44 px（3.4 %）**…画面に何かを足すときは、この検査が先に落ちる」。
  5 本の格子に 1 行（本文の行の高さ 15.5 px × 1.7 ≈ 26 px。`CoverageGrid.tsx:149` のコメント）ずつ足せば約 130 px で、2 本目の 800 px にも 2 本ぶん効く。見出しの行（`CoverageGrid.tsx:58-82` の `h2`）に並べても、360 px 幅では「携帯端末のアプリ利用 — 30 分ごと」が折り返しうる。
  ST13 の「壊してはいけないもの」（`docs/stories/ST13.md`・stories.json の `breaks`）は「ST12 の取り込み経路と最終日の表示」だけで、ST12 の `breaks` にある「ST02 / ST04 の稼働状況の画面（…Must の 5 ソースがひとスクロール以内に見えること）」を持っていない
- 直すところ: requirements（FR-15 ★ に、添える文字を予算に数えるか・どこに置くか —— 見出しの行に入れて行を増やさない、Must の 5 本には添えない、予算を置き直す —— を書く。予算が既に割れていること（#75）を前提にするなら本人に問う）と、凍結した Story への申し送り（ST13 の `breaks` に ST02 / ST12 の予算を足す）
- 機械: 安く捕まえられる（`web/e2e` で Must の 5 本目と 2 本目の格子の下端を本物の Chromium で測る。いまは jsdom の勘定だけで、文字の高さと折り返しを数えない —— K1）
- 行き先: requirements（FR-15 の★ 2026-10-21 補強に、添える文字を予算に数え格子の高さを増やさない既定を書いた）と stories（ST13 の `breaks` に ST02 / ST12 の予算を足した）。置き方の細部は st13 の `spec --answers Q3`

## C2. 「格子の各行」は、いまの稼働状況では週の行を指す。Q3 の proto は格子を 1 ソース 1 行の横長に描いており、本人は実物と違う形を見て選んでいる
- 重大度: 高
- 文書: docs/requirements.md:312
- 要件: FR-15, FR-54
- 根拠: FR-15 ★（:312）「稼働状況の**格子の各行**に読むだけで添える」、★ の引用（:314）「稼働状況の格子の各行に添える」。
  `docs/ui-direction.md:227-228`「第 6 回 Q25 で格子が**縦長**になり、**行は週**を指すことになった。**ソースは行ではなく格子で分かれる**」。FR-54 ★（requirements.md:727）「縦長（1 行 = 1 週）」。正典 `collection-coverage`（spec.md:748）「1 行が 1 週（7 日）で、上から下へ 53 週」。実装も同じ（`web/src/CoverageGrid.tsx:26`「縦長（1 行 = 1 週…）」、1 ソースは `<section>` 1 つで、中に週の帯 `WeekRow` が 4 本以上）。
  本人が Q3 で触った proto（`openspec/changes/st13-scheduled-account-fetch/proto.html:141-148` の `coverageView`。`docs/briefs/ST13-proto.html` と `ST13-deep-Q3-proto.html` も同じ）は、1 ソースを**横 1 行に 53 セル（360 px では 20 セル）**並べ、その下に 1 行の注記を置く形 —— 第 6 回 Q25 が NFR-19 のために退けた横長の形で、ひとスクロールの勘定も出していない。
  同じ proto は格子に無いソース（`c01-clock`・`s01-archive-inbox`）も行として描いている（C3）。逐語の「各行」を週の行と読むと、53 週の帯ごとに添えることになり、本人の意図（ソースごと）と食い違う
- 直すところ: requirements（「各行」を「各ソースの格子（見出しの横か下）」に言い換える）。実物の縦長の格子で見え方が変わるなら、st13 の `spec --answers Q3` の前に本人に確かめる
- 機械: できない（proto の描いた形が実物の形と同じかは読み比べ）。語の側は安く捕まえられる（要件の本文で「格子の各行」「格子の行」をソースの意味で使っていないかを grep する）
- 行き先: requirements（FR-15 の★ 2026-10-21 補強で「各行」を各ソースの格子の見出しの行と読んだ。表示だけで計算し直せば戻るので仮で進め、GO 資料の合意で本人が見る）

## C3. 格子に出ているのは Must の 5 本・集計・書庫だけ。FR-15 ★ が名指しする見回りの間隔（`s01-archive-inbox`）と、後から足す定期取得のソースは、添える行が無い
- 重大度: 中
- 文書: docs/requirements.md:311
- 要件: FR-15, FR-54, FR-61
- 根拠: FR-15 ★（:311）「C-03 は定期取得の枠と**取り込み器の見回りの間隔**だけを持ち、**後のソースは登録簿に 1 行足して乗る**」、（:312）「ソースごとの取り方と取得の間隔は…稼働状況の格子の各行に…添える」。
  格子に出すソースは定数で決まっている —— `crates/server/src/lib.rs:3122-3133` が `coverage::must_sources()` の 5 本・`ROLLUP_SOURCE`・`logical_source LIKE 'c03-%'` だけを並べ、正典 `external-ingestion`（spec.md:748）も「上から Must の 5 ソース・集計・書庫のソース・退役したソースの順」とする。
  登録簿の 23 行（`migrations/*.sql` の `core.source` への INSERT）のうち `c01-clock`・`s01-archive-inbox`・`s01-stay`・`s01-attribute`・`s01-feeling`・`s01-place` の 6 本は格子を持たない。見回りの間隔は `s01-archive-inbox` の行（st13 の design D4）なので、FR-15 ★ が自分で名指しした間隔に置き場が無い。
  後から足す定期取得のソースは `c03-` 以外の前置きになる（st13 の design D2「`c04-` など」）ので、登録簿に 1 行足しても格子に出ず、取り方も間隔も画面のどこにも出ない。推奨の形（S-7 の一覧）にあった「定期取得するソースはまだありません」の空の節（proto.html:136）にあたるものも、格子には置き場が無い
- 直すところ: requirements（FR-15 ★ に、格子を持たないソースの取り方と間隔を出さないのか、どこに出すのかを書く。定期取得のソースを格子に出す条件も）。正典の並びを変えるなら st13 の spec の段で MODIFIED
- 機械: 安く捕まえられる（st13 の試験に「登録簿に `acquisition = 'scheduled'` の行を足すと、稼働状況にその行の取り方と間隔が出る」を置く。いまの `coverage_get` では落ちる）
- 行き先: leave: 表示だけの決定（登録簿から計算し直せば戻る）。格子を持たないソースと見回りの間隔をどこに出すかは st13 の `spec --answers Q3` が決める（deep.md「spec --answers Q3 へ送るもの」）

## C4. 「取り方と取得の間隔は登録簿だけが持ち」は、FR-1 / FR-2 / FR-13 と収集アプリの定数とぶつかる
- 重大度: 中
- 文書: docs/requirements.md:312
- 要件: FR-15, FR-1, FR-2, FR-13, FR-35
- 根拠: FR-15 ★（:312）「ソースごとの取り方と取得の間隔は**登録簿だけが持ち**、画面からは変えない」。
  FR-1（:101）「C-01 が位置取得の契機（**60 秒間隔**）に達する」、FR-2（:109）「アプリ利用の取得契機（**30 分間隔**）」、FR-13（:228）「ブラウザ履歴の取得契機（24 時間間隔）」は、間隔を要件の本文と収集アプリが持つ。
  コード: `collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationFix.kt:14` `const val FIX_INTERVAL_MS: Long = 60_000`、`CollectionSource.kt:19` `const val USAGE_INTERVAL_MS: Long = 30 * 60 * 1000L`。登録簿 `core.source` が持つのは `expected_gap_sec`（FR-35 の想定間隔。位置は 6 時間）・`alert_after_days` などで、取り方と取得の間隔の列は無い（`migrations/202609081618_envelope.sql:6-11` と以後の `ADD COLUMN`）。
  走っている st13 の design D9（仮）は「収集側（C-01 / C-02）は自分の定数で動き、登録簿の値を読まない。登録簿の値は S-7 に出すための**写し**」で、FR-15 ★ の「だけが持ち」と逆。写しは収集アプリの定数を変えても動かないので、格子に添えた「60 秒ごと」が実際と食い違っても誰も気づかない
- 直すところ: requirements（FR-15 ★ を「サーバが回すもの（取り込み器の見回り・定期取得）の間隔は登録簿だけが持つ。端末と PC の間隔は FR-1 / FR-2 / FR-13 と収集アプリが持ち、登録簿は表示のための写し」のように分ける）
- 機械: 安く捕まえられる（登録簿の写しと収集アプリの定数を突き合わせる試験。既存の `expected_gap_seeded` が定数と登録簿のずれを CI で止めている形と同じ —— `crates/server/src/coverage.rs:635` のコメント）
- 行き先: requirements（FR-15 の★ 2026-10-21 補強で、サーバが回す取得は登録簿の間隔に従い、端末と PC の間隔は端末が決めて登録簿が写すと分けた）

## C5. ST13 の表題・価値・完了の判定 1 は、改訂で消えた前提（定期取得するアカウント系のソースがある）を固定している
- 重大度: 中
- 文書: docs/stories/stories.json:312
- 要件: FR-15
- 根拠: stories.json の ST13「title": "**アカウント系のソースを定期取得する**"」「value": "本人として、**取れるものは自動で取ってほしい**」「done": "ソースごとの取得間隔を登録簿で設定でき、**その間隔で自動取得が走る**"」（`docs/stories/INDEX.md:36`・`docs/stories/ST13.md` も同じ）。
  改訂後の FR-15 ★（:310-311）「**いま要件の中に、API で定期取得できるアカウント系のソースは無い**」「C-03 は定期取得の枠と取り込み器の見回りの間隔だけを持ち」。完了の判定 1 は本番の登録簿では観測できず、試験用の行（st13 の design D4「試験は自分専用の行」）でしか満たせない。
  逆に、FR-15 ★ が足した画面の義務（格子に取り方と間隔を添える）は完了の判定に無い。`judged` は 2026-09-15 で、Q1 / Q3 の答えより前
- 直すところ: 凍結した Story への申し送り（ST13 の完了の判定 1 を「定期取得の枠が試験の行で間隔どおりに走り、0 本でも起動する」と「稼働状況に取り方と間隔が出る」に読み替えることを、st13 の archive で `/stories` へ返す）か、stories（表題と完了の判定を書き直す）
- 機械: 捕まえられた（`req_impact.py` が stories.json:312 を名指し）。前提の消失そのものは `check_chain.py` の warn 10 に出ていない（★ が「判断より新しい訂正」に数えられていない）
- 行き先: stories（stories.json の ST13 の表題・完了の判定 1・breaks・judged を直して再生成した）

## C6. `docs/ui-direction.md` は ST13 と FR-15 を S-7 設定に割り当てたまま。S-1 の行に ST13 が無く、`story_brief.py --screens` は ST13 の面を「S-7 設定」と返す
- 重大度: 中
- 文書: docs/ui-direction.md:23
- 要件: FR-15, FR-53
- 根拠: `docs/ui-direction.md:23`「| S-7 設定 | **ソース**・停止・… | **ST13**, ST15, ST29, ST33, ST38 | FR-53, **FR-15**, …」、:16「| S-1 稼働状況 | どのソースがいつ動いていたか | ST02, ST04, ST10, ST12, ST14, ST15 | FR-54, FR-55, FR-9, FR-34 |」。
  改訂後の FR-15 ★（:312）「画面では**設定ではなく**稼働状況の格子の各行に…添える」。`python3 scripts/story_brief.py ST13 --screens` の出力は `["S-7 設定"]`（2026-10-21 実行）で、下流の proto と画面の検査は S-7 を見に行く。
  S-7 の「ソース」はいま担い手が無い。st13 の Q3 の `why`（`deep-questions.json` の Q3）と spec（`specs/external-ingestion/spec.md:204`）は「ST15 のソースごとの停止の操作は、後から**同じ一覧の行に乗る見込み**（`docs/ui-direction.md` の S-7）」としていたが、Q3 でその一覧は作られなくなり、ST15 の停止の操作が乗る先が前提ごと消えた
- 直すところ: その文書（`docs/ui-direction.md` の S-1 に ST13 と FR-15 を足し、S-7 から ST13・FR-15・「ソース」を外すか、「ソース」を ST15 の停止の一覧として残す旨を書く）
- 機械: 捕まえられた（`req_impact.py` が :23 を名指し）。安く捕まえられる（要件の本文が「画面では…に」と面を名指ししたら、`story_brief.py --screens` の面と突き合わせる）
- 行き先: docs（`docs/ui-direction.md` の S-1 に ST13 と FR-15 を足し、S-7 から外した）

## C7. FR-15 ★ の根拠にした外の事実（Data Portability API が日本で使えない・YouTube の視聴履歴の API が無い）が、§5 の「外部世界の主張」に無い
- 重大度: 中
- 文書: docs/requirements.md:310
- 要件: FR-15, NFR-12
- 根拠: FR-15 ★（:310-311）「（Data Portability API は日本で提供されず、テストの OAuth は 7 日で切れ、YouTube の視聴履歴の API は無い。ST13 の `premises.md`）」。
  §5 の表（:1353-1369）は外の主張を一次情報・逐語・確認日・**再確認期限**つきで持つが、このうち表にあるのは EXT-A（OAuth 7 日。影響に FR-15 がある）だけ。Data Portability API の国の一覧（`premises.md` の 1 行目。確かさ 2、出所 https://support.google.com/accounts/answer/14452558）と YouTube の視聴履歴の API（同 5 行目。**確かさ 1**、旧プロジェクトの EXT-7 の引用）は表に無い。
  `premises.md` 自身が Q1 の `loss`（uncaptured）を「本人が使っていて、取得元の保持が短いアカウントのサービスがあるのに、定期取得を作らない」側に置いており、日本で提供が始まったときに気づく手段が要件の側に無い
- 直すところ: requirements（§5 に Data Portability API の提供地域の行を足し、影響に FR-15 / NFR-12 を書く。YouTube の視聴履歴の API は一次情報を取り直して足すか、★ から外す）
- 機械: 安く捕まえられる（表の再確認期限を読む検査はいま無い（`scripts/` `tools/` に「再確認期限」を読むものが 0 件）。足せば、表に載った主張の期限切れを機械が見る）
- 行き先: requirements（§5 に EXT-N を足し、YouTube の視聴履歴の API の話は FR-15 の★から外した）

## C8. 書庫のソースの見出しに、起点の違う 2 つの日数が並ぶ（「N 日前」と「通知まで あと N 日」）
- 重大度: 低
- 文書: docs/requirements.md:313
- 要件: FR-15, FR-55, FR-16, FR-35
- 根拠: FR-15 ★（:313）「書庫のソースは間隔の代わりに『手作業・2 か月ごと』と**通知までの残り日数**」。FR-55（:739）「書庫のソースも稼働状況の格子として出し、**見出しの横に最終日と何日前か**を添える」、正典 `external-ingestion`（spec.md:749）「『`YYYY-MM-DD` まで（N 日前）』の形」。
  「N 日前」は FR-16 の最終日（いちばん新しい出来事の日）から、残り日数は FR-35 の「そのソースを運んだ最後の書庫」の作られた時刻 + 75 日から数える。FR-35 ★（:472-473）はこの 2 つを「別物（C9）」と分けている。同じ見出しに並ぶと、使わなかった製品では「40 日前」と「あと 70 日」のように足して 75 にならない組が出る
- 直すところ: その文書（st13 の spec で、残り日数の文字に起点（最後の書庫）を添える）
- 機械: できない（読み手が取り違えるかは読み比べ）
- 行き先: leave: 表示の文言で計算し直せば戻る。st13 の `spec --answers Q3` が残り日数の起点を添える（deep.md「spec --answers Q3 へ送るもの」）

## C9. 走っている st13 の spec・design・proposal・tasks・go.json・proto・ブリーフは S-7 設定の「ソース」節で書かれている（印だけ）
- 重大度: 中
- 文書: openspec/changes/st13-scheduled-account-fetch/specs/external-ingestion/spec.md:188
- 要件: FR-15
- 根拠: spec :188「### Requirement: **S-7 設定の「ソース」の節に**、登録簿の全ソースの取り方と間隔が読むだけで並ぶ」と Scenario 14 本（:208-278）、:171「2 か月ごとの書き出しが遅れていることは、**S-7 のソースの一覧**の通知までの残り日数で見える」。`specs/device-collection/spec.md` と spec :263「休止の対象から外れていないことが**端末の節に**出る」（Q4 の表示の置き場も S-7 の一覧の中）。
  design の D10（仮）、proposal :25 / :27 / :40 / :53 / :60、tasks :21 / Task 9（:109）/ :128、`go.json`（S-7 が 11 か所）、`proto.html`（7 か所）、`docs/briefs/ST13-pr.md`（3 か所）。
  `agreement.json` は `"head": "08494ec3…"`・`"picked": "合意する"` で、S-7 の形の spec に対する合意（deep.md「`spec --answers Q3` の後に合意を引き継げるかをグラフが決める」）。失われるものは無い（表示の置き場だけ）
- 直すところ: leave: st13 の `spec --answers Q3` が直す（C1〜C3 と、Q4 の休止の表示の置き場もそこで受ける）
- 機械: 捕まえられた（`req_impact.py` が st13 の文書を名指し）
- 行き先: leave: st13 の `spec --answers Q3` が直す（C1〜C3 と、Q4 の休止の表示の置き場もそこで受ける）

## K1. 正典の画面の Scenario「ひとスクロールで 5 ソースすべてが見える」「開いた直後に 2 ソース以上の直近 1 か月が同時に見える」の印は jsdom の勘定だけにあり、本物のブラウザでは測っていない。実寸では成り立っていない
- 重大度: 中
- 文書: openspec/specs/collection-coverage/spec.md:872
- 要件: FR-55, FR-54, NFR-13, FR-15
- 根拠: 正典 :867-874 の 2 本の Scenario（360 × 640 CSS px で 640 / 1,280 CSS px に収まる）。印は `web/src/__tests__/one-scroll.test.tsx:75` / `:86` / `:113` だけで、`web/e2e` には無い（`grep -rn "Scenario: ひとスクロールで 5 ソース" web/e2e` が 0 件）。
  そのファイルの冒頭（:16-17）「jsdom は実寸を測らないので、これは**指定の勘定**であって実寸ではない。実寸は `tasks.md` の『人間の確認待ち』が持つ」。`AGENTS.md`「**画面の Scenario は `web/e2e`（本物のブラウザ）で担保する。** jsdom へも『人間の確認待ち』へも逃がさない」。
  実寸は割れている —— `web/e2e/archive-layout.spec.ts:141-142`「土台の 1,280 px は画面の表面（2026-10-12）で既に超えていて」、:159 `test.fail(true, "D20（仮）: ST02 の 640 px の土台が実寸で超えている（followup ST02）")`、issue #75（OPEN）。jsdom の勘定は緑のままなので、FR-15 ★ で格子に文字を足しても（C1）落ちる検査が無い
- 直すところ: コード（`web/e2e` に Must の 5 本目と 2 本目の下端を測る試験を置く。いまは成り立たないので #75 が直るまで `test.fail` で固定する）。予算の数そのものは #75 の側
- 機械: 安く捕まえられる（画面の正典の Scenario の印が `web/e2e` にあるかを、正典の capability ごとに機械で見る。`collection-coverage` と `external-ingestion` の Scenario は全件どこかに印がある —— 2026-10-21 に全件を grep した）
- 行き先: leave: 格子に文字を足すのは st13 なので、st13 の `spec --answers Q3` が tasks に `web/e2e` の実寸の試験を置く（#75 が直るまでの扱いも）（deep.md「spec --answers Q3 へ送るもの」）

## 未確認

- 取り方と間隔を見出しの行に入れたとき、360 px で何本が折り返し、Must の 5 本目と 2 本目の下端がいくつ下がるか（測っていない。C1 の約 130 px は 1 行 ≈ 26 px × 5 の見積もり）
- Data Portability API の提供地域がいまも EU・スイス・英国だけか（`premises.md` の 2026-10-20 の確認を写しただけで、取り直していない。C7）
- Q3 の proto で本人が「指示をコピー」した出力の全文（`deep-answers-1.txt` は `(playground の出力)` と補足だけを持つ）。「各行」がソースの意味かは補足の文と proto の描き方から読んだ（C2）

## 触らなくてよいと判断したもの

- `docs/requirements.md:52` の C-03 の「書庫の解析とアカウント系の定期取得」: FR-15 ★ も「定期取得の枠」を C-03 に残しているので食い違わない
- 見回りの間隔がいま環境変数 `ASHIATO_ARCHIVE_SCAN_SEC`（`crates/server/src/archive/config.rs:28`）で、登録簿に無いこと: FR-15 ★ は ST13 がこれから満たす義務で、st13 の design D4 が登録簿へ移す。事実の主張ではない
- NFR-3 / NFR-12 の逐語（ST13.md に写っている）: FR-15 ★ の「手作業・2 か月ごと」と同じ周期で、食い違わない
- FR-15 ★ の「ST13 の `premises.md`」: archive で `openspec/changes/archive/…/premises.md` に移るが、ほかの ★ も同じ形で change の文書を指している（ST12 の `deep.md` など）
- 走っている `st09-media-capture`: 足す 3 本（`c01-screenshot` / `c01-image` / `c01-video`）は格子に出ないソースで、FR-15 ★ の画面の置き場の影響は C3 に含まれる。st09 の文書は FR-15 を名指ししていない
- `check_chain.py` の FAIL「整合の確認の後に要件が変わった: FR-15」: この監査が受けるもの
