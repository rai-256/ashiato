# CH07 deep-questions の検査（deep-review）

検査の対象: `openspec/changes/ch07-declare-verify-reset/deep-questions.json`（Q1・Q2。どちらも open / B・推奨あり）と `premises.md`（P1〜P7・C1〜C7）。
手順 1〜8 は、問いの一覧を見る前に、依頼の逐語（`docs/changes/CH07.md`）・CH06 の design / spec / tasks・CH06 の木（`/home/yosis/dev/ashiato2-ch06`）・harness2 #112（`origin/feat/verify-reset-declare`）から先にやり直した。
#112 の振る舞いは写しではなく、PR の head を `git archive` で一時の場所に出し、**本物の `evidence.run_one` / `reset_note` と `init.check_verify_env`** を、使い捨ての repo と `tools/verify-env.sh` の見本に当てて走らせた（docker には触っていない。一時の場所は消した）。

実測（#112 の `run_one`。検証コマンドは `echo HARNESS_RESET: later`）:

| 準備の出力 | status | `reset` | 注意 |
|---|---|---|---|
| 標準出力に `HARNESS_RESET: testdb`、rc=0 | PASS | `['testdb']` | 出ない |
| 標準エラーに `keep: …` だけ、rc=0 | PASS | `[]` | 出る |
| `HARNESS_RESET: testdb` を出して rc=1 | BLOCKED_INFRA | （無い） | 出ない |
| 標準エラーに `HARNESS_RESET: testdb`、rc=0 | PASS | `['testdb']` | 出ない |
| 行頭に空白 `  HARNESS_RESET: testdb`、rc=0 | PASS | `[]` | 出る |

検証コマンドの後ろの `HARNESS_RESET: later` は、どの場合も数えられなかった。

実測（#112 の `check_verify_env`）: いまの main の `tools/verify-env.sh` → `[人間] … 宣言していない`。CH06 の木のいまの `tools/verify-env.sh`（Task 3 の前。`HARNESS_VERIFY` の字が 0 個）→ 同じ。CH06 の木の版に `HARNESS_VERIFY` の字と `echo "HARNESS_RESET: testdb"` を足したもの → `[ok] tools/verify-env.sh（HARNESS_VERIFY=1 で作り直して HARNESS_RESET を宣言する）`。

## 手順ごとの記録

- 手順 1（衝突）: `satisfies: []`。依頼の完了の判定どうしと、#112・CH06 の spec を突き合わせた。
  (a) 依頼の判定 1「CI でない main の checkout は出さない」と #112 の「作り直すものが無ければ `HARNESS_RESET: none（理由）`」（`evidence.py:34`）—— 依頼が名指しで決めている → R6。
  (b) 依頼の「標準出力に出す」と #112 —— #112 は準備の標準出力と標準エラーを同じログで読む（`evidence.py:387`。上の表）ので食い違わない。標準出力は検査の側だけの約束。
  (c) 依頼の判定 4「CH06 の振る舞いを変えない」と CH06 の `--plan`（tasks 3.1 の検証 `[ "$(HARNESS_VERIFY=1 tools/verify-env.sh --plan)" = reset ]` が標準出力の全体を比べる）—— C5 の「`--plan` は宣言を出さない」で衝突しない。
  (d) 依頼の判定 3「init.py --check が tools/verify-env.sh の項目を出さない」と #112 の `check_verify_env`（通っても `[ok]` の行を出す）→ R1
