# ST41 spec レビュー（spec --draft・第 1 回の先読み）

対象: `openspec/changes/st41-event-end-time/`（推奨の答えで書いた proposal / specs 3 本 / design / tasks）。
deep.md の `（答え待ち・推奨: …）` を推奨の答えとして読んだ。観点 6（answers.md）は `--draft` なので書かない。

## 機械の検査（2026-10-22）

```
$ openspec validate st41-event-end-time --strict
Change 'st41-event-end-time' is valid                                   rc=0

$ python3 scripts/check_chain.py .
  要件 142 件 / Story 47 本 / 扉 31 項
  [ok]   どれかの Story に拾われた要件: 140/142 件
  [ok]   INDEX の「Story の対象外」: NFR-11, NFR-8
chain: OK (0 件 / 未回収 0 件 / warn 0 件)                              rc=0

$ python3 scripts/check_scenarios.py . st41-event-end-time
  Scenario 1531 件 / 印 1897 個 / 担保あり 1485 / 人間の確認待ち 0
  [warn] spec に無い Scenario を指す印 …（15 件。どれも ST41 の外のファイル）
  [FAIL] 担保の無い Scenario: …（46 件。すべて ST41 で新しく足した Scenario）
scenarios: FAIL (担保なし 46 件 / 名無しの確認待ち 0 件)                  rc=1

$ python3 scripts/review_triage.py . st41-event-end-time
  指摘 8 件 / 仮決め 0 件 / 要件へ戻すもの 0 件
  [FAIL] deep.md R1: escalated なのに deep.md に R1 が無い
  [FAIL] deep.md R2: escalated なのに deep.md に R2 が無い
  [FAIL] deep.md R3: escalated なのに deep.md に R3 が無い
  [FAIL] deep.md R4: escalated なのに deep.md に R4 が無い
  [FAIL] deep.md R6: escalated なのに deep.md に R6 が無い
  [FAIL] deep.md R7: escalated なのに deep.md に R7 が無い
triage: FAIL (6 件)                                                       rc=1
```

- `check_scenarios` の 46 件の FAIL は、実装の前なので想定どおり。spec の新しい 46 本の名前と tasks.md の `Scenario: …` の名前を突き合わせ、
  **46 本すべてが tasks のどこかで一字一句同じ名前で名指しされている**ことを確かめた（tasks にあって spec に無い名前は書式の例 `<名前>` だけ）。
  「63 本（record-envelope 54・data-durability 6・collection-coverage 3）、うち新しく足したのは 46 本、MODIFIED で写した既存分 17 本」も数えて合っていた。
  MODIFIED の 3 要件は正典の本文と Scenario を 1 つも落としていない（正典との差分は足した行だけ）。
- `review_triage` の FAIL は指摘として扱う（R1）。

---

## R1. deep.md に deep-review の出どころ（R1・R2・R3・R4・R6・R7）が無く、review_triage が落ちる。tasks 7.1 の検証もこのままでは通らない
- 成果物: openspec/changes/st41-event-end-time/deep.md
- 根拠: `python3 scripts/review_triage.py . st41-event-end-time` が rc=1（上の 6 件）。review/deep.md は R1→Q7、R2→Q1、R3→Q7 の族、R4→note (3)、R6→note (6)、R7→Q1 の族を `escalated` にしているが、
  deep.md には `出どころ: review/deep.md R<n>` の行が 1 つも無い（同じ流れの ch02 の deep.md:9 / 15 / 21 / 28 は持っている）。R4 と R6 は問いではなく「聞かずに決めること」に写したので、deep.md には書く場所すら無い。
  tasks.md:148 の 7.1 は検証に `python3 scripts/review_triage.py . st41-event-end-time` rc=0 を置いているので、下流がこの段で必ず落ちる
