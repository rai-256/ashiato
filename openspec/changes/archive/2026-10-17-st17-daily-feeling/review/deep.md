# ST17 の深掘りの独立レビュー（deep-review）

対象: `openspec/changes/st17-daily-feeling/deep-questions.json`（11 問）/ `deep.md`（C1〜C10）/ `proto.html`

schema の手順 1〜5 は、問いの一覧を開く前に自分でやり直した。一覧と重なったものは書かない。

- 手順 1（衝突）: 自分で挙げたのは FR-58×主観の文、FR-43×書いた日、FR-39×FR-76（前に書いた記録の紐づけ先）、
  **FR-19（出来事が起きた時刻）×FR-41（記入日時）**、FR-57 の「記録が無い日」の範囲の 5 件。前の 3 件は Q2 / Q8 / Q5 にある。残りの 2 件は R1 と R9
- 手順 2（扉 #1 #2 #6 #15）: 尺度（Q3）・文（Q2）・滞在の口（Q4）・吸収（Q10）・感度（C6）は一覧にある。扉 #6 の幅（出来事の時刻の列に何を入れるか）は R1
- 手順 3（一方通行）: **鍵の入力**（`content_hash` = 論理ソース・`event_time`・原文。`crates/server/src/ingest.rs:178-189`）は R1 / R2。
  書き直しの上書き（C2）・外へ出るもの（Q6 / C9）は一覧にある
- 手順 4（日常）: 通知（Q6〜Q8）・夜中の既定（Q9）・入力画面（Q1）は一覧にある。proto の見せ方の誤りは R4、通知の時刻を変える場所の前提は R8
- 手順 5（既存コード）: grep で確かめた。`default_sensitivity` が個人属性にしか分岐しない（`crates/server/src/lib.rs:344-351`）、
  「本人が書いた」記録は書き換えられる（`tools/check-immutable.sh:796`）、収集アプリが画面を持たない（`MainActivity.kt:13`）、
  Web に S-3 の経路が無い（`web/src/Root.tsx`）。この 4 件は deep.md の手順 5 と合っている。合わなかったものは R3 / R6 / R7 / R10 / R11 / R12

---

## R1. 出来事の時刻（`event_time`）に記入日時を入れる（C3）のは FR-19 と食い違う。この列は凍結され、鍵の入力にもなるので、後から変えると全行の書き直しになる
- 成果物: openspec/changes/st17-daily-feeling/deep.md（C3）/ deep-questions.json（該当する問いが無い）
- 根拠: docs/requirements.md:265-266（FR-19「出来事が起きた時刻」と「D-01 に入った時刻」）/ docs/requirements.md:447（FR-41「対象の日付とは**別に**記入した日時」）/
  `crates/server/src/ingest.rs:178-189`（`content_hash_of` は `event_time` を混ぜる）/
  `migrations/202609160220_personal_attributes.sql`（ST19 の錠は `event_time` の書き換えを拒み、門は消去でも `content_hash` が変わらないことを求める。C2 はこれと同じ形にすると書いている）/
  openspec/changes/st25-day-timeline/design.md:110（1 日の一覧は `event_time ∈ [d0, d1)` で記録をその日に置く）
- kind: conflict
- loss: rewrite-all
- 提案: C3 から外して A の問いを 1 つ立てる。「主観の `event_time` に何を入れるか（記入日時 / 対象の日の始まり（Asia/Tokyo）/ 滞在の始まり）」。
  context には次の 3 点を書く。(1) 記入日時を入れると、翌朝に書いた昨日の分が `event_time` で日を引く読み手（ST25 の一覧、QS-8 を答える ST27）では今日に出る。
  (2) 対象の時刻を入れると、FR-41 の記入日時は payload にしか残らない。(3) どちらを選んでもこの列は錠と鍵で固まり、後から変えると凍結した全行の書き直しになる。
  ST19 が主張した日時を `event_time` に入れたのは C4（FR-45 が根拠）で、主観には同じ根拠が無い
- 処置: escalated — C3 から外し、A の問い Q12（loss: rewrite-all。推奨: 対象の時刻）を立てた。context に ST25 の一覧・QS-8・ST19 の根拠の違いを書いた。deep.md の手順 1 と C3 を直した
## R2. C2 に原文の乱数（ST19 の C12）が無い。快–不快は 5 値しか無いので、本文を消した後に残る列から値を当て直せる
- 成果物: openspec/changes/st17-daily-feeling/deep.md（C2）
- 根拠: deep.md の C2 は「原文に記録ごとの識別子を入れ、内容で畳まない」だけ / openspec/changes/archive/2026-09-17-st19-personal-attributes/deep.md:103（C12「識別子は `id` の列に残るので鍵を守らない」）/
  同 design.md:100-107（D4: 128 bit の乱数を原文にだけ入れる）/ `crates/server/src/attributes.rs:24-28`（`NONCE_MIN_CHARS`）/
  `crates/server/src/ingest.rs:178-189`（鍵 = 論理ソース・`event_time`・原文）。消去（FR-51）の後も `id`・`event_time`・`content_hash` は残るので、
  値の候補が 5 つで対象の日も推せるなら、総当たりは数十回で済む
