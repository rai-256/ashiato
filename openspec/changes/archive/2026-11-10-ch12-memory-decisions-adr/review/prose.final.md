# CH12 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
Claude のメモリにしか無かった ashiato2 の判断 5 件（ポートの割り当て・衛星の基盤が主・純粋な計算の分け方・画面は表面だけ直す・lint を一段厳しく）が、repo の docs/adr/ に ADR として残る。メモリが消えても、Codex やほかの席からも、なぜそう決めたかを repo の中で読める。

<!-- prose impact.overview -->
メモリにしか無かった判断 5 件が docs/adr/ に ADR として入り、CLAUDE.md の表から辿れるようになる。本文は依頼のまま置くので、repo の今と食い違う 8 つの箇所も含んだまま入る（一覧は docs/changes/CH12.md の節にある）。この分析では、できなくなることは見つけていない。足すのは文書と検証の台本 1 本で、CI には載せないので、後の ADR の書き忘れは CI が止めない。

<!-- prose impact.problem -->
判断の理由が手元の Claude のメモリにしか無く、Codex やほかの席が tools/ports.sh や lint の設定を変えるとき、一度退けた案を知らずに選び直しうる。メモリが消えれば理由も戻らない。

<!-- prose uses[U1].title -->
ポートの決め方の理由を、メモリの無い席で調べる

<!-- prose uses[U1].trigger -->
Codex で動いている席が、新しい種類の作業ツリーを足すことになった。tools/ports.sh を名前のハッシュで簡単にしたくなったが、なぜ台帳で割り当てているのかが分からない。

<!-- prose uses[U1].outcome -->
ハッシュは一度退けた案だと分かり、台帳の割り当てに沿って足す。変えたいなら新しい ADR で置き換える

<!-- prose uses[U1].without -->
理由は手元の Claude のメモリにしか無く、Codex の席からは読めない。同じ取り違えをもう一度起こしうる

<!-- prose uses[U1].steps[0].do -->
CLAUDE.md の「文書とハーネス」の表を読む

<!-- prose uses[U1].steps[0].see -->
`docs/adr/` の行（Story をまたぐ判断の記録）

<!-- prose uses[U1].steps[1].do -->
docs/adr/README.md の一覧を見る

<!-- prose uses[U1].steps[1].see -->
「2026-10-03 worktree の待ち受けのポートは、どの worktree でも被らないよう機械が割り当てる」

<!-- prose uses[U1].steps[2].do -->
その ADR を開く

<!-- prose uses[U1].steps[2].see -->
背景（ST08 の再検証が ST22 の DB に書いた）と、退けた案に「名前のハッシュで番号を決める。衝突しうる」

<!-- prose model.caption -->
この change が置くものと、辿り方

<!-- prose features[place].summary -->
docs/adr/ に README と ADR 5 件を置き、CLAUDE.md の表から辿れるようにする

<!-- prose points[place.1].text -->
docs/adr/ に README（書き方と一覧）と ADR 5 件の計 6 ファイルを置く。ADR は 1 件 1 ファイルで、名前は「時刻-英小文字の slug.md」

<!-- prose points[place.2].text -->
README の一覧は ADR のファイルと 1 対 1 で対応し、日付と題は各 ADR の日付と見出しに一致し、新しい順に並ぶ

<!-- prose points[place.3].text -->
CLAUDE.md の「文書とハーネス」の表に `docs/adr/` の 1 行（Story をまたぐ判断の記録・書き込み: する）を docs/ の行の直後に足す。agent への案内はこの 1 行だけで、AGENTS.md には「ADR を書け」の指示を足さない。いつ ADR を書くかは README が持つ

<!-- prose features[body].summary -->
本文は 1 字も変えず、事実と食い違う箇所は直さずに別の場所に挙げる

<!-- prose points[body.1].text -->
6 ファイルの中身は docs/changes/CH12.md の「依頼（逐語）」の各塊と 1 字も違わない。比べる元の節そのものも、登録したときの中身（sha256）と一致することを確かめる

<!-- prose points[body.2].text -->
ADR の本文は repo の今と食い違う箇所を 8 つ含んだまま入る。たとえば衛星の ADR は「要件の書き直しは未着手・接続は未決」と書くが、要件は 2026-10-11 に書き直し済みで、接続は「いったん MCP」になっている。直さずに PR 本文と docs/changes/CH12.md の「ADR と repo の今の食い違い（直していない）」の節に一覧で挙げる。直す change を起こすかは、一覧を見て本人が決める

