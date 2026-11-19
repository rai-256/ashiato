# CH21 深掘りの問いの独立検査（deep-review）

対象: `openspec/changes/ch21-trial-use-before-verify/deep-questions.json`（Q1〜Q9）・`premises.md`（7 行）・`docs/changes/CH21.md`（依頼。「どこまで」は本人の決定）・`lane-verdict.json`。
手順 1〜8 を一覧を見る前にやり直し、その後で突き合わせた。指摘は R1〜R11。

## 手順ごとの記録

- 手順 1（要件どうしの衝突）: change なので `satisfies` は空。代わりに依頼の「どこまで」の各行と、いまの仕組みを突き合わせた。
  「手順書に載せる」と「issue にしなければ外に出ない」→ 手順書は公開の repo に出る（R1・R2）。
  「落ちても確認バッチは止めない（警告として手順書に書く）」と、`verify-prep.sh` が落ちると manifest ごと手順書から消えること → R7。
  「残すべきものは『違和感』の issue にする」と、ラベル「違和感」の定義（人間が見て・触って感じたもの）→ R10。
  「Windows Sandbox で」と、録画が Sandbox を 1 つしか起こさず、別の Sandbox が動いていれば止まる規則 → R6・R7
- 手順 2（扉の幅）: 該当なし（確かめた範囲: `docs/changes/CH21.md` の front matter は `satisfies: []`・`requires: []` で doors が無い。
  `grep -rln -i "sandbox\|違和感\|確認バッチ" docs/adr/` は `202610030000-worktree-port-allocation.md` の 1 件だけで、内容は port の割り当てで、この change の解釈を縛らない）
- 手順 3（一方通行）: 外に出るものは 3 つある。(a) issue（Q7 が扱う）。(b) 手順書（`docs/verify/<tag>.json` の commit と verify PR の本文。どちらも公開の repo）→ R1・R2。
  (c) 画面の写し・ログを repo に commit する場合は、公開の git 履歴に残る → R1。
  Sandbox は閉じると消えるので、閉じる前に取り出さなかったログ（サーバ・同梱の PostgreSQL・更新係）は取れない（uncaptured。ただし走らせ直せば別の回のものは取れる）→ R1 の提案に含めた
- 手順 4（日常）: 確認バッチは Story が gate を通るたび・fix の後に自動で作り直される。この席はそのたびに走り、Sandbox の窓が本人の画面に出て、Claude の利用枠を使い、作業ツリーの錠を持つ → R4。
  手順書に出る候補の数（Q6 が上限 10 件で扱う）。人間が 1 件ずつ /iwakan に貼る手間（Q7 が扱う）
- 手順 5（既存コードとの突き合わせ）: `verify-prep.sh` は Linux（WSL）のサーバと画面だけを組み、Windows の zip は組まない（Q2 の context は正しい）。
  インストールしたサーバは `127.0.0.1:18700` で待ち受ける（`crates/installer/src/secrets.rs:52`）ので、外の Claude から画面を開くには Sandbox の網と転送が要る（Q3・R3）。
  録画の Sandbox は網を切ってある（`tools/recording/recording.json:115`）→ R3。
  インストーラを Sandbox の中で通したことは一度も無い（ST55 の P19 は CI に委ねた）→ R5。
  manifest は段落ごとに 1 行の文字の note になり、画像は載らない（`.harness2/scripts/verify_checklist.py:583-586`）→ R6
- 手順 6（過去の取りこぼし）: 「後のために残すときは、どう見つけるか・残る先の寿命」→ 画面の写しとログの置き場と寿命が一覧に無い（R1）。
  「送らずに止める・落ちたことを本人がどう知るか、示せないときの向き」→ 失敗の族（R7）。
  「OS のサービスを情報源にする決定は、権限・起動の種別を実機で確かめる」→ インストーラのサービス登録と昇格を Sandbox で見ていない（R5）。
  「合成の負荷ではなく本人の使い方」「初回・大量の場合」→ 合成の書き出しの量（R8）。
  時計の族は Q5 が扱っている。それ以外の観点（移行・検証コマンド・削除の届き方など）は、この change が DB・要件・tasks の検証コマンドをまだ持たないので当たらない
