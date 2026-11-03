# 深掘り（第 1 回）

> 推奨の答えで先に仕様と GO 資料を書いた（2026-10-20 の流れ）。2026-11-02 に本人の答え（`deep-answers-1.txt`）を受けた。
> Q1〜Q9 はどれも推奨の選択肢のままで補足が無い（本人は推奨から変えなかった）。仕様への合意（QA）は「合意する」。

### Q1. 追加の許可の文の中身と、正本の言葉

- 問い: 本体の AGPL-3.0 に足す追加の許可を、下の英文の案で正本にしますか。
- 答え: (推奨のまま: この英文の案を正本にする。日本語の訳は README に「参考訳（正本は英語）」として置く)

### Q2. 許可が受ける Google のライブラリを、何で数えるか

- 問い: 追加の許可が受ける「Google Play 開発者サービスのライブラリ」を、どの範囲で数えますか。
- 答え: (推奨のまま: `play-services-location` と、それが引く Google の部品（今は `-base`・`-basement`・`-tasks`）だけを名指しする)

### Q3. 正式な鍵を置く前に、印から鍵を読める経路をどう閉じるか（ST52 から申し送られた重大な判断）

- 問い: 正式な署名の鍵を GitHub の Environment に置く前に、main に入っていない commit に印（`v*`）を打って鍵を読む経路を、どう閉じますか。
- 答え: (推奨のまま: Environment `release` に本人を必須の承認者として付け、エージェントの資格情報を管理と承認の権限の無いものに替えてから、鍵を置く)

### Q4. 最初の本物の公開（印・正式な鍵での署名・本人のサーバの切り替え）をいつ見るか

- 問い: 最初の本物の印を打って正式な鍵で署名・公開し、本人のサーバをその版へ移すのを、ST53 の確認バッチの中で見ますか、ST53 が main に入った後に本人が行いますか。
- 答え: (推奨のまま: ST53 が main に入った後に本人が行う。ST53 の確認では下書きのリリースで許諾と守りを見る)

### Q5. ソースの先頭の許諾の表示（SPDX）

- 問い: ソースの先頭と `Cargo.toml`・`web/package.json` の許諾の表示を、追加の許可が分かる形に書き換えますか。
- 答え: (推奨のまま: 506 ファイルと `Cargo.toml`・`web/package.json` を `AGPL-3.0-only WITH AdditionRef-ashiato-play-services` にし、許可の文を `LICENSES/AdditionRef-ashiato-play-services.txt` にも置く)

### Q6. 携帯の収集アプリに許諾を読む画面を足すか

- 問い: 携帯の APK の中に入れる許諾の一覧を、アプリの画面からも開けるようにしますか。
- 答え: (推奨のまま: 画面は作らない（APK の中に置くだけ）。Play ストアへ出す前に足すことを、要件の未決として残す（/requirements から足す。受ける Story はまだ無い）)

### Q7. 開発版の配布物で、対応するソースの場所をどう示すか

- 問い: 毎日入れ替わる開発版の配布物では、対応するソースの場所を何で示しますか。
- 答え: (推奨のまま: 正式版・ベータ版は印 `v*` で、開発版は commit（40 桁）で、リポジトリの場所を示す)

### Q8. 確認バッチの準備が本人の携帯に開発用の鍵の版を入れうる件（ST52 からの申し送り K1）

- 問い: 確認バッチの準備（`tools/verify-prep.sh`）が、繋いだ最初の端末に開発用の鍵の APK を入れてしまう件を、この Story で直しますか。
- 答え: (推奨のまま: この Story で直す（Task を 1 つ足す）。直るまでは、ST53 の確認の手順に「携帯を繋がない」と書く)

### Q9. サーバの画面にソースの場所と許諾を出すか

- 問い: サーバの画面（ブラウザで開くもの）に、ソースの場所（同じ版の印のリポジトリ）と許諾へのリンクを出しますか。
- 答え: (推奨のまま: 出さない。一般向けの配布の前に足すことを、要件の未決として残す（/requirements から足す。受ける Story はまだ無い）)

