# 整合の確認 — ST53（許諾の同梱）を、同梱の PostgreSQL の改訂の入った main へ rebase した組み合わせ（2026-11-03）

- 基準: `796bb3685e56aecbb2f3fec3305bc59d7a4f5cc0`..`6c84c86752546890482bac0f81c831e38425b028`

見る向きは 2 つ。(1) ST53 が改訂した ID（FR-127 の ★ 補足群・§5 の「V-01 はソースの場所と許諾を出さない」・C-01 の許諾・K1 の訂正）と main の新しい文書（ST55・ST59・ST60・handoff ST55・EXT-S・release.md §7・production-prep.md）、
(2) main が改訂した ID（FR-106・FR-108・FR-109・FR-111・FR-112・FR-131・FR-132・FR-133・NFR-25）と ST53 の成果物。
既に処置済みの監査（`2026-11-02-st53`・`2026-11-03-st53-r2`・`2026-11-03-st53-r3`・`2026-11-03-native-postgres`）の決着を前提にする。とくに native-postgres の C3（本人の環境の切り替えは ST55 の後。行き先は requirements と release.md §7）と C12（同梱の PostgreSQL の許諾は ST55 が FR-127 の一覧と NFR-26 の検査に足す。行き先は EXT-S と handoff ST55）は出し直さない。

## 機械の入力（写し）

`/tmp/st53-impact-a.txt`（`req_impact.py . --base 796bb368`。ST53 が改訂した ID）:

- FR-127（改訂）: `docs/handoff/ST55.md:18`・`docs/stories/INDEX.md:77, 374`・`docs/stories/stories.json:1260`・ST53 の change の中（deep.md・design.md・proposal.md・tasks.md・specs/distribution/spec.md ほか）

`/tmp/st53-impact-b.txt`（base は rebase 前の ST53 の head `dac01ae3`。main が改訂した ID）:

- FR-106（改訂）: `docs/handoff/ST55.md:22`・`INDEX.md:78, 523`・`stories.json:1286`・`openspec/specs/distribution/spec.md:734`
- FR-108（改訂）: `docs/handoff/ST55.md:5, 11, 14, 21`・`docs/production-prep.md:48`・`docs/release.md:82, 84`・`INDEX.md:79, 519`・`stories.json:1308`
- FR-109（改訂）: `docs/handoff/ST55.md:5, 10, 12, 15`・`INDEX.md:79, 519`・`stories.json:1309, 1399`
- FR-111（改訂）: `docs/handoff/ST55.md:20`・`INDEX.md:79`・`stories.json:1311`
- FR-112（改訂）: `docs/handoff/ST55.md:21`・`INDEX.md:81, 523`・`stories.json:1360`
- FR-131・FR-132・FR-133（新規）: INDEX.md・stories.json（FR-133 は `docs/handoff/ST55.md:5` も）
- NFR-25（改訂）: `INDEX.md:76, 332, 379`・`stories.json:1234`・`openspec/specs/distribution/spec.md:535`
- ST53 の成果物（`openspec/changes/st53-license-bundle/`・`docs/stories/ST53.md`・`docs/handoff/ST53.md`）は、この 9 件のどれも名指ししていない

`python3 scripts/check_chain.py .`（chain: FAIL 1 件 / 未回収 0 件 / warn 0 件）:

- [FAIL] 整合の確認: 整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-106, FR-108, FR-109, FR-111, FR-112, FR-131, FR-132, FR-133, NFR-25（この監査がそれに当たる）

## 1. 改訂後の要件と、ほかの文書