- 手順 2（扉の幅）: 「作り直したとき」が、D2 の判定が `reset` のことか、volume を消した（コンテナが無くて新しく立てた）事実のことかに幅がある。C2 は前者寄りで、DB のコンテナが無い場合だけ書いている → R8
- 手順 3（新たに立つ一方通行）: 該当なし（確かめた範囲: 変わるのは準備の出力の 1 行と検査。`reset` の値を読むのは `reset_note` の `!= []` だけ（`git -C .harness2 grep '"reset"' origin/feat/verify-reset-declare -- scripts graph`）で、`testdb` という字を後から変えても読む側が無い。ashiato2 で `verify-env.sh` の出力を読む台本は無い（`grep -rn verify-env tools .github web/playwright.config.ts`）。DB の中身・作り直しの対象は CH06 のまま）。両問 B・`loss` 無しの分類は妥当
- 手順 4（日常）: `verify-run` の「注意」は表示と記録だけで status を変えない（上の表）。ただし #112 が先に `.harness2` に入ったとき、宣言の無い木の全部で毎回「注意」が出る —— その中には `verify-env.sh` を書いている最中の CH06 の下流が入る → R3
- 手順 5（既存コード）: いまの main の `tools/verify-env.sh` は `HARNESS_VERIFY` の字も宣言も持たない。CH06 の木も Task 3・4 の前で同じ。成功の `exit 0` は待ちのループの中（`tools/verify-env.sh:22`）、`up` と `db-roles.sh` の標準出力は捨てている（`:13`・`:21`）→ R4。`tools/stack.sh:24` が印で `STACK_RESET=1` にし、`:76` で `down -v` すること（Q1 の context）はコードどおり。P5 の「`check-verify-env-reset.sh` はまだ無い」はいまの CH06 の木と合わない → R5
- 手順 6（過去の取りこぼし）: 「検証コマンドが成立しない」（11 件）の型が、依頼の判定 3 の確かめ方にそのまま当たる（文字どおりの判定が成り立たず、rc も使えない）→ R1・R2。「既存コードの振る舞いを確かめずに置いた」が C2 の置き場所（CH06 のまだ書かれていない構造）に当たる → R4。「他 Story の決定の波及」が #112 の merge の順に当たる → R3。「DB を共有する連鎖の検証」（ST33 Q10）は CH06 が扱う。ほかの観点（時計・画面・取り込み・台帳など）は該当なし
- 手順 7（族の網羅）: Q2 の族「作り直さない・作り直せない場合の宣言」の兄弟を、CH06 の D1〜D5 の経路から挙げた。
  (a) 印つき・CI でない main の checkout = Q2（依頼が決めている → R6）、(b) 印が無い = C3、(c) 非 0（D4 の確かめ・D1 の 4. 5. の失敗・D5 の network の確かめの失敗）= C2、(d) DB のコンテナが無い = C2、
  (e) **DB のコンテナはあるが止まっていた**（D3 の `切った接続: 0（DB は止まっていた）`）= 一覧にも C にも無い、(f) **印つき・git の木でない**（D2 は作り直さない側と読む）= 無い、
  (g) **作り直しの経路を通ったが、控えた volume が 0 個**（DB のコンテナが volume の mount を持たない）= 無い、(h) 検証コマンドの中で `verify-env.sh` がもう一度呼ばれる（`check-verify-env-reset.sh` の本体を `verify-run` で走らせる）= 数えられない（上の表）ので決めることが無い。
  C7 の族「#112 と CH07 の入る順」: (i) CH07 が先 = C7 が無害と書く（確かめた）、(ii) **#112 が先** = 無い → R3。(e)〜(g) → R8
- 手順 8（前提の確かさ）: `loss` のある問い・推奨・C は無い。P2（確かさ 4）は確かめ方が「写して当てた」で、本物の `run_one` を走らせたのはこの検査が初めて —— 結果は P2・P3・P6 のとおりで、4 に上げられる（上の表）。P4（確かさ 2）も本物の `check_verify_env` を走らせて規則どおりだった。ただし OPEN の PR の規則で、変わりうる → R2。P5（確かさ 2）は結果の一部がいまの木と合わない → R5

---

## R1. 完了の判定 3「`init.py --check` が `tools/verify-env.sh` の項目を出さない」は、#112 が入っても文字どおりには成り立たない。通ったときも `[ok] tools/verify-env.sh（…）` の行が必ず出る。rc も他の項目で 1 になる
- 成果物: openspec/changes/ch07-declare-verify-reset/premises.md
- 根拠: docs/changes/CH07.md:29 / `origin/feat/verify-reset-declare:scripts/init.py:374`（`r.line("ok", "tools/verify-env.sh（HARNESS_VERIFY=1 で作り直して HARNESS_RESET を宣言する）")`）と `:103-106`（`Report.line` は ok も `  [ok] …` と print する）/ `:469-474`（rc は pending・todo・conflict のどれか 1 つで 1。1 度だけの工程・GitHub・録画の項目を含む）/ 冒頭の実測（宣言を足した版で `[ok] tools/verify-env.sh（…）` が出た）
- kind: premise
- 提案: C7 に、判定 3 の確かめ方を「`init.py --check` の出力に `[人間] tools/verify-env.sh` で始まる行が無く、`[ok] tools/verify-env.sh` で始まる行がちょうど 1 つある（rc は見ない）」と書く。spec の段で `## 完了の判定` を書き直すときもこの形にする
- 処置: fixed premises.md P4・C7（`[人間]` の行が無く `[ok]` の行がちょうど 1 つ、rc は見ない）