- kind: technical
- 提案: deep.md の Q1 / Q7 に `出どころ: review/deep.md R2・R7` / `R1・R3` を書き、「聞かずに決めること (3) / (6)」の節を 1 つ足して `R4` / `R6` を書く。書いた後で triage が rc=0 になることを確かめる
- 処置: fixed deep.md —— Q1 / Q7 に `出どころ: review/deep.md R2・R7` / `R1・R3` を書き、「聞かずに決めたこと」の C1〜C6 の節を足して R4 / R5 / R6 / R8 を出どころに書いた（design の D の見出しにも C 番号を添えた）

## R2. Q1 の推奨の選択肢の説明（本人が見る文）は「書き出しは逆順の終わりを点として数える」と言うが、spec と design は点にせず次の順へ進める
- 成果物: openspec/changes/st41-event-end-time/deep-questions.json（と specs/data-durability/spec.md）
- 根拠: deep-questions.json:43 の Q1 の推奨の detail は「期間の重なり・書き出しは逆順の終わりを使わず、点として数える」。
  specs/data-durability/spec.md:6 は「列の終わりが記録の時刻と同じか前なら、列の終わりを持たないものとして内容の終わりの項目から順に読む」、:13 は「点にせず次の順へ進む…除き過ぎの側」、
  design.md:71-72（D4）も「点に縮めるより除き過ぎの側」。次の順へ進めば (b) 長さ・(c) 次の記録までの幅になりうるので、点とは除く範囲が違う。
  本人はこの説明を読んで「推奨のとおり」と答えるので、合意した中身と仕様が食い違ったまま合意が記録される
- kind: premise
- 提案: Q1 の detail を spec と D4 に合わせて「期間で引くときは始まりの点として扱う。書き出しは逆順の終わりを使わず、内容の終わり・長さ・次の記録の順に幅を読む（除き過ぎの側）」に直してから本人に見せる
- 処置: escalated —— Q1（第 1 回の問い。まだ本人に渡していない）。本人が見る Q1 の推奨の選択肢の説明を spec に合わせて直し（期間では始まりの点、書き出しは内容の終わり→長さ→次の記録の順。除き過ぎの側）、deep.md の Q1 の出どころに R2 を書いた。本人は直した文で Q1 に答える

## R3. 推奨の D6 / D7 では、更新時刻を持たない進行中の版を送り手が送り直すと（応答を取り落とした正常な再送）、入った終わりが消えて前の版が積む。Scenario にも申し送りにも無い
- 成果物: openspec/changes/st41-event-end-time/specs/record-envelope/spec.md・design.md
- 根拠: design.md:103 の判定 `(stored.content_hash != hash || stored.end_time != req.end_time) && external_id.is_some()` は、保存済み Some・届いた None でも真になる。
  spec.md:89「届いた版が終わりを持たないとき、更新後の記録の終わりを空にする」と、正典の「更新時刻を持たない記録を、届いた順で適用する」（spec.md:93）が重なると、
  外部の識別子 `sleep-1` で終わりなしの到着 A（更新時刻なし）→ 終わりつき B → **A の再送**、の順で終わりが空に戻り、前の版が 1 件増える。
  `lib.rs:1580-1584` は更新時刻が両方あるときしか古い到着を止めない。A の再送は `lib.rs:1250` のコメントが「応答を取り落とした端末が再送する正常な経路」と書いている経路で、`id_reused` にもならない。
  いまは同じ再送が内容の鍵一致で `advance_watermark`（`lib.rs:1540-1558`。更新時刻なしなら何もしない）に落ちて無害だった —— ST41 で、正常な再送が記録を巻き戻す経路に変わる。
  note (4) の「更新で終わりが無くなった版が来たら、その版のとおり終わりを空にする」は本人に見せるが、再送でもそうなることは書いていない。`docs/handoff/ST39.md` の R3 の節も更新時刻に触れていない
