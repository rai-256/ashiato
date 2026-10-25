# CH07 の前提（questions の段、2026-10-25）

この change の決定が依る、外の振る舞い（harness2 #112 の `verify-run` と `init.py --check`・走っている CH06）の前提。

| # | 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|---|
| P1 | harness2 #112（`feat/verify-reset-declare`）は 2026-10-25 時点で **OPEN**。`.harness2` の checkout にはまだ入っていない | 完了の判定 3（`init.py --check`）をいつ確かめられるか | `gh pr view 112`・`grep -rn HARNESS_RESET .harness2/scripts` | OPEN。今の `.harness2` には `HARNESS_RESET` が 1 か所も無い | 実物 | 5 |
| P2 | `verify-run` は宣言を `^HARNESS_RESET:[ \t]*(\S.*?)\s*$`（行頭・複数行）で探し、**準備の出力（検証コマンドの `$ (cd …` の行より前）だけ**を数える。準備の stdout と stderr は同じログに入る | 行の形（行頭に置く・前に何も付けない）と、出す場所（`verify-env.sh` の中。検証コマンドの台本では数えない） | `origin/feat/verify-reset-declare:scripts/evidence.py` の `RESET_MARK` と `run_one` を写し、ログの見本に当てる | `HARNESS_RESET: testdb` は `['testdb']`、検証コマンドの後ろの行は数えない、`keep:` の行だけなら `[]`。review/deep.md が PR の head の本物の `run_one` を使い捨ての repo で走らせ、標準エラーの行も数える・**行頭に空白があると数えない**ことを確かめた | 公式の source を走らせた | 4 |
| P3 | `verify-run` は準備が rc≠0 なら BLOCKED_INFRA で終え、宣言を読まない。準備が rc=0 で宣言が無ければ（`reset == []`）「注意」を出す。status は変えない | 作り直さない場合に何も出さないと何が起きるか（Q2）・作り直せないとき出さなくてよいか | 同じ `run_one` と `reset_note` を読む | そのとおり（review/deep.md が本物の `run_one` で rc=1 → BLOCKED_INFRA・注意なし、宣言なし rc=0 → 注意を確かめた） | 公式の source を走らせた | 4 |
| P4 | `init.py --check` の `check_verify_env` は**字面**で見る: ファイルに `HARNESS_RESET:` の字が無ければ todo、`HARNESS_VERIFY` の字も `HARNESS_RESET: none` も無ければ todo、それ以外は ok | 完了の判定 3 | `origin/feat/verify-reset-declare:scripts/init.py` の `check_verify_env` を読む | そのとおり。`verify-env.sh` に `HARNESS_RESET: testdb` の字があり `HARNESS_VERIFY` を読めば ok になる。**ok のときも `[ok] tools/verify-env.sh（…）` の行を出し**、rc は他の項目（1 度だけの工程・GitHub など）で 1 になる（review/deep.md R1。本物の `check_verify_env` を走らせた） | 公式の source を走らせた（OPEN の PR。merge までに変わりうる） | 4 |
| P5 | CH06（下流が走っている。7 Task のうち Task 1 が完了、Task 2 が進行中）が、作り直しの判定（D2）・手順（D1）・切った接続の 1 行（D3）・作り直せないときの非 0（D4）と、検査 `tools/check-verify-env-reset.sh`（D8）を入れる。いまの main にはどれも無い | CH07 が積む場所（`requires: [CH06]`）。CH07 は CH06 の Task に差し戻さない | `ashiato2-ch06` の `git log main..HEAD`・`tasks.md` の `[x]`、`openspec/changes/ch06-reset-verify-db/design.md` | `check-verify-env-reset.sh` は `--self-test` の部分だけが Task 2 で入りつつあり、本体（D8 の 1.〜8.。C5 が足す先）は Task 5 でまだ無い（いまは `:14` で「本体はまだ無い」の rc=2）。`verify-env.sh` の判定と手順（Task 3・4）も未着手で、いまの形では成功の `exit 0` が待ちのループの中、`up` と `db-roles.sh` の標準出力は `/dev/null`（`tools/verify-env.sh:13`・`:21`・`:22`）（review/deep.md R4・R5） | 既存のテスト材料 | 2 |
| P6 | `verify-run` は準備を呼ぶとき必ず `HARNESS_VERIFY=1` を渡す。印の無い呼び出しは人や台本が直接呼ぶときだけ | 「印が無いときは出さない」が `verify-run` の注意に効かないこと | `evidence.py` の `env = {**os.environ, VERIFY_FLAG: "1"}` | そのとおり | 公式の source | 2 |
| P7 | 確認バッチの作業ツリー（`-verify`）は worktree で、CH06 の判定では作り直す側。印つきで作り直さない木は「CI でない main の checkout」だけで、`verify-run` がそこで走るのは人が手で呼ぶときだけ | Q2 の効く範囲 | CH06 design D2 | そのとおり | 既存のテスト材料 | 2 |
| P8 | 共有の `.harness2`（`/home/yosis/dev/harness2`）は全部の worktree が同じ 1 つの checkout を指し、いまは `fix/panel-reload-board` の枝で #112 を含まない。#112 が入ったかは `grep -q check_verify_env .harness2/scripts/init.py` で機械が見分けられる | 完了の判定 3 の確かめ方（C7）・#112 が先に入った場合（C8） | `readlink -f .harness2`・`git -C .harness2 merge-base --is-ancestor origin/feat/verify-reset-declare HEAD`（review/deep.md R2） | そのとおり | 実物 | 5 |

