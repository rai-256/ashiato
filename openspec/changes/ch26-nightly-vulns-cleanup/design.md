# CH26 design — 夜間の脆弱性検査の 4 件の処置

## Context

依頼（`docs/changes/CH26.md`）は「互換の範囲で上げる。上げられないものは例外の表に `届かない:` か `上げる予定:` と 90 日以内の期限で書き、勧告が実際に当たる使い方かも確かめて理由に書く」と決めている。
同梱の PostgreSQL は「minor を 17.12 に上げ、major は変えない。既に入っている PC で minor の更新が既存の DB をそのまま使えることを確かめる」。
外の事実は `premises.md`（P1〜P21）。CH22・CH24・CH25 は 2026-11-21 に main へ merge 済み（archive 済み）。実装は main の上で始める（`ci.yml`・`Cargo.lock`・`osv-scanner.toml` は main の形が元）。

> 第 1 回の合意で本人が R2 の答えを A から C に変えた（`deep.md` の「仕様への違和感（第 1 回の合意）」）。D5 を書き直し、CI の決め方を D7（仮）〜D10 に足した。D8・D9・D10 は第 2 回の深掘りで本人が今の本文のまま決めた（`deep.md` 第 2 回 Q1〜Q3）。

## Goals / Non-Goals

- Goals: `tools/check-vulns.sh`（全体）・`tools/check-vulns.sh nightly`・`tools/check-pg-minor.sh` が通る。#334〜#337 を閉じられる状態にする。`pgsql.lock` を変える PR では、CI が前の lock の zip から今の lock の zip への更新と戻しを Windows で通す（この PR が最初の 1 回）
- Non-Goals: chrono の置き換え（jiff / time）・rusqlite 0.41 への上げ（どちらも本体のコードに及び、互換の範囲の外）・検査の台本（`tools/check-*.sh`）の変更・大きい版を上げる道（`pg_upgrade`）・本人の PC に入っている配布版の移行（ST64）

## Decisions

### D1. rustix は `Cargo.lock` だけで 1.1.6 以上に上げる

`cargo update -p rustix@1.1.5 --precise 1.1.6`（1.1.6 より新しい 1.1.x があればその最新でよい）。`tempfile` 3.27.0 の範囲（rustix `^1`）の中なので `Cargo.toml` は変えない。
rustix 0.38.44 は勧告の範囲の外（P2）なので触らない。`cargo update` を crate を名指しせずに走らせない（ほかの依存を動かすと「振る舞いを変えない」の証跡が広がり、CH24 との衝突も増える）。

### D2（仮）. libsqlite3-sys は例外の表に `届かない:` で書く

直る版（libsqlite3-sys 0.39 = rusqlite 0.41）は、`crates/collector-windows/Cargo.toml` の `rusqlite = "0.32"` の互換の範囲（0.32 の系列）の外（P4）。上げるには依存の範囲を替える。依頼の「互換の範囲で上げる」を越えるので、この change では上げない
（Rust の版は障壁ではない。repo は toolchain を固定せず stable を使う。spec-review R4）。

勧告が当たるのは「信頼の低い者が書けた DB と `-wal` を一緒に開く処理」（P5）。PC の収集アプリは、本人のブラウザの履歴を `-wal` ごと写して読み取り専用で開く（P6）ので、**形は当たる**。
ただし、利用者の置き場の下の profile を書けるのは本人と同じ利用者の権限を持つ処理で、そういう処理はもともと収集アプリの記憶を読める（P7。確かさ 1）。勧告の攻撃（問い合わせの結果に処理の記憶の古いバイトが混ざる）で、攻撃する側が新しく得るものは無い、と読む。
Firefox の `IsRelative=0` の profile は利用者の置き場の外も指せる（P16）。そこが本人以外にも書ける場所なら、この読みは成り立たない。それは反転条件に置く。

**正典の「届かない」の型との違い**（spec-review R6）: 正典（`dependency-vulnerability-scan` の「例外の表の行は理由と期限を持ち…」）は `届かない:` を「製品に届かない理由」と定め、今の表の `届かない` は全部「組み立ての道具専用」「試験専用」。この勧告は製品（PC の収集アプリ）に入っている。
それでも `上げる予定:` にしないのは、上げる change がまだ無く、予定と書くと事実と違うから（CH24 は例外の理由が事実と違っていたことから立った）。`届かない:` の後ろに「なぜ攻撃が新しく何も得ないか」を書き、確認バッチの仮の決定の一覧で本人に見せる。型を足すのは検査の台本と正典の変更で、この change の外。

