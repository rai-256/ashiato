# ST07 深掘りの独立レビュー（deep-questions.json）

**やり方**: `openspec/schemas/ashiato/schema.yaml` の `deep` の手順 1〜5 を、
問いの一覧を**開かずに**先に自分でやり直し、終わってから
`openspec/changes/st07-active-window/deep-questions.json`（9 問 / A 6・B 3）と突き合わせた。
問いの JSON も他のファイルも編集していない。

## 手順ごとの結果

- **手順 1（satisfies の要件どうしの衝突）** —— FR-12 × NFR-5（容量）は Q5 が、
  PERM-3 × PERM-6（感度の既定）は Q2 が、NFR-13 × C-02 の実態は Q1 が、
  FR-35 × C-02 の実態は Q9 が受けている。**新しく見つけたのは R1 / R10。**
  NFR-1（1 時間以内）は「位置とアプリ利用」だけを名指ししており（docs/requirements.md:465）
  ウィンドウに遅延の目標は無い —— 送信間隔は可逆で費用も小さいので問いは要らない。
- **手順 2（決定済の扉の幅）** —— 扉 #15 は Q2 が受けている。
  **幅が残っているのに一覧に無いのは 扉 #5（R8）・扉 #7（R2）・扉 #12（R3）・扉 #14（R1）。**
- **手順 3（新たに立つ一方通行）** —— Q3 / Q4 / Q5 / Q6 が URL・除外・間引き・離席を受けている。
  **差分は R1（PC が止まっていた期間）・R2（原文と解析済みの形＝鍵の入力）・R4（未送信の保持）。**
- **手順 4（日常に影響する選択）** —— 通知は Q9、容量は Q5、CPU は Q8 が触れている。
  **差分は R9（常駐・自動起動・毎日目に入るトレイ）。** 面は無い（`docs/stories/ST07.md` /
  ブリーフの「面: なし」）ので、**画面の構造を文字で問うている問いは 1 つも無い —— 該当なし**。
- **手順 5（既存コードが要件を満たしていない箇所）** —— R3 / R4。
  `crates/collector-windows/src/main.rs` は 6 行のスタブで、これは「要件を満たしていない」
  ではなく未実装。`core.source.user_id` が seed で NULL のままなのは確かめたが
  （`migrations/202609111111_coverage_rebuild.sql:126-133`）、FR-29 は列を持たせよと言うだけで
  ST07 の判断ではないので指摘にしない。

---

## R1. PC が止まっていた期間を残す手段が、どの問いにも無い（Q1 と Q9 の土台）

- 成果物: openspec/changes/st07-active-window/deep-questions.json
- 根拠: Q9 の選択肢 3 が「**『PC が切れている』と『C-02 が死んでいる』は外から区別できない**」と
  自分で書いているが、**区別する手段を問う問いが無い**。
  `crates/server/src/coverage.rs:825` が `gap_days = 21600 / 86400 = 0.25` を作り、
  同 `:806-813` の `near` は `diff <= 0.25` なので**同じ日しか近傍にならない** ——
  記録も生存信号も無い日は即 `DayState::Outage`（⑥途絶）になる。
  `active_days`（同 `:723-741` の UNION）が見るのは `core.event` と `core.heartbeat` だけで、
  「PC が止まっていた」を表す行は 1 つも無い。
  FR-80（docs/requirements.md:280）は「**止まったものは自分から報告しない**」と明記し、
  扉 #14（同 :755-761）が求める区別の担い手を、生存信号（FR-78）と
  「受け手の側が『来ないこと』を検知する」FR-80 の 2 つに置いている。
  C-02 では**両方とも電源とともに消える**ので、扉 #14 の区別が原理的に成立しない。
  次の起動時に「前回の停止から今までが空白だった」ことを残せば区別は作れるが、
  **残さなかった期間は後から作れない**（Windows のイベントログは既定の大きさで巡回し、
  古い起動・停止の記録から消える）。
- kind: irreversible
- loss: uncaptured
- 提案: **A で 1 問足す** ——「C-02 の起動時に、前回の停止からの空白区間を
  『PC が止まっていた』として稼働記録（`core.coverage_span` 相当）に残すか」。
  Q1 の分母の議論も Q9 の通知の議論も、この事実が残っていて初めて
  「PC が切れていた」と「C-02 が死んでいた」を分けられる。
  残さない場合、1 年の格子（FR-54）は**週末ごとに⑥途絶**を並べる。
