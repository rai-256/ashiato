# 整合の確認（2026-10-12・ST17 の深掘りを要件へ戻した改訂）

- 基準: `90952ed325ed6ba063a79e46132e6e66e60ce5b0`..`380febd3118cf6ee986b907f12b347b5a2c9c6f0`

対象は ST17（`openspec/changes/st17-daily-feeling/`）の深掘りの決定を要件へ戻した改訂（FR-35 / FR-36 / FR-39 / FR-40 / FR-41 / FR-42 / FR-43 / FR-57。★ 2026-10-07）と、
同じ枝で足した `docs/stories/INDEX.md` の訂正・`docs/ui-direction.md` の ★・`docs/handoff/ST14.md` `ST18.md` `ST24.md` `ST26.md` `ST29.md`。
改訂は 2026-10-07 に書かれ、main の「衛星の基盤」の改訂（2026-10-11）と UI の表面の改訂（2026-10-12）の上へ rebase された。**main 側で新しくなった要件・Story・正典との食い違いを重点に見た。**
文書は head（`380febd3`）の内容で読んだ。作業ツリーの未追跡ファイル（`deep-answers-3.txt`）は見ていない。

機械の入力:
- `req_impact.py --base 90952ed3`: 改訂 8 件・扉 0 項。名指しは `docs/stories/INDEX.md` `stories.json` `docs/ui-direction.md` `docs/handoff/ST14.md`・ST17 の change の各文書・走っている st08 / st12 / st25 の文書・正典 `collection-coverage` `desktop-collection` `device-collection`（FR-35）
- `check_chain.py`: 未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった」（この監査が受けるもの）

## C1. 収集アプリの資格情報を「送るだけ」に締める ST44 の完了の判定が、ST17 の通知（FR-43）が要る読み出しを塞ぐ
- 重大度: 高
- 文書: docs/stories/stories.json:1045
- 要件: FR-43, PERM-10
- 根拠: ST44 の完了の判定「収集アプリの資格情報では、送ることだけができて記録は読めない（配り方は深掘りで決める。handoff/ST29.md の Q4）」。ST17 の改訂後の FR-43 は「その日に紐づく主観の記録が無い」ときだけ送り、spec は「通知を出す前に、その日の通知の判断を求め、その日に紐づく記録があれば通知を出さない」（`openspec/changes/st17-daily-feeling/specs/subjective-log/spec.md:640`）。携帯はこれを収集アプリの合言葉で `GET /feelings/reminder` に聞く（design D9 / D10。`FeelingReminderReceiver.kt`）。design D10 は「`recorded == null`（得られない）なら Notify」なので、ST44 が読み出しを塞ぐと 401 が「確かめられない」に落ち、**書いた日にも毎日鳴る** —— FR-43 ★ が避けた「鳴りすぎる通知は切られる」に黙って戻る。ST17 の申し送りは `docs/handoff/ST29.md:18`「**ST29 で範囲を絞るとき**: 携帯の収集アプリが要るのは `/feelings/reminder` だけ」と ST29 に宛てているが、main の改訂で宛先は ST44 に移った（`docs/stories/INDEX.md:103`「収集アプリの資格情報（PERM-10★ / `handoff/ST29.md` の Q4）は ST44」）。ST17 の spec も「ST29 までは資格情報の範囲が分かれていない」（spec:598）・design D9「ST29 で範囲を絞ったときに」と ST29 を指したまま。ST44 は `device-collection` / `desktop-collection` / `data-sensitivity` で、ST17（`subjective-log` / `browsing-views`）と capability が重ならないので admission は並べて走らせる
- 直すところ: stories（ST44 の完了の判定に「通知の判断の口（`/feelings/reminder`）だけは読める」を例外として書く）と、その文書（`docs/handoff/ST29.md` の R5 の宛先を ST44 に書き換える、ST17 の spec:598 と design D9 の「ST29」）
- 機械: 安く捕まえられる（ST17 の結合テストで、`GET /feelings/reminder` を収集アプリの資格情報で呼んで 200 を見る試験を置けば、ST44 が締めた時点で落ちる。いまの Scenario は「資格情報を持つ呼び出し元」で、どの資格情報かを固定していない）
- 行き先: handoff ST44（収集アプリの資格情報は ST44 に移った。`/feelings/reminder` だけ読める例外と試験を置く。ST29 の R5 にも移った旨を書いた。仕様は合意済みなので ST17 の spec / design は直さない）