## 第 2 回（2026-11-03 起票）—— 仮で進めた決定のうち、別の席が否とした 6 件

> merge gate（PR #230）の前に、別の席（`provisional-review.md`）が仮の決定 9 件を確かめ直し、D4・D5・D7・D8・D9・D16 を否とした（D6・D10・D11 は確か）。
> 6 件とも台帳から外し（`provisional.py resolve`）、design の見出しの（仮）を外して「未決（Q<n> の答え待ち）」にした。
> Q10・Q12・Q14 が本体で、Q11・Q13・Q15 はそれぞれの答えに乗る（rests_on の漏れだけで否になったもの）。
> 問いは `deep-questions-r2.json`、渡した形は `docs/briefs/ST53-deep-r2.html`。
> 2026-11-03 に本人の答え（`deep-answers-2.txt`）を受けた。6 問とも本人がはっきり選び、どれも問いの本文の「推奨」と同じ選択肢で補足は無い。
> 第 2 回の答えは推奨を先に仕様へ書いていないので、6 問とも `spec --answers Q10,Q11,Q12,Q13,Q14,Q15` で design の D4・D5・D7・D8・D9・D16 の「未決（Q<n> の答え待ち）」へ写す（`deep_draft.py later`）。

### Q10. 移行の 2 ファイルの許諾の表示を、最初の印の前に書き換えるか（design D4）

- 問い: 移行のうち表示が `AGPL-3.0-only` のままの 2 つ（と戻し手順の 1 つ）を、最初の印 `v*` を打つ前に `AGPL-3.0-only WITH AdditionRef-ashiato-play-services` に書き換えるか
- 推奨: 書き換えない（例外の一覧に名前で載せる）。追加の許可が要るのは APK に入るコードで、移行は APK に入らない。最初の印から後はこの 2 つの表示を直せないことを受け入れる
- 否の根拠: Q4 の答えで ST53 が main に入った直後に最初の印を打つと、`tools/check-released-migrations.sh:52-54` が前進側の中身の差を落とすので、「いつでも書き換えて例外から外せる」は成り立たない。配った版のソースとサーバの実行ファイル（`crates/server/build.rs:114`）に WITH の無い表示が残る（exported）。書き換える側にも指紋の警告（`migration_ledger.rs:518-530`）の費用がある
- 答え: 書き換えない（AGPL-3.0-only のまま、表示の検査の例外に名前で載せる）

### Q11. 表示の検査の例外の一覧と見る窓（design D5。Q10 に乗る）

- 問い: 表示の検査を先頭 25 行の `SPDX-License-Identifier:` だけを見て表示の無いファイルは見ない形にし、例外の一覧は Q10 の答えに従って決めるか
- 推奨: そうする（Q10 が「書き換えない」なら例外は gradle の wrapper の 2 つと移行の 3 つ、書き換える側なら wrapper の 2 つだけ）
- 否の根拠: 例外の一覧が D4 の決定そのものを含むのに rests_on に D4 が無かった。台帳の文は「先頭 5 行」のまま古かった。検査の範囲そのものには失うものは見つかっていない
- 答え: 先頭 25 行・表示の無いファイルは見ない・例外は Q10 に従う

### Q12. 携帯の依存の POM に許諾が 2 つ以上並ぶとき、どう読むか（design D7）

- 問い: 1 つの部品の POM の `<licenses>` に名前が 2 つ以上並んだとき、OR（どれか 1 つが許可の一覧にあれば通す）と AND（全部が許可の一覧にあるか名指しに当たるときだけ通す）のどちらで読むか
- 推奨: AND で読む。あわせて「POM は cache に入っている」を「`@pom` を解決して取り寄せ、`--offline` で取れなければ組み立てを落とす」に直す
- 否の根拠: OR は通す側（公開する側）の読み方で、keeps の「落ちる側に倒れる」と合わない。両方が掛かる部品を通すと名指し（D2）の外のプロプライエタリが APK に入り、配った後に戻らない（exported）。今の 55 部品で 2 つ以上並ぶものは 0。POM が gradle の cache にあるという前提も誤り
- 答え: AND で読む（全部が許可の一覧にあるか、名指しに当たるときだけ通す）

