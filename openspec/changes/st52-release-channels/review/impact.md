# impact-review — st52-release-channels

seen: IMP-01 high, IMP-02 medium, IMP-03 low, IMP-04 low, IMP-05 low

## R1 — IMP-01 の根拠: E9 は印の話を支えず、unknowns の 1 つ目は推測でなく既知の挙動
- 対象: impact.findings[IMP-01].evidence / unknowns / basis
- 種類: 根拠
- 指摘: evidence の E9（premises.md:19）は P12「同じリポジトリの枝からの PR の実行から Environment の値が読めない」で、印 v* の実行の話ではない。IMP-01 の主張（印の commit の workflow が正式な鍵を読む）を支える行はどちらの evidence にも無い。E8 は「pre-push は feat/st* だけ」（.githooks/pre-push:30）しか言わない。unknowns の 1 つ目「印の push で走る workflow は印の commit の定義」は GitHub の確立した挙動（GITHUB_SHA・workflow ファイルとも印の指す commit）で、未確認の本体ではない。本当に未確認なのは 3 つ目（エージェントの資格情報で印を push できるか）と、ルールセットで印の「作成」を本人だけに限れるか。high は妥当（鍵の流出は気づけず、配った APK の署名は取り替えられない）だが、guess の中身が分かれていないので読む人が重さを測れない。「Environment の配る対象は印の名前だけを見る」ことの公式文書の該当箇所を evidence に足し、unknowns は 3 つ目とルールセットの作成制限の可否に絞るのが正しい。
- 直し方の案: E9 を印の規則の文書（deployment branches and tags）に差し替え、unknowns を「印の push の権限」「作成を限るルールセットの有無」に絞る。
- 処置: fixed: IMP-01 の E9 を design.md:160（配布用の Environment は印 v* にだけ開く）に差し替え、E20（design.md:150 の手順）を足した。unknowns を「エージェントが印を push できるか」「印の作成を限れるか（本人の資格情報で動くエージェントと分けられない）」に絞り、workflow の定義の件は「既知の挙動と読むが確かめていない」と下げた。high は残す

## R2 — IMP-01 の閉じ方: safe_now は本人が置く順を守ることだけで、機械の止めが無い
- 対象: impact.findings[IMP-01].if_deferred.safe_now / interim、impact.acceptance
- 種類: 閉じ方
- 指摘: safe_now は「ST52 の間は Environment に本物の鍵が無い」。ST52 が鍵を置かないのは本当だが、鍵を置くのは本人の手作業で、design.md:150 の docs/release.md の手順は「Environment と Secrets を置く」を「ルールセットを入れる」より前に並べている。しかもそのルールセット（D2）は削除と付け替えだけを拒み、IMP-01 の経路（作成）には効かない。つまり手順どおりに進めると、IMP-01 が残ったまま鍵が置かれる。acceptance に「IMP-01 に本人が答えている」はあるが、答える前に鍵を置けない機械の止めは無い。さらに R8（rules）の「ルールセットが無ければ公開しない」は、改ざんできる同じ workflow の中の判定なので、IMP-01 の攻撃者には効かない。
- 直し方の案: docs/release.md の順を「ルールセット（作成制限を含む）か required reviewers → 鍵」に直し、鍵を置く手順の先頭に IMP-01 の答えの確認を置く。R8 の「公開しない」は改ざんで外れる旨を限定して書く。
- 処置: fixed: IMP-01 の text と undefined に、docs/release.md の並び（Environment と鍵がルールセットより先）とルールセットが作成に効かないことを書き、options の先頭を承認（required reviewers）・ルールセットと承認を鍵より先にする並べ替えにした。R8 の規則に「workflow の中の確かめは書き換えた実行に効かない」を足し、acceptance にも並びの問題を書いた。手順書と design の並びそのものは reading の段では直さない（IMP-01 の答えで spec の段が直す）