例外の行: `id = "RUSTSEC-2026-0459"`・`ignoreUntil` は書いた日から 90 日以内・`reason = "届かない: 開くのは本人のブラウザが書いた履歴の写しで、書ける者は収集アプリの記憶をもともと読める"`（形は `tools/check-vulns.sh` の「届かない: 」で始まる規則に合わせる）。
osv-scanner は別名でも外すが、表に置く ID は検査が出した ID（`RUSTSEC-2026-0459`。P1）にする。検査が別名（`RUSTSEC-2026-0502`）でも当たりを出すなら、その行も同じ理由で足す（tasks 1.2 で走らせて決める）。

- 失わない側: 例外は 90 日で切れて検査がまた落ちる。本人が上げると決めれば、rusqlite 0.41 へ上げる change をいつでも立てられる。何も捨てない
- **反転条件**: 収集アプリが、ブラウザの履歴の写しのほかに本人以外が書ける SQLite のファイルを開くようになった／本人以外も書ける置き場の Firefox の profile（`IsRelative=0`）を読む使い方が本人にある／P7 が外れる（同じ利用者の処理が収集アプリの記憶を読めない設定を前提にする）と分かった／本人が上げると決めた。どれかに当たれば `届かない` をやめて、rusqlite を 0.41 へ上げる change を立てる

### D3（仮）. chrono は例外の表に `届かない:` で書き、期限が来たら検査がまた落ちるままにする

直した版は無い（P8。保守終了の知らせ）。置き換え（jiff / time）は 5 つの crate の本体と sqlx・utoipa の feature に及ぶ（P9）ので、この change の範囲の外。

勧告は欠陥の知らせではなく、保守が終わったという知らせ。今わかっている欠陥は無い。「上げる予定」は書かない（上げる版が無く、置き換えの change もまだ無いので、予定と書くと事実と違う）。
chrono は製品が直に使うので、正典の「製品に届かない」の型にも素直には当たらない（D2 と同じ。spec-review R6）。`届かない:` の後ろに「届くのは欠陥ではなく保守終了の知らせ」であることを書く。

例外の行: `id = "RUSTSEC-2026-0543"`・`ignoreUntil` は書いた日から 90 日以内・`reason = "届かない: 保守終了の知らせで、直す版も既知の欠陥も無い"`。

期限が来ると、夜間の検査はこの行を「期限切れ」で落とし、issue が立つ。そのとき本人が、置き換えるか期限を延ばすかを決める。この change はその判断を前払いしない。

- 失わない側: 何も捨てない。行は 90 日で切れて、本人の前に戻る
- **反転条件**: chrono に欠陥の勧告（保守終了でない勧告）が出た／保守を引き継いだ版が出て勧告が直った版を示した。どちらも夜間の検査が新しい ID で落として知らせる

### D4. 同梱の PostgreSQL は `pgsql.lock` の 3 つの値をそろえ、DLL の一覧を 17.12 の zip で調べ直す

`version=17.12-1`・`url=https://get.enterprisedb.com/postgresql/postgresql-17.12-1-windows-x64-binaries.zip`・`sha256=<落とした zip の値>`。sha256 は spec に書かない（実装のときに落とした実物から取る。`tools/release/pgsql.sh fetch` が照合する）。

`dlls=` と VC の実行時の DLL の注記は 17.6-1 の zip について調べたもの（P13）。17.12-1 の zip で、注記と同じ手順（`x86_64-w64-mingw32-objdump -p` で実行ファイル・一覧の DLL・`lib/*.dll` の取り込みを辿る）で調べ直し、
足りない DLL があれば一覧に足し、要らなくなったものは外す。注記の日付と「the zip with the sha256 above」の対象を 17.12-1 に書き換える。
ICU の版（`icuuc67.dll` など）が変わっていれば一覧の名前も替える。

大きい版は 17 のまま（`crates/installer/build.rs` は `version=` の `.` の前だけを読む。P12）。実行ファイルの一覧・`license=` の置き場は変えない（17.12 の zip にその置き場が無ければ、`tools/release/pgsql.sh slim` が落ちる。そのときは置き場を 17.12 に合わせる）。

### D5. 小さい版の入れ替えは、PostgreSQL の置き場の入れ替えを Linux で、配布物どうしの更新と戻しを lock を変える PR の CI の Windows で確かめる（本人の決定 R2 = C）