### Q13. 依存の一覧の携帯の部品（design D8。Q12 に乗る）

- 問い: 依存の一覧（`THIRD-PARTY-NOTICES.txt`）の携帯の部品を、Q12 の答えで決まる検査の task の出力から作り、ほかは D8 の案のままにするか
- 推奨: Q12 の答えに従う（検査と一覧が同じ部品の集合を見る）
- 否の根拠: 携帯の入力は D7 の task の出力（design D8）で D7 に乗るのに rests_on に D7 が無かった。中身（多めに入れる側に倒す）には失うものは見つかっていない
- 答え: Q12 の答えに従う（携帯の部品は検査と同じ出力から。ほかは D8 の案のまま）

### Q14. PR の組み立ての SOURCE.txt に、どの commit を書くか（design D9）

- 問い: PR の組み立ての配布物の `SOURCE.txt` に、合成の merge commit（`refs/pull/<n>/merge`。今の checkout の既定）と PR の head の commit のどちらを書くか
- 推奨: PR の組み立てでは head の commit（`github.event.pull_request.head.sha`）を書き、組み立ての印との比べはその場合だけ外す。確認バッチ 11.1 で閉じた PR の head を取れるかを確かめる
- 否の根拠: 合成の merge commit（`.github/workflows/ci.yml:355` の checkout は `ref` を渡さない）を PR を閉じた後も取れるかは P14 の確かさ 2 のまま。確かめ（確認バッチ 11.1）より前に、90 日誰でも落とせる PR の成果物が出る。取れなければ落とされた写しのソースの場所は直せない（exported）
- 答え: PR の組み立てでは head の commit を書き、組み立ての印との比べはその場合だけ外す

### Q15. 開発版と PR の組み立てを commit で示す（design D16。Q14 に乗る）

- 問い: 開発版と PR の組み立てを FR-127 の「同じ印」ではなく 40 桁の commit で示すとき、PR の組み立てで書く commit は Q14 の答えに従うか
- 推奨: Q14 の答えに従う（開発版は main の commit、PR は Q14 で選んだ commit）。FR-127 の ★ 2026-11-02 補足はどの commit かを design D9 に預けているので、要件の本文は変わらない
- 否の根拠: 「組み立ての commit で示す（D9）」のとおり D9 に乗るのに、rests_on は Q7 だけだった。Q7（答え済み）との向きは合っている
- 答え: Q14 の答えに従う（開発版は main の commit、PR は Q14 で選んだ commit）

## 第 3 回（2026-11-03 起票）—— Q14 の答えを写した後に見つかった前提

> `spec --answers Q10〜Q15` の spec-review（`review/spec.md` R17・R18）が、Q14 の問いに書いていなかった帰結を見つけた（premise）。
> 問いは `deep-questions-r3.json`、渡す形は `docs/briefs/ST53-deep-r3.html`。答えは `spec --answers Q16` で design D9・D16・D17（仮）と spec・tasks に写す。

### Q16. PR の配布物を head から組むか（Q14 の前提。review/spec.md R17・R18）

- 問い: PR の組み立て（`package-linux`・`package-windows`）を、合成の merge commit からではなく PR の head の commit そのものから組むようにするか
- 推奨: head から組む（checkout に `ref: ${{ github.event.pull_request.head.sha || github.sha }}`）。組んだソース・組み立ての印・`SOURCE.txt` が一致し、比べはすべての組み立てで残る。D17（仮）は要らない
- 出所: R17（`SOURCE.txt` の head と組んだ木が、その時の main の差だけ食い違う。FR-127 の対応するソース）・R18（D17 の前提「PR と main への push を分けられない」は誤り。`ci.yml:34`）
- 答え: (推奨のまま: PR の組み立てを head から組む（checkout に head の commit を渡す）)
  - 本人は推奨の選択肢を明示して選んだ（`deep-answers-3.txt`）。補足なし

