# 整合の確認（2026-11-02・ST53 の深掘り第 1 回の答えを FR-127 と §5「配布の方針」へ戻した改訂）

- 基準: `a5926cf9ec3399504c76be6aaced4636dfd3d006`..`71fd0bb10a1df7c386d38e28abfa4f14ce768541`

対象は commit 71fd0bb1 が足した 3 つの ★（`docs/requirements.md:1072` の FR-127 ★ 2026-11-02 補足、`:1526` の V-01 のソースの場所と許諾 ★、`:1530` の C-01 の許諾を読む画面 ★）と、
同じ commit が直した `docs/stories/stories.json`（→ `ST52.md`・`ST53.md` の完了の判定）と `docs/stories/INDEX.md` の訂正（2026-11-02）。
答えの記録（`openspec/changes/st53-license-bundle/deep.md`・`deep-answers-1.txt`・`go.json`・`agreement.json`）と突き合わせた。文書は head（`71fd0bb1`）の内容で読んだ。

機械の入力:
- `req_impact.py . --base a5926cf9…`: 改訂した要件 1 件（FR-127）・扉 0 項。名指しは `docs/stories/INDEX.md:76,365`・`stories.json:1260` と st53-license-bundle の中だけ
- `check_chain.py .`: 要件 171 / Story 58 / 扉 35・拾われた要件 169/171・未回収 0・warn 0。FAIL 1 件「整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-127」（この監査が受けるもの）
- `check_scenarios.py .`: 担保の無い Scenario 45 件。すべて st53-license-bundle の change のもの（tasks は 23 件すべて未着手）。正典の Scenario で印の無いものは 0

走っている change は `st53-license-bundle`（出どころ）だけ（ST52 は `archive/2026-11-02-st52-release-channels`）。st53 の tasks は `- [x]` が 0 件なので、失われるもの（uncaptured / discarded / exported / rewrite-all）を持つ印は無い。

## C1. IMP-01・IMP-02 の「リリースの前に判断し直す」の期限が「ST54 の上流の合意まで」で、Q4 が決めた最初の本物の公開（鍵を置く）より後になりうる
- 行き先: docs（`docs/release.md` §3 の鍵を置く前に IMP-01・IMP-02 の判断し直しを置いた。期限「ST54 の上流の合意まで」は合意の記録（`agreement.json`・go.json）が持ち、書き換えると合意を聞き直すことになるので変えない —— 手順書で鍵を置く前に必ず通る形にした）
- 重大度: 高
- 文書: openspec/changes/st53-license-bundle/deep.md:55
- 要件: FR-127（関与: 扉 34）
- 根拠: 本人の答え `deep-answers-1.txt`「IMP-01 […] -> 受け入れて進める（リリースの前に判断し直す）」「IMP-02 […] -> 受け入れて進める（リリースの前に判断し直す）」。
  `deep.md:56`・`:58`「リリースの前の判断し直しは go.json の `if_accepted.recheck`（本人・ST54 の上流の合意まで）」、`agreement.json` の `impacts.IMP-01.recheck`・`IMP-02.recheck` は `{"owner": "本人", "due": "ST54 の上流の合意まで"}`。
  一方 Q4 の答え「ST53 が main に入った後に本人が行う」（`design.md:27`・`:162` D12）で、最初の印と正式な鍵を Environment に置くのは ST53 の merge の直後になる。ST54 は `requires: [ST52]`（`INDEX.md:77`）で、ST53 の後にも最初の公開の前にも順が付いていない。
  残るリスクは取り返せない側（`go.json` IMP-02 の residual「エージェントが同じ利用者の設定から本人の管理者の token を読めば、承認者を外して正式な鍵を読める」、`specs/distribution/spec.md:243`「一度読まれた鍵は、読まれなかったことにできない」）。
  ST53 の `tasks.md:201-210`（10.2 `docs/release.md` の書き直し）の項目にも、鍵を置く前に IMP-01・IMP-02 を判断し直す手順は無い。「リリースの前」と「ST54 の上流まで」のどちらが先かを決めるものが無い