- kind: daily
- 提案: 次のどちらかを決めて Scenario を 1 本置く。(a) 内容の鍵が同じで終わりだけが「有る→無い」の到着は更新にしない（終わりを消すのは内容の違う版か更新時刻の新しい版だけ）、
  (b) いまの形のまま、`docs/handoff/ST39.md` に「進行中を送る衛星は `source_updated_at` を必ず付ける」を書き、Scenario「更新時刻の無い、終わりを持たない版の再送で終わりが消える」を置いて振る舞いを固定する。
  どちらも失うものの側が違う（(a) は送り手が本当に終わりを取り消したいとき、(b) は再送で終わりが消える）ので、note (4) に書き足して本人に見せる
- 処置: fixed D18 仮 —— (a) を採った。内容が同じで終わりを持たない到着は更新にせず、保存済みの終わりを消さない（失わない側。費用は送り手が内容を変えずに終わりを取り消せないこと）。spec の更新の要件に 1 行と Scenario「終わりを持たない同じ版の再送で、入った終わりは消えない」、note (4) と C4・design D6 / D7 を直した。反転条件は D18

## R4. 「終わりを持たない版が届くと…」の Scenario は内容も違う版で書かれていて、新しく足した「終わりだけが無くなる」経路を通らない
- 成果物: openspec/changes/st41-event-end-time/specs/record-envelope/spec.md
- 根拠: spec.md:145-149 の WHEN は「内容が異なり終わりを持たない版を送る」。内容が違えばいまのコードでも `lib.rs:1371` の `stored.content_hash != hash` で更新に入るので、
  D7 で足す `stored.end_time != req.end_time`（design.md:103）の「有る→無い」の枝は、この Scenario が通っても試されない。外して実装しても緑のまま
- kind: technical
- 提案: WHEN を「原文と出来事の時刻が同じで終わりを持たない版」にした Scenario を足す（R3 の決め方に合わせて、終わりが消えるか残るかを THEN に書く）。いまの Scenario は内容の違う版として残してよい
- 処置: fixed specs/record-envelope/spec.md —— D18 で「終わりを空にする」は内容が違う版のときだけになったので、Scenario の WHEN を「原文が異なり終わりを持たない版」と明示し、終わりだけが無くなる到着は新しい Scenario（R3）で試す

## R5. 終わりの時刻の比べ方の精度が決まっていない。マイクロ秒より細かい桁を送る送り手では、同じ版の再送のたびに前の版が積む
- 成果物: openspec/changes/st41-event-end-time/design.md（D7）・specs/record-envelope/spec.md
- 根拠: design.md:103 は `stored.end_time != req.end_time` を比べる。保存は `timestamptz`（µs。design.md:39「失う桁は始まりと同じ。µs 未満」）で、届いた値は `DateTime<Utc>`（ナノ秒まで持つ）。
  始まりの側は鍵を `timestamp_micros()` で作る（ingest.rs:190）ので再送が一致するが、終わりは鍵に入らず直に比べるので、`…T07:00:00.123456789Z` を送る送り手は毎回「終わりが違う」になり、
  更新時刻が無ければ毎回前の版を書く（正典の「往復するたびに履歴が無限に積む」と同じ形）。spec.md:157-160「終わりも内容も同じ版は履歴を増やさない」は秒の値でしか試されない
- kind: technical
- 提案: D7 に「終わりの比べはマイクロ秒に切り詰めた値で行う（鍵の始まりと同じ）」と書き、spec.md:157 の Scenario の WHEN に µs 未満の桁を持つ終わりの時刻を入れる（または同じ形の Scenario を 1 本足す）
- 処置: fixed D7 —— 届いた終わりを受けた直後にマイクロ秒へ切り捨て、その値で比べて格納する。spec に 1 行と Scenario 2 本（格納の切り捨て・µs 未満だけの違いは更新にならない）

