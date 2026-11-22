# impact-review — st61-auto-version-bump

seen: IMP-01 medium, IMP-02 medium, IMP-03 medium, IMP-04 medium, IMP-05 low, IMP-06 low, IMP-07 low

high にすべきものは見つからなかった。IMP-01・IMP-02 は、記録を捨てずに続ける道（番号の APK・番号の組み立てを使う）が仕様の上に残るので medium で妥当。

## R1 — サーバのインストーラは版の大小を見ない。PR の 0.0.1 を番号の版の上に入れると「更新」に入り、IMP-02 が同じ版の場合だけを扱っている
- 対象: impact.findings[IMP-02] / impact.situations[update, rollback, mixed] / impact.scenarios[S2].not_when
- 種類: 新しい帰結
- 指摘: `crates/installer/src/update_plan.rs:14-25` の `version_action` は `installed == candidate` なら走らせ直し、それ以外（大小を問わず）は更新。つまり 0.2.x（や 0.1.1）が入った PC に PR の組み立て（0.0.1）のインストーラを走らせると、`main.rs:2372` で `Update` に入り、古い番号の版を名乗るプログラムへ入れ替わる。S2 の `not_when` は「0.1.x の PC に PR の組み立てを入れる（版が違うので更新に入る）」と書き、無害な側に置いている。この Story が 0.0.1 を名乗らせるまでは、PR の組み立ては 0.1.1 で、版の順では 0.1.x の PC に「同じか新しい」側だった。0.0.1 になったことで、「番号の PC に PR のサーバを入れると版の順では下げる更新になる」ことが新しく生まれる。premises P15 は「版の大小は見ず」と知っているが、IMP や lost には出ていない。DB が先に進んでいる場合は `migration_ledger.rs` の `db_ahead` が止めるが、止まる・戻すまでの更新の手順（サーバを止める・退避）を本人の PC が通る。窓や更新の履歴には 0.2.x → 0.0.1 と出る。`rollback` の situation は前の番号へ戻る場合しか書いていない。
- 直し方の案: IMP-02 に「番号の版の PC に PR の組み立てを入れると、版が下がる更新に入る（db_ahead が止める範囲は未確認）」を足し、S2 の `not_when` から外して `occurs_when` 側に書く。確かめは ST64 の守り（札の無い組み立てを入れない）の範囲に含まれるかを申し送りで言う。
- 処置: fixed: IMP-02 を「入れ替わらないか、版を下げる更新に入る」にし、本文・unknowns（db_ahead の止まる段）・S2 の occurs_when（not_when から移した）・situations の update と mixed に版を下げる更新を足した（根拠 E12 main.rs:2373）

## R2 — IMP-02 の「走らせ直しは足りないものだけを足す（正典）」と窓の表示の言い切りが、実物を挙げていない
- 対象: impact.findings[IMP-02].derived / harm.who
- 種類: 根拠
- 指摘: `derived` の「走らせ直しは足りないものだけを足す（正典）」は refs に正典の Requirement が無く、evidence は E1（版の比べだけ）。`harm.who` の「窓には走らせ直しと出る」も、`Rerun` の分岐（`main.rs:2372` の後ろ）が何を表示するかを挙げた根拠が無い。`basis: observed` は E1 が支える範囲（同じ版なら `Rerun`）までで、「プログラムを入れ替えない」「窓に出る」は `derived` か `guess` が正しい。
- 直し方の案: 走らせ直しの分岐でプログラムが替わらないことの根拠（コードの行か正典の Requirement）を足すか、basis を分ける。
- 処置: fixed: IMP-02 の basis を derived にし、走らせ直しの根拠に正典 E13（distribution/spec.md:1728）を足した。窓の表示の言い切りを消し、unknowns に「窓に何が出るかは確かめていない」を置いた

## R3 — overview の「同じ番号で中身の違う物は作られない」は、自分の IMP と食い違う
- 対象: impact.overview / impact.rules[R3].legit
- 種類: 言い切り
- 指摘: 「番号の組み立てについては、同じ番号で中身の違う物は作られない」と言い切るが、IMP-04 は手元で `ASHIATO_BUILD_VERSION=0.2.5` を渡せば 0.2.5 を名乗る別の中身を組めると言う（E8）。限定は「番号付けが作る下書きの中では」までで、「番号の組み立て」は手元の物も含んで読める。さらに `design.md:74` の「落ちたら走らせ直して同じ番号の続きを置く」は、同じ commit を別の時に組み直すので、再現できる組み立てかは確かめていない（Windows 側・依存の取り直し）。「中身の違う」の確かめが無い。
- 直し方の案: 「番号付けが下書きに置く組み立ての間では、印と説明文が別の commit を指すものは置かない」まで限定し、手元の番号の形（IMP-04）と組み直しの再現性は残る、と書く。
- 処置: fixed: overview を「番号付けが下書きに置く組み立ての間では…並ばない」に限り、手元で番号を名乗らせる道（IMP-04）と組み直しの再現性が残ることを書いた

