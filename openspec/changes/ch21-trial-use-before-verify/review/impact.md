# impact-review — ch21-trial-use-before-verify

seen: IMP-02 medium, IMP-03 medium, IMP-04 medium, IMP-05 high, IMP-06 low, IMP-07 medium, IMP-08 medium, IMP-09 medium, IMP-12 medium, IMP-13 high

## R1 — IMP-13 は「PATH に adb があるか」を unknown にしているが実物で言え、重大度が IMP-05 と釣り合わない
- 対象: impact.findings[IMP-13].(severity, unknowns, derived)
- 種類: 重大度
- 指摘: unknowns の「席を起こす環境の PATH に adb があるか」は実物で確かめられる。`tools/verify-prep.sh:48` 付近が `tools/android-env.sh` を source して PATH に `platform-tools` を足し（E20）、席の台本は `os.environ` をそのまま子に渡す（`.harness2/scripts/tsukau.py:497-498`）。確認バッチから呼ぶ席は準備の中で起きるので、adb は PATH にあるはずで、手で呼ぶ席も `agent-env.sh` 経由で同じになる。つまり残る不確かさは「席が案内に反して実機に触るか」だけで、IMP-05（high・同じ根・同じ auto）と同じ形になる。害は、実機の収集アプリを消す・止めると送る前の記録が失われ、戻せず、本人は気づけない。これは high の三条件（続けられない／自分で気づけない／配った後では直せない）の後 2 つに当たる。それなのに medium で、IMP-05 のような `if_accepted`（残る余地・見直しの担当と期限）が無い。IMP-05 の答え（受け入れて進める）は携帯の相手を足す前のもので、IMP-13 には及ばない。
- 直し方の案: PATH の unknown を derived に直して「触るか」だけを unknown にし、high にして `if_accepted` を書くか、medium のままなら「実機が繋がっていない日は起きない」を条件にした理由を書く。
- 処置: fixed: IMP-13 を high にし、basis を derived（E21: 準備が android-env.sh を source する・E20）に直した。unknowns を「席が実機に触るか」などに絞り、if_accepted（residual・scope・recheck）と「仕様を直す」の案を足した。IMP-05 の答えが及ばないことを text と since_last に書いた

## R2 — 形 A の「戻せない扉」に実機の未送信の記録が無い
- 対象: impact.memory（kind: one_way_door）、impact.lost
- 種類: 新しい帰結
- 指摘: 一方通行の扉の一覧は「公開の repo の題・モデルに渡った入力・外に出た IP と時刻・Sandbox の窓の記録」の 4 つ。IMP-13 の害（実機の収集アプリの中の、まだ送っていない記録が消える）は、人間が覚えて持つ私的データを戻せなくする扉なのに、memory にも `lost` にも出てこない。IMP-13 の中にだけあり、形 B の 1 件に埋もれている。
- 直し方の案: one_way_door に実機の未送信の記録（席が adb で実機に触ったとき）を足し、`lost` に 1 行加える。
- 処置: fixed: memory の one_way_door に、席が実機に触れたときに消えうる未送信の記録（IMP-13）を足した。lost には足さない（変更の前にできていたことが減る話ではない）

## R3 — IMP-12 は席の APK が置き換わることを unknown にしているが、置き換わり、その先の害を書いていない
- 対象: impact.findings[IMP-12].(unknowns, harm, text)
- 種類: 新しい帰結
- 指摘: `tools/verify-prep-device.sh:4-18` は `emulator-*` で `device` のものすべてに `adb -s … install -r` する。席の APK も準備の APK も同じ debug の組み立てなので、パッケージ名と署名は同じと見てよく、`install -r` は置き換える（design D30 は debug の組み立てを使うと書いている）。置き換わった APK の接続先は準備の `ashiato.baseUrl`（`verify-prep.sh` が loopback に限っている）で、席が `adb reverse` で渡したその回のサーバの port とは違う。席の携帯の収集アプリは送れなくなり、「携帯の不具合」の候補が偽で出る。その候補は処置に渡り、「この回が原因」と選別されれば IMP-08・IMP-09 の道で審査を通った Story のブランチに要らない直しが載る。IMP-12 の harm は「席が別の APK を触った結果になる」で止まり、この先を書いていない。`link: なし` も、この道があるので言い切れない。
- 直し方の案: 置き換わりを derived にし、偽の候補が処置へ流れる帰結を IMP-09 とつなげて書く（または準備の APK の入れ先から専用の AVD を外す案を推奨にする）。
- 処置: fixed: IMP-12 の text に、install -r で席の APK が置き換わり接続先が変わること・偽の候補が IMP-08・IMP-09 の道に入りうることを書き、unknowns から置き換わりを外し、harm.link を直した

