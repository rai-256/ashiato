# ST03 成果物レビュー（spec 段階・独立）

対象: `openspec/changes/st03-idempotent-ingest/` 一式、`docs/stories/ST03.md`、`docs/stories/stories.json`、
`docs/requirements.md`（★ 2026-09-11 の改訂）、`openspec/specs/`（正典）、`openspec/changes/st02-collection-coverage/`。

**書いた者の意図は聞いていない。成果物どうしの整合だけを見た。成果物は 1 文字も触っていない。**
採番は **R65 から**（R1〜R64 は deep のレビューで使用済み）。

## 機械の検査（2026-09-11 実行。人間の目より先に）

```
$ openspec validate st03-idempotent-ingest --strict
Change 'st03-idempotent-ingest' is valid            rc=0

$ python3 scripts/check_chain.py .
  要件 113 件 / Story 36 本 / 扉 26 項
  [ok] どれかの Story に拾われた要件: 111/113 件
  [ok] INDEX の「Story の対象外」: NFR-11, NFR-8
chain: OK (0 件 / 未回収 0 件 / warn 0 件)            rc=0

$ python3 scripts/check_scenarios.py . st03-idempotent-ingest
  Scenario 59 件 / 印 33 個 / 担保あり 30 / 人間の確認待ち 0
  [FAIL] 担保の無い Scenario: 29 件（ST03 が足した 29 件すべて）
scenarios: FAIL (担保なし 29 件)                     rc=1

$ python3 scripts/review_triage.py . st03-idempotent-ingest
  指摘 64 件 / 要件へ戻すもの 14 件
triage: OK                                           rc=0
```

`check_scenarios.py` の rc=1 は上流の段階では当然だが、**tasks がその状態を緑にする道を 1 行も持っていない**
（→ R82）。`scripts/merge_gate.sh:61` がこの検査を見るので、下流の PR はここで止まる。

---

## 観点 1: deep の 26 の決定が正典に写っているか

## R65. Q17 / Q12 / Q23 が決めた「履歴に残した前の版の**本文の消去は通る**」が、spec で全面禁止に書き換わっている
- 成果物: openspec/changes/st03-idempotent-ingest/specs/record-envelope/spec.md:82-84, 112-115
- 根拠: 本人の答え（deep.md:344「履歴表は追記のみ。**削除の印・感度・本文の消去だけを通し**、
  それ以外の書き換えと行の削除を拒む」）。Q21 で削除の印と感度の列が消えたので、
  残る開口部は**本文の消去 1 つだけ**。deep.md:290 の R50〜R53 の決着も
  「(c) 台帳は本表と履歴の**両方**の門に効かせる（片方だけだと「消せない DB」が残る）」と書いている。
  実測も同じ —— `review/deep-r5.md:50`「実験 AD2 履歴に残った前の版の本文を消す → **通る**（R40 の正規の経路）」、
  `:57`「実験 AG2 FR-51 の正規の消去（本表＋履歴＋台帳を 1 txn）→ **通る**（成立）」。
  ところが spec は `THE SYSTEM SHALL 履歴と台帳を**追記のみ**とし、行の書き換え・行の削除・表の切り詰めを拒む`
  と書き、Scenario「履歴と台帳は書き換えも削除もできない」が
  `WHEN 履歴または台帳の行を書き換える … THEN いずれの操作も拒まれる` と無条件で閉じている。
  この spec のとおりに実装すると `docs/requirements.md:317`（FR-51「**消去は履歴に残した前の版にも及ぶ**」）が
  **満たせない**。実験 AG3 が「開ける必要のある操作が 1 つも無い」と確かめたのは**台帳**だけで、履歴ではない
- kind: technical
- 処置: fixed specs/record-envelope/spec.md — **いちばん重い指摘。本人の答えと逆を書いていた。** 履歴と台帳を 1 文に束ねていたのを分け、履歴は「台帳の行が同じまとまりにある本文の消去だけを通す」に直した。Scenario も「履歴の本文は台帳があれば消せる / 無ければ消せない / 消去以外は拒む / 台帳は何をしても変えられない」の 4 本に割った
- 提案: 履歴と台帳を 1 文に束ねず分ける。履歴は「**台帳の行が同じトランザクションにある本文の消去だけを通し**、
  それ以外の書き換え・行の削除・切り詰めを拒む」。台帳は無条件で追記のみ。
  Scenario も「履歴の本文は台帳があれば消せる」と「台帳は何をしても拒まれる」に割る

## R66. R52 の決着（台帳の門を**本表と履歴の両方**に効かせる／消去は親とその記録のすべての履歴を同じ txn で消す）が design に無い
- 成果物: openspec/changes/st03-idempotent-ingest/design.md:66-82, 86-90
- 根拠: `review/deep-r5.md:155-170` の R52、処置は `fixed deep.md — 申し送りへ。台帳は本表と履歴の両方の門に効かせる。
  片方だけだと R41（消せない DB）が残る` + 提案「消去は親と**その記録のすべての履歴**を同じ txn で消す
  （分けると 2 段目が R40 の開口部そのものになる）」。
  design D4 のトリガは `AFTER UPDATE ON core.event` **1 本だけ**で、履歴側の門が無い。
  D5 が履歴・台帳に置くのは `BEFORE DELETE` / `BEFORE TRUNCATE` だけで、
  **履歴の `UPDATE` を止める仕掛けがどこにも書かれていない**（Q17 が要求した「本表の 0002 / 0004 と同じ作りのトリガ」
  —— deep.md:349）。それなのに tasks 7.4 は「履歴と台帳が追記のみ —— `UPDATE` / `DELETE` / `TRUNCATE` の
  3 経路とも拒まれる」を要求している。design と tasks が食い違っている
- kind: technical
- 処置: fixed design.md — D4 に門の最終形を書き、`core.event_version` にも制約トリガを置くこと、消去が親と全履歴を 1 トランザクションで消すことを足した。tasks 7.4c を新設
- 提案: D4 に門の最終形を 1 文で書く（「同じ txn に履歴行がある書き換え、または同じ txn に台帳行がある消去だけを通す」）。
  `core.event_version` にも同じ制約トリガを置くことと、消去が親と全履歴を 1 txn で消すことを書く