## 影響の判断（第 1 回の合意）

重大な IMP への本人の答え（`deep-answers-1.txt` の逐語）。閉じたか・リリースまで残るかは `agreement.json` の `impacts` が持つ。

- IMP-01 [印を push できる人が main を通さずに正式な鍵を読めるかもしれない（ST52 から）] -> 受け入れて進める（リリースの前に判断し直す）
  - 補足: なし。リリースの前の判断し直しは go.json の `if_accepted.recheck`（本人・ST54 の上流の合意まで）
- IMP-02 [本人の管理者の資格情報がエージェントと同じ OS の利用者に残ると、資格情報を替えても分けられない] -> 受け入れて進める（リリースの前に判断し直す）
  - 補足: なし。リリースの前の判断し直しは go.json の `if_accepted.recheck`（本人・ST54 の上流の合意まで）

## 仕様への合意（第 1 回）

- QA [仕様への合意] -> 合意する（`agreement.json` に機械が記録する。違和感の補足は無い）

## 影響の判断（第 2 回の合意）

重大な IMP への本人の答え（`deep-answers-4.txt` の逐語。問いは `deep-questions-agree-2.json`）。閉じたか・リリースまで残るかは `agreement.json` の `impacts` が持つ。

- IMP-01 [印を push できる人が main を通さずに正式な鍵を読めるかもしれない（ST52 から）] -> 答え無し（答えの欄が空だった）
  - 補足: なし。第 1 回は「受け入れて進める（リリースの前に判断し直す）」だったが、第 2 回では答えが無い。読んだだけでは閉じない（`agreement.json` は `closed: false`）
- IMP-02 [本人の管理者の資格情報がエージェントと同じ OS の利用者に残ると、資格情報を替えても分けられない] -> 受け入れて進める（リリースの前に判断し直す）
  - 補足: なし。リリースの前の判断し直しは go.json の `if_accepted.recheck`（本人・ST54 の上流の合意まで）

## 仕様への合意（第 2 回）

- QA [仕様への合意] -> 合意する（補足なし）。ただし IMP-01 が未回答のため、合意は成立していない（`agreement.json` の `status: rejected`）

## 仕様への違和感（第 2 回の合意）

- IMP-01 に答えが無い（`deep-answers-4.txt`: `IMP-01 [...]` の答えの欄が空）。違和感の補足は無く、仕様の中身への指摘ではない。
  仕様は直さずに GO 資料を作り直し、IMP-01 をもう一度聞く
  → 対応: 仕様（proposal / specs / design / tasks）は直さない。理由: 指摘は仕様の中身ではなく IMP-01 の答えが無いことだけで、
    QA は「合意する」、IMP-02 は「受け入れて進める（リリースの前に判断し直す）」で閉じている。IMP-01 は消さず・重大度を下げず、
    材料（`if_accepted`・`recheck`）も第 2 回と同じまま、次の合意で IMP-01 だけに答えを求める。
    spec の段の検査（`openspec validate --strict`・`evidence.py lint`・`review_triage.py`・`spec_overlap.py check`）は直さずに rc=0（2026-11-03）

## 影響の判断（第 3 回の合意）

重大な IMP への本人の答え（`deep-answers-5.txt` の逐語。問いは `deep-questions-agree-3.json`）。閉じたか・リリースまで残るかは `agreement.json` の `impacts` が持つ。

- IMP-01 [印を push できる人が main を通さずに正式な鍵を読めるかもしれない（ST52 から）] -> 答え無し（答えの欄が空だった）
  - 補足: なし。第 2 回に続き、第 3 回でも答えが無い（第 1 回は「受け入れて進める（リリースの前に判断し直す）」）。読んだだけでは閉じない（`agreement.json` は `closed: false`）
- IMP-02 [本人の管理者の資格情報がエージェントと同じ OS の利用者に残ると、資格情報を替えても分けられない] -> 受け入れて進める（リリースの前に判断し直す）
  - 補足: なし。リリースの前の判断し直しは go.json の `if_accepted.recheck`（本人・ST54 の上流の合意まで）