## R2. C7 の「#112 が入っていなければ、字面の規則を写した台本で確かめる」は、OPEN の PR の規則の写しで、#112 が merge までに変わっても落ちない。共有の `.harness2` は main でない枝にあり、「#112 の merge 後」と「確かめる時点の `.harness2` に入っている」も別の条件
- 成果物: openspec/changes/ch07-declare-verify-reset/premises.md
- 根拠: `gh -R rai-256/harness2 pr view 112` → `OPEN`・`mergedAt: null` / `readlink -f .harness2` → `/home/yosis/dev/harness2`、`git -C .harness2 branch --show-current` → `fix/panel-reload-board`、`git -C .harness2 merge-base --is-ancestor origin/feat/verify-reset-declare HEAD` → 含まない / `grep -c check_verify_env .harness2/scripts/init.py` → 0 / P4（確かさ 2。OPEN の PR の source）
- kind: premise
- 提案: C7 の後半を「写した台本」でなく「#112 自身の `check_verify_env` を走らせる」に替える（入っていれば `.harness2/scripts/init.py --check`、入っていなければ `git -C .harness2 show <#112 の head か merge の SHA>:scripts/init.py` を一時の場所に出して走らせる）。どちらを使ったかと、harness2 の SHA を証跡に残す。入っているかは `grep -q check_verify_env .harness2/scripts/init.py` で機械が決める
- 処置: fixed premises.md P8・C7（#112 自身の `check_verify_env` を走らせ、SHA を証跡に残す）

## R3. #112 が CH07 より先に `.harness2` に入った場合が C7 に無い。宣言の無い木の全部で `verify-run` のたびに「注意」が出て、その文面は「その行を出す」という指示の形。`verify-env.sh` を書いている最中の CH06 の下流がそれを読み、C6 が禁じる「CH07 の行を CH06 に入れる」が起きうる
- 成果物: openspec/changes/ch07-declare-verify-reset/premises.md
- 根拠: `origin/feat/verify-reset-declare:scripts/evidence.py:439-445`（注意の文面「…作り直して、その行を出す」）と `:518`（`cmd_run` が項目ごとに print）/ 冒頭の実測（宣言が無く rc=0 なら `reset == []` で注意が出る）/ `.harness2` は全部の worktree が共有する 1 つの checkout（`readlink -f .harness2`）/ CH06 の tasks 3.1・4.1・4.2 が `tools/verify-env.sh` を書き換える（`/home/yosis/dev/ashiato2-ch06/openspec/changes/ch06-reset-verify-db/tasks.md`）/ C6（CH07 の行を CH06 へ差し戻さない）/ 走っている change: st23・st29・st41・st48・ch03・ch04（`ls openspec/changes`）
- kind: daily
- 提案: C7 の族に「#112 が先に入った場合」を足す。仮でよい（B）。既定の例: CH06 の下流の間に注意が出ても宣言を CH06 に足さないことを CH06 の Task の申し送りか C に書く / あるいは #112 の merge を CH07 の main 入りの後にする（harness2 側の順の決定。CH07 は待たない、の逆向き）。どちらにするかを C7 に 1 行書く
- 処置: fixed D1 仮（premises.md C8）

## R4. C2 の置き場所（「役割まで整えた後、rc=0 で終わる直前に 1 回」）は、CH06 がまだ書いていない `verify-env.sh` の形に依る。いまの形では成功の `exit 0` が待ちのループの中にあり、下請けの標準出力は捨てている。宣言を誰が・何を条件に出すかが C に無い
- 成果物: openspec/changes/ch07-declare-verify-reset/premises.md
- 根拠: tools/verify-env.sh:13（`docker compose up -d --wait db >/dev/null`）/ :21（`tools/db-roles.sh >/dev/null`）/ :22（`exit 0` がループの中）/ CH06 の tasks 3.1・4.1・4.2 が未着手（`[ ]`）/ #112 の `check_verify_env` は `tools/verify-env.sh` の本文の字だけを見る（`origin/feat/verify-reset-declare:scripts/init.py:361-374`）/ 行頭に空白があると数えない（冒頭の表の最後の行）
- kind: premise
- 提案: C2 に 3 点を足す: (1) 行は `verify-env.sh` 自身の `echo` で出す（`db-roles.sh` などの下請けや、`>/dev/null` で包んだ塊・`$(…)` の中で出さない）、(2) 字 `HARNESS_RESET:` は `verify-env.sh` の本文に置く（読み込む別のファイルに移すと init.py が見つけない）、(3) 行頭から出す（字下げしない）。出す条件は D2 の判定の変数と、成功の終わり（どの `exit 0` の前か）で書く
- 処置: fixed premises.md P5・C1・C2（自身の echo・本文に字・行頭から、出す条件）