依頼は「既に入っている PC で minor の更新が既存の DB をそのまま使えることを確かめる」。`docs/release.md` の手順は「変更前の小さい版を同梱した zip から更新と戻しの両方を通す」。
今の CI は古い zip と新しい zip を同じ commit・同じ lock から組むので、小さい版の違う zip の間の更新を走らせない（P14）。本人は「手で確かめる手間を最低限にしたい」として、これを CI で機械が行う形を選んだ（`deep.md` の R2、第 1 回の合意で A → C）。

確かめは 2 つ:

1. **Linux（この change の台本）**: `check-ch26.sh minor-swap`（Docker の `postgres:17.6` / `postgres:17.12`。initdb はインストーラと同じ引数。P18）。
   17.6 で置き場を作り、サーバの移行を当てて行を入れる → 同じ置き場を 17.12 で起こし、中身が同じ・移行をもう一度当てても足すものが無い → 17.12 で 1 行足す → 同じ置き場を 17.6 で起こし、中身が同じ → 17.12 の `pg_dump --format=custom` の退避を 17.6 の `pg_restore` で空の DB に戻し、中身が同じ。
   「中身が同じ」は、同じ `pg_dump`（17.12 の像の中のもの。平文）の出力から、版の行（`-- Dumped from` / `-- Dumped by` で始まる行）と毎回変わる鍵の行（`\restrict` / `\unrestrict` で始まる行）を除いて一致することをいう。
   Windows の runner より先に、手元で PostgreSQL の側の約束が崩れていないかを見る（崩れていれば Windows を待たずに止まる）
2. **Windows（CI）**: `pgsql.lock` を変える PR では、`package-windows` が更新の試験の古い側の zip（`package-windows-test-old`。版 `0.4.0`）だけを **PR が変える前の lock（base との分岐点の lock）** で組み、新しい側（`package-windows-test-new`。版 `0.5.0`）は今の lock で組む。
   `install-windows` は今のまま `tools/release/update-apply-install.sh` に 2 つを渡すので、17.6-1 の zip を入れた上で 17.12-1 の zip で更新し、移行を落として 17.6-1 へ戻す Scenario 群が Windows で走る。lock を変えない PR・main への push の組み立てと試験は今のまま（両方とも今の lock）

lock は `tools/release/pgsql.sh`（`PGSQL_LOCK_FILE` を読む）のほかに、`tools/release/build.sh`（許諾の文の版と DLL の一覧）と `crates/installer/build.rs`（大きい版）が `tools/release/pgsql.lock` を直に読む（P19）。だから古い側を組む間だけ、作業木の `tools/release/pgsql.lock` を前の lock で置き換えて組み、組んだら戻す（`Cargo.toml` の版を替えて組んで戻す今の形と同じ）。組み立ての台本は変えない。

この change では Windows の配布物どうしの更新と戻しを本人の PC で見ない（前の tasks 5.1「本人の PC で更新して記録が見えるか」を削った）。本人の PC に入っている配布版の移行は ST64 が扱う。

- **反転条件**: `minor-swap` のどこかで中身が変わる・起きない・戻せない／lock を変えたこの PR の `install-windows` の更新と戻しが、小さい版の違いが原因で落ちる（→ どちらもこの change を止めて A として本人に返す）／インストーラに小さい版で分かれる処理が入った（→ CI の確かめの範囲を見直す）

### D6. 確かめの台本は change の中に置く

Scenario の印を持つ試験は、予定のパスの外（`tools/**`）に置かない。`openspec/changes/ch26-nightly-vulns-cleanup/check-ch26.sh` に置き、サブコマンドごとに `echo "== Scenario: <名前>"` を出す（`scripts/check_scenarios.py` は `git ls-files` の全体から印を拾う）。
archive の後は `openspec/changes/archive/…` へ一緒に移る。この台本は CI に組み込まない（一度きりの更新の確かめ。夜間の検査と `check-pg-minor.sh` が以後の遅れを見張る）。

### D7（仮）. 「lock を変える PR」は PR 自身の差分で決め、前の lock は分岐点の lock。入れ替えるのは古い側の 1 つだけ

