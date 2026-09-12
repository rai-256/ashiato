# ST07 上流成果物の独立レビュー（spec-review）

**やり方**: `deep.md` を読み、そこで本人が決めた 8 件を 1 件ずつ「効く先」の実物まで追った。
`specs/` / `design.md` / `tasks.md` / `docs/stories/ST07.md` / `docs/requirements.md` /
`docs/stories/INDEX.md` / `openspec/specs/` / `docs/handoff/` を突き合わせた。
**どのファイルも編集していない。**

## 機械の検査（先に走らせた。2026-09-12）

```
$ openspec validate st07-active-window --strict
Change 'st07-active-window' is valid                                   rc=0

$ python3 scripts/check_chain.py .
要件 116 件 / Story 36 本 / 扉 26 項
[ok] どれかの Story に拾われた要件: 114/116 件
chain: OK (0 件 / 未回収 0 件 / warn 0 件)                              rc=0

$ python3 scripts/check_scenarios.py . st07-active-window
[FAIL] 担保の無い Scenario: 23 件（desktop-collection の全部）
scenarios: FAIL (担保なし 23 件)                                       rc=1

$ python3 scripts/review_triage.py . st07-active-window
指摘 10 件 / 仮決め 2 件 / 要件へ戻すもの 5 件
triage: OK                                                             rc=0
```

`check_scenarios` の FAIL 23 件のうち **17 件は上流では原理的に満たせない**（test がまだ 1 本も
無い。`merge_gate.sh` も `lane=upstream` では見ない）。**残り 6 件は違う** —— 「人間の確認待ち」に
挙げてあるのに機械に届いていない。R3 を見てほしい。

---

## R1. Q3（感度は PERM-3 のまま）の決定が、specs / design / tasks のどこにも無い

- 成果物: openspec/changes/st07-active-window/specs/desktop-collection/spec.md
- 根拠: `deep.md:89-93` は Q3 の答えを「**PERM-3 のまま（外部 AI に出してよい）**。
  `core.event.sensitivity` の既定 1 のままでよく、登録簿にソースごとの既定を持たせる必要も無い」と
  書いている。**8 問のうち唯一、推奨（PERM-6 へ引き上げ）と違う答え**（`deep.md:182-185`）。
  ところが `grep -rn "感度\|sensitivity\|PERM-3" openspec/changes/st07-active-window/` の結果は
  `design.md:147`（Risks の地の文）と `proposal.md:46`（表）と `spec.md:228`（ログの理由）だけで、
  **Requirement も Scenario も D 番号も task も 0 件**。
  実装者が「題名と URL は私的だから」と `sensitivity=2` を付けて送っても、
  23 本の Scenario も 40 件の task も 1 つも落ちない。落ちたことは
  **成功条件 2 の QS-7 / QS-10（`docs/requirements.md:629`）が外部 AI で答えられなくなった時**に
  初めて判る。推奨と違う答えほど、機械の担保が要る（推奨側は実装者の直感と一致するので黙って戻る）。
- kind: technical
- 提案: `desktop-collection` に Requirement を 1 本足す ——「WHEN ウィンドウの記録が格納される
  THEN その記録の感度は『外部 AI に出してよい』である」。または最低限、design に D 番号
  （「記録に感度を明示せず、既定 1 に委ねる。ST07 は PERM-6 を広げない」）と、
  `tools/smoke.sh` で `c02-window` の行の感度が 1 であることを見る task を置く。
- 処置: fixed specs/desktop-collection/spec.md — Requirement「PC からの記録の感度は収集の既定に従う」を新設し、Scenario を 2 本（`既定の感度で格納される` / `収集側が厳しい側の感度を付けて送らない`）置いた。tasks に 8b.1（`cargo test sensitivity_uses_collection_default` と smoke で `sensitivity=1` を見る）を足し、Story の完了の判定にも「感度は『外部 AI に出してよい』のままである」を入れた。**推奨と違う答えほど機械の担保が要る**という指摘に同意する

## R2. 扉 #15 の決着状態が Q3 の答えと逆を言ったまま、正典に置かれている

