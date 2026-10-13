# spec-review — st21-place-registry

対象: `proposal.md` / `specs/personal-entities/spec.md` / `design.md` / `tasks.md` / `docs/stories/ST21.md` /
`docs/stories/INDEX.md` の「訂正（2026-10-01、ST21 の上流工程）」/ `docs/handoff/ST23.md` の st21 の項 / `docs/handoff/ST25.md`。
レビューした者は change を書いた者ではない（成果物どうしの整合だけを見た）。

## 機械の検査（2026-10-01）

| コマンド | 結果 |
|---|---|
| `openspec validate st21-place-registry --strict` | `Change 'st21-place-registry' is valid` |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（観点 8 の再生成との一致を含む） |
| `python3 scripts/check_scenarios.py . st21-place-registry` | `FAIL (担保なし 128 件)`。128 件はすべてこの change の Scenario（上流の段なので実装前で当然）。**担保ありと出た 11 件のうち 1 件は名前の衝突による偽の担保**（R1） |
| `python3 scripts/review_triage.py . st21-place-registry` | `triage: OK`（`review/deep.md` の 9 件。この `review/spec.md` を足す前） |

補足の確かめ: proposal の「MODIFIED にしない理由」は事実だった。正典を写した scratch で MODIFIED に既存の Scenario「人物と場所のタブは無い」を落とした版を作ると、
`openspec validate --strict` が `MODIFIED ... omits scenario(s) the current spec still has: "人物と場所のタブは無い"` で落ちる（実測）。REMOVED + ADDED の判断は指摘しない。

---

## R1. Scenario「届かなかったとき入力が残り届かなかったと出る」が ST19 の Scenario と同名で、ST19 の印が偽の担保になる
- 成果物: openspec/changes/st21-place-registry/specs/personal-entities/spec.md
- 根拠: spec.md:734 と正典 `openspec/specs/personal-entities/spec.md:636` が同じ名前。archive 後も ST19 の Requirement「個人属性の画面から主張を書く」は残るので、同じ spec の中に同名が 2 つになる。
  `check_scenarios.py` は名前で突き合わせるので、いま既に `web/src/__tests__/master-form.test.tsx:311`（個人属性のフォーム）の印で「担保あり」と出ている（上の 11 件の 1 つ）。
  場所のフォームで届かなかったときの試験が 0 本でも FAIL にならない。tasks.md:19 の「10 本は ST19 の印がそのまま当たる」も、実際は 11 本当たっている
- kind: technical
- 提案: 場所の側を「場所を足すで届かなかったとき入力が残り届かなかったと出る」など一意な名前に変え、tasks 10.2 の Scenario 名も合わせる。tasks に「この change の Scenario 名が正典の他の Scenario と重ならない」検査を 1 行足す
- 処置: fixed specs/personal-entities/spec.md — 場所の側を「場所の登録が届かなかったとき入力が残り届かなかったと出る」に改名し、tasks 10.2 も合わせた。正典との名前の重なりが design D20 の 10 本だけであることを確かめる検査を tasks 11.2 に足した（いまの重なりは 10 本）

## R2. design D13 の画面の文と D1 の `place_id_taken` の扱いが、spec の「理由の種別ごとに異なる文」「押し直しは同じ識別子」と矛盾する（振る舞いが design にだけある）
- 成果物: openspec/changes/st21-place-registry/design.md
- 根拠: spec.md:678 / spec.md:754 は「理由の種別ごとに異なる文で」と言うが、design.md:217 は `unknown_place` / `invalid_coord_supersedes` / `invalid_coord_change` の 3 種別に同じ文を当てている。
  spec.md:675 は「入力を変えずに押し直したとき、同じ器の識別子と同じ原文を送る」と言うが、design.md:218・222 は `place_id_taken` のときだけ押し直しで器の識別子を作り直す。
  `POST /places` の 200 / 400 `place_id_taken` / 401（design.md:44-45・249）は応答の形と状態符号の意味で、spec.md:7・26-29 は「受け付けない」としか言わない。archive では design は正典に入らない