- **変えたか**: 変更の判定の job（`changes`。履歴を全部持つ）が、PR の run でだけ、PR の base と head の分岐点（`git merge-base`）から head までの差分に `tools/release/pgsql.lock` があるかを `pgsql-prev-lock.sh changed <base> <head>` で決め、`pgsql`（`true` / `false`）と分岐点の commit を出力に置く。
  base の先端と head の 2 点で比べると、base の側だけが lock を上げた後の古い枝が「変えた」になり、小さい版を下げる向きで更新を走らせる（spec-review R19）。分岐点からの差分なら、その枝は「変えていない」で今のまま組む
- **前の lock**: 分岐点の commit の lock（PR が変える前の lock）。`package-windows` は `pgsql == true` の run でだけ分岐点を浅く fetch して取り出す
- **落とす**: `pgsql == true` なのに分岐点の lock が取れない・`decide` が「同じ中身」と言う・大きい版が違う、のどれかなら `package-windows` を落とす（取れないときに今の lock へ倒すと、17.12 → 17.12 で緑になる。spec-review R16）
- **今のまま**: `pgsql == false`・PR でない run（main への push・手動）では、前の lock を取らず、台本も呼ばない（CH26 より前の head の枝でも、lock を変えていなければ今のまま組める。spec-review R19）
- 入れ替えるのは `package-windows-test-old` だけ。`dev-old`・`compat-old` は今の lock のまま（本人の補足が名指ししたのは test-old。開発版・互換の組の Scenario は版番号の違いを見るもので、PostgreSQL の版を見ない）

- 失わない側: 何も捨てない。CI の定義の中の決め方で、変えれば次の run から効く
- **反転条件**: 分岐点の後に main も lock を変えた PR が出て、どちらから上げる道を見るかが問題になった／開発版・互換の組でも小さい版の違いを見る必要が出た／大きい版を上げる Story が始まった（そこで大きい版の違う前の lock の扱いを決め直す）

### D8. 判定と確かめは台本 `tools/release/pgsql-prev-lock.sh` に置き、ci.yml は呼ぶだけ

> 本人の決定（第 2 回 Q1「台本に置き、ci.yml は呼ぶだけ」）。仮の D7 の上に立つ —— D7 が覆れば `changed`・`decide`・`check-workflow` を見直す。

ci.yml の中の bash は Windows の runner でしか走らず、手元で落ちるかを確かめられない。check_scenarios も ci.yml の印を拾わない（`.yml` は試験の拡張子に無い。spec-review R17）。だから判定と確かめを台本に置き、Scenario の印も台本に置き、`--self-test` を Linux（`chain` の job）で走らせる:

- `changed <base> <head>`: 分岐点から head までの差分が lock を変えたら `true <分岐点>`、変えていなければ `false` を出す（`changes` の job の Linux で走る）
- `decide <前の lock> <今の lock>`: 中身が違い大きい版が同じなら `swap`、同じなら `same` を出して rc=0。大きい版が違えば 2 つの大きい版を出して rc≠0。`version=` が無い・`<大>.<小>-<n>` の形でない lock も rc≠0。中身は行末の CR を落として比べる（Windows の checkout は `core.autocrlf` で作業木を CRLF にしうる。`build.sh:46`。spec-review R26）
- `zip-version <サーバの zip>`: zip の `THIRD-PARTY-NOTICES.txt` の `== PostgreSQL` の節の `version: ` の値を出す（`build.sh` が組んだときの作業木の lock から書く。P19）。節か値が無ければ rc≠0
- `check-zips <前の lock> <今の lock> <古い側の zip> <新しい側の zip>`: 古い側の版が前の lock の版、新しい側の版が今の lock の版なら `古い側 <版> → 新しい側 <版>` を出して rc=0、違えば rc≠0
- `check-workflow [<ci.yml>]`: ci.yml の形を python3 と yaml で読んで見る（Linux の `chain` でだけ走る。`windows-build-key.sh --check-workflow` と同じ）: `changes` の job が `changed` を PR の run でだけ呼んで出力を持つ／`package-windows` の前の lock を取る step が `pgsql == 'true'` を条件に持ち鍵の step より前にある／鍵の step の道具の版の文字列が前の lock の指紋（取らなければ `none`）を持つ／試験の zip を組む step が `old` を組む間だけ lock を置き換えて組んだ後に戻し、同じ step の中で `check-zips` を呼ぶ／save の step の後に、`cache-hit` を条件に持たず `pgsql == 'true'` を条件に持つ `check-zips` の step がある。自己試験は、形を 1 つずつ崩した写し（spec の Scenario の 7 通り）で落ちることを見る
- `changed`・`decide`・`zip-version`・`check-zips` は Windows の Git Bash でも動くよう bash・git・unzip・sha256sum・awk/sed だけで書く（python を使わない。`windows-build-key.sh:11` と同じ制約。spec-review R26）

