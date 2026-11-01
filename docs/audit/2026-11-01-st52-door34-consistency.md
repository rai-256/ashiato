# 整合の確認（2026-11-01・ST52 の深掘り第 2 回 Q1・Q2・Q4・Q11 の答えを扉 34 へ戻した改訂）

- 基準: `3c345e456be41fb2f80319bde597f7c0b51db0fe`..`bfe0af109f5b052e7fec718a817b127b1f012af0`

対象は commit bfe0af10 が扉 34 に足した 2 つの ★（`docs/requirements.md:1775-1778`）。
★ 2026-11-01 追加（Q2・Q11）: 「**正式な鍵は、署名のための写しを GitHub の Environment にも置く**（印と main にだけ開く。CI が携帯の APK に署名する）」「**開発版（`dev.ashiato.collector.dev`）は正式版と別の鍵で署名する**（開発版の Environment に置き、控えは USB に 1 部）。開発版に携帯の APK を入れ始めるのは ST54 から（ST52 の深掘り Q4）」。
★ 2026-11-01 訂正（Q1）: 「本人の携帯には C-01 がまだ入っておらず、記録も集めていない。上の『本人の端末も 1 回だけ入れ直す』は本人には当たらない（入れるのは最初から正式な鍵の版）」。
答えの記録（`openspec/changes/st52-release-channels/deep.md`・`deep-answers-2.txt`・`deep-questions.json`）と突き合わせた。文書は head（`bfe0af10`）の内容で読んだ。

機械の入力:
- `req_impact.py . --base 3c345e45…`: 改訂した要件 0 件・扉 1 項（扉#34）。名指しの path:line は出ない（扉は ID で名指されないので、下は全部 grep で当たった）
- `check_chain.py .`: 要件 171 / Story 58 / 扉 35・拾われた要件 169/171・未回収 0・warn 0。FAIL 1 件「整合の確認の後に要件が変わった（この監査を名指ししない改訂）: 扉#34」（この監査が受けるもの）

走っている change は `st52-release-channels`（出どころ）と `st51-update-history`。st52 への指摘は**印だけ**（C6）。st52 の tasks は 28 件すべて未着手（`- [x]` が 0 件）なので、失われるもの（uncaptured / discarded / exported / rewrite-all）を持つ印は無い。st51 はこの改訂を前提にしていない（下の「触らなくてよい」）。

## C1. ST54 が「開発版に携帯の APK を入れ始め、開発版の鍵を作って使い始める」役を受けたのに、Story の本文・完了の判定・申し送りのどこにも無い
- 行き先: handoff ST54（`docs/handoff/ST54.md` に「ST52 Q4・Q11 — 開発版に携帯の APK を足し、開発版の鍵を作る」を書いた。完了の判定に足すかは ST54 の上流）
- 重大度: 高
- 文書: docs/stories/ST54.md:25
- 要件: FR-106（扉 34）
- 根拠: 改訂後の扉 34 ★（`docs/requirements.md:1777`）「**開発版（`dev.ashiato.collector.dev`）は正式版と別の鍵で署名する**（開発版の Environment に置き、控えは USB に 1 部）。開発版に携帯の APK を入れ始めるのは ST54 から」、`deep.md` Q11 の帰結「開発版の鍵を作って使い始めるのも ST54 から」。
  ST54 側は改訂の前のまま: `docs/stories/ST54.md:25-29` の扉 34 の写しは base の 4 行で切れ「…（続きは requirements.md）」（★ 2026-11-01 は載らない）。完了の判定（`stories.json:1294-1296`）は「携帯に開発版（`dev.ashiato.collector.dev`）を入れても、正式版はそのまま動き…」「開発版のリリースに『サポートしない』と書いてある」だけで、開発版のリリースに携帯の APK が並ぶこと・開発版の鍵で署名されることを持たない（`judged: 2026-10-25`）。
  `docs/handoff/ST54.md` は st51 の premises 9（C-02 の端末識別子）だけで、ST52 からの申し送りが無い。
  Story を組んだ時点（2026-10-25）と Q4 の推奨（`deep-questions.json` Q4 options[0].detail「扉 34 の値を ST52 で使い始める。**PC 側の分け方と『サポートしない』の明記は ST54 のまま**」）では、携帯の `.dev` の APK と開発版の鍵は ST52 が作る前提だった。Q4 で推奨と違う側を選んだので、ST54 は「携帯の分はもうある」前提で作られうる。
  開発版の鍵は「一度配った開発版の鍵は替えられません」（Q11 の why。loss: exported）なので、作る Story が誰の手で・どの鍵の形（st52 D3（仮）の RSA 4096・25 年以上）で作るかを持たないまま始まると、取り返せない側で決まる