## R6. 「終わりのずれと TZ だけが違う到着は更新にしない」は観測できる振る舞いなのに design（D7）にしか無い
- 成果物: openspec/changes/st41-event-end-time/design.md（D7）→ specs/record-envelope/spec.md
- 根拠: design.md:104「比べるのは `end_time` だけ（終わりのずれと TZ だけが違う到着は…更新にしない）」。spec.md:85 は「内容が異なるか終わりの時刻が異なる」と書くだけで、ずれ・TZ だけの違いをどう扱うかの文も Scenario も無い。
  `openspec archive` は design を正典へ写さないので、この決定は正典から落ちる。帰結として、外部の識別子のある記録の終わりの TZ の誤りは取り込み口から直せない
  （`advance_watermark` は `source_updated_at` しか書かない。lib.rs:1551）—— 本表の錠の外に置いた（spec.md:176）理由「設定の誤りを直す余地」とは別の話で、どちらの経路で直すのかが spec から読めない
- kind: technical
- 提案: 外部サービス由来の更新の要件に「終わりの時刻が同じで、終わりのずれかタイムゾーン識別子だけが違う記録は、更新として扱わない（記録も履歴も変えない）」の 1 行と Scenario を足す。
  直す経路が DB の直の書き換えだけであることを、錠の外に置く文（spec.md:176）の注に添える
- 処置: fixed specs/record-envelope/spec.md —— 更新の要件に「終わりのずれか TZ だけが異なる記録を更新として扱わない」と Scenario を足した

## R7. data-durability を ADDED にしたので、archive の後の正典に「幅の順」が 2 つ並び、文の上で食い違う
- 成果物: openspec/changes/st41-event-end-time/specs/data-durability/spec.md・proposal.md
- 根拠: 正典 openspec/specs/data-durability/spec.md:211-215（と CH02 の MODIFIED の 8-12 行目）は「記録の時刻の幅は次の順で決め… (a) 内容が終わりの時刻を持てば… (c) どちらも持たず…次の記録まで」と言い切る。
  ST41 の ADDED（specs/data-durability/spec.md:5-6）は「内容の終わりの項目より先に列の終わり」と言う。列にだけ終わりを持つ記録について、前者は (c)、後者は列の終わり、と読める。
  ADDED にした理由（proposal.md:40-41。CH02 との上書きを避ける）は筋が通るが、食い違いを後で畳む手当てがどこにも無い（tasks・handoff に無い）
- kind: conflict
- 提案: ADDED のままにし、tasks の 7.1 に「ST41 と CH02 のうち後で archive する側が、正典の (a) の前に列の終わりを名指しする MODIFIED を足す」を書き、`docs/handoff/CH02.md`（または ST41 の後続）に同じことを申し送る。
  あわせて ADDED の要件の文に「この順は『書き出しは本人が消した場面を出さない』の (a) より前に置く」と明記する
- 処置: fixed D17 仮 —— ADDED のまま並べ、ADDED の本文に「(a) の前に置く読み方で、(a)〜(d) の順は元の要件が持つ。次にその要件を MODIFIED する change が畳む」と書いた。反転条件: CH02 が archive 済みなら MODIFIED に切り替える

## R8. Q1 の族で「通す」と決めた兄弟（終わりが未来・幅が何年もある）が Scenario に無く、実装が上限を足しても spec が落ちない
- 成果物: openspec/changes/st41-event-end-time/specs/record-envelope/spec.md
- 根拠: deep-questions.json の Q1 の family は「終わりが未来 → 通す」「幅が何年もある（端末の時計の飛びなど）→ 通す（断ると失う）」を既定にして本人に見せる（review/deep.md R7 の処置）。
  spec の「記録は 2 つの時刻と地域を持つ」の Scenario（spec.md:33-81）は逆順・同じ時刻・TZ の値は固定しているが、未来の終わりと長い幅は 1 本も無い。
  上限を足す実装（たとえば 1 日を越える幅を断る）は、どの Scenario も落とさない