**版の確かめの置き場**（spec-review R18）: (1) 試験の zip を組む step の中（`cache-hit != 'true'` の中。保存の前）で、`old` と `new` を組んだ後に `check-zips` を呼ぶ —— 食い違った組み立てを保存させない。(2) save の step の後に、引けたかどうかに依らず走る `check-zips` の step を置く —— 使い回した run でも確かめる。
今の `windows-build-key.sh --check-workflow` は、upload の前と、最後の upload と save の間に、`if:` の無い step を足すと落とす（実測。spec-review R18）。この 2 か所ならその検査を変えずに足せる（save の後の step は rc=0 を実測。step の中の呼び出しは step の数を変えない）。`windows-build-key.sh` は変えない。

`docs/release.md` の「PostgreSQL の小さい版を上げる PR」の小節は、手で通す手順から「CI が通す。PR では `install-windows` が緑で、`package-windows` の出力に `古い側 <前の版> → 新しい側 <今の版>` があることを見る」に書き換える（本人の補足「docs/release.md の手順を CI で機械が行う」）。main では §8（この小節を含む）の頭に「置き換え中」の ★ の注記がある（`docs/release.md:7`）。注記は残し、小節だけを書き換える。

- 失わない側: 何も捨てない。台本は CI の確かめの道具で、配る物に入らない
- **反転条件**: 組み立てが許諾の文の版の行を書かなくなった（`zip-version` の読み先を替える）／`windows-build-key.sh --check-workflow` の形の検査が変わり、2 か所のどちらかに置けなくなった

### D9. Windows の組み立ての使い回しの鍵に、前の lock を入れる（正典の鍵の Requirement を MODIFIED で書き換える）

> 本人の決定（第 2 回 Q2「入れる」）。`prev-pgsql` の値と `none` の意味は仮の D7 から来る —— D7 が覆れば値を見直す。

使い回しの鍵（`tools/release/windows-build-key.sh`）は HEAD の木（`pgsql.lock` を含む）と道具の版から出す（P21）。前の lock は木の外から来る入力なので、鍵に入れないと、同じ head で前の lock だけが違う組み立て（分岐点を動かした後の push など）を引きうる。
鍵の step の `WINDOWS_BUILD_KEY_TOOLS` の末尾に `|prev-pgsql=<前の lock の sha256、取らなければ none>` を足す（前の lock を取る step の出力から）。鍵の step・restore・保存の形は今の `--check-workflow` のまま通る（実測。spec-review の第 2 回の「実物で確かめたこと」）。
正典の Requirement「PR の CI は、組み立ての入力が同じ Windows の配布物を使い回し、インストールの試験は全部走らせる」は鍵の入力を列挙しているので、`## MODIFIED` で入力に前の lock を足し、Scenario「鍵を出す手順は、古い側を組んだ前の lock の指紋を鍵に入れる」を足す（印は `pgsql-prev-lock.sh` の `check-workflow`。spec-review R20）。

- 失わない側: 何も捨てない。鍵が変わると組み直すだけ（約 25 分の Windows の組み立て）
- **反転条件**: 使い回しの鍵の作り方を変える change が、木の外の入力を別の形で扱うようになった

### D10. CI は、更新した後・戻した後に動いている PostgreSQL の版を見ない

> 本人の決定（第 2 回 Q3「見ない」）。費用の見積もりは D8 の zip の版の確かめがある前提。

`install-windows` は `update-apply-install.sh` の Scenario 群を古い側（17.6-1）と新しい側（17.12-1）の zip で通し、`package-windows` は 2 つの zip の同梱の版を確かめる。インストーラが更新でプログラムの置き場（`pgsql` を含む）を入れ替え、戻しで前の版の置き場に戻すことは正典の Requirement（「同梱の PostgreSQL の大きい版が違う配布物では、何も替えずに止まる」の 2 文目）が言うが、その一文を見る Scenario は無く、`update-apply-install.sh` も動いている `postgres.exe` の版を読まない（spec-review R22）。
この change では見ない。見るには `update-apply-install.sh` の判定（組み立ての台本で、この change の「変えないもの」）に `pgsql\bin\postgres.exe -V` を読む所を足すことになり、本人の補足の範囲（「既存の update-apply-install.sh で通す」）を越える。