## R4 — 「私設網と手元の網を塞げたと確かめた」は、確かめた範囲より広く読める
- 対象: impact.scope.preconditions[2]、impact.rules[R15]、impact.overview
- 種類: 言い切り
- 指摘: design D26 の確かめは、WSL 側で `tailscale ip` の宛先、Windows 側で手元の網のアドレスの、一時の口 2 種類に塞ぐ前後で当てるだけ。本人の携帯や LAN の他の機器など、それ以外の宛先は、D29 の数から組んだ範囲（自己試験は見本の宛先を当てるだけ）に頼っていて、生きた Sandbox では当てない（D29「確かめの対照が当てない機器（本人の携帯など）への穴を、範囲の側で塞ぐ」）。それなのに scope の「私設網と手元の網を塞げたと確かめた後にだけ配る形を入れる」と R15 は、網全体を確かめたように読める。overview の「私設網と手元の網は塞ぐ」は事実の限定として足りるが、確かめの強さは書いていない。
- 直し方の案: 「確かめたのは、ホストの 2 つの宛先に届かないこと。ほかの機器は範囲の組み方（自己試験）で塞ぐ」と限定を書き、IMP-02 の unknowns に「範囲に載らない私的な宛先」を足す。
- 処置: fixed: pc.1・pc.5・R15・preconditions・since_last を「この PC の 2 つの宛先に届かないと確かめる」に直し、ほかの機器は数から組んだ範囲に頼ることを pc.5 と scope.residual に書いた

## R5 — IMP-02 の harm は「外には見えない」と言うが、失敗の仕方で外に届く
- 対象: impact.findings[IMP-02].harm.who、harm.link
- 種類: どれくらい困るか
- 指摘: harm.who は「本人（PC の席が回らない手順書になる）。外には見えない」。しかし unknowns の 1 つ（範囲の数をファイアウォールが受け付けるか）が外れて規則が付かなかった場合は、R15 の確かめで落ちて断る側に倒れる一方、確かめ自体が通っても規則が半端に入る（一部の範囲だけ入る）形は確かめの 2 宛先では見つからない（R4）。その場合 Sandbox の中の更新係・席が開いたサイトから、本人の網の機器へ届く余地が残る。harm は「席が回らない」だけで、この向きの害を書かず、「外には見えない」と言い切っている。
- 直し方の案: unknown に「規則が一部だけ入った場合を確かめの 2 宛先が見つけるか」を足し、who の「外には見えない」を限定に直す。
- 処置: fixed: IMP-02 の harm.who から「外には見えない」を外し、規則が一部だけ入ったときに 2 つの宛先の外へ届くかは確かめていないと書いた

## R6 — IMP-03 の harm.when の頻度が、延びる時間の最悪を軽く見せる
- 対象: impact.findings[IMP-03].harm.when、impact.lost[0].after
- 種類: どれくらい困るか
- 指摘: when は「1 日 1〜5 回まで」。premises.md の 43 行（1 日 1〜5 本・平均 2 本）は確認バッチの候補が作られる回数で、準備が席を呼ぶ回数ではないが、Q11 で作り直しは新しく入ったものだけを試すので 1 回あたりは短くなる、という含意が text には出てこない。逆に最悪（5 回 × 9,360 秒 ≈ 13 時間の錠）は書かれず、「急いで確認バッチを見たい日」の 1 件の軽さしか読めない。
- 直し方の案: 最悪の合計と、Q11 で作り直しが縮む前提（前の回で試した分を外す）を harm に並べる。
- 処置: fixed: IMP-03 の harm.when に、最悪 5 回とも上限まで延びて約 13 時間錠を握ることを足した