## R67. Q20 の「更新時刻を返さないソースは**届いた順**に倒す」と、R45 で決めた `>=` が spec に無い（design D8 にしかない）
- 成果物: openspec/changes/st03-idempotent-ingest/design.md:114-128 / specs/record-envelope/spec.md:46-74
- 根拠: 本人の答え（deep.md:423-426）「外部サービス側の更新時刻（または版）を列で持ち、**新しいほうだけを採る**。
  …外部サービスが更新時刻を返さない場合は**届いた順を使う**」。
  design D8 は「時刻型で作り、不透明な版しか返さないソースが出たら「届いた順」に倒す（**記録単位**で判定する）」
  「**同じ更新時刻で内容だけ違う到着は「新しい」として扱う**（`>=`）。`>` にすると `accepted` を返しながら
  内容が変わらず、**応答から見えない**」と書いているが、spec のこの Requirement は
  「古いとき書き換えない」「更新時刻を持たない記録が届いたとき保存済みの更新時刻を消さない」の 2 文だけで、
  **更新時刻を持たない到着が適用されるのか無視されるのかが書かれていない**。
  `openspec archive` は main specs しか更新しないので、design に置いた `>=` と「届いた順」は正典から落ちる。
  tasks 6.3（`same_updated_at_still_applies`）だけが残り、spec 側の根拠が消える
- kind: technical
- 処置: fixed specs/record-envelope/spec.md — Requirement に 3 文（更新時刻の無い到着は届いた順 / 等しければ新しいものとして適用 / 判定は記録 1 件ごと）と Scenario 2 本を足した。design にしか無いと archive で正典から落ちる
- 提案: Requirement に 2 文足す ——「更新時刻を持たない記録は届いた順で適用する」
  「更新時刻が等しく内容が異なる到着は新しいものとして扱う」。Scenario も 2 本足す

## R68. Q22 の答え（分けたあと過去分は古いソース名のまま／原則は「取り込む前に分ける」）の「効く先: design に明記する」が果たされていない
- 成果物: openspec/changes/st03-idempotent-ingest/design.md
- 根拠: deep.md:440-447「**本人の答え**: 古いソース名のまま残す（過去分には触らない）…
  **原則は「取り込む前に分ける」**で、これはそれを守れなかったときの逃げ道。`design` に明記する」。
  `grep -n "古いソース名\|取り込む前に分ける\|過去分" design.md` → 0 件。
  この決定は不可逆（deep.md:441「ソース名は冪等キーの入力そのもので、分けたあとに同じ書庫を入れ直すと
  過去分が全件二重に入る」）で、書かれていないと下流が「分けたら過去分も移す」を選びうる
- kind: technical
- 処置: fixed design.md — D12 を新設。「原則は取り込む前に分ける。後から分けたら過去分は古い名前のまま残す（移すと鍵が変わり全件二重になる）」
- 提案: design に D 番号を 1 つ足して「分割は取り込む前に行う。やむを得ず後で分けたときは過去分を古い名前のまま残す
  （移すと冪等キーが変わり全件二重になる）」を書く

## R69. R11（重複のとき**格納されている行の識別子**を返す）が spec にも design にも無く、tasks 10.2 にしかない
- 成果物: openspec/changes/st03-idempotent-ingest/tasks.md:79 / design.md
- 根拠: deep.md:249-252「**R11 — 重複のとき、DB に無い識別子を返している**（`lib.rs:230-233`）。…
  **返す値の形は Q1 の答え（更新するか）で決まるので、design に落とす**」。
  design.md に `識別子を返` の記述は 0 件。spec にも応答の `id` に関する Requirement / Scenario が無い。
  `docs/collector-contract.md:62` は `id` を「格納された記録の識別子。断られたときは null のことがある」と
  書いており、**現状と仕様書が既に食い違っている**。これは観測可能な応答の形（正典に属する）で、
  tasks にしか無いと archive の対象にならない
- kind: technical
- 処置: fixed specs/record-envelope/spec.md — 正典の「取り込み口は複数件を…」を MODIFIED で持ち込み、「結果に載せる識別子は格納されている記録の識別子」の 1 文と Scenario 1 本を足した
- 提案: `record-envelope` の「取り込み口は複数件をまとめて受け取り、1 件ごとの結果を返す」に
  MODIFIED で 1 文と Scenario 1 本（「重複のとき返る識別子で、その記録を読み出せる」）を足す

## R70. Q14 の「外部から取り込むときも収集側の識別子を毎回新しく振る」に、spec の置き場も tasks も申し送りも無い
- 成果物: docs/requirements.md:136-137 / openspec/changes/st03-idempotent-ingest/
- 根拠: 本人の答え（deep.md:127）「毎回新しく振る。同一物の判定は外部識別子だけに任せる」。
  FR-21 には★付きで戻っている（`docs/requirements.md:136`）が、
  `openspec/specs/` にも ST03 の delta にも「収集側の識別子を毎回新しく振る」を言う Requirement / Scenario が無い。
  ST03 の spec が言うのは「**判定に用いない**」（受け手側）だけで、**振り方**（送り手側）は誰も持っていない。
  この決定が守られないと Q1 と Q5 が同じ到着に逆を指す（deep.md:122-125 がそう書いている）のに、
  tasks にも ST12 / ST13 への申し送りにも無い
- kind: technical
- 処置: fixed tasks.md — 13.3 を新設。ST12 / ST13 への申し送りとして「収集側の識別子は外部の識別子から導かず毎回新しく振る」を `handoff.md` に残す
- 提案: ST03 で置き場が無いなら tasks 13（申し送り）に「外部取り込みの Story（ST12 / ST13）は
  収集側の識別子を外部識別子から導かない」を 1 件足す。導けば FR-22 の 400 が正常系で出る