- 成果物: docs/requirements.md
- 根拠: 扉 #15（`docs/requirements.md:813-816`）の決着状態は
  「4 段階。**既定は厳しい側に寄せる**。緩く始めて締めると、締める前に外部へ出た分が戻らない」。
  Q3 はまさにこの扉の幅の中で「ウィンドウ題名と URL は緩い側（PERM-3）」を選んでおり
  （`deep.md:85-89`）、`deep.md:96-97` は「**再設定リンクの鍵・共有リンクの鍵・検索語が
  既定で外部 AI に出る経路ができる**」と代償まで書いている。
  それなのに **`docs/requirements.md` には ★ 2026-09-12 の印が 1 つも入っていない** ——
  FR-12 / NFR-13 / FR-81 / FR-82 / FR-83 の 5 件には入っているのに、感度だけ入っていない
  （`deep.md:162-170` の「要件へ戻すもの」の表に感度の行が無い）。
  結果、正典を読む次の Story は「既定は厳しい側」だけを読み、ST07 が意図して緩い側に置いたことを
  知る手段が `openspec/changes/archive/` の中の `deep.md` しか無い。
- kind: technical
- 提案: **決定は蒸し返さない**（`deep.md:99` が明示的に禁じている）。記録だけを正典に入れる ——
  PERM-3 か 扉 #15 の決着状態に `★ 2026-09-12` で 1〜2 行。「ウィンドウ題名と URL は PERM-3 のまま
  （ST07 の深掘り Q3）。守りは FR-83 の除外と PERM-9 の 2 つだけ」。
- 処置: fixed deep.md — 決定は蒸し返さず、記録だけ正典に入れた。PERM-3 の本文に `★ 2026-09-12 補足`（ウィンドウ題名と URL もこの既定に従う。PERM-6 を広げない。本人が推奨と違う側を選んだ）、扉 #15 の決着状態にも `★ 2026-09-12 補足` と関与要件への FR-83 追加。`deep.md` の「要件へ戻すもの」の表にも PERM-3 の行を足した

## R3. 「人間の確認待ち」の 6 件が、3 本のスクリプトのどれにも届いていない

- 成果物: openspec/changes/st07-active-window/tasks.md
- 根拠: `tasks.md:139-146` は `- [ ] V1 Scenario: \`アプリを切り替えると 1 件増える\`（実機で…）`
  の形。既存の通る形は `openspec/changes/st02-collection-coverage/tasks.md:456` の
  **`- Scenario: 1 年ぶんが一目で読める`**（裸・注釈なし）。3 本とも外れる:
  - `check_scenarios.py` —— `human_waiting()` を実行すると 6 件とも
    `'\`アプリを切り替えると1件増える\`（実機で切り替えて件数を見る）'` として取れる。
    `norm()` は空白しか落とさないのでバッククォートと括弧が残り、spec の名前と一致しない。
    実測: `check_scenarios.py . st07-active-window` の出力に `[wait]` が **0 行**、
    6 件は `[FAIL] 担保の無い Scenario` の側に並んでいる
  - `verify_checklist.py:69,76` —— `^\s*-\s*\[ \]\s*(\d+\.\d+…)` は `V1` に当たらず、
    `^\s*-\s*Scenario:` も `- [ ] V1 Scenario:` に当たらない。**確認バッチの手順書から 6 件が丸ごと落ちる**
  - `verify_record.py:78` —— 貼り戻しの書き込み先が `^(\s*-\s*Scenario:\s*<名前>)\s*$` なので、
    本人が実機で確認しても `tasks.md` に印が付かない
  実機でしか確かめられない 6 件が、**機械には「テストを書き忘れた 6 件」に見える**。
  下流は `merge_gate` を通すために偽のテストを書くか、`check_scenarios` を無視することになる。
- kind: technical
- 提案: ST02 と同じ裸の形に直す —— `- Scenario: アプリを切り替えると 1 件増える`。
  やり方（実機で切り替えて件数を見る）は次の行の `>` 引用か別の箇条書きに置く。
  チェックボックスと V 番号は付けない（`merge_gate.sh:71` の `- [ ]` の走査にも掛かる）。
- 処置: fixed tasks.md — ST02 と同じ裸の形（`- Scenario: <名前>`）に直し、やり方は次行の `>` 引用に置いた。チェックボックスと V 番号は外した。`python3 scripts/check_scenarios.py . st07-active-window` の出力で `[wait]` が **0 行 → 6 行**になったことを確認済み