- 手順 7（族の網羅）: Q5（時計を動かすか）は「Sandbox が本物の利用者の環境とどう違うか」という族の 1 つ。兄弟には、網の有無（R3）・zip の届き方（ダウンロードの印。R8）・データの量（R8）・Tailscale が無いこと・Sandbox の遅さ（R8）があり、どれも一覧に無い。
  Q2（どの組み立てか）の兄弟の「版の文字列（正式な版か・開発版か）で、更新の確認の振る舞いが変わる」は R9 に含めた
- 手順 8（前提の確かさ）: `loss` を持つのは Q7 だけで、依る前提（公開の repo）は確かさ 5。
  ただし Q7 の推奨の「何も外に出ない」は表に無い前提（手順書は外に出ない）に乗っていて、その前提は誤り → R2。
  この change の土台の「配る zip のインストーラが Sandbox の中で最後まで通る」が premises に無い → R5。
  Q9 の推奨は、表に無い前提（11-19 の違和感がどの版・どの条件で起きるか、Sandbox で再現するか）に乗っている → R9。
  確かさ 1 の 7 行目（外から中の Edge を操れる）に乗る Q3 は B で、`loss` は無い（指摘しない）
- 範囲（change の第 1 回）: 「何を変えるか」は Q1・Q2・Q3、「どこまでで終わるか」は Q9 が問うている。
  **「何を変えないか」（harness2 の手順書・録画・issue の切り出し・agent の起こし方に手を入れるか）を問う問いが無い** → R6。
  依頼の「どこまで」を聞き直しているのは Q8 の「どこで」の部分 → R11

---

## R1. 画面の写し・ログ・候補の全件をどこに置くかの問いが無い。repo に置けば公開の履歴に残る
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: docs/changes/CH21.md「出すもの: 見たもの（画面の写し・ログ）と『違和感の候補』の一覧」/ `gh repo view --json visibility` → `PUBLIC`（premises.md 1 行目）/ .harness2/scripts/verify_batch.sh:209・214（`git add docs/verify` → 公開の origin へ push）/ .gitignore:36-37（`dist/` は commit しない。`dist/verify-<tag>/` は確認バッチの作業ツリーにだけ残る）/ tools/recording/recording.json:2（録画は `/mnt/c/dev/ashiato2-recordings` に置き、repo には場所だけを書く）/ .harness2/record-run の `wsb_xml`（Sandbox から取り出せるのは書き込みできる置き場 `C:\rec\out` に書いたものだけ。閉じれば中身は消える）/ Q6 の推奨「上限 10 件」（11 件目からの候補の行き先が書かれていない）
- kind: irreversible
- loss: exported
- 提案: A の問いを足す。「見たもの（画面の写し・ログ）と候補の全件をどこに置くか」。選択肢は (1) 録画と同じ手元の置き場に置き、手順書には要約と場所だけ (2) 画面の写しを `docs/verify/` に commit する（公開の git 履歴に残り、消すには履歴を書き直す）。context には、置き場の寿命（作業ツリーの `dist/` は作り直しで残るか）と、Sandbox を閉じる前に取り出すログの一覧（サーバ・同梱の PostgreSQL・更新係）を書く。Q6 の上限を超えた候補も、その置き場に全件を残すと書く
- 処置: fixed Q7（置き場の A の問いを足した。報告は手元に置き、手順書には件数・題・場所だけを推奨。閉じる前に取り出すログと上限を超えた候補の行き先を context に書いた）

## R2. Q7 の推奨「何も外に出ない」は誤り。手順書に載せた候補の文は、公開の repo と PR の本文に出る
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: .harness2/scripts/verify_checklist.py:583-586（manifest の段落 → `note` → `docs/verify/<tag>.json`）/ .harness2/scripts/verify_batch.sh:209・214（その JSON を commit し、公開の origin へ push）/ .harness2/scripts/verify_batch.sh:234（PR の本文に `cat "$manifest"`）/ deep-questions.json Q7 の選択肢 1 の `detail`「何も外に出ない」・`irreversible`「なし（出さない側）」/ Q7 の `why`「AI が書いた文と画面の写しが誰でも読める場所に出る」は、選択肢 1 にも当たる / `gh issue create` は画像を上げられない。issue に「写し」が出るのは、画像を repo に commit したとき（R1）か、手元の場所だけを書いたとき（既存の .harness2/scripts/verify_iwakan.py の `videos()` は手元のパスを書く）
- kind: premise
- loss: exported
- 提案: Q7 の選択肢 1 の `irreversible` を「候補の文は手順書（公開の repo の `docs/verify/` と PR の本文）に出る。issue のように通知は配られない」に直す。選択肢 2・3 の「文と写し」は、R1 の答えに従って「文（と、写しを commit したなら写し）」に直す。`rests_on` に R1 で足す問いを挙げる
- 処置: fixed Q7/Q8（Q7 の推奨の irreversible を「題 1 行ずつは公開の手順書に出る」に直した。issue の問いは Q8 に分け、写しは手元の場所だけが書けると context に書いた。前提の表に手順書が公開に出る行を足した）

