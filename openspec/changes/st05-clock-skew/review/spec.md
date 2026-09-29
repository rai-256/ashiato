# ST05 spec レビュー（spec-review）—— st05-clock-skew

- 対象: `proposal.md` / `specs/device-collection/spec.md` / `specs/desktop-collection/spec.md` / `design.md` / `tasks.md` /
  `docs/stories/ST05.md` / `docs/stories/INDEX.md`（未 commit の訂正 2026-09-29 を含む作業ツリー）
- 決定の正本: `deep.md`（Q1〜Q3・C1〜C8）/ `deep-answers-1.txt`。要件: `docs/requirements.md` FR-1 / FR-7（★ 2026-09-15）
- 並走: ST06（`openspec/changes/st06-app-usage`）/ ST08（`openspec/changes/st08-browser-history`・`origin/feat/st08-browser-history`）
- 日付: 2026-09-29

## 機械の検査（最初に）

| コマンド | 結果 |
|---|---|
| `openspec validate st05-clock-skew --strict` | `Change 'st05-clock-skew' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0。観点 8 の再生成一致も OK） |
| `python3 scripts/check_scenarios.py . st05-clock-skew` | `scenarios: FAIL (担保なし 36 件 / 名無しの確認待ち 0 件)`（rc=1）。Scenario 39 本のうち既存の印 3 本。下流が印を置く前提なので想定どおり |
| `python3 scripts/review_triage.py . st05-clock-skew` | `triage: OK`（`review/deep.md` の 9 件。この文書は処置前） |
| `python3 scripts/evidence.py unverified openspec/changes/st05-clock-skew --task N`（N=1〜9） | **Task 2: `["2.1", "2.3"]` / Task 4: `["4.3"]`**。他は `[]`（R9） |

MODIFIED の写し: `desktop-collection`「PC 側の収集は時計のずれを測って残す」と `device-collection`「位置は 60 秒間隔で記録される」は、
正典（`openspec/specs/desktop-collection/spec.md:276-291` / `openspec/specs/device-collection/spec.md:10-29`）の見出し・本文・Scenario を
すべて含んでいる。desktop 側で落ちたのは正典 285-286 行の「FR-7 の本文は C-01 だけを名指しており、要件の改訂は ST05 の担当」の 2 行だけで、
FR-7 ★ で改訂済みなので落として正しい。**写しの欠落は無い。**

並走 Story と同じ Requirement: ST06 の delta（ADDED 6 / MODIFIED「端末から失われた記録を破棄として報告する」「記録は 1 時間以内に格納される」）、
ST08 の delta（ADDED 6 / MODIFIED 5）と、ST05 の MODIFIED 2 本・ADDED 4 本の見出しは**1 本も重ならない**。
ただし ST06 の ADDED の本文と ST05 の design が食い違う（R5）。

---

## R1. 本人の決定 Q1（外部の時刻サーバへ問い合わせない。exported）を破っても落ちる Scenario・検証が無い
- 成果物: openspec/changes/st05-clock-skew/specs/device-collection/spec.md / specs/desktop-collection/spec.md / tasks.md
- 根拠:
  - Q1 は本文 `specs/device-collection/spec.md:9`（「外部の時刻サーバへ問い合わせない」）にだけあり、Scenario は `:73-76`「測るために通信を起こさない」
    （THEN「収集側が起こした通信の数が変わらない」）。design D2 `design.md:68` はその測り方を「**偽の `Transport` の呼び出し回数**」と定めている。
    時刻サーバへの問い合わせ（SNTP の UDP・`java.net` の直接の接続）は `Transport` を通らないので、**問い合わせても呼び出し回数は変わらず緑になる**
  - 代わりの静的検査 `tasks.md:80` の `! grep -nE "import (java\.net|.*HttpURLConnection|.*Transport)" …/ClockSkew*.kt` は 3 通りに空振りする:
    (a) `Transport` は `Sender.kt:16` で同じ package `dev.ashiato.collector` にあり（`HttpTransport.kt:2` / `Sender.kt:2`）、**Kotlin は同じ package の import を要らない**
    (b) 基準を読む `ClockReferences`（3.2）/ `ResponseDateCache`（3.1）/ `SystemTimeSources` は名前が `ClockSkew*` でないので glob に入らない
    (c) `ClockSkew*.kt` が 1 本も無ければ grep は rc=2、`!` で rc=0
  - PC 側: 本文 `specs/desktop-collection/spec.md:11-12`（外部の時刻サーバへ問い合わせない・Windows に同期をさせる操作もしない）に対し、Scenario `:70-73` は
    「**取り込み口へ**送った要求の数」だけを見る。`w32tm /resync` を叩いても、外部の時刻サーバへ問い合わせても緑
- kind: technical
- 提案: 両 spec に Q1 そのものの Scenario を立てる（例「WHEN 測る THEN S-01 以外の宛先への接続は 0 件」、PC は「Windows の時刻同期を起こす操作をしない」）。
  検証は `Transport` の回数ではなく、測定の口が触れる OS の口（ソケット・子プロセスの引数）を偽物にして記録するか、main の全ソースに対する静的検査
  （`SntpClient` / `DatagramSocket` / `/resync` の不在）にする。3.2 の grep は package と glob を直す
- 処置: fixed 10.1 —— kind を irreversible（loss: exported）から technical に直した: 本人の決定 Q1 は済んでおり（deep.md Q1）、指摘は「その決定を破っても落ちる検証が無い」という検証の穴で、人間に決め直してもらうものは無い。両 spec に Scenario `外部の時刻サーバへ問い合わせない` / `PC は外部の時刻サーバへ問い合わせない` / `PC は Windows に時刻を同期させない` を立て、配布物の静的検査 `tools/check-no-time-server.sh`（`--self-test` で植えた経路を捕まえることを自分で確かめる。CI に足す）を 10.1、w32tm の引数の固定を 8.1 に置いた。4.2 の grep は package の内側の名前（`Transport\b` / `Sender\b`）と `ClockReferences.kt` / `ResponseDateCache.kt` を名指しし、ファイルが無ければ `ls` で落ちる形に直した

## R2. D7（PC の `/healthz` をやめ送信の応答の `Date` を読む）の根拠が本人の決定ではなく、C3（飛んだ直後に測る）を PC で空にする
- 成果物: openspec/changes/st05-clock-skew/design.md（D7）/ specs/desktop-collection/spec.md / docs/requirements.md（FR-7）
- 根拠:
  - D7 の理由 `design.md:148` は FR-7 ★ の「通信を増やさずに読める」。その ★ `docs/requirements.md:128` は出所を「Q1 / Q3 と C2 / C5 / C6 / C8」とするが、
    **Q1 は携帯端末だけの問い**（`deep.md:9`、`deep-questions.json` Q1 の question「携帯端末のずれを測るとき」）、**C5 も「携帯端末では」**（`deep.md:53`）。
    Q3 の選んだ選択肢の「通信は増えない」は Windows の同期状態を足すことの説明で、`/healthz` をやめる選択は本人に示されていない（`deep-questions.json` Q3 options[0].detail）。
    C-02 に「通信を増やさずに」を及ぼしたのは要件へ戻すときの書き手の拡張
  - D7 は（仮）なのに反転条件 `design.md:151-152` が「FR-7 の『通信を増やさずに』を変えることなので本人に返す」—— 仮（B）と A の扱いが同じ判断に同居している
  - 実害: 壁時計の飛び（`specs/desktop-collection/spec.md:44-47`、C3 の理由は「破れた直後こそ測らないと扉 #5 に届かない」`deep.md:51`）で測るとき、
    D7 では `s01-date` は**飛ぶ前に受け取った応答**（前回の測定より後なので使われる。`design.md:145`）で、差は飛ぶ前の時計のもの。
    PC のもう 1 つの基準（Windows の同期状態）は差を持たない（`design.md:160`）。**飛んだ直後の測定記録に、飛んだ後の差が 1 つも載らない**。
    ST07 の `/healthz` は測定の時点で叩くので、いまは載る（`crates/collector-windows/src/clock.rs:115-118`）
- kind: conflict
- 提案: (a) 飛び・時計の変更の契機では「飛ぶ前に受け取った応答」を捨て（`no_response_since_last` 相当の理由で残し）、測り直しで次の送信の応答を拾う、
  を spec の Scenario にする。または (b) PC は `/healthz` を残し FR-7 の C-02 の文を直す。どちらにしても FR-7 ★ の出所の記述を事実に合わせ、
  D7 を C（既定）として理由を書くか、本人への問い（B 以上）として立て直す。端末の `time_set` 契機にも同じ形の問題がある（R3）
- 処置: fixed D7 仮 —— kind を premise から conflict に直した: 崩れた前提は本人の決定ではなく、record 段が FR-7 を書き戻すときに「通信を増やさずに」を C-02 へ及ぼした書き手の拡張（指摘のとおり Q1 / C5 は携帯端末だけ）。本人の Q3 の答え（①② を揃え ③ に Windows の同期の状態）はどちらの読み方でも成り立つ。D7 を ST07 のまま「測定の契機に `/healthz` を叩く」に戻し（飛んだ直後の差が残る）、FR-7 の C-02 の文を ★ 2026-09-29 訂正で事実に合わせた。反転条件（本人が PC も要求を足さないを選ぶなら送信の応答へ倒す）を D7 に書き、PR 本文の仮決めに挙げる。desktop の Scenario に `時計が飛んだ直後の測定は飛んだ後の差を持つ` を足した。端末の time_set の同じ穴は spec の本文と Scenario `時計の変更より前に受け取った応答の日付は使わない`（D2 の `clock_changed_since`）で閉じた

## R3. `s01-date` の差が「いつの差か」と秒の切り捨ての偏りが spec に無く、「差が約 300000」の Scenario がこの基準で偽になる
- 成果物: openspec/changes/st05-clock-skew/specs/device-collection/spec.md
- 根拠: 本文 `:11-12` は基準ごとの「端末の時計との差」とだけ書く。design D2 `design.md:62-63` は `s01-date` の差を**応答を読み終えた直後の壁時計**で取り、
  それは最長で前回の測定まで（約 1 時間）遡る。Scenario `:31-35` の THEN「差が約 300000 ミリ秒（読む前後の経過時間の幅に収まる）」は、
  `s01-date` については HTTP の日付の秒の切り捨て（0〜+999 ms。`design.md:63`）が幅の外に出るので、**幅が 999 ms 未満なら設計どおりの実装で落ちる**。
  また `time_set` 契機（`:52-55`）の記録では、`network` / `gnss` が変更後の差、`s01-date` が変更前の差を並べる（R2 と同じ構造）
- kind: technical
- 提案: 本文に「差はその基準を読んだ時点の端末の時計との差。`s01-date` は応答を受け取った時点」と書き、Scenario の許容を
  「読む前後の幅 ＋ 日付の分解能（1 秒）」にする。時計の変更の契機で変更前の応答を使わないなら Scenario を足す（R2 の (a)）
- 処置: fixed D2 —— spec の本文に「差に使う時計はその基準を読む前後の間で読む。S-01 の応答の日付は受け取った時点」を書き、Scenario を基準ごとに分けた（ネットワーク時刻は 300000 ± 読む前後の幅、応答の日付は `応答の日付の差は秒の分解能の幅に収まる` で 300000 以上 301000 未満 ± 幅）。time_set の構造は R2 の処置

## R4. Q3 ②（差に混ざる遅れ）は「前後の経過時間が入っている」だけでは直らない。差に使う PC の時計を読む時点が spec に無い
- 成果物: openspec/changes/st05-clock-skew/specs/desktop-collection/spec.md
- 根拠: Q3 の ② は「壁時計を見回りの先頭で読み、窓の観測と保存の**後**に取り込み口を叩くので、その遅れ（最大 30 秒）が差に混ざる」（`deep-questions.json` Q3 context）。
  いまのコード `crates/collector-windows/src/runtime.rs:396-398` は見回りの `wall` を `measure` に渡している。spec の Scenario `:54-57` は
  「出どころと読む直前と直後の起動からの経過時間を持つ」（欄がある）だけを見るので、**差の計算に先頭の `wall` を使ったまま欄を足しても緑**。
  直ったかどうか（差に使った PC の時計がその前後の間で読まれたか）を観測する Scenario が無い。design D7 `design.md:144` の `wall_after` は置き場の話で、spec ではない
- kind: technical
- 提案: 本文に「差に使う PC の時計の時刻は、その基準を読む直前と直後の間で読んだもの」を足し、Scenario を「偽の時計で見回りの先頭から基準を読むまでに
  30 秒進めても、差にその 30 秒が混ざらない」の形で立てる。端末側（`specs/device-collection/spec.md:11-12`）も同じ文にする
- 処置: fixed 8.2 —— desktop の本文に「差に使う PC の時計の時刻は、その基準を読む直前と直後の間で読む」を足し、Scenario `差に使う PC の時計は基準を読む前後の間で読む`（見回りの先頭から応答までに 30 秒進めても混ざらない）を立てた。端末も同じ文と Scenario `差に使う端末の時計は基準を読む前後の間で読む`（4.2）。design D7 に「見回りの先頭の wall を measure に渡さない」を書いた

## R5. `c01-clock` は生存信号を送らない（D1）が、並走 ST06 の ADDED は「取得できないソースも生存信号を送り続ける」「どのソースも生存信号の区間に取得契機が入る」
- 成果物: openspec/changes/st05-clock-skew/design.md（D1）/ specs/device-collection/spec.md
- 根拠: `design.md:46`（生存信号は送らない（仮））、`deep.md:49`（C1）。ST06 `openspec/changes/st06-app-usage/specs/device-collection/spec.md:229`
  「取得できないソースについて…生存信号を送り続ける」、`:233`、Scenario `:281-284`「**収集しているすべてのソース**について…生存信号の区間の長さが取得契機の間隔以上」。
  両方が archive されると、`device-collection` の正典は「全ソースが生存信号を持つ」と読め、`c01-clock`（1 時間ごとの取得契機を持つ論理ソース）はそれを満たさない。
  ST05 の spec は `c01-clock` が生存信号の対象外であることを**1 行も書いていない**（design にだけある）。proposal の重なりの表 `proposal.md:86` は ST06 の MODIFIED だけを見て「別の Requirement」としている
- kind: conflict
- 提案: ST05 の `device-collection` の ADDED に「測定記録の論理ソースは生存信号を送らない（収集のソースではない）」と Scenario を置き、
  ST06 には差し戻さず `docs/handoff/ST06.md` に「`ソースごとに独立して収集する` の『ソース』から `c01-clock` を除く」ことを書く。
  逆に送る側に倒すなら D1 を反転（ST06 の数えの形に乗る）
- 処置: fixed D1 仮 —— device の spec の本文に「その論理ソースは収集のソースではなく、生存信号を送らない」と Scenario `測定記録の論理ソースは生存信号を送らない`（6.2）を置き、ST06 の「ソースごとに独立して収集する」の『ソース』との境界を ST05 の側で引いた。D1 に理由と ST06 へ戻すものは無いことを書いた。D1 は（仮）で反転条件（ST14 の途絶の判定）を持つ

## R6. PC は取れなかった測定も毎時 `c02-window` に 1 件入れるので、稼働状況の `c02-window` の日が ③/② でなく ① になる（C1 の理由を PC にだけ当てていない）
- 成果物: openspec/changes/st05-clock-skew/specs/desktop-collection/spec.md / design.md（D10）
- 根拠: C1（`deep.md:49`）は端末の測定を位置と混ぜない理由を「成功条件 1 の数え」に置いた。PC の測定は D10 `design.md:175-177` で `c02-window` の `kind = clock-skew` のまま。
  `crates/server/src/coverage.rs:959-966` の `decide()` は `event_count > 0` を生存信号の `capturable` より先に見て `Recorded` を返す。
  ST05 で**基準が取れない契機も**毎時 1 件入る（`specs/desktop-collection/spec.md:17`）ので、PC が動いていた日は窓が読めない日（③）も記録なしの日（②）も、
  すべて ① 記録あり になる。`c02-window` は `USAGE_SUBJECT`（`coverage.rs:30`）なので NFR-13 の達成日の数えは変わらないが、FR-54 の日の状態は変わる。
  ST07 でも取り込み口に届く日は同じだった（同じ機械の構成ではいつも）ので、既存の性質を ST05 が「届かない日」にも広げる形
- kind: conflict
- 提案: design の Risks に書いたうえで、(a) `c02-clock` の別ソースにする（端末と同じ理由）か、(b) 稼働状況の数えから `kind = clock-skew` を除くことを
  `collection-coverage` の持ち主へ handoff する、のどちらかを決める。どちらも本人に聞くほどではない（計算し直せば戻る）が、D 番号に残す
- 処置: fixed D12 仮 —— PC の測定記録は `c02-window` に残す（過去の測定記録の置き場・ST08 が送信のソースを 2 本にしている最中）。日の状態が ① に塗られるのは ST07 からある性質で、数え方は `collection-coverage`（ST12 が走行中）の領分なので `docs/handoff/ST14.md` に申し送った（ST06 R5 と同じ型・同じ着地点）。反転条件（ST14 が kind で除けないなら `c02-clock` へ移す）を D12 に書いた

## R7. D8 の反転先（レジストリ・`GetSystemTimeAdjustment`）では spec の「最後に時刻を合わせた時刻」が取れない。前提の確かめ（7.1）が結果を問わず緑で、置き場も遅い
- 成果物: openspec/changes/st05-clock-skew/design.md（D8）/ tasks.md（7.1 / 8.3）
- 根拠:
  - Scenario `specs/desktop-collection/spec.md:59-62` は「最後に時刻を合わせた時刻と同期元を持つ」。D8 の反転先 `design.md:164-165` の
    `W32Time\Parameters\NtpServer` / `Type` は**設定**（同期元の候補）で、最後に同期した時刻ではない。`GetSystemTimeAdjustment` も調整量で、同期の時刻ではない。
    反転しても Scenario は満たせないが、D8 は「独立な比較が 1 つも残らないなら」だけを本人へ返す条件にしている。設定だけでは独立な比較にもならない
  - 7.1 `tasks.md:127-131` の `clock_time_sync_is_readable` は「読めたか・読めなかった理由を**そのまま assert**」—— 読めても読めなくても rc=0 で、
    D8 を反転するかの判断材料がどの artifact にも残らない（2.1 は `docs/collector-contract.md` に書かせているのと対照的）。
    8.3 `tasks.md:157` の `clock_skew_runtime` も「`clock_references` か `clock_unavailable` に持つ」で、どちらでも緑
  - D8 は「下流の最初に確かめる」（`design.md:163`）が、7.1 は Task 7（Android の Task 2〜6 の後）。反転の先が本人への問い（A）なら、Android を全部終えてから止まる
- kind: technical
- 提案: 反転先を「最後に同期した時刻と同期元が読める別の口（イベントログの Time-Service の記録など）を探し、無ければ本人に返す」に直す。
  7.1 は結果（読めた / 読めない・エラー符号・表示言語）を `docs/collector-contract.md` か `design.md` の D8 に書かせ、その grep を検証にする。
  7.1 を下流の最初（Task 1 の前後）へ移す
- 処置: fixed D8 —— kind を premise から technical に直した: 崩れたのは design の反転先の選び方で、本人の決定ではない。反転先を「最後に同期した時刻と同期元が読める別の口（System のイベントログの Time-Service の記録）。設定・調整量は代わりにしない。どれも読めなければ Q3 ③ の前提が崩れるので deep.md に (未回答) で書いて止まる」に直した。確かめを Task 1（1.3）へ移し、実行時テスト `clock_time_sync_is_readable` は読めなければ落ちる形にし、結果を design の D8 に「w32tm の確かめ」として書かせて grep を検証にした。9.3 の `clock_skew_runtime` も `clock_references` に `windows-time-sync` があることを要求する形にした

## R8. `w32tm` の子プロセス（最大 5 秒）を 1 秒の見回りの輪の中で走らせると、その間の前景の切り替えが落ちる。Scenario はそれを捕まえない
- 成果物: openspec/changes/st05-clock-skew/design.md（D8 / D10 / D11）/ specs/desktop-collection/spec.md
- 根拠: `crates/collector-windows/src/runtime.rs:38` `POLL_INTERVAL_SEC = 1`。D11 `design.md:197`「見回りの輪を 5 秒より長く止めない」（= 5 秒は止める）。
  D10 `design.md:190` は取れない間の測り直しを 60 秒ごとのまま —— 取り込み口に届かず `w32tm` が打ち切られる状態では、**毎分 5 秒**輪が止まる。
  Scenario `specs/desktop-collection/spec.md:95-98`「基準の読み取りが失敗し続ける状態でアプリを切り替える → 切り替えの記録は 1 件ずつ残る」は、
  切り替えが止まっている 5 秒の外で 1 回起きれば緑
- kind: technical
- 提案: `w32tm` を輪の外（別スレッド・前回の結果を持つ）で読むか、打ち切りを見回りの間隔未満にする。Scenario を
  「基準の読み取りが打ち切りまでかかる状態で、その間に切り替えたアプリも記録に残る」に絞る
- 処置: fixed D7 —— 基準の読み取り（`/healthz` と w32tm）を 1 本の作業スレッドで行い、見回りは通り道を覗くだけにした（D7 / D11）。Scenario を `基準の読み取りが長引いても前景の切り替えは記録に残る`（5 秒止める偽物の間の切り替えが残る）と `測定が失敗し続けても送信は続く` に分けた（8.3）

## R9. 2.1 / 2.3 / 4.3 の検証コマンドはハーネスが抽出できない（`grep` は検証コマンドとして認識されない）。6.1 / 8.3 の grep も黙って落ちる
- 成果物: openspec/changes/st05-clock-skew/tasks.md
- 根拠: `scripts/evidence.py:41` の `CMD_LIKE` は `./ tools/ scripts/ cd python cargo bash npm … make` で始まるものだけを検証コマンドにする。
  `python3 scripts/evidence.py unverified … --task 2` → `["2.1", "2.3"]`、`--task 4` → `["4.3"]`（抽出 0 本）。
  6.1 `tasks.md:123` の `grep -c 'Scenario: 収集の起動時にその場で測る' …ClockSkewInstrumentedTest.kt` と 8.3 `tasks.md:161` の
  `grep -c "clock_references" docs/collector-contract.md` は抽出から外れ、`tools/android-emulator.sh` / 実行時テストだけが走る
- kind: technical
- 提案: `bash -c 'grep -q … file'` の形にする（`3.2` / `7.3` と同じ）。`grep -c … が 1 以上` は `grep -q` で rc に写す
- 処置: fixed 3.2 —— grep の検証をすべて `bash -c 'grep -q …'` の形にした（1.1 / 1.2 / 1.3 / 3.2 / 5.3 / 7.1 / 9.3 / 10.1 / 10.3）。`python3 scripts/evidence.py unverified openspec/changes/st05-clock-skew --task N` は N=1〜10 で全部 `[]`

## R10. 検証が変更の前から緑になる項目が 6 つある（0 本 rc=0 の同類）
- 成果物: openspec/changes/st05-clock-skew/tasks.md
- 根拠:
  - 2.2 `tasks.md:62` `--tests '*LocationFix*' --tests '*FixCollector*'` —— 既存の `LocationFixTest.kt` / `FixCollectorTest.kt` に一致するので、新しい 3 項目の試験が無くても rc=0
  - 3.1 `tasks.md:72` `--tests '*ResponseDateCache*' --tests '*SenderTest*'` —— 既存の `SenderTest.kt` があるので `ResponseDateCache` の試験が 0 本でも rc=0
    （Gradle の「一致 0 本で失敗」はフィルタ全体での判定）
  - 8.1 `tasks.md:150` の `clock_skew_` —— 既存の `clock_skew_is_measured`（`crates/collector-windows/src/clock.rs:164`）に一致するので rc=0
  - 1.2 `tasks.md:48` の `clock_source_` で「2 本以上」—— 1.1 の `clock_source_migration` も一致するので、1.2 の試験は 1 本で通る（Scenario は 2 本）
  - 8.2 `tasks.md:155` の `clock_skew_` で「6 本以上」—— 8.1 の試験も同じ絞り込みに入るので 8.2 の 6 本を保証しない
  - 2.2 / 3.1 / 4.1 / 5.1 の「試験が 1 本ある」（`tasks.md:63` / `:73` / `:92` / `:106`）は文で、コマンドではない
- kind: technical
- 提案: 絞り込みを新しい試験だけに一致する名前にする（例 `--tests '*LocationFixReceivedTest*'`、`clock_source_accepts_`、`clock_skew_payload_`）。
  件数は Gradle の XML（`build/test-results`）か `cargo test` の passed で数える。文の条件はコマンドに写す
- 処置: fixed 3.1 —— 絞り込みを新しい試験だけに一致する名前にした（Gradle は `LocationFixClockFieldsTest` / `ResponseDateCacheTest` / `ClockReferencesTest` / `ClockSkewMeasurerTest` / `ClockSkewPayloadShapeTest` / `ClockSkewSchedulerTest` / `LocationServiceClockTest` を 1 つずつ、cargo は `clock_source_migration` / `clock_record_ingest_` / `time_sync_` / `clock_reference_` / `clock_worker_` / `clock_skew_payload_` / `clock_skew_record_`）。既存と重なる `FixCollectorTest` / `SenderTest` / `clock_skew_is_measured` は「直す・緑のまま」の検証として別に書いた。「試験が 1 本ある」の文は下限つきの件数か、Scenario の印の grep に写した

## R11. 設計を覆しうる確かめが、それに依る Task より後にある。実機 1 週間の反転条件に観測する係がいない
- 成果物: openspec/changes/st05-clock-skew/tasks.md / design.md（D3 / D4 / D7）
- 根拠: D3 の反転（`ACTION_TIME_CHANGED` が届かなければ 5 分の刻みの比較へ。`design.md:79-80`）を判定するのは 6.1（`tasks.md:116-120`）だが、
  その方式で作る 5.1 / 5.2 が先。D8 の 7.1 は R7。D4 `design.md:89`・D7 `design.md:151` の反転条件は「実機の 1 週間で…」だが、
  tasks にも確認バッチにもそれを観測する項目が無く、Global Constraints は「人間の確認待ちに逃がせる Scenario は 1 本も無い」（`tasks.md:22`）
- kind: technical
- 提案: 6.1 の「時計の変更が届くか」だけを Task 5 の前に切り出す。D4 / D7 の反転条件は、観測する場所（`docs/verify/` の手順書か、
  `c01-clock` の記録を数える SQL）と時期を書くか、観測できないなら反転条件から外す
- 処置: fixed 1.2 —— 設計を覆しうる確かめ（Location.getTime の出どころ・ACTION_TIME_CHANGED が届くか・w32tm が読めるか）を Task 1 にまとめて先頭に置いた。D7 の実機 1 週間の反転条件は D7 を ST07 のままにしたので消えた。D4 の反転条件は観測する係を置いた（10.3: `tools/verify-prep.sh` が `c01-clock` の retry / hourly の件数を手順書に載せる）

## R12. 観測可能な振る舞いが design にだけある（archive で正典から落ちる）
- 成果物: openspec/changes/st05-clock-skew/design.md（D1 / D2 / D3 / D5 / D8）
- 根拠:
  - D3 `design.md:77` —— 記録に「何の契機で測ったか」（`hourly` / `start` / `time_set` / `retry`）を残す。これが無いと spec の「測り直しで取れたら別の 1 件」
    （`specs/device-collection/spec.md:127-130`）の記録を、1 時間の契機の記録と記録の上で見分けられない
  - D5 `design.md:116` —— 3 つの出どころが `references` と `unavailable` のどちらかに必ず 1 回ずつ出る（取れなかった理由の無い欠けが起きない）
  - D2 `design.md:56` / Risks `:204` —— 「OS の版で取れない（`unsupported`）」と「いま取れない」を理由で見分ける（「後から OS の版のせいと分かる」が Risks の論拠）
  - D8 `design.md:158` / Risks `:208` —— Windows の同期状態の出力を原文のまま持つ（「解析を後から直せば過去の記録も読み直せる」。持たなければ uncaptured）
  - D1 `design.md:46` —— 生存信号を送らない（R5）
- kind: technical
- 提案: 上の 5 つを spec の本文と Scenario に上げる（名前・欄名ではなく振る舞いの言葉で）。値の綴り（`unsupported` など）は design / 契約文書のままでよい
- 処置: fixed D5 —— 5 つを spec に上げた: 測った契機（`測った契機が記録に残る` / `PC の測定記録に測った契機が残る`）、出どころの網羅（`3 つの出どころは取れたか取れなかったかのどちらかに 1 回ずつ出る` / `2 つの出どころは…`）、OS の版の区別（`OS の版が対応していない基準はいま取れない基準と区別して残る`）、w32tm の原文（`Windows の時刻同期の状態は読んだままの出力が残る` / `項目を読み取れなかった出力も残る`）、生存信号を送らない（`測定記録の論理ソースは生存信号を送らない`）。値の綴りは design と契約文書のまま

## R13. 「起動の識別を持つ」Scenario と、design の「取れなければ `null`」が食い違う
- 成果物: openspec/changes/st05-clock-skew/specs/device-collection/spec.md / design.md（D5 / D6）
- 根拠: Scenario `specs/device-collection/spec.md:42-45`・`:117-120`・`:203-206` は起動の識別を「持つ」。design `design.md:115`「取れない端末では `null`」、
  `:131`「取れなければ `null`」（`AgeClock.kt:19` `fun bootCount(): Int?`）
- kind: technical
- 提案: spec に「起動の識別が OS から取れない端末では、取れないことを示す値を持つ」と Scenario を足すか、Scenario の THEN を「欄を持つ（値は取れなければ空）」に直す
- 処置: fixed D5 —— spec の本文に「起動の識別を OS から取れないときは、取れないことを示す値を持たせる」を書き（測定記録と位置の記録の両方）、Scenario `起動の識別が取れない端末では取れないことが残る` を足した。位置の Scenario の THEN にも「取れない端末では取れないことを示す値」を書いた

## R14. D3 の反転先は 60 秒未満の時計の変更で測らないのに、「spec は変わらない」としている
- 成果物: openspec/changes/st05-clock-skew/design.md（D3）/ specs/device-collection/spec.md
- 根拠: `design.md:79-80` の反転先は「前回の壁時計と単調時計の進みの差が 60 秒以上」。Scenario `specs/device-collection/spec.md:52-55`
  「端末の時計が変更されると…待たずに 1 件」には閾値が無いので、反転後は 30 秒の変更で落ちる。PC 側は Scenario に 60 秒がある（`specs/desktop-collection/spec.md:46`）
- kind: technical
- 提案: 端末の Scenario にも閾値を書く（反転してもしなくても成り立つ値）か、D3 の「spec は変わらない」を消して反転時に spec を直すと書く
- 処置: fixed D3 —— 端末の Scenario の WHEN を「60 秒以上変更される」にした（OS の通知でも反転先の見回りでも成り立つ）。D3 に「OS は変更の大きさによらず通知するので 60 秒以上を満たす」を書いた

## R15. 観測の言葉になっていない THEN と、許容の無い「約」
- 成果物: openspec/changes/st05-clock-skew/specs/desktop-collection/spec.md / specs/device-collection/spec.md
- 根拠:
  - `specs/desktop-collection/spec.md:66-68` THEN「出どころから同じ機械だと**分かる**」—— 何が記録にあれば真かが無い。D7 `design.md:150` は「ループバックなら」だが、
    基点 URL が同じ機械の Tailscale の名前ならループバックにならず「分からない」
  - `specs/desktop-collection/spec.md:40-42`「差が**約** 300000」—— 許容が無い（端末側 `:34` は幅を書いている）
  - `specs/device-collection/spec.md:26-29`「1 時間が経過すると 1 件増えている」—— 取れない状態だと測り直しの 1 件が足されうる。基準が取れる状態を WHEN に書いていない
  - `specs/device-collection/spec.md:83-86`「測定記録だけの日は位置の記録がある日にならない」の THEN「位置の論理ソースの記録は 0 件のまま」は `:78-81`（別のソース）から自明で、
    C1 の理由（成功条件 1 の「端末が主語の日」に数えられない）を観測していない。`coverage.rs:28` の `DEVICE_SUBJECT` に `c01-clock` を足しても緑
- kind: technical
- 提案: 同じ機械の Scenario は「取り込み口の出どころに、宛先の名前と解決した IP が入る」など記録の欄で言う。PC の「約」に幅を書く。
  1 時間の Scenario の WHEN に「どれかの基準が取れる状態で」。成功条件の Scenario は稼働状況の達成日で言う（`collection-coverage` にまたがるなら理由を書く）
- 処置: fixed 2.2 —— 同じ機械の Scenario は「取り込み口の宛先と Windows の時刻同期の状態が同じ記録に並ぶ」に直した（「分かる」を消した）。PC の差に幅（300000 以上 301000 未満 ± 読む前後の幅）を書いた。1 時間の Scenario の WHEN に「どれかの基準が取れる状態で」。成功条件の Scenario は `測定記録だけの日は端末が主語の達成日にならない`（稼働状況の数えで見る。2.2 の試験は `coverage.rs` の数えを通す）に直した

## R16. 1 つの Scenario に主張が 2 つ（片方だけで緑になる）
- 成果物: openspec/changes/st05-clock-skew/specs/device-collection/spec.md / specs/desktop-collection/spec.md
- 根拠: 端末 `:31-35`（進み 5 分 ＋ 遅れは負）、`:62-66`（端末の基準で測る ＋ S-01 が理由つきで残る）、`:111-115`（印つき 1 件 ＋ 理由）、`:146-150`（位置が続く ＋ ログ）。
  PC `:38-42`（正 ＋ 負）、`:64-68`（同じ機械と分かる ＋ 同期状態が並ぶ）、`:75-78`（並ばない ＋ 理由つきで残る）、`:80-83`（1 件 ＋ 理由）、`:95-98`（切り替え ＋ 送信）
- kind: technical
- 提案: AND の後ろを別の Scenario に分ける（特に「負になる」と「理由つきで残る」は実装の分岐が別）
- 処置: fixed 5.1 —— AND で束ねた Scenario を分けた: 進み / 遅れ（端末・PC とも）、端末の基準で測る / 応答の日付が理由つきで残る、印つき 1 件 / 基準ごとの理由（端末・PC とも）、位置が続く / ログは種別だけ、並ばない（PC は D7 で消えた）、Windows の同期の状態の原文 / 読み取れなかった原文、時刻を同期させない / 時刻サーバへの経路が無い、切り替え / 送信。残る AND は正典から写した `端末識別子が端末をまたいで一意である` だけ

## R17. Requirement の本文だけが言い、Scenario が言っていないこと
- 成果物: openspec/changes/st05-clock-skew/specs/device-collection/spec.md / specs/desktop-collection/spec.md
- 根拠:
  - 端末 `:16`「保持の上限に乗せる」—— Scenario は `:88-91`（後から届く）だけ。90 日・2 GB で `c01-clock` も捨てられ破棄の報告になることを見ない
  - 端末 `:17`「格納時刻を補正しない」、PC の本文には C4（補正しない）自体が無い
  - 端末 `:141`「ログに時刻の値・位置の値を出さない」—— Scenario `:146-150` は「種別が残る」だけ。D5 `design.md:120` の「差を出さない」も同じ
  - PC `:10`「OS が示すなら OS の見積もったずれも」—— Scenario `:59-62` に無い（proposal `:31` は 3 つ並べると書く）
  - Q1 は R1
- kind: technical
- 提案: ログの Scenario は「測定が失敗したログに時刻の値と差が含まれない」を別に立てる（ST08 の「題名と URL は出ない」と同じ形）。
  保持の上限は ST04 の Scenario に `c01-clock` が乗ることを 1 本で示す。OS のずれは「読めたときは並ぶ」を 1 本
- 処置: fixed 6.2 —— Scenario を足した: 保持の上限（`測定記録も保持の上限で捨てられ破棄として報告される`）、ログ（`測定のログに時刻の値と差が出ない` / `測定の失敗は種別だけがログに残る`）、PC の補正しない（本文に C4 と `PC の記録の時刻は補正されない`）、OS の見積もったずれ（`OS の見積もったずれが読めたときは並ぶ`）

## R18. 重なりの表の前提が ST08 の実際の差分と違う（`runtime.rs` は触っておらず、`contract.rs` を両方が触る）。影響範囲に漏れ
- 成果物: openspec/changes/st05-clock-skew/proposal.md / design.md（Risks）
- 根拠: `git diff --stat main...origin/feat/st08-browser-history -- crates/collector-windows` は `contract.rs`（+105）・`heartbeat.rs`・`telemetry.rs`・`exclusion.rs` 等で、
  **`runtime.rs` と `main.rs` は 0 行**。一方 proposal `:87` / design `:203` は「ST08 は `runtime.rs` に 2 本目のソースを載せる」を前提に対策を書き、
  ST05 の D10（`WindowPayload` に欄を足す。`design.md:177`）が触る `contract.rs` を重なりに挙げていない。proposal の Impact `:73` も `contract.rs` を落としている。
  また 7.3 で消す `ReferenceClock` は `crates/collector-windows/tests/runtime_windows.rs:539` の `NoReference` と `main.rs:100` が使っているが、tasks は触ると書いていない
- kind: technical
- 提案: 重なりの表と Impact を実際の差分で書き直し、`contract.rs` の `payload_shape_is_pinned` を ST08 も「最初から最後まで緑」の検証にしている
  （`openspec/changes/st08-browser-history/tasks.md:22`）ことを書く。7.3 に `main.rs` と `tests/runtime_windows.rs` を名指す
- 処置: fixed design.md —— `git diff --stat origin/main...origin/feat/st08-browser-history -- crates/collector-windows` と ST06 の同じ差分を自分で取り直し、design の Risks と proposal の重なりの表を実際の差分で書き直した（ST08 は `contract.rs` を触り `runtime.rs` / `main.rs` は 0 行。ST06 は `LocationService` / `FixCollector` / `Sender` / `AgeClock` / `IngestRequest` を触る）。`payload_shape_is_pinned` を ST08 も緑のままにしていることを Global Constraints に書いた。8.2 に `tests/runtime_windows.rs` の `NoReference` と `main.rs` を名指しした

## R19. ST05 の `satisfies` は FR-7 だけだが、spec は FR-1 の ★ の節を満たす
- 成果物: docs/stories/ST05.md / docs/stories/INDEX.md
- 根拠: `docs/stories/ST05.md:4` `satisfies: [FR-7]`。spec の MODIFIED「位置は 60 秒間隔で記録される」（導出元 FR-1。`specs/device-collection/spec.md:168-177`）と
  完了の判定 `ST05.md:47`（位置の記録の 3 項目）は FR-1 ★ 2026-09-15（`docs/requirements.md:81-86`）を満たす。FR-1 を満たす ST01 は archive 済みで、
  ★ の節を誰が満たすかが鎖に無い（`check_chain.py` は FR-1 が ST01 に拾われているので OK を出す）。INDEX の訂正 2026-09-15（`INDEX.md:218-229`）は capability の理由で、satisfies の理由ではない
- kind: technical
- 提案: `stories.json` の ST05 の `satisfies` に FR-1 を足して再生成するか、INDEX に「FR-1 ★ の節は ST05 が満たす（前倒し）」と書く
- 処置: fixed proposal.md —— `docs/stories/stories.json` の ST05 の satisfies に FR-1 を足し、`make_story.py` で再生成した（`check_chain.py` OK）。`INDEX.md` の ST05 の行と、訂正 2026-09-29 に理由（FR-1 ★ の節は ST05 の Q2 の決定で、ST05 が MODIFIED して満たす）を書いた

---

観点 3（specs に実装の名前）: 該当なし（確かめた範囲: 両 spec の本文・Scenario に関数名・crate 名・列名・欄名は無い。`S-01` / `C-01` / `C-02` は要件の記号）。
観点 3（proposal と specs のディレクトリ）: 一致（`device-collection` / `desktop-collection`）。INDEX の表も一致（`INDEX.md:68-70`、record-envelope から外した訂正の理由あり）。
観点 1（要件へ戻すもの）: FR-7・FR-1 とも `docs/requirements.md:83` / `:128` に ★ 2026-09-15 あり。当初案を覆した旧い内容（3 択・「衛星由来は衛星の時計」）は Scenario に残っていない。
観点 5（再生成との一致）: `check_chain.py` OK。ST05.md の価値・完了の判定は deep の決定（Q2 で価値の文を変えない）と矛盾しない。
