## D11
- 判定: 否
- 根拠: 条件 4（`rests_on` に挙がっていない土台の決定）で否。`rests_on` は `["D1"]` だけだが、D11 の置き場所は D2（仮）に依る。
  `chosen` 自身が「かたまりの中に置くと check-shared-appends の『動かしただけ』が外れる」を理由にしており、その判定は D2 の規則
  （design.md:46-58「読む口のかたまりを別の位置へ動かすだけの変更…は通す。かたまりの中の注記の行は比べない」、`tools/check-shared-appends.sh:43` `mouth_sig`）。
  さらに D11 の名で D2 の道具そのものを変えている: `tools/check-shared-appends.sh:33` `PREP='`*.d/` の節の前の準備'`（「CH01 design D11」）と守る範囲 1 の起点の変更・`--self-test` 3 件（design.md:131-133、review/code.md:240 R19 の処置「fixed D11 仮」）。
  D2 が反転（読む口の移動を通さない・見出しの型を狭める等）すれば、D11 の「かたまりの外の直前」という置き場所の理由と PREP の守る範囲が変わる。`rests_on` に D2 を足すべき。
  条件 1〜3 は満たす（参考）: 1) 変えるのは検査の台本（`tools/check-immutable.sh:1174-1179` の 3 行と注記、:1340-1341 の再実行は残り `app_psql()` の定義は :1179 の 1 か所）で、データの uncaptured / discarded / exported / rewrite-all は無い。
  ST09 なしの木で `.superpowers/sdd/logs/20261023-190010-immut-final.log`・`20261023-194327-immut-finish.log` とも末尾「書き換え禁止 OK」・NG 0 行、ST09 ありの直す前は `20261023-184029-immut-r1-red2.log` で `role "ashiato_app" does not exist`。
  `tools/check-shared-appends.sh --self-test` は手元で「self-test OK」rc=0。2) 費用は 3 行と `db-roles.sh` の再実行で、要件の本文は変えない。Global Constraint を越える理由は design.md:140-142 にある。3) `follows` は null で該当なし。
