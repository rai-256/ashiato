# ST12 spec レビュー 第 2 回（独立）

対象: 第 2 回の深掘り（deep.md の Q9 / Q10 / Q11）を反映した差分 `git diff d8f742e..HEAD` ——
`docs/requirements.md` / `docs/stories/{stories.json,ST12.md,ST13.md,INDEX.md}` / `openspec/changes/st12-archive-ingestion/{deep.md,proposal.md,design.md,tasks.md,specs/external-ingestion/spec.md,specs/collection-coverage/spec.md}`。
突き合わせの正本: deep.md（Q9〜Q11 の本人の答えと読み取り）、`deep-questions-r2.json`（選択肢の文）、`openspec/changes/st04-offline-retention/specs/collection-coverage/spec.md`（main と `origin/feat/st04-offline-retention` の 2 版）、
`scripts/board.py` / `scripts/merge_gate.sh` / `scripts/story.sh`、`web/src/__tests__/one-scroll.test.tsx`。**成果物は触っていない。**

実施日: 2026-09-15。ブランチ `docs/st12-upstream`（HEAD d8f742e の後の差分）。

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate st12-archive-ingestion --strict` | `Change 'st12-archive-ingestion' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0。観点 8 の再生成との一致を含む） |
| `python3 scripts/check_scenarios.py . st12-archive-ingestion` | `scenarios: FAIL (担保なし 110 件)`（rc=1）。内訳は external-ingestion 101 本（上流なのでテストが無い）+ collection-coverage の新しい 2 本（`箱があるときは…`）+ ST04 の破棄の印の 7 本（テストは `origin/feat/st04-offline-retention` の `drop-mark.test.tsx` / `drop-detail.test.tsx` にだけあり、main に無い）。どれも上流の段階では想定どおりで、tasks 12.2 が回収する |
| `python3 scripts/review_triage.py . st12-archive-ingestion` | `triage: OK`（rc=0。このファイルを書く前の状態。指摘 38 件 / 仮決め 8 件 / 要件へ戻すもの 6 件） |

手で確かめたこと:

- **Q9 の読み取りは本人の選択と矛盾しない**: 選んだ選択肢の文は「Must の 5 本は箱の高さぶん下へ押され、…（1,280 px）と…（640 px）を超える —— 選ぶと、その予算を変える判断になる」（`proto-r2.html:145`、deep.md:229）。
  「予算を箱の高さを除いて数える」は「箱の高さぶん押される」をそのまま予算に写した形で、選択肢の文を越えていない。proto の実測（箱 132 px のとき Must の最後 1,379 px / 2 本目 754 px）から箱を引くと 1,247 px / 622 px で、既存の予算に収まる計算とも合う。
  ただし **160 px の上限は本人が見ていない数**で、deep.md:239 は D12（仮）としている（R14）