## R7 — R14 の legit と blocks_legit が食い違う
- 対象: impact.rules[R14]
- 種類: 言い切り
- 指摘: `legit` は「ほかの試験で外とつないで初めて出る不具合を見たいときは、名指しを足す（正典を直す）までできない」と、正当な利用が止まると書いているのに `blocks_legit` は false。同じ規則で R1 や R15 は同じ形で true にしている。
- 直し方の案: blocks_legit を true にするか、legit を「止まらない」側の文に直す。
- 処置: fixed: R14 の legit を「ほかの試験は本物の配布元へ繋がない。外とつないで初めて出る不具合は名指しの 3 つで見る」に直し、blocks_legit は false のまま

## R8 — IMP-12 の text は「偽の候補は処置に渡り」と言い切るが、unknowns は渡るかを未確認にしている
- 対象: impact.findings[IMP-12].(text, harm.link)
- 種類: 言い切り
- 指摘: text は「その偽の候補は処置に渡り、要らない直しの道（IMP-08・IMP-09）に入りうる」と続けるが、unknowns の 2 つ目は「手で呼んだ席の候補が処置に渡るのは、本人が確認バッチ以外の道で回したときだけか」と未確認。さらに `scope.preconditions` は「手で呼んだ席は目録を作らない」と言う。席のエミュレータが残る・取り合う経路で、確認バッチの目録に入る道は D22（確認バッチの席は準備の後）で閉じており、渡る道は本人の手の経路に限られる。「渡り」は限定を落としている。
- 直し方の案: text を「本人が手で処置へ回せば渡りうる（目録には入らない）」に直し、IMP-08・09 への連鎖の条件を書く。
- 処置: fixed: IMP-12 の text を「偽の候補が処置に渡るのは、本人が手で回したときだけ」に直した

## R9 — IMP-13 は「WSL の adb に実機が見えるか」を前提にしているが unknowns に無い
- 対象: impact.findings[IMP-13].(unknowns, harm.when, if_accepted.residual)
- 種類: 根拠
- 指摘: PATH に adb があること（E21・E20）は実物で言える（席の呼び出しは verify-prep.sh の中で、android-env.sh を source した後）。しかし席が走るのは WSL で、WSL2 は既定では USB の携帯を見ない（usbipd 等が要る）。見えるのは WSL から届く無線デバッグか、本人が USB を渡した場合に限られる。「USB か無線のデバッグで繋がっていれば」は、その WSL の adb に実機が出る条件を飛ばしている。unknowns は「繋がっている時間」だけで、WSL の adb が実機を見る構成か（docs/ や design に usbipd・adb connect の記述は CH21 の外にしか無い）を挙げていない。この前提が外れれば high の根拠（害の現実性）が変わる。逆に本人が WSL に渡す構成なら、`adb devices` 1 回で確かめられる。
- 直し方の案: unknowns に「WSL の adb が本人の実機を見る構成か」を足し、harm.when の条件にも入れる（確かめ方は実地の journal と合わせて `adb devices` の出力）。

前の R1〜R7 の処置は実物で直っている（IMP-13 の high・derived・if_accepted・memory の扉、R14 の legit、IMP-02 の who、IMP-03 の最悪 13 時間と 9,360 = 2,400 + 2,220×3 + 300）。
- 処置: fixed: IMP-13 の text・unknowns・harm.when に「WSL の adb に実機が見える構成のとき」の前提を足し、見えない構成なら起きないと書いた。high は残す（無線のデバッグなら見えうり、前提を unknowns に出した）