- kind: conflict
- 提案: spec の側を design に合わせて書き直す（「同じ文を出す種別の組」と「器の識別子が別の利用者に取られていたときだけ識別子を作り直す」を Requirement と Scenario にし、`place_id_taken` と状態符号を spec に置く）か、design を spec に合わせて 3 種別に別の文を当てる
- 処置: fixed D13 仮 — 3 種別（`unknown_place` / `invalid_coord_supersedes` / `invalid_coord_change`）に別の文を当て、`invalid_coordinate` の文も足した。`place_id_taken` を spec の器の Requirement（理由の種別）と、場所を足す Requirement（その後の押し直しだけ識別子と原文を組み直す）に移し、Scenario「器の識別子が取られていたら押し直しで識別子を作り直す」を足した。D13 に（仮）と反転条件

## R3. 「場所の記録を積む」の Scenario が「読み出した」で確かめると言うが、design D15 の読み出しの形にはその欄が無い
- 成果物: openspec/changes/st21-place-registry/specs/personal-entities/spec.md
- 根拠: spec.md:80-83「読み出した名前の記録は、書いた日時と D-01 に入った時刻を別々の欄に持つ」—— design.md:240-245 の `GET /places` は `previous_names[].written_at` だけで D-01 に入った時刻を返さず、いまの名前は文字列だけ。
  spec.md:65-68「読み出した座標の記録は 3 件で、それぞれ … のどれかを持ち、『間違いを直す』の記録は直した座標の記録の識別子を持つ」—— D15 のいまの座標 `coord` は `change` を持たず、直す先は直した側ではなく直された側の `fixed_by` にある。
  spec.md:70-73「読み出したその記録の『いつから』は … 年月」—— いつから 2026-04 の移転はいまの座標になる（spec.md:395-396）が、`coord` は `valid_from` を持たない。
  tasks.md:99-107 はこの 3 本を `GET /places` の試験（`CT place_view_endpoint`）に置いている。また「別々の欄に持つ」は `core.event` の列が別である限り常に真で、値を確かめていない
- kind: conflict
- 提案: 3 本の WHEN / THEN を「格納された記録」（DB の行）で確かめる形に直すか、D15 のいまの座標に `change` / `valid_from` / `supersedes` と各記録の D-01 に入った時刻を足す。2 つの時刻の Scenario は「送った書いた日時と一致し、D-01 に入った時刻はそれと違う値」まで言う
- 処置: fixed D15 仮 — `GET /places` の各記録に記録の識別子・書いた日時・D-01 に入った時刻、いまの座標に変え方・「いつから」・直す先を足し、spec の Requirement に返す欄を書いた。Scenario を「座標の記録は変え方を持つ」「直す記録は直した座標の記録を指す」に分け、2 つの時刻の Scenario は送った値と一致するところまで言う形にした。D15 に（仮）と反転条件

## R4. 未来の移転の座標を「前の座標とは別に」返すと言う Scenario が、Requirement と design D15 と食い違う。画面の「予定」は design にだけある
- 成果物: openspec/changes/st21-place-registry/specs/personal-entities/spec.md
- 根拠: spec.md:398-401（THEN「新しい座標はいつからとともに前の座標とは別に返る」）。spec.md:338 の Requirement は前の座標を「いまの座標の版以外の座標の記録」とする（未来の版も入る）。
  design.md:126・244-245 は未来の版を `previous_coords` に `state: "upcoming"` で入れる（同じ一覧）。画面の「予定（YYYY-MM から）」の文字（design.md:197）は spec.md:611 に無く、spec は「直したものか移る前のものか」の 2 つしか言わない
- kind: conflict
- 提案: 未来の移転を前の座標の一覧に入れるか別に返すかを決め、Requirement と Scenario を揃える。画面の「予定」の文字と、それを「前の名前・座標 N」の N に数えるかを spec の Requirement と Scenario に移す
- 処置: fixed D7 仮 — 未来の移転は前の座標の一覧に「予定」として入れる（別に返さない）に揃え、Requirement・Scenario・design D7 / D15 を合わせた。画面の「予定」の文字と N に数えることを spec の Requirement に移し、Scenario「予定の移転は予定の文字で出る」を足した

## R5. 「初めての座標」「移った」の検査が、削除の印・消去を数えるかを design D4 にだけ書いている（受け付ける / 断るが design にある）
- 成果物: openspec/changes/st21-place-registry/design.md
- 根拠: design.md:101「`first` はその場所に座標の記録が 1 件でもあれば断る（削除の印・消去を問わない）。`move` は 1 件も無ければ断る」。spec.md:127 は「座標の記録を持つ」としか言わず、消した記録を数えるかで受理と拒否が変わる。
  ST23 が座標の記録を全部消せるようになると、その場所に `first` を書けるかどうかがこの一文で決まる。
  あわせて、spec.md:121（`malformed_place_record`「必須の欄が欠ける」）・spec.md:127（「直す座標の記録を持たない」）・spec.md:129（「直す座標の記録が無い」）の当たる範囲が言葉の上で重なり、どれを返すかは design D4（仮）の検査の順にしか書いていない
