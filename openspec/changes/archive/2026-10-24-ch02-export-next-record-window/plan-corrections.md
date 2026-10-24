# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-10-23 6.2

- 旧: `CT st09_exp_`
- 新: `CT export_`
- 理由: ST09 を merge --no-commit で重ねて撮った証跡は merge --abort の後に作業ツリーと食い違い、Task gate で常に古いコード扱いになる（ST09 の移行がテスト DB に残り export の試験 77 本も落とした）。CH02 の木では st09_exp_ が 0 本で FAIL。ST09 の上での試験は確認バッチの統合の木に任せる
- 実証: evidence.jsonl の 2026-10-23T09:33:29+00:00（FAIL、rc=0、ログ .superpowers/sdd/logs/20261023-183329-evidence-6.2-0-03db9f.log、HEAD cce182b）
- 承認: ユーザー 2026-10-23 の会話（6.2 は CT export_ だけにし、ST09 の上での試験は確認バッチに任せる案を承認）