- 直すところ: その文書（recheck の期限を「最初の本物の印・鍵を Environment に置く前」にする）と、`docs/release.md` の鍵を置く節の前に「IMP-01・IMP-02 の判断し直し」を置く（st53 の tasks 10.2 に足す。走っている change なので印のみ）
- 機械: 安く捕まえられる（`imp_ledger.py` が open な recheck の `scope` に「正式な鍵」を含むものを、`docs/release.md` の鍵の節の手順（`check-guards.sh` の副コマンド 1 つ）で列挙して 0 でなければ止める）

## C2. ST53 への申し送りが、Q4 の答えと逆の「本物の印の公開・正式な鍵での署名・サーバの切り替えは ST53 の確認で見る」のまま
- 行き先: docs（`docs/handoff/ST53.md` の 3 か所を Q4・Q8 の答えに合わせた）
- 重大度: 中
- 文書: docs/handoff/ST53.md:20
- 要件: —（ST52 の完了の判定・扉 34）
- 根拠: `docs/handoff/ST53.md:20`「本物の印の公開、正式な鍵での署名、本人のサーバの切り替えは ST53 の確認で見る（Q13 / D22）」、`:18`「最初の印と開発版の公開は、ST53 の確認が終わった後に本人が始める」。
  改訂後の ST52 の完了の判定（`stories.json` ST52 done[0]・`ST52.md:56`）「本物の印による公開は、ST53 が main に入った後に本人が main に打って見る。ST53 の確認バッチでは見ない —— ST53 の深掘り Q4」、`:58`「正式な鍵での署名は、ST53 が main に入った後に、本人が鍵を Environment に置いて最初の本物の印で見る」。
  record の段は ST52 の Story を直したが、同じ文を持つ申し送りを直していない（`deep.md:71`・`:92` の「Story へ」に handoff が無い）。ST53 の下流と確認バッチの準備は申し送りを読むので、確認バッチの中で本物の印を打つ側に誤りうる（そのとき ST53 の承認者の守り（Q3）は main にまだ無い）。
  あわせて `:29`「ST53 の上流で引き受けるか、`scripts/hx change` で CH を起こして直す」は、Q8 で ST53 が引き受けたことを反映していない
- 直すところ: その文書（`docs/handoff/ST53.md` の「ST52 Task 11」の節に ★ 2026-11-02 で Q4 の答えを添え、K1 の節に Q8 で ST53 が引き受けたことを添える）
- 機械: 安く捕まえられる（record の段で stories.json の done を書き換えた文と同じ句（「ST53 の確認で見る」）を `docs/handoff/` から grep して残っていれば warn）

## C3. §5 に足した 2 つの未決（C-01 の許諾の画面・V-01 のソースの場所）は受ける Story も FR も無く、鎖の検査が数えない
- 行き先: leave: 本人の答え（Q6・Q9）が「要件の未決として残し、/requirements から足す。受ける Story はまだ無い」と決めた。期限つきの `[NEEDS CLARIFICATION]` で §5 に残し、memory の「どの Story も持っていない」の型と同じく /requirements で拾う
- 重大度: 中
- 文書: docs/requirements.md:1526
- 要件: FR-127（§5「配布の方針」）
- 根拠: `:1526`「[NEEDS CLARIFICATION: V-01 にソースの場所と許諾へのリンクを出すか（受ける Story はまだ無い。/requirements から足す）。期限: 一般向けの配布の前]」、`:1530`「[NEEDS CLARIFICATION: C-01 に許諾の一覧を読む画面を足すか（受ける Story はまだ無い。/requirements から足す）。期限: Play ストアへ出す前]」。
  `check_chain.py:9`・`:202` は「要件本文の [NEEDS CLARIFICATION] が、それを satisfies する Story へ運ばれているか」を FR / NFR の本文でだけ見る（`reqs.get(r, "")`）。§5 の箇条は FR でないので、どの Story にも運ばれず、「未回収」にも数えられない（`check_chain` の出力は未回収 0）。
  「/requirements から足す」の宛先は、要件を次に改訂する人が §5 を読むことにだけ乗る。期限（一般向けの配布の前・Play ストアへ出す前）を見張る Story も表も無い（`docs/stories/INDEX.md` の扉の表は扉だけ、`docs/production-prep.md` にも無い）
- 直すところ: requirements（FR として起こすか、§5 の未決を受ける Story を /stories で足す）か、INDEX に「Story の無い未決と期限」を置く
- 機械: 安く捕まえられる（check_chain: §5・§6 の `[NEEDS CLARIFICATION` で「受ける Story はまだ無い」を含むものを数えて warn に出す）