- kind: technical
- 提案: spec に「削除の印の付いた座標の記録・本文を消去した座標の記録も『座標の記録を持つ』に数える」を Requirement と Scenario（消したあとに `first` を送ると `invalid_coord_change`）で足す。表の 3 行は「`supersedes` が null」「`supersedes` が指す記録が無い」のように欄の値で書き分ける
- 処置: fixed specs/personal-entities/spec.md — 「座標の記録を持つ」に削除の印・消去も数えることを Requirement に書き、Scenario「座標をすべて消した場所にも初めての座標は書けない」を足した（tasks 3.1）。表の 3 行を欄の値（欠ける / 空 / 指す記録が無い）で書き分けた

## R6. 名前の無い居た所をまとめる 100 m が Scenario で固定されていない
- 成果物: openspec/changes/st21-place-registry/specs/personal-entities/spec.md
- 根拠: deep.md:136（C9「滞在の代表点を 100 m でまとめた派生」）/ spec.md:555。Scenario は互いに 30 m（spec.md:565）と 500 m（spec.md:570）だけで、まとめる半径を 50 m にしても 400 m にしても両方通る。
  また spec.md:555「代表点が 100 m 以内に集まるものごとに」は、design.md:172-173 の決め方（始まりの順に、平均の中心から 100 m 以内の最初のまとまりへ足す）と読み方が 1 つに決まらない（鎖状に 90 m ずつ並んだ滞在が 1 つになるか割れるか）
- kind: technical
- 提案: 境目の Scenario を足す（中心から 90 m は同じまとまり・110 m は別のまとまり）。まとめ方（何からの 100 m か）を Requirement の本文に書く。tasks 6.1 の `CANDIDATE_RADIUS_M` の固定はそのまま
- 処置: fixed specs/personal-entities/spec.md — まとめ方（始まりの順に、平均の中心から 100 m 以内の最初のまとまりへ）を Requirement に書き、境目の Scenario 2 本（90 m は同じ・110 m は別）を足した（tasks 6.1）

## R7. Requirement の本文だけが言っていて Scenario が言っていない振る舞いがある
- 成果物: openspec/changes/st21-place-registry/specs/personal-entities/spec.md
- 根拠:
  - spec.md:609「当たった滞在を持たない場所のカードに、まだ居たことが無いことを出す」—— Scenario 無し（design.md:194 は文言まで決めている）
  - spec.md:611「直したものか移る前のものか」—— Scenario は「移った」だけ（spec.md:641-644）。「直した」の文字は確かめていない
  - spec.md:676 / spec.md:753「送っている間は押した操作を押せなくする」—— どちらのフォームにも Scenario 無し
  - spec.md:753-754 の「変える」側の押し直しで同じ原文・届かなかったときの別の文 —— Scenario は「足す」側だけ（spec.md:724-737）
  - spec.md:677 / spec.md:754「受理されたときフォームを閉じる」—— Scenario は「読み直した後」だけで、閉じたことを見ていない
  - 「座標を変える」を開いたとき名前の無い居た所が 0 件の状態が spec にも design にも無い（spec.md:670 は「場所を足す」の側だけ）
- kind: technical
- 提案: 各条項に Scenario を 1 本ずつ足し、tasks 10.1〜10.3 に割り当てる。0 件の「座標を変える」は「居た所がまだ無い」と送る操作が押せないことを決めて書く
- 処置: fixed specs/personal-entities/spec.md — Scenario を 9 本足した（まだ居たことが無い / 直したの文字 / 登録を送っている間 / 登録でフォームが閉じる / 居た所が 0 件の座標を変える（送れない）/ 変えるを送っている間 / 変えるの押し直し / 変えるでフォームが閉じる / 変えるが届かなかった）。tasks 10.1〜10.3 に割り当てた

