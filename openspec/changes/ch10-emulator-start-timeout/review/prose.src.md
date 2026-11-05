# CH10 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
手元でエミュレータを起こして計測テストを走らせる台本が、エミュレータを起こせないときに来ない端末を待ち続けず、早く・理由つきで止まるようになる。止まった理由とエミュレータの出力の末尾が残り、/dev/kvm のときは verify-run が環境の欠落と読める文言で落ちる。正常に立つときの振る舞いは変わらず、それを偽の SDK の試験で毎回確かめる。

<!-- prose impact.overview -->
この change の後、計測テストの台本と利用状況の採寸の台本は、/dev/kvm を開けないときは起こす前に、エミュレータが途中で終わったときはすぐ、端末が現れないときは 120 秒で、理由とエミュレータの出力の末尾を出して止まる。正常に立つときの振る舞いは、端末が上限までに現れる機械では変えない（偽の SDK の試験と手元の本物の 1 回で確かめる計画。混んだ機械で上限を超えうることは IMP-03）。この分析で見つけたできなくなることは 2 つ: 同じ AVD のエミュレータが既に動いているとき計測テストの台本はその端末を使わずに止まり（IMP-01）、失敗で止まったときは KEEP=1 でもエミュレータが残らない。ほかに、実機や別の端末が繋がっていると端末の待ちが上限まで続いて誤って止まりうる（IMP-04。推測）。

<!-- prose impact.problem -->
2026-11-05、ST54 の ci_fix で、Codex が sandbox の中から計測テストの台本を走らせた。/dev/kvm を開けずエミュレータは落ちたが、台本は来ない端末を約 20 分待ち、人間が kill するまで別のセッションが止まった。理由も残らなかった。

<!-- prose uses[U1].title -->
sandbox の中から計測テストを走らせてしまっても、すぐ理由が分かる

<!-- prose uses[U1].trigger -->
ST54 の ci_fix で、Codex が sandbox の中から `scripts/quiet-run … -- ./tools/android-emulator.sh` を走らせた。sandbox の中では /dev/kvm を開けず、今の台本は来ない端末を約 20 分待ち、人間が kill した。

<!-- prose uses[U1].outcome -->
数秒で止まり、何が足りないか・次に何をすればよいかが出力に残る。人間が待ちを kill しに行かなくてよい

<!-- prose uses[U1].without -->
20 分前後待ち続け、人間が気づいて kill するまで別のセッションが止まる。落ちた理由も残らない

<!-- prose uses[U1].steps[0].do -->
同じ命令を、この change の後の台本で走らせる

<!-- prose uses[U1].steps[0].see -->
エミュレータを起こす前に `error: /dev/kvm を開けない（調べた場所: /dev/kvm）。エミュレータを起こさない。verify-run で走らせ直す（Codex なら AGENTS.md の「/dev/kvm」の手順で sandbox の外を申請する）` が出て rc=1

<!-- prose uses[U1].steps[1].do -->
AGENTS.md の「/dev/kvm（Codex のとき）」の手順どおり `scripts/verify-run` で走らせ直す

<!-- prose uses[U1].steps[1].see -->
verify-run が BLOCKED_INFRA にし、sandbox の外での実行を 1 回申請する

<!-- prose uses[U2].title -->
手元でエミュレータが起動の途中で落ちたとき、理由を見る

<!-- prose uses[U2].trigger -->
手元で `./tools/android-emulator.sh` を走らせたら、すぐ rc=1 で終わった。何が悪かったのかを知りたい。

<!-- prose uses[U2].outcome -->
加速が使えないのか、別の理由なのかを出力から決められる

<!-- prose uses[U2].without -->
出力は捨てられていて、台本の出力からは理由が分からない。手でエミュレータを起こし直して出力を見ることになる

<!-- prose uses[U2].steps[0].do -->
標準エラーを読む

<!-- prose uses[U2].steps[0].see -->
`error: エミュレータが起動の途中で終わった` と、エミュレータの出力の末尾 40 行（例: `ERROR | x86_64 emulation currently requires hardware acceleration!`）と、全文の置き場

<!-- prose uses[U2].steps[1].do -->
`collector-android/app/build/outputs/emulator.log` を開く

<!-- prose uses[U2].steps[1].see -->
その回のエミュレータの出力の全文

<!-- prose walkthroughs[W1].title -->
エミュレータを起こせない 4 つの場合と、正常な場合