- kind: technical
- loss: rewrite-all
- 提案: C2 に「原文に、他の列へ写さない 128 bit の乱数を入れる（ST19 の C12 / D4 と同じ）」を足す。人間に聞く必要は無く、C のままでよい。
  ただし最初の行が入った後に鍵の入力を変えると、凍結した `content_hash` を全行で作り直すことになる（消去済みの行は門が `content_hash` の変更を拒むので、作り直しもできない）。
  したがって design より前に C に書いておく
- 処置: escalated — deep.md の C2 に乱数を足し（ST19 の C12 / D4 と同じ）、問いの HTML の冒頭に C の一覧として「最初の 1 件より前に決める。後から変えると全行の作り直し」と載せ、異論を番号で受ける形で本人に返した（問いは立てない。推奨と違う側を選ぶ理由が見当たらないため）
## R3. Q5 の context にある「予定を後で滞在に付け直す口は ST18 の範囲」は誤り
- 成果物: openspec/changes/st17-daily-feeling/deep-questions.json（Q5 の context）
- 根拠: docs/stories/ST18.md（satisfies は FR-38 だけ。「新しい紐づけ先の**種類**を登録する」。完了の判定は「既存の主観記録が 1 件も変わらない」）/
  deep.md の C2（主観の行の書き換えを DB が拒む）。付け直しを担う Story も要件も無い
- kind: premise
- 提案: context を直す。付け直しの持ち主は無い。付けるとすれば、前の記録を指す新しい記録を積む形（ST19 の訂正と同じ）になる。
  この Story で持つかどうかが QS-5 の答えやすさに効くので、選択肢の detail にも書く。本人は「後で滞在に付け直せる」と読んで Q5 を選びうる
- 処置: escalated — Q5 の context を「付け直しの持ち主は無い。付けるなら前の記録を指す新しい記録。この Story では作らない」に直した
## R4. proto の「翌朝 8:10 に昨日の分」が、どの要件にも問いにも無い 2 回目の通知を描いている。今日（10/3）も記録済みのデータで 21:30 の通知を出している
- 成果物: openspec/changes/st17-daily-feeling/proto.html
- 根拠: proto.html:194（`when === "yday"` のとき「あしあと · 8:10 / 昨日（10/2）の分がまだです」という通知の帯を描く）/ docs/requirements.md:449（FR-43「1 日 1 回」）/
  Q7 は時刻を 1 つしか問わない / Q1 の context は「昨日を書いていない朝に開いた場合」と書くが、絵は通知になっている。
  あわせて proto.html:128-129 の `seq[29] = 1` で、10/3 は記録済み（+1）になっている。Q8 の推奨（書いてあれば鳴らさない）を採ると、21:30 の通知は鳴らない
- kind: premise
- 提案: 翌朝の文脈は通知の帯を出さず「自分で開いた」として描く。朝の追い通知を持つかどうかを問うなら、別の問い（B / daily）にする。
  10/3 は未記入にするか、通知の帯の横に「Q8 で鳴らさない側を選ぶとこの通知は出ない」と添える。本人は Q1 をこの絵で選ぶので、絵の誤りがそのまま答えに入る
- 処置: escalated — proto の翌朝の文脈を「自分で開いた」に直し通知の帯を消した（FR-43 の 1 日 1 回を明記）。10/3 を未記入にした（未記入 9 日）。Q1 の context を直した
## R5. Q4 の選択肢 1 と 2 は画面の構造の違い（S-3 に滞在の一覧を並べるか、S-2 の行の中で書くか）なのに、文字で問うていて proto に無い
- 成果物: openspec/changes/st17-daily-feeling/deep-questions.json（Q4）/ proto.html
- 根拠: Q4 の選択肢 1 の detail「S-3 に『その日の滞在』（時刻の範囲）を並べ、選んで快–不快を押す」/ proto.html の `AXES` には滞在の軸が無く、S-3 に滞在の一覧が出る形を描いていない（proto.html:141-159）/
  docs/ui-direction.md:197-202（S-2 の 1 行は本文を行の中に出す。最も高い行は 171 px、1 画面に 6 行）。S-3 に 1 日 5〜15 件の滞在を足すと、30 秒の画面の縦が変わる