失われるもの（`loss`）のある決定は無い（変わるのは準備の出力の 1 行と検査だけで、DB の中身・作り直しの対象は CH06 のまま）。

## 聞かないで決めるもの（C）

問いは 0 件（review/deep.md R6・R7: 範囲と、作り直さない木での宣言は、どちらも依頼の逐語が決めている）。下の C1〜C9 は `deep.md` にも同じ番号で写す。

- **C1. 行は `HARNESS_RESET: testdb` そのもの（依頼の逐語）。** 前に `verify-env:` などを付けず、**行頭から**出す（P2 の正規表現は行頭から読み、字下げした行は数えない）。中身の詳細（volume の名前・切った接続）は CH06 の D3 の 1 行が持ち、宣言には足さない。
- **C2. 出すのは `verify-env.sh` 自身の `echo` で、作り直して役割（`tools/db-roles.sh`）まで整えた後、rc=0 で終わる直前に 1 回だけ（標準出力）。** 字 `HARNESS_RESET:` は `verify-env.sh` の本文に置く（`init.py` はこのファイルの字だけを読む。P4）。下請け（`db-roles.sh` など）・`>/dev/null` で包んだ塊・`$(…)` の中では出さない（P5 のとおり下請けの出力は捨てている）。
  作り直しの途中で落ちたとき（volume を消した後の `up` の失敗など）は出さない —— 依頼の「作り直せずに非 0 で終わる場合は出さない」と、P3（rc≠0 なら読まれない）のどちらにも合う。
  出す条件は「D2 の判定が `reset` で、かつ（控えた volume を 1 つ以上消した か DB のコンテナが無くて新しく立てた）」。
  DB のコンテナが止まっていた場合（CH06 D3 の `切った接続: 0（DB は止まっていた）`）も、この条件を満たせば出す（review/deep.md R8 (e)）。
  **判定が `reset` でも、コンテナがあって控えた volume が 0 個（手で置いた override が bind mount で中身を持つなど）なら出さない** —— 中身が残ったまま「作り直した」と宣言しない。`verify-run` の注意が事実どおりになる（R8 (g)。厳しい側。CH06 の手順は変えない）。
- **C3. 作り直さない場合は出さない（依頼の逐語）**: 印が無い（`verify-run` は必ず印を渡すので注意には効かない。P6）・CI でない main の checkout（印があっても。`verify-run` はそこで注意を出し、それは事実どおり —— 溜まった DB の上の証跡）・git の木でない木（CH06 D2 は main の checkout と同じ作り直さない側。R8 (f)）・作り直せずに非 0。
  `HARNESS_RESET: none（理由）` は使わない（#112 の none は「作り直すものが無い」の意味で、溜まった DB がある木とは意味がずれる。R6）。
- **C4. CH06 の振る舞いは変えない**: 判定の表（D2）・`--plan` の出力（標準出力の全体を比べる検証がある）・作り直しの手順（D1）・切った接続の 1 行の形（D3）・作り直せないときの rc（D4）・`keep:` の標準エラーの 1 行。CH07 は行を 1 つ足すだけ。
  `tools/stack.sh` など検証コマンドの側の台本には出さない（`verify-run` は検証コマンドの出力を数えず、`init.py` も `verify-env.sh` だけを読む。R7）。
- **C5. 検査は CH06 の `tools/check-verify-env-reset.sh` の本体（docker を使う方）に足す**: 作り直した段（D8 の 1.・5.・6.・7.・8. の立て直し）では準備の出力に `HARNESS_RESET: testdb` がちょうど 1 行、作り直さない段（3. の印なし・CI の `keep`・4. と 8. の非 0）では 0 行。`--self-test`（docker に触らない）は `--plan` が宣言を出さないことと、`verify-env.sh` の本文に字 `HARNESS_RESET: testdb` があることを確かめる。
  反転条件: CH06 の Task 5 が本体の段（D8 の番号）を変えたら、C5 の段の対応を読み直す（R5）。
- **C6. `requires: [CH06]`。** CH06 の tasks は凍結していて、CH07 の行をそこへ差し戻さない（走っている change へ戻さない規則）。CH06 が gate を通るまでは spec は proposal まで。
- **C7. harness2 #112 の merge は待たない。完了の判定 3 は #112 自身の `check_verify_env` を走らせて確かめる**（R1・R2）: `grep -q check_verify_env .harness2/scripts/init.py` が通れば `.harness2/scripts/init.py --check`、通らなければ `git -C .harness2 show <#112 の head>:scripts/init.py` を一時の場所に出して走らせる。
  判定は「出力に `[人間] tools/verify-env.sh` で始まる行が無く、`[ok] tools/verify-env.sh` で始まる行がちょうど 1 つ」（rc は見ない —— 他の項目で 1 になる）。どちらを使ったかと harness2 の SHA を証跡に残す。行は #112 が入らなくても無害（ログに 1 行増えるだけ）。
- **C8. #112 が CH07 より先に `.harness2` に入った場合**: 宣言の無い木の全部で `verify-run` のたびに「注意」が出る（表示と記録だけで status は変えない。P3）。CH06 の下流がその注意を見て宣言の行を CH06 に入れても、失うものは無い（CH07 はその行を確かめる側になる）。CH07 は注意を止めるために順を変えない・CH06 へ差し戻さない（R3）。
- **C9. 完了の判定の「作り直したときだけ」は、CH06 の判定（D2）と C2 の条件の両方を満たすときと読む。** 確認バッチの作業ツリー（`-verify`）は worktree で作り直す側（P7）なので、そこでは出す。
