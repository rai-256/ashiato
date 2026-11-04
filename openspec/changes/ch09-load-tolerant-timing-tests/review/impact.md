# impact-review — ch09-load-tolerant-timing-tests

seen: IMP-01 medium, IMP-02 medium, IMP-03 low, IMP-04 low

## R1 — IMP-01 の「下流が CH09 の 2 回だけ」は tasks と合わない
- 対象: impact.findings[IMP-01].options[0] / unknowns
- 種類: 閉じ方
- 指摘: 受け入れの理由に「2 回だけ」とあるが、tasks.md は host 負荷を走らせる場所をほかにも持つ。Task 1.1 の `--self-test` が `--load host` / `--load db` で負荷を立てる（tasks.md:36-41）。Task 1.2 の before と Task 9.3 の after は、それぞれ 2 本 × 10 回 × 負荷の種類。9.3 は「検証」欄の命令なので、gate や fix の往復のたびに再び走る。つまり CH09 自身の負荷は 2 回で終わらない。さらに before は `--expect-fail` で「落ちる回が出るまで」の証跡なので、組み立て（cargo build）も全コア負荷の下で走る。1 回の所要時間は unknowns に書いてあるとおり未測定で、「重なる頻度」を見積もる材料が無い。medium のままでよいが、options[0] の理由は予定を書いただけ。
- 直し方の案: options[0] の括弧を「検証の再走のたびに起きる」に直し、選択肢 2（ほかの下流が走っていないときだけ）か 3（負荷の時間に上限）を推奨側に置く。
- 処置: fixed: IMP-01 の text・options・unknowns に、負荷が self-test・1.2・9.3 で走り、9.3 は gate・fix のたびに走り直すことを書いた

## R2 — IMP-02 の「印 ch09-load-<pid> で探して止める」は手元の busy loop に効かない。シグナルも HUP が抜けている
- 対象: impact.findings[IMP-02].options[1] / unknowns
- 種類: 閉じ方
- 指摘: 印が付くと design D7（design.md:102-108）に書いてあるのは DB のコンテナの中の過程だけ。手元（host）の `nproc` 本の bash busy loop には印の置き場が書かれておらず、`ps` では他の bash と区別できない。「残ったら印で探して止める」で受け入れるなら、host 側が探せない。また trap は EXIT・INT・TERM のみで HUP が無い（端末・セッションが閉じたときは HUP）。`scripts/quiet-run`（subprocess.run、timeout も signal 転送も無い。scripts/quiet-run:70 付近）は、自分が殺されても子の load-repeat にシグナルを送らないので、quiet-run 経由（tasks.md の検証はすべてこれ）で上位が殺されると、load-repeat の trap は走らない。unknowns の「KILL を送るか見つけていない」は、KILL 以外でも残る道があることを覆い隠している。
- 直し方の案: busy loop に親 pid の消滅か上限時間で自分が止まる条件を持たせる（options[0]）を既定にし、trap に HUP を足す。host の過程にも探せる印（argv0 か環境変数）を決める。
- 処置: fixed: IMP-02 の text に HUP が無いこと・印が DB のコンテナの中だけのことを、unknowns に quiet-run がシグナルを渡すかを、options に手元の印と HUP を足した

## R3 — IMP-03 は壊す差分の当たりを心配しているが、実際に当たるのは試験の土台の側（runtime.rs の mod tests）
- 対象: impact.findings[IMP-03] / dependencies[D3] / scenarios[S1]
- 種類: 依存の見落とし
- 指摘: E7（st54 tasks.md:94）が示すのは `HistoryCollector::new` の引数と `collect.rs:315` の変更。壊す差分 `break-wait-on-read.patch` の対象は `Stage::Reading` の周り（collect.rs:401-480）で、new の周りではない。S1 の「壊す差分が当たらない」は E7 から言えない（derived の飛躍）。一方 st54 tasks.md:99-100・111 は、CH09 の T1 が書き直す `runtime.rs` の `#[cfg(test)]`（`cfg()`・`history_runtime`・偽の読み手を使う新しい試験）を直接触り、`HistoryCollector::new` の呼び口 `tests/runtime_windows.rs:1443` と `collect.rs:844` の試験の土台も書き換える。T1 の試験の土台の呼び方は確実に影響を受けるのに、D3 の assumes は「collect.rs の形」だけを置いている。st55 も runtime.rs（製品の部分）を触る（coverage に書いてあるのみで dependencies に無い）。
- 直し方の案: D3 の assumes を「runtime.rs の mod tests（cfg()・history_runtime）と collect.rs:844 の土台」に直し、S1 の trigger から壊す差分が当たらない前提を外すか、`git apply --check` を main の上で見た根拠を足す。
- 処置: fixed: D3・IMP-03・S1・R4・situations[update] を、試験の土台（runtime.rs の mod tests・collect.rs:844）の側に直し、E13 を足した。壊す差分は別の場所と書いた