## R5. P5 の結果「`check-verify-env-reset.sh` は Task 2 で作られる（まだ無い）」「Task 1 まで完了」は、いまの CH06 の木と合わない。ファイルは `--self-test` だけで既にあり、本体（C5 が足す先）は「まだ無い」で rc=2 を返す
- 成果物: openspec/changes/ch07-declare-verify-reset/premises.md
- 根拠: `/home/yosis/dev/ashiato2-ch06/tools/check-verify-env-reset.sh:14`（`"") echo "check-verify-env-reset: 本体はまだ無い（--self-test だけ）" >&2; exit 2`）/ `git -C /home/yosis/dev/ashiato2-ch06 log --oneline main..HEAD`（`73aa5140 feat(ch06): env-init の自己検査と確認バッチの準備での呼び出し`・`2ca212ca`・`e6d714bd`・`0a4ee5f8` Task 2 の証跡）
- kind: premise
- 提案: P5 の結果を「`--self-test` の部分は Task 2 で入りつつある。C5 が足す先の本体（D8 の 1.〜8.）は Task 5 でまだ無い」に直す。C5 は本体の D8 の段の番号に載っているので、CH06 の Task 5 が段を変えたら C5 を読み直す、と反転条件を 1 行書く
- 処置: fixed premises.md P5・C5 の反転条件

## R6. 不要: Q2（CI でない main の checkout で宣言をどうするか）は、依頼の完了の判定 1 が名指しで「出さない」と決めている
- 成果物: openspec/changes/ch07-declare-verify-reset/deep-questions.json
- 根拠: docs/changes/CH07.md:27（「作り直さない場合（印が無い・**CI でない main の checkout**・作り直せずに非 0 で終わる）は出さない」）/ 同 :17（依頼は「無ければ『注意』を出す」を知った上で書いている）/ Q2 の推奨の選択肢自身が「依頼どおり」と書く
- kind: defer
- 提案: Q2 を外し、C3 に「CI でない main の checkout（と git の木でない木）でも出さない。`verify-run` は注意を出し、それは事実どおり」と並べる。本人が `none` を出したいなら依頼の本文を変えることになるので、そのときは A になる
- 処置: fixed deep.md Q2 を外して C3 へ（問いは 0 件で deep.md を直接書いた）

## R7. 不要: Q1（`tools/stack.sh` にも行を出すか）は、依頼の範囲（`tools/verify-env.sh` の 1 行）で決まっていて、もう一方の選択肢は機械のどこにも効かない
- 成果物: openspec/changes/ch07-declare-verify-reset/deep-questions.json
- 根拠: docs/changes/CH07.md:23・27（どちらも `tools/verify-env.sh` だけを名指し）/ 冒頭の実測（検証コマンドの出力の `HARNESS_RESET:` は数えない）/ `init.py` の `check_verify_env` は `tools/verify-env.sh` だけを読む（`origin/feat/verify-reset-declare:scripts/init.py:364`）/ CH06 の Global Constraints が `tools/stack.sh` を変えないものに挙げる（ch06 tasks.md）
- kind: defer
- 提案: Q1 を外し、C4 か C1 に「`tools/stack.sh` などの検証コマンドの側には出さない（`verify-run` も `init.py` も読まない）」と 1 行書く。R6 と合わせると問いが 0 になる —— その場合は note を「問い無し・C だけ」の形にする
- 処置: fixed deep.md Q1 を外して C4 へ

## R8. 族の兄弟が 3 つ抜けている: DB のコンテナが止まっていた / 印つきで git の木でない / 作り直しの経路を通ったが消した volume が 0 個。最後の 1 つは、中身が残ったまま `HARNESS_RESET: testdb` を出しうる
- 成果物: openspec/changes/ch07-declare-verify-reset/deep-questions.json
- 根拠: CH06 design D3（`切った接続: 0（DB は止まっていた）`。作り直しの経路）/ D2（「git の木でなければ worktree でない木と読む（作り直さない側）」）/ D1 の 3.（`Type` が `volume` の mount の名前を控え、4. でその名前だけを消す。0 個なら何も消さずに `rm -s -f db` → `up` に進む）/ `docker-compose.yml` は volume を宣言せず、postgres の image の匿名 volume に頼る（CH06 premises P11）/ 手で置く `docker-compose.override.yml` が既にある（`ashiato2-integ`・`ashiato2-rec-st22`。いまは `ports` だけで mount は無い）
- kind: technical
- 提案: Q2 の family（か C2）に 3 つを足す。(e) 止まっていた → C2 と同じく出す、(f) git の木でない → (a) と同じく出さない、(g) 控えた volume が 0 個 → 仮でよい（B）。既定の例: 宣言の条件を「判定が reset で、控えた volume を 1 つ以上消したか、DB のコンテナが無くて新しく立てた」にし、0 個のときは出さない（`verify-run` の注意が事実どおりになる）。CH06 の振る舞いは変えない
- 処置: fixed premises.md C2・C3（止まっていた → 出す、git の木でない → 出さない、volume 0 個 → 出さない）