## R3. Sandbox の網を開けるか切るかの問いが無い。Q3 の推奨と、拾いたい違和感（更新の確認）の両方がこれで変わる
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: tools/recording/recording.json:115（`"networking": "disable"`）/ .harness2/scripts/record-run の `wsb_xml`（`<Networking>` は設定から決める）/ crates/installer/src/secrets.rs:52（サーバは `127.0.0.1:<port>` で待ち受ける）/ premises.md 7 行目（Q3 の推奨は「デバッグの口を Sandbox の網の口へ転送する」—— 網が開いていないと成り立たない）/ crates/server/src/update_status.rs（画面に出るのは更新係が配布元に問い合わせた結果。網が無いと毎回失敗する）/ tools/recording/scenarios.d/ の ST58 の `waive`（「録画の Windows Sandbox は網を切ってあり」）
- kind: technical
- 提案: B の問いを足す（推奨つき）。網を開ける（Q3 の推奨が成り立つ。更新係が本物の配布元に問い合わせるので、正式な版が出ていればそれを入れに行く）か、切る（更新の確認は毎回失敗する。その失敗は候補から外すと決めておく）か。Q3・Q9 の `rests_on` にこの問いを挙げる
- 処置: fixed Q10（網を開けるか切るかの B を足した。推奨は開ける。Q3 の context から Q10 を指した）

## R4. 席が走る回数と、そのたびの負担（画面に出る窓・利用枠・錠）の問いが無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: .harness2/scripts/verify_batch.sh:13（Story が gate を通るたびに確認バッチを自動で作り直す）/ .harness2/graph/harness_graph/autoverify.py:1-15 / .harness2/graph/tests/test_verify_graph.py:266（ci_fix の後も作り直す。`CI_FIX_ATTEMPTS + 1` 回）/ .harness2/graph/harness_graph/verify.py:108（`verify_batch.sh` の間は確認バッチの作業ツリーの錠を持つ。その中で `verify-prep.sh` が走る）/ .harness2/scripts/record-run の `windows_record`（`cmd /c start <wsb>` で本人のデスクトップに窓を出す。`<MemoryInMB>4096`）/ .harness2/graph/harness_graph/effects.py:111-116（利用上限で Story の thread が落ちた前例）/ Q8 は 1 回の上限（45 分）だけを問うている
- kind: daily
- 提案: B の問いを足す（推奨つき）。「作り直しのたびに走らせる / 束ねた木が前の回と同じなら走らせない / 人間が確認を始める（freeze）前に 1 回だけ」。context に、作り直しの回数の実績、1 回あたりの時間（Q2 の手元での組み立てを含む）、Claude の利用枠、Sandbox の窓が本人の作業中の画面に出て前景を取りうること（`wsb` の CLI で窓を出さずに起こせるか）を書く。Q8 の上限の問いはここにまとめる（R11）
- 処置: fixed Q11（走る回数の B を足した。推奨は zip の中身が変わったときだけ）

## R5. 「配る zip のインストーラが Sandbox の中で最後まで通る」が premises に無い。Sandbox の中で通したことは一度も無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/premises.md
- 根拠: openspec/changes/archive/2026-11-07-st55-server-installer/premises.md:28（P16「サンドボックスの利用者で UAC の確認が出るか」確かさ 1）・:30（P19「仮想アカウントの登録」は Linux では確かめられず CI に委ねた。確かさ 1）/ tools/release/sandbox-probe.sh（PostgreSQL の置き場が無いか Sandbox を起こせなければ `UNAVAILABLE` を書いて rc=0）/ .github/workflows/ci.yml:575-620（インストールの確かめは `windows-latest` の runner だけ）/ premises.md 2 行目の実績は、PC の収集アプリ（利用者の権限で動くもの）を Sandbox に入れた録画だけ
- kind: premise
- 提案: premises に行を足す。「配る zip のインストーラが Sandbox の中で通る（昇格・UAC・`NT SERVICE\ashiato-db` の登録・サービスと更新係の登録）」確かさ 1、確かめ方は下流の最初の Task。通らなかったときにこの change がどこで終わるか（インストールで止まったことを候補にして終える・サービスを使わない形に替える）を Q9 の族として問うか、P にして台帳に載せる。昇格の確認の画面は別のデスクトップに出て Edge からは押せないので、Q3 の context にも書く
- 処置: fixed premises.md（インストーラが Sandbox で通るかの行を確かさ 1 で足した。下流の最初の Task で試す。Q2 の context に、通らなければ警告にすると書いた。loss のある決定は依らない）