## C4. FR-127 ★ が PR の組み立ては「組み立ての 40 桁の commit」と固定したが、その commit を後から取れるかは確かめておらず、design の反転の側（PR の head の commit）は「組み立ての commit」でない
- 行き先: requirements（FR-127 ★ を「それ以外の組み立て（開発版・main への push・PR）は 40 桁の commit」とし、どの commit を書くかは D9（仮）が確かめて決めると書いた）
- 重大度: 中
- 文書: docs/requirements.md:1072
- 要件: FR-127
- 根拠: FR-127 ★「開発版と PR の組み立ては、組み立ての 40 桁の commit で示す」。
  `design.md:139`「PR の組み立ての commit は、GitHub が PR のために作る合成の merge commit（`refs/pull/<n>/merge`）になる。PR を閉じた後もその commit を取れるかは確かめていない（premises P14・確かさ 2）」、`:141` 反転条件「閉じた PR の merge commit を取れないと分かったら、PR の組み立てでは PR の head の commit（`github.event.pull_request.head.sha`）を `SOURCE.txt` に書き、組み立ての印との比べはその場合だけ外す」。
  `go.json` の IMP-06「閉じた PR の組み立ての SOURCE.txt が指す commit を、後から取れるか分からない」（medium・`if_accepted` 無し）。`premises.md:23` P14「未走行」。
  コード: `.github/workflows/ci.yml:2-4` は `pull_request` で走り、`package-linux`（`:350-365`）の checkout は `ref` を渡さない（既定の合成 merge commit）。
  要件の字面は「示す」だけで、示した commit から対応するソースを取れることを求めていない。反転の側に倒れると要件の字面（組み立ての commit）と食い違う。
  また ★ は「開発版と PR の組み立て」だけを挙げ、`ci.yml:3` の main への push の組み立て（同じ `package-*` の成果物を上げる）を名指さない（spec `:163` は「それ以外の組み立て」で受けている）
- 直すところ: requirements（★ を「正式版・ベータ版以外は、対応するソースを取れる 40 桁の commit で示す」にし、PR の合成 merge commit か head かは D9 に任せる）
- 機械: 安く捕まえられる（確認バッチの 11.1（`tasks.md:220`）が閉じた PR の commit を `git fetch origin <commit>` する。取れなければ D9 の反転と一緒に FR-127 ★ を見直すと tasks に書く）

## C5. §5 の V-01 の未決が、ソースの場所を「同じ版の印のリポジトリ」と書き、FR-127 ★ の「開発版と PR の組み立ては commit」と食い違う
- 行き先: requirements（V-01 の ★ のソースの場所を FR-127 の言い方に合わせた）
- 重大度: 中
- 文書: docs/requirements.md:1526
- 要件: FR-127
- 根拠: `:1526`「S-01 の画面（V-01）は、対応するソースの場所（**同じ版の印のリポジトリ**）と許諾へのリンクを出さない」（Q9 の問いの文 `deep.md:48`「ソースの場所（同じ版の印のリポジトリ）」の写し）。
  同じ commit の FR-127 ★（`:1072`）「**「同じ印」で示すのは正式版・ベータ版（印 `v*`）だけ。開発版と PR の組み立ては、組み立ての 40 桁の commit で示す**」。
  サーバは開発版（`nightly.yml:33`・`:48` が `server-linux`・`server-windows` を組む）にも PR の組み立てにも入る。未決を受ける Story が §5 の文から作ると、開発版の画面に動く印 `nightly` を出す側で作られる
- 直すところ: requirements（`:1526` の括弧を「対応するソースの場所（FR-127 ★ 2026-11-02 の決まり。正式版・ベータ版は印、それ以外は commit）」にする）
- 機械: できない（散文の中の言い換え）