## C2. ST17 の読み出しの口は「資格情報を持つ呼び出し元」に主観を返すと定め、衛星（外部 AI を含む）の資格情報を断る決まりが無い
- 重大度: 高
- 文書: openspec/changes/st17-daily-feeling/specs/subjective-log/spec.md:491
- 要件: PERM-4, PERM-15, FR-63, FR-88, PERM-14
- 根拠: spec:491「WHEN 資格情報を持つ呼び出し元が日付の範囲…を指定して主観を求める THE SYSTEM SHALL 範囲の各日について…その日に紐づく記録を返す」、spec:501「THE SYSTEM SHALL 感度で記録を絞らずに返す」、Scenario `感度で主観を絞らない`（spec:564「感度を『AI に出さない』に変えた主観の記録がある日を読み出す → その記録は返る」）。spec:593 の通知の判断も同じ「資格情報を持つ呼び出し元」。改訂後の要件では資格情報は衛星ごとに発行され（PERM-10 ★ 2026-10-11）、「外部 AI」の衛星は主観を承認されない（FR-88 の表「主観 | — | 承認」、PERM-4「最初は『ローカル AI』の衛星にだけ承認」）、印の付いた記録はどの承認でも返さない（PERM-15）。ST17 は 2026-10-07 に衛星の資格情報が無い前提で書かれ、spec の文のまま作ると、ST29 の後は外部 AI の資格情報で `/feelings` が主観の本文（ひとこと）を返す。塞ぐ側の Story —— ST29「衛星の資格情報では、まだどの口も記録を返さず受け取らない」（stories.json:699）、ST45「承認されていない種類・印の付いた記録は、どの口からも衛星に返らない」（stories.json:1073）—— は capability の表で `subjective-log` を持たない（`docs/stories/INDEX.md:85`「`subjective-log` | … | ST17, ST18, ST24」）。ST17 が先に archive されると、正典 `subjective-log` の「資格情報を持つ呼び出し元に返す」を ST29 / ST45 が MODIFIED できない。ST29 が先なら、ST17 の spec が正典 `data-sensitivity` と食い違ったまま入る
- 直すところ: その文書（ST17 の spec の 2 つの口の呼び出し元を「画面のログインと本体の収集アプリの資格情報」に限り、衛星の資格情報は断ると書く。ST17 の change は改訂した側なので差し戻しではない）か、stories（INDEX の `subjective-log` に ST29 / ST45 を足す）
- 機械: 安く捕まえられる（ST29 で「衛星の資格情報で router の全 route を呼び、記録を返す口が 0」を route の一覧から回す試験にすれば、後から足された `/feelings` も拾える。handler ごとに書くと漏れる）
- 行き先: handoff ST45（主観の 2 つの口を読み出しの門に入れる。ST29 にも全 route の試験を書いた。ST29 の前に衛星の資格情報は無く、失われるものは無い）