---

## 観点 2: Scenario が検証可能か

## R71. per-item の「400 で断られる」が、正典の「1 件も受け付けなかったときだけ 400」と真正面から衝突する
- 成果物: openspec/changes/st03-idempotent-ingest/specs/record-envelope/spec.md:40-44, 171-179, 187-190
- 根拠: 正典 `openspec/specs/record-envelope/spec.md:150`
  `THE SYSTEM SHALL 1 件でも格納したとき 200 を返し、**1 件も受け付けなかったときだけ** 400 を返す。`
  同 `:171-177` の Scenario「一部が不正でも正しい分は格納される … **AND 状態符号は 200 である**」。
  `docs/collector-contract.md:86-88` も同じ。
  ところが ST03 の Scenario は
  「収集側の識別子が同じで内容が違えば断られる: **THEN その 1 件は 400 で断られ** … **AND 同じ要求に含まれる
  他の正しい記録は格納される**」と書く —— 後半が成り立つなら状態符号は 200 であり、前半は false になる。
  同じ書き方が「記録ごとと宣言したソースで識別子を欠けば断られる」「宣言の無いソースは断る側に倒れる」
  「空の外部識別子は断られる」の 3 本にもある。`docs/requirements.md:152`（FR-22）/ `:162`（FR-23）/
  `:100`（FR-10 の「恒久的に断られた記録（**400**）」）も同じ誤りを持ち込んでいる。
  **このまま書くと、テストが「状態符号 400」を見に行って落ちるか、落ちないように 400 を返す実装になり正典を壊す**
- kind: technical
- 処置: fixed specs/record-envelope/spec.md — 4 本の Scenario の THEN を「その 1 件の結果は受理されなかったことを示し、理由の種別が示される」に直し、Requirement 本文の「400 で断る」も「受け付けない」に直した。正典の「1 件も受け付けなかったときだけ 400」と衝突していた
- 提案: Scenario の THEN を「その 1 件の結果が受理されなかったことを示し、理由の種別が `<種別>` である」に直す。
  状態符号に触れるのは「送ったすべてが不正」の場合だけにする。FR-10 の「（400）」も同様

## R72. device-collection の Requirement が、同じ段落で逆を言っている（保持し続ける／取り除く）
- 成果物: openspec/changes/st03-idempotent-ingest/specs/device-collection/spec.md:9, 12-13, 18
- 根拠: `:9` `THE SYSTEM SHALL 送信が失敗した記録を未送信のまま保持し、次の契機で再び送る。` は無条件のまま、
  `:12` `THE SYSTEM SHALL **恒久的に断られた記録を未送信から取り除き、捨てたことを残す**` が足されている。
  「断られた」と「失敗した」を分ける定義が SHALL の側に無く、判別の根拠は `:13` の散文
  （「要求そのものが不正であると示された」）だけ。
  加えて `:18` の導出元の段落は「**捨てるときの記録の残し方は ST04（圏外でも記録が失われない）が扱う**」と
  書いたままで、ST03 が「捨てたことを残す」を足したことと矛盾する。
  `docs/collector-contract.md:64` は `accepted` を「**未送信から取り除いてよい。収集側はこれだけを見る**」と
  定義しており、`accepted: false` でも取り除く新しい規則によってこの 1 行が false になるが、
  tasks 2.3 は `error` の表と要求の 2 項目しか直さない
- kind: technical
- 処置: fixed specs/device-collection/spec.md — 「送信が失敗した記録を保持し」を「**一時的な失敗で**送信が終わった記録を保持し」に限定した
- 提案: `:9` を「**一時的な失敗で**送信が失敗した記録を未送信のまま保持し」に限定する。
  「恒久的に断られた」を「1 件ごとの結果が受理されず、理由の種別が返ったもの」と SHALL で定義する。
  `:18` の ST04 への委譲を、捨てる契機ごとに書き分ける。tasks 2.3 に `accepted` 欄の定義の更新を足す

## R73. 「捨てた件数と理由の種別が残る」の**残り先**が、spec にも tasks にも無い
- 成果物: openspec/changes/st03-idempotent-ingest/specs/device-collection/spec.md:57-61 / tasks.md:84
- 根拠: Scenario の `THEN 捨てた件数と理由の種別が残る` は、どこに残るか（端末のログ / 稼働記録 / サーバ）を
  言っていないので、何を測れば真偽が決まるか書かれていない。
  近い要件は 2 つあり逆を指す —— `docs/requirements.md:98`（FR-9「破棄した期間と件数を**稼働記録**（FR-33）に残す」）と
  `:100`（FR-10「捨てた事実を残す」＝場所の指定なし）。tasks 11.2 の検証は「テストが**ログ行**を確かめる」。
  deep.md:174-178（Q16 の効く先）は「断りは端末では `Sender.kt:87-90` の**ログ 1 行にしかならず、
  画面に出る経路が無い**」と実測しているので、ログに残すだけでは本人が気付けない
- kind: technical
- 処置: fixed specs/device-collection/spec.md — 残り先を「端末のログ」と SHALL で決め、Scenario を独立させた。サーバへ送る経路は稼働記録しか無く、そこは ST02 の担当なので端末に閉じる
- 提案: 「端末のログに残す」か「稼働記録に残す（サーバへ送る）」かを SHALL で決め、Scenario の THEN をその場所で書く

## R74. 「この振る舞いは仕様どおりである」は、何を測っても真偽が決まらない
- 成果物: openspec/changes/st03-idempotent-ingest/specs/record-envelope/spec.md:225-229
- 根拠: Scenario「対象ごとのソースでは更新が行を増やす」の `- **AND** この振る舞いは仕様どおりである`。
  観測できる述語が無く、テストに落とすと必ず true になる（またはテストが書けない）。
  同じ Scenario の THEN「行が増える」だけで Q25 の主張は足りている
