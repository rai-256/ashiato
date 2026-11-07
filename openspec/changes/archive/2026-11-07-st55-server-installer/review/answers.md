spec-digest: 3c41dfbebecb766d3c6b83614cbb11070fd6472d658b5028c7da06b8f58c6e75
- Q10: 写った
答え以外の変更: あり（`repair` の最初（(a) の後・(b) の前）に、前の `repair` が (c) の後で落ちて残した一時の `postgres` を `pg_ctl stop -D <DB の置き場> -m fast` で止め、一時の `hba_file` と `pg_ident` を消す段を design D27 に足し、tasks 6.2 の「repair の途中で落ちても、もう一度 repair できる」を一時の `postgres` を残したまま落ちる形で走らせることにした（829bf324。provisional-review.md の D28 の「あわせて（否の理由ではない）」(2) から。D27 は仮でない））
答え以外の変更の決定: D27

<!-- 以下は機械が読まない。確かめの中身 -->

## 確かめた範囲

基準は合意を記録したときの head（a5524cc7）。`git diff a5524cc7 HEAD -- openspec/changes/st55-server-installer/{proposal.md,specs,design.md,tasks.md}` を全部読んだ（HEAD は f0ce603f）。加えて、呼び出し元が気づいた点 1・2 を直した作業木の tasks.md（commit 前。`git diff -- openspec/changes/st55-server-installer` は tasks.md の 4.1・5.2 だけ）も読んだ。digest はその作業木で取った。proposal.md と specs/ は差分なし。差分は design.md（D1・D7・D27・D28・Risks）と tasks.md（Global Constraints の本人の決定と仮の一覧・4.1・6.2）だけ。
間の commit は 91d9d01c（provisional-review.md だけ）・829bf324（D28 を台帳から外し Q10 を立てる。design・tasks も触る）・07d40e27（deep.md と答えのファイルだけ）・f0ce603f（Q10 の写し）。829bf324 と f0ce603f を個別に `git show` で読み、どの変更がどちらから来たかを分けた。
台帳（provisional.jsonl）の最後の状態: D28・D10・D17 は resolved、D3・D16・D18・D26 は active。design の見出しの「（仮）」も D3・D16・D18・D26 だけで一致する。design・tasks に「未決」「答え待ち」は残っていない（829bf324 で入れた「未決。Q10 の答え待ち」は f0ce603f で全部外れている）。

### Q10（DB を作るときに名前を固定する（`initdb -U ashiato`）。補足なし）—— 選択肢の detail の項目ごと
- 「design D1 の `initdb` に `-U ashiato` を足す（開発の DB の管理者の役割と同じ名前）」: design.md:32 の命令に `-U ashiato`、design.md:33 に「`tools/db-roles.sh` の `POSTGRES_USER` の既定と同じ」（tools/db-roles.sh:39 の `admin="${POSTGRES_USER:-ashiato}"` と合う）。tasks 4.1 の命令と Linux の試験に「`-U ashiato` がある（Q10）」
- 「インストールの役割の段も `repair` の一時の接続の規則も、この固定の名前で入る」: design.md:33（D10 の (7)・D7 と D28 を名指す）・D7（design.md:76。`db-roles.sql` の `admin` の変数に同じ名前）・D27 の (c)（design.md:220）・D28 の本文（design.md:229 の `host all ashiato 127.0.0.1/32 sspi`・`pg_ident` の結び先・`psql -U ashiato`）・反転条件（`local all ashiato trust`）。tasks 6.2 の段と Linux の試験（一時の `hba_file` の役割が `ashiato`・`psql -U ashiato`・設定が無くても組める）
- 「設定のファイルにも別の置き場にも名前を控えない」: design.md:33 の最後の文。D4 の設定の鍵（design.md:54-）に名前の鍵は足されていない。tasks の Global Constraints の Q10 にも「設定にも別の置き場にも控えない」
- 「D28 の土台に D1・D4・D6 を挙げる」: D28 の注（design.md:227）に「土台は D1（`-U ashiato`）・D4（設定に名前を控えない）・D6（一時の起動も同じ `postgresql.conf` を読み、文の字面をログに書かない）と D27・D21」。D28 は答えに乗る決定として仮を外したので台帳には載らず、rests_on の欄は無い（台帳の D28 は resolved のまま）
- 「Windows の CI で『別の管理者 B が `repair` しても入れる』を見る（spec の既存の Scenario）」: spec の既存の Scenario「別の管理者が repair しても、本人は変わらない」（specs/distribution/spec.md:327。差分なし）を tasks 6.2 の install-check.sh の項目が持つ。この Scenario の THEN は「作り直した設定の本人は A」で、設定は (c) の役割の段の後にだけ置かれる（design D27）ので、B が `ashiato` に入れなければ THEN は偽になる —— 入れることを間接に見る
- 「費用は `initdb` の引数 1 つ」: 足したのは引数 1 つと、それを読む D7・D27・D28・tasks の名指しだけで、新しい置き場・鍵・口は無い
- 選ばなかった 2 つ（状態の置き場に名前を控える・`pg_ident` の `all`）の内容は、design・tasks のどこにも入っていない