## 仕様への合意（第 3 回）

- QA [仕様への合意] -> 合意する（補足なし）。ただし IMP-01 が未回答のため、合意は成立していない（`agreement.json` の `status: rejected`）

## 仕様への違和感（第 3 回の合意）

- IMP-01 に答えが無い（`deep-answers-5.txt`: `IMP-01 [...]` の答えの欄が空）。違和感の補足は無く、仕様の中身への指摘ではない。
  第 2 回・第 3 回と 2 回続けて IMP-01 だけが未回答で、IMP-02 と QA には答えがある。問いの JSON（`deep-questions-agree-3.json`）の
  IMP-01 と IMP-02 は同じ形（選択肢 2 つ）で、問いの側の違いは見つけていない。仕様は直さずに IMP-01 をもう一度聞く
  → 対応: 仕様（proposal / specs / design / tasks）は直さない。理由: 指摘は仕様の中身ではなく IMP-01 の答えが無いことだけで、
    QA は「合意する」、IMP-02 は「受け入れて進める（リリースの前に判断し直す）」で閉じている。IMP-01 は消さず・重大度を下げず、
    材料（`if_accepted`・`recheck`）も変えない。問いの HTML を `ask_wizard.py` で組み直して見ても、GO 資料の直後に IMP-01 → IMP-02 → QA
    の順で並び、IMP-01 にも 2 つの選択肢があり、問いの側の欠けは見つけていない。次の合意でも IMP-01 だけに答えを求める。
    spec の段の検査（`openspec validate --strict`・`evidence.py lint`・`review_triage.py`・`spec_overlap.py check`）は直さずに rc=0（2026-11-03）

## 影響の判断（第 4 回の合意）

重大な IMP への本人の答え（`deep-answers-6.txt` の逐語。問いは `deep-questions-agree-4.json`）。閉じたか・リリースまで残るかは `agreement.json` の `impacts` が持つ。

- IMP-01 [印を push できる人が main を通さずに正式な鍵を読めるかもしれない（ST52 から）] -> 受け入れて進める（リリースの前に判断し直す）
  - 補足: なし。第 2 回・第 3 回は未回答だったが、第 4 回で答えがあった（第 1 回と同じ答え）。リリースの前の判断し直しは go.json の `if_accepted.recheck`（本人・ST54 の上流の合意まで）
- IMP-02 [本人の管理者の資格情報がエージェントと同じ OS の利用者に残ると、資格情報を替えても分けられない] -> 受け入れて進める（リリースの前に判断し直す）
  - 補足: なし。リリースの前の判断し直しは go.json の `if_accepted.recheck`（本人・ST54 の上流の合意まで）

## 仕様への合意（第 4 回）

- QA [仕様への合意] -> 合意する（補足なし）。IMP-01・IMP-02 とも答えがあり、合意は成立した（`agreement.json` の `status: accepted`）。
  違和感は無く、仕様（proposal / specs / design / tasks）を直すものは無い。要件・Story へ戻すものも無い

## 要件へ戻すもの（record の段で戻した。2026-11-02）

- FR-127 に ★ 2026-11-02 補足: 「同じ印」で示すのは正式版・ベータ版だけ、開発版と PR の組み立ては 40 桁の commit で示す（Q7・review/spec.md R14・D16（仮））。許諾は配布物の中にだけ置き、画面からは開けない（Q6・Q9）
- §5「配布の方針」に ★ 2026-11-02 追加: C-01 の許諾を読む画面（Q6。期限: Play ストアへ出す前）と、V-01 のソースの場所と許諾へのリンク（Q9。期限: 一般向けの配布の前）を `[NEEDS CLARIFICATION]` で残した。受ける Story はまだ無い（/requirements から足す）
- 整合の確認（`docs/audit/2026-11-02-st53-consistency.md`。14 件）の処置で、FR-127 ★ の範囲（main への push も commit・どの commit かは D9（仮））と §5 の「NFR-26 の検査が止める」の範囲（K1）を直し、`docs/release.md`（鍵を置く前に IMP-01・IMP-02 を判断し直す。C1）・`docs/handoff/ST53.md`・`docs/production-prep.md` を答えに合わせた。design・spec の「答え待ち」の印（C11）は合意に結び付くので、この段では直さない