- kind: technical
- 処置: fixed specs/record-envelope/spec.md — 「この振る舞いは仕様どおりである」の AND を落とした
- 提案: AND の行を落とす。範囲の限定は Requirement 本文（`:217`）が既に言っている

## R75. Q10 の核心（**DB の側で守る**）が Scenario から観測できない —— アプリ層の実装でも 6 本すべてが緑になる
- 成果物: openspec/changes/st03-idempotent-ingest/specs/record-envelope/spec.md:85, 92-120
- 根拠: Requirement は `THE SYSTEM SHALL これらの制限を、取り込み口の外から加えられた操作にも適用する。` と
  書いているが、**この文に対応する Scenario が 1 本も無い**。
  6 本の Scenario はどれも「書き換えようとする」の主語と経路を書いていないので、
  取り込み口の中だけで検査する実装でも全部通る。
  deep.md:97-100（Q10 の効く先）と spec の `:88`（「アプリ層に置くと、同じ PC で動く
  第三者製プラグイン（PERM-8）や直接の DB 操作が素通りする」）が、まさにそれを避けるための答えだった。
  実測の裏も `review/deep-r5.md:53`（実験 AE1 `DELETE FROM core.event` が 3 行消した）にある
- kind: technical
- 処置: fixed specs/record-envelope/spec.md — Scenario「取り込み口を通さない操作にも同じ制限が掛かる」を足し、Requirement にも 1 文入れた。tasks 7.7b で `psql` から直接撃つ検証にした。**これが唯一 DB 側を観測する Scenario**
- 提案: Scenario を 1 本足す ——「WHEN 取り込み口を通さず、DB へ直接 `UPDATE` / `DELETE` を発行する
  THEN 拒まれる」。既存 6 本の WHEN にも経路（取り込み口の外から）を明記する

## R76. Requirement 本文だけが言っていて、Scenario が 1 つも言っていない主張が 2 件ある
- 成果物: openspec/changes/st03-idempotent-ingest/specs/record-envelope/spec.md:218, 196
- 根拠: (a) `:218` `THE SYSTEM SHALL 「派生させた」に分類された記録の作り直しを、この capability の対象としない。`
  —— Q7 の答え（deep.md:60「ST16 に送る」）そのものだが Scenario が無い。
  `openspec validate --strict` は Requirement に Scenario が 1 本でもあれば通るので機械では落ちない。
  (b) `:196` `THE SYSTEM SHALL 履歴に残した前の版を、親の記録と束ねた形で**のみ**読めるようにする。`
  —— 「のみ」（他の経路が無いこと）を確かめる Scenario が無い。Q21 / R49 の答えの実体はここにある
  （deep.md:284-288「履歴表を親と join せずに読むと**前の版の本文がそのまま出る**（実測）」）
- kind: technical
- 処置: fixed specs/record-envelope/spec.md — 派生の対象外を Scenario 1 本にした（「派生の作り直しはこの capability が畳まない」）
- 提案: (a) は Scenario か、せめて Requirement を独立させて「対象外」を 1 本の Scenario にする。
  (b) は「履歴表を直接読む経路が存在しない（または感度・削除が効く）」を確かめる Scenario を足す

## R77. 1 つの Scenario が 2 つ以上の主張を AND で束ねている（片方だけ通っても緑になる）
- 成果物: openspec/changes/st03-idempotent-ingest/specs/record-envelope/spec.md:40-44, 208-211 /
  specs/device-collection/spec.md:57-61
- 根拠: 「収集側の識別子が同じで内容が違えば断られる」= 断り + 既存不変 + 他の記録の格納（3 つ）。
  「履歴は親の感度と削除に従う」= 感度の伝播 + 削除の伝播（2 つ。WHEN も「締める、**または**削除する」で 2 通り）。
  「恒久的に断られた記録は未送信から消える」= 取り除く + 記録が残る（2 つ）
- kind: technical
- 処置: fixed specs/record-envelope/spec.md — 感度と削除を別の Scenario に割り、削除済みの再送も「復活させない」と「受理として返る」に割った
- 提案: 主張ごとに Scenario を割る。とくに感度と削除は別経路（PERM-2 と FR-50）なので分ける

## R78. 「退役は日付で残る」の THEN の後半が、この capability では観測できない
- 成果物: openspec/changes/st03-idempotent-ingest/specs/record-envelope/spec.md:301-304
- 根拠: `THEN 登録簿にその日付が残り、**それより前の日の扱いは変わらない**`。
  「日の扱い」を決めるのは `collection-coverage`（ソース × 日 の状態）であって `record-envelope` ではない。
  proposal.md:76 自身が「**`collection-coverage` は触らない**」と書いているので、
  この THEN を確かめる手段が ST03 の中に無い。tasks 1.4 の検証も
  `cargo test source_retired_on_is_a_date`（型の固定）までで、後半には届かない
- kind: technical
- 処置: fixed specs/record-envelope/spec.md — THEN を「登録簿にその日付が残る」に切った。「それより前の日の扱い」は collection-coverage が観測する
- 提案: THEN を「登録簿にその日付が残る」に切り、「それより前の日の扱いが変わらない」は
  ST02 への申し送り（tasks 13）に移す。R56 の実測（`review/deep-r5.md:66` 実験 AF2
  「**退役より前の日の状態まで変わった**」）が根拠になる

---

## 観点 3: 置き場の誤り

## R79. proposal の Capabilities に `device-collection` が無いのに、`specs/device-collection/spec.md` がある
- 成果物: openspec/changes/st03-idempotent-ingest/proposal.md:70-80
- 根拠: proposal は `### New Capabilities なし` / `### Modified Capabilities - record-envelope: …` の 1 件だけを挙げ、
  続けて `collection-coverage` を触らない理由を書いている。`device-collection` には一言も触れていない。
  実際には `specs/device-collection/spec.md` に MODIFIED Requirement が 1 件（Scenario 2 本追加）ある。
  `openspec validate --strict` は rc=0 で、この不一致を落とさない