- kind: technical
- 提案: Scenario「終わりが何年も先の記録も、送られたまま残る」（例: 始まり 2026-10-01、終わり 2053-10-01）を 1 本足す。期間で引くときに毎回出ることは R7（review/deep.md）が受け入れた側なので、Scenario にはしなくてよい
- 処置: fixed specs/record-envelope/spec.md —— Scenario「終わりが未来の記録も、幅が何年もある記録も受け付ける」と要件の本文の 1 行を足した

## R9. Q4 の推奨「『その日に何があったか』は重なった日すべてに出す」は、1 日の並びには写らず、決定の行き先も無い
- 成果物: openspec/changes/st41-event-end-time/design.md（D12）
- 根拠: deep.md の Q4 の推奨は「件数は始まりの日に 1 件、『その日に何があったか』は重なった日すべてに出す」で、Q4 の why は「稼働状況の格子や 1 日の並びの数え方」と言う。
  spec で重なった日すべてに出るのは `GET /events` の期間の読み出し（record-envelope spec.md:338-341）だけ。design.md:163-164（D12）は 1 日の並びを「衛星のソースを許可リストに足す Story」に回すが、
  その Story も申し送り（`docs/handoff/`）も名指ししていない。design は正典に写らないので、1 日の並びで日をまたぐ出来事をどう出すかの決定は archive で消える
- kind: defer
- 提案: 本人に見せる Q4 の推奨の文を「期間で引けば重なった日すべてに出る（1 日の並びはこの Story では変えない）」に直す。
  あわせて、衛星の記録を 1 日の並びに出す Story（ST39 か ST43）の `docs/handoff/` に「重なった日すべてに出し、幅は列→内容の鍵の順で読む（`Span::EndKey`）」を申し送る
- 処置: deferred ST39 —— 1 日の並びに衛星のソースを足すときの読み方（列→内容の鍵・重なった日すべて）を `docs/handoff/ST39.md` に書き、design D12 に宛先を書いた

## R10. design にある観測可能な決定（D1 の DB の CHECK・D16 の件数の上限なし・D9 の 400 の本文）が spec に無い
- 成果物: openspec/changes/st41-event-end-time/design.md → specs/record-envelope/spec.md
- 根拠: design.md:41-44（D1）の 2 つの CHECK は「取り込み口の外の書き込みへの止め」で、tasks.md:48 の `st41_gate_check_rejects_partial_end_in_db` が「印なし」で試すだけ。
  正典には同じ形の「取り込み口を通さない操作にも同じ制限が掛かる」がある（spec.md:228-231）ので、終わりの 3 列の揃いと値の範囲も Scenario で固定できる。
  design.md:129 / 194-196（D16（仮））の「期間の求めに件数の上限を置かない」、design.md:123（D9）の「400 の本文は種別の名前だけ」も応答の形で、spec に無い
- kind: technical
- 提案: 「記録は 2 つの時刻と地域を持つ」に「取り込み口を通さずに終わりの 3 列の一部だけ・範囲外のずれ・空の TZ を書くと拒まれる」の Scenario を足し、1.2 の試験に印を付ける。
  D16 と D9 の 400 の本文は、残すなら「期間に重なる記録を引ける」の SHALL に 1 行ずつ（D16 は仮なので、反転条件とともに）
- 処置: fixed specs/record-envelope/spec.md —— D1 の CHECK は要件の 1 行と Scenario「取り込み口を通さない書き込みでも…」、D16 は「件数で切らずにすべて返す」、D9 の 400 の本文は「種別の名前だけ・受け取った値を載せない」と Scenario の AND を足した