## 要件へ戻すもの（record の段で戻した。2026-11-03・第 2 回）

- FR-127 に ★ 2026-11-03 補足: PR の組み立ての `SOURCE.txt` は PR の head の commit、開発版と main への push は main の commit（Q14・Q15。★ 2026-11-02 の「D9（仮）」が決まった）。PR の head も閉じた後・push し直した後に取れるかは確認バッチで確かめる（整合の確認 `docs/audit/2026-11-03-st53-r2-consistency.md` の C1）。SPDX の例外は §5 を見よ（C7）
- §5「配布の方針」に ★ 2026-11-03 補足: 表示の検査は先頭 25 行・表示の無いファイルは見ない・移行の 3 つと gradle の wrapper の 2 つを例外にする。最初の印から後に直せないのは移行の前進側の 2 つだけ（Q10・Q11。K1・C8）
- `docs/production-prep.md` の「本体のライセンス」の行に、例外の 5 つと、前進側の 2 つは最初の印から後に直せないことを足した（C6）
- Q12・Q13 は要件を変えない（NFR-26 ★「許可の外のプロプライエタリな依存が入ったら検査が止める」と AND の読みは向きが同じ）

## 要件へ戻すもの（record の段で戻した。2026-11-03・第 3 回）

- FR-127 に ★ 2026-11-03 補足（第 3 回）: PR の組み立ては PR の head の commit そのものから組む（GitHub が作る merge の commit からは組まない）。`SOURCE.txt` の head と組んだソースが一致する（Q16）。整合の確認 `docs/audit/2026-11-03-st53-r3-consistency.md`（4 件）の K2 で、合わせた木で走る範囲を job で限った。C1・K1 は spec --answers Q16 へ、C2 は leave
- 仮の決定 D17 は外す（R18）。要件の本文には D17 の名が無いので要件は変えない

## spec の段へ渡すもの（record の段から。2026-11-03・第 3 回）

`spec --answers Q16` で写す: design D9（head から組む・比べはすべての組み立てで残す）・D16・D17（仮）を外し `provisional.py resolve`、spec の PR の組み立ての文と Scenario、tasks の `package-linux`・`package-windows` の checkout と `check-artifacts.sh` の比べ（Q14 で「外す」とした分岐を戻す）、go.json の walkthrough と IMP-06、premises P14

整合の確認 `docs/audit/2026-11-03-st53-r3-consistency.md` が見つけた、一緒に直すもの:

- C1（第 3 回の整合の確認）: tasks 11.1（`tasks.md:229`）の「組み立ての印（合成の merge commit）と違うことを見る」を「同じことを見る」に反転する。design の Risks の合成の merge commit の項（`design.md:240` 付近）を外す。`proposal.md:71` に checkout の `ref` を足す。`provisional.jsonl` の D17 は `provisional.py resolve` で外す
- K1: 配布物を検査する `package-artifacts`（`ci.yml:392-400`）も同じ `ref` で checkout するか、期待する版番号と `LICENSE` を配布物を組んだ木から取るかを D9 に書き、tasks 7.1 の検証を合わせる（今のままだと、枝を切った後に main が版番号か `LICENSE` を変えると PR の CI が誤って落ちる）
- K2: `package-linux` の job の中の自己試験（`ci.yml:366-370`）と、足す許諾・表示の検査が head の木で走ることを D9 に書く（FR-127 ★ 第 3 回はそう狭めた）
- 未確認（同じ監査）: fork の PR で `ref: head.sha` を `fetch-depth: 1` で取れるか・detached HEAD で組んだ組み立ての印に `.dirty` が付かないか —— tasks の検証か premises に置く