## R8. 前提を置かない WHEN で空のまま真になる Scenario と、1 本に 2 つの主張を束ねた Scenario
- 成果物: openspec/changes/st21-place-registry/specs/personal-entities/spec.md
- 根拠: spec.md:631-634（WHEN「場所のタブを開く」だけ。カードが 0 枚でも「各カードに … 見えている」は真）、spec.md:656-659（場所が 0 件なら識別子は必ず出ない）。
  spec.md:651-654 は「宛先がすべて同じ所」と「地図を描く要素は無い」の 2 つを束ね、後者は何を数えれば真偽が決まるかが書かれていない。spec.md:65-68 は件数・変え方・いつから・直す先の 4 つを 1 本で見ている
- kind: technical
- 提案: 前 2 本の WHEN に「滞在の当たった場所が 1 つ以上ある状態で」を足す。「地図を描く要素」は外して宛先の 1 本にするか、観測できる言葉（`img` / `canvas` / `iframe` が 0 個など）にする。spec.md:65-68 は変え方と直す先・いつからに分ける
- 処置: fixed specs/personal-entities/spec.md — 2 本の WHEN に前提（当たった滞在を持つ場所が 1 つ以上 / 場所が 1 つ以上ある状態で前の値も開く）を足し、「地図を描く要素」を外して宛先の 1 本にした。座標の記録の Scenario を変え方と直す先に分けた（R3 と同じ）

## R9. 「登録を 2 回押しても場所は 1 つ」は、押し直しで識別子を作り直す誤りがあっても緑になりうる
- 成果物: openspec/changes/st21-place-registry/specs/personal-entities/spec.md / tasks.md
- 根拠: spec.md:727 の THEN は「同じ原文が送られる**か**、1 回だけ送られ」と 2 通りを許す。tasks.md:187 は「1 回目の応答を `page.route` で落とす」で、`route.abort()` で落とすと 1 回目は**サーバに届かない**ので、2 回目が新しい識別子でも場所は 1 つ増えるだけになる。
  spec.md:681 の理由（押し直しで組み直すと 2 件になる）が試験で落ちない
- kind: technical
- 提案: THEN を「2 回の求めの本文（器の識別子と各記録の原文）が一致する」に絞る。tasks 10.2 に「1 回目は `route.fetch()` でサーバへ通してから応答だけを捨てる」と書く
- 処置: fixed 10.2 — THEN を「2 回の求めの本文（器の識別子と各記録の原文）が一致する」に絞り、tasks 10 の前置きと 10.2 に「1 回目は `route.fetch()` で届けて応答を捨てる」と書いた

## R10. Task 7.3 の検証コマンドは書かれたとおりには終わらない
- 成果物: openspec/changes/st21-place-registry/tasks.md
- 根拠: tasks.md:142 の `tools/stack.sh up --check-only` —— `tools/stack.sh` に `--check-only` は無く（grep 0 件）、`up` は「前景で待つ（Ctrl-C で全部止まる）」（tools/stack.sh:4）ので戻らない。
  tasks.md:143-144 の `curl -fsS "$BASE/places"` —— `$BASE` はどこでも定義されていない。読み出しは `authorize` を通る（`crates/server/src/lib.rs:1616` の `attributes_get` など。design.md:249「資格情報が無ければ 401」）ので、`Authorization` の無い curl は 401 で `-f` が落ちる
- kind: technical
- 提案: 起動と待ちと curl を 1 本の台本にする（`tools/smoke.sh` と同じ起動のしかた・`BASE` と合言葉の出どころを明記し、`-H "Authorization: Bearer …"` を付ける）。終了条件は rc=0 のまま
- 処置: fixed 7.3 — 検証を `tools/smoke.sh` の最後の段（同じサーバに `tools/seed.sh normal` を当て、`AUTH` つきの curl で数えて `OK seed places` を出す）に移した。待ち続ける `tools/stack.sh up` と未定義の `$BASE` を使わない

## R11. Task 4.2 が使う「今日の差し込み」が、後の 4.3 で入る
- 成果物: openspec/changes/st21-place-registry/tasks.md
- 根拠: tasks.md:105 の Scenario「移る前の座標は移る前のものとして返る」（spec.md:395「2026-10-01 に読み出す」）と「未来のいつからの移転はいまの座標を変えない」は今日を固定しないと再現しないが、
  `今日` を差し込める形にするのは tasks.md:108 の 4.3
- kind: technical
- 提案: 4.3 の差し込み口を 4.1 か 4.2 の前に移す（4.3 は Asia/Tokyo の境目の確かめだけ残す）
- 処置: fixed 4.1 — 今日と現在時刻の差し込み口を 4.1 で作る形にし、4.3 は Asia/Tokyo の境目の確かめだけにした