- 失わない側: 何も捨てない。インストーラはアプリのプログラムの置き場ごと入れ替える（P12・P17）ので、版の観測を後から足す change をいつでも立てられる
- 費用: 見ない間は、インストーラが `pgsql` の置き場だけを入れ替え損ねても CI は緑になりうる（そのとき新しいサーバは古い PostgreSQL で動く。同じ大きい版なので DB は読める）
- **反転条件**: インストーラのプログラムの置き場の入れ替え方が変わった（`pgsql` を別に扱うようになった）／本人が「見る」に変えた（`update-apply-install.sh` に版の観測を足す change を立てる）

### D11. url は `Cargo.lock` だけで 2.5.9 以上に上げる（本人の決定。2026-11-23）

> 実装の途中（Task 1.2）で、計画に無い勧告 `RUSTSEC-2026-0592`（url 2.5.8。入れ子の blob: URL で `Url::origin()` がスタックあふれ。2.5.9 で直る）を夜間の検査が挙げた（2026-11-22。#374）。
> 実装の席は範囲が変わるので A で返し、本人が「CH26 で上げる」（例外にする・別の change にする、ではなく）を選んだ。直る版が互換の範囲にあるので、`AGENTS.md` の「互換の範囲で上げる」に当たり、例外にする理由が無い。別の change にしても CH26 の完了の判定 1（検査が rc=0）が url で落ちる。

`cargo update -p url@2.5.8 --precise 2.5.9`。url 2.5.9 は form_urlencoded 1.2.3 を要るので、これも 1.2.2 → 1.2.3 に動く（P22）。`Cargo.toml` は変えない。
url を引くのは sqlx（DB の接続文字列の解釈）だけで、この repo のコードは `Url::origin()` を呼ばない。form_urlencoded は axum・serde_urlencoded も使う（パッチ版の上げ）。
`check-ch26.sh lock` は、動いてよい組を rustix・url・form_urlencoded の 3 つにする。`vulns` は `RUSTSEC-2026-0592` も出ないことを見る。

- 失わない側: 何も捨てない。パッチ版の上げで、戻すのも lock の 2 行
- **反転条件**: url 2.5.9 で sqlx の接続文字列の解釈が変わり、試験（tasks 5.2）が落ちた → 上げをやめ、例外の行（`上げる予定:`）にして本人に返す

## Risks / Trade-offs

- **D2 は P7（確かさ 1）に乗っている**。Firefox の `IsRelative=0` の profile が本人以外にも書ける置き場にあれば成り立たない（P16）。Windows で同じ利用者の処理が別の処理の記憶を読めないように絞った環境（保護された処理など）では、勧告の攻撃が新しく何かを得うる。収集アプリは保護された処理として動かない（そういう設定をこの repo は持たない）が、確かめていない
- 例外の 2 行は 90 日で切れ、chrono は置き換えるまで 90 日ごとに夜間の検査を赤にする。それ自体が本人への知らせになる（D3）
- lock を変える PR の CI は、古い側の zip を前の lock で組むので、PostgreSQL の zip を 2 つ落とす（約 370 MB ずつ。`pgsql.sh fetch` は版ごとの名前で置き場に置き、キャッシュに 2 つ残る。P20）。lock を変えない PR は今のまま
- Windows の更新と戻しの確かめは CI の run にあり、Task の手元の検証では走らせられない。手元で見るのは台本の自己試験と 2 つの `check-workflow`。lock を変えた PR では、版の組が違えば `package-windows` が落ちる（D7・D8）ので、CI が緑であることが `17.6-1 → 17.12-1` の組で更新と戻しが通ったことを含む（完了の判定 8。merge gate は CI の緑を要る）
- CI は、更新した後・戻した後に動いている PostgreSQL の版を見ない（D10。本人の決定）
- CH22（`install-windows` に置く段）・CH25（gradle のキャッシュ・`chain` の頭）は main に merge 済みで、`ci.yml` の別の行を変えた。CH26 は `changes`（出力を 2 つ足す）・`package-windows`（前の lock・鍵・古い側の組み立て・版の確かめ）・`chain`（2 行）だけを変える
- CH24（merge 済み）が main で `Cargo.lock`・`osv-scanner.toml` を変えた。実装は main の上で取り直す（proposal の Impact）