## R11. 検証の手段が書かれていない言葉が 3 か所ある
- 成果物: openspec/changes/st41-event-end-time/specs/record-envelope/spec.md・specs/data-durability/spec.md
- 根拠:
  - record-envelope spec.md:81「結果と状態符号は、終わりの欄を足す前と同じである」—— 何と比べるかが無い（比べる元の応答は足した後には無い）。何を見るかは tasks.md:65（「結果の鍵の集合で見る」）にしか書かれていない
  - data-durability spec.md:9 / :43「値の無い記録では空の値」—— JSONL の行で `null` か `""` か欄が無いのかが決まらない（正典の data-durability にも「空の値」の定義は無い）
  - record-envelope spec.md:52-54 は「±1,440 を断る」と「±1,439 を格納する」の 2 つの主張を 1 本に束ねている（片方の境界だけ実装しても、もう片方を試さない試験で緑になりうる）
- kind: technical
- 提案: :81 を「結果の各要素の鍵の集合が `id` / `accepted` / `duplicate`…（いまの鍵）のままで、状態符号が 200 である」のように固定値で書く。
  「空の値」を「JSON の `null`」（またはいまの書き出しが使う形）と書く。:52-54 は受理側を別の Scenario に分ける
- 処置: fixed specs/record-envelope/spec.md —— 「足す前と同じ」を「受理を示し状態符号 200」に、書き出しの「空の値」を JSON の `null` に、±1,440 と ±1,439 を 2 本の Scenario に分けた

## R12. 要件の本文が言っていて Scenario が言っていないこと・試していない枝がある
- 成果物: openspec/changes/st41-event-end-time/specs/record-envelope/spec.md・specs/data-durability/spec.md
- 根拠:
  - record-envelope spec.md:301「期間に重なる…記録を出来事の時刻の順に返す」—— 期間を指定したときの並びを見る Scenario が無い（期間なしは :369-372 が見る）。D10 は 2 つの引き方を `UNION ALL` で合わせる（design.md:146）ので、並びが崩れやすい側
  - record-envelope spec.md:308「（`source` を指定すればそのソースのすべて）」—— 期間なしで `source` だけを指定する Scenario が無い
  - data-durability spec.md:6「列の終わりが記録の時刻と**同じか**前なら…内容の終わりから」—— Scenario（:25-28）は「前」だけで、「同じ」の枝を試さない（内容の終わりは `>=` で採る（`excluded.rs:246`）ので、列だけ `>` にする差が実装で落ちやすい）
- kind: technical
- 提案: 3 つそれぞれに Scenario を 1 本ずつ足す（期間つきで時刻の順・`source` だけ・列の終わりが始まりと同じ時刻）
- 処置: fixed specs/record-envelope/spec.md —— Scenario「期間で引いた記録は出来事の時刻の順に並ぶ」「期間を指定せずソースだけを指定すると…」と、data-durability に「列の終わりが記録の時刻と同じなら、内容の終わりから読む」を足した

## R13. INDEX の capability 表に ST41 は record-envelope にしか無いのに、specs は data-durability と collection-coverage にも足している。INDEX に理由が無い
- 成果物: openspec/changes/st41-event-end-time/proposal.md・docs/stories/INDEX.md
- 根拠: docs/stories/INDEX.md:84（collection-coverage）・:92（data-durability）に ST41 が無い。proposal.md:38 は「照合: 合っている。前倒し無し」と record-envelope についてだけ言い、
  data-durability（proposal.md:39-41）には INDEX との照合が無い。collection-coverage の理由（proposal.md:42-44）は proposal にだけあり、INDEX の注（:97 や :114 の書き方）には無い。
  ST41 の `satisfies` は FR-91 だけで、足した 2 要件の導出元は NFR-16 / FR-50（申し送り C7）と FR-33 / FR-79 / NFR-13（INDEX:108 の C34）
- kind: technical
- 提案: INDEX の表の 2 行に ST41 を足し、注に「ST41 は data-durability（申し送り C7: 書き出しが列の終わりを読む）と collection-coverage（C34: 日の区切りと件数の数え方を Scenario で固定）に足した」を 1 行書く。proposal.md:38 の照合を 3 つの capability すべてについて書く
- 処置: fixed proposal.md —— Capabilities の照合を 3 つの capability について書き直し、`docs/stories/INDEX.md` の capability 表の `collection-coverage` と `data-durability` に ST41 を足して訂正の節に理由を書いた。`check_chain.py` rc=0