## R6. 範囲のうち「何を変えないか」を問うていない。harness2 に手を入れずに済むかで、候補の見せ方と Sandbox の取り合いが決まる
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: lane-verdict.json の `declared.paths`（`tools/**`・`docs/**` だけ。harness2 の `scripts/` は含まない）/ .harness2/scripts/verify_checklist.py:583-586（manifest は段落ごとに 1 行の文字の note になる。画像も問いも作れない）/ .harness2/scripts/record-run の `windows_record`（「別の Windows Sandbox が動いている」なら止まる。この席が Sandbox を閉じずに残すと、続く録画が落ちる。本人が自分で起こした Sandbox が動いていれば、この席も同じ規則で止まる必要がある）/ .harness2/graph/harness_graph/effects.py:227（agent の起こし方と利用上限の扱いはグラフの `agent()` が持つ。`verify-prep.sh` から直に `claude -p` を呼ぶと、その外で走る）/ docs/changes/CH21.md の「壊してはいけないもの」は空 / Q6 の推奨は「各候補に画面の写し」を添える、Q8 の選択肢 2 は「harness2 に足すのは範囲を越える」と書いている
- kind: technical
- 提案: B の問いを足す（推奨つき）。「harness2（手順書の作り方・録画・issue の切り出し・agent の起こし方）には手を入れない。候補は manifest の文字と手元の場所で見せる」か「手順書に画像や候補の問いを出せるように harness2 にも足す」か。答えを「壊してはいけないもの」（Story ごとの違和感の問い、この席が落ちても `run.sh` と manifest が手順書に載ること、録画の Sandbox の順番）に写す
- 処置: fixed Q12（harness2 に手を入れるかの範囲の B を足した。推奨は入れない）

## R7. 「落ちても止めない、警告として手順書に書く」が成り立たない形がある。失敗の族が一覧に無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: tools/verify-prep.sh:14（`set -euo pipefail`）/ .harness2/scripts/verify_batch.sh:180-186（`verify-prep.sh` が 0 以外で終わると、manifest ごと捨てて続ける。手順書から起動の手順も警告も消え、PR の本文には「無いか落ちた」としか出ない）/ Q8 の推奨の `detail`「手順書に必ず結果か警告が載る」/ .harness2/scripts/record-run の `windows_record`（Sandbox が既に動いていれば止まる）/ .harness2/graph/harness_graph/effects.py:111-116（利用上限は数時間戻らない）
- kind: technical
- 提案: Q8 の context に、落ち方の族を並べる。Sandbox が既に動いている（本人のものかもしれない）・手元の Windows で zip が組めない（Q2）・インストールが通らない（R5）・Claude が利用上限で止まる・時間の上限を越える・Sandbox を閉じられない。どれでも `verify-prep.sh` は 0 で終わり、manifest に警告の段落を書く、と書く。閉じられなかった Sandbox が後の録画を止めないことも書く。これは失わない側が安いので P にしてよいが、一覧には載せる
- 処置: fixed Q9（落ち方の族を context に挙げ、どれも準備を落とさず冒頭の警告の段落にすると書いた。扉を開けたままの既定として design に残す）