## R4. design にしか無い観測可能な振る舞いが 3 件ある（archive で正典から落ちる）

- 成果物: openspec/changes/st07-active-window/design.md
- 根拠: `openspec archive` は main specs しか更新しないので、`design.md` の記述は正典に残らない。
  次の 3 件は「報告する / 載せる / 出す」という**外から観測できる振る舞い**で、specs に属する:
  - **D4**（`design.md:66-74`）——「**UI Automation が応答すること**」を取得可否の条件に入れ、
    満たされない側を `blockers` に載せる。これは `deep.md:108-110` が Q4 の「効く先」として
    名指しした決定（「生存信号の『取得できる状態か』に UI Automation の応答が入る」）そのもの。
    ところが spec の Scenario は `spec.md:180-183`「**前景を読めない**状態は取得できないとして
    報告される」の 1 本だけで、**URL が読めない（= UI Automation が死んでいる）状態を覆っていない**。
    正典に無ければ、前景だけ読めていれば `capturable=true` を返す実装が spec 上は正しくなり、
    NFR-13 の訂正 (2)（`docs/requirements.md:561-` が名指しした「権限が剥がれたまま 365/365」）が
    URL について再発する
  - **D6**（`design.md:85-92`）—— C-02 も 1 時間ごとに時計のずれの**測定記録を出す**。
    `tasks.md:115` に task はあるが Requirement も Scenario も無いので、
    `check_scenarios.py` は消えても気づかない
  - **D9 後半**（`design.md:114`）——「最後の入力からの経過時間も**併せて載せる**」。
    閾値 5 分を後から引き直せるのはこの項目があるからで、`tasks.md:69-70` にしか無い
- kind: technical
- 提案: D4 は Scenario を 1 本足す（「URL を読み取れない状態も取得できないとして報告される」）。
  D6 / D9 後半は Requirement 1 本ずつか、少なくとも既存 Requirement の SHALL を 1 行増やして
  Scenario を付ける（`check_scenarios.py` の網に入れるため）。
- 処置: fixed specs/desktop-collection/spec.md — 3 件とも spec に上げた。D4 は送る側の SHALL に「URL を読み取る経路が応答すること」を足し、Scenario `URL を読めない状態も取得できないとして報告される` を新設。D6 は Requirement「PC 側の収集は時計のずれを測って残す」を新設（tasks 8b.2）。D9 後半は離席の Requirement に SHALL を 1 行足し、Scenario `閾値を後から引き直せる形で残る` を新設

## R5. ST07 の「完了の判定」が、ST07 の Non-Goals で明示的に捨てた結果を求めている

- 成果物: docs/stories/ST07.md
- 根拠: `docs/stories/ST07.md:70`（= `docs/stories/stories.json:169`）の完了の判定は
  「PC を 1 日閉じてから起動すると、その期間が『PC が止まっていた』として残り、**途絶にならない**」。
  一方 `design.md:19-21` の Non-Goals は「**1 年の格子が FR-82 の記録をどう読むか**
  （②『稼働・記録なし』にするか新しい状態を足すか）。`collection-coverage` の判断」と書き、
  `deep.md:64-66` も同じ（「ST07 では決めない」）。
  実物でも成り立たない —— `crates/server/src/coverage.rs:723-756` の `active_days` は
  `core.event` と `core.heartbeat` の**行がある日**しか拾わず、FR-82 の記録は
  起動した日に 1 行入るだけ。同 `:795-815` の `near` は `gap_days = 21600/86400 = 0.25` なので
  同じ日しか近傍にならず、**閉じていた中日は `DayState::Outage`（⑥途絶）のまま**。
  この判定は `verify_checklist.py:59-63` が確認バッチの問いに機械的に変換するので、
  **実機で必ず「通らなかった」と返ってくる**（ST07 の実装に落ち度が無くても）。
- kind: conflict
- 提案: `stories.json` の done から「途絶にならない」を落とし、ST07 の範囲で判定できる言葉にする
  （例:「その期間が『PC が止まっていた』として 1 件残る」）。
  格子の読み方は `docs/handoff/ST02.md` の申し送りに含めるか、ST14 / ST15 の完了の判定へ移す。
