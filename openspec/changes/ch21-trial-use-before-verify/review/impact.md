# impact-review — ch21-trial-use-before-verify

seen: IMP-01 medium, IMP-02 medium, IMP-03 medium, IMP-04 medium, IMP-05 low

## R1 — 席が走る間、確認バッチの作業ツリーの錠も握られる（IMP-02 が塞がるものを「段」だけにしている）
- 対象: impact.findings[IMP-02].text / dependencies[D6] / evidence[E7]
- 種類: 新しい帰結
- 指摘: グラフは verify_batch.sh を `verify_tree_lock` を握ったまま待つ（.harness2/graph/harness_graph/verify.py:108-109）。この錠は「verify_batch.sh / 記録 / fix の撮り直し」を直列にする錠（effects.py:181-186）で、verify.py:85 では fix 側の撮り直しも同じ錠を取る。席が最大およそ 1 時間 50 分走る間、別の確認バッチ・撮り直し・fix も待たされる。IMP-02 は「その確認バッチの段」と「手元が落ちたらやり直し」だけで、この錠の待ちが無い。E7（effects.py:157）は `run` が時間の上限を持たないことしか示さず、錠を握ることは示していない。
- 直し方の案: D6 / IMP-02 に verify.py:108 の錠を evidence として足し、他の確認バッチ・fix の待ちを帰結と受け入れの手当て（上限の縮め方・錠の外で走らせる案）に書く。
- 処置: fixed: D6・IMP-02 に verify.py:108 の錠と effects.py:182（撮り直し・fix も同じ錠）を evidence（E13・E14）として足し、別の確認バッチ・撮り直し・fix も待たされることを帰結に書いた。錠の外で走らせる案を options に足した（harness2 の変更が要るので Q12 の範囲の外と明記）

## R2 — IMP-04 の「出所が変わるたびに」は、警告で終わった回の再走を落としている
- 対象: impact.findings[IMP-04].text / situations[normal・update].can / overview の「効くのは…出所が変わった回」
- 種類: 言い切り
- 指摘: spec.md:240 と design.md:103 は、前の回が「一通り終えた・時間の上限で打ち切った」以外で終わると同じ出所でも走り直すと定める。インストーラが Sandbox で通らず毎回「インストールまで届かなかった」の警告で終わるなら、出所が同じでも全バッチで組み立て（初回 60 分）と Claude 45 分を使い直す。S3 の not_when にも「前の回が警告で終わった」が入っている。IMP-04 の「出所が変わるたびに」と overview の「出所が変わった回だけ」は、この失敗の形では成り立たない。IMP-02 の遅れと IMP-03 の利用枠の帰結も同じ分だけ広がる。
- 直し方の案: IMP-04 に「警告が続く限り毎バッチ走る」を帰結として書き、「届かなかった」を打ち止めの終わり方にする（同じ出所では数えて止める）か、1.1 の install が通るまで本線に入れない条件を release_ok に置く。
- 処置: fixed: IMP-04 に、警告で終わった回は同じ出所でも走り直すので確認バッチのたびに組み立てと 45 分を使うこと（IMP-02・03 も広がる）と、「届かなかった」を走り直さない終わり方に数える案を書いた。overview の走る条件に「前の回が使い終えていない」を足した。仕様は変えない（reading の段は仕様に触らない）。重大度は medium のまま（手間と時間で、記録は失わない）

## R3 — IMP-01 は「その回の録画」だけと書くが、残った席の Sandbox は次の回以降も席と録画を止める
- 対象: impact.findings[IMP-01].text / situations[recovery].cannot / dependencies[D5]
- 種類: 重大度
- 指摘: record-run:317 は動いている Sandbox があれば止まり、「本人のものかもしれないので閉じない」とある。席の規則 R1 も同じく既に動いている Sandbox を閉じない。席のプロセスごと落ちて Sandbox が残ると、次の確認バッチの席は「別の Sandbox が動いていた」の警告で走らず、録画も止まる。本人が気づいて閉じるまで毎回続く。警告の文は、残ったものが前の席の Sandbox かどうかを区別しない（spec.md:82 は閉じ損ねた席の回の段落にだけ書く）。IMP-01 の `undefined`（誰が閉じるか）は、「その回」の話で止まっている。
- 直し方の案: 席が起こした Sandbox の印（pid や名前）を置き場に残し、次の席が自分のものだけ閉じ直す。または警告に「前の席の残りかもしれない」と書く。
- 処置: fixed: IMP-01 の題と本文を、残った Sandbox が本人が閉じるまで次の回以降の席と録画も止めることに直し、undefined に警告が前の席の残りを区別しないことを、options に印を残して閉じ直す案と警告の文の案を足した。recovery の cannot も同じに直した。重大度は medium のまま（警告の段落で本人が気づける・閉じれば戻る）