## R3 — IMP-02: 再現手順が主張と合わず、同じ種類の順序の抜けが他にもある
- 対象: impact.findings[IMP-02].verify / derived、rules[R3].legit
- 種類: 根拠 / 新しい帰結
- 指摘: (a) verify は「v0.4.0-beta.2 の後に移行を足し、その前の commit に v0.4.0 を打つ」。「その前」が beta.2 より後の commit なら beta.2 の DB は正式版で起動でき、failure を再現しない。text の条件（最後のベータ版**より前**の commit）に合わせて「beta.2 の印の commit より前に打つ」と書かないと、確かめが通って誤って安心する。deps の D4（BUILD_VERSION の出所）もこの主張の根拠にならない。(b) 同じ原因（印の順を判定が見ない）の帰結が、spec の判定（distribution/spec.md:112-116。main の祖先・出所の版の一致・配布物の揃いだけ）にはほかにもある。すでに v0.4.0 や v0.4.0-beta.3 がある出所の版に、後から v0.4.0-beta.2 を打つ（印は消せないので取り消せない）と、versionCode は 40002 で 40099/40003 より小さく、Android は上書きを断る。ベータ版を入れた携帯は、その印の後も新しい版へは進めるが、公開した beta.2 は誰も入れられず、版番号が使えなくなる。search: grep で「小さい・最大・前の印」を spec・design に探したが、印を打つ順を見る規則は無い。(c) 機械で見られる: 正式版の印のとき、同じ版の最後のベータ版の印が祖先にあるか、を `git merge-base --is-ancestor` で見る一行で IMP-02 は閉じる。options が「確かめる」だけでなく、この判定の追加を挙げていない。
- 直し方の案: verify を text の条件に合わせ、印の判定に「同じ版の既存の印より前の commit・小さい番号の印は公開しない」を option に足す（(b) は別 IMP にするか IMP-02 に含める）。
- 処置: fixed: IMP-02 の verify を「beta.2 の commit より前に v0.4.0 を打つ」に直し、後から小さいベータの印を打てる帰結を text と derived に足した。options の先頭に git merge-base --is-ancestor と既存の印との大小の判定を置いた。deps の D4 を外した。R3 の規則の legit も直した

## R4 — situations[update].can: 開発版が毎日同じ versionCode になることを確かめていない
- 対象: impact.situations[update].can / situations[mixed].can
- 種類: 言い切り
- 指摘: update の can は「携帯は versionCode が上がるので上書きで入る」と、ベータ版・正式版を前提に書く。一方 spec（distribution/spec.md:90）は開発版を毎日同じ versionCode 40000 にする。同じ識別子 dev.ashiato.collector.dev に毎日の APK を上書きで入れる場面は、「versionCode が上がる」では説明できず、同じ番号の上書きを端末の取り込みの種類（adb・Obtainium などの更新の確認）が通すかは、この change のどこにも確かめが無い。mixed の can は「開発版は並べて入る」としか言わない。小さい番号を断るのは Android の既知の規則だが、同じ番号を既存の署名で更新できるかは取り込みの経路に依る。
- 直し方の案: update の can を「ベータ版・正式版は versionCode が上がる」に限り、開発版の同じ番号の上書きは unknowns に置く（あるいは端末で 1 回確かめる）。
- 処置: fixed: situations[update].can を、ベータ版・正式版は versionCode が上がる、開発版の同じ数の上書きは adb でだけ確かめた、に限った。scope.residual に残した

## R5 — 依存の見落とし: D1・D2 は tasks に直す先が無い、D14 の leads_to が別の場面
- 対象: impact.findings[IMP-05]、dependencies[D14].leads_to
- 種類: 依存の見落とし
- 指摘: IMP-05 の主張は実物と合う（tasks.md の 3.2 は build-stamp.ts と vite.config.ts だけ。web/vitest.config.ts と web/e2e/st49-client-version.spec.ts は tasks・design のどこにも出ない）。ただし「low」は軽い。この 2 つは ST52 自身の PR の CI（vitest・web の e2e）を確実に落とすので、直す先が tasks に無いまま実装に渡ると、配る形の組み立てより前に引っかかる。low のままなら、少なくとも tasks に 3.2 の直す先として 2 ファイルを足す必要がある（medium の方が実態に近い）。また D14（nightly を動かすと git fetch --tags が rc=1）の leads_to は S3 だが、S3 は「正式版の後に出所の版を上げ忘れる」場面で fetch とは無関係（IMP-04 の場面）。D14 に紐づく場面か IMP が無く、手元の clone・worktree の利用者（本人・エージェントが tag を取る台本）への手当てが決まらない。search: tools・scripts・.githooks の `git fetch` は `-q origin` だけで --tags は見つからず、ハーネスの台本は無事だが、本人が手で `git fetch --tags` する場面は覆えていない。
- 直し方の案: tasks 3.2 に 2 ファイルを足して IMP-05 を medium に、D14 の leads_to を実際の帰結の IMP に付け直す（無ければ新しい IMP を起こすか docs/release.md に手当てを書く）。
- 処置: fixed: IMP-05 を medium にし、ST52 自身の PR の CI を落とすことを text に足した（tasks に 2 ファイルを足すのは spec の段。IMP の答えで直す）。D14 の leads_to を新しい IMP-06（low。nightly を動かすと手元の fetch --tags が rc=1）に付け直した