- 処置: fixed D13 仮 — `stories.json` の完了の判定から「途絶にならない」を落とし、「その期間が『PC が止まっていた』として **1 件残る**」に直した（`make_story.py` で再生成、`check_chain.py` rc=0）。判断そのものは design の **D13（仮）**に残し、反転条件を「ST02 の archive 後に格子の読み方が決まったら、『途絶にならない』を ST14 / ST15 の完了の判定として立て直す」とした。**確認バッチで必ず「通らなかった」と返る**という指摘が決め手

## R6. payload の形を凍結する task（2.1 / 2.2）が、項目を足す task（4.1 / 6.2）より前にある

- 成果物: openspec/changes/st07-active-window/tasks.md
- 根拠: 冪等キーは `logical_source` + `event_time` + **`raw` の文字列そのもの**から作られ
  （`design.md:33-37`。`crates/server/src/ingest.rs`）、`design.md:47` は
  「**D4・D9・D11 で載る項目が変わるので、この 3 つを先に決める**」と自分で書いている。
  ところが `tasks.md:42-45`（2.1）が列挙する項目は「アプリ名・実行ファイルのパス・プロセス名・
  ウィンドウ題名・URL・記録の種類・範囲の終わり」で、
  **D9 の「最後の入力からの経過時間」（`tasks.md:70`）と D11 の「除外した件数」（`tasks.md:90-91`）が
  入っていない**。その直後の `tasks.md:48-49`（2.2）が
  `cargo test payload_shape_is_pinned` で形を固定する。
  順に進めると、2.2 で固定した形を 4.1 と 6.2 が 2 回書き換えることになる ——
  **形が変われば同じ 1 件が別の鍵になる**ので、これは「後で直せばよい」種類の手戻りではない。
- kind: technical
- 提案: 2.1 の列挙に「最後の入力からの経過時間」と「除外した件数」を足す。
  または 2.2（形の凍結）を 4 章・6 章の後ろ（例: 6.4）へ動かし、2 章は契約の文書化だけにする。
- 処置: fixed 6.3 — 形を凍結する task を 2.2 から **6.3**（4 章・6 章の後）へ動かし、2.1 の列挙に「最後の入力からの経過時間」（D9）と「除外した件数」（D11）を足した。design D1 自身が「D4・D9・D11 で載る項目が変わる」と書いていたのに、順が逆だった

## R7. D2 の「列がまだ無ければ何もしない分岐」は、merge の順に依存する（design は逆を書いている）

- 成果物: openspec/changes/st07-active-window/design.md
- 根拠: `design.md:55-56` は「**ST03 の移行の当たる順に依存しない形**にする（列がまだ無ければ
  何もしない分岐を置く）」。実際には依存する。適用順は `crates/server/src/lib.rs:31` の
  `MIGRATIONS` 配列の並び順で、ST03 も ST07 も**末尾に足す**（`tasks.md:37`、
  `openspec/changes/st03-idempotent-ingest/tasks.md` 1.1）。
  ST07 が先に merge されると ST07 の移行が配列の先に入り、`external_id_kind` 列はまだ無いので
  **何もしない分岐が走って終わる**。その後に ST03 の移行が
  `external_id_kind text NOT NULL DEFAULT 'record'` を足し、続く UPDATE の
  `NOT IN (…外部ソース…)` は**省略記号のまま**（`review/deep.md:85-87` が既に指摘した箇所）。
  `c02-window` が含まれなければ `'record'` で確定し、**ST07 の記録が全件 400**。
  検査も素通りする —— `tasks.md:30-32`（1.1）の検証は「`none` を返す**か、列が無ければ**移行が
  rc=0 で通る」という論理和で、no-op を合格として数える。
- kind: technical
- 提案: 順に依存しない形にするなら、ST07 の移行が
  `ALTER TABLE core.source ADD COLUMN IF NOT EXISTS external_id_kind …`（ST03 と同じ DDL）から
  始めて必ず UPDATE まで走らせる。それが重いなら、**全移行を当てた後**に
  「`c02-window` の `external_id_kind` が `'record'` ではない」ことを見るテストを 1 本置く
  （列の有無で分岐させない）。**ST03 の tasks は触らない**（凍結）。