## C1. ST53 の成果物が「本人のサーバの切り替え」を「ST53 が main に入った後に本人が行う」に束ねたままで、FR-108 ★ 2026-11-03 の「切り替えは ST55 の後、インストーラで」とぶつかる。ST52 の完了の判定は rebase の中で新しく同じ誤りを持った
- 重大度: 高
- 文書: docs/stories/stories.json:1252（= docs/stories/ST52.md:60）
- 要件: FR-108, FR-109, FR-127
- 根拠: stories.json:1252（ST53 の record が Q4 に合わせて書き直した ST52 の done）「本人がサーバをその版へ移す手順が `docs/release.md` にある（切り替えるのは ST53 が main に入った後の最初の本物の公開の後。…）」。docs/handoff/ST53.md:20「本物の印の公開、正式な鍵での署名、本人のサーバの切り替えは、ST53 が main に入った後に本人が行う（Q13 / D22。★ 2026-11-02 ST53 の深掘り Q4 …）」。openspec/changes/st53-license-bundle/design.md:27（Non-Goals）「最初の本物の印・正式な鍵を置くこと・本人のサーバの切り替え（Q4。ST53 が main に入った後に本人が `docs/release.md` の手順で行う）」。proposal.md:35-36「**最初の本物の公開は、ST53 が main に入った後に本人が行う** … 本物の印・正式な鍵・本人のサーバの切り替えは、`docs/release.md` の手順で本人が行う」。deep.md:23 Q4 の問い「…本人のサーバをその版へ移すのを、ST53 の確認バッチの中で見ますか、ST53 が main に入った後に本人が行いますか」。docs/release.md:86（§7 の旧い手順の段落。rebase で「ST53 の確認後」を書き換えた）「ST53 が main に入り、正式版の公開が完了してから本人が行う。まず現在のサーバを止め…」。対して改訂後の requirements.md:1001「**本人の環境を配った版へ切り替えるのは、ST55 が済んでから、そのインストーラで入れて行う**（Q4）。ST53 の後に Docker の DB のまま切り替えない」・release.md:83 同旨。native-postgres の C3 の行き先は requirements と release.md §7 だけで、ST52 の done は直していない（当時の done は「ST53 の確認の後」で、その後 ST53 の rebase が別の誤った文に書き換えた）
- 失われるもの: なし（文書だけ。ただし文書どおり ST53 の後・ST55 の前に切り替えると、配った版の本人の記録が Docker の DB に溜まり、移す口が無い —— native-postgres C3 と同じ帰結）
- 直すところ: requirements の後の /stories（凍結した ST52 の done の括弧を「切り替えるのは ST55 の後、そのインストーラで」に。`stories.json` → `make_story.py`）・その文書（docs/handoff/ST53.md:20 の「本人のサーバの切り替え」を外すか ST55 の後と書く。release.md:86 は「以下は旧い手順」の中なので書き換えた語を戻すか、§7 冒頭の ★ に従う旨で足りる）・走っている ST53 の design.md:27・proposal.md:36 は印だけ（Q4 の本人の答えは書き換えない。切り替えの部分だけが FR-108 ★ 2026-11-03 r3 Q4 に上書きされた）
- 機械: できない（どの文も FR-108 を名指ししないので req_impact.py に出ず、check_chain.py の warn 10 も ST52 の判断の日を見るだけ）
- 行き先: stories（ST52 の done の括弧を「切り替えるのは ST55 が済んでから、そのインストーラで」に書き直し、make_story.py で ST52.md を作り直した）。handoff ST53 にも ★ を足した。design.md:27・proposal.md:36 は印だけ（Q4 の答えは書き換えない）。release.md:86 は §7 冒頭の ★ の下の「旧い手順」なので触らない

