# impact-review — ch10-emulator-start-timeout

seen: IMP-01 medium, IMP-02 low, IMP-03 low

## R1 — 実機や別の端末が繋がっていると、端末の待ちが上限まで続いて「現れない」と誤って止まる
- 対象: impact.dependencies（見落とし。T1 の「端末の待ち」を読む側）／ rules[R3] ／ situations[mixed]
- 種類: 新しい帰結
- 指摘: design.md D2 の `emu_wait_device` は serial 無しで `adb get-state` を 1 秒ごとに問い合わせる。adb は端末が 2 台以上あるとき serial 無しの `get-state` を `error: more than one device/emulator` で失敗させる（実機、別 AVD、録画の `harness2-rec-*` が `-port` で立てる端末。`.harness2/scripts/record-run:190`）。この失敗を「まだ現れない」と読むと、自分のエミュレータが立っていても 120 秒待ち、`error: 120 秒待っても端末が adb に現れない` で自分のプロセスを止める。今の台本は同じ状況で `adb wait-for-device`（`tools/android-emulator.sh:35`）が即座に失敗する。spec の Scenario「実機か別の AVD のエミュレータだけが繋がっているときは、今のとおり起こす」は偽の adb が `get-state` に答えるので、この経路を通らない。`situations[mixed]` の `cannot` にも出ていない。
- 直し方の案: 待ちの判定を「自分の端末の serial」か `adb devices` の一覧で行う。または `get-state` が複数端末で失敗する場合を spec に書いて、その場合は理由つきで即止まる。偽の adb に複数端末で失敗する答えを足す。

- 処置: fixed: IMP-04（medium・推測）を足し、situations[mixed]・scope.residual・W1 の判断に出した。仕様は直さず、直し方の案を options に置いた

## R2 — overview の「できなくなることは 1 つ」と `lost` の数が合わない。正常時を変えないという言い切りも IMP-03 と食い違う
- 対象: impact.overview ／ impact.lost ／ impact.rules[R5].blocks_legit
- 種類: 言い切り
- 指摘: overview は「できなくなることは 1 つ見つけた」「正常に立つときの振る舞いは変えない」と言う。しかし `lost` は 2 件ある。2 件目は、失敗で止まるときの KEEP=1 の残りがなくなること。さらに rules[R5] は `legit` で「KEEP=1 で失敗したエミュレータを残して調べる使い方は止まる」と書きながら `blocks_legit: false` にしている。`lost` と矛盾する。`lost` 2 件目には、対応する finding（IMP）も無い。正常に立つときも、IMP-03 のとおり 120 秒の上限で落ちうるので、「変えない」は限定が要る。
- 直し方の案: overview を 2 つにするか、`lost` を絞る。R5 は `blocks_legit: true` にして、手当て（emulator.log に出力が残る、`ASHIATO_EMULATOR_DEVICE_TIMEOUT` で延ばせる）を書く。「変えない」には上限を超える機械を除く旨を足す。

- 処置: fixed: overview を「上限までに現れる機械では変えない」と限定し、できなくなることを 2 つにした。rules[R5] を blocks_legit: true にして手当てを書き、筋書き S3 を足した

## R3 — IMP-01 の影響が「他の worktree の端末」に限られている。前の実行が残した端末でも、環境の問題が通常の失敗として返る
- 対象: impact.findings[IMP-01].text / derived ／ dependencies[D1]
- 種類: 重大度
- 指摘: 同じ AVD が見えて止まる最も起こりやすい場面は、他人の端末ではなく、kill された前の実行（今回の事故そのもの）の残りだと考えられる。人間が kill した台本では trap が走らないことがある。その後は、残りを手で止めるまで、この台本は毎回 rc=1 で止まる。その間、下流の検証はすべて落ちる。さらに、この文言は `evidence.py:75-79` の `INFRA_SIGNS` に当たらず（premises 9 行目も同じ指摘）、verify-run は環境の欠落でなくふつうの失敗と読む。fix の agent がコードを直す往復に入る。D1 の `affects` は「往復が 1 回増えうる」で済ませている。自分の残りなら文言のとおりに止めてよいので、IMP-01 の「他人の端末」だけを見ると、この繰り返しを見落とす。重大度は medium のままでよいが、`text`・`unknowns` に入れるべき。
- 直し方の案: IMP-01 に残りの端末で繰り返し落ちる場面を足す。あわせて、この `error:` が verify-run に環境の欠落と読まれる語を含むかを Requirement 側で決める（読まれると fix の往復は起きないが、読まれると BLOCKED_INFRA と区別がつかなくなる）。

- 処置: fixed: IMP-01 の text・unknowns・options と D1 の affects に、前の実行の残りで毎回ふつうの失敗として返り fix の往復に入りうる場面を足した。重大度は medium のまま

## R4 — IMP-02 の「取り違えは直さない」は、新しい失敗の道でも名指しの kill を残す
- 対象: impact.findings[IMP-02] ／ dependencies[D6]
- 種類: 根拠
- 指摘: IMP-02 は採寸の台本の `started` を扱うだけだが、spec の「正常に進んだ後は `emulator-5554` を止める（今のまま）」（Requirement 6 本文・R6）は計測テストの台本にも同じ取り違えを残す。R1 の経路では、serial 無しの `get-state` が他人の 5554 を自分の端末と読み、最後に他人の端末を止める。今の台本でも起きていたため、この change が増やす帰結ではない。ただし `situations[normal].cannot` や IMP-02 の `affects` は採寸の台本に限った書き方なので、計測テストの台本の取り違えが「見つけていない」ように読める。
- 直し方の案: IMP-02 の対象に計測テストの台本の後片付けを含めるか、別の IMP にして Q4 の範囲外である旨を書く。

- 処置: fixed: IMP-02 を 2 つの台本の正常な経路の取り違えに広げ（Q4 の範囲の外・この change が増やす帰結ではない）、依存 D8 を足した

## R5 — D6 の `holds: unknown` は、挙げた根拠から言える内容より弱い
- 対象: impact.dependencies[D6].holds
- 種類: 根拠
- 指摘: E10（`usage-volume.sh:103` が `started` があれば名指しで `emu kill`）と、現行コード（`tools/usage-volume.sh:136-139` で起こした直後に `started=emulator-5554` を置く）から、現れる前に止まれば他人の 5554 を止めうることは観測で言える（`holds: no`）。いまの `unknown` は「現れた端末が自分のものか」の話と混ざっている。
- 直し方の案: D6 を 2 つに分け、前半を `holds: no`（この change が直す）、後半を `unknown`（IMP-02）にする。
- 処置: fixed: D6 を 2 つに分け、前半を「今は成り立たず、変更の後は成り立つ」（holds: yes。依存の holds は変更の後で読む欄のため）、後半を D8 の unknown（IMP-02）にした