- 処置: fixed D2 — design の D2 を書き直した。列の有無で分岐させず、**ST03 と同じ DDL（`ADD COLUMN IF NOT EXISTS`）から始めて必ず UPDATE まで走らせる**形にする。tasks 1.1 の検証から「列が無ければ rc=0」の論理和を外し、**全移行を当てた後**に `'record'` でないことを見る 1.1b を足した。「順に依存しない」と書きながら依存していたのは、こちらの誤り

## R8. 実行できない / Scenario を落とせない検証コマンドが 2 件

- 成果物: openspec/changes/st07-active-window/tasks.md
- 根拠:
  - `tasks.md:99-100`（7.1）—— Scenario `想定間隔ごとに生存信号が届く` の検証が
    「`tools/smoke.sh` に手順を足して rc=0」。想定間隔は **21600 秒**
    （`migrations/202609111111_coverage_rebuild.sql:131`、`tasks.md:99` 自身が書いている）。
    smoke は数十秒で終わるので、**間隔を守らない実装でも必ず緑になる**。
    時計を差し替えられる単体テストでなければこの Scenario は落ちない
  - `tasks.md:122-123`（9.2）—— 検証が `grep -c "自動起動" ...` で、
    **パスが `...` のまま**。そのままでは実行できない
- kind: technical
- 提案: 7.1 は `cargo test heartbeat_interval_is_expected_gap`（注入した時計で間隔を進める）に
  置き換え、smoke は「1 件届くこと」だけを見る。9.2 は
  `grep -c "自動起動" crates/collector-windows/README.md` のようにパスを書き切る。
- 処置: fixed 7.1 — 7.1 の判定を `cargo test heartbeat_interval_is_expected_gap`（注入した時計で間隔を進める）にし、smoke は「1 件届くこと」だけを見る形にした。9.2 の `...` を `grep -c "自動起動" crates/collector-windows/README.md` に書き切った

## R9. 述語が弱くて、実装が間違っていても真になる Scenario が 2 本

- 成果物: openspec/changes/st07-active-window/specs/desktop-collection/spec.md
- 根拠:
  - `spec.md:67-70` —— 「題名が最小滞留より短い間隔で **10 回**変わる」→
    「増える記録は **10 件より少ない**」。**9 件でも真**になる。意図（`deep.md:137-138`:
    「1 本で 1 時間 3,600 件」を止める）に対して、この述語は 10 % しか間引かない実装を合格させる。
    `tasks.md:65` の `cargo test min_dwell` はこの Scenario の逐語を写せば同じ穴を持つ
  - `spec.md:185-188` —— 1 つの Scenario が「試行回数と成功回数を**持ち**」と
    「成功回数は試行回数を**超えない**」の 2 主張を AND で束ねている。
    片方（項目の存在）だけ通っても緑になる
- kind: technical
- 提案: 前者は「増える記録は 1 件以下」か「最小滞留を超えて前景にあった題名の数と一致する」に。
  後者は Scenario を 2 本に割る。
- 処置: fixed specs/desktop-collection/spec.md — 前者は「最後の題名だけが最小滞留を超えて前景にとどまる」を WHEN に足し、THEN を「増える記録は **1 件だけ**であり、その記録は最後の題名を持つ」にした。後者は Scenario を 2 本に割った（`試行回数と成功回数が載る` / `成功回数は試行回数を超えない`）

## R10. 扉 #14 の関与要件に FR-81 / FR-82 が入らなかったので、ST07.md が自分と食い違っている

- 成果物: docs/requirements.md
- 根拠: `docs/requirements.md:812` の扉 #14（稼働記録を別に持つか＝欠損の意味）の関与要件は
  `FR-33, FR-34, FR-9, FR-54, FR-35, FR-61, FR-80` で、2026-09-12 に新設した
  **FR-81 / FR-82 が入っていない**。FR-82 の本文自身は
  「扉 #14 が求める『データが無い日の意味』は…」で始まり（`docs/requirements.md:135-137`）、
  `spec.md:116-118` も `deep.md:41-43` も同じ扉を根拠にしている。
  結果 `make_story.py` が生成した `docs/stories/ST07.md:52` は
  「（satisfies する要件は、扉リストのどの項の関与要件にも現れない）」と印字し、
  同じファイルの `:61` は「**扉 #14 の区別**（PC を開かなかった日を、本物の故障と同じ形で残さない）」を
  壊してはいけないものに挙げている。**1 つの生成物の中で逆のことを言っている。**
  `check_chain.py` は扉 26 項を数えるだけなので落ちない（rc=0）。