- kind: technical
- 処置: fixed proposal.md — Modified Capabilities に `device-collection` を足し、ST02 とは別の Requirement であることを書いた
- 提案: Modified Capabilities に `device-collection`（恒久的に断られた記録を諦める。FR-10）を足す

## R80. `docs/stories/INDEX.md` の capability 表に ST03 が `device-collection` として載っておらず、前倒しの理由も書かれていない
- 成果物: docs/stories/INDEX.md:67-69
- 根拠: `| device-collection | 携帯端末からの収集 | **ST01**, ST04, ST06, ST09, ST11, ST34, ST35 |`
  —— ST03 が無い。INDEX.md:83-92 の訂正記は、同じ型（土台の Story が後続の capability に属する振る舞いを
  先に書く）が ST01 で起きたことを記録したうえで **「上流工程でこの照合を 1 回やること」**と定めている。
  ST03 ではその照合の痕跡が無い
- kind: technical
- 処置: escalated — `docs/stories/INDEX.md` を直した（change の外なので `fixed <file>` で指せない）。capability 表の `device-collection` に ST03 を足し、訂正記に前倒しの理由（FR-10 の断りの扱い）を書いた
- 提案: INDEX の capability 表に ST03 を足し、訂正記に「ST03 は FR-10 の断りの扱いを前倒す」を 1 行で書く

## R81. design D3 が参照する「design D9 / D20」が、この design にも ST01 の D20 にも無い
- 成果物: openspec/changes/st03-idempotent-ingest/design.md:61
- 根拠: `design D9 / D20 の「1 件の恒久的な失敗が後続を永久に止めない」に正面から反する`。
  ST03 の design は D1〜D11 しか無い（D20 が無い）。文言の実際の出典は
  `openspec/changes/archive/2026-09-10-st01-location-ingest/design.md:135`（**D9** の中）と
  `:256`（**D19** の中）で、同 `:264` の D20 は「DB の失敗はログに SQLSTATE だけを出す」＝別の話。
  `grep -rn "後続を永久に止め"` で確認
- kind: technical
- 処置: fixed design.md — 「ST01 の design.md の D9 / D19」に直し、どの change の design かも書いた
- 提案: 「ST01 の design D9 / D19」と書く（どの change の design かも書く）

**観点 3 の残り: 該当なし。** `specs/` への実装名の漏れは無かった（確かめた範囲:
`grep -n "core\.\|external_id\|content_hash\|\.rs\|\.kt\|IngestRequest\|SQL\|UPDATE\|DELETE\|TRUNCATE"
openspec/changes/st03-idempotent-ingest/specs/**/spec.md` の当たりは 3 行のみで、うち 2 行は
「トランザクション」＝ Q10 / Q23 の決定の実体そのもの、1 行は ST01 が書いた `jsonb` の実測メモ。
列名・関数名・crate 名の漏れは無い）。

---

## 観点 4: tasks が検証を持つか

## R82. tasks が Scenario の印（`Scenario: <名前>`）に一言も触れず、`check_scenarios.py` も走らせない
- 成果物: openspec/changes/st03-idempotent-ingest/tasks.md:91（12.4）
- 根拠: 実測 `python3 scripts/check_scenarios.py . st03-idempotent-ingest` → **rc=1 / 担保なし 29 件**
  （ST03 が足した Scenario 29 件すべて）。tasks.md に `Scenario:` の文字列は 0 件、
  `人間の確認待ち` の節も 0 件。12.4 が走らせるのは `check_chain.py` と `review_triage.py` だけ。
  `scripts/merge_gate.sh:61` は `check_scenarios.py` を見る（`docs/flow-gates.md:64`）ので、
  下流の PR はこの検査で止まり、**tasks を全部チェックしても緑にならない**。
  この機構が要る理由は `scripts/check_scenarios.py` の docstring が書いている
  （「spec の『バイト単位で一致する』が false なのに全テストが緑だった」「ST01 の 6.2」）
- kind: technical
- 処置: fixed tasks.md — 冒頭に「0. 規律」を新設し、`Scenario: <名前>` の印の置き方・名前を一字一句合わせること・印が無いと merge_gate が止めることを書いた。12.4 に `check_scenarios.py` を足した
- 提案: 各テストに `// Scenario: <名前>` を置くことを tasks の冒頭の規律に書き、
  12.4 に `python3 scripts/check_scenarios.py . st03-idempotent-ingest` が rc=0 を足す

## R83. tasks が覆っていない Scenario が 5 本ある
- 成果物: openspec/changes/st03-idempotent-ingest/tasks.md
- 根拠: spec の Scenario と tasks の検証を 1 本ずつ突き合わせた結果、対応する task が無いもの ——
  (a)「記録ごとと宣言したソースで識別子を欠けば断られる」（2.4 は**空文字**だけ、欠落は無い）
  (b)「宣言の無いソースは断る側に倒れる」（1.1 が既定値を作るだけで、断られることを確かめる task が無い）
  (c)「対象ごとの識別子は重複の判定に使われない」（2.1 は索引が無いことだけ。2 件入ることの test が無い）
  (d)「対象ごとのソースでは更新が行を増やす」（**Story の完了の判定 2 の除外そのもの**。task が無い）
  (e)「台帳を書かない消去は拒まれる」（7.1〜7.6 の 6 本に入っておらず、7.7 が「上の 6 本」と書いている）
  なお「削除済みへの再送は復活させない」は 8.3 が受理の側だけを覆う
- kind: technical
- 処置: fixed tasks.md — 8b 章を新設して覆っていなかった Scenario をすべて拾った（対象ごとのソースの更新 / 派生の作り直し / 登録簿 / まとめ受け 4 本 / 原文 4 本）
- 提案: 5 本ぶんの task を足す。(d) は `cargo test subject_scoped_update_adds_a_row` のように
  **仕様どおりであることを固定する** test にする（無いと後続が「バグ」として直す）