## C2. requirements.md の FR-108 ★ が `docs/release.md` §7 を番号で名指しするが、ST53 の tasks 10.2 は §3〜§5 を組み替えて以降の節を 1 つ繰り下げる。10.2 は §7 の ★ 2026-11-03 の段落を保つことも言っていない
- 重大度: 中
- 文書: docs/requirements.md:1001
- 要件: FR-108
- 根拠: requirements.md:1001「ST53 の後に Docker の DB のまま切り替えない（Q5 の「移さない」と合わせる。`docs/release.md` §7）」。openspec/changes/st53-license-bundle/tasks.md:213-217「10.2 `docs/release.md` を書き直す … §3「エージェントの資格情報」… §4「ルールセットと承認者」… §5「鍵と Secrets」: ST52 の §3 の中身」（いまの §3 Environment と Secrets・§4 ルールセットの 2 節が 3 節になる）。design.md:186 は既に繰り下げた後の番号で「`docs/release.md` §6「最初の印」」と書く（いまの release.md:42 は「## 5. 最初の印と開発版の公開」）。繰り下げると「## 7. 本人の環境を移す」（release.md:80）は §8 になり、要件の「§7」は「壊れた版と修正の枝」を指す。10.2 の箇条（tasks.md:214-222）に §7 の ★ 2026-11-03（release.md:82-84。rebase 前の ST53 が知らなかった段落）への言及は無い
- 失われるもの: なし
- 直すところ: 走っている ST53 の tasks 10.2 は印だけ（下流が release.md を書き直すときに、requirements.md:1001 の参照を節の名前「本人の環境を移す」に直すか番号を合わせ、§7 冒頭の ★ を残す）。requirements の側を節の名前で指す形にしてもよい
- 機械: 安く捕まえられる（`release.md` §<n> の参照と見出しの番号・名前の突き合わせ。いまはどの検査も見ない）
- 行き先: handoff ST53（Task 10.2 で release.md を組み替えるとき、要件の §7 の参照を合わせ、§7 冒頭の ★ を残す）

## C3. handoff ST55 の C12 は ST53 の道具（`notices.sh server`・`check-artifacts.sh` の許諾の 3 点）に足せと言うが、ST55 の requires に ST53 が無く、同じ layer 5 で先に入りうる
- 重大度: 中
- 文書: docs/handoff/ST55.md:18
- 要件: FR-127, NFR-26, FR-108
- 根拠: docs/handoff/ST55.md:18-19「同梱する PostgreSQL と zip に入るほかの部品（DLL）の許諾を、FR-127 の一覧（ST53 の `tools/release/notices.sh server`）と NFR-26 の検査（…）に足す。`tools/release/check-artifacts.sh` で、サーバの配布物に PostgreSQL があるなら `THIRD-PARTY-NOTICES.txt` に PostgreSQL の見出しがあることを見る」。stories.json の ST55: requires `['ST52', 'ST28']`・layer 5、ST53: requires `['ST52']`・layer 5（INDEX.md:76・:79 も同じ）。`tools/release/notices.sh` と配布物の `THIRD-PARTY-NOTICES.txt` は ST53 の tasks 5.1・6.1 が作るもので、いまの木には無い（`ls tools/release/` に notices.sh は無い）。ST55 の上流は答えずに止めてあり /stories の後に深掘りからやり直す（native-postgres C2）ので、ST53 より先に下流へ進むことはありうる
- 失われるもの: なし（先に ST55 が入ると、申し送りの宛先の道具が無いまま PostgreSQL を同梱した配布物が出る。許諾の一覧に PostgreSQL が無い版は、配った後に直せない —— ST53 の spec:185「配った写しに書いた場所は、後から直せない」と同じ型）
- 直すところ: requirements の後の /stories（ST55 の requires に ST53 を足す）か、その文書（handoff ST55 に「ST53 が main に入っていなければ…」の但し書き）
- 機械: 安く捕まえられる（requires に足せば hx の着手の順で止まる。足さない限りどの検査も見ない）
- 行き先: handoff ST55（ST53 が main に入っていなければ依存として取り込むか ST53 の後へ回す）