- 直すところ: 凍結した Story への申し送り（`docs/handoff/ST54.md` に「開発版の nightly に携帯の APK を足す・開発版の鍵と開発版の Environment を作る（扉 34 ★ 2026-11-01・ST52 Q4・Q11）」）。完了の判定に携帯の分を足すかは /stories で
- 機械: 安く捕まえられる（check_chain: 扉の ★ が「ST<NN> から」と Story を名指し、その Story の判定日が ★ より古く、`docs/handoff/ST<NN>.md` にその扉の番号も ★ の日付も無ければ warn）

## C2. 扉 34 の ★ が正式な鍵の置き場を「印と main にだけ開く」と書き、走っている design D21（仮）と ST53 への申し送り（IMP-01）の「印 `v*` にだけ開く（main に開かない）」とぶつかる
- 行き先: requirements（扉 34 の ★ 2026-11-01 の括弧に「印と main は上限、どこまで狭めるかは D21（仮。いまは印 `v*` だけ）」を足した。本人の選んだ文面を消さず、厳しい側の D21 を正にする）
- 重大度: 中
- 文書: docs/requirements.md:1775
- 要件: —（扉 34。関与 FR-104）
- 根拠: 扉 34 ★「**正式な鍵は、署名のための写しを GitHub の Environment にも置く**（**印と main にだけ開く**。CI が携帯の APK に署名する）」。
  走っている change: `design.md:160`「D21（仮）. 配布用の Environment `release` は印 `v*` にだけ開く（枝 `main` に開かない）」、`:161`「`main` にも開くと、main に入った workflow の変更（AI が書いた PR を本人が merge したもの）が次の main の実行で正式な鍵を読める」、`specs/distribution/spec.md:193`「正式な鍵を、印 `v*` の push の実行にだけ開いた置き場」、`:222`「配布用の Environment の配る対象は印 `v*` だけ…そうでなければ検査は 0 でない終了コードで終わる」。
  `docs/handoff/ST53.md:5`（IMP-01）「正式な鍵の置き場（Environment release）は印の名前 v* だけを見て開く…D21（仮）が main に開かない理由（AI が書いた変更から鍵を読ませない）」。Q11 の選んだ側の detail（`deep-questions.json`）「正式な鍵は印のときだけ使う」。
  ★ は Q2 の選択肢の label（「印と main にだけ開いた Environment」）を逐語で写したもので、D21 はそれを狭めた仮。要件の文面どおりに作ると、D21・IMP-01 が閉じた経路（main の実行から正式な鍵を読む）が要件で許されたことになり、ST53 の深掘り（IMP-01 を受ける）が広い側を前提に始まる。開発版の Environment（main に開く）と取り違えた書き方にも読める
- 直すところ: requirements（★ の括弧を「印 `v*` の実行にだけ開く（main に開かない）」にするか、「印と main を上限とし、どこまで狭めるかは ST52 の D21」と書く。D21 は仮で台帳にあるので、どちらに寄せるかは改訂した側が決める）
- 機械: できない（要件の散文と design の仮の範囲の比べ。D21 が確定した後は spec:222 の Scenario が検査で固定する）

## C3. 製造準備の A-3「C-01 の署名鍵の保管」が、Environment の写しと開発版の鍵を持たない
- 行き先: docs（`docs/production-prep.md` の A-3 に ★ 2026-11-01 で Environment の写しと開発版の鍵を足した）
- 重大度: 中
- 文書: docs/production-prep.md:74
- 要件: —（扉 34）
- 根拠: A-3 の行（★ 2026-10-25 追加。「保管の欄は `docs/production-prep.md` の A-3 に置く」と扉 34 ★ 2026-10-25（`requirements.md:1774`）が名指す欄）「**鍵ファイルは暗号化して 2 か所（手元の USB と、FR-68 の拠点外の保管先へ本人が手で置く写し）、鍵の合言葉は紙で 1 部物理保管する。**」「記録先: 本表・扉 #34」。
  改訂後の扉 34 は、これに (a) 署名のための写しを GitHub の Environment に置く（控えにはならない）、(b) 開発版は別の鍵・開発版の Environment・控えは USB に 1 部、を足した（`requirements.md:1775-1777`）。A-3 はどちらも持たない。
  A-3 は「実データを入れる前でないと手遅れなもの」の表で、鍵の保管の正の欄として名指されている。ここだけを読むと鍵は手元の 2 か所にしか無いことになり、開発版の鍵の控えの数（1 部）も分からない。st52 の `docs/release.md`（tasks 11.1。まだ無い）が書く手順と A-3 が食い違う