## Story へ戻したもの（record の段。2026-11-03・第 2 回）

- `docs/stories/ST53.md` の完了の判定: `SOURCE.txt` を commit で示す組み立てを「それ以外の組み立て —— 開発版・main への push・PR」にした（C9。`stories.json` → `make_story.py`）

## Story へ戻したもの（record の段。2026-11-02）

- `docs/stories/ST52.md` の完了の判定: 「本物の公開・正式な鍵での署名・本人のサーバの切り替えは ST53 の確認で見る」を、Q4 の答え（ST53 が main に入った後に本人が行う）に書き直した（`stories.json` → `make_story.py`）
- `docs/stories/ST53.md` の完了の判定（review/spec.md R13・R9）: SPDX の文に例外の一覧（移行の 3 つ・gradle の wrapper の 2 つ。D4（仮）・D5（仮））を除くと添え、「正式な鍵を読む公開は本人の承認を経る」「`SOURCE.txt` がチャンネルの決まりどおり」「確認バッチの準備はエミュレータにだけ入れる」を足した。`doors` は satisfies から機械で決まるので 35 のまま、扉 34 の守りを引き受けた理由は `docs/stories/INDEX.md` の訂正（2026-11-02）に書いた

## 答えと仮の決定の関係

- 第 2 回（2026-11-03）: D4 ← Q10（書き換えない。移行の 3 つは例外の一覧に名前で載せ、最初の印から後は表示を直せないことを受け入れる）・D5 ← Q11（先頭 25 行・表示の無いファイルは見ない・例外は wrapper の 2 つと移行の 3 つ）・D7 ← Q12（AND。`@pom` を解決して取り寄せ、`--offline` で取れなければ落とす）・D8 ← Q13（携帯の部品は D7 の task の出力から）・D9 ← Q14（PR は head の commit `github.event.pull_request.head.sha`。組み立ての印との比べはその場合だけ外す）・D16 ← Q15（開発版は main の commit、PR は Q14 の commit）。どれも本人の決定で、（仮）には戻さない

- Q5 は推奨のまま（「506 ファイル」）で補足が無い。D4（第 1 回の時点では仮。第 2 回の Q10 で本人が決めた）（移行の 3 ファイルは表示を書き換えない。書き換えるのは 503）は、答えの上に乗る例外として残す（D4 の「Q5 で本人が全部の書き換えを補足したら外す」に当たらない）
- 第 3 回（2026-11-03）: D9 ← Q16（PR の `package-linux`・`package-windows` は checkout に `ref: ${{ github.event.pull_request.head.sha || github.sha }}` を渡して head から組む。`SOURCE.txt`・組み立ての印・組んだソースが同じ head を指すので、Q14 の「組み立ての印との比べはその場合だけ外す」は要らなくなり、比べはすべての組み立てで残す）・D17（仮）は外す（R18。前提が誤り）。本人の決定で、（仮）には戻さない
- Q7 は推奨のまま。D16（第 1 回の時点では仮。第 2 回の Q15 で本人が決めた）は答えと一致した（反転条件「全部、印で示す」に当たらない）

## 問いの出所（review/deep.md の指摘）

問いを人間に渡す前の独立検査（`review/deep.md`）で escalated にした指摘が、どの問いに入ったか。record の段で答えを書き換えても、この節は残す。

- Q1 ← R3（許可の文の中身と 3 つの分岐）
- Q2 ← R2（許可が受ける範囲を何で数えるか）
- Q3 ← R4（管理者の資格情報で承認と規則を書き換えられる）
- Q4 ← R1（本物の公開を確認の中で見るか後か）
- Q6 ← R5（PR の成果物にも同じものが入る）・R7（Rust の標準ライブラリと aar の中の第三者）・R10（全文と著作権の表示を C にした）
- Q7 ← R6（開発版の動く印）
- Q10 ← provisional-review.md の D4（否）・review/deep.md R11・review/spec.md R9
- Q11 ← provisional-review.md の D5（否）・review/deep.md R8・review/spec.md R3
- Q12 ← provisional-review.md の D7（否）・review/deep.md R7
- Q13 ← provisional-review.md の D8（否）・review/deep.md R7
- Q14 ← provisional-review.md の D9（否）・review/deep.md R5・R6・review/spec.md R15（premises P14）
- Q15 ← provisional-review.md の D16（否）・review/spec.md R14
- Q16 ← review/spec.md R17・R18

