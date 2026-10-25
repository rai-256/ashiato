# CH07 — design

> questions の段（2026-10-25）は、review/deep.md R3（kind: daily）の仮の決定だけを置いた。残りは spec の段が書く。

## Decisions

### D1（仮）. harness2 #112 が CH07 より先に入っても、順を変えず CH06 へ差し戻さない（premises.md C8）

#112 が先に `.harness2` に入ると、宣言の無い木の全部で `verify-run` のたびに「注意」が出る。注意は表示と記録だけで status を変えない（premises.md P3）。
CH06 の下流がその注意を見て宣言の行を CH06 に入れても失うものは無く、CH07 はその行を確かめる側になる。CH07 は注意を止めるために #112 や CH06 の順を変えない。
反転条件: 注意が CH06 以外の走っている change の下流を止める・誤った直しを起こす（注意を見て `verify-env.sh` に宣言を足す・`none` を出す）と分かったら、#112 の merge を CH07 の main 入りの後にするよう harness2 側に返す。