- kind: daily
- 提案: Q4 を 2 つに分ける。(a) 滞在の口をこの Story で作るか（A / uncaptured。いまの選択肢 3 と、作る側）。
  (b) 置き場（`visual` / proto に「滞在に書く」の軸を足し、S-3 に並べたときの高さと手数を見せる）。capability の代償（`browsing-views`）は (b) の context に移す
- 処置: escalated — Q4 を「作るか」（A / uncaptured）だけにし、置き場を proto の軸 5（S-3 に並べる / S-2 の行の中）として Q1 に移した。capability の代償は Q1 の context へ
## R6. Q5 の選択肢 1 と 3 の「何が不可逆か」が「—」になっているが、detail は本人のつもりが残らないと書いている
- 成果物: openspec/changes/st17-daily-feeling/deep-questions.json（Q5 の options[0] / options[2]）
- 根拠: Q5 の options[0].irreversible は「—（本人の『つもり』は残らないが、時刻は残るので数え方は変えられる）」。options[2] も前/後は機械で決める側で、つもりが残らない点は同じ。
  Q5 の context も「失われうるのは『書く口があったか』と『本人が前と思って書いたか』の 2 つ」と書いている。
  さらに選択肢 1 では、当日 21:00 に書く毎日の記録がすべて「前」になる（options[0].detail）
- kind: irreversible
- loss: uncaptured
- 提案: options[0] と options[2] の irreversible に「本人が前（期待）と後（振り返り）のどちらのつもりで書いたかは残らない。当日の夜に書いた記録も時刻の上では『前』になる」と書く。
  「—」は選択肢 2 だけにする
- 処置: escalated — Q5 の選択肢 1 と 3 の irreversible に「本人のつもりは残らない。当日の夜の記録も時刻の上では前」を書いた
## R7. Q6 の選択肢 1 にある「網に届かないときは、画面が選んだ値を端末に残して再送する」は、いまの Web では成り立たない
- 成果物: openspec/changes/st17-daily-feeling/deep-questions.json（Q6 の options[0]）
- 根拠: `grep -rln "serviceWorker\|manifest.webmanifest\|workbox\|localStorage\|indexedDB" web/src web/index.html web/vite.config.ts` は 0 件。
  画面は自宅の PC から配られる（tools/stack.sh）ので、S-01 に届かないときは S-3 の HTML 自体が開かない。値を選ぶ前の段階で止まる
- kind: premise
- 提案: detail を「網に届かないときは S-3 が開かない。後で過去の日として書く（FR-42。記入日時は後の時刻になる）。端末に残すには画面をオフラインで開ける形（PWA）にする追加が要る」に直す。
  これで選択肢 2（アプリの未送信に積む）との違いが読める。失われるものは無いので B のままでよい
- 処置: escalated — Q6 の選択肢 1 の detail を「網に届かないと S-3 が開かない。後で過去の日として書く。端末に残すには PWA の追加が要る」に直した
## R8. Q7 の「携帯の設定で変えられる」の置き場が無い。収集アプリは画面を持たず、設定画面（S-7）は Web にある
- 成果物: openspec/changes/st17-daily-feeling/deep-questions.json（Q7 の question / context）
- 根拠: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:13（「画面は持たない —— 表示は V-01（web）の仕事」）/ docs/ui-direction.md:23（S-7 設定は Web）。
  時刻を Web で設定すると、携帯がサーバから設定を読む経路が要る（R9 と同じ経路の話）
- kind: premise
- 提案: context に「通知の時刻を変える場所はまだ無い。収集アプリに設定画面を足すか、Web の S-7 で設定して携帯が読むか」を書く。
  どちらにするかは Q6 の答えで決まるので、Q6 の選択肢の detail にも入れる
- 処置: escalated — Q7 の context に「時刻を変える場所はまだ無い。Q6 の答えで決まる」を書き、Q6 の各選択肢の detail に設定の置き場を書いた
## R9. Q8 の推奨は、収集アプリの合言葉で主観の有無を読む経路に依っている。その経路は ST29 で絞る予定になっている
- 成果物: openspec/changes/st17-daily-feeling/deep-questions.json（Q8 の context）
- 根拠: Q8 の context「携帯が通知の時刻にサーバへ問い合わせる」/ openspec/changes/st28-private-network-only/deep.md:88-100（Q4: 収集側の合言葉は ST29 まで全読み。本人が推奨を覆した）/
  docs/handoff/ST29.md（「収集側の資格情報が読める範囲を ST29 で絞る」）/ docs/requirements.md:663（PERM-4: 主観は「ローカル AI まで」）。
  推奨の形は、APK の合言葉で主観の記録に触れる読み出しを 1 本足すことになる
