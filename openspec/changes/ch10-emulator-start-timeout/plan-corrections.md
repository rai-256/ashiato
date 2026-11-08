# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-11-07 5.1

- 旧: `bash -c 'grep -q "error: /dev/kvm を開けない" AGENTS.md && ! grep -q "数十倍遅い" tools/android-emulator.sh && grep "手元: \`
- 新: `./openspec/changes/ch10-emulator-start-timeout/check-5.1.sh`
- 理由: 旧コマンドは code span の中に \` を含み、検証として拾うと「grep "手元: \」で切れて bash の構文エラー（rc=2）になる。同じ 3 つの確かめを check-5.1.sh に置いた（壊すと 3 つとも落ちることを doctor が確かめた）
- 実証: evidence.jsonl の 2026-11-07T05:11:44+00:00（FAIL、rc=2、ログ .superpowers/sdd/logs/20261107-141144-evidence-5.1-0-cf53ec-vne2qu377.log、HEAD dbf415f）
- 承認: machine: evidence.jsonl 2026-11-07T05:36:32+00:00 PASS
