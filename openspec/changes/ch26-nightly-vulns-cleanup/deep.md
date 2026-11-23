# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: same（振る舞いが変わらない）
- 申告の根拠: 夜間の脆弱性検査が挙げた 4 件を互換の範囲で上げるだけ。chrono・libsqlite3-sys（rusqlite 経由）・rustix は Cargo.toml / Cargo.lock の版を上げ、上げられないものは AGENTS.md の規則どおり osv-scanner.toml の例外の表に理由と 90 日以内の期限で書く。同梱の PostgreSQL は pgsql.lock の version・url・sha256 を 17.12 にそろえるだけで major（17）は変えない（installer の build.rs が読むのは major だけ）。何を・なぜ・どこまでは依頼が決めてあり、要件は変えない。API・送信の契約・移行には触れない。新しい版が足す依存の許諾文が要れば tools/licenses に置く。minor の更新と戻しの確かめは docs/release.md §PostgreSQL の小さい版を上げる PR の手順で行う。走っている CH24 も Cargo.lock と osv-scanner.toml を触るので、後から merge する側が rebase で合わせる
- 触れる予定のパス: Cargo.toml / Cargo.lock / crates/*/Cargo.toml / osv-scanner.toml / tools/release/pgsql.lock / tools/licenses/** / docs/release.md / docs/changes/CH26.md

## spec-review から人間へ返す問い（第 1 回。`deep-questions-r1.json`）

### R2. 17.6 から 17.12 への更新と戻しを、どこまで確かめるか

- 出所: `review/spec.md` の R2（kind: premise）。軽い路の申告は「minor の更新と戻しの確かめは docs/release.md §PostgreSQL の小さい版を上げる PR の手順で行う」と言ったが、design D5 は Windows の zip どうしの更新と戻しを Linux の PostgreSQL の確かめで代えていた。依頼の「既に入っている PC で…確かめる」を狭めるので本人に問う
- 選択肢: A. PostgreSQL の部分（置き場の 17.6→17.12→17.6・17.12 の退避を 17.6 で戻す）を Linux で確かめ、Windows は本人の PC で見る（推奨）／ B. Windows でも 17.6 の zip から 17.12 の zip への更新と戻しを通す ／ C. CI で前の lock の zip を組み、毎回の PR で走らせる（通常の路）
- 答え: A. PostgreSQL の部分を Linux で確かめ、Windows は本人の PC で見る（本人の選択。推奨と同じ・補足なし。`deep-answers-1.txt`）
- 決まったこと: 17.6 → 17.12 の確かめは、PostgreSQL の置き場の入れ替え（17.6 → 17.12 → 17.6）と「17.12 の退避を 17.6 の pg_restore で戻す」を Linux で機械が走らせる（design D5・tasks 3.1）。Windows の配布物どうし（17.6-1 の zip → 17.12-1 の zip）の更新と戻しはこの PR では走らせず、本人の PC で確認バッチの問い（tasks 5.1）として見る。`docs/release.md` の「前の版を同梱した zip から更新と戻しを通す」手順はこの PR では省く（手順の本文は変えない）。CI は変えない
- 要件へ戻すもの: なし（要件は変えない change。依頼の「どこまで」の確かめ方を本人が決めた）
- 答えの変更（第 1 回の合意で本人が変えた。`deep-answers-2.txt`）: **A → C**。lock を変える PR だけ、前の lock（今回は 17.6-1）で古い側の試験用の Windows の zip を組み、`tools/release/update-apply-install.sh` で 17.6 → 17.12 の更新と戻しを Windows の runner で機械が通す。tasks 5.1（本人の PC で見る）は削る。上の「決まったこと」のうち、Windows を本人の PC で見る・CI は変えない・release.md の手順を省く、は覆った（下の「仕様への違和感（第 1 回の合意）」）

## 仕様への違和感（第 1 回の合意）

`deep-answers-2.txt` の QA（答え: 違和感がある）の補足を逐語で:

- R2 の答えを A から C に変える。手で確かめる手間を最低限にしたい。pgsql.lock を変える PR だけ、古い側の試験用の Windows の zip（package-windows-test-old）を変更前の lock（今回は 17.6-1）で組み、既存の tools/release/update-apply-install.sh で 17.6 → 17.12 の更新と戻しを Windows の runner で通す（docs/release.md「PostgreSQL の小さい版を上げる PR」の手順を CI で機械が行う）。lock を変えない PR の組み立てと試験は今のまま。tasks 5.1（本人の PC で更新して記録が見えるかを答える）は削る。5.1 の「本人の PC の配布版と記録は機械から見えない」は誤りで、WSL から読み取り専用で読める。本人の PC の移行は ST64 で扱う
  → 対応: 直した。design D5 を本人の決定 R2 = C に書き直し（Linux の置き場の入れ替えは残し、Windows の配布物どうしの 17.6-1 → 17.12-1 の更新と戻しは lock を変える PR の CI が通す）、CI の決め方を D7（仮）（変えたかは PR 自身の差分で決め、前の lock は分岐点の lock・入れ替えるのは test-old だけ・前の lock が取れない／大きい版が違えば落とす・lock を変えない PR と PR でない run は今のまま）・D8（仮）（判定と版の確かめは `tools/release/pgsql-prev-lock.sh`、版の確かめは保存の前と使い回しのときの 2 か所、`docs/release.md` の小節を CI の通し方に書き換える）・D9（仮）（前の lock を使い回しの鍵に入れる。正典の鍵の Requirement を MODIFIED）・D10（仮）（更新の後に動いている PostgreSQL の版は見ない）に足した。specs/distribution に Requirement「同梱の PostgreSQL の lock を変える PR では、変える前の lock で組んだ試験の zip から今の lock の zip への更新と戻しを Windows で通す」と 12 の Scenario を足し、tasks に Task 4（4.1〜4.3）を足した。本人の PC で見る前の tasks 5.1 と「人間の確認待ち」の節は削り、CH26.md の完了の判定 6 を置き換え、8（この PR の CI の Windows の更新と戻しが緑）を足した。古い側の zip が黙って今の lock で組まれて 17.12 → 17.12 で緑になる穴は、組んだ zip の同梱の版を確かめる step（使い回しのときも走る）で落とす。本人の PC の移行は ST64 に任せ、この change では扱わない

# 深掘り（第 2 回）

> merge gate（PR #369）で、仮で進めた D8・D9・D10 について、別の席が「失わない」を否とした（`provisional-review.md`。3 つとも条件 4）。どれも、まだ仮の D7（D10 は D8 も）の上に立っているのに、台帳の `rests_on` は本人の決定の D5 だけを挙げていた。D7 が確認バッチで覆っても、台帳からこの 3 つの見直しに辿れない。仮のままにできないので A として本人に返す。台帳から外し（`provisional.py resolve`）、design の見出しの（仮）も外した。問いは `deep-questions-r2.json`・`docs/briefs/CH26-deep-r2.html`。
> 3 つとも、選んだ側で記録や配る物を失うことは無い（別の席の判定の (1)・(2) は通っている）。返すのは「土台の D7 と一緒に本人が決める」ため。

### Q1. 判定と zip の版の確かめを、台本に置くか ci.yml に直接書くか（D8 を返した）

- 問い: lock を変える PR かの判定と、組んだ 2 つの zip の同梱の版の確かめを、台本 `tools/release/pgsql-prev-lock.sh` に置いて ci.yml から呼ぶか、ci.yml の step の中に直接書くか
- 答え: 台本に置き、ci.yml は呼ぶだけ（D8 のまま）（本人の選択。推奨と同じ・補足なし。`deep-answers-4.txt`）
- 決まったこと: design D8 の本文のまま、本人の決定にする（仮でない）。判定と版の確かめは `tools/release/pgsql-prev-lock.sh`、版の確かめは保存の前と使い回しのときの 2 か所、`docs/release.md` の小節を CI の通し方に書き換える
- kind: premise（失うものは無い。土台の D7 が仮のまま、台帳に依存が無かった）
- 出所: `provisional-review.md` D8・design D8
- 推奨: 台本に置き、ci.yml は呼ぶだけ。版の確かめは保存の前と save の後の 2 か所。`docs/release.md` の小節を CI の通し方に書き換える（design D8 の本文のまま）
- 族「仮の D7（前の lock の決め方）の上に立つ CI の決め方」: 前の lock の取り方 → D7（仮。別の席は確か）／判定と確かめの置き場 → Q1／使い回しの鍵 → Q2／動いている版を見るか → Q3／確かめ方そのもの → 第 1 回 R2 = C（本人の決定）

### Q2. Windows の組み立ての使い回しの鍵に、前の lock を入れるか（D9 を返した）

- 問い: 使い回しの鍵に、木の外から来る前の lock の指紋（`prev-pgsql=<sha256|none>`）を足し、正典の鍵の Requirement を `## MODIFIED` で書き換えるか
- 答え: 入れる（D9 のまま）（本人の選択。推奨と同じ・補足なし。`deep-answers-4.txt`）
- 決まったこと: design D9 の本文のまま、本人の決定にする（仮でない）。鍵に `prev-pgsql=<sha256|none>` を足し、正典の鍵の Requirement を MODIFIED で書き換える
- kind: premise（失うものは無い。鍵の値と `none` の意味が D7 から来るのに、台帳に依存が無かった）
- 出所: `provisional-review.md` D9・design D9
- 推奨: 入れる（design D9 の本文のまま）
- 族: Q1 と同じ

### Q3. CI で、更新した後・戻した後に動いている PostgreSQL の版を見るか（D10 を返した）

- 問い: CI の Windows の更新と戻しの試験で、動いている `postgres.exe` の版まで読むか、zip の同梱の版と `update-apply-install.sh` の Scenario 群が通ることだけを見るか
- 答え: 見ない（D10 のまま）（本人の選択。推奨と同じ・補足なし。`deep-answers-4.txt`）
- 決まったこと: design D10 の本文のまま、本人の決定にする（仮でない）。CI は更新の後・戻した後に動いている PostgreSQL の版を読まず、zip の同梱の版の確かめ（D8）と `update-apply-install.sh` の Scenario 群が通ることで見る。`update-apply-install.sh` は変えない
- kind: premise（失うものは無い。費用の見積もりが D8 の版の確かめの上に立つのに、台帳に依存が無かった）
- 出所: `provisional-review.md` D10・design D10・`review/spec.md` R22。第 1 回の合意で本人は IMP-05（CI は更新の後に動いている PostgreSQL の版を見ない）を受けている
- 推奨: 見ない（design D10 の本文のまま。見るには、この change が変えないとした `update-apply-install.sh` に手を入れることになる）
- 族: Q1 と同じ

### 第 2 回のまとめ

- 3 問とも本人が推奨の側（いまの仕様のまま）を選んだ。D8・D9・D10 は本人の決定になり、仮の台帳には戻さない（第 2 回の差し戻しで `provisional.py resolve` 済み）。土台の D7 はまだ仮のまま（別の席は確かとした）
- 要件へ戻すもの: なし（要件は変えない change。CI の決め方を本人が決めた）
- 仕様（design D8〜D10・`docs/changes/CH26.md` の完了の判定 6 の「D7（仮）〜D9（仮）」「D10（仮）」の書き方）へ写すのは、この後の `spec --answers Q1,Q2,Q3`

## 実装の途中の本人の決定（2026-11-23）

### Q-impl-1. 実装の途中で挙がった url の勧告（`RUSTSEC-2026-0592`、#374）をどう扱うか

- 経緯: Task 1.2 の検証で `tools/check-vulns.sh` が url 2.5.8 の新しい勧告で落ち、実装の席が A で返した（診断 `.git/hx/doctor/CH26-202611221920.md`）
- 選択肢: ① CH26 で 2.5.9 に上げ、lock の Scenario と `check-ch26.sh lock` を直す ② 例外の表に足す ③ url だけ別の change にする
- 答え: ①（本人の選択。推奨と同じ）
- 決まったこと: design D11。spec の Scenario「上げた後の Cargo.lock は…」の THEN を rustix・url・form_urlencoded の 3 組に、「全体の検査と毎晩の検査は…」に `RUSTSEC-2026-0592` を足した。完了の判定 1・2・7 も合わせた
- 要件へ戻すもの: なし

## Task 3 の実装の途中で出た問い（2026-11-23）

### Q-impl-2. 「中身が同じ」の比べ方に、退避の戻しで必ず出る表記の違いを許すか（未回答）

- 経緯: `check-ch26.sh minor-swap` の Scenario 1〜3（17.6→17.12→17.6 の置き場の入れ替え）は PASS。Scenario 4（17.12 の `pg_dump --format=custom` を 17.6 の `pg_restore` で空の DB に戻す）だけ、平文の `pg_dump` の比べで 2 行違う（CHECK 制約 `event_end_values`・`event_version_end_values` の括弧の入れ子が `((a AND b) AND c)` → `(a AND b AND c)` に平らになる）
- 実測: **17.12 の退避を 17.12 の空の DB に戻しても同じ 2 行が違う**。小さい版の違いでなく、`pg_restore` が式を読み直す表記の正規化（意味は同じ）
- 問い: design D5 の「中身が同じ」（版の行と鍵の行だけを除いた平文の一致）を、この表記の正規化を許す形に変えてよいか（変えるなら D5 の定義を人間が決める）
- 台本の状態: 上のまま commit 済み・`minor-swap` は rc≠0（Scenario 4）。台本は直していない