- 処置: escalated — **Q1 として新設し、先頭に置いた**（`ask_wizard` の A。`irreversible` / `loss: uncaptured`）。選択肢を 3 つにし、推奨を「記録として 1 件残す」（`desktop-collection` の中で閉じ、格子の読み方は後から決め直せる側）に置いた。指摘のとおり `coverage_span` に種別を足す案も選択肢 2 として残したが、**ST07 が持たない capability の表**だと明記した。`coverage.rs:806-813, 825` の `gap_days = 0.25` と `coverage_span` の `CHECK (kind IN ('stopped','dropped'))` を実物で確認したうえで context に入れた

## R2. 記録 1 件に何を載せるか（原文と解析済みの形）が、day one で凍結されるのに問いが無い

- 成果物: openspec/changes/st07-active-window/deep-questions.json
- 根拠: 冪等キーは `crates/server/src/ingest.rs:105-115` で
  `logical_source` + `event_time`（マイクロ秒）+ **`raw` の文字列そのもの**から作られる。
  `raw` と `payload` は `migrations/202609100000_immutable_origin.sql:23-33` のトリガで
  凍結され、FR-30（docs/requirements.md:175）が書き換えを禁じる。
  `raw` は `text`（`migrations/202609092315_raw_text.sql`）なので **SQL から引けない** ——
  `docs/collector-contract.md` §冪等キーが「収集側は 1 件を同じ形で直列化し続けなければならない」
  と書いているとおり、**直列化の形を後から変えると同じ 1 件が別の鍵になる**。
  引く側は `payload` を使うしかなく（同ファイル）、FR-58（同 :363）は
  「アプリ名・ウィンドウ題名・URL」を検索対象に名指ししている。
  それなのに一覧には、**1 件に載せる項目**（実行ファイルのパス / プロセス名 / 表示名 /
  ディスプレイ・全画面か / 前景プロセスの PID）と**その並べ方**を決める問いが 1 つも無い。
  取らなかった項目は後から作れない（uncaptured）。ST02 の実測（`raw` を `jsonb` にして
  「そのまま残す」が成立していなかった）と同じ型で、**型と鍵の入力は製造準備で既に決まっている**。
- kind: technical
- 提案: **C に置く（design の D 番号）。** 既定は「扉を開けたままにする側」——
  実行ファイルのパスとプロセス名を**列として持ち**、`raw` は収集側が組んだ JSON を
  文字列のまま送る。**Q3（URL）と Q6（離席）の答えで載る項目が変わる**ので、
  依存関係を design に明記する。問いにはしない（片方の選択肢が扉を開けたままにし、費用が小さい）。
- 処置: fixed D1 — 問いにはしない。design の D1 に「実行ファイルのパスとプロセス名を列として持ち、`raw` は収集側が組んだ JSON を文字列のまま送る」を置く（扉を開けたままにする側）。**Q4（URL）と Q7（離席）の答えで載る項目が変わる**ので、D1 に依存関係を書く

## R3. `c02-window` は外部識別子を持たないのに、ST03 が入れる登録簿の既定は `'record'`

- 成果物: openspec/changes/st07-active-window/deep-questions.json
- 根拠: FR-23（docs/requirements.md:156）は「『記録ごと』と宣言されたソースからの記録が
  識別子を欠けば **400 で断る**。**書き忘れたときは『記録ごと』に倒す**」。
  `openspec/changes/st03-idempotent-ingest/tasks.md:25` が
  `external_id_kind text NOT NULL DEFAULT 'record'` を足し、同 `:26` が既存の端末ソースを
  `'none'` に落とす UPDATE を書いているが、その `NOT IN (…外部ソース…)` は**省略記号のまま**で、
  `c02-window` が含まれるかが決まっていない。
  `c02-window` は既に登録簿にある（`migrations/202609111111_coverage_rebuild.sql:131`）。
  ウィンドウの記録に外部サービス上の識別子は存在しない（`external_id` は常に null）ので、
  `'record'` のまま残ると **ST07 の記録が全件 400 で断られる**。