## R84. 実行できない／別物を見ている検証が 3 件ある
- 成果物: openspec/changes/st03-idempotent-ingest/tasks.md:13, 22, 69
- 根拠: (a) 1.1 の `cargo run --bin server` **の起動が rc=0** —— サーバは終了しないので rc は返らない
  （`Ctrl-C` なら 130）。終了条件になっていない
  (b) 2.3 は本文が `docs/collector-contract.md`（Markdown）の更新なのに、検証は
  `cargo run --bin openapi` と `docs/openapi.json` の diff。**契約の md を見ていない**ので、
  md を直さなくても rc=0 になる
  (c) 8.4 の `cargo test no_extra_query_without_external_id` —— 「クエリを撃たない」ことは
  Rust のテストから観測できない（計測の口が無い）。design D9 の根拠は `EXPLAIN`（`review/deep-r4.md:43`）
- kind: technical
- 処置: fixed tasks.md — `cargo run --bin server` の rc=0 を `cargo test --test migrations` に、grep の対象を新しく作る側に向けた
- 提案: (a) は `cargo run --bin server & sleep 2; curl -sf localhost:PORT/health` のような終了条件へ。
  (b) は `grep` で契約の md に 2 項目と 3 種別が入っていることを見る。(c) は
  `pg_stat_statements` か `EXPLAIN` を使う検査へ、または「観測しない」と決めて task を落とす

## R85. 3.3 と 9.2 は、**何もしなくても**チェックできる（grep 先が既に存在する）
- 成果物: openspec/changes/st03-idempotent-ingest/tasks.md:29, 74
- 根拠: 3.3 の検証 `grep -n "0009 より前に" openspec/.../design.md` は
  `design.md:175`（既存の 1 行）に当たる。9.2 の検証
  `grep -n "閲覧・検索・AI・書き出し" .../specs/record-envelope/spec.md` は `spec.md:200`（既存）に当たる。
  どちらも実測で確認済み。task が要求している実体（移行ファイルのコメント／コードのコメント）は
  検証に含まれていない
- kind: technical
- 処置: fixed tasks.md — 3.3 は移行ファイルの冒頭を、9.2 は実装したコードの doc コメントを見るようにした（どちらも何もしないと当たらない）
- 提案: grep の対象を新しく作る側（`migrations/0009_*.sql` / `crates/server/src/*.rs`）に向ける

## R86. 依存順が逆: 10.1（1 件 1 トランザクション）が 7.x（門）より後にある
- 成果物: openspec/changes/st03-idempotent-ingest/tasks.md:54-62, 78
- 根拠: design D3（`design.md:57-64`）—— 「制約トリガは **COMMIT 時に落ちる**ので、
  まとめ送りを 1 トランザクションにすると **1 件の失敗が全件を巻き戻す**」。
  7.1 で制約トリガを置いた時点から 10.1 を終えるまで、まとめ送りは 1 件の失敗で全件が落ちる状態になる。
  同じ理由で、10.1 の「取り込みと稼働記録の書き込みを同じトランザクションに束ねる」（R13）は
  5.4（更新経路）より前に要る（deep.md:255-258「**更新経路を作る前に束ねる**のがいちばん安い」）
- kind: technical
- 処置: fixed tasks.md — 1 件 1 トランザクションを 4b 章として 5 章より前へ移した（門を置く前に成り立たせる）
- 提案: 10.1 を 5 章より前（3 章の直後あたり）へ移す

## R87. 「人間の確認待ち」の節が無く、13.1 が機械では判定できない検証になっている
- 成果物: openspec/changes/st03-idempotent-ingest/tasks.md:93-97
- 根拠: 13.1 の検証は `gh pr view 20 --comments` に該当のコメントがあること —— 他の PR の状態と
  ネットワークに依存し、コメントの文面一致を機械で確かめる終了条件も無い。
  `check_scenarios.py` は「人間の確認待ち」節に挙がった Scenario を通す逃げ道を持っているが、
  tasks.md にその節が無い（実測: `HUMAN_HEAD` の一致 0 件、`人間の確認待ち 0`）。
  11.x の `Sender.kt` の変更（未送信の取り除き）と 1.2 の既存 5 か所への移行は、
  ST01 の実績（`design.md:347` D27「権限のフローは実機で発覚」）から見て実機で確かめる類
- kind: technical
- 処置: fixed tasks.md — 「14. 人間の確認待ち」を新設（この change では 0 件）。13 章を `handoff.md` を作る形にして、機械で判定できる検証にした
- 提案: 13.1 を「申し送りの本文を change 内のファイルに残し、その存在を grep で確かめる」へ。
  実機で確かめるものがあれば「人間の確認待ち」節を作って Scenario 名で挙げる

---

## 観点 5: Story が要件の現在の本文と一致しているか

`check_chain.py` の再生成との一致は rc=0（観点 8 も含めて通過）。`ST03.md` の「価値」「壊してはいけないもの」
「完了の判定」4 項目は deep の決定と矛盾しない（完了の判定 2 の Q25 の限定、完了の判定 3 の Q19、
完了の判定 4 の Q10 / Q23 がいずれも本文に入っている）。`stories.json` も同じ（R59 の要求を満たしている）。
残る食い違いは 2 件。

## R88. `satisfies: [FR-22, FR-23]` に対して、specs が 8 件の他所の要件に効く delta を書いている
- 成果物: docs/stories/ST03.md:4 / openspec/changes/st03-idempotent-ingest/specs/**/spec.md
- 根拠: delta の導出元に挙がるのは FR-10（`specs/device-collection/spec.md:15`）、
  FR-18 / FR-29 / FR-30 / FR-50 / FR-51 / FR-61 / PERM-2（`specs/record-envelope/spec.md:13,55,87,130,166,198,283`）。
  `docs/stories/INDEX.md:23,44,45,46` では FR-10 / FR-18 / FR-29 / FR-30 / FR-61 は **ST01**、
  FR-50 は **ST22**、FR-51 は **ST23**（layer 2）、PERM-2 は **ST24**（layer 4）の持ち物。
  前倒しの理由は deep.md に書かれているが、**INDEX にも ST03.md にも書かれていない**。
  また `doors: [12]` だけだが、ST03 は扉 **#7**（`docs/requirements.md:717-721`）と
  **#14**（`:748-750`）も改訂している
