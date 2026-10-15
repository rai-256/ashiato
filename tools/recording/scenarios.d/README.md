# tools/recording/scenarios.d/

Story ごとの録画のシナリオ。ハーネスの `record_summarize.py` が `recording.json` の `scenarios` の後ろへ、名前の順に足す。

- 名前は `YYYYMMDDHHMM_st<NN>.json`（作成時刻）。中身は `recording.json` の `scenarios` と同じ形の配列
- 録画しない Story は `{"waive": {"ST<NN>": "理由"}}`（`{"scenarios": [...], "waive": {...}}` も可）
- **Story は `recording.json` を変えない。** 並走する Story がみな `scenarios` の末尾を取り合い、どの 2 本を合わせても衝突した
  （2026-10-15）。`feat/st*` が `recording.json` を変えると `tools/check-shared-appends.sh` が落とす（本体を直すのは chore / fix の PR）
