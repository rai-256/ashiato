# CH22 の処置の fix report（settle が追記する）

## 2026-11-20: R39, R40

- R39: この fix report に処置内容と検証結果を追記した。
- R39 の成果物はこの回の処置コミットに含める。
- R40: `soak_collector_process` のバックグラウンド終了時、親の一時ディレクトリが既に削除されていた場合の status ファイル書込みを、標準エラーに漏らさず無視するようにした。通常の実行では status を従来どおり記録する。
- 検証: `scripts/quiet-run ch22-settle-self-test -- tools/release/soak-install.sh --self-test`
  - rc=0（12秒）
  - 出力末尾: `collector pid changes=0` / `collector outage_notice_unavailable=0` / `OK: soak-install self-test`