## record の段へ渡すもの（spec の段から。答えに合わせて行う）

- Story へ: `docs/stories/ST52.md` の完了の判定（「本物の公開・正式な鍵での署名・本人のサーバの切り替えは ST53 の確認で見る」）を Q4 の答えに合わせて書き直す（`stories.json` → `make_story.py`）
- Story へ（review/spec.md R13）: `docs/stories/ST53.md` の完了の判定に「正式な鍵を読む公開は本人の承認を経る（`check-guards.sh approvals` が通る）」「配布物の `SOURCE.txt` がチャンネルの決まりどおり」「確認バッチの準備はエミュレータにだけ入れる」を足し、doors に 34 を足すか、申し送り（ST52 IMP-01・K1）から引き受けた理由を INDEX の訂正の節に書く
- 要件へ（review/spec.md R14）: FR-127 に ★ で「開発版・PR の組み立ては、組み立ての commit で示す（印 `nightly` は動くため）」を補う（Q7 の答えに合わせて）
- 要件へ: Q6（携帯の許諾を読む画面。Play ストアへ出す前）と Q9（サーバの画面のソースの場所。一般向けの配布の前）を、答えに合わせて未決として残す

## spec の段へ渡すもの（record の段から。2026-11-03・第 2 回）

`spec --answers Q10,Q11,Q12,Q13,Q14,Q15` で写すときに、整合の確認 `docs/audit/2026-11-03-st53-r2-consistency.md` が走っている change の中に見つけた食い違いも一緒に直す:

- C2（Q12）: `specs/distribution/spec.md` の「どれか 1 つが許可されていれば通す」の文と Scenario「許諾の名前を 2 つ持つ部品は、どれか 1 つが許可されていれば通る」を AND に反転する（片方でも許可の外なら落ちる）。`design.md` D7（OR の文・「POM は cache に入っている」）、`tasks.md` の同じ Scenario 名を直す
- C5（Q12）: D7 に、`@pom` を取り寄せる段（online）と読む段（`--offline`）を分けて書き、tasks 4.1 の検証をそれに合わせる（手元の gradle の cache には今の部品の `.pom` が無い）
- C3（Q14・Q15）: D9・D16 の本文、spec の PR の `SOURCE.txt` の文と 2 つの Scenario、tasks 5.x の `--allow-debug-key` の比べ（比べを外すのは PR の `package` と下書きの両方。`tools/release/draft.sh:32`）、go.json の walkthrough と IMP-06、premises P14 を head の commit に直す
- C1（Q14）: tasks 11.1 に、push し直す前の head と閉じた PR の head を `git fetch origin <sha>` で取れるかの確かめを足し、P14 を head の前提に書き直す（取れなければ上流へ返す）
- C4（Q10・Q11）: D4 の失わない理由「表示は後から書き換えられる」を、Q10 の「最初の印から後は前進側の 2 つを直せないことを受け入れる」に直す。D4・D5 の「未決」と spec・tasks の「答え待ち」「（仮）」を外す
- C9（第 2 回の整合の確認）: spec の「それ以外の組み立て（開発版・PR の組み立て）」に main への push を足す
- → 対応（spec --answers Q10〜Q15。2026-11-03）: C1〜C5・C9 の仕様の側を design D4・D5・D7・D8・D9・D16、spec、tasks 2.x・4.x・5.3・6.1・7.1・11.1、proposal、premises P14 に写した。
  `check-artifacts.sh --allow-debug-key` は PR と main への push を分けられないので、main への push も比べを外す形を D17（仮）にした。
  go.json の walkthrough（W2）と IMP-06 は spec の段では直さない（GO 資料は reading の段が直す）