- kind: technical
- 提案: **C に置く（tasks / design）。** ST07 の移行で `c02-window` を
  `external_id_kind='none'` にするか、`docs/handoff/ST03.md` に 1 件書く
  （`docs/handoff/README.md` の規則 2(ii)。**ST03 は走行中なので差し戻さない**）。
  問いにはしない —— FR-23 が既に決めていて幅が無い。
- 処置: fixed D2 — ST07 の移行で `c02-window` を `external_id_kind='none'` にする。**ST03 は走行中なので差し戻さない**（`docs/handoff/README.md` の規則 2(i)：見つけた Story 自身の change で直す）。ST03 の移行が先に main に入る前提で、当てる順に依存しない形（`UPDATE … WHERE logical_source='c02-window'`）にする

## R4. C-02 の未送信（S-01 が止まっている間の記録）を扱う問いも既定も無い

- 成果物: openspec/changes/st07-active-window/deep-questions.json
- 根拠: FR-8 / FR-9 / FR-10（docs/requirements.md:95-99）は **すべて C-01 を名指ししており**、
  C-02 には未送信の保持も、捨てたときに稼働記録へ残す義務も書かれていない。
  `openspec/specs/device-collection/spec.md` の Outbox の要件も C-01 のもの。
  S-01 と C-02 は同じ Windows PC の上で動きうるが（docs/requirements.md:644）、
  サーバの再起動・移行の適用・DB の停止中に生成されたウィンドウの記録は、
  未送信を持たなければ**その場で消える**（後から作れない）。
  ウィンドウは 1 日数千件出る想定（Q5 の表）なので、数分の停止でも数百件になる。
  既定（未送信を持つ）を採れば失われるものは無いので、`loss` は付けない。
- kind: technical
- 提案: **C に置く（design の D 番号）。** 既定は ST01 と同じ形の追記 JSONL の未送信を持つこと
  （`docs/collector-contract.md` §再送の扱い）。
  **上限（FR-8 の 90 日 / 2 GB を C-02 にも及ぼすか）と、捨てたときに
  `coverage_span(kind='dropped')` を書くか**は要件の穴なので、proposal で明示的に触れる。
- 処置: fixed D3 — 既定は「ST01 と同じ形の追記 JSONL の未送信を持つ」。指摘のとおり**上限（FR-8 の 90 日 / 2 GB を C-02 にも及ぼすか）と、捨てたときに `coverage_span(kind='dropped')` を書くか**は要件の穴なので、proposal の Impact で明示的に触れる

## R5. Q7（生存信号の「取得できる状態か」）は要件と spec で既に決まっていて、幅が無い

- 成果物: openspec/changes/st07-active-window/deep-questions.json
- 根拠: FR-78（docs/requirements.md:218-222）は取得可否と「何が満たされていないか」を
  載せることを求め、`openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md`
  の「理由の無い『取れない』は受け付けられない」が受け口として強制している
  （`blockers` が空の `capturable=false` は `missing_blockers` で断られる。
  `docs/collector-contract.md` §送る形）。
  NFR-13 の訂正 (2)（docs/requirements.md:524-526）は
  「**権限が剥がれたまま 1 年放置すると 365/365 の達成**になる」ことを名指しで欠陥としている。
  Q7 の選択肢 2（常に「取得できる」と報告する）は**この 3 つに正面から反する**ので、
  選べる選択肢ではない。選択肢 1 は要件の逐語をなぞっているだけ。
- kind: technical
- 提案: **C に落とす。** 問いから外し、design の D 番号に
  「前景ウィンドウが取れること（＋ Q3 で URL を採るなら UI Automation の応答）を見る」と書く。
  人間の時間を使わせない。
- 処置: fixed deep-questions.json — 旧 Q7 を**問いから外した**。design の D4 に「前景ウィンドウが取れること（＋ Q4 で URL を採るなら UI Automation の応答）を見る」を置く。指摘のとおり選択肢 2 は FR-78 と ST02 の受け口と NFR-13 の訂正 (2) に正面から反しており、選べる選択肢ではなかった

## R6. Q8（変化の拾い方）は問い自身が可逆だと書いていて、C に落ちる