## C3. main が ST17 に宛てた申し送り（`docs/handoff/ST17.md`）を、ST17 の深掘りは「無い」と書いたまま受けていない
- 重大度: 中
- 文書: openspec/changes/st17-daily-feeling/deep.md:10
- 要件: PERM-2, PERM-4, FR-88
- 根拠: deep.md:10「`docs/handoff/ST17.md` は無い。`deferred ST17` の処置も無い」。rebase 後の木には `docs/handoff/ST17.md`（2026-10-11）があり、「主観の『ローカル AI まで』は、記録の値ではなく承認になる」「ST17 が archive されると、`subjective-log` の正典に 4 段階の値が Scenario として残る。ST24 はそれを MODIFIED する」と書いている。ST17 の spec の Requirement「主観の既定の感度」（spec:389-404）は導出元を「PERM-4, PERM-2」としているが、改訂後の PERM-4 は「種類の読み取りを…承認した状態で始める」、PERM-2 は「『どの衛星にも出さない』の印を付けられる」で、どちらも「感度『ローカル AI まで』で格納する」を言っていない。申し送りの中身は「変えなくてよい（ST24 が読み替える）」なので失われるものは無いが、宛先が受けた記録が無く、spec の導出元が今の要件と合わない
- 直すところ: その文書（deep.md の「この Story に宛てられたもの」に `handoff/ST17.md` を足し、spec の「主観の既定の感度」の導出元を「PERM-4 の 2026-10-11 より前の文。読み替えは ST24（handoff/ST17.md）」と書く）
- 機械: 安く捕まえられる（`story_brief.py` の「宛てられたもの」は `docs/handoff/ST<NN>.md` の有無で決まるので、deep.md の「無い」と突き合わせられる。rebase の後に引き直していない）
- 行き先: docs（deep.md の「宛てられたもの」に追記。spec の導出元は合意済みなので直さず、ST24 の MODIFIED で読み替える）

## C4. ST17 が足した INDEX の行・ST24 への申し送り・design D3 が、main で消えた ST24 の前提（感度の既定を登録簿へ移す）を指している
- 重大度: 中
- 文書: docs/stories/INDEX.md:349
- 要件: PERM-2, PERM-4, PERM-9
- 根拠: INDEX:349「PERM-4（主観の既定の感度） | ST24 | 主観の記録をローカル AI までで格納する（C6）。**感度の操作と、既定を登録簿へ移すのは ST24**」。同じ INDEX の main 側の訂正（:105）は「2026-09 の訂正の表で PERM-3 / PERM-4 の担い手を ST24 としている行は、2026-10-11 から ST27（AI の衛星の最初の承認）と ST38（承認）に移った」。ST24 は今「記録ごとに『どの衛星にも出さない』の印を付けられる」（satisfies PERM-2 / PERM-9、requires [ST22]）で、登録簿に既定の感度を持つ話は無い。`docs/handoff/ST24.md:5-6`「登録簿に『ソースごとの既定の感度』を持たせるなら、この分岐を登録簿の値に移す」、design.md:84（D3 の反転条件）「ST24 が登録簿に『ソースごとの既定の感度』を持たせたとき」、proposal.md:90「ST18 / ST24 / ST26 | 待ち（requires に ST17）」も同じ前提（ST24 の requires に ST17 はもう無い）。**残る中身で効くのは錠の 1 点**: design D2「削除の印と感度は通す」・handoff/ST24.md:7「錠は感度の列を通す」。PERM-2 の例「ある日の日記」は主観の記録そのものなので、ST24 が印を感度の列以外に持つと、ST17 の錠が主観への印付けを拒む
- 直すところ: その文書（INDEX:349 の本体の Story を ST27 / ST38 に直し「主観の記録に 4 段階の 2 を書く。読み替えは ST24」とする。handoff/ST24.md を「印を感度の列以外に持つなら、主観の錠（`core.reject_feeling_rewrite()`）が通す列に足す」に書き直す。design D3 の反転条件と proposal:90）
- 機械: 安く捕まえられる（ST24 の試験に「主観の記録に印を付けられる」を 1 本置けば、錠が拒んだ時点で落ちる）
- 行き先: handoff ST24（錠が「出さない」の印を通す 1 点を書き直した。INDEX の PERM-4 の行も ST27 / ST38 に直した。design D3 と proposal:90 は合意済みなので直さない）

