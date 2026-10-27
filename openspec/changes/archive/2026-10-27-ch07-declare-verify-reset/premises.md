# CH07 の前提（questions の段、2026-10-25。spec の段で P1・P4・P8 を更新し P9 を足した）

この change の決定が依る、外の振る舞い（harness2 #112 の `verify-run` と `init.py --check`・走っている CH06）の前提。

| # | 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|---|
| P1 | harness2 #112（`feat/verify-reset-declare`）は **2026-10-25T00:09 に MERGED**（merge commit `42fb957`）。共有の `.harness2` の HEAD（`aaecd2c`）に入っている | 完了の判定 3（`init.py --check`）をいつ確かめられるか | `gh -R rai-256/harness2 pr view 112 --json state,mergedAt,mergeCommit`・`grep -c check_verify_env .harness2/scripts/init.py` | MERGED。`.harness2` に `check_verify_env` がある（2 か所）。questions の段（OPEN）から spec の段で変わった | 実物 | 5 |
| P2 | `verify-run` は宣言を `^HARNESS_RESET:[ \t]*(\S.*?)\s*$`（行頭・複数行）で探し、**準備の出力（検証コマンドの `$ (cd …` の行より前）だけ**を数える。準備の stdout と stderr は同じログに入る | 行の形（行頭に置く・前に何も付けない）と、出す場所（`verify-env.sh` の中。検証コマンドの台本では数えない） | `origin/feat/verify-reset-declare:scripts/evidence.py` の `RESET_MARK` と `run_one` を写し、ログの見本に当てる | `HARNESS_RESET: testdb` は `['testdb']`、検証コマンドの後ろの行は数えない、`keep:` の行だけなら `[]`。review/deep.md が PR の head の本物の `run_one` を使い捨ての repo で走らせ、標準エラーの行も数える・**行頭に空白があると数えない**ことを確かめた | 公式の source を走らせた | 4 |
| P3 | `verify-run` は準備が rc≠0 なら BLOCKED_INFRA で終え、宣言を読まない。準備が rc=0 で宣言が無ければ（`reset == []`）「注意」を出す。status は変えない | 作り直さない場合に何も出さないと何が起きるか（Q2）・作り直せないとき出さなくてよいか | 同じ `run_one` と `reset_note` を読む | そのとおり（review/deep.md が本物の `run_one` で rc=1 → BLOCKED_INFRA・注意なし、宣言なし rc=0 → 注意を確かめた） | 公式の source を走らせた | 4 |
| P4 | `init.py --check` の `check_verify_env` は**字面**で見る: ファイルに `HARNESS_RESET:` の字が無ければ todo、`HARNESS_VERIFY` の字も `HARNESS_RESET: none` も無ければ todo、それ以外は ok | 完了の判定 3 | `.harness2/scripts/init.py:361-374`（merge 後）を読み、`python3 .harness2/scripts/init.py --check` を走らせる | そのとおり。いまの木（CH06 の前の main）で `[人間] tools/verify-env.sh が作り直したものを宣言していない…` が出る（2026-10-25、spec の段）。**ok のときも `[ok] tools/verify-env.sh（…）` の行を出し**、rc は他の項目（1 度だけの工程・GitHub など）で 1 になる（review/deep.md R1） | 公式の source を走らせた | 4 |
| P5 | CH06 が gate を通った `c666673`（`deps.json`）に、作り直しの判定（D2）・手順（D1）・切った接続の 1 行（D3。標準エラー）・作り直せないときの非 0（D4）と、検査 `tools/check-verify-env-reset.sh`（`--self-test` と本体の段 1.〜8.。D8）が揃っている | CH07 が積む場所（`requires: [CH06]`）。CH07 は CH06 の Task に差し戻さない | `/home/yosis/dev/ashiato2-ch06`（HEAD `c6666730`）の `tools/verify-env.sh`・`tools/check-verify-env-reset.sh` を読む | 揃っている。`verify-env.sh` の成功の出口は待ちのループの中の `exit 0` 1 つ、`up` と `db-roles.sh` の標準出力は `/dev/null`。宣言の字は無い。検査の `echo "Scenario: ` は 23 本。questions の段の結果（Task 1 完了・本体はまだ無い）は spec の段で古くなった（review/spec.md R7） | 実物 | 5 |
| P6 | `verify-run` は準備を呼ぶとき必ず `HARNESS_VERIFY=1` を渡す。印の無い呼び出しは人や台本が直接呼ぶときだけ | 「印が無いときは出さない」が `verify-run` の注意に効かないこと | `evidence.py` の `env = {**os.environ, VERIFY_FLAG: "1"}` | そのとおり | 公式の source | 2 |
| P7 | 確認バッチの作業ツリー（`-verify`）は worktree で、CH06 の判定では作り直す側。印つきで作り直さない木は「CI でない main の checkout」と「git の木でない木」（CH06 D2 は main の checkout と同じ `keep` と読む）だけで、`verify-run` がそこで走るのは人が手で呼ぶときだけ | Q2 の効く範囲 | CH06 design D2 | そのとおり | 既存のテスト材料 | 2 |
| P8 | 共有の `.harness2`（`/home/yosis/dev/harness2`）は全部の worktree が同じ 1 つの checkout を指す。spec の段（2026-10-25）では HEAD `aaecd2c`（origin/main）で #112 を含む。CI には `.harness2` が無い（gitignore の symlink） | 完了の判定 3 の確かめ方（C7）・#112 が先に入った場合（C8） | `readlink -f .harness2`・`git -C .harness2 merge-base --is-ancestor origin/feat/verify-reset-declare origin/main`（review/deep.md R2） | 含む（questions の段では含まなかった） | 実物 | 5 |
| P9 | compose の service に `tmpfs: /var/lib/postgresql/data` を書いて立てた postgres のコンテナは、`docker inspect` の `Mounts` が空（`[]`）で、image の `VOLUME` の匿名 volume も作られない | 検査の本体が「DB のコンテナはあるが volume を持たない」状態を作れること（design D3（仮）） | 一時の compose project に `postgres:17-alpine`・`tmpfs:` を書いて `docker compose create` し `docker inspect` で `Mounts` と `HostConfig.Tmpfs` を見た（`docker create --tmpfs` でも同じ）。後で消した | `Mounts` は `[]`、`HostConfig.Tmpfs` に `/var/lib/postgresql/data`（2026-10-25、spec の段）。spec-review が CH06 の `docker-compose.yml`・`tools/db-roles.sh` を写した一時の木で、環境変数 `COMPOSE_FILE` に tmpfs のファイルを並べて `up --wait` が Healthy、`.env` を読み直す `db-roles.sh` の `exec` も同じコンテナで rc=0 を確かめた（review/spec.md 観点 4） | 実物 | 5 |

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
  反転条件: CH06 が gate の後に本体の段（D8 の番号）を変えたら（deps の spec の SHA が進み、merge gate が「依存の spec:」で落とす）、C5 の段の対応を読み直す（R5・review/spec.md R7）。