<!-- prose walkthroughs[W1].scene -->
計測テストの台本を、エミュレータがいろいろな形で起こせない機械で走らせる。どの場合も台本は止まるか、今のとおり進む。

<!-- prose walkthroughs[W1].why -->
待つ間、1 秒ごとに自分の起こしたプロセスが生きているかを先に見て、端末の待ちには時間の上限を付ける。/dev/kvm と同じ AVD は起こす前に見る。

<!-- prose walkthroughs[W1].figure.caption -->
場合ごとの止まり方（秒数は既定の上限。手元の実測では、端末が現れるまで約 15 秒・起動の完了まで約 35 秒）

<!-- prose walkthroughs[W1].cases[0].result -->
今は 20 分前後待つ。この後は数秒で rc=1

<!-- prose walkthroughs[W1].cases[1].result -->
今と同じ順で最後まで走り、rc=0

<!-- prose walkthroughs[W1].exceptions[0].text -->
説明用の場合分け。Codex の sandbox が /dev/kvm を開くのを拒む形そのものは再現していない（/dev/null を被せた形で再現した）

<!-- prose walkthroughs[W2].title -->
KEEP=1 の後に続けて走らせる

<!-- prose walkthroughs[W2].scene -->
手元で `KEEP=1 ./tools/android-emulator.sh` を走らせてエミュレータを残し、少し直してからもう一度台本を走らせる。

<!-- prose walkthroughs[W2].why -->
同じ AVD の 2 台目は起こしても数秒で死ぬ。今の台本は「どれかの端末」が現れたら進むので、前からある端末で走っていた。この change はその端末を使わず・止めずに止まる。

<!-- prose walkthroughs[W2].figure.caption -->
2 回目に何が起きるか（emulator-5554 は説明用の番号）

<!-- prose walkthroughs[W2].cases[0].result -->
止める手間が 1 回増える

<!-- prose walkthroughs[W2].cases[1].result -->
台本は止まる。文言のとおり止めると、別の worktree の計測テストを途中で壊す（IMP-01）

<!-- prose walkthroughs[W2].cases[2].result -->
今のとおり起こす

<!-- prose walkthroughs[W2].exceptions[0].text -->
採寸の台本には当てない（動いている端末を使う今の振る舞いのまま）

<!-- prose model.caption -->
台本がエミュレータを起こす流れと、止まる 4 つの所

<!-- prose features[kvm].summary -->
起こす前に開いてみて、開けなければ起こさずに止まる

<!-- prose points[kvm.1].text -->
起こす前に /dev/kvm を読み書きで開いてみる。無いか開けなければ、エミュレータを起こさずに `error: /dev/kvm を開けない` と次の手（verify-run で走らせ直す）を出して rc=1

<!-- prose points[kvm.2].text -->
文言は /dev/kvm の語を必ず含み、verify-run が走らせた後に出力で「環境の欠落」と読む判定に掛かる。手元では判定の正本（evidence.py）をそのまま当てて確かめる

<!-- prose points[kvm.3].text -->
/dev/null を被せた形のように「開けるが使えない」ときは、ここでは止まらず、起こした後に「起動の途中で終わった」で止まる。そのときも出力が加速の不足を言えば /dev/kvm の語を含む行を足す

<!-- prose features[die].summary -->
待ちの上限を待たずにすぐ止まり、出力を残す

<!-- prose points[die.1].text -->
端末が現れるのを待つ間も、起動の完了を待つ間も、自分の起こしたプロセスが終わったらすぐ `error: エミュレータが起動の途中で終わった` で rc=1

<!-- prose points[die.2].text -->
エミュレータの出力は毎回 `collector-android/app/build/outputs/emulator.log` に書き直し、止まるときは末尾 40 行と置き場を出す

<!-- prose features[limits].summary -->
端末は 120 秒、起動の完了は今のまま 180 回 × 2 秒

<!-- prose points[limits.1].text -->
プロセスが生きたまま端末が adb に現れなければ、既定 120 秒で `error: <上限> 秒待っても端末が adb に現れない`。待ち始めに使う上限を出す

<!-- prose points[limits.2].text -->
上限は環境変数で上書きできる（端末の秒・起動の完了の回数・1 回の adb の秒・調べる /dev/kvm の場所）。正の整数でない値なら起こさずに止まる。混んだ機械で 120 秒を超える正常な起動は、延ばさないと落ちる