- kind: technical
- 提案: 扉 #14 の関与要件に FR-81, FR-82 を足す（★ 2026-09-12 の印つき）。
  FR-83 は扉 #15（感度の既定）の関与要件にも当たるので、R2 の追記と同じ 1 回で入れられる。
- 処置: fixed deep.md — 扉 #14 の関与要件に FR-81 / FR-82 を足した（★ 2026-09-12 補足つき。PC では扉の担い手が両方とも電源とともに消えること、FR-81 は同じ扉の裏面であることを書いた）。再生成後の `ST07.md` は扉 #14 と #15 を逐語で載せるようになり、「どの項の関与要件にも現れない」と「扉 #14 の区別を壊さない」の食い違いが消えた

## R11. D11 の「除外は送る前に判定する」が spec に無い（出てしまえば戻らない）

- 成果物: openspec/changes/st07-active-window/specs/desktop-collection/spec.md
- 根拠: `design.md:135` は「除外の判定は**送る前**に行う —— **送ってから消すと、消す前に
  バックアップへ入る**」と、失われ方まで書いている。
  一方 spec（`spec.md:136-141, 148-151`）が言うのは「**記録**しない」「どの**記録**にも現れない」
  だけで、**取り込み口へ送らない**ことは 1 行も言っていない。
  受け取ってからサーバ側で落とす実装でも spec 上は真になるが、
  `core.event` は移行 0004 のトリガで凍結され（`design.md:33-34`）、
  データ耐久性のバックアップ（ST10 / ST30）は取り込み口の後段にある。
  Q5 の守りは「そもそも記録しない」ことだったはずで（`deep.md:125`:
  Q3 で緩い側に置いた結果「**除外が唯一の『そもそも記録しない』手段**になった」）、
  送ってしまえばその守りは成立しない。
- kind: technical
- loss: exported
- 提案: 除外の Requirement に SHALL を 1 行足す ——「THE SYSTEM SHALL 除外した対象の
  アプリ名・ウィンドウ題名・URL を、取り込み口へ送らない」。Scenario は
  「除外に登録したアプリを前景にしたとき、送信の本文にその題名と URL が現れない」。
- 処置: escalated — `loss: exported` なので**人間へ返した**。`deep.md` の末尾に R11 として積み、第 2 回の問い 1 問（`deep-questions-r2.json` / `docs/briefs/ST07-deep-r2.html`）にした。spec には先に**送らない側**（扉を開けたままにする側）の SHALL と Scenario `除外した本文は取り込み口へ送られない` を書いてあり、答えが逆なら直す。**問うのは守りの置き場（収集側かサーバ側か）で、除外するかどうかではない**

## R12. 完了の判定が、仮の値（D8 の 5 秒）を確定値として本人に見せる

- 成果物: docs/stories/ST07.md
- 根拠: `docs/stories/ST07.md:68`（= `stories.json:167`）は
  「題名だけが **5 秒未満**で変わり続けても記録は増えない」。
  5 秒は `design.md:102-108` の **D8（仮）**で、反転条件は「実測した年間の容量が NFR-5 の枠に対して
  大きく余る / 足りないと分かったら変える」。`tasks.md:23` も D8 を仮決めとして挙げている。
  下流が反転条件どおり 10 秒に変えると、`verify_checklist.py:59-63` が生成する確認バッチの問いは
  「5 秒未満で…」のまま本人に出る。要件本文（`docs/requirements.md:110-121`）は
  賢く数値を避けて「最小の滞留時間」とだけ書いているので、**Story だけが値を握っている**。
- kind: technical
- 提案: `stories.json` の done を「題名だけが最小の滞留時間より短く変わり続けても記録は増えない。
  アプリと URL の変化は必ず増える」に直す（値は design D8 が持つ）。
- 処置: fixed D8 — `stories.json` の完了の判定を「題名だけが**最小の滞留時間**より短く変わり続けても記録は増えない。アプリと URL の変化は必ず増える」に直した。値は design D8（仮）が持つ。要件本文が賢く数値を避けていたのに Story だけが握っていた、という指摘は正しい

---

## 観点ごとの結果