<!-- prose points[body.3].text -->
README と 3 つの ADR（ポート・純粋な計算の分け方・lint）は harness2 の ADR を題名で参照するが、その ADR は 2026-11-08 の時点で harness2 の main に無い（push も PR も無い）。リンクではないので壊れたリンクの検査には掛からない。PR 本文にそう書く

<!-- prose features[origin].summary -->
ADR の出所にメモリのファイル名を書き、メモリの残りを数える台本からも外れることを確かめる

<!-- prose points[origin.1].text -->
各 ADR は出所の行を持つ。移した 4 つのメモリ（ポートの 2 つ・衛星の 1 つ・lint と純粋な計算と表面の 1 つ）は、移した先の ADR の出所に名前が出る

<!-- prose points[origin.2].text -->
完了の判定の 1 つ（harness2 の adr_backlog.py が 4 つのメモリを残りに数えない）は、手元でしか確かめられない。台本は harness2 の手元の作業ツリーにしか無く、メモリも手元にしか無い。harness2 の側が main に入るのは待たない。台本かメモリが見つからなければ「環境の欠落」で止まり、黙って合格にはしない

<!-- prose points[origin.3].text -->
台本が 4 つのメモリを本当に数えているかも見る。ADR を渡さずに走らせると 4 つとも残りに出ること（数える対象に入っている）を先に確かめ、そのうえで docs/adr/ を渡すと 4 つとも消えることを見る

<!-- prose features[private].summary -->
repo は公開なので、網の名前・私設 IP と、公開していないメモリの中身を出さない

<!-- prose points[private.1].text -->
docs/adr/ は網の名前・私設 IP を含まない。確かめは既存の tools/check-private.sh に任せ、docs/adr/ のための例外は足さない。この検査は git が追跡しているファイルしか見ないので、先に docs/adr/ の 6 ファイルが追跡されていることを見る

<!-- prose points[private.2].text -->
ADR の本文は依頼として既に公開の origin にあるので、docs/adr/ に置いても新しく外に出る中身は無い。外に出ることは問いにしていない

<!-- prose features[check].summary -->
検証の台本を 1 本足すが、CI には載せない

<!-- prose points[check.1].text -->
足すのは docs/adr/ の 6 ファイル・CLAUDE.md の 1 行・検証の台本 tools/test-adr-docs.sh の 1 本だけ。コード・移行・既存の台本・CI は変えない。台本は、一覧と実ファイルの対応・出所・CLAUDE.md の表・網の名前を見て、自己検査ではわざと壊した木で落ちることを確かめる

<!-- prose points[check.2].text -->
台本は CI に載せない。そのため、後で ADR を足した人が一覧に書き忘れても、CI は止めない（台本を自分で走らせれば見つかる）。書き忘れが実際に起きたら、CI に 1 行足す change を起こす

<!-- prose points[check.3].text -->
依頼の本文と一致するかを見る判定（--verbatim）は、CH12 を受け入れる 1 回のための判定で、正典の Scenario にはしない。後で ADR を「置き換え済み」に直すと、この判定は落ちうる（README が認める手入れなので、それでよい）

<!-- prose impact.scope.covers[0].text -->
依頼に挙がった ashiato2 に固有の判断 5 件と README

<!-- prose impact.scope.preconditions[0].text -->
一覧が対応を持つのは docs/adr/ の README 以外の .md だけ

<!-- prose impact.scope.preconditions[1].text -->
網の名前の検査は git が追跡しているファイルしか見ないので、docs/adr/ が追跡されているときだけ効く

<!-- prose impact.scope.preconditions[2].text -->
出所にメモリのファイル名を書くのは、メモリから移した判断の ADR だけ

<!-- prose impact.scope.excludes[0].text -->
ハーネスの判断（harness2 の docs/adr/ が持つ）

<!-- prose impact.scope.excludes[1].text -->
ADR の本文の手入れ

<!-- prose impact.scope.residual[0].text -->
ADR の本文は repo の今と食い違う 8 つの箇所を含む。読む人が古い「影響」を今の状態と取り違えうる（一覧は docs/changes/CH12.md の節）

<!-- prose impact.scope.residual[1].text -->
harness2 の ADR への参照は、harness2 の側が main に入るまで行き先が無い

<!-- prose impact.scope.residual[2].text -->
台本を CI に載せないので、後の ADR の一覧の書き忘れは止まらない

