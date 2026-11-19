# CH19 機械が承認した検証コマンドの訂正の確かめ

## 4.3 2026-11-19T14:44:01+00:00
- 壊したもの: 「この change が触ったファイルは予定のパスの中だけ」を、作業ツリーで予定外のパスに触れて壊した。(a) 未追跡の `crates/server/src/ch19_probe_untracked.rs` を作る、(b) 追跡済みの `tools/smoke.sh` の末尾に 1 行足す、(c) 足された許可 `docs/handoff/CH19.md` の隣の `docs/handoff/CH20.md` を作る（許可が handoff 全体に広がっていないか）。それぞれ 1 つずつ入れて走らせ、毎回元に戻した。前後の `git status --porcelain` と `git diff` の sha256 は同じ（どちらも空）
- 実行: ./openspec/changes/ch19-emulator-only-class/check-allowed-paths.sh
- 結果: 落ちた
