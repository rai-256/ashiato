# ST58 settle report

## R1 / R13 / R14 / R15 / R16 / R17 / R18 / R19 / R20 / R9 / R10 / R12

- `record-rollback` の CLI 組み立てに版と種別を渡し、`replay` を positional path に合わせた。控えが無い replay は no-op にした。
- サーバに `--version` を実装し、既存インストールの版読取りを可能にした。
- 更新・戻しの停止を停止待機の既存コマンドに置換し、DB 再起動後は接続可能になるまで待つ。プログラム移動の前に `moved` 段を永続化し、待受後の失敗は rollback と記録しない。
- Android は置換開始印を cleanup から除外し、通知で発行した一回限りのトークンだけが更新を開始する。
- 配布物検査に `ASHIATO_SERVER_TEST_` を追加し、P20 は Windows CI 未実行のため確かさ 3 に戻した。

## R11

- A: 失敗 DB の写しを保持するか消すかは、本人の削除した記録と調査材料のどちらかを失う選択である。Q10 を `deep.md`、`deep-questions-r3.json`、`docs/briefs/ST58-deep-r3.html` に作成した。

## 検証

- `cargo test -p ashiato-setup st58_` — PASS（1 passed）
- `cargo test -p ashiato-server st58_` — PASS（0 tests selected）
- `cargo test -p ashiato-collector-windows st58_` — PASS（19 passed）
- Android unit test は Gradle が実行中に応答を返さず、完了結果は未取得。

## 追加の settle（R1 / R4 / R5）

- R1: command builder による `record-rollback` の全引数と、終了コード 5 の pending 経路を固定した。
- R4: `migrate` の機械可読な失敗行から種別・落ちた移行名・SQLSTATE を outcomes の理由へ写すようにした。
- R5: 再開した戻しでも理由付き `rolled_back` を書く既存修正を、status の読み取り試験と照合した。
- 検証: `scripts/quiet-run st58-settle-r1-r4 -- cargo test -p ashiato-setup st58_` — rc=0、2 passed。

## 処置記録の整備（R1〜R10 / R12〜R22）

- 直近の recovery 修正コミットに対応する R1・R4・R5・R9・R10・R12〜R20 の処置を、自己参照ではなく設計決定または change 直下の成果物を指す形式にした。
- R2・R3 は Windows 通しが採る実状態と self-test の判定範囲、R6〜R8 は PC/携帯/状態の既存検証経路、R21・R22 は更新実行経路へそれぞれ対応付けた。
- R8 の既存の詳細な壊し試験を triage が根拠として認識できる形式に補った。R11 は既に Q10 として本人の判断を待つため、この回では変更していない。

## 検証（この回）

- `python3 scripts/review_triage.py . st58-update-apply-rollback --no-verdict` — rc=0。
- `scripts/quiet-run st58-settle-installer -- cargo test -p ashiato-setup st58_` — rc=0、2 passed。
- `scripts/quiet-run st58-settle-pc -- cargo test -p ashiato-collector-windows st58_` — rc=0、19 passed。
- `tools/release/update-apply-install.sh --self-test` — rc=0。