<!-- prose points[limits.3].text -->
起動の完了を待つ回数（180 回 × 2 秒）と文言 `error: 起動が終わらない` は今のまま。1 回ごとの `adb shell` には 10 秒の上限を掛ける

<!-- prose features[sameavd].summary -->
正常時の振る舞いが変わる唯一の所

<!-- prose points[sameavd.1].text -->
起こす前に、adb に見えているエミュレータの AVD の名前を聞き、起こそうとする AVD と同じなら、起こさず・その端末で走らせず・止めずに、止め方を含む文言で rc=1

<!-- prose points[sameavd.2].text -->
今は `KEEP=1` の残りや別の worktree の同じ AVD の端末で計測テストが走っていた。この後は止まるので、続けて走らせるには先に止める手間が 1 回要る

<!-- prose points[sameavd.3].text -->
実機や別の AVD のエミュレータだけが繋がっているときは、今のとおり起こす。adb に見える前の同じ AVD（起動の途中）は見つけず、新しいプロセスが数秒で死んで「起動の途中で終わった」で止まる

<!-- prose features[cleanup].summary -->
自分の起こしたプロセスだけを止める

<!-- prose points[cleanup.1].text -->
失敗で止まるとき、自分の起こしたプロセスとその子を `KEEP=1` に依らず止める。起こす前に止まるときは何も止めない

<!-- prose points[cleanup.2].text -->
失敗で止まるときは `emulator-5554` を名指しで止めない。ただし正常に計測テストまで進んだ後は今のまま `emulator-5554` を止め、待つ相手も今のまま「どれかの端末」（取り違えは直さない）

<!-- prose features[normal].summary -->
今の振る舞いを偽の SDK の試験で固定する

<!-- prose points[normal.1].text -->
今と同じ引数で起こし、アニメーションを切り、写真と動画を入れ、計測テストを 6 段で走らせて段ごとの XML を残す。`KEEP=1` でなければ最後に止める

<!-- prose points[normal.2].text -->
emulator・システムイメージが無いときの文言、AVD が無ければ作ること、採寸の試験を通常の段から除く行の形も今のまま

<!-- prose points[normal.3].text -->
試験は使い捨ての木に本物の台本を写し、偽の SDK・adb・gradle を置いて走らせる。本物のエミュレータも /dev/kvm も要らないので、CI の検査の job で毎回走る。本物でも手元で 1 回通す

<!-- prose features[usage].summary -->
起こせなければ「測れなかった」（rc=2）ですぐ止まる

<!-- prose points[usage.1].text -->
利用状況の採寸の台本も同じ部品でエミュレータを起こし、/dev/kvm・途中で終わる・現れない・起動が終わらないで、同じ文言を出して rc=2

<!-- prose points[usage.2].text -->
adb に見えているエミュレータがあれば起こさずに使う・`-wipe-data` の引数は今のまま。同じ AVD の確かめは当てない

<!-- prose impact.scenarios[S1].result -->
rc=1 と「KEEP=1 の残りなら adb -s emulator-5554 emu kill で止めてから」。verify-run はこれを環境の欠落と読まず、ふつうの失敗として扱う

<!-- prose impact.scenarios[S1].lost -->
文言に従って止めると、相手の計測テストが途中で壊れる。止めなければ、相手が終わるまで自分の検証が通らない

<!-- prose impact.scenarios[S2].result -->
正常な起動の途中でも rc=1 で止まり、自分の起こしたプロセスを止める

<!-- prose impact.scenarios[S2].lost -->
その回の検証。上限を延ばして走らせ直せば通る見込み

<!-- prose impact.scenarios[S3].result -->
エミュレータは残らない。出力の全文は emulator.log に残る

<!-- prose impact.scenarios[S3].lost -->
止まった時点のエミュレータを手で触って調べること

<!-- prose impact.rules[R1].legit -->
この分析では正当な利用を止める場面を見つけていない。x86_64 のエミュレータは加速が無いと起動直後に終わる（premises 1 行目の実測）

<!-- prose impact.rules[R2].legit -->
終わったプロセスを待っても端末は来ないので、正当な利用は止めない

<!-- prose impact.rules[R3].legit -->
混んだ機械で正常な起動が 120 秒を超えると止まる。超えるかは確かめていない（IMP-03）

<!-- prose impact.rules[R4].legit -->
KEEP=1 で残した自分のエミュレータで続けて走らせる使い方も止まる。止め方の文言に従うと、別の worktree が走らせている計測テストを壊しうる

