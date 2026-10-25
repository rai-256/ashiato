# CH07 検証の準備が作り直した DB を宣言する（上流）

change: `openspec/changes/ch07-declare-verify-reset/`。CH06 が立てる capability `verify-environment` に Requirement を 2 つ ADDED（Scenario 12 本）。
依頼の逐語・価値・壊してはいけないもの・完了の判定は `docs/changes/CH07.md`。`requires: [CH06]`（CH06 の gate 済みの spec `c666673` を読んで書いた。`deps.json`）。

## 深掘り

問いは 0 件。依頼の逐語が範囲（`tools/verify-env.sh` の 1 行とその検査）も、作り直さない木で宣言を出さないことも決めている（`deep.md`、review/deep.md R6・R7）。
聞かずに決めたもの（`premises.md` の C1〜C9）:
- C1: 行は `HARNESS_RESET: testdb` そのもので、行頭から出す
- C2: `verify-env.sh` 自身の echo で、成功の出口の直前に 1 回だけ出す。volume を 1 つ以上消したか、DB のコンテナが無くて立てたときだけ。止まっていた DB を作り直したときも出す。volume が 0 個なら出さない
- C3: 印なし・CI でない main の checkout・git の木でない・非 0 のときは出さない。`none` も出さない
- C4: CH06 の振る舞いは変えない
- C5: 検査は CH06 の `tools/check-verify-env-reset.sh` に足す
- C6: `requires: [CH06]`
- C7: 判定 3 は `.harness2` の `init.py --check` で確かめる
- C8: #112 が先に入っても、順を変えない
- C9: 「作り直したときだけ」は、CH06 の判定と C2 の両方を満たすときと読む

spec の段の時点で、harness2 #112 は merge 済みで `.harness2` に入っていました。これに合わせて、premises.md の P1・P4・P8・C7・C8 を直しました。

## 仮決め（B）と反転条件（design の（仮））

- **D1（仮）** #112 が先に入っても、CH07 の順を変えず、CH06 へも差し戻さない。#112 は先に入ったが、CH06 は宣言を足していない。
  反転条件: CH06 が後で宣言の行を足したら、tasks 1.1 がその行を `reset_db` の印に結び直す。注意が誤った直しを起こすと分かったら、harness2 側に返す
- **D3（仮）** 検査の本体では、既にある段 1・3〜8 に宣言の数の確かめを足す。新しい段は 3 つ足す:
  - 9.: 止まっていた DB
  - 10.: volume を持たない DB。tmpfs の compose のファイルを、環境変数 `COMPOSE_FILE` で並べて作る
  - 11.: 途中で落ちる。`ENV_FILE` に無いファイルを指して作る

  反転条件: `db-roles.sh` が `.env` を読み直すせいで 10. が落ちたら、override の形に替える（spec-review の実測では落ちなかった）

仮でない決定:
- D2: 宣言は `reset_db` が立てた印を見て、`exit 0` の直前で出す
- D4: `--self-test` に足す確かめ（`--plan`・git の木でない・本文の字・`init.py --check`）
- D5: 文書
- D6: delta は ADDED

## capability

`verify-environment`（CH06 が立てる。正典にはまだ無い）に ADDED で足しました。前倒しはありません。archive は CH06 の archive の後に回ります。

## tasks

4 Task・8 項目。Scenario 12 本の置き場は次のとおりです。
- `check-verify-env-reset.sh --self-test`: 3 本。うちハーネスの点検の 1 本は、`.harness2` のある worktree だけで走る
- 同じ台本の本体: 9 本。うち CI でない main の checkout の 1 本は、CI だけで走る

確認バッチの「人間の確認待ち」はありません。画面も人の操作も変わりません。

## レビュー

- review/deep.md: 8 件。すべて fixed（うち 2 件は deep の問いを外した）
- review/spec.md: 10 件。すべて fixed
  - R1: D1 に（仮）を戻した
  - R2: Scenario を 2 本足した
  - R3〜R6: tasks の検証コマンドを直した
  - R7: premises の古い記述を直した。kind は premise から technical に直した。前提は実物で確かめ直しており、決定は変わらない
  - R8〜R10: spec の言い方・CH07.md・注記の例外を直した