- **collection-coverage の MODIFIED の写し**: main の ST04 の delta（733257f）の同名 Requirement と行単位で diff を取った。差は「予算の SHALL 1 行・導出元 2 行・2026-09-15 の注記・既存 Scenario 2 本の WHEN・新しい Scenario 2 本」だけで、**main の版に対しては予算の文だけを変えている**。PR #49 の head の版とは食い違う（R1）
- **Q11 と D9**: 選んだ選択肢「システムの写しを作らない（既に作った写しは残す）」（`deep-questions-r2.json` Q11）と、spec.md:324 / design D9 の「補足の読み」は同じことを言っている。食い違いは Q10 の強制の写しとの組み合わせの側（R7）
- **Q10 の選択肢の文と spec**: 「印を置くまで Takeout の書庫の中身は格納しない / 台帳と写しには残す / 印を置いたら写しから読み直す / 確認待ちは写しの設定に関わらず写す / Timeline.json と移行前のロケーション履歴は待たない」は、spec.md:147-158 の SHALL にすべてある。越えているのは「また確認待ち」の拡張（R4）
- **proposal の Capabilities と specs/**: `external-ingestion` と `collection-coverage` の 2 つで一致。INDEX の capability 表（`INDEX.md:72`）に **ST12** がある。盤面（`board.py`）は ST14 / ST15 を「`collection-coverage` を ST04/ST12 が触っている」で衝突待ちにしていて、proposal の重なりの表と一致
- **tasks の Scenario の割り当て**: external-ingestion の 101 本はすべて tasks.md に名前がある。collection-coverage で tasks に名前が無い 18 本は、MODIFIED で写した既存（ST02 の 11 本と ST04 の 7 本）で、既存の印を生かす（tasks.md:19）

---

## R1. collection-coverage の delta が写した ST04 の文は main の版で、PR #49 の head はすでに同じ Requirement に 1 文と 1 Scenario を足している
- 成果物: openspec/changes/st12-archive-ingestion/specs/collection-coverage/spec.md / tasks.md（0.1）/ docs/stories/INDEX.md
- 根拠: `git show origin/feat/st04-offline-retention:openspec/changes/st04-offline-retention/specs/collection-coverage/spec.md`（head 8abafb5）の「稼働状況は 1 年を週に畳んだ格子で見える」には、
  `THE SYSTEM SHALL 件数を持たない区間（時間ごとの件数を持たない破棄の範囲）を、件数を添えずに「破棄（時刻〜時刻）」とだけ表示する。` と `#### Scenario: 件数を持たない区間は件数を添えずに時刻だけが出る`（テストの印は `drop-detail.test.tsx:67`）がある。
  ST12 の delta（spec.md:1-206）にはどちらも無い。MODIFIED は Requirement を丸ごと置き換えるので、このまま archive すると ST04 の 1 文と 1 Scenario が正典から消える。
  deep.md:243 / design.md:283 / INDEX.md:160 の「ST04 の delta の文を写した」は、main の 733257f の版に対してだけ正しい。
  回収は tasks 0.1 だが、終了条件が「`python3 - <<'PY'` で…差が…だけであることを**出力で確かめ**」（tasks.md:35-38）で、差があっても rc は 0 のまま
- kind: technical
- 注: 呼び出し元が kind を conflict から technical に直した —— 本人の決定を解いたのではなく、写した元の版の取り違え（写し直せば戻る）
- 提案: 0.1 の検査を「許した差（予算の 1 行・導出元・注記・WHEN の 2 本・Scenario 2 本）以外の行があれば exit 1」にし、rc で判定する。いまの delta も PR #49 の head から写し直すか、「写した元は main の 733257f」と書き直す
- 処置: fixed specs/collection-coverage/spec.md — delta を PR #49 の head（8abafb5）の delta から写し直し（「件数を持たない区間」の 1 文と Scenario を含む）、写した元を注記と design D13 に書いた。tasks 0.1 の終了条件を「ST04 の change が残っていれば rc=1」「許した差のほかの行があれば exit 1 の比較」に直した

## R2. 「`requires` がそれを機械に持たせる」は事実と違う。tasks.md があると盤面と merge_gate は ST04 の archive を待たずに下流の起動を出す
- 成果物: docs/stories/INDEX.md / design.md（D13）/ proposal.md（重なりの表）/ tasks.md（冒頭の「着手の前提」）
- 根拠: INDEX.md:160「ST12 の下流は ST04 の archive を待つ（`requires` がそれを機械に持たせる）」。
  `scripts/board.py:115-118` の `state()` は、`has_tasks` なら requires を見ずに `下流` か `実装待ち` を返す（requires を見るのは tasks.md が無いときの `着手可` / `proposal迄` だけ）。
  `実装待ち` は盤面で「下流を始められる」として `scripts/story.sh ST<NN>` を出す（board.py:185-189, 202）。`scripts/merge_gate.sh:112-114` は上流の lane で無条件に「merge 後・別ターミナル : scripts/story.sh $ST」を出す。
  `scripts/story.sh` に `requires` の文字列は無い（`grep -n requires scripts/story.sh` が 0 件）。
  いま待たせているのは tasks.md:10 の文と 0.1 の文だけ。CLAUDE.md の「上流は先行 Story が merge されるまで deep と proposal で止まる…書いても古くなる」が想定した状態で、R1 はその実例
- kind: premise
- 提案: INDEX / design D13 / proposal の文を「requires は盤面の表示だけで、下流の起動は止めない」に直し、0.1 の先頭を `test -d openspec/changes/st04-offline-retention && exit 1` のような止まる形にする。もしくは ST04 の archive まで上流の PR を draft に留める理由を PR 本文に書く
- 処置: escalated — deep.md の第 3 回の節に R2 を記録し、PR 本文と issue の冒頭で本人に見せた（下流は tasks 0.1 で止まる。harness2 の gate の穴として報告）。成果物は 「requires が機械に持たせる」を INDEX / design D13 / proposal で「requires は盤面の表示だけで下流の起動は止めない。止めるのは tasks 0.1 の終了条件と PR・issue の冒頭の文」に直した。gate が requires の未 archive を見ない穴は harness2 側の問題として報告する

## R3. Story の「価値」「完了の判定」「壊してはいけないもの」が第 2 回の答えのまま更新されていない
- 成果物: docs/stories/stories.json（→ ST12.md）/ design.md（D15）
- 根拠: ST12.md:14 の価値「フォルダに置くだけで取り込ませたい。…手順を増やすと続かない」、ST12.md:64 の完了の判定「書庫を置くと、しばらくして中身が D-01 に入る」は、
  Q10 の答え（印を置くまで Takeout の書庫の中身は格納しない。印を置く手順が増える）と食い違う。Takeout の書庫を置いただけでは入らない。
  ST12.md:60 の壊してはいけないもの「Must の 5 ソースがひとスクロール以内に見えること」は、Q9 の「箱の高さを除いて数える」と食い違う。
  design.md:308 の D15 の 1 行目は「合成の Takeout の書庫を置いて記録の件数を待つ」のままで、印を置く段が無い（この手順だと Q10 の規則で 0 件になる）。
  `check_chain.py` は stories.json からの再生成との一致しか見ないので落ちない
- kind: technical
- 注: 呼び出し元が kind を conflict から technical に直した —— 本人の決定を解いたのではなく、答えの Story への戻し漏れ（再生成で戻る）
- 提案: stories.json の `done` の 1 行目を「書庫を置き、（Takeout の書庫は形の確認の印を置くと）しばらくして中身が D-01 に入る」に、`breaks` の 3 つ目を「箱の高さを除いて」に直して `make_story.py` で再生成する。価値の「手順を増やすと続かない」と印の手順の関係は deep.md の Q10 の読み取りに 1 行書く。D15 の 1 行目に `archive-shape.sh --confirm` の段を足す
- 処置: fixed D15 — stories.json の完了の判定 1 行目に「Takeout の書庫は、形の確認の印を置いてから」、壊してはいけないもの 3 つ目に「箱の高さを除いて」を入れ、完了の判定に「印を置くまで Takeout の中身が入らない」を足して再生成した。design D15 の 1 行目に印の段を足した

## R4. 「知らない形のファイルだけまた確認待ち」は、選択肢の「最初に 1 回」を越えて手作業を繰り返させる
- 成果物: openspec/changes/st12-archive-ingestion/specs/external-ingestion/spec.md（spec.md:152, 185-188）/ design.md（D16）/ deep.md（Q10 の読み取り）
- 根拠: 本人が選んだ選択肢の detail は「**最初に 1 回**、`archive-shape.sh` の出力を見て設定に印を置く手順が増える」（`deep-questions-r2.json:31`）。
  deep.md:254 の読み取りと spec.md:152 は「後の書庫に確かめた形に無いものが出てきたら、そのファイルだけまた確認待ち」で、design.md:329 はさらに「知らない `products` の値が 1 つ増えただけでも確認待ち」。
  マイアクティビティの製品は書庫によって増える（design.md:197「検索・Discover・Play・マップ・アシスタント…」）ので、2 か月ごとの書庫のたびに手作業が起きうる。本人はこの頻度を読んで選んでいない。
  反転条件（design.md:330「2 か月に 1 回より多く起きたら緩める」）は、緩める側を選ぶと Q10 が防いだ凍結（rewrite-all）に戻るので、「計算し直せば戻る」B ではない
- kind: daily
- 提案: 「また確認待ち」を本人の決定ではなく読み取りの拡張だと PR 本文の冒頭で明示し、merge の前に本人に 1 行で確かめる（「新しい製品が出るたびに印を置き直すか」）。問いにするなら A（loss: rewrite-all）として立てる
- 処置: escalated — 第 3 回 Q12（A / rewrite-all。推奨: 知らない製品のファイルだけまた確認待ち）として本人に問うた。deep.md の第 2 回 Q10 の読み取りに「選んだ文を越えた読み取りだった」と書き、第 3 回 Q12 に R4 を記録。tasks 7b.1 の後半は答えが入るまで着手しない

## R5. 「また確認待ち」になる条件が spec と design D16 で違う
- 成果物: specs/external-ingestion/spec.md / design.md（D16）/ tasks.md（5b.1）
- 根拠: spec.md:152 は条件を「印を置いた形に無いもの（**知らない製品の名前・知らない最上位の鍵**）」とする。
  design.md:323 の「形」は `(見分けた種類, 書庫の中のパスの型, 最上位の鍵の集合, **項目の欄の名前の集合**, products の値の集合)` で、印は「形の全体に対して置く」（design.md:329）。
  項目の欄の名前が 1 つ増えた（Google が任意の欄を足した）だけ、パスの型が変わった（Takeout のフォルダ名は言語で訳される。`deep-questions-r2.json` Q10 の context）だけでも design では確認待ちになり、spec はそれを言っていない。
  tasks.md:113 の 5b.1 は design の側（欄の名前・パスの型を含む）で実装させる
- kind: technical
- 提案: どちらかに揃える。design の側に揃えるなら spec.md:152 の括弧に「項目の欄の名前・書庫の中のパスの型」を足し、欄が増えた場合の Scenario を 1 本置く
- 処置: fixed D16 — 形を振り分けを決めるもの（見分けた中身・マイアクティビティの products の値）だけに狭め、spec の SHALL と Scenario「欄の名前が増えただけでは確認待ちにならない」を置いた。確認の出力には判断の材料として欄の名前なども出す（値は出さない）

## R6. 第 1 回の Scenario の多くが、WHEN に印の状態を持たず、印の前は格納しない規則と字面で矛盾する
- 成果物: specs/external-ingestion/spec.md
- 根拠: 前提は Requirement「取り込み待ち置き場に置かれた書庫を読む」の理由の段落（spec.md:23）に 1 文あるだけで、SHALL でも Scenario でもない。archive 後は他の Requirement からは見えない。
  字面のまま試験にすると落ちる Scenario: `専用のフォルダに置いた書庫が読まれる`（:25-28。YouTube の Takeout を置いて 10 分で格納）/ `ダウンロードのフォルダの Takeout の書庫が読まれる`（:30-33）/
  `名前が書き込み途中でなくなったファイルは読まれる`（:50-53）/ `分割書庫は 1 本ずつ読まれる`（:70-73）/ `6 つの中身がそれぞれ読まれる`（:112-115）/ 重複の 7 本（:250-283 の視聴履歴）/ `壊れた 1 件があっても残りは格納される`（:296）/ `残さない設定でも記録は格納される`（:360-363）。
  `印を置く前の Takeout の書庫は格納されない`（:160-163）と `専用のフォルダに置いた書庫が読まれる`（:25-28）は、WHEN が同じ状態を指しうるのに THEN が逆
- kind: technical
- 提案: 該当する Scenario の WHEN に「形の確認の印を置いた後に」を足す（Timeline.json / Records.json だけの Scenario は不要）。spec.md:23 の段落は削るか、SHALL に上げる
- 処置: fixed specs/external-ingestion/spec.md — Takeout の書庫の中身が格納されることを見る Scenario 30 本の WHEN に「形の確認の印を置いた後に、」を足し、理由の段落の前提文は削った。tasks 0 に「格納する試験の準備で合成の形の印を入れる」を足した

## R7. 確認待ちの強制の写しと「残さない」設定の Scenario が衝突し、印を置いた後の写しの扱いが決まっていない
- 成果物: specs/external-ingestion/spec.md / design.md（D9）
- 根拠: `残さない設定では写しを作らない`（spec.md:355-358。WHEN「写しを残さない設定で書庫を置く」THEN「写しが増えない」）と、`確認待ちの書庫は写しを残さない設定でも写される`（:170-173）は、Takeout の書庫を印の前に置いた場合に THEN が逆になる。
  R6 の前提文（:23）は「格納される」Scenario だけが対象で、写しの Scenario を覆わない。
  SHALL（:324）の「（既に作った写しは消さない。形の確認を待っている書庫は除く）」は、「除く」が「作らない」に掛かるのか「消さない」に掛かるのか（確認待ちの写しは消す、とも読める）が文から決まらない。
  **印を置いて読み直した後、残さない設定で強制的に作った写しを残すのか消すのか**は spec にも D9 にも無い。Q11 で本人が選んだ「残さない」の利用者が、Q10 の分だけ本文（検索語・URL）の写しを持ち続けるかどうかがここで決まる（ST23 の物理削除が届かない写し。deep.md:290）
- kind: conflict
- 提案: SHALL を 2 文に分ける（「残さない設定でも、確認待ちの書庫の写しは作る」「その写しを、読み直した後に〈残す / 消す〉」）。:355 の WHEN に「Timeline.json を」か「印を置いた後に」を足す。読み直した後に消す側は、印の後に読み直せなくなるので Q11 の選択肢 2 の不可逆と同じ性質になる。どちらにするかを PR 本文で本人に見せる
- 処置: fixed D9 仮 — SHALL を 2 文に分け（残さない設定では写しを作らない・既存は消さない / それでも確認待ちの書庫の写しは作り、読み直し終えたら消す）、Scenario「確認待ちのために作った写しは読み直した後に消える」を足した。消す側にしたのは第 2 回 Q11 の「写しを作らない」に戻すためで、代償（その書庫を版の上げで読み直せない）は Q11 で本人が受け入れたものと同じ。design D9 に（仮）と反転条件を書き、PR 本文に列挙する

## R8. 形の確認の印（5b）が、それに依存する写し（7.2）より前にあり、それが変える格納の振る舞いに依存する 5.7 / 6 / 7 / 9 より後にある
- 成果物: tasks.md
- 根拠: 5b.1（tasks.md:113-117）は「確認待ちの書庫は写しの設定に関わらず写す」、5b.2（:118-120）は「写しから読み直す」で、写しそのものを作るのは 7.2（:137）。依存が逆向き。
  一方、5.7 の `CT archive_end_to_end`（:106-109。合成の Takeout の書庫を置いて走査 1 回で 6 つの中身から 1 件以上）、6.1 の重複、7.2 の `残さない設定でも記録は格納される`、9.1 の最終日は 5b より前に書かれ、
  どれの検証にも「印を置いた DB で」が無い。5b を入れた時点で、先に緑にしたこれらの試験が落ちるか、印を置く準備を後から足すことになる（鍵に依存するタスクより鍵を変えるタスクが後ろにある型）
- kind: technical
- 提案: 5b を 7.2 の後に移すか、5b.1 を「印の表と判定」、写しと読み直しを 7.2 の後の 5b.2 に分ける。5.7 / 6.x / 7.2 / 9.1 の検証の書き出しに「形の印を置いた DB で（試験の準備で `core.archive_shape_confirmation` に合成の形を入れる）」を足す
- 処置: fixed tasks.md — 形の確認の印の章を写し（7.2）の後ろの 7b に移し、Takeout を格納する試験（5.7 / 6.x / 7.x / 9.x / 11.3）の準備で `core.archive_shape_confirmation` に合成の形を入れる、を tasks 0 に書いた

## R9. 印の前は格納しないことの検証が YouTube だけを数え、Scenario 2 本が主張を 2 つ束ねている
- 成果物: tasks.md（5b.1 / 5b.2）/ specs/external-ingestion/spec.md
- 根拠: SHALL（spec.md:149）の対象は「YouTube の視聴履歴・検索履歴・マイアクティビティ・Chrome の履歴」。Scenario `印を置く前の Takeout の書庫は格納されない`（:160-163）は視聴履歴だけを言い、
  5b.1 の検証（tasks.md:117）は「`core.event` の `c03-youtube-*` が 0 件」だけを数える。Chrome の履歴やマイアクティビティが印の前に格納されても緑になる（凍結されるのはまさにマイアクティビティの名前。Q10 の context (1)）。
  `知らない製品のファイルだけがまた確認待ちになる`（:185-188）の THEN は「視聴履歴は格納され」と「そのマイアクティビティのファイルは確認待ち」の 2 つ。
  `形の確認の出力に記録の値が出ない`（:175-178）の THEN は「『京都 旅館』が無い」と「種類と欄の名前と件数が出る」の 2 つ
- kind: technical
- 提案: 印の前の Scenario の WHEN を 4 つの製品を含む書庫にし、THEN を「`Timeline.json` と移行前のロケーション履歴以外の書庫の論理ソースに記録が 0 件」にする。5b.1 の検証も `logical_source LIKE 'c03-%' AND logical_source NOT LIKE 'c03-timeline-%' AND NOT LIKE 'c03-legacy-%'` の 0 件にする。束ねた 2 本はそれぞれ 2 本に分ける
- 処置: fixed tasks.md — 印の前の Scenario を 4 つの製品を含む書庫にし、THEN を「タイムラインと移行前のほかの書庫の論理ソースに 0 件」にした。検証の SQL を `c03-%` からタイムラインと移行前を除いた 0 件にした。束ねた 2 本（知らない製品 / 形の出力）をそれぞれ 2 本に分けた

## R10. 確認待ちの書庫の台帳の行が design の中で「1 行 = 1 回の読み」と食い違い、印の後の読み直しと「同じ中身は読み直さない」の関係が spec に無い
- 成果物: specs/external-ingestion/spec.md / design.md（D7 / D16）
- 根拠: design.md:176「`core.archive_ledger` 1 行 = 1 回の読み」に対し、D16（design.md:326）は「台帳に `outcome = 'pending_shape'` の行（**ファイルの数だけ**。件数は 0）」。
  spec の SHALL（:385）は置き場に残り続けるファイルについて走査のたびに台帳の行を足さないと言うが、それを見る Scenario `ダウンロードのフォルダに残り続ける書庫は台帳を増やさない`（:406-409）の WHEN は「読み終わった後」だけで、
  **確認待ちのままダウンロードのフォルダに残る書庫**（走査は既定 120 秒。design D1）を見る Scenario が無い。台帳は追記のみなので、ここを外す実装は印を置くまで消せない行を積む。
  また「覚えている書庫と同じ中身・同じ解析器の版のファイルを読み直さない」（:383）と「印を置くと写しから読み直す」（:151）の関係（確認待ちの行は『覚えている』に入るか、読み直した行の outcome は何か）が spec に無い
- kind: technical
- 提案: D16 の「ファイルの数だけ」を D7 の粒度（1 回の読みに 1 行）に合わせ、ファイルごとの確認待ちは `archive_pending_shape` の側に持つと直す。Scenario を 2 本足す（確認待ちの書庫が走査を 3 回受けても台帳の行は 1 つ / 印を置いた後の読み直しは、同じ中身・同じ版でも読み直して台帳に 1 行足す）
- 処置: fixed D16 — 確認待ちの台帳の行を 1 回の読みに 1 つにし（ファイルごとの確認待ちは archive_pending_shape の側）、確認待ちは覚えている書庫に入れて走査のたびに読み直さない、印の後は同じ中身・同じ版でも読み直して台帳に 1 行、を design と spec の SHALL に置いた。Scenario「確認待ちの書庫は走査を重ねても台帳の行が増えない」「印を置いた後の読み直しは台帳に 1 行足す」を足した

## R11. Q9 の軸 3「書庫のソースの格子は開いておく」を見る Scenario が無い
- 成果物: specs/external-ingestion/spec.md（稼働状況の画面の Requirement）
- 根拠: proto-r2.html:148-150 の軸 3 は「開いておく（直近 4 週を出す）」と「見出しの行だけ（押すと開く）」で、本人は前者を選んだ（deep.md:223, 231）。
  spec.md:562 は「Must の 5 ソースと同じ形の格子を出す」とだけ言い、開いた直後に書庫のソースの直近 4 週が出ていることを見る Scenario は :584-662 に無い（`grep -n "4 週" spec.md` は理由の段落の 1 件だけ）。
  collection-coverage の既存 Scenario「開いた直後に 2 ソース以上の直近 1 か月…」は Must の 2 本で満たされるので、書庫のソースを畳む実装でも緑になる。退役したソースは既定で畳むので、「同じ形」から開いた状態が一意に読めるわけでもない
- kind: technical
- 提案: Scenario を 1 本足す（WHEN 視聴履歴の記録がある状態で画面を開く / THEN 書庫のソースの格子が押さずに直近 4 週の行を出している）。tasks 10.2 か 10.4 に割り当てる
- 処置: fixed 10.2 — Scenario「書庫のソースの格子は開いた直後から直近 4 週を出す」と SHALL を足し、tasks 10.2 に割り当てた

## R12. 箱の高さを含む予算の Scenario が 132 px と 160 px で割れ、tasks 10.4 の試験の形と合わない
- 成果物: specs/collection-coverage/spec.md / specs/external-ingestion/spec.md / tasks.md（10.4）
- 根拠: collection-coverage spec.md:152-155 は「高さ **132** CSS px の箱」で 772 px、:157-160 は「高さ **160** CSS px の箱」で 1,440 px。上限（160 px）のときの 2 ソースの予算（800 px）を見る Scenario は無い。
  tasks.md:185 の `archive-one-scroll.test.tsx` は「高さ 160 px の箱」で両方を見るので、132 px の Scenario の WHEN を再現していない（jsdom の宣言の勘定で箱の高さを 132 に合わせる手段も書いていない）。
  external-ingestion spec.md:644-647 は「1,280 CSS px + 箱の高さ」で箱の高さに上限が無く、collection-coverage の 2 本と同じことを別の形で言う重複
- kind: technical
- 提案: 2 本とも上限の 160 px（800 px / 1,440 px）に揃える。external-ingestion の :644 は collection-coverage の Scenario に寄せて消すか、THEN を「1,440 CSS px 以内」にする
- 処置: fixed 10.4 — collection-coverage の Scenario を上限の 160 px（800 px / 1,440 px）に揃え、external-ingestion の Scenario の THEN を 1,440 px にした

## R13. 「箱が無い状態」は ST12 の後には起きず、既存の試験の印とも合わない。10.4 は予算を箱と一緒に動かす形を許す
- 成果物: specs/collection-coverage/spec.md / tasks.md（10.3 / 10.4）
- 根拠: collection-coverage spec.md:144, 149 は既存 2 本の WHEN に「『直近に置いた書庫』の箱が無い状態で」を足した。
  一方 external-ingestion spec.md:572 は台帳が空でも箱に文字を出し、design.md:269 も「台帳が空なら『まだ書庫が置かれていません』」で、箱が無いのは `/archives/status` の読み出しが失敗したときだけ。
  tasks.md:181 は「既存の `one-scroll.test.tsx` の勘定から箱の高さを引く」とし、印（`one-scroll.test.tsx:170, 181, 208`）の付いた試験は**箱がある**画面を測ることになり、印の Scenario の WHEN（箱が無い）と食い違う。
  同じファイルの 15 行目は「突き合わせる相手は固定の予算。両側が一緒に動く形にしない」で、箱の宣言の高さを引く勘定は箱が伸びるほど予算が伸びる。歯止めは `箱は 160 px を超えない`（10.3。`VT latest-archive.test.tsx`）だけで、10.3 の検証は件数 ≥ 1 だけ、宣言の高さの勘定（`declaredHeight`）は `one-scroll.test.tsx` の中の関数で他から使えない。
  10.4 は第 1 回にあった `git diff --exit-code origin/main -- web/src/__tests__/one-scroll.test.tsx` を外した
- kind: technical
- 提案: 既存 2 本の WHEN は「箱を除いて数えたとき」のように、実際に起きる状態で書き直す。10.4 の勘定は「引くのは min(箱の宣言の高さ, 160)」とし、`grep -q 'ONE_SCROLL_PX = VIEWPORT_H_PX \* 2' web/src/tokens.ts` と `VIEWPORT_H_PX = 640` の不変を終了条件に足す。10.3 の 160 px の試験は `declaredHeight` を共有の helper に出してから測る、と書く
- 処置: fixed 10.4 — 既存 2 本の Scenario の THEN を「箱の高さを除いて数えると…」に直し（WHEN の「箱が無い状態」は外した）、Scenario「箱の高さを除く量は 160 px を超えない」を足した。tasks 10.4 の勘定を min(箱の宣言の高さ, 160) にし、予算の定数が変わらないことを grep の終了条件に足した。declaredHeight を共有の helper に出して 10.3 の 160 px の試験も同じ勘定で測る（design D12）

## R14. 箱が溢れたときの優先順で、読めなかった書庫の結果がいちばん先に省かれる
- 成果物: design.md（D12）/ specs/external-ingestion/spec.md
- 根拠: design.md:272 の優先順は「読んでいる途中 → 読めない置き場・取り込み器の止まり → 形の確認待ち → **直近の書庫の結果**」で、溢れた行から省く。
  spec.md:569 / :619-622 は「出しきれない文字は省き、省いたことを示す」を許していて、`読めなかった書庫は文字で出る`（:634-637）の WHEN は他の状態が同時に起きていない場合だけ。
  読めなかった書庫が画面に出ることは第 1 回 Q5 の理由（「置いたのに入っていないことに気づけない。書庫は約 7 日で失効する」spec.md:580）で、ST12 の完了の判定の 5 つ目（ST12.md:68）。
  2 か月ぶりに書庫を置いた日は「形の確認待ち」と「読んでいる途中」が同時に起きやすく、そのとき結果の行が省かれる。160 px 自体も本人が見ていない数（deep.md:239 の D12（仮））で、requirements.md:471 は「Q9 の決定」の★の中に入れている
- kind: conflict
- 提案: 「読めなかった / 格納に失敗した」の行は省かない、を SHALL と Scenario（読んでいる途中・確認待ちと同時に読めなかった書庫がある）に置き、優先順を直す。requirements.md の★の 160 px には「（仮。design D12）」を添える
- 処置: fixed D12 仮 — 読めなかった・格納に失敗した行は省かない、を spec の SHALL と Scenario「箱が溢れても読めなかった書庫は省かれない」、design D12 の優先順に置いた。requirements.md の★の 160 px に「仮。design D12」を添えた

## R15. 要件 FR-55 の本文に、覆された「格子の群の頭に」がそのまま太字で残っている
- 成果物: docs/requirements.md
- 根拠: requirements.md:468 の本文は `**格子の群の頭に、直近に置いた書庫 1 件の結果（…）を出す。**` のまま。直下の★（:470-472）が「格子の群の頭ではなく Must の 5 ソースの格子の前」に訂正している。
  同じ文書の他の訂正（例 :97 / :189-190 の FR-16）は本文を直して★に経緯を書く形。ST12.md の抜粋（:41-44）は本文と★の両方を引くので、Story の読み手には両方が同じ重さで見える
- kind: technical
- 注: 呼び出し元が kind を conflict から technical に直した —— 本人の決定を解いたのではなく、要件の本文と★の食い違い（本文を★に合わせれば戻る）
- 提案: 本文を「Must の 5 ソースの格子の前（達成の下）に、直近に置いた書庫 1 件の結果…を出す。書庫を読んでいる間は件数を出す」に直し、★は経緯だけにする。再生成で ST12.md に写す
- 処置: fixed D12 — FR-55 の本文を「Must の 5 ソースの格子の前（達成の下）に…読んでいる間は件数を出す」に直し、★は経緯だけにした。ST12.md を再生成した

## R16. deep.md の「要件へ戻すもの」の表が空行で割れ、第 2 回の 2 行が表にならない
- 成果物: openspec/changes/st12-archive-ingestion/deep.md
- 根拠: deep.md:278 の表の最終行（§5 EXT-M）と :280 の `| **FR-55**（第 2 回） |` の間に空行（:279）があり、Markdown では :280-281 がヘッダ行の無い別の塊になる（表として描かれない）
- kind: technical
- 提案: :279 の空行を消して 1 つの表にする
- 処置: fixed deep.md — 「要件へ戻すもの」の表の空行を消した

---

観点ごとの該当:

- 観点 1（deep の決定が正典に写っているか）: R3 / R4 / R7 / R11 / R14 / R15
- 観点 2（Scenario が検証可能か）: R6 / R9 / R12 / R13
- 観点 3（置き場の誤り）: R5 / R10。design にだけある観測可能な振る舞いのうち、D12 の優先順（R14）と D16 の形の範囲（R5）と台帳の行の数え方（R10）が spec に無い。spec に実装の名前は入っていない（`Timeline.json` / `Records.json` は取得元のファイル名）
- 観点 4（tasks が検証を持つか）: R1（0.1）/ R2 / R8 / R9 / R13。人間の確認待ち H.1 は Q10 に合わせて「箱の確認待ちに対して印を置く」に直っている
- 観点 5（Story が要件の現在の本文と一致しているか）: `check_chain.py` の観点 8 は通っている。R3 / R15

件数: 16 件（conflict 5 / technical 9 / premise 1 / daily 1）


## 処置のまとめ（呼び出し元が付けた）

16 件すべてに処置を付けた。

| 処置 | 件数 | 指摘 |
|---|---|---|
| 人間へ（第 3 回 Q12） | 1 | R4 |
| 仮決めとして design に置いた | 2 | R7（D9）/ R14（D12） |
| 成果物を直した | 13 | R1 / R2 / R3 / R5 / R6 / R8 / R9 / R10 / R11 / R12 / R13 / R15 / R16 |