## C5. ST17 が足す口は利用者識別子を引数から取る。PERM-1 ★ はそれを禁じ、ST37 の壊してはいけないものに ST17 が無い
- 重大度: 中
- 文書: openspec/changes/st17-daily-feeling/design.md:137
- 要件: PERM-1, FR-92, PERM-10
- 根拠: design D8「`GET /feelings?from=YYYY-MM-DD&to=YYYY-MM-DD&user_id=`（`user_id` は省けば nil UUID。ほかの読み出しと同じ）」、D9「`POST /feelings/reminder`（`{user_id?, at:"HH:MM"}`）」「`GET /feelings/reminder?date=YYYY-MM-DD&user_id=`」、tasks.md:102 も同じ。改訂後の PERM-1 ★（requirements.md:795）「利用者識別子は送られた内容から取らず、資格情報から決める（PERM-10）」。ST37 の壊してはいけないもの（stories.json:868）は「走っている ST21 / ST25 の口（利用者識別子を引数から取っている）」で ST17 を挙げていない。完了の判定（:873）「既存の口（場所・1 日の画面など）は利用者識別子を引数から取らない」の「既存」は ST37 の着手時点で決まるので、ST17 が後から入れば外れる。ST37 は `browsing-views` を共有するので admission は順に並べるが、順序は ST17 の archive が先とは限らない
- 直すところ: その文書（ST17 の design / tasks で `user_id` を引数から取らない。または ST37 の壊してはいけないものに ST17 を足す）
- 機械: 安く捕まえられる（`docs/openapi.json` の params に `user_id` を持つ口を数える grep。ST37 の完了の判定を「0 本」にすれば後から足した口も捕まる）
- 行き先: handoff ST37（ST17 の 3 つの口を壊してはいけないものに入れる。openapi.json の user_id を数える）

## C6. ST17 が登録する論理ソース `s01-feeling` は種類（FR-88）に属していない。FR-102 を作る ST38 と並んで走れる
- 重大度: 中
- 文書: openspec/changes/st17-daily-feeling/design.md:66
- 要件: FR-102, FR-88, PERM-4
- 根拠: design D1「登録簿の行: `('s01-feeling', '主観', 86400, 'none')`」。FR-102「すべての論理ソースを、登録簿でどれか 1 つの種類（FR-88）に属させる」★「属さない論理ソースは、どの承認でも読めず、どこにも出ない」。ST38 の完了の判定（stories.json:901）「すべての論理ソースがどれか 1 つの種類に属し」。ST38（`data-sensitivity` / `satellite-platform`）と ST17 は capability が重ならず、admission は並べる。ST38 が先に入ると、ST17 の移行は種類の無いソースを足す（PERM-4 が承認するローカル AI からも主観が読めない）。ST17 が先なら、ST38 が `s01-feeling` → 「主観」を写す必要があるが、ST38 の文書はそれを知らない
- 直すところ: その文書（ST17 の design に「種類は ST38 の『主観』。ST38 が先なら移行で種類を入れる」を足す）か、ST38 への申し送り
- 機械: 安く捕まえられる（ST38 が種類の列を `NOT NULL` で持てば ST17 の移行の INSERT が落ちる。`registry_tests.rs` に「全論理ソースが種類を持つ」を 1 本）
- 行き先: handoff ST38（`s01-feeling` → 種類「主観」。どちらが先でも写る形を ST17 の handoff にも書いた）

## C7. FR-35 の除外は「本人が書くソース」を主観と個人属性の 2 つで名指ししており、場所（ST21）と衛星の「本人が書いた」記録が残る
- 重大度: 中
- 文書: docs/requirements.md:389
- 要件: FR-35, FR-25, FR-48
- 根拠: FR-35「**本人が書くソース（主観 FR-36・個人属性 FR-44）は対象にしない。**」。走っている ST21 の design（`openspec/changes/st21-place-registry/design.md:86`）は場所を `('s01-place', '場所', 86400, 'none')`・`origin='authored'` で登録する。FR-25 ★ 2026-10-11「読書ログに本人が付けた感想は『本人が書いた』になる」（衛星の本人が書いた記録）。★ の理由「書かない日は壊れていない」は場所にも衛星の感想にも当てはまるが、本文の括弧が 2 つだけを挙げるので、ST14 が逐語で作ると場所を 3 日登録しなければ途絶として鳴る。`docs/handoff/ST14.md` の ST17 の節も「`s01-feeling` と `s01-attribute` を外す」と 2 つだけ
- 直すところ: requirements（「本人が書いた（FR-25）記録のソースは対象にしない」と分類で書き、括弧は例にする）
- 機械: 安く捕まえられる（ST14 の判定を `origin` で絞れば自動で外れる。登録簿の全ソースについて「authored のソースは途絶の判定に入らない」を試験で回せる）
- 行き先: handoff ST14（除外を本人が書いたソースの分類で決めるかを ST14 の上流で。計算し直せる B 段）