- 直すところ: その文書（A-3 の行に ★ 2026-11-01 で (a)(b) を足す。可逆性の欄に開発版の鍵を失ったときの範囲（開発版の利用者だけが入れ直す。Q11 の context）も）
- 機械: 安く捕まえられる（check_chain: 扉の ★ が「保管の欄は <path> の <節>」と名指す文書の該当行の最終更新が、その扉の最新の ★ より古ければ warn）

## C4. Story の索引の扉 34 の行が、訂正された「本人の端末も debug の鍵の版から 1 回入れ直す」のまま
- 行き先: stories（`docs/stories/INDEX.md` の扉 34 の行を ★ 2026-11-01 訂正で直した）
- 重大度: 中
- 文書: docs/stories/INDEX.md:345
- 要件: —（扉 34）
- 根拠: INDEX「| #34 C-01 のアプリ識別子と署名鍵 | 最初に APK を配る前（ST52 が正式な鍵で署名する。**本人の端末も debug の鍵の版から 1 回入れ直す**） | ST52, ST54 |」。
  扉 34 ★ 2026-11-01 訂正（`requirements.md:1778`）「上の『本人の端末も 1 回だけ入れ直す』は本人には当たらない（入れるのは最初から正式な鍵の版）」。
  あわせて、扉 34 の担い手が ST52 だけでなく ST54（開発版の鍵・開発版の APK）にもなったことが期限の欄に無い（関与の欄の ST54 は前からある）。索引の扉の表は Story の期限を読む所で、ここを読む上流は入れ直しの手順（送り切ってから消す）を前提に置く
- 直すところ: stories（INDEX の扉 34 の行を ★ で直す: 入れ直しを消し、開発版の鍵と APK は ST54 からを足す）
- 機械: 安く捕まえられる（check_chain: 扉の最新の ★ の日付より INDEX の扉の表のその行（`git blame` の日付）が古ければ warn）

## C5. 「開発版に携帯の APK を ST54 まで入れない」の例外が扉 34 にしか無く、FR-104・FR-105 の「3 部品を 1 つのリリースに」は例外を持たない
- 行き先: requirements（FR-104 に ★ 2026-11-01 補足「開発版のリリースには ST54 まで C-01 を並べない」を足した）
- 重大度: 中
- 文書: docs/requirements.md:964
- 要件: FR-104, FR-105, FR-106
- 根拠: FR-104（`:964`）「THE SYSTEM SHALL S-01（V-01 を含む）・C-01・C-02 を、同じ版番号を付けた 1 つのリリースとして配布元に置く」、FR-105（`:969-970`）「開発版は**上書きされる 1 つのリリース**（nightly）」。どちらにも ★ 2026-11-01 が無い。
  例外は扉 34 ★（`:1777`）「開発版に携帯の APK を入れ始めるのは ST54 から」だけ。扉 34 は「アプリ識別子と署名鍵」の扉で、チャンネルに何を並べるかを決める項ではない。
  ST52 は FR-104・FR-105 を満たす Story（`stories.json` の ST52 の satisfies）で、st52 の正典の候補も「1 つのリリースに…携帯の収集アプリの APK・PC の収集アプリ…を」（`specs/distribution/spec.md:109`）・「配布物が 1 つでも組めなければ公開しない（…開発版は前の版のリリースを残す）」（`:112`）と全チャンネルに 3 部品を求める。要件の側で例外が FR に付いていないと、spec の段（`spec --answers Q1,Q4,Q11`）がどの文を例外にするかの根拠が扉 34 の 1 句になり、ST54 の後に例外を外す時（FR-104 に戻す時）の目印も無い
- 直すところ: requirements（FR-104 か FR-105 に ★ 2026-11-01 で「開発版の nightly には ST54 まで C-01 を並べない（ST52 Q4。扉 34）」）
- 機械: できない（扉の ★ の中の時限の例外が、どの FR の SHALL を緩めるかは散文）