## C6. FR-127 ★ は PR の組み立てを配布物に数えたが、扉 35 の期限「最初に配布物を公開する前」は、公開リポジトリの PR の成果物で既に過ぎている
- 行き先: leave: いま置かれている PR の成果物は保持の既定（90 日）で消え、本体のソースと `LICENSE`（AGPL-3.0）は公開リポジトリに同じ commit である。ST53 の後の組み立てから許諾が入る。扉 35 の追加の許可は付ける側の決定で、既に置かれた成果物が許可を欠いていても取り消せないものは生じない
- 重大度: 中
- 文書: docs/stories/INDEX.md:346
- 要件: FR-127, NFR-26（扉 35）
- 根拠: INDEX「| #35 AGPL の追加の許可（Google Play 開発者サービス） | 最初に配布物を公開する前 | ST53 |」、扉 35（`requirements.md:1784-1787`）「付けて配った版は、その許可付きで出回り続ける（取り消せない）」。
  FR-127 ★ は「開発版と PR の組み立ては、組み立ての 40 桁の commit で示す」と、PR の組み立ても対応するソースを示す配布物として扱う。
  コード: `ci.yml:364-365`（`package-linux`。携帯の APK を含む）・`:385-386`・`:401-402`（`release-package`）は `upload-artifact` に `retention-days` を付けずに上げ、許諾は入っていない（`tools/release/build.sh` に許諾の語は 0 件）。`git remote` は `rai-256/ashiato`（`design.md:13`・P7 で公開リポジトリ）。
  `review/deep.md:134-140` R5「期限内の `package-linux`・`release-package` は 14 件」、`design.md:143-148` D10（仮）は merge の後に本人が消す手順を置く。
  扉の表の期限はまだ先のことのように読め、ST53 が merge されるまで、ST53 を含まない枝の PR ごとに許諾の無い gms 入りの APK が公開の成果物として足され続ける
- 直すところ: requirements（扉 35 か FR-127 に ★ で「PR の成果物は ST52 から公開されている。ST53 の merge までのものは D10 で消す」を書き、INDEX の期限の欄を合わせる）。leave でもよい（D10 が掃除を持つ）
- 機械: できない（何を「公開」に数えるかの判断）

## C7. 製造準備の「本体のライセンス」の行が、扉 35 の追加の許可を持たないまま。ST53 の tasks も直さない
- 行き先: docs（`docs/production-prep.md` の「本体のライセンス」の行に追加の許可を足した）
- 重大度: 中
- 文書: docs/production-prep.md:33
- 要件: FR-127（扉 35）
- 根拠: `docs/production-prep.md:33`「| **本体のライセンス** | **AGPL-3.0 で公開する**（2026-09-08、本人が決定）。★ 2026-10-11: 衛星向けの API の契約と SDK は Apache-2.0 … | 要件 §5 | **不可逆** — 公開後は貢献者全員の同意が要る |」。
  改訂後の FR-127（`:1069`）「本体の許諾（AGPL-3.0 と追加の許可。§5）」、§5（`:1535-1539`）「本体の AGPL-3.0 に GPLv3 第 7 条の追加の許可を書き足す」、扉 35「付けて配った版は、その許可付きで出回り続ける（取り消せない）」。
  st53 の `tasks.md` 10.3 が直すのは「`docs/production-prep.md` の「依存ライセンスの検査」の行に ★ で携帯の節と表示の検査（ST53）を足す」だけで、`:33` の行は ST53 の後も AGPL-3.0 だけを書く。
  この表は不可逆の決定の台帳で、追加の許可（取り消せない）が載っていない
- 直すところ: その文書（`:33` に ★ 2026-10-25 / 2026-11-02 で追加の許可と範囲（名指しの 4 つ・Q2）を足す）。st53 の tasks 10.3 に足すなら印のみ
- 機械: できない

## C8. 手順書 `docs/release.md` が本人の作業の時期を「ST53 の確認後」と書き、Q4 の「ST53 が main に入った後」と違う（走っている change の tasks は冒頭だけを直す・印のみ）
- 行き先: docs（`docs/release.md` の 3 か所を「ST53 が main に入った後」にした）
- 重大度: 低
- 文書: docs/release.md:27
- 要件: —（扉 34・ST52 の完了の判定）
- 根拠: `docs/release.md:3`「本物の印を打ったりするのは、ST53 の確認後に本人が行う」、`:27`「ST53 の確認後、最初の正式版の印を打つ前に本人が設定する」、`:80`「ST53 の確認後、正式版の公開が完了してから本人が行う」。
  改訂後の ST52 の完了の判定（`ST52.md:56-59`）は「ST53 が main に入った後」。確認バッチと merge の間に main の手順書のまま鍵を置くと、並びは ST52 のもの（Environment と Secrets がルールセットより前。`docs/handoff/ST53.md:5` の IMP-01 の経路）になる。
  st53 の `tasks.md:202` は「冒頭: 本物の作業は ST53 が main に入った後」だけを挙げ、`:27`・`:80` を名指さず、`:210` の検証の grep も「ST53 の確認後」が残っていないことを見ない