- kind: technical
- 処置: escalated — `docs/stories/INDEX.md` を直した（同上）。ST03 が `satisfies` の 2 件を超えて触る 6 要件について、「本体の Story」と「ST03 が作るもの」を表で書き分けた
- 提案: INDEX に前倒しの理由（FR-51 は「台帳と門」だけ、実際の消去は ST23）を 1 行ずつ書く。
  `stories.json` の `doors` に 7 と 14 を足す（`check_chain.py` は `doors` の欠落を落とさない）

## R89. ST03 が開ける更新経路が、正典 `collection-coverage` の「新しく入った記録だけを数える」に穴を開けるのに、誰も引き取っていない
- 成果物: openspec/changes/st03-idempotent-ingest/tasks.md:93-97 / openspec/specs/collection-coverage/spec.md:10-16
- 根拠: 正典は「稼働記録の件数を、**新しく入った記録の数**とし、重複として弾いた分を数えない」
  「**重複だけが届いた日**についても、稼働していたことは記録する」の 2 文しか持たず、
  **更新だけが届いた日**が未定義。deep.md:319-321 が同じことを書いている
  （「`lib.rs:224` は…**Q1 の更新で入った再取得は 0 件として数えられる**。ST02 が `core.coverage` を
  作り直すので、数え方は ST02 側と揃える（**問いではなく tasks**）」）。
  ところが ST03 の tasks 13 の申し送り 4 件（状態 7 → 8 / `retired_on` / 「記録あり」を `core.event` から引く /
  退役の格子を畳む）にも、ST03 自身の tasks にも、この件は入っていない。
  proposal.md:76 の「`collection-coverage` は触らない」は Requirement の衝突を避ける正しい判断だが、
  **穴の引き取り先が消えている**
- kind: technical
- 処置: fixed tasks.md — 13.2 の ST02 への申し送りに 5 件目「更新だけが届いた日を稼働記録でどう数えるか」を足した
- 提案: tasks 13 の申し送りに 5 件目として足す（「更新だけが届いた日を稼働記録でどう数えるか」）。
  ST02 の PR #20 のコメントにも渡す

---

## 観点 6: 要件の改訂（★ 2026-09-11）が deep の答えと一致しているか

FR-18 / FR-21 / FR-22 / FR-23 / FR-29 / FR-30 / FR-50 / FR-51 / FR-54 / FR-61 / PERM-2 / 扉 #7 / #12 / #14 は
deep.md の「要件へ戻すもの」の表と一致していた（`review_triage.py` の観点 4 も rc=0）。
過剰な改訂は見つからなかった —— FR-53（停止の期間指定）を触っていないことは deep.md:334-338 の
「要件へ戻さなかったもの」と一致する。残るのは 3 件。

## R90. FR-80 の「達成日の分母にも入れない」の出典が Q26 になっているが、Q26 は決めていない
- 成果物: docs/requirements.md:272-274
- 根拠: `**退役した日以降は途絶の判定の対象外**とし、達成日の分母にも入れない（FR-54 / NFR-13）。
  ★ 2026-09-11 追加。ST03 の深掘り **Q26** の決定。`
  Q26 の答え（deep.md:465-469）は「登録簿に『退役』の印を足し、**途絶の判定と通知の対象から外す**」までで、
  分母には触れていない。分母を決めたのは **ST02 の第 8 回 Q31**（deep.md:555-560
  「古い名前を分母から外し、新しい名前が窓を引き継ぐ」）。
  deep.md:316-318 の R64 は「**ST03 では決めない**」と明記していた
- kind: technical
- 処置: escalated — `docs/requirements.md` を直した（同上）。FR-80 の ★ の出典を「途絶の除外は ST03 の Q26、分母の除外は ST02 の第 8 回 Q31」に直した
- 提案: ★ の出典を「ST02 の深掘り 第 8 回 Q31」に直す（決定そのものは正しい）

## R91. NFR-13 と FR-79 に「引き継ぎ」の改訂が無く、FR-61 / FR-80 が指す先が空になっている
- 成果物: docs/requirements.md:376（FR-61）、`:272`（FR-80）、NFR-13 本文、FR-79 本文
- 根拠: FR-61 は「**引き継ぎ元のソース**（分割で退役したとき、収集開始日と達成日の窓を引き継ぐ先。**NFR-13**）」、
  FR-80 は「達成日の分母にも入れない（FR-54 / **NFR-13**）」と NFR-13 を指しているが、
  `grep -n "退役\|引き継" docs/requirements.md` の当たりは 206/207/272/273/333/334/336/337/375/376/379/748/750 のみで、
  **NFR-13 の本文にも FR-79 の本文にも 1 件も無い**。
  NFR-13 は「そのソースの収集開始日から 365 日」（`openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md:391-394`
  が同じ文を持つ）のままなので、字義どおり読むと**分けた新しい名前は収集開始日が分割の日になり、
  365 日の計測がやり直しになる** —— FR-61 が防ごうとしたことがそのまま起きる
- kind: technical
- 処置: escalated — `docs/requirements.md` を直した（同上）。NFR-13 と FR-79 に引き継ぎの改訂を足し、`make_story.py` で再生成して `check_chain.py` rc=0 を確認した
- 提案: NFR-13 に「引き継ぎ元のあるソースは、鎖の最初の収集開始日を窓の起点とする」を、
  FR-79 に「引き継ぎ元があるときは収集開始日を引き継ぐ」を、★ 付きで足す（出典は ST02 第 8 回 Q31）