## C6. 走っている st52 の proposal・design・specs・tasks が、Q1・Q4 の推奨（本人の携帯の入れ直し・開発版の `.dev` の APK を ST52 で配る）のまま（印のみ）
- 行き先: leave: 写しは次の spec の段（`spec --answers Q1,Q4,Q11`）が行う。tasks は未着手で失うものは無い。nightly の「1 つでも組めなければ公開しない」の数え方と、開発版の Environment・鍵の検査を ST54 へ渡すことは、その段が C1 の申し送りと揃えて決める
- 重大度: 中
- 文書: openspec/changes/st52-release-channels/proposal.md:35
- 要件: FR-104, FR-105, FR-106（扉 34）
- 根拠: 改訂後の扉 34 ★（`requirements.md:1777-1778`）「開発版に携帯の APK を入れ始めるのは ST54 から」「『本人の端末も 1 回だけ入れ直す』は本人には当たらない」。change はまだ逆:
  - 入れ直し（Q1）: `proposal.md:35`「いまの携帯の debug 鍵の版から正式な鍵の版へ移るときに 1 回入れ直すのは扉 34 の決定どおりで、ST52 は本人の携帯を切り替えない（Q1 の推奨。ST56 の後）」、`design.md:20`「本人の携帯を配布版へ切り替えること（Q1 の推奨。ST56 の後）」、`tasks.md:19`「Q1 本人の携帯は ST52 で切り替えない（ST56 の後）」、`premises.md:28`「実機の携帯に今なにが入っているか…→ Q1 で聞く」、premises P4（署名の引き継ぎ）は Q1 の答えで要らなくなった
  - 開発版の携帯の APK（Q4）: `proposal.md:20`「開発版の携帯の APK は `dev.ashiato.collector.dev`（Q4 の推奨・扉 34）」、`design.md:146`「開発版の識別子は `ASHIATO_BUILD_VERSION` が `-dev.` を持つときだけ `applicationIdSuffix = ".dev"`（Q4 の推奨）」、`tasks.md:20`「Q4 開発版の APK は `dev.ashiato.collector.dev`」、`tasks.md:69`（`applicationIdSuffix`）、`specs/distribution/spec.md:109`・`:112`（nightly にも APK を含む 4 つの配布物）、`:192`「開発版の携帯の APK を、アプリ識別子 `dev.ashiato.collector.dev` と…開発版の鍵で署名する」、Scenario `:207`「開発版の APK は、.dev の識別子と開発版の鍵で署名されている」・`:217`（取り違え）、`tasks.md:137-138`（`sign-apk.sh` の開発版の self-test）
  - 開発版の鍵（Q11 の帰結「使い始めるのも ST54 から」）: `design.md:41` D3（仮）「正式な鍵と開発版の鍵は RSA 4096…」、`design.md:150` D19「`nightly`（枝 `main`）を作り Secrets と指紋の変数…を置く・開発版の鍵の控えを USB に 1 部置く」、`tasks.md:171`・`:175`（`docs/release.md` に「USB に 1 部」「本人の環境を移す」があることを grep で検査）、spec `:196`（台本が開発版の鍵も作る）、`:222`（開発版の Environment の検査）
  `deep.md` は Q1・Q4 の帰結を書き、写しは spec の段（`spec --answers Q1,Q4,Q11`）の予定。写すときに一緒に決めるもの: nightly が APK を欠いた 3 つで公開できるように「配布物が 1 つでも組めなければ公開しない」の開発版の数え方を変えるか、開発版の Environment と鍵の検査（`check-guards.sh environments`・`sign-apk.sh` の開発版の側）を ST52 に残すか ST54 へ渡すか（C1 の申し送りの中身になる）
- 直すところ: その文書（走っている st52 の spec の段が写す。印のみ）
- 機械: 安く捕まえられる（review_triage の検査 4 の逆向き —— deep.md の「要件へ戻すもの」に挙がった扉の最新 ★ が、change の Scenario の日付より新しければ warn）

## C7. 網の手順書の「本人の機械での移行」が、本人の携帯に C-01 が入っている前提で「APK を入れ直す」と書いている
- 行き先: leave: 手順書の移行は入っている端末の一般の手順として読める。本人の携帯に最初に入れるのは ST56 の後の正式な鍵の版で、その手順は ST52 の `docs/release.md`（tasks 11.1）が持つ
- 重大度: 低
- 文書: docs/network.md:61
- 要件: —（扉 34）
- 根拠: `docs/network.md:61`「5. `~/.gradle/gradle.properties` の `ashiato.baseUrl` を `https://` にして APK を入れ直す（§3）」、`:29-31`「## 3. 収集アプリの入れ直し …APK を入れ直す」。`tools/verify-prep.sh:54` も「docs/network.md の移行の 5 を済ませる」と案内する。
  扉 34 ★ 2026-11-01 訂正「本人の携帯には C-01 がまだ入っておらず」。手順書の移行は ST28 の時点のもので、本人の携帯が無いなら 5 は「入れる」で、入れるのは debug の鍵の版になる（K1 とつながる）