## R12. 画面の Scenario の印の置き場について、tasks の Global Constraints が自分の中で食い違う
- 成果物: openspec/changes/st21-place-registry/tasks.md
- 根拠: tasks.md:22「画面の Scenario の印は `web/e2e` に置く。jsdom（`web/src/__tests__`）は … 印は置かない」と、tasks.md:19-20「10 本は ST19 の印がそのまま当たる（`web/src/__tests__/master-view.test.tsx` ほか）」。
  REMOVED + ADDED にしたので、その 10 本はこの change が ADDED する画面の Scenario で、印は jsdom にある（`master-view.test.tsx:91` / `:202` など）。AGENTS.md「画面の Scenario は `web/e2e` で担保する。jsdom へも … 逃がさない」。
  例外の「場所の画面は文字のコントラストの下限を満たす」（tasks.md:23・158）は `tokens.ts` からの計算で、spec.md:836 の「場所の画面の本文と補助の文字の色を背景と比べる」を画面で測っていない
- kind: conflict
- 提案: 再 ADDED の 10 本を e2e に移すか、jsdom の印のままにする理由と範囲を Global Constraints に明記して食い違いを解く。コントラストは e2e で `getComputedStyle` から測る（フォーカスの輪郭と同じ形）か、例外にする理由を書く
- 処置: fixed D20 仮 — 振る舞いを変えていない ST19 の画面の 10 本は ST19 の jsdom の印のまま（理由と反転条件を design D20 に）、新しい画面の Scenario は e2e、と Global Constraints を書き直した。場所の画面の文字のコントラストは e2e で `getComputedStyle` から測る形に移した（tasks 10.4）

## R13. deep Q1 の「効く先」にある 360 px での縦の長さと 1 画面目の件数が、spec にも tasks にも無い
- 成果物: openspec/changes/st21-place-registry/tasks.md
- 根拠: deep.md:86-87（効く先「tasks の画面の章と `web/e2e`（360 px での縦の長さ、1 画面目の件数をアサートできる）」）。本人が選んだ理由は deep-answers-1.txt の「縦 1,599 px = 2.5 画面。1 画面目に見える登録した場所 3 / 6」で、spec.md:619 も理由にこの量を挙げている。
  tasks.md:195-198 の 360 px の試験は触れる対象の寸法だけで、縦の長さ・1 画面目の件数を見ていない。カードを大きくして 1 画面目が 1 件になっても落ちない
- kind: daily
- 提案: 「登録 6 の量で、幅 360 px の 1 画面目に場所のカードが 3 枚以上見える」程度の Scenario を足して 10.4 に割り当てる（数は proto の実測から）。見送るなら deep の効く先と食い違う理由を design に書く
- 処置: fixed D21 仮 — Scenario「登録 6 の量で 1 画面目に場所が 3 枚見える」を足して tasks 10.1 に割り当て、design D21 に実測の出所・操作 3 つを 1 行に並べること・反転条件を書いた

## R14. S-2 の見出しを場所の名前にする `fix/` に、起こす係がいない
- 成果物: docs/handoff/ST25.md
- 根拠: docs/handoff/ST25.md:4「ST25 にやってほしいことではない」・:9「担当は見つけた側（ST21）」。ST21 は archive で thread が終わり、ST25 の下流は handoff を読んでも自分の仕事ではないと読む。
  tasks.md にも proposal.md:76-77 にも、3 本の archive を待って `fix/` を起こす issue や記録を作る項目が無い。CLAUDE.md は「merge の後に作る規則だと作る係がいなくなる」（issue は PR と同時に作る）としている
- kind: defer
- 提案: ST21 の PR と同時に `fix/` の issue を作る（`refs` で ST22・ST25 を指す）項目を tasks 11 に足すか、`docs/handoff/ST25.md` の項に `処置: followup` の形で拾う側を決めて書く
- 処置: followup ST25 — `docs/handoff/ST25.md` に起こす係を書いた（ST21 の下流が PR 本文の「後続」に挙げる。ST25 の下流も PR 前に handoff を読んだとき、ST21・ST22 が archive 済みなら自分の PR 本文の「後続」に挙げる）。ST21 の tasks 11.1 と design D19 にも書いた