## R14. 移行の CHECK の追加が 120 万行を錠を取ったまま読む時間を測っていない。「索引の 29.3 秒より軽い」は前提に無い
- 成果物: openspec/changes/st41-event-end-time/design.md（D15・Risks）・premises.md
- 根拠: design.md:184 は `ADD CONSTRAINT`（CHECK）で「全行を 1 回読む」と書き、design.md:204 は「索引の 29.3 秒より軽い」「作成時間の試験は置かない」と言うが、premises.md に CHECK の時間の行は無い（確かめたのは索引だけ）。
  `ALTER TABLE … ADD CONSTRAINT … CHECK` は検査の間 `ACCESS EXCLUSIVE` を持つので、起動時の移行の間 `core.event` は読みも書きも待つ（索引の作成は書き込みだけを待たせる）。
  既存の行は 3 列とも NULL なので、`NOT VALID` で足せば全行を読まずに新しい行にだけ効く
- kind: technical
- 分類の直し: レビューは premise としたが、呼び出し元が実物で測って確かさ 4 にし、決定がその前提に依らない形（`NOT VALID`）にしたので technical に直した（本人が決めることが残らない）
- 提案: CHECK を `NOT VALID` で足す（既存の行は NULL だけなので失うものは無い）か、premises.md に 120 万行での `ADD CONSTRAINT` の時間を確かさ 4 で足す
- 処置: fixed D15 —— 使い捨ての PostgreSQL 17・120 万行で測った: `NOT VALID` の CHECK は 1.4 ms、素の CHECK でも 357.7 ms（premises.md に確かさ 4 の行を足した）。CHECK を `NOT VALID` で足す形にし、Risks の「索引の 29.3 秒より軽い」を測った値に置き換えた。失うものは無く、本人に問う前提は残らない

## R15. tasks の検証に、コマンドと終了条件を持たない箇所がある
- 成果物: openspec/changes/st41-event-end-time/tasks.md
- 根拠:
  - tasks.md:66「いまの木で落ちることを確かめてから 2.2 に進む」—— 2.1 の 9 本のうち `st41_ingest_without_end_is_unchanged`（:65）は、いまの木でも通る（終わりの欄を足す前の振る舞いを固定する試験なので）。
    「全部落ちる」とも「どれが落ちる」とも書いていないので、この確かめは判定できない。6.1（:137）は落ちる本を名指ししている。5.1（:104-119）も「先に試験を書く」だけで、落ちることの確かめと落ちる本の名指しが無い
  - tasks.md:123「アサートは変えていないことを `git diff crates/server/src/day_records_tests.rs` が呼び方の行だけであることで確かめる」—— 人の目で読む判定で、終了コードが無い
  - tasks.md:145-148（7.1）—— CH02 が main に入ったかを `git log … | head -3` の読みで決め、検証は `review_triage` rc=0 で、申し送りを書いたか（書かなかったか）を確かめない（しかも triage はいま落ちる。R1）
- kind: technical
- 提案: 2.1 / 5.1 に「いまの木で落ちるのは N 本目と M 本目」を名指しする。5.2 は `git diff -U0 crates/server/src/day_records_tests.rs | grep -E '^[-+][^-+]' | grep -v 'events(' ` が空であること（rc=1）のような機械の判定にする。
  7.1 は CH02 の関数名など main にだけある印を `grep -q` で見て分岐し、書いた場合は `test -f docs/handoff/CH02.md && grep -q st41-event-end-time docs/handoff/CH02.md` を検証にする
- 処置: fixed tasks.md —— 2.1 / 3.2 / 5.1 / 6.1 に落ちる本を名指しして `CT … rc≠0` で確かめる形に、5.2 を `git diff -U0 … | grep` の空で、7.1 を `git log … || test -f docs/handoff/CH02.md` で判定する形にした