- 直すところ: その文書（st53 の tasks 10.2 が `:27`・`:80` も直す。走っている change なので印のみ）
- 機械: 安く捕まえられる（10.2 の検証に `! grep -q "ST53 の確認後" docs/release.md` を足す）

## C9. Q3 の決定（承認者とエージェントの資格情報を替えてから鍵を置く）が、扉 34 と製造準備の鍵の保管の欄に無い
- 行き先: docs（`docs/production-prep.md` の鍵の保管の行に Q3 の順を足した。扉 34 の本文は鍵の値と保管を決めるもので、鍵を読む経路の守りは ST53 の spec（正典へ archive される）が持つので足さない）
- 重大度: 低
- 文書: docs/requirements.md:1778
- 要件: —（扉 34）
- 根拠: Q3 の答え「Environment `release` に本人を必須の承認者として付け、エージェントの資格情報を管理と承認の権限の無いものに替えてから、鍵を置く」（`deep.md:19`）。
  扉 34 ★ 2026-11-01（`:1778`）「正式な鍵は、署名のための写しを GitHub の Environment にも置く（印と main にだけ開く。…）」は置く条件を持たず、`docs/production-prep.md:74`（A-3 の鍵の保管の欄）も同じ。
  `deep.md:64-67` の「要件へ戻すもの」は FR-127 と §5 だけで、Q3 は spec（`specs/distribution/spec.md:230-244`）にだけ入る。正典に入るのは ST53 の archive の後で、それまで扉 34 を読む席（ST54 の上流が開発版の鍵を置く）は前提を知らない
- 直すところ: requirements（扉 34 に ★ 2026-11-02 で「正式な鍵を置くのは、必須の承認者とエージェントの資格情報の切り替えの後（ST53 Q3）」）と、`docs/production-prep.md` の A-3 に同じ 1 句
- 機械: できない

## C10. ST53 の「効いてくる未決」が「マーカーは無い」のまま。FR-127 ★ は §5 の未決 2 つを名指している
- 行き先: leave: `ST53.md` の「効いてくる未決」は satisfies の要件本文のマーカーを機械で写す（`make_story.py`）。§5 の未決 2 つは ST53 が満たすものではなく、受ける Story が無い（C3）
- 重大度: 低
- 文書: docs/stories/ST53.md:36
- 要件: FR-127
- 根拠: `ST53.md:36`「（要件本文に [NEEDS CLARIFICATION] マーカーは無い。旧形式の未決は OPEN 表 / DDR 台帳側にある — 移行の課題）」。
  同じ Story の `:21` に写した FR-127 ★「携帯の画面は Play ストアへ出す前、サーバの画面は一般向けの配布の前に決める。§5「配布の方針」の未決」、§5 の `:1530` は「C-01 の許諾（FR-127）は…[NEEDS CLARIFICATION: …]」と FR-127 を名指す。
  `check_chain.py:202` は満たす FR の本文の中のマーカーだけを運ぶので、§5 から FR を名指すマーカーは運ばれない
- 直すところ: leave でもよい（どちらの期限も ST53 の後で、ST53 は画面を作らない）。運ぶなら make_story
- 機械: 安く捕まえられる（make_story / check_chain: §5 の `[NEEDS CLARIFICATION` を持つ箇条が FR-ID を名指していれば、その FR を満たす Story に運ぶ）