## R15. 座標の版の期間は、「いつから」が前の版より古い移転と、今日を含む版が無い場所で決まり方が不自然・未定
- 成果物: openspec/changes/st21-place-registry/specs/personal-entities/spec.md
- 根拠: spec.md:424「版を書いた順に並べ、各版を次の版の当て始めまで当てる」。移ったを 2026-04 で書いた後に、いつから 2025-06 の移ったを書くと、2 つ目の版は 2026-04 → 2025-06 の空の期間になり、1 つ目の版と 3 つ目の版が 2025-06〜2026-04 で重なる。spec.md:127 の `invalid_coord_change` はこれを断らない。
  初めての座標の記録を消して（ST23）未来の移転だけが残った場所は、spec.md:336 の「今日を含む版」が無く、いまの座標が決まらない（design.md:241 の `coord` は null を持たない）。
  spec.md:423 の `Asia/Tokyo` が年・年月の当て始めにも掛かるかが文から読めない
- kind: technical
- 提案: (a) 書いた順ではなく当て始めの順に並べる、(b) 前の版より古い「いつから」を断る、のどちらかを D7（仮）で決めて Requirement に書く。今日を含む版が無いときのいまの座標（最も近い版・null など）と、3 つの精度すべてが `Asia/Tokyo` の 0 時であることを本文に書く
- 処置: fixed D7 仮 — 当て終わりを「後に書いたどの版の区切りよりも前まで」（min）にして、後に書いた版が勝つ形にした（古い「いつから」の移転を断る案は、位置の記録が無い期間の移転を後から書けなくするので採らない）。今日を含む版が無いときのいまの座標（書いた順の最後の版）と、3 つの精度すべてが Asia/Tokyo の 0 時であることを Requirement に書き、Scenario「後から書いた古いいつからの移転が後を占める」を足した（tasks 5.2）

## R16. 再 ADDED の Requirement の導出元「design D9」が、この change の D9 と取り違えられる
- 成果物: openspec/changes/st21-place-registry/specs/personal-entities/spec.md
- 根拠: spec.md:862「深掘り Q2（本人が proto で決めた構造）。design D9。ST21 の深掘り Q1 …」。正典では ST19 の design D9（S-6 の骨格）を指していたが、この change の design.md:161 の D9 は「場所ごとの合計・最後に居た日・24 時間の帯・並び」
- kind: technical
- 提案: 「ST19 の深掘り Q2 / ST19 design D9」と Story を付けて書く
- 処置: fixed specs/personal-entities/spec.md — 導出元を「ST19 の深掘り Q2 と ST19 design D9」に直した

---

## 観点ごとの該当なし

- 観点 1（deep の決定が正典に写っているか）: R6・R13 以外は該当なし（確かめた範囲: Q1〜Q4 の本人の答えと C1〜C13 を spec の Requirement / Scenario と突き合わせた。
  Q3 の「外部 AI に出してよい」は spec.md:590・595-598 にあり、推奨の「ローカル AI まで」は場所について残っていない。
  Q1 で覆した 4 軸（2 段・長く居た順・緯度経度を貼る・広さを出さない）は spec に残っていない。上位 10 件（spec.md:684-687）・既定 100 m（spec.md:353-356・704-707）・照合の 80 / 120 m（spec.md:484-492）は数値つきの Scenario がある。
  「要件へ戻すもの」の FR-48 / FR-49 / PERM-3 / 扉 #15 / 扉 #17 は `docs/requirements.md` に ★ 2026-10-01 つきで入っている（:476・:483・:653・:1058・:1068））
- 観点 3（置き場）: R2・R4・R5 以外は該当なし（確かめた範囲: spec に関数名・crate 名・列名・表名は無い（`core.` / `.rs` / `payload` / `logical_source` を grep して 0 件）。
  proposal の Capabilities（`personal-entities` だけ）・`specs/` のディレクトリ・`INDEX.md:75` の capability 表は一致し、要件の前倒しは INDEX の訂正の節に理由つきで書いてある）
- 観点 5（Story）: 該当なし（確かめた範囲: `check_chain.py` OK。ST21.md の requires に ST16、layer 2。価値・完了の判定「名前を変え、座標を修正しても識別子が変わらない」は deep の決定と食い違わない。
  satisfies に無い PERM-3 / FR-50 / FR-51 / NFR-17〜19・22・23 は INDEX の訂正の節に、ST19 と同じ型で載っている）