## R16. Q5 / Q6 の推奨（滞在と収集済みの記録は変えない）を確かめるタスクが無い
- 成果物: openspec/changes/st41-event-end-time/tasks.md
- 根拠: tasks.md:6 は「滞在（`stay_store.rs`）の書き方と、収集済みの行の内容は変えない（D13（仮）・D14（仮））」と言うが、検証は 2.3 の `git diff --quiet -- collector-android crates/collector-windows`（:72）だけで、
  `stay_store.rs` にも、移行の後に終わりを持つ行が 0 件であることにも検査が無い。D13 / D14 は仮で、本人の答えで反転しうる決定なので、推奨のままなら「変わっていない」ことを機械で残しておく必要がある
- kind: technical
- 提案: 7.2 に `git diff --quiet origin/main -- crates/server/src/stay_store.rs` rc=0 を足し、1.2 か 5.3 の試験で「移行を当てた直後に `end_time IS NOT NULL` の行が 0 件」を 1 本見る
- 処置: fixed tasks.md —— 7.1b を足した（`git diff --quiet` で滞在と収集アプリに触っていないこと、`st41_stays_and_collected_rows_have_no_end_column` で滞在と位置の記録の終わりが NULL のままであること）

## R17. 本人に見せる Q5 の context が、もう無い「聞かずに決める (7)」を指している
- 成果物: openspec/changes/st41-event-end-time/deep-questions.json
- 根拠: deep-questions.json:130 の Q5 の context は「読む側（書き出し・消した滞在の時間）は聞かずに決める (7) で列→中身の順に読む」。note は (1)〜(6) で、該当するのは (6)（review/deep.md R6 の処置で番号が詰まった）
- kind: technical
- 提案: 「(7)」を「(6)」に直す

---

## 観点ごとの確かめた範囲

- 観点 1（deep の決定が正典に写っているか）: Q1〜Q7 の推奨と note (1)〜(6) を 1 件ずつ specs / design / tasks と照らした。指摘は R2・R3・R8・R9。
  Q2（半開区間・点も入れる）・Q3（`GET /events` に `from` / `to` / `source`）・Q5 / Q6（D13 / D14（仮））・Q7（鍵を変えない・外部識別子つきだけ更新・削除済みの止め・固定値 `39d0…df1` は ingest.rs:269-274 の値と同じ入力で一致）・
  note (1)(2)(3)(5)(6) は写っていた。「要件へ戻すもの」は deep に無く、`docs/requirements.md` の改訂も要らない（FR-91 の本文は変わらない）
- 観点 2（検証可能か）: R4・R5・R11・R12
- 観点 3（置き場）: R6・R7・R10・R13。specs に関数名・crate 名・列名以外の実装の名前は無い（`end_time` などは取り込みと応答と書き出しの欄の名前で、契約の一部）
- 観点 4（tasks）: R1・R14・R15・R16。依存順（移行 → 取り込み → 更新 → 稼働 → 読み出し → 書き出し → 申し送り）は鍵や列に依るタスクより前に置かれていて問題なし。
  件数の下限は `export_` 約 129 本 + 新しい 6 本・`dedup_` 約 36 本 + 新しい 3 本をソースの `#[test]` から数え、130 本・30 本の下限を満たすことを確かめた。人間の確認待ちが要るタスクは無い（サーバだけで、実機・実データに依らない）
- 観点 5（Story との一致）: `check_chain.py` は rc=0（観点 8 の再生成との一致も通っている）。ST41.md の「完了の判定」2 つ（ずれと TZ ごと残る・期間に重なる出来事が引ける）は deep の推奨・specs と矛盾しない。
  `satisfies` に無い要件への手当ては R13
- 観点 6: `--draft` のため対象外
- 処置: fixed deep-questions.json —— Q5 の context の「(7)」を「(6)」に直した