## C4. ST53 が ADDED で正典に入れる Requirement が、同梱の PostgreSQL で ST55 が広げるもの（許可の一覧・サーバの一覧の中身・配布物の形）を固定していて、handoff ST55 はそれを MODIFIED することを言っていない。Linux の D-01（ST60）には宛先が無い
- 重大度: 中
- 文書: openspec/changes/st53-license-bundle/specs/distribution/spec.md:144
- 要件: NFR-26, FR-127, FR-108, FR-131
- 根拠: spec.md:91「一覧は `MIT`・…・`Python-2.0` のまま」・:144-146 Scenario「許可の一覧は、AGPL-3.0-only を外したほかは前と同じ … 一覧はこの Requirement に並べた 17 の値とちょうど同じ」。PostgreSQL の許諾の SPDX `PostgreSQL` はこの 17 にも、いまの `tools/check-licenses.sh:9` の `ALLOW` にも無い。spec.md:169-170「`THIRD-PARTY-NOTICES.txt` の依存に、次のものを入れる。サーバ: 実行ファイルに入る Rust の部品、組み込んだ画面に入る Node の部品…、Rust の標準ライブラリ」（同梱のバイナリが無い）。spec.md:165「tar / zip では根に」（ST55 の done・FR-108 は「サーバのインストールを走らせる」で、配布物の形は決まっていない）。EXT-S（requirements.md の EXT 表）「PostgreSQL と zip に入るほかの部品の許諾は、同梱する Story（ST55）が FR-127 の一覧と NFR-26 の検査に足す」。handoff/ST55.md:18-19 は足す先を道具の名前でだけ言い、ST53 の Requirement と Scenario を MODIFIED すること・許可の一覧に足すことを言わない。FR-131（requirements.md の FR-131）は Linux の D-01 の入れ方を未決にし、ST60.md の価値・完了の判定・handoff のどれにも FR-127 / NFR-26 は無い（EXT-S の向け先は ST55 だけ）
- 失われるもの: なし
- 直すところ: その文書（handoff ST55 に「ST53 の distribution の 2 つの Requirement —— 依存の許諾の検査（許可の一覧の 17 の Scenario）と配布物の 3 点（サーバの一覧の中身）—— を MODIFIED し、許可の一覧に `PostgreSQL` ほか zip の部品の許諾を足す」を 1 行足す。ST60 には、Linux で D-01 を配布物に入れるなら同じことを足す旨を handoff で送る）。走っている ST53 は印だけ
- 機械: 捕まえられる側（ST55 が許可の一覧に足すと、ST53 の Scenario「許可の一覧は…17 の値とちょうど同じ」の自己試験が落ちる）。ただし落ちるのは ST55 の下流で、上流で MODIFIED を立てる手がかりにはならない
- 行き先: handoff ST55（ST53 の distribution の 2 つの Requirement を MODIFIED し、許可の一覧に PostgreSQL ほかを足す）。ST60 にも docs/handoff/ST60.md で同じことを送った

## C5. §5 の「V-01 はソースの場所と許諾を出さない」の末尾が、もう無い「Docker の箇条」を指す
- 重大度: 低
- 文書: docs/requirements.md:1575
- 要件: FR-127
- 根拠: requirements.md:1575「整合の確認 `docs/audit/2026-11-02-st53-consistency.md` の C5・C13 で、ソースの場所の言い方を FR-127 に合わせ、置き場を Docker の箇条の後へ移した」。いまの直前の箇条（:1568）は「**S-01 は D-01 の PostgreSQL を同梱して配り、どちらも OS のサービスとして…**」で、Docker の箇条ではない（rebase の衝突で main の箇条を取った）
- 失われるもの: なし
- 直すところ: requirements（「置き場を Docker の箇条の後へ移した」を「S-01 の配り方の箇条の後へ移した」に。経緯の文なので、★ 2026-11-03 で箇条が書き換わった旨を添えてもよい）
- 機械: できない
- 行き先: requirements（§5 の箇条の末尾に、Docker の箇条が書き換わった旨と、この監査の名前を足した）

## 2. 文書とコード

この組み合わせで文書が新しく述べたコードの事実のうち、食い違いは見つからなかった（下の「確かめた事実」）。K の指摘は無い。

## 確かめた事実（食い違いなし。指摘ではない）