## C11. 走っている change の design・spec が、答えの前の印（「答え待ち」・Open Questions・FR-127 の改訂前の要約）のまま（印のみ）
- 行き先: leave: design・spec は本人が合意した中身（`agreement.json` の digest）に結び付く。答えは全問が推奨のままで、「答え待ち」の印は推奨と同じ中身を指す。書き換えると合意を聞き直すことになるので、次に spec を直す段（gate の fix など）でまとめて直す
- 重大度: 低
- 文書: openspec/changes/st53-license-bundle/design.md:202
- 要件: FR-127
- 根拠: `design.md:36`・`:49`・`:57`・`:164`・`:175`・`:183`・`:202`「Q<n> の推奨（答え待ち）」、`:226`「Open Questions: Q1〜Q4（A）の本人の答え」。答えは `deep-answers-1.txt` で全問が推奨のまま（`deep.md:3-4`）。
  D16 は「（仮）」のまま（`:200`）だが、`deep.md:77`「D16（仮）は答えと一致した」、FR-127 ★ は「design D16（仮）」を出所に名指す。
  `specs/distribution/spec.md:171` の導出元「FR-127（…対応するソースは同じ印のリポジトリで示す）」は ★ 2026-11-02 を名指さない。
  tasks は 23 件すべて未着手で、失われるものは無い
- 直すところ: その文書（走っている change。印のみ）
- 機械: 安く捕まえられる（record の段の後に design の「答え待ち」を grep して残っていれば warn）

## C12. FR-127 ★ が「開発版の印 `nightly` は毎日動き」と書くが、開発版は変更のあった日だけ作り直す
- 行き先: requirements（FR-127 ★ の言い回しを「作り直すたびに付け替わり」にした）
- 重大度: 低
- 文書: docs/requirements.md:1072
- 要件: FR-127, FR-107
- 根拠: FR-127 ★「開発版の印 `nightly` は毎日動き」。FR-107（`:978`）「WHEN 1 日の終わりに、幹にその日の変更がある THE SYSTEM SHALL 開発版を作り直す。変更の無い日は作らない」。
  コード: `nightly.yml:19` が `version.sh nightly-plan` の `publish=` で決め、`:86` `git push --force origin "$GITHUB_SHA:refs/tags/nightly"` で動かす。結論（印では指せない）は変わらない
- 直すところ: requirements（「変更のあった日ごとに動き」）。leave でもよい
- 機械: できない

## C13. §5 の V-01 の ★ が、Docker の箇条の途中（未決とその理由の文の間）に入った
- 行き先: requirements（V-01 の ★ を Docker の箇条の後の独立した箇条へ移した）
- 重大度: 低
- 文書: docs/requirements.md:1526
- 要件: —
- 根拠: `:1525`「[NEEDS CLARIFICATION: どちらにするか。…期限: 一般向けの配布の前]。」→ `:1526`「★ 2026-11-02 追加。ST53 の深掘り Q9 …」→ `:1527`「自宅サーバ型のソフトでは Docker を前提にするのは一般的だが、実行環境の導入まで面倒を見るものは少ない（本人の言葉:…）」。
  `:1527` は S-01 の配り方の理由で、V-01 の ★ の後ろに来て、V-01 の未決の理由のように読める
- 直すところ: requirements（★ を `:1527` の後ろへ移す）
- 機械: できない

## K1. §5 が「他人の GPL / AGPL のコードを取り込むと…NFR-26 の検査が止める」と言うが、いまの検査は他人の AGPL の依存を通し、取り込んだコードは見ない
- 行き先: requirements（§5 に ★ 2026-11-02 訂正で、検査が止める範囲を依存と表示を持つ写したファイルに狭めた。依存の側は ST53 の D6 が閉じる）
- 重大度: 高
- 文書: docs/requirements.md:1539
- 要件: NFR-26, FR-127
- 根拠: `:1539`「他人の GPL / AGPL のコードを取り込むと、その部分に許可が及ばない（NFR-26 の検査が止める）」。
  コード: `tools/check-licenses.sh:9` の許可一覧が `AGPL-3.0-only` を持つ（他人の AGPL-3.0 の crate / npm も通る）。`:136-142` の Android の節は「この検査の対象外」と echo するだけ。repo に写したコードを見る検査は無い（`tools/check-spdx.sh` はまだ無い）。
  st53 の `design.md:90-99` D6（仮）が許可一覧から `AGPL-3.0-only` を外して依存の側を閉じるが、`design.md:216`「表示の無い写したコードは止まらない（D5）。§5 の「NFR-26 の検査が止める」は、依存と表示を持つファイルについてだけ成り立つ」で、ST53 の後も §5 の文面どおりにはならない。
  `deep.md:64-67` の「要件へ戻すもの」にこの狭めは無い