## C8. FR-41 は主観の「日」の記録の出来事の時刻に Asia/Tokyo を凍結するが、main は日の境界のタイムゾーンを未決（計算し直せる B 段）として ST41 に送っている
- 重大度: 中
- 文書: docs/stories/INDEX.md:104
- 要件: FR-41, FR-19, FR-91
- 根拠: INDEX:104「C34（日の境界のタイムゾーン）は ST41 の上流で決める（計算し直せる B 段）」。`docs/audit/2026-10-11-consistency.md` の C34「衛星が海外で書くと日の境界がずれる…導出なので戻せる（B 段）」。改訂後の FR-41「FR-19 の『出来事が起きた時刻』には対象の時刻を入れる（日なら Asia/Tokyo のその日の 0:00…）」★「この列は凍結され重複判定の鍵にも入るので、後から入れ替えると全行の書き直しになる」。ST17 の design D1 は記録の地域も `Asia/Tokyo`（+540）に固定し、違えば断る（design.md:48・:76）。主観の記録については、日の境界はもう導出ではなく凍結された値になっていて、ST41 の上流が「B 段」と読んで決めると、主観だけが別の境界で残る
- 直すところ: その文書（INDEX の C34 の行に「主観の日の記録は FR-41 で Asia/Tokyo に凍結済み（rewrite-all）」を足す）
- 機械: できない
- 行き先: docs（INDEX の C34 の行に、主観の日の記録は FR-41 で凍結済みと足した）

## C9. ST18 の完了の判定「1 行登録すると書けるようになる」は、ST17 の design（行とコードの検査を同じ change で足す）と食い違う
- 重大度: 中
- 文書: docs/stories/stories.json:438
- 要件: FR-38
- 根拠: ST18 の完了の判定「新しい紐づけ先の種類を 1 行登録すると、その種類に主観を書けるようになる」。ST17 の design D5（design.md:103）「紐づけ先の種類は登録簿 `core.feeling_target_kind`（D15）に行があり、**かつコードがその種類の検査を持つときだけ受ける**。ST18 が種類を足すときは、表の行とコードの検査を同じ change で足す」、`docs/handoff/ST18.md`「表の行と `feelings.rs` の検査（鍵の形・出来事の時刻の決め方・その種類の『終わり』）を同じ change で足す」。完了の判定は 1 行の登録で書けることを固定していて、ST17 の器ではそれでは書けない（申し送りは届くが、判定の文は残る）
- 直すところ: stories（ST18 の完了の判定を「種類を 1 つ足すと…書けるようになり、既存の主観記録は 1 件も変わらない」に直す）
- 機械: できない
- 行き先: handoff ST18（完了の判定の読み替えを ST18 の上流で）

## C10. UI の方向の件数の表が、主観を 1 日 1 件と数えている
- 重大度: 低
- 文書: docs/ui-direction.md:83
- 要件: FR-36
- 根拠: 「生の記録 | 1 日 約 4,120 件 | … + 主観 1（FR-36）」。改訂後の FR-36 と ST17 の C2 / Q11 では、押し直し・ひとことを欄から離すたび・滞在ごとに 1 件増える（design D12「ひとこと: …欄を離れたとき…に 1 件」）
- 直すところ: その文書（「主観 数件」）か leave（桁は変わらない）
- 機械: できない
- 行き先: leave: 桁（約 4,120 件）は変わらず、件数の表は目安