- 直すところ: その文書（§6 の 5 に、本人の携帯にまだ入っていないときは入れない（正式な鍵の版を ST56 の後に入れる）を ★ で添える）。leave でもよい（手順は入っている端末の一般の手順として読める）
- 機械: できない

## K1. 確認バッチの準備が、繋がっている端末に debug の鍵の `dev.ashiato.collector` を黙って入れる
- 行き先: fix（`tools/verify-prep.sh` が入れる端末を、エミュレータか debug の鍵の `dev.ashiato.collector` が入っている端末に限る。コードなので上流のこの段では直さず、change で拾う。それまで確認バッチの準備で本人の携帯を繋がない）
- 重大度: 中
- 文書: docs/requirements.md:1778
- 要件: —（扉 34）
- 根拠: 扉 34 ★ 訂正「入れるのは最初から正式な鍵の版」、base（`:1771`）「鍵を失うと、それまでに配った APK を上書き更新できない」、premises P2（`premises.md:9`）「入っている APK と違う鍵で署名した APK は上書きで入らない」。
  コード: `tools/verify-prep.sh:55-62` は `./gradlew :app:assembleDebug` で組んだ `app-debug.apk`（`collector-android/app/build.gradle.kts:49` の `applicationId = "dev.ashiato.collector"`。`signingConfigs` は無く、debug は組んだ PC の debug 鍵）を、`adb devices` の最初の端末へ `adb -s "$device" install -r "$apk"` で入れる。端末を選ぶ・本人の携帯かを確かめる口は無い。
  確認バッチの間に本人の携帯を繋ぐと、まだ C-01 の無い携帯に debug の鍵の版が入り、★ 訂正が前提にした「最初から正式な鍵の版」が崩れて、扉 34 の base の「1 回入れ直す（未送信を送り切ってから）」の状態がもう一度できる。
  逆に正式な鍵の版が入った後は、`install -r` が署名の食い違いで失敗し、`:63-64` が「端末が繋がっていないので手で入れる」と誤った理由を出す
- 直すところ: コード（`verify-prep.sh` が端末に入れるのは、エミュレータか、すでに debug の鍵の `dev.ashiato.collector` が入っている端末だけにする。入っていない実機・別の鍵の版が入った実機には入れず、その理由を出す）。leave なら扉 34 ★ に「確認バッチの準備で本人の携帯を繋がない」を書く
- 機械: 安く捕まえられる（`verify-prep.sh` の中で `adb shell pm path dev.ashiato.collector` と `getprop ro.kernel.qemu` を見てから入れる。試験は偽の `adb` で「実機・未導入なら install を呼ばない」を固定できる）

## K2. 本人の実機でアプリ利用の未送信の量を数える台本は、収集アプリが入っていることを前提にしている（ST14 が確認バッチに置いた手順）
- 行き先: leave: 実機の計測は本人の携帯に正式な鍵の版が入る ST56 の後に回る。計測用 APK の署名はその時に決める
- 重大度: 低
- 文書: docs/handoff/ST14.md:90
- 要件: FR-8
- 根拠: `docs/handoff/ST14.md:90-91`「`tools/usage-volume.sh --device [serial]` で**本人の実機の直近 10 日の実イベント**を数え…しきい値を確定する」、ST14 は手順を確認バッチに置いた（`openspec/changes/archive/2026-10-17-st14-outage-alert/tasks.md:226`・`design.md:252` D19）。
  コード: `tools/usage-volume.sh:16`「アプリ本体は入れ直さず（入っている収集アプリをそのまま使う）」、`:175-176` `pm path "$PKG"` が無ければ「収集アプリ（$PKG）が入っていない」で rc=2。
  扉 34 ★ 訂正「本人の携帯には C-01 がまだ入っておらず」の下では、この手順は rc=2 で止まるか、先に debug の鍵の版を入れることになる（K1 と同じ崩れ）。正式な鍵の版を入れた後は、`am instrument` の計測用 APK（debug の鍵）が正式な鍵のアプリに対して走らない
- 直すところ: leave（実機の計測は本人の携帯に正式な鍵の版が入る ST56 の後に回る。その時に計測用 APK の署名をどうするかは ST56 か ST14 の続きで決める）。行き先を決めるなら凍結した Story への申し送り
- 機械: できない（どの端末が本人の携帯かは機械に分からない）

