# impact-review — ch08-erasure-nested-and-late-stays

seen: IMP-01 medium, IMP-02 medium, IMP-03 low

## R1 — 作り直しが本人の見ていない所で本文を消すことが、IMP にも situations にも無い（D4 の leads_to が違う先）
- 対象: impact.dependencies[D4].leads_to / impact.findings（欠け）/ impact.situations[update]
- 種類: 新しい帰結
- 指摘: D4 は holds=no で「P と重なる一覧の滞在の本文が消えうる（仕様どおり。確認で前もって知らせる）」と書くが、leads_to は S2。S2 は気分の場面（IMP-01）で、この帰結の場面ではない。
  帰結そのもの（本人が何も押していない後着の位置の取り込み `lib.rs:1967`、基準を変えた `rebuild_all` `lib.rs:2388` で、一覧に出ていた L の本文と前の版が戻らない形で消える）を扱う IMP が無い。
  「確認で前もって知らせる」が効くのは、P を消した時に L が一覧に出ていたときだけ（Q4・scope.preconditions「1 件以上あるときだけ添える」）。
  P を消した後に一覧に出た L（P の後の別の作り直しで P と重なる形になったもの）や、確認を見た後の基準の変更には、知らせる場面が無い。
  `/stays/rebuild` は web に呼び口が無く（`web/src` に `stays/rebuild` なし。`tools/verify-prep.sh` だけ）、日常の経路は取り込みの方になる。
  situations[update] は起動時の手当てと基準の変更に触れるが、取り込み経由で常時起きることと「戻せない・本人は気づけない」を `can` / `cannot` に書いていない。
  high にはならない（本人が P で本文を消した意思の延長で、DB には消した印の行が残る）が、medium の IMP が要る。
- 直し方の案: D4 を根拠にした IMP（本人の操作なしに L の本文が消える・知らせる場面が確認の時点だけ）を足し、D4.leads_to をそれに付け替える。
- 処置: fixed: IMP-04（medium。本人の操作なしに取り込み・基準の変更で一覧の重なる滞在の本文が消える）を足し、D4 の leads_to を IMP-04 に、situations[update]・scope.residual・W2 の decisions に足した

## R2 — IMP-01 は design D2 自身の反転条件に当たるのに、「受け入れる」と「吸収の表にも行を書く」の 2 択になっている
- 対象: impact.findings[IMP-01].options / .unknowns
- 種類: 閉じ方
- 指摘: design.md D2 の（仮）は「`f` の行の識別子が作り直しのたびに変わって困る読み手（気分・場所の付け先など）が見つかったら、引き継いで書き換えた後に…消す形にする」を反転条件にしている。
  IMP-01 は気分の読み手（`feelings.rs:461`・`:673`）が困ることを見つけたものなので、条件に当たる可能性があるが、IMP-01 の options にこの反転が無く、D2 の（仮）との関係に触れていない。
  IMP-01 の evidence は吸収された L（D2 の (a)）の経路だけ。(b)（引き継がずに新しい行で隠す）では L は吸収されず `user:purged` になるだけなので、
  L の気分は吸収先へどころか、置き換えの新しい行へも出ない。同じ帰結だが、IMP-01 は (b) を歩いていない。
  また unknowns の「P と重なる一覧の滞在に気分を書くことがどれだけあるか」は、`FeelView.tsx:168` が `hidden_stay_records` を画面に出すので、
  「数だけは出る」ことまで言えるのに書いていない（本人が気づけるかの材料）。
- 直し方の案: options に D2 の反転条件の形を足し、(b) の経路と、画面に出る件数（`FeelView.tsx:168`）を derived / unknowns に書く。
- 処置: fixed: IMP-01 の options に design D2（仮）の反転条件の形（引き継いで書き換えた後に前の版を含めて消す）を足し、(b) の経路を辿っていないことを unknowns に、画面が件数を出すこと（E15 `FeelView.tsx:168`）を text に書いた

## R3 — IMP-03 の「本人は消す操作をしていない」は、画面の文を読めば言える範囲が違う
- 対象: impact.findings[IMP-03].text / .unknowns
- 種類: 言い切り
- 指摘: unknowns は「書き出しの画面の文（読んでいない）」だが、`web/src/messages.ts:66` は「この書き出しの後に消した記録が N 件あります。書き出したファイルの中には残っています」で、
  `SettingsView.tsx:277` が 1 件以上で出す。文は「本人が消した」とは言わず、書き出したファイルに残る記録が後で消えた事実を伝える。
  本文が実際に消えたのでこの数え方は合っており、「本人は消す操作をしていない」を問題の側に置く text と options が 1 つ（受け入れる）なのは、読めば閉じられる。
  low は妥当。unknowns が読めば分かることを未確認のまま残している。
- 直し方の案: unknowns を `messages.ts:66` の文で閉じ、text の「本人は消す操作をしていない」を文の事実に合わせる。
- 処置: fixed: IMP-03 の text を画面の文（E16 `messages.ts:66`）に合わせ、unknowns を「直接消していない滞在が入ると気づけるか」に直した

## R4 — D8（ST49）は unknown のまま残すより、止まる仕組みを書いて閉じられる
- 対象: impact.dependencies[D8]
- 種類: 依存の見落とし
- 指摘: D8 の assumes は「削除の台帳に行を積むコードは deletion.rs と stay_store.rs の既知の所だけ」。CH08 の D3 は `deletion.rs` の既存の `ledger(...)`（`deletion.rs:945` の呼び方）と同じ並びで積むので、
  ST49 が `ledger` の関数を直せば追従する。一方 ST49 の tasks.md:210 は、足された INSERT を `st49_ledger_inserts_name_server_version`（D10）が落として知らせると書く。
  つまり「気づけずに壊れる」ではなく、先に merge した側の試験が落ちる形。unknown の理由と affects が実物（ST49 の D10・tasks 11.1）より弱い。
  ただし ST49 の移行は `core.event` / `event_version` の列も足す（design.md D7）ので、CH08 が書く `erase_bodies` の UPDATE がその列を触らないことは、どちらの change にも確かめが書かれていない。
- 直し方の案: D8 に ST49 D10 の試験が止める旨を書き、`erase_bodies` の UPDATE と版の列の関係を確かめる 1 行を unknowns に足す。
- 処置: fixed: D8 に ST49 の試験 `st49_ledger_inserts_name_server_version`（E17）が止めることと、本文の消去と ST49 の版の列の関係はどちらの change も確かめていないことを書いた（holds は unknown のまま）

## R5 — 重大度: high が要るものは見つからなかった（IMP-01・IMP-02 の medium、IMP-03 の low は妥当）
- 対象: impact.findings の severity
- 種類: 重大度
- 指摘: IMP-01 は気分の本文が残り（`feelings.rs:461` は表示しないだけ）件数も画面に出るので、データを失わず medium。IMP-02 は消した印の位置が残るだけで、戻せない滞在は本文ごと消す口が使える（R1 の blocks_legit=true は、はみ出しの位置 2 件の消した印を外せないことで、記録を捨てるか DB を触るしかない形には当たらない）。
  根拠が薄い high も、根拠があるのに低い medium / low も見つけていない。ただし R1 の IMP が足されるなら、それも medium。
- 直し方の案: 変更なし。
- 処置: kept: 重大度は変えない。足した IMP-04 も medium