- kind: premise
- 提案: context に、この依存と ST29 で絞られうることを書く。そのうえで、問い合わせの返事を「その日の記録の有無」だけに限る口を置くのか、`/events` を使うのかを design へ渡す、と明記する。
  Q8 自体は B のままでよい
- 処置: escalated — Q8 の context に、収集アプリの合言葉で有無を読む経路と ST29 で絞られうること、返事を有無だけに限る口かは design で決めることを書いた
## R10. 本人が消した滞在の主観について、Q10 の context が ST22 の決定と食い違う。主観は消した記録と一緒には消えない
- 成果物: openspec/changes/st17-daily-feeling/deep-questions.json（Q10 の context / options[0]）
- 根拠: Q10 の context「本人が消した滞在（ST22）は、消した時間帯の記録と一緒に画面から消える」/
  openspec/changes/st22-record-deletion/design.md:106（「位置以外（PC のウィンドウ・ブラウザ履歴・写真・**主観**）は触らない（Q1 の本人の答え）」）。
  主観の行には削除の印が付かず `core.event_live` に残るので、ST26（検索）・ST27（AI）・書き出しからは見える。隠れるのは ST17 の読み方を通る画面だけ
- kind: premise
- 提案: context を「消した滞在に付けた主観は消えない（ST22 の Q1）。この問いで決めるのは、S-2 / S-3 で滞在と一緒に隠すかどうかだけ」に直す。
  options[0] の「隠れる」も「この画面では隠れる（記録は残り、検索と AI からは見える）」に直す
- 処置: escalated — Q10 の context と選択肢 1 を「消した滞在の主観は消えない（ST22 Q1）。この画面で隠すかだけ」に直した
## R11. Q1 の context が、`ui-direction.md` で FR-57 が S-2 の担当になっていることを書いていない
- 成果物: openspec/changes/st17-daily-feeling/deep-questions.json（Q1 の context）
- 根拠: docs/ui-direction.md:18（S-2 主表現の「触れる要件」に FR-57）。Q1 は ui-direction.md の「S-3 の細かい入力手順は Story の担当」（同 :224）だけを引いている。
  proto の軸「S-3 に 1 か月の暦（S-2 には出さない）」（proto.html:156）を選ぶと、ui-direction.md の割当から外れる
- kind: premise
- 提案: context に「ui-direction.md は FR-57 を S-2 に置いている。S-3 だけを選ぶなら ui-direction.md の表を直す（record の段で戻す）」を書く。
  admission の代償（ST22 / ST25 待ち）は、その割当と並べて読めるようにする
- 処置: escalated — Q1 の context に「ui-direction.md は FR-57 を S-2 に置く。S-3 だけなら record で表を直す」を書き、代償と並べた
## R12. 「主観の記録が無い日」（FR-57）の範囲を問う問いが無い
- 成果物: openspec/changes/st17-daily-feeling/deep-questions.json（該当する問いが無い）
- 根拠: docs/requirements.md:566（FR-57「主観の記録が無い日」）/ docs/requirements.md:440（FR-37 の紐づけ先は日か滞在）/ Q4（滞在だけに書いた日がありうる）/ Q5（未来の日に「前」だけを書いた日がありうる）。
  proto.html:128-133 は日の記録の有無だけで「未記入」を決めている。ST17 を使い始める前の日も、暦の上では「未記入」になる
- kind: daily
- 提案: B の問いを 1 つ足す。「滞在にだけ書いた日・前だけ書いた日・使い始める前の日を『未記入』に数えるか」。推奨を付ける。読み方の規則なので後から計算し直せ、loss は無い
- 処置: escalated — B の問い Q13（推奨: 紐づく記録が 1 件も無い日だけ。最初の記録より前は「使い始める前」）を足した
## R13. deep.md の手順 5 にある検査の行番号が違う
- 成果物: openspec/changes/st17-daily-feeling/deep.md（手順 5）
- 根拠: deep.md は `tools/check-immutable.sh:147` と書いているが、`grep -n "主張以外の本人が書いた記録" tools/check-immutable.sh` は 796 / 801 行。
  147 行の付近は稼働記録の錠の検査（159 行の注記）
- kind: premise
- 提案: `tools/check-immutable.sh:796` に直す。この Scenario は `logical_source = 'immutable-check'` の行だけを書き換えるので、C2 で主観に錠を掛けても落ちない。そのこともあわせて書く
- 処置: fixed deep.md — 行番号を :796 に直し、錠を掛けても検査が落ちないことを書いた