- §5「V-01 はソースの場所と許諾を出さない」: `web/src` に許諾・ソースの場所へのリンクは無い（SPDX の見出しを除くと `license`・`github.com`・`許諾` は 0 件）
- §5 C-01 の ★ 2026-11-02「APK の中に置くだけで、アプリの画面からは開けない」: `collector-android/app/src/main` に許諾の画面・`oss-licenses` は無い（SPDX の見出しを除いて 0 件）。`assets/` はまだ無い（ST53 の tasks 6.1 が作る）
- handoff ST55 C12「`tools/check-licenses.sh` は同梱のバイナリを見ない」: `tools/check-licenses.sh` は Rust（cargo metadata）と Node の 2 つの python 節だけで、配布物の中のファイルを読まない
- 許可の一覧: `tools/check-licenses.sh:9` の `ALLOW` は `AGPL-3.0-only` を除いて ST53 の spec:91 の 17 と同じ（`Unicode-3.0` が 2 回書かれている）。`PostgreSQL` は無い
- FR-106 の ★ 2026-11-03（開発版は別のサービス・別の置き場）と ST53 の `SOURCE.txt` の開発版の決まり（main の commit）: 開発版の配布物の中身の話と置き方の話で、重ならない
- NFR-25 の改訂（1.0 の合図を「一般向けの配布」だけに）: ST53 の成果物は NFR-25 にも 1.0 にも触れない（spec の例は `v0.4.0`）
- `python3 scripts/check_scenarios.py .`: 担保の無い 51 件はすべて `openspec/changes/st53-license-bundle/` の中（未実装の change。tasks.md:13 の 51 本と同じ数）。正典の Scenario は全件に印がある

## 未確認

- EDB の PostgreSQL 17 の Windows の zip に入る部品と、その許諾（native-postgres でも未確認）。LGPL の部品（gettext の `libintl`・`libiconv` など）が入っていると、FR-127「対応するソースは、同じ印（タグ）のリポジトリで示す」と ST53 の `SOURCE.txt`（ashiato のリポジトリと印 / commit だけ）では、その部品の対応するソースを示せない。zip を落として中身を見ていないので指摘にしていない
- ST55 のサーバの配布物の形（今の zip のままか、インストーラの実行ファイル・MSI か）。インストーラになれば、ST53 の spec:165「tar / zip では根に」・`check-artifacts.sh` の 3 点の検査の置き場が合わなくなる（C4 に関わる）。ST55 の深掘りで決まる
- hx が requires の無い同じ layer の 2 本を、どちらの順でも下流へ進めるか（C3 の前提。`.harness2` の着手の判定を読んでいない。記憶の「着手は capability で止めない」に依る）

## 触らなくてよいと判断したもの

- EXT-S の向け先を ST55 にしたこと・release.md §7 の切り替えの時期: native-postgres の C12・C3 で決着済み。この監査はその上に残るずれ（C1・C3・C4）だけを出した
- ST59（FR-133。V-01 の設定から合言葉を変える）と §5「V-01 はソースの場所と許諾を出さない」: どちらも V-01 に関わるが、合言葉の設定とソースの場所の表示は重ならない。ST59 が設定の画面を作っても、ソースの場所を出す未決はそのまま残る
- FR-112 ★ 2026-11-03（更新係は別のサービスのアカウント）と ST53 の `notices.sh server`（`cargo tree -p <crate>` でサーバの crate だけ）: 更新係が別の crate になるか同じ実行ファイルの別の起動になるかは ST57 の設計で決まる。別の crate になれば一覧から漏れうるが、FR-112 の「別のプロセス」は 2026-10-25 からで、この rebase で生じたずれではない
- ST53 の tasks・design・spec に Docker・PostgreSQL・サービスへの言及が無いこと: grep で 0 件。改訂された前提に乗っていないので、C1・C2・C4 のほかに食い違いは無い
- release.md:3・:27 の「ST53 が main に入った後」（配布元への設定と本物の印）: 切り替えを含まないので FR-108 ★ と合う
- production-prep.md:48（配布元と D-01 の入手元）と ST53 が直した :32（本体のライセンス）: 別の行で、互いを前提にしていない。:46（依存ライセンス）が同梱のバイナリを見ないことは native-postgres C12 の範囲
- `check_chain.py` の FAIL 1 件: この監査が当たる
