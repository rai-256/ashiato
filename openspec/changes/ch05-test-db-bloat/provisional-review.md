## D8
- 判定: 否
- 根拠: 条件 1（失わない側を選んでも `discarded` が起きる）に当たる。`keeps` の「外した後も、どの作業ツリーの volume だったかを検証のログから引ける」は、ログを書いた作業ツリーが生きている間しか成り立たない。
  - 1 行の行き先: tools/docker-tidy.sh:129・:131 は標準エラーへ出すだけ。tools/verify-env.sh:126 から呼ばれ、`.harness2/scripts/evidence.py:368-387` が `sdd_dir(root) / "logs"`（= その作業ツリーの `.superpowers/sdd/logs/`。`evidence.py` の `sdd_dir` が `*` の `.gitignore` を置く git の管理外）に書く。
    evidence.jsonl に残るのはログの path と末尾 20 行の `summary` だけで、準備の段の行は入らない（`grep -c "残す volume\|docker-tidy: コンテナ" openspec/changes/ch05-test-db-bloat/evidence.jsonl` → 0）
  - そのログを持つ作業ツリーも、いずれ消える。docker-tidy が相手にしているのはまさに「作業ツリーが消えた」場合で、消えるときには git の管理外の `.superpowers/sdd/logs/` ごと消える。そうなると、volume と作業ツリーの結び付きは docker からもログからも引けない（R20 の `loss: discarded` は、消える時刻が遅くなるだけで残る）。
    検証の準備の外から手で `tools/docker-tidy.sh` を呼んだときは、どこにも残らない（design.md:161 の Risks もそう書いている）
  - design D8 の反転条件（design.md:153「行では足りない（ログの残る作業ツリーも消える）と決めたら、main の checkout の下の記録のファイルへ…（spec を MODIFIED する）」）は、この誤りを自分で挙げている。本当に失わない側は記録のファイルの方で、`cost` に spec の MODIFIED（片付けの Requirement の本文 spec.md:53 の「標準エラーに 1 行」）が入る。そのため条件 2（要件の本文を変える）に当たるかどうかを、人間に問わずに決めてはいけない
  - 条件 4: `rests_on` に D5（design.md:112。片付けを `reset` のときだけ verify-env.sh から呼ぶので、1 行が verify-run のログに入る）が無い。`keeps` の「検証のログ」は D5 があって初めて成り立つ。D5 も（仮）
  - 条件 2（いまの実装の側）: 外すコンテナごとに `docker inspect` を 1 回（tools/docker-tidy.sh:92）。高くつかず、範囲も越えず、spec.md:53 の「1 行」に足すだけで本文は変えない —— ここは確か。条件 3: `follows` なし