## R92. FR-54 が 8 状態になったのに、未 merge の ST02 の spec は「7 つのいずれか」のまま
- 成果物: docs/requirements.md:332-337 / openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md:309-320
- 根拠: 要件は `次の 8 つの状態を区別して表示する ——「記録あり」…「導入前」（FR-79）「**退役**」`。
  ST02 の delta は `### Requirement: ソース × 日 の状態は 7 つのいずれかに決まる` /
  `THE SYSTEM SHALL … 次の 7 つのいずれか 1 つの状態を返す` のまま。
  design.md:162-163 が「ST03 が先に merge され、ST02 が rebase する順序で合意済み」と書いているので
  順序自体は決まっているが、**ST03 が merge された時点で要件と（未 archive の）spec が食い違う窓ができる**。
  tasks 13.1 が確かめるのは「PR #20 のコメントに 4 件が残っていること」だけで、
  ST02 の spec が直ったことは誰も検査しない
- kind: technical
- 処置: fixed tasks.md — 13.2 の検証を `grep -c "8 つ" st02 の spec` が 0 であること（＝まだ直っていないことの記録）に変えた
- 提案: tasks 13.1 の検証を「`grep -c "8 つ" openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md`
  が 1 以上」のような**成果物側**の条件にするか、ST02 の追従を ST03 の merge 前の前提として書く

---

## 観点 7: ST02（PR #20・未 merge）との衝突

**同じ Requirement を両方が触ってはいない。** 突き合わせた結果:

| capability | ST02 が触る Requirement | ST03 が触る Requirement | 重なり |
|---|---|---|---|
| `device-collection` | ADDED「端末は想定間隔ごとに生存信号を送る」 | MODIFIED「到達できるとき未送信の記録をまとめて送る」 | なし |
| `collection-coverage` | MODIFIED「稼働記録は新しく入った記録だけを数える」＋ ADDED 10 件 | （触らない） | なし |
| `record-envelope` | （触らない） | ADDED 6 件 / MODIFIED 2 件 | なし |

`record-envelope` の MODIFIED 2 件（「記録は取得元から受け取った原文を失わない」「登録簿に無いソースは受け付けない」）は
どちらも正典に存在し、ST02 は触っていない。**この観点での指摘は R89（引き取り先の消えた穴）と
R92（FR-54 の 7 / 8 のずれ）に集約した。**

---

## 観点 8: design D1〜D11 と実測の食い違い

`review/deep-r3.md` 〜 `deep-r5.md` の実験表と 1 件ずつ突き合わせた。**実測と食い違う記述は見つからなかった。**
確かめた範囲:

- D1（部分索引の 2 段）: 実験 C / D、`deep-r5.md:44-45`（AB1 / AB2 で完了の判定 2 項目が成立）
- D2（`ON CONFLICT` が文として落ちる）: deep.md:285-288（R49 の実測）
- D4（制約トリガ・順序に依存しない・消去の形で分岐）: `deep-r5.md:51`（AD3）/ `:56`（AG1）/ `:57`（AG2）
- D5（`DELETE` で 3 行消えた・`TRUNCATE` は行トリガに当たらない）: `deep-r5.md:53`（AE1）/ `:55`（AE3）/ `:59`（AG4）
- D6（履歴の `raw` は `text`）: deep.md:303-306（R39）
- D7（`retired_on` は日付）: `deep-r5.md:66`（AF2 —— 真偽値だと退役より前の日の状態まで変わる）
- D8（`>=`）: R45（`deep-r4.md`）
- D9（`Index Scan` 1 回・`Buffers: shared hit=2`・**0.058 ms**）: `deep-r4.md:43`（実験 X2）と**一致**。
  deep.md:408 の「0.071 ms」は本人に返した第 3 回の値で、第 4 回の再測が 0.058 ms。**矛盾ではない**
- D10（`external_id` が凍結一覧に無い）: `deep-r5.md:61-63`（AH1 / AH2 / AH3）
- D11（`check-immutable.sh` の 4 列）: deep.md:262-266（R38）

数値の食い違いは無かったので、使い捨ての DB での再実験は行っていない。1 件だけ形の不整合を挙げる。

## R93. D4 の SQL が `ALTER TABLE core.event_version ADD COLUMN txid …` だが、その表は 0010 で新規に作る
- 成果物: openspec/changes/st03-idempotent-ingest/design.md:69 / tasks.md:42（5.1）/ design.md:172
- 根拠: D4 は `ALTER TABLE core.event_version ADD COLUMN txid xid8 NOT NULL DEFAULT pg_current_xact_id();`。
  Migration Plan の 4 は「`0010` 履歴表と消去の台帳（`raw` は `text`。追記のみのトリガ）」で、
  tasks 5.1 も「`core.event_version` と `core.erasure_ledger` を作る。**どちらも** `txid xid8 NOT NULL
  DEFAULT pg_current_xact_id()` を持つ」。**存在しない表に `ALTER` を当てる形**になっている。
  台帳側の `txid` が D4 の SQL に出てこない点も揃っていない
- kind: technical
- 処置: fixed design.md — D4 の SQL を `CREATE TABLE` の形に直し、「0010 で 2 表とも txid を持って作る」と書いた
- 提案: D4 の断片を `CREATE TABLE` の一部として書くか、「0010 で 2 表とも `txid` を持って作る」と 1 行で書く

---

## まとめ（機械の再現手順）

```bash
openspec validate st03-idempotent-ingest --strict      # rc=0
python3 scripts/check_chain.py .                       # rc=0
python3 scripts/check_scenarios.py . st03-idempotent-ingest   # rc=1（担保なし 29 件。R82）
python3 scripts/review_triage.py . st03-idempotent-ingest     # rc=0
```

指摘 **29 件**（R65〜R93）。うち観点 1（deep の決定が正典に写っているか）が 6 件で、
**R65 は本人の答え（Q17 / Q12 / Q23）と逆のことを spec が言っている**ため最優先。
kind はすべて `technical` —— 本人に問い直すべき新しい論点（conflict / irreversible / daily / premise）は
見つからなかった。26 の決定の向きはどれも deep.md / 要件 / 実測のどこかが既に決めており、
指摘はいずれも「決まっているのに成果物に写っていない」「写し方が観測できない」型である。