## R8. 「本物の利用者と何が違うか」の族が Q5 の時計だけ。zip の届き方・データの量・環境の違いが一覧に無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: docs/changes/CH21.md「普通の利用者として」/ .harness2/scripts/record-run の `wsb_xml`（物は読みだけの置き場（MappedFolder）で渡す。ブラウザで落とした zip に付くダウンロードの印（Zone.Identifier）が無いので、本物の利用者が最初に見る SmartScreen・「ブロックの解除」が出ない）/ Q5 の推奨は「合成の書き出し」で、量を決めていない（本人は何年分もの書き出しを入れる。`tools/verify-prep.sh` の偽データにも `SEED=normal|max|empty` の量の選択がある）/ crates/installer/src/commands.rs:130（Tailscale の設定を促す文。Sandbox には Tailscale が無い）/ Q6 の推奨「説明なしに 10 秒を超えて待たされる」（Sandbox は 4096 MB で本人の PC より遅い）
- kind: technical
- 提案: Q5 を「数日分の記録」から「本物の利用者との違いをどう扱うか」に広げるか、兄弟の B の問いを足す。(1) zip にダウンロードの印を付けて渡すか (2) 合成の書き出しの量（少し・本人と同じくらい）(3) Sandbox に元から無いもの（Tailscale・ブラウザの履歴・網）による候補を「既知の違い」として外すか。Q6 の基準に「既知の違い」を書き、`rests_on` に挙げる
- 処置: fixed Q5/Q6（兄弟 —— SmartScreen・Tailscale・データの量・Sandbox の遅さ —— を Q5 の context に挙げ、候補の「Sandbox だから」の注記で扱うとした。待ち時間の候補には Sandbox で測ったと添える）

## R9. Q9 の推奨（11-19 の違和感が残る版で拾えること）が、表に無い前提に乗っている
- 成果物: openspec/changes/ch21-trial-use-before-verify/premises.md
- 根拠: deep-questions.json Q9 の推奨の `detail`「その違和感が残る版の zip に当てて拾えること」/ premises.md に、11-19 の違和感がどの版で起き、何が原因か（網・配布元に正式な版が無い・版の文字列）の行が無い / `gh issue list --label 違和感` に 11-19 のものは無い（原因の記録も無い）/ 更新の確認の結果は網と版に依る（R3）/ 筋書きなしの Claude が毎回同じ所を触るとは限らないので、「拾えること」の機械の確かめが落ちたり通ったりしうる
- kind: premise
- 提案: premises に行を足す。「11-19 の違和感は、版 X の zip を Sandbox に入れると（網を開けて / 切って）再現する」確かさ 1、確かめ方は下流の最初の Task。Q9 の context には、その版の zip を確かめの間ずっと手に入れられるか（配布元の正式な版か、手元に残すか）と、拾えないときの判定（何回走らせて何回拾えばよいか）を書く。Q9 は B のままでよい
- 処置: fixed Q13・premises.md（前提の行を確かさ 1 で足し、推奨を「実地で 1 回走り結果か警告が載る。11-19 の版の結果は記録するが判定に使わない」に変えた。拾えることまで求める案は選択肢 2 に残した）

## R10. 機械が挙げた候補を「違和感」の issue にすると、ラベルの意味（人間が見て・触って感じたもの）が変わる。Q7 の選択肢に書かれていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: .harness2/scripts/verify_iwakan.py:28（`LABEL_DESC = "人間が見て・触って感じた違和感や直したいこと。…次にその capability を触る Story の deep か fix/ で拾う"`）/ `gh label list` の「違和感」の説明「確認バッチで人間が挙げた違和感」/ .harness2/scripts/verify_iwakan.py の docstring（2026-10-04 に、AI の fix が issue を作ろうとして auto mode に「外への公開」として断られた）/ Q7 の選択肢 2・3
- kind: conflict
- 提案: Q7 の選択肢 2・3 の `detail` に「ラベル『違和感』の issue に機械の候補が混ざり、後の Story の deep が拾う対象になる。分けるなら別のラベル」と書き足す。作るのは agent ではなく台本（auto mode に断られる）と context に書く
- 処置: fixed Q8（ラベルの意味を context に書き、自動の案は別ラベル「違和感の候補」と、同じラベルの案に分けた）

## R11. Q8 の「確認バッチのどこで」は依頼で決まっている。幅は時間の上限だけ
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: docs/changes/CH21.md「走らせるのは確認バッチの前（差し込み口の `tools/verify-prep.sh` から呼ぶ想定）」「落ちても確認バッチは止めない」/ Q8 の選択肢 2 は Q8 自身が「この change の範囲を越える」と書き、選択肢 3 は「確認バッチ全体が止まる」で依頼の「止めない」に反する
- kind: defer
- 提案: Q8 から「どこで」を外し、上限の数字は R4 の回数の問いにまとめるか、P として台帳に載せる（上限は失うものが無い）。人間の時間を使わせない
- 処置: fixed Q9（「どこで」は問わず、時間の上限だけの B にした。範囲を越える案と依頼に反する案を外した）