## R4 — situations[normal].can は Scenario を写した言い切りで、直した試験が落ちないことをまだ誰も確かめていない
- 対象: impact.situations[normal].can / overview
- 種類: 言い切り
- 指摘: 「混んでいる機械で…直した試験は「〜秒以内」で落ちない。普段の所要時間は変わらない」は basis: spec で、Scenario（10 回通る）の写し。load-before.txt（再現）も load-after.txt も無く（not_examined の 3 つ目がそれを認めている）、review/deep.md R1 は 2 の試験の落ち方が上限の短さでなく時計を進める競り合いだと模型で示している。「普段の所要時間は変わらない」も、画面の偽の時計の 5 ファイル（D6 holds: unknown, basis: unconfirmed）が未確認のまま。overview の「減る」は限定が付いているが、can は「落ちない」と書いている。
- 直し方の案: can を「Task 9.3 で 10 回通ることを確かめる計画」に直すか、「落ちにくくなる見込み」と限定して basis を guess にする。
- 処置: fixed: situations[normal].can を「落ちない形にする。10 回通ることは下流の load-after.txt で確かめる（上流では走らせていない）」にした

## R5 — rules[R1/R2].legit の数字の根拠が evidence に無い
- 対象: impact.rules[R1].legit・rules[R2].legit
- 種類: 根拠
- 指摘: R1 の「見回り 1 回の普段の所要時間は 30 秒より 3 桁短い」は、同じ analysis の E12（tools/tmp-leak-check.sh:13）が「見回り 1 回 1.1〜2.2 秒」を記録しているのと食い違う（これは遅いディスクの場合だが、混み具合で同じ桁が出うることを示す）。1〜2 秒なら 30 秒との差は約 1.5 桁。R2 の「load 14 で 1.3 秒・5 秒が足りなかった実測」は evidence のどれにも無く、CH09.md・premises にも「1.3 秒」の語が無かった。どちらも blocks_legit: false / unknown の判断を支える数字なので、出所が要る。
- 直し方の案: E12 を引いて「1〜2 秒が最悪の実測、30 秒はその 15 倍以上」と書くか、数字を外す。R2 の実測は出所を evidence に足す。
- 処置: fixed: R1 の legit を 1〜2 秒（E12）・1 桁以上に、R2 の legit を CH09.md の実測（1.34 秒・5 秒の待ち）と「確かめていない」に直した

## R6 — IMP-04 は D3 の誤りを見つけたが、runtime_windows.rs の「10 秒以内」が走る環境の遅さを unknown のまま low に置いている
- 対象: impact.findings[IMP-04].severity / derived
- 種類: 重大度
- 指摘: runtime_windows.rs:211 の `start.elapsed() < 10s` は UI オートメーションの窓の出現待ちで、GitHub の Windows runner（コア数が少なく遅い）は「混まない」とは限らない。derived の「runner は並走で混む手元の機械ではない」は、混み具合と原因が違うと言い切るが、unknowns に書いたとおり落ちた記録は見ていない。st54 がこのファイルを触る（tasks.md:100）ので、そこで CI の Windows job を走らせた PR が落ちる可能性は D3 と同じ経路にもある。severity low は妥当だが、derived の言い切りは guess の強さ。
- 直し方の案: derived の「混まない」を「混まないと見ている（記録未確認）」に直す。
- 処置: fixed: IMP-04 の text から「混まない」の言い切りを外し、確かめていないと書いた（重大度は low のまま）