- **C6. `requires: [CH06]`。** CH06 の tasks は凍結していて、CH07 の行をそこへ差し戻さない（走っている change へ戻さない規則）。CH06 が gate を通るまでは spec は proposal まで。
- **C7. 完了の判定 3 は `.harness2` の `init.py --check` を走らせて確かめる**（R1・R2。#112 は spec の段の時点で merge 済み・`.harness2` に入っている。P1・P8）:
  判定は「`python3 .harness2/scripts/init.py --check` の出力に `[人間] tools/verify-env.sh` で始まる行が無く、`[ok] tools/verify-env.sh` で始まる行がちょうど 1 つ」（rc は見ない —— 他の項目で 1 になる）。
  使った `.harness2` の SHA を証跡に残す。`.harness2` が無い・`check_verify_env` を持たない木では、確かめずに落ちる（黙って通らない）。CI には `.harness2` が無いので、この判定は手元の `verify-run` の証跡で取る。
- **C8. #112 が CH07 より先に `.harness2` に入った場合**（spec の段の時点で起きている。P1）: 宣言の無い木の全部で `verify-run` のたびに「注意」が出る（表示と記録だけで status は変えない。P3）。CH06 の下流がその注意を見て宣言の行を CH06 に入れても、失うものは無い（CH07 はその行を確かめる側になる）。CH07 は注意を止めるために順を変えない・CH06 へ差し戻さない（R3）。
  spec の段で確かめた: CH06 が gate を通った木（`c666673`）の `tools/verify-env.sh` に `HARNESS_RESET` の字は無い（CH06 は宣言を足していない）。design D1。
- **C9. 完了の判定の「作り直したときだけ」は、CH06 の判定（D2）と C2 の条件の両方を満たすときと読む。** 確認バッチの作業ツリー（`-verify`）は worktree で作り直す側（P7）なので、そこでは出す。
