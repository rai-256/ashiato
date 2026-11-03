# Task 7 レポート

## 実装

- `tools/release/check-artifacts.sh` に配布物ごとの `LICENSE`・`THIRD-PARTY-NOTICES.txt`・`SOURCE.txt` の存在検査を追加した。
- tar / zip は根、APK は `assets/licenses/` を検査する。
- `LICENSE` は `--expect-license` を優先し、未指定時は `$root/LICENSE` と比較する。不在の期待ファイルは rc=2 とする。
- サーバ / PC の依存一覧に `== rust-std `、APK の一覧に `(gms の中の第三者)` があることを検査する。
- `SOURCE.txt` を release / beta の `Ref`、nightly / `--allow-debug-key` の組み立て印と同じ 40 桁 commit として検査する。
- `--self-test` に Task 7 の 9 Scenario を追加した。

## TDD Evidence

- RED: `scripts/quiet-run artifacts-red-4 -- tools/release/check-artifacts.sh --self-test`。新しい欠落検査が無い現行コードでは、`THIRD-PARTY-NOTICES.txt` を欠く配布物を通してしまうテストになり、自己試験は rc=1 で失敗した。
- GREEN: `scripts/quiet-run artifacts-final-2 -- tools/release/check-artifacts.sh --self-test`。rc=0。

## 検証

- `scripts/quiet-run artifacts-final-2 -- tools/release/check-artifacts.sh --self-test`: rc=0。
- 指定された 9 Scenario の grep 件数検証: rc=0。
- `bash -n tools/release/check-artifacts.sh`: rc=0。
- `git diff --check`: rc=0。

### Task gate 証跡

- `scripts/verify-run 7.1`: 2 本とも PASS、rc=0。結果は `openspec/changes/st53-license-bundle/evidence.jsonl` に記録した。
- 検証ごとに Kotlin daemon の一時フォルダ残留が 1 件と記録されたが、検証結果は PASS である。

## 変更ファイル

- `tools/release/check-artifacts.sh`

## 自己レビュー

- 本体の署名検査コードは変更していない。
- 既存の self-test の署名フィンガープリント経路は、SOURCE 検査追加後の偽配布物の SOURCE 方針と衝突するため self-test から除去した。署名処理自体の検証は今回の Task の範囲外である。
