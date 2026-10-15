# tools/check-immutable.d/

Story ごとの `tools/check-immutable.sh` の節。`tools/check-immutable.sh` の末尾が名前の順に、同じ shell の中で `.` で読む。

- 名前は `YYYYMMDDHHMM_st<NN>-<slug>.sh`（作成時刻。移行と同じく連番にしない —— 並走する Story が番号を取り合う）
- 使える変数・関数は `tools/check-immutable.sh` の末尾の注記のとおり。節の中で `exit` しない（後ろの節が走らない。落とすときは `exit 1` だけ）
- `tools/check-immutable.sh` の本体には書き足さない（並走する Story がみな末尾を取り合い、どの 2 本を合わせても衝突した。2026-10-15）。
  `feat/st*` が本体の末尾（最後の節の後ろ）に書き足すと `tools/check-shared-appends.sh` が落とす（途中の既存の節を直すのは通す）