- **観点 1（deep の決定が正典に写っているか）** —— Q1 / Q4 / Q5 / Q6 / Q7 は
  対応する Requirement と Scenario が実在し、答えと同じことを言っていた
  （`spec.md:108-134` / `:34-52` / `:136-161` / `:54-80` / `:82-106`）。
  Q2 と Q8 は「要件へ戻す / 登録簿を変えない」で、`docs/requirements.md` の NFR-13 に
  `★ 2026-09-12 再々々訂正（4 回目）` が入り（`docs/requirements.md:598-`）、
  `c02-window` の `expected_gap_sec = 21600` は既に seed 済みで一致していた
  （`migrations/202609111111_coverage_rebuild.sql:131`）。
  **覆した当初案（PERM-6 にウィンドウと URL を足す）が成果物に残っていないことも確かめた**
  （`grep -rn "PERM-6" openspec/changes/st07-active-window/` は `deep.md` の 1 件のみ）。
  **Q3 だけが、決定も代償もどこにも写っていない** → R1 / R2。
  数値（5 秒・5 分・6 時間）はいずれも可逆と本人が確認した側なので design の（仮）に置くのが正しく、
  Scenario に数値が無いことは指摘にしない（ただし Story が握っている件は R12）。
- **観点 2（Scenario が検証可能か）** —— 「適切に」「正しく」「そのまま」の類は 0 件。
  `そのまま` は `spec.md:34-52` にあるが、直後に「クエリ文字列もフラグメントも落とさずに」
  「`https://` は補われていない」と測り方が書いてあるので検証可能。弱いのは 2 本 → R9。
- **観点 3（置き場の誤り）** —— D4 / D6 / D9 後半 → R4。D11 の一部 → R11。
  `specs/` に実装の名前（関数名・crate 名・列名）は入っていなかった
  （`grep -n '\`' spec.md` の 5 件は `https://` の literal と `collection-coverage` と
  `docs/collector-contract.md` のみ）。
- **観点 4（tasks が検証を持つか）** —— 40 件すべてにコマンドがある。落ちるのは 2 件 → R8。
  依存順の乱れが 1 件 → R6。
  **Scenario の覆いは完全**: spec の 23 本すべてが `tasks.md` に名前で挙がっており、
  spec に無い名前を tasks が挙げている例も 0 件（突合を機械で確認した）。
  ただし「人間の確認待ち」の 6 件は書式で機械に届いていない → R3。
- **観点 5（Story が要件の現在の本文と一致しているか）** —— `check_chain.py` の観点 8 は rc=0。
  FR-12 / FR-81 / FR-82 / FR-83 の逐語引用 4 ブロックを `docs/requirements.md` と
  1 行ずつ突き合わせ、**すべて一致**した。価値と satisfies にも矛盾は無い。
  食い違うのは「完了の判定」の 2 行 → R5 / R12。
- **観点 6（capability の境界）** —— **差分なし。**
  `proposal.md:53-68` の Capabilities（New: `desktop-collection` のみ / Modified: 無し）は
  `docs/stories/INDEX.md:69`（`desktop-collection` = ST07, ST08）と一致し、
  `specs/` のディレクトリも `desktop-collection` 1 つだけ。前倒しは無い。
  `collection-coverage` と `record-envelope` への delta は 0 件
  （`ls openspec/changes/st07-active-window/specs/` で確認）。
  `spec.md:190-198`（識別子を持たない記録）は取り込み口の振る舞いに触れるが、
  `openspec/specs/record-envelope/`（ST03 の delta `:197-205`）が定める
  「登録簿の宣言に従う」の**内側**にあり、矛盾はしていない。
- **観点 7（走っている Story への差し戻し）** —— **差分なし。**
  `tasks.md:8` が「`collection-coverage` と `record-envelope` には触らない」と明示し、
  ST02 の `tasks.md` を変えさせる記述は 0 件。NFR-13 の実装は
  `docs/handoff/ST02.md` に `docs/handoff/README.md` の書式どおり書かれており
  （change 名 `st07-active-window` と `R10` を含む）、`review_triage.py` は rc=0。
  ST03 についても `design.md:58-59` が規則 2(i) を引いて自分の change で直している。
  **R7 は差し戻しではなく、同じ 1 行（`core.source` の `c02-window`）を 2 つの移行が触ることによる
  順序の危険**で、処置も ST07 の側で閉じられる。