- 成果物: openspec/changes/st07-active-window/deep-questions.json
- 根拠: Q8 の `why` が「**後からいくらでも差し替えられます**（取り方が変わっても記録の形は
  変わらない）」と自分で書いている。3 つの選択肢のうち「イベント通知＋見回りの併用」が
  取りこぼしを作らない側で、費用は 1 秒ごとの `GetForegroundWindow` 1 回 ——
  片方の選択肢が扉を開けたままにし、費用が小さい（C の定義そのもの）。
  記録の形に触れないことは R2 の根拠（鍵の入力は `raw` の文字列）からも裏づけられる。
- kind: technical
- 提案: **C に落とす。** design の D 番号に「取りこぼさない側（併用）を既定」とだけ残す。
  ただし**取りこぼした変化は後から作れない**ので、D の反転条件に「CPU が実測で問題になったら」と書く。
- 処置: fixed D5 仮 — 旧 Q8 を**問いから外した**。design の D5 に「イベント通知＋1 秒の見回りの併用（**仮**）」と置き、反転条件を「CPU が実測で問題になったら見回りを止める」と書く。指摘のとおり取りこぼした変化は後から作れないので、取りこぼさない側を既定にする

## R7. Q5 の容量の概算のうち、「ウィンドウ以外 年 0.87 GB」が同じ表の 891 B/行 と合わない

- 成果物: openspec/changes/st07-active-window/deep-questions.json
- 根拠: Q5 の `context` は「1 行 ≒ 891 B」を置き、表の 5 行はその値と整合している
  （3,000 件/日 × 891 B × 365 = 0.976 GB、6,540 → 2.126 GB、10,000 → 3.25 GB、
  15,765 → 5.13 GB、20,000 → 6.50 GB。すべて再計算で一致）。
  一方「位置 1,440 + アプリ利用 500 + 履歴 300 + 健康 1,000 = 3,240 件/日」を同じ 891 B で
  引くと `3,240 × 365 × 891 = 1.054 GB` で、**書かれている 0.87 GB にならない**
  （0.87 GB は 1 行 736 B に相当）。従って「ウィンドウに使えるのは年 2.1〜5.1 GB
  = 1 日 6,500〜15,800 件」も実際は **1.95〜4.95 GB = 1 日 5,990〜15,200 件**。
  併せて、NFR-5（docs/requirements.md:473）が名指しする 6 種のうち
  **「写真メタ」が内訳から落ちている**（年 2,000 枚なので影響は小さい）。
  結論（通常のウィンドウ切り替え 1,500〜3,000 件/日 は枠に収まり、問題は
  「題名が流れ続ける」型だけ）は**変わらない**。
- kind: premise
- 提案: 数値を 1.05 GB / 5,990〜15,200 件に直すか、「他ソースは 1 行 736 B で見積もった」と
  明記する。**問いの結論は動かないので、選択肢は触らない。**
- 処置: fixed deep-questions.json — Q6 の context を **891 B/行 で一貫**させた。他ソースの概算を 0.87 GB → **1.05 GB**、ウィンドウの枠を 2.1〜5.1 GB → **1.95〜4.95 GB**、1 日あたりを 6,500〜15,800 件 → **5,990〜15,200 件**に直し、表の「上限」行も 6,540 → 5,990 / 15,765 → 15,207 に差し替えた。内訳に**写真メタ（年 2,000 枚）**を足した。指摘のとおり結論は動かないので選択肢は触っていない

## R8. 扉 #5（端末時計のずれ）が C-02 に及ぶかは、この Story の文脈で決まっていない

- 成果物: openspec/changes/st07-active-window/deep-questions.json
- 根拠: 扉 #5（docs/requirements.md:720-724）は「決定済（1 時間ごとに測定記録を残す。
  **『常に正しい時刻を確保する』はオフライン時に必ず破れ、破れたことを後から知る手段が無い**ため）」
  だが、関与要件の FR-7（同 :93）は **C-01 だけを名指ししている**。
  ST07 の `event_time` は PC の時計そのものであり（R2 のとおり鍵の入力でもある）、
  ずれていた期間の測定記録は**後から作れない**。
  S-01 と同じ PC の上で動くなら差は 0 で費用も 0 だが、**同じ PC である保証は要件に無い**
  （docs/requirements.md:644 は「C-02 は Windows」としか書いていない）。
  既定（C-02 も測る）を採れば失われるものは無いので、`loss` は付けない。