## C11. ST17 の spec の理由の文に「プラグイン」が残っている
- 重大度: 低
- 文書: openspec/changes/st17-daily-feeling/specs/subjective-log/spec.md:303
- 要件: FR-77
- 根拠: 「凍結をアプリ層に置くと、同じ PC で動く psql やプラグインが素通りする」。FR-77 ★ 2026-10-11「『プラグインおよび衛星アプリ』を『衛星』にまとめた」。INDEX:100 は正典に残る「第三者製プラグイン」の言い回しを ST24 で直すとしており、ST17 が archive すると直す対象が 1 つ増える
- 直すところ: その文書（「衛星」）
- 機械: 安く捕まえられる（`openspec/` の archive 以外と `openspec/specs/` を `プラグイン` で grep）
- 行き先: handoff ST17（合意済みの spec を変えず、下流が読む申し送りに書いた。正典の言い回しは ST24 が直す）

## C12. ST17 の proto は 9/8 の表面のままで、UI の方向の正典（★ 2026-10-12）と色・角・操作の間が違う
- 重大度: 低
- 文書: openspec/changes/st17-daily-feeling/proto.html:132
- 要件: NFR-21, NFR-23
- 根拠: proto は `hsl(132 30% …)` だけを使い（12 か所）、角は 6 px、快–不快の 5 つの間は `gap: 6px`（:132）。`docs/ui-direction.md` ★ 2026-10-12「表面（色・形・密度・文字）はこの節が正典」—— 色相 140 / 307 / 150、角 12 / 7 px、「横に並ぶ操作の間は 8px を下回らない」。ST17 の design D14 は色を `tokens.ts` から取ると書き、spec / e2e は proto から寸法（64 px・44 px・40 px）だけを取るので、作られる画面は食い違わない。proto を見本にして色や間を写すと 9/8 に戻る
- 直すところ: leave（proto の冒頭に「表面は 9/8 のまま。色・角・間は ui-direction ★ 2026-10-12 と tokens.ts」と 1 行添えるなら、その文書）
- 機械: できない
- 行き先: handoff ST17（下流に色・角・間を ui-direction ★ 2026-10-12 と tokens.ts から取るよう書いた）

## K1. ST17 の design が指すコードの行が rebase でずれている
- 重大度: 低
- 文書: openspec/changes/st17-daily-feeling/design.md:12
- 要件: —
- 根拠: design.md:12「`authorize()`（`lib.rs:468-498`）」→ 今は `crates/server/src/lib.rs:483`（`async fn authorize(` の始まり）。design.md:82「`default_sensitivity()`（`lib.rs:576`）」→ 今は `lib.rs:591`。design.md:15「`coverage::must_sources()`。`lib.rs:1999`」→ 呼び出しは今 `lib.rs:2014`。どれも関数名が添えてあるので探せる。`lib.rs:726-760`（主張の分岐。今 735-760）・`check-immutable.sh:797-805`・`stay_store.rs:258-266`（`Mark::of`）・`LocationService.kt:881-990` は合っている
- 直すところ: その文書（行番号を外して関数名だけにする）
- 機械: 安く捕まえられる（`<file>:<line>` の参照先の行に、添えた識別子があるかを grep する）
- 行き先: handoff ST17（行番号のずれを下流の申し送りに書いた。design は合意済みなので直さない）

## K2. design D14 の「破線は `tone` の既存の輪郭の色」に当たる値が `tokens.ts` に無い。近い `line` は 3:1 に届かない
- 重大度: 低
- 文書: openspec/changes/st17-daily-feeling/design.md:233
- 要件: NFR-23
- 根拠: 「破線は `tone` の既存の輪郭の色（3:1 は e2e で測る）」。`web/src/tokens.ts:106` の `tone()` は HSL を文字列にする関数で、輪郭の色の値ではない。色の組（`SCHEMES`）で縁に使えるのは `line` と `muted` で、`tokens.ts:42`「`line` は区切りの線（装飾。意味は担わせない —— 意味を担う縁は `muted` で引く）」。`line` を破線に使うと、面（ground / surface1 / surface2）に対する比はライト 1.44 / 1.32 / 1.22、ダーク 2.13 / 1.69 / 1.30（WCAG の式で計算）で、spec の「未記入の破線は 3:1 以上」を割る。`muted` なら 5.09〜8.33
- 直すところ: その文書（「破線は `muted`」）
- 機械: 捕まえられた（spec の Scenario `押された状態と未記入の破線は 3:1 以上` が e2e で測る。下流で 1 往復するだけ）
- 行き先: handoff ST17（破線は `muted` で引く、と下流の申し送りに書いた。e2e が 3:1 を測る）