## 未確認

- ST14 の確認バッチで `tools/usage-volume.sh --device` を本人の実機で走らせたか（走らせたなら、その時点で本人の携帯に C-01 が入っていたことになり、Q1 の答え「まだ入っていない」と食い違う）。このリポジトリの `docs/verify/20261016-1818.json` に該当の記録が無く、答えは ashiato2-verify の側にある
- 扉 34 ★ の「Environment の値は取り出せない」は GitHub の仕様の事実で、コードでは確かめられない（st52 premises P12 は確かさ 2 とし、確認バッチで使い捨ての値で確かめる計画。`tasks.md:183`）
- `tools/android-emulator.sh` は先頭の注記（`:11-13`）で「実機を adb で繋いでいるときは…実機で走らせる」と書き、`:61-62` で `-s` なしに `adb uninstall dev.ashiato.collector` を呼ぶ。エミュレータと本人の携帯が同時に繋がっているときに、どちらに効くか（2 台なら adb が失敗して `|| true` で抜けるはず）・gradle の `connectedDebugAndroidTest` が本人の携帯にも debug の鍵の版を入れて消すかは走らせていない。正式な鍵の版が入った後に本人の携帯を繋いで走らせると、手元の未送信を消しうる経路かもしれない

## 触らなくてよいと判断したもの（理由つき）

- **ST52 の Story（題・価値・完了の判定）**: 完了の判定（`stories.json` の ST52「携帯の APK は正式な鍵で署名されている」「`v0.x.y` の印を付けると、配布元の**正式版**に 3 部品が同じ版で並び」）は正式版の署名と並びで、開発版の携帯の APK も本人の携帯の入れ直しも固定していない。価値の「自分の環境をその版で動かしたい」の携帯の分は、Q1 の推奨のときから ST56 の後で、この改訂で変わらない。`check_chain.py` の warn 10 も出ていない
- **ST53・ST56 の Story**: ST53（許諾）は扉 34 を触らず、IMP-01 の申し送り（`docs/handoff/ST53.md`）は機械が書く塊で、参照元の日時と答えのファイルが 2026-11-01T12:47:34Z・`deep-answers-2.txt` に更新済み（C2 の食い違いは要件の側の文面）。ST56（繋ぎ先の設定）は「配布元の APK を入れて初めて起動すると…設定すると送信が始まる」で、本人の携帯に最初に入れるのが正式な鍵の版になったこと（Q1）とむしろ合う
- **`docs/stories/ST52.md:37-41` の扉 34 の写し**: base の 4 行で切れ「…（続きは requirements.md）」で ★ 2026-11-01 は載らない。切りは `make_story.py` の形どおりで、ST52 は走っている change が deep.md で答えを持つので、写しの欠けで誤る席が無い（ST54 は C1）
- **`docs/briefs/ST52-pr.md:7-11`・`ST52-deep.html`・`ST52-spec.html`**: Q1 の推奨（入れ直し）の文が残るが、グラフが段ごとに作り直す資料で、答えの前の写し
- **`openspec/changes/st51-update-history`**: `design.md:162` の「消して入れ直した C-01 は端末識別子が変わる」は一般の危険の記述で、本人の携帯の入れ直し（扉 34）を前提にしていない（premises 8 は `DeviceId.kt` の振る舞い）。ST54 への申し送り（C-02 の端末識別子）も扉 34 と独立
- **扉 34 の base の文（`:1772`「**いまの C-01 は debug の鍵で入っている**ので…」）**: ★ 訂正が名指して打ち消しているので、要件の改訂の流儀（★ で直し base を残す）どおり。語の揺れとしても扱わない
- **扉 34 の識別子の値**: `dev.ashiato.collector` は `collector-android/app/build.gradle.kts:49` と一致し、`.dev`（`applicationIdSuffix`）がまだコードに無いのは「ST54 から」と合う。`.github/workflows` は `ci.yml` 1 本で、Environment・署名の記述は無い（ST52 の実装の前）
- **正典（`openspec/specs/`）**: この改訂は正典の Requirement も Scenario も変えない。`distribution` の capability は正典にまだ無く、st52 の change の中だけ
- **扉 32・33 の期限「本人の環境を配布の経路に切り替える日」**: 本人の携帯に C-01 が無くても、サーバと PC の切り替えの日として読め、期限の守り（ST49・ST51 が ST52 の前）は変わらない