<!-- prose impact.rules[R6].legit -->
今と同じ。emulator-5554 にいるのが別のエミュレータでも止める取り違えは残る（Q4 で直さない）

<!-- prose impact.rules[R5].legit -->
KEEP=1 で失敗したエミュレータを残して調べる使い方は止まる。手当ては、出力の全文が emulator.log に残ることと、上限を環境変数で延ばせること

<!-- prose impact.findings[IMP-01].title -->
同じ AVD で止まったときの文言が、他人の計測テストを止める手を勧めうる

<!-- prose impact.findings[IMP-01].text -->
同じ AVD のエミュレータが動いていると、計測テストの台本は止まり、止め方（`adb -s <serial> emu kill`）を出す。それが別の worktree の計測テストの端末なら、文言どおりに止めると相手の実行が途中で壊れる。下流の agent は文言を手順として読みやすい。もう 1 つの場面: kill された前の実行（今回の事故の形）の残りの端末があると、手で止めるまで毎回 rc=1 で止まる。この文言は verify-run の環境の欠落の判定（INFRA_SIGNS）に当たらないので、ふつうの失敗として返り、fix の agent がコードを直す往復に入りうる

<!-- prose impact.findings[IMP-02].title -->
2 つの台本は、現れた端末が自分のものかを確かめずに使い、最後に止める

<!-- prose impact.findings[IMP-02].text -->
採寸の台本は起こした後に emulator-5554 が現れたらそれを自分の端末として使い、計測テストの台本も「どれかの端末」で進み、どちらも正常に進んだ後に emulator-5554 を名指しで止める。この change は失敗の経路の名指しの emu kill をやめるが、正常な経路の取り違えは今のまま残る（Q4 の範囲の外。この change が増やす帰結ではない）

<!-- prose impact.findings[IMP-04].title -->
実機や別の端末が繋がっていると、端末の待ちが上限まで続いて誤って止まりうる

<!-- prose impact.findings[IMP-04].text -->
design の端末の待ちは serial を付けずに `adb get-state` を問い合わせる。adb は端末が 2 台以上あると serial の無い問い合わせを `more than one device/emulator` で失敗させる（AI の知識。確かめていない）。そう動くなら、実機・別の AVD・録画が別の番号で立てた端末が繋がっている間は、自分のエミュレータが立っていても 120 秒待ってから「端末が adb に現れない」で止まる。spec の「実機か別の AVD のエミュレータだけが繋がっているときは、今のとおり起こす」は、偽の adb が答えるのでこの経路を通らない

<!-- prose impact.findings[IMP-03].title -->
混んだ機械では、120 秒の上限が正常な起動を落としうる

<!-- prose impact.findings[IMP-03].text -->
並走が多い日（load average 9〜14 の実測が ch09 にある）に、端末が adb に現れるまで 120 秒を超えると、正常な起動でも rc=1 になる。verify-run はこれをふつうの失敗と読み、下流の fix が往復しうる

<!-- prose impact.scope.covers[0].text -->
手元で走らせる計測テストの台本と利用状況の採寸の台本の、エミュレータの起動と待ち

<!-- prose impact.scope.preconditions[0].text -->
同じ AVD の確かめは計測テストの台本だけ。採寸の台本には当てない

<!-- prose impact.scope.preconditions[1].text -->
失敗の経路だけで名指しの emu kill をやめる。正常に計測テストまで進んだ後の後片付けは今のまま

<!-- prose impact.scope.preconditions[2].text -->
正常な経路の振る舞いは、偽の SDK の試験と手元の本物の 1 回で確かめる

<!-- prose impact.scope.excludes[0].text -->
CI の計測テスト（android-emulator-runner）・st54 の side-by-side の台本・ハーネスの verify-run

<!-- prose impact.scope.excludes[1].text -->
計測テストの途中でエミュレータが死んだときの扱い（gradle が端末を失って落ちる）

<!-- prose impact.scope.residual[0].text -->
同じ AVD で止まったときの止め方が、別の worktree の計測テストを壊しうる。前の実行の残りがあると毎回ふつうの失敗で返る（IMP-01）

<!-- prose impact.scope.residual[1].text -->
端末が複数繋がっていると、待ちが誤って上限まで続きうる（IMP-04）

<!-- prose impact.scope.residual[2].text -->
st54 の side-by-side は、この change の後も来ない端末を無期限に待ちうる