### 答えの写しとしたもの（名指しに入れない）
- 作業木の tasks.md の直し（呼び出し元。commit 前）: 4.1 の install-check.sh の「配る DB は OS に依らない並べ方で作られ、符号の順に並ぶ」に「管理者の役割（`rolsuper`）が `ashiato` の 1 つだけ」を見ることを添えた・5.2 の役割の段を `psql -U ashiato`（`admin` の変数も `ashiato`）と名指し、Linux の試験に「`psql` の命令が `-U ashiato` で入る」を足した。どちらも detail の「インストールの役割の段も…この固定の名前で入る」と D1 の CI の確かめを tasks に降ろしたもので、新しい振る舞いも決定も足していない。答えの写しとした（D1・D7 の中身で、spec は変わっていない）
- D1 の Windows の CI の確かめに「管理者の役割（`rolsuper`）が `ashiato` の 1 つだけ」を足した（design.md:36）: `-U ashiato` が効いたことを見る確かめで、D1 の答えの帰結
- D28 の見出しから（仮）を外し、本文の `<DB の管理者の役割>` を `ashiato` に替え、否とされた経緯と答えの注を付けた・Risks の「D28（仮）の反転条件」を「D28 の反転条件」にした・tasks の仮の一覧から D28 を外し本人の決定に Q10 を足した: 台帳から外して Q10 で返した D28 を、答えで決めた側に書き直したもの

### 答え以外としたもの（D27）
- design D27 の「`repair` の最初に前の一時の形が残っていれば片付ける」段（design.md:221）と、tasks 6.2 の「途中で落ちても、もう一度 repair できる」を一時の `postgres` を残す形で走らせる指定は、829bf324 で足した。design 自身が「答えに依らない」と書き、Q10 の選択肢の detail にも無い（問いの family に「design D27（…答えに依らない）」として既決の扱いで載っていただけ）。D27 の節の中身として書かれているので D27 を名指す。D27 は仮でない（見出しに（仮）が無く、台帳に D27 の行も無い）
- 振る舞いとしては、合意した Scenario「repair の途中で落ちても、もう一度 repair できる」（spec.md:332。差分なし）を成り立たせるための段で、spec は変わっていない

### 食い違いではないが、気づいたこと
- （直った）design D1 の Windows の CI の確かめ「管理者の役割（`rolsuper`）が `ashiato` の 1 つだけ」（design.md:36）を持つ task が無かった → 作業木の tasks 4.1 の install-check.sh の項目に添えられた。spec の Scenario「配る DB は OS に依らない並べ方で作られ、符号の順に並ぶ」（spec.md:120-123）の THEN は役割に触れないままなので、この確かめは tasks の層でだけ持つ（Scenario の名前の印の下で見る）
- （直った）tasks 5.2 が `psql -U ashiato` / `admin` の変数の値を名指していなかった → 作業木で名指され、Linux の試験も足された
- （呼び出し元は対処しない。D28 の形は Q10 の問いの前提で、本人はその上で選んだため）D28 は Q10（名前の出所）への答えで仮を外したが、本文の起こし方（`pg_ctl start` に一時の `hba_file` と `sspi`。`postgres --single` を使わない）・反転条件は、第 2 回の写しで AI が仮で決めた中身のまま。Q10 の問いの文がこの形を前提に書かれていて本人はその上で選んだので、仮を外すこと自体は答えに沿う。上がった札で `sspi` が通るかは確かさ 1 のまま（Risks と tasks 6.2 の反転条件に残っている）