- kind: technical
- 提案: **C に置く（design の D 番号）。** 既定は「C-02 も 1 時間ごとにずれの測定記録を出す」
  （扉を開けたままにする側）。別の PC での運用を許すと決めるなら A に上げる余地がある。
- 処置: fixed D6 — 既定は「C-02 も 1 時間ごとにずれの測定記録を出す」（扉を開けたままにする側。同じ PC なら差は 0 で費用も 0）。FR-7 が C-01 だけを名指ししている件は、**扉 #5 の関与要件の穴**として proposal の Impact に書き、ST05（`record-envelope`）へ `deferred` の候補として残す

## R9. 常駐・自動起動・トレイの可視性（毎日目に入るもの／手作業）が手順 4 から漏れている

- 成果物: openspec/changes/st07-active-window/deep-questions.json
- 根拠: NFR-12（docs/requirements.md:485）は「収集に要する手作業は **Google 系のみ**
  2 か月に 1 回まで。**他のすべてのソースは手作業を要さない**」。
  C-02 がログオン時に自動起動しなければ、PC を再起動するたびに手で起動することになり
  NFR-12 に反し、起動を忘れた期間の記録は後から作れない
  （FR-80 の途絶としては見えるが、中身は戻らない）。
  一覧で日常に触れているのは Q9（通知）と Q5（容量）と Q8（CPU）だけで、
  **常駐の可視性（トレイの常設・「収集中」表示の有無）は 1 問も無い** ——
  schema の手順 4 が名指しする「毎日目に入るもの」にあたる。
- kind: daily
- 提案: **C に置く（design の D 番号）。** 既定は「ログオン時に自動起動し、トレイに常駐する」
  （止まっていることに気づける側）。後から消しても何も失われないので問いにはしない。
- 処置: fixed D7 仮 — design の D7 に「ログオン時に自動起動し、トレイに常駐する（**仮**）」を置き、反転条件を「常駐表示が邪魔だと本人が言ったら消す」と書く。NFR-12（他のすべてのソースは手作業を要さない）を満たすのは自動起動の側なので、既定はそちらに倒す

## R10. Q1 の答えが ST07 の capability の外（`collection-coverage`）に落ちることが書かれていない

- 成果物: openspec/changes/st07-active-window/deep-questions.json
- 根拠: Q1 の選択肢 1 と 2 はどちらも **NFR-13 の改訂**（docs/requirements.md:490-）と、
  `openspec/specs/collection-coverage/spec.md` の delta と、
  `crates/server/src/coverage.rs:935-960`（`Subject::Usage` の分母と分子の数え方）の変更を伴う。
  だが `docs/stories/INDEX.md:70` のとおり ST07 の capability は `desktop-collection` で、
  `collection-coverage` は同 :72 のとおり **ST01 / ST02 / ST14 / ST15** のもの。
  CLAUDE.md の盤面の規則は「同じ capability を 2 本が同時に触ると差し戻しが起きる」
  （実測: ST02 と ST03 が 12 時間で 5 往復）と書いている。
  問いの本文・`context` のどこにも、答えがこの範囲に出ることが書かれていない。
- kind: defer
- 提案: **問いは A のまま残す**（`premise` の判定は正しい）。
  答えを受けた後の `deep.md` / `proposal.md` に「NFR-13 の改訂と `collection-coverage` の
  delta は ST07 の外で、`fix/` の小さな change か ST14 / ST15 で拾う」と経路を書く。
  ST07 の specs を `desktop-collection` だけで閉じられるかを、proposal の時点で確かめる。
- 処置: fixed deep.md — 指摘のとおり問いは A のまま残した。Q2 の context に**答えの落ち先**（NFR-13 の本文改訂はこの深掘りで行い、`collection-coverage` の実装は ST02 の archive 後に小さな change で拾う。**走っている Story へ差し戻さない**）を追記した。ST07 の specs を `desktop-collection` だけで閉じられるかは proposal の時点で確かめる