- 直すところ: requirements（§5 の括弧を「依存は NFR-26 の検査が、表示を持つ写したコードは表示の検査が止める。表示の無い写したコードは止まらない」に狭める）。依存の側はコード（ST53 の D6）
- 機械: 捕まえられた（ST53 の design D5・Risks が書いている）。依存の側は ST53 の `check-licenses.sh` の Scenario「他人の部品の AGPL の許諾は、許諾の検査が落とす」で固定される

## 未確認

- GitHub が PR を閉じた後も合成の merge commit（`refs/pull/<n>/merge`）を `git fetch origin <commit>` で返すか（P14・IMP-06。st53 の確認バッチ 11.1 で見る計画。C4 の前提）
- 公開リポジトリの PR の成果物（`package-linux`・`release-package`）がいま何件あり、誰が落とせるか（`review/deep.md` R5 の 14 件は 2026-11-02 時点の別の席の実測で、この席では `gh api` を走らせていない。C6）
- ST54 の上流と、ST53 の merge 後の最初の本物の公開のどちらが先に来るか（C1。順を決めるものが無いことは確かめたが、予定は分からない）
- 携帯の許諾を APK の `assets/licenses/` にだけ置き画面から開けない形が、Apache-2.0 第 4 条 (d) の NOTICE の表示や Google Play 開発者サービスの許諾の表示の求めを満たすか（法の読み。コードでは確かめられない）
- AGPL-3.0 第 13 条（ネットワーク越しの利用者へのソースの提示）と、V-01 にソースの場所を出さない Q9 の関係（改変した者に課される条で、本人の版には直ちに効かないと読んだが、法の読みで確かめていない）

## 触らなくてよいと判断したもの（理由つき）

- **ST52 の完了の判定の書き直し（`ST52.md:56-59`）**: ST52 は archive 済み（`archive/2026-11-02-st52-release-channels`）で、Q4 の答えに合わせて時期の句だけを直した。題・価値・ほかの判定は変わらない。`judged` は 2026-10-25 のままだが、change のある Story は check_chain の 10 の対象外
- **ST53 の完了の判定の SPDX の例外（移行の 3 つ・gradle の wrapper の 2 つ）と Q5 の答えの「506 ファイル」**: 推奨の文は 506 だが、spec の段で D4（仮）が 503 に絞り、GO 資料で見せた（`design.md:78`・review/spec.md R9）。本人は仕様に合意している（QA「合意する」）。`deep.md:76` がこの関係を書いている
- **INDEX の扉 34 の行（`:345`）に ST53 が無い**: INDEX の訂正（`:360-366`）が、doors は satisfies から機械で決まり、ST53 は鍵を読む経路の守りだけを足すと書いている。期限の欄（最初に APK を配る前）は変わらない
- **FR-104 ★ 2026-11-01（開発版に ST54 まで C-01 を並べない）と FR-127 ★ の「携帯の APK の中（`assets/licenses/`）」**: ★ は開発版に APK があるとは言っていない。`check-artifacts.sh:92-98` は nightly に APK があれば落とし、st53 の spec `:154`・`:165` も配布物ごとに入れる形で、食い違わない
- **「印」の二義（タグと組み立ての印 `<版>+<commit>`）**: ST49 からの用語で、この改訂が持ち込んだものではない。FR-127 ★ の「印」はどれもタグの意味で使っている。spec `:163` の「組み立ての印の 40 桁の commit」は ST49 の用語どおり
- **`docs/production-prep.md:237` の依存の件数（Rust 237 / Node 170）**: 2026-09 の実測の記録として書かれた行で、いまの件数（P3 の 357 / 306）を主張していない
- **`docs/handoff/ST53.md:3-14` の IMP-01 の塊**: 機械が書く塊で、手で直すと merge gate が止める。中身は ST52 の時点の問題の記述で、Q3 の答えと食い違わない
- **ST56（C-01 の繋ぎ先の設定画面）**: Q6 の未決（許諾を読む画面）の置き場になりうるが、ST56 の完了の判定は許諾の画面を持たず、この改訂と食い違わない
- **正典（`openspec/specs/`）**: この改訂は正典の Requirement も Scenario も変えない。`distribution` の正典に FR-127 の Requirement はまだ無く、st53 の change の中だけ。正典の Scenario で印の無いものは 0（`check_scenarios.py`）