## R4 — situations[update].can の携帯の「できる」が、署名の鍵を確かめていない
- 対象: impact.situations[update].can / impact.situations[carryover]
- 種類: 言い切り
- 指摘: `update` の `can` は「携帯は 0.2.0（20,000）が 0.1.x（10,199）より大きい」と値の大小だけを言う。これは仕様の Scenario（最初の番号は前の式の 0.1.x より大きい）の写しで、本人の携帯（debug の鍵。P10。端末の versionCode は未確認）へ上書きで入るかは、`coverage.not_examined` が自分で「CI の debug の鍵が組み立てごとに同じか」を見ていないと言っている。「値では入る」（D2.affects）の限定が situations では落ちている。`carryover.cannot` の「この分析では見つけていない」も、携帯の署名・端末側の版の保存を見ていない範囲では空の確かめを「無い」と読める。
- 直し方の案: `update.can` を「versionCode の値の上では」に限定し、鍵が違えば入れ直しになること（ST64）を `cannot` に写す。
- 処置: fixed: situations[update].can の携帯の行に「値では大きい（署名の鍵が同じかは確かめていない）」と限定を足した

## R5 — D2 の evidence E7 は変更前の式で、「番号の APK（20,000〜）は値では入る」を支えない
- 対象: impact.dependencies[D2].evidence / impact.evidence[E7]
- 種類: 根拠
- 指摘: E7（`tools/release/version.sh:38` の `MAJOR * 1000000 + …`）は今の式で、新しい式（左×10,000,000 + 真ん中×10,000 + 右）ではない。20,000 は新しい式の値で、根拠は design の D3 だけ。`basis: derived` は許されるが、E7 が「新しい式」の根拠と誤読されうる。同じく D5 の E3（`ci.yml:35` の `github.event.before`）は変更前の基準で、`holds: yes` の「前に成功した run を基準にする」の実物ではない（新しい基準は未実装）。
- 直し方の案: E7・E3 を「変更前の姿」と注記するか、新しい式・基準の根拠を design の行で挙げる。
- 処置: fixed: 依存 D2 の evidence から E7（変更前の式）を外した。D5 の affects を「今の基準は直前の push（E3）で…この Story が変えるので成り立つ」に直した

## R6 — IMP-03 の derived が、確かさ 2 の前提を重ねて「同じ番号は 2 つの commit に付かない」と言う
- 対象: impact.findings[IMP-03].derived / severity
- 種類: 重大度
- 指摘: 「同じ番号の印の 2 つ目の push は断られるので、同じ番号が 2 つの commit に付くことは無い（D10・P19）」は、P19 の「断られる側は git の振る舞い」（確かさ 2、走らせていない）に依る。ただし最悪は番号が遅れるだけで、記録の消失も誤った番号も無く、その実害の軽さは `harm` が書いている。P18 が外れても影響は遅れに限るので、重さは low 寄り。medium に置くなら「P19 が外れたら二重の番号になる」を `unknowns` に足す。`dependencies[D4]` の `who`（ci.yml の同時実行の組）は、実際に未確認の組が置かれる `number.yml` ではない。
- 直し方の案: severity を low にするか、`unknowns` に P19 の未走行を足し、D4 の `who` を number.yml の判定の job にする。
- 処置: fixed: IMP-03 を low にし、unknowns に P19（印の 2 つ目の push の拒否）を走らせていないことを足した。derived を条件つきに直し、依存 D4 の who を number.yml の判定の job にした

## R7 — IMP-01 の選択肢に、debug の APK の `adb install -r -d` を確かめた形跡が無い
- 対象: impact.findings[IMP-01].options / verify
- 種類: 閉じ方
- 指摘: 「消して入れ直すか番号の APK を待つ」と二択に狭めているが、debug 可能な APK の downgrade 上書きの可否（`-d`）を確かめていない。可なら未送信が消えずに試せる可能性がある。`verify` は「`adb install -r` が断られる」だけで、入れ直し以外の道を確かめる組になっていない。また「消すと未送信が消えうる」（`harm.link`）の根拠（収集アプリの保存場所が消えるか）も refs が無く、`guess`。
- 直し方の案: `verify` に `-d` の可否と、アンインストールで未送信が消えるかの確かめを足し、basis を `guess` にする。
- 処置: fixed: IMP-01 の options に adb install -r -d の確かめを足し、verify に -d と消したときの未送信の確かめを足した。未送信が消えることは推測として unknowns と harm に置いた（中心の帰結は仕様の式と P6 から導けるので basis は derived のまま）

## R8 — scope.residual の「入れ替えにくい」は IMP-02 の範囲（同じ版）しか言わない
- 対象: impact.scope.residual / impact.situations[mixed].cannot
- 種類: 言い換えた言い切り
- 指摘: `residual` は PR の組み立てを「入れ替えにくい」と控えめに書くが、R1 のとおり PR の 0.0.1 を番号の版の上に入れると版が下がる更新に入る。`mixed.cannot` の「PR の組み立てを混ぜるとどの番号も『新しい』と見る」は携帯の `isNewer`（`UpdateCheck.kt:124-128`）の読みで正しいが、サーバ側（大小を見ない）は逆の向きで、同じ「混ぜる」の行に載っていない。
- 直し方の案: `mixed.cannot` に、サーバ側は大小を見ず更新に入ることを足す。
- 処置: fixed: scope.residual を「携帯には上書きで入らず、サーバには入れ替わらないか版を下げる更新に入る」に、mixed.cannot にサーバ側は大小を見ず更新に入ることを足した