## R4 — rollback 欄の「1 行を消せば戻る」は、公開済みの題とメモ・置き場を落としている
- 対象: impact.situations[rollback].can / cannot
- 種類: 言い切り
- 指摘: `can` は「準備の台本の 1 行を消せば…いまの形に戻る」だが、`cannot` は「この分析では見つけていない」。memory には、一度 push した題は公開の origin の履歴に残る（one_way_door）と書いてある。1 行を消しても、すでに手順書・PR 本文に載った題、`C:\dev\ashiato2-trial-use\` の置き場、workbox.json の `host_only`、CI の自己検査の行は戻らない。「見つけていない」は、同じ分析の memory と矛盾する。
- 直し方の案: `cannot` に公開済みの題と置き場を書き、戻す手順に workbox.json と CI の行も含める。
- 処置: fixed: rollback の can に workbox.json と CI の行を別に消すことを、cannot に公開済みの題と手元の置き場が戻らないことを書いた

## R5 — D9「host_only は箱の外で走らせる」の根拠が、当たり方を示していない
- 対象: impact.dependencies[D9] / evidence[E9]
- 種類: 根拠
- 指摘: E9（tools/workbox.json:25）の quote は `"host_only": [` の 1 行だけで、何に当たるかを示さない。当たり方は検証コマンドの文字列への `re.search` で、台本の中身は見ない（archive 済みの ch13 の premises.md:9）。tasks 1.1 の `tools/trial-use/(probe|seat)\.sh` は、検証コマンドにその字面がある Task 1 と 11 には当たる。しかし `verify-prep.sh` 経由で席が呼ばれる確認バッチの経路は作業箱の検証を通らないので、「箱の中で Sandbox を探して落ちない」の裏づけは 1 つの Task の字面に限られる。
- 直し方の案: D9 の evidence に ch13 の premises の判定規則を足し、当たるのは検証コマンドの字面のときだけだと書く。
- 処置: fixed: D9 の assumes を検証コマンドの字面への当たりに直し、ch13 の premises.md:9 を E15 として足した。確認バッチの経路は作業箱を通らないので当たらなくてよいことを affects に書いた

## R6 — D2 は `holds: unknown` なのに、観測した内容がそのまま確かめられる
- 対象: impact.dependencies[D2] / findings[IMP-05]
- 種類: 根拠
- 指摘: tools/verify-prep-device.sh:82 は `grep -v install_on_emulators` で残った行に `adb|install` があれば落とす。足す行は `./tools/trial-use/seat.sh "$tag" "$out/manifest.md" || echo "warn: …"` で、予定どおりなら語は当たらない。`unknown` ではなく、`holds: yes` で条件（警告の文に英字を入れない）を書けばよい。一方、D2 の「予定の行には無い」は design.md:66 の警告文の写しで、実装後の行を見るものではない。IMP-05 の `feasibility: unverified` も、実物の grep で確かめられる。
- 直し方の案: 実装後に `grep -v install_on_emulators tools/verify-prep.sh | grep -Eq 'adb|install'` が当たらないことを tasks の検証に置き、D2 を yes にする。
- 処置: fixed: D2 を holds: yes にし、条件（警告の文に install の英字を入れない）と、CI の自己検査が実装の後の行で確かめることを affects に書いた。tasks の検証の追加は reading の段では仕様に触らないので行わない（既存の verify-prep-device.sh --self-test が CI で同じ grep を走らせる）