## 未確認

- ST17 の issue が作られて tasks.md が凍結済みかどうか（`docs/handoff/ST17.md` は「凍結している」と書くが、確かめていない）。凍結済みなら C1・C2・C4〜C6 の「その文書」は ST17 自身の上流の直しとして扱えるかを、改訂した側が決める
- ST44 の上流が `docs/handoff/ST29.md` の ST17 の節（R5）まで読むかどうか（ST44 の完了の判定は「Q4」だけを名指ししている）
- 読み出しの記録（`access_log::middleware`、`lib.rs:2260`）が router 全体に掛かっていて、ST17 が足す `/feelings` `/feelings/reminder` にも自動で行が残るか。route ごとには確かめていない
- 画面が `/api/ingest` に送る主観の記録の `user_id` をどこから入れるか（ST17 の spec / tasks から読み切れなかった。C5 は読み出しの口と通知の口についてだけ書いた）
- 正典 `openspec/specs/*/spec.md` の Scenario 646 件はすべて `crates` `web/src` `web/e2e` `collector-android/app/src` `tools` に印がある（空白を無視して照合。0 件欠け）。印の先の試験が Scenario を観測しているかは見ていない

## 触らなくてよいと判断したもの

- **FR-91（終わりの時刻の列）と ST17 の「滞在」の記録の `target.start` / `target.end`** —— 主観の記録は幅のある出来事ではなく、滞在そのものの終わり（`payload.end`）は ST16 のもの。ST41 が列を足しても主観の器は変わらない
- **`docs/audit/2026-10-09-satellite-foundation.md:78` の「衛星へ回す候補 ST17・18（主観の入力 UI）」** —— 候補の列挙で、その後の要件（`docs/requirements.md:53` の V-01「…主観入力…」）と stories.json は本体に置いたまま。食い違いではない
- **UI の方向の「主ボタン（書く・積む・ログイン）はアクセントで塗る」と S-2 の頭の「書く」** —— ST17 の spec は「書く」の色を固定していない。下流が tokens.ts で塗れば合う
- **「横に並ぶ操作の間は 8px」と幅 360 px の 5 つのボタン** —— `tokens.ts` の値（画面の余白 17.5 px、節の内側 9.8 px）で残る幅は約 305 px、間 8 px × 4 を引いて 1 つ約 55 px。NFR-19（24 px）も spec の「横 1 列・横のスクロール無し」も満たせる
- **ST14 の完了の判定** —— FR-35 の除外が増えても、3 つの判定（18 時間・Takeout の 2 か月・退役）のどれとも食い違わない。除外は `docs/handoff/ST14.md` で届く（範囲の不足は C7）
- **`docs/ui-direction.md:19` と :277-283** —— S-3 の行に FR-57 を足した ★ と「決めなかったこと」の決着は、ST17 の deep.md Q1 と spec（`browsing-views` の頭の欄・S-3 の暦）に合っている
- **走っている change への印**（差し戻しではない。どれも失われるもの無し）:
  - `st21-place-registry` —— 場所のソースが FR-35 の除外の括弧に入っていない（C7）。FR-35 の判定はまだどこにも無いので、ST21 の作るものは変わらない
  - `st25-day-timeline` —— ST17 が 1 日の画面の頭に 40 px 以下の欄と滞在の行の 1 行を足す。ST17 は admission で ST25 の archive を待つ（`browsing-views`）。ST25 の `review/deep.md:5` は FR-57 を読んだ範囲に挙げているだけ
  - `st08-browser-history` / `st12-archive-ingestion` —— FR-35 を名指しするのは収集したソースの想定間隔と生存信号の箇所で、本人が書くソースの除外の影響を受けない
- **正典 `collection-coverage` / `desktop-collection` / `device-collection` の FR-35 の名指し** —— どれも収集したソースの文脈で、改訂の除外と食い違わない
