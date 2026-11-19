# CH21 spec レビュー（第 1 回の先読み `spec --draft` の版。2026-11-19）

対象: `openspec/changes/ch21-trial-use-before-verify/`（proposal / specs/trial-use/spec.md / design.md / tasks.md / premises.md / deep.md）と `docs/changes/CH21.md`。
deep.md は全問が `（答え待ち・推奨: …）`。推奨を本人の答えとみなして突き合わせた。

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate ch21-trial-use-before-verify --strict` | rc=0（valid） |
| `python3 scripts/check_chain.py .` | rc=0（chain: OK。観点 8 は change なので無し） |
| `python3 scripts/check_scenarios.py . ch21-trial-use-before-verify` | rc=1。この change の担保なし 21 件・人間の確認待ち 1 件（「本物の確認バッチで、手順書の冒頭に使ってみた段落が載る」）。実装の前なので担保なしは見込みどおり。22 本の名前はすべて tasks.md が一字一句で名指ししている（突き合わせ済み） |
| `python3 scripts/review_triage.py . ch21-trial-use-before-verify` | rc=1。`review/deep.md` の処置の形で 17 件 FAIL（R23） |
| `python3 scripts/spec_overlap.py candidates . ch21-trial-use-before-verify` | `[]`（0 件） |

---

## R1. 公開の手順書に載せるものが、Q7（A）の推奨「件数・題 1 行・場所だけ」を越えている
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/trial-use/spec.md
- 根拠: deep.md:38（Q7 の推奨「手順書には件数・各候補の題 1 行・手元の報告の場所だけ」。deep-questions.json の Q7 の irreversible も「候補の題 1 行ずつは公開の手順書に出る」だけ）/ spec.md:128（「件数・各候補の題 1 行・**巡った目的の一覧**・手元の報告の場所だけ」）・spec.md:129・spec.md:143 / design.md:123-124（D12 の段落の形に `<所要> 分・<終わり方>`・`手順書には上位 <M> 件`・巡った目的）/ docs/changes/CH21.md:54（「件数・題 1 行ずつ・巡った目的・手元の報告の場所だけ」）。
  「巡った目的」は D6 の goals.md の写しではなく、Claude が巡ったと書いたもの（spec.md:142「巡った目的を 3 つ書き…偽の Claude」）なので、Claude の書いた文が公開の repo と PR の本文に出る。本人が Q7 の選択肢で読む「出るもの」に入っていない
- kind: conflict
- 提案: Q7 の推奨の文と irreversible に「巡った目的（Claude が書く）と所要・終わり方」を足してから本人に見せるか、spec / D12 / CH21.md から外す（巡った目的は goals.md の項目の番号だけにする等、Claude の自由文を出さない形）。どちらにするかは Q7 の答えに合わせる
- 処置: fixed D12 仮 （巡った目的は目的の一覧の札だけ・所要と終わり方は機械の値だけにし、Claude の自由な文は出さない。Q7 の context と推奨の irreversible に札・所要・終わり方も出ると書き足して本人に見せる。反転条件は D12）

## R2. Q7（loss: exported）の安全の言い分「題に本人のものは入らない」が premises に無く、題を縛る Scenario も無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/premises.md
- 根拠: design.md:152（「題は Claude が書くので、Sandbox の合成の記録以外の固有名は入らない（材料に本人のものが無い）」）。premises.md の表にこの行は無い（確かさ 1 相当: 言っただけ）。spec.md:128-135 は題の長さ・1 行であること・中身の制約を持たず、Scenario は目印の文字列が無いことだけを見る。題には手元のパス（`C:\Users\…`）・網の名前・Sandbox の中で見た外のサイトの文も入りうる（Q10 で網を開ける）。premises.md:18 は「loss を持つ決定は確かさ 5 の前提に依る」と書くが、この 1 行は数えていない
- kind: premise
- 提案: premises.md に行を足す（確かさを付ける）。あわせて spec に「段落の題は 1 行・N 文字まで・`tools/check-private.sh` の型に当たらない（当たれば伏せて手元にだけ残す）」の Requirement と Scenario を足し、機械で縛る
- 処置: escalated （Q7 の context に題の縛り（1 行 60 文字・私的な値の形は伏せる）を書いて本人に見せる。spec に題の Scenario を足し、premises.md に行を足した（確かさ 3、自己検査が通れば 4））

## R3. 本人の決定の数値「45 分」が Scenario に無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/trial-use/spec.md
- 根拠: deep.md:48（Q9 推奨「45 分」）/ spec.md:74（本文に「既定 45 分」）/ spec.md:78-81（唯一の Scenario は上限を 2 秒にして走らせる）。既定値を 30 分や 90 分に変えても、どの Scenario も落ちない
- kind: technical
- 提案: 「上限を指定しないとき、Claude を包む上限は 2700 秒である」（偽の `timeout` か偽の Claude が受け取った上限を書き留める）の Scenario を 1 本足す
- 処置: fixed 6.1 （Scenario「上限を指定しなければ、Claude に許す時間は 2700 秒である」を足した）

## R4. Q9 の選択肢が示す代償「手順書が渡るのが最大 45 分遅れる」が、design の作りと合わない
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md
- 根拠: deep-questions.json の Q9 推奨の detail「手順書が渡るのが最大 45 分遅れる」/ design.md:108（D10: 組み立てに別に 60 分の上限、Sandbox の起動・片付けは 45 分の外）/ design.md:94（D8: 使い終えた後に 2 回目の `claude -p`）/ `.harness2/scripts/verify_batch.sh:182`（`./tools/verify-prep.sh "$tag"` を上限なしで待ち、その後に録画と手順書）。最悪の遅れは 45 分でなく 105 分＋起動・片付け・照合になる。本人は Q9 で 20 / 45 / 90 分を「遅れ」で比べて選ぶ
- kind: daily
- 提案: Q9 の detail の遅れを「組み立てを含めて最大 <合計> 分（初回）、2 回目から最大 <合計> 分」に直してから本人に見せる。あるいは席の全体（組み立て〜片付け）に 1 つの上限を spec に置き、Q9 の数がそれを指すようにする
- 処置: fixed D10 仮 （45 分は Claude の時間だけで、組み立て 60 分・片付け・照合が別にかかると D10 に書き、Q9 の選択肢の detail と context を直した）

## R5. Q12 の推奨が約束する「Sandbox の取り合いは ashiato2 の側の錠で避ける」が、design・spec・tasks のどこにも無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md
- 根拠: deep-questions.json の Q12 推奨の detail「Sandbox の取り合いは ashiato2 の側の錠で避ける」/ `grep -n 錠` を design.md・tasks.md・spec.md に掛けて 0 件 / design.md:114（D11 は「起こす前に動いている Sandbox のプロセスを問う」だけ。問うてから起こすまでの間に、並走する別の worktree の確認バッチの席や録画が起こせば、両方が起こす）
- kind: technical
- 提案: D11 に錠（例: `flock` を手元の置き場の固定の名前に掛け、取れなければ「別の席が走っている」の警告）を書き、spec に「錠が取られていれば、Sandbox を起こさず警告を書く」の Scenario を足す。錠を置かないなら、Q12 の detail から「錠で」を消す
- 処置: fixed D11 （錠 `flock -n` を D11 に書き、spec に「別の席が走っていれば、Sandbox を起こさず警告を書く」を足した）

## R6. Requirement の本文だけが言い、Scenario が言っていないこと
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/trial-use/spec.md
- 根拠:
  - spec.md:88「Sandbox の時計は動かさない」（Q5 の推奨の一部。deep.md:28）—— Scenario 無し
  - spec.md:104 の 4 つの根拠のうち、Scenario が欠かすのは「画面の写し」だけ（spec.md:112）。「直前の操作の列」「期待したことと見えたこと」「もう 1 回したときに起きたか」が欠けた候補を数えても落ちない
  - spec.md:107「Sandbox だから」の注記・待ち時間の候補に「Sandbox の中で測った」—— Scenario 無し（Q6 の推奨 detail と Q5 の context に依る）
  - spec.md:108 の並び（「1 回だけ」でないものが先）—— Scenario 無し。13 件の Scenario（spec.md:120-123）は数だけを見る
  - spec.md:13「Sandbox の実行ファイルが無い」—— Scenario（spec.md:21-24）は PowerShell が無い場合だけ
  - spec.md:86「Claude の作業場所」にも置かない —— Scenario（spec.md:91-94）は材料の置き場だけを並べる
  - spec.md:148 違和感メモの行（ページ・表題・場所・日時・こうなってほしい、場所に報告と写しの場所）—— Scenario（spec.md:157）は見出しと `何が引っかかったか:` だけ
- kind: technical
- 提案: それぞれに 1 本ずつ Scenario を足す（時計は「偽の頼みの箱が受けた頼みに時計を変える命令（`Set-Date` 等）が無い／`box.sh` が断る」で観測できる）。足さないものは Requirement の本文から外す
- 処置: fixed 7.1 （時計・4 つの根拠それぞれ・Sandbox の印と待ち時間・並び・Sandbox の実行ファイルが無い場合・作業場所・メモの全行を Scenario にした。5.1・2.1・6.1・7.1 が名指しする）

## R7. 「目的の一覧」の Scenario が、Requirement と design の例に矛盾し、後半が測れない
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/trial-use/spec.md
- 根拠: spec.md:98（「インストール・合言葉・ログイン・取り込み・見て回り・設定・更新の確認・アンインストール・入れ直し・PC の収集アプリ、の語がそれぞれ 1 回以上ある」）/ spec.md:87（「利用者の言葉で」）/ design.md:74-75（D6 の例「入れて、合言葉を決め、昨日までの自分の行動を見る」「新しい版が出ていないか確かめる」「いったん消して、入れ直す」には、インストール・ログイン・取り込み・見て回り・更新の確認・アンインストールの語が無い）。字面で合わせると目的が仕様の語彙（=画面の部品の言葉）に寄り、Q4 の「筋書きにしない」に反する。
  また spec.md:99「画面の部品の名前（ボタンの文言・URL の道）も無い」は、何と照らすか（文言の一覧の出所）が書かれておらず、真偽が決まらない
- kind: technical
- 提案: 対象ごとに「受け付ける言い方の集合」（例: 入れ直し = 入れ直す|もう一度入れる）を spec か goals.md の注記に固定し、Scenario はその集合に当たることを見る。部品の名前は「`web/src` の画面の文字列の定数と、`/` で始まる道」と照らす、のように出所を書く
- 処置: fixed 5.1 （目的の一覧に対象ごとの札を持たせ、Scenario は札と、番号つきの行・`/` で始まる道・「」の文言が無いことを見る形にした）

## R8. GitHub に書き込まない Scenario が拒否の列挙で、書き込みの抜け道を落とさない
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/trial-use/spec.md
- 根拠: spec.md:147（本文は「GitHub に書き込む命令を打たせない」）/ spec.md:153（Scenario は `issue create`・`issue comment`・`issue edit`・`api` の `-X POST/PATCH/PUT/DELETE` だけを数える。`gh api -f`（暗黙の POST）・`--method`・`issue close`・`issue reopen`・`pr comment`・`label create/edit`・`git push` は通る）/ tasks.md:18（Global Constraints は「`gh` は `issue list` だけを使う」と許す側で書いている）/ tasks.md:106（静的な `grep -rnE "gh (issue (create|…" tools/trial-use/` は、自己検査の本文や README が同じ文字列を書くと自分に当たる）。Q8 は A（loss: exported）
- kind: technical
- 提案: Scenario を許す側に直す:「書き留めた `gh` の命令は、すべて `issue list` で始まる」。tasks 8.1 の静的な grep は外すか、`tools/trial-use/check-trial-use.sh` と `README.md` を除いて掛ける
- 処置: fixed 8.1 （Scenario を許す側「すべて `issue list` で始まる」に直し、静的な grep を外した）

## R9. Q4「渡すものだけ」を実際に縛る Claude の届く範囲が design（D4）にしか無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md
- 根拠: design.md:57-58（D4: リポジトリのパスを渡さない・`--add-dir` しない・`--allowedTools` はブラウザの道具・`box.sh`・候補の置き場への Write だけ）/ spec.md:85-86・91-94（spec は材料の置き場のファイルを並べるだけ。Claude の作業場所がリポジトリの中でも、Read や Bash を許していても落ちない）。偽の Claude は受け取った引数と作業場所を書き留められるので観測できる
- kind: technical
- 提案: spec に「Claude を起こす命令の作業場所はリポジトリの外で、引数にリポジトリのパスが無く、許す道具に Read が無く、Bash は `box.sh` を呼ぶ形だけに限られる」の Scenario を足す（D4 は仮のまま、形の約束だけを spec に）
- 処置: fixed 6.1 （Scenario「Claude はリポジトリの外で、限った道具だけで起こされる」を足した）

## R10. 1 つの Scenario に主張が 2 つ束ねられている
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/trial-use/spec.md
- 根拠: spec.md:115-118（「1 回だけ」の印 と 既知を数えない、の 2 つ）/ spec.md:169-171（出所が変わったら走る と 前の回が警告なら走る、の 2 つ。WHEN も 2 つの置き場を持つ）。片方だけ通っても、印は 1 つなので緑になる
- kind: technical
- 提案: それぞれ 2 本に分ける（印は tasks の名指しも合わせて直す）
- 処置: fixed 7.1 （「1 回だけ」と既知、出所が変わる と 前の回が走れていない、をそれぞれ 2 本に分けた。既知は 8.1、出所は 9.1）

## R11. 「席が落ちても、準備は rc=0」の Scenario の印が、準備（verify-prep.sh）を走らせない試験に付く
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md
- 根拠: spec.md:17-18（WHEN「`tools/verify-prep.sh` の席の段を走らせる」・THEN「準備は rc=0」）/ tasks.md:49-50（2.1 は「一時の木で `seat.sh` を呼び…`verify-prep.sh` 全体の rc は 2.2 で見る」）/ tasks.md:54（2.2 は 1 回きりの検証コマンドで、印も CI も無い）。印の付いた試験は準備の rc を一度も見ないので、`verify-prep.sh` の 1 行が `set -e` で落ちる形になっても緑のまま
- kind: technical
- 提案: Scenario の WHEN / THEN を「入口（seat.sh）が非 0 の本体のもとで rc=0 を返し、既存の段落を変えない」に直し、準備の rc は「準備の台本は席の入口の失敗を `|| …` で受ける」（台本を読む Scenario、spec.md:26-28 に足す）で縛る。あるいは自己検査で verify-prep.sh の前段を差し替えて本当に走らせる
- 処置: fixed 2.1 （入口の Scenario を「入口が rc=0 で既存の段落を変えない」に直し、準備の rc は台本の `||` を読む Scenario で縛った）

## R12. design に観測できる振る舞いが置かれていて、archive で正典から落ちる
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md
- 根拠:
  - D4（design.md:59）: Claude の利用上限・認証切れ・起動できない は「Claude が動けなかった」の警告、再試行しない
  - D8（design.md:95）: `gh` で一覧を読めなければ照合を飛ばし、報告にその旨を書く
  - D10（design.md:108）: 組み立てに 60 分の上限、越えたら「組めなかった」
  - D11（design.md:115-116）: 閉じた後に残れば警告に書く・取り出せなかったログは報告に「取り出せなかった」
  - D12（design.md:123-124）: 段落の形（所要・終わり方・「手順書には上位 <M> 件」）
  - Risks（design.md:151）: インストーラが Sandbox で通らなければ、候補でなく警告として段落に書き、後の目的は巡らない（Q9 の context が挙げる落ち方の族の 1 つ。deep-review R7 の処置は「design に残す」）
  - Q9 の族のうち spec に Scenario があるのは「Sandbox が既に動いている」「組めない」「時間切れ」だけで、「インストーラが通らない」「Claude の利用上限」は design にだけある
- kind: technical
- 提案: 上の各項を spec の Requirement と Scenario に移す（偽の Claude が非 0 で候補 0 件 → 段落が「Claude が動けなかった」を含み、Claude を 2 度起こさない、など）。D には「どう作るか」だけを残す
- 処置: fixed D4 （Claude が動けない・照合を飛ばす・組み立ての上限・閉じても残る・取り出せなかったログ・インストールまで届かない、を spec の Requirement と Scenario に移し、D には作り方だけを残した）

## R13. D9 の「出所」の範囲が、組み立てに入るファイルを欠き、spec の定義に反する
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md
- 根拠: spec.md:161（出所 =「配る 2 つの zip の組み立てに入るファイルの中身」）/ design.md:100-101・tasks.md:110（比べるのは `crates` `web` `migrations` `Cargo.toml` `Cargo.lock` `tools/release` `tools/trial-use` だけ）/ `tools/release/build.sh:111`（`cp "$root/tools/db-roles.sql" "$stage/"`）・`build.sh:48`（`git show HEAD:LICENSE`）・根の `.cargo/`（ビルドの設定）。`.github/workflows/ci.yml` の windows の判定も `^tools/db-roles\.` を持つ。これらだけが変わった確認バッチでは、席は走らず前の回の結果を写す
- kind: technical
- 提案: D9 と tasks 9.1 の一覧に `tools/db-roles.sql` `LICENSE` `.cargo`（`.cargo/config.toml` は追跡されている）を足す。ci.yml の windows の判定（`LICENSE` も `.cargo` も持たない）は正本にできないので、一覧を `tools/trial-use/` の 1 か所に置き、`build.sh` / `package.sh` が読むパスと突き合わせる自己検査の Scenario を足すと漏れが落ちる
- 処置: fixed D9 （`.cargo` `LICENSE` `tools/db-roles.sql` を足し、一覧を `sources.txt` の 1 か所に置いて build.sh・package.sh の読むパスと突き合わせる検査を 9.1 に置いた）

## R14. spec が組み立ての台本の名前（`tools/release/package.sh windows`）に縛られ、D2 の反転で spec が偽になる
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/trial-use/spec.md
- 根拠: spec.md:37（「配る形の組み立て（`tools/release/package.sh windows`）で…組ませ」）/ spec.md:43（「配る形の組み立てを、呼ばれた木の commit を書き留め…る偽のものに替えて」）/ design.md:39（D2（仮）の反転条件: 手元で組めなければ CI の artifact を落とす案に倒す）。倒すと、手元の組み立ては呼ばれず、Requirement も Scenario も書き直しになるが、D2 はそれを書いていない（D3 は「仕様は変わらない」と明記している）
- kind: technical
- 提案: spec は「Sandbox に渡す 2 つの zip は、準備が走っている木の HEAD から組まれたもの（zip の中の組み立ての印の commit が HEAD と一致する）」と結果で言い、台本名を外す。外さないなら D2 の反転条件に「倒すときは spec の Requirement を MODIFIED で直す」を書く
- 処置: fixed D2 （spec から組み立ての台本の名前を外し、HEAD から組んだ 2 つの zip であることだけを約束する形にした）

## R15. tasks の順が、Scenario の依る仕組みより先に Scenario を置いている
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md
- 根拠: tasks.md:70（4.1 の「席が起こした Sandbox は、どう終わっても閉じる」は Claude が非 0・時間の上限を越える、の 3 通りを要るが、Claude の段と上限は 6.1）/ tasks.md:87（6.1 の「時間の上限を越えると、それまでの候補で報告を作って閉じる」は段落の件数 = 1 を見るが、候補を数えて段落を作る `report.py` は 7.1）/ tasks.md:95（7.1 の「既知の候補は数えない」は `known` を書き足す照合が要るが、それは 8.1）
- kind: technical
- 提案: 4.1 の閉じる Scenario を 6.1 の後へ、6.1 の上限の Scenario を 7.1 の後へ移すか、7.1 を 6.1 より前にする。既知の Scenario は 8.1 に移す（または 7.1 で `known` を持つ偽の候補を直に置く形と明記する）。累計の `-ge` の数も合わせて直す
- 処置: fixed 6.1 （順を組み直した: 報告の下地 5.2 → Claude と閉じ方 6.1 → 報告を席を通して確かめる 7.1 → 既知 8.1。累計の数も直した）

## R16. Task 1.1 と 11.1 は本物の手元の Windows が要るのに、そう書かれておらず、作業箱では必ず確かめられない
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md
- 根拠: tasks.md:31-36（`probe.sh` は Windows が無ければ rc=1、検証は 3 つとも rc=0）/ tasks.md:126（11.1 は `seat.sh` と `/mnt/c/...`）/ `tools/workbox.json` の `host_only` は `powershell\.exe`・`tools/release/sandbox-probe\.sh` などを持つが `tools/trial-use/` を持たない（検証コマンドの字面に `powershell.exe` は出ない）ので、作業箱が有効なら箱の中で走り UNAVAILABLE → rc=1 で通らない。前例の `tools/release/sandbox-probe.sh` は ST55 で UNAVAILABLE のまま CI に委ねた。
  また `install` は `build` の zip に依るが、`build` が FAIL / UNAVAILABLE のときの 1.1 の扱いが無い。1.2 の検証（tasks.md:40）は 3 行とも確かさ 5 を求め、UNAVAILABLE の行の行き先が無い
- kind: technical
- 提案: 1.1 と 11.1 に「手元の Windows と Claude の利用枠が要る（作業箱の外）」と書き、`tools/workbox.json` の `host_only` に `tools/trial-use/(probe|seat)\.sh` を足すタスクを 1.1 に含める。UNAVAILABLE のときの扱い（BLOCKED_INFRA として止まる、など）と `install` が `build` に依ることを書く
- 処置: fixed 1.1 （1.1 と 11.1 に手元の Windows が要ると書き、1.1 で `tools/workbox.json` の `host_only` に `tools/trial-use/(probe|seat)\.sh` を足す。UNAVAILABLE は rc=1 で BLOCKED_INFRA）

## R17. Task 2.2 の検証が確認バッチの準備を丸ごと走らせる
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md
- 根拠: tasks.md:54（`TRIAL_USE_BODY=false ./tools/verify-prep.sh ch21probe`）/ `tools/verify-prep.sh:36-70`・`99-126`（`env-init.sh`、release の `cargo build`、`npm ci`、gradle の APK とエミュレータへの install、前の回の DB の容器を起こして数える）。数分〜十数分かかり、AGENTS.md の「長いコマンドは `scripts/quiet-run`」に当たる。動いているエミュレータに APK を入れる副作用もある。席の振る舞いを見るのに要るのは manifest の heredoc の後の 1 行だけ
- kind: technical
- 提案: 検証を `scripts/quiet-run` で包むか、自己検査の中で verify-prep.sh の重い段を差し替えられる形（既存の環境変数が無ければ足さず、一時の木で manifest までを置いて席の行だけを走らせる）にする。R11 と合わせて 1 か所で準備の rc を見る
- 処置: fixed 2.1 （準備を丸ごと走らせる 2.2 をやめ、台本を読む Scenario と入口の自己検査で準備の rc を縛った）

## R18. 11.1 の手元の 1 回が `last.json` を書き、確認バッチの実地の 1 回が走らずに写しで済む
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md
- 根拠: tasks.md:111（走ったら `$TRIAL_USE_HOME/last.json` を書き直す）/ tasks.md:126（11.1 は既定の `TRIAL_USE_HOME` = `/mnt/c/dev/ashiato2-trial-use` で走る）/ docs/changes/CH21.md:62（完了の判定「確認バッチで 1 回実地に走り」）。確認バッチの木の出所が 11.1 の木と同じ（CH21 だけを束ねたバッチなど）なら、席は Sandbox を起こさず「前の回（probe-…）の結果」を写し、完了の判定の「実地に走り」が満たされないまま手順書に段落が載る。
  また 11.1 の検証は `report.html` の有無だけを見て、CH21.md:60 が求める「画面の写し」を見ない
- kind: technical
- 提案: 11.1 は `TRIAL_USE_HOME` を probe 専用の場所にするか、probe の tag は `last.json` を書かない規則を D9 と spec に書く。11.1 の検証に `shots/` が 1 枚以上あることを足す
- 処置: fixed 11.1 （`probe-` の tag は前の回の記録を読みも書きもしない規則を spec と D9 に足し、11.1 に写しとインストールまで届いたことの確かめを足した）

## R19. CH21 の「壊してはいけないもの」のうち、変えないパスを確かめるコマンドがどの task にも無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md
- 根拠: docs/changes/CH21.md:53・55（harness2・製品のコード・移行・画面・収集アプリ・`tools/release/`）/ tasks.md:17・22（Global Constraints に文で書くだけ）。どの Scenario もこれを見ず、検証コマンドも無い
- kind: technical
- 提案: Task 10.1 の検証に `bash -c '! git diff --name-only origin/main...HEAD | grep -E "^(crates|migrations|web|collector-android|tools/release|tools/recording|\.harness2)/|^tools/(record-env|stack)\.sh$"'` rc=0 を足す（`.harness2` は symlink なので、harness2 側の変更は別に見る必要があることも書く）
- 処置: fixed 10.1 （変えないパスが差分に無いことの検証を足した）

## R20. 自己検査を走らせる CI の job が、design と tasks / CH21 で食い違い、md だけの変更では走らない
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md
- 根拠: design.md:135（D14「CI の `chain` job で走る」）/ tasks.md:117・docs/changes/CH21.md:59（rust job）/ `.github/workflows/ci.yml:94-96`（rust job は `needs.changes.outputs.code == 'true'` のときだけ）/ `tools/release/changes.sh:7`（`\.md$` だけの変更は code=false）。`tools/trial-use/goals.md`・`prompt.md` だけを変えた PR では、goals.md を見る Scenario（spec.md:96-99）の自己検査が CI で走らない
- kind: technical
- 提案: D14 と tasks / CH21 を 1 つの job にそろえる。md だけでも走らせたいなら、出し分けの無い `chain` job に置く（0.2 分の job。自己検査は Windows も Claude も要らない）
- 処置: fixed 10.1 （CI は出し分けの無い `chain` job にそろえた。D14・CH21.md も同じ）

## R21. 完了の判定の 2 つめが、確かさ 1 の前提（Sandbox でインストーラが通る・外から Edge を操れる）が外れると満たせず、そのときの報告の形が spec に無い
- 成果物: docs/changes/CH21.md
- 根拠: docs/changes/CH21.md:60（「手元の置き場に報告の HTML と画面の写しが残り」）/ premises.md:13-14（確かさ 1）/ tasks.md:38-39（`install` が FAIL なら「席はインストールが通らなかったことを警告に書いて閉じる」）/ spec.md:127-129（報告の HTML を作るのは候補を扱う流れだけで、警告で終わったときに報告と写しを残すかは書かれていない）/ spec.md:33（人間の確認待ちも「（走ったとき）」）。loss のある決定は乗っていないが、完了の判定がこの前提に乗っている
- kind: premise
- 提案: spec に「警告で終わった回も、そこまでの写しとログを手元の置き場に残し、報告の HTML を作る」を足すか、CH21.md の 2 つめを「報告の HTML と写し、または警告の理由と取り出せたログ」にする。Task 1 の結果が出たら、どちらで閉じるかを確かめる。前者にするなら、11.1 の検証（`ls report.html`）は警告で終わった回でも通るので、11.1 に「Sandbox の中でインストールまで届いた」ことを見る条件を別に足す
- 処置: escalated （Q13 の context に、インストーラが通らない・外から操れないときはその回が警告と報告で終わることを書いて本人に見せる。spec に「警告で終わった回も、そこまでの写しで報告を作る」を足し、CH21.md の完了の判定の 2 つめに、Task 1 で外れたら spec の段に戻すと書いた）

## R22. CH21.md の「壊してはいけないもの」が、答え待ちの A（Q7・Q8）の推奨を不変の約束として書いている
- 成果物: docs/changes/CH21.md
- 根拠: docs/changes/CH21.md:54（「公開の repo に出るもの … issue は作らない」）/ deep.md:38・43（Q7・Q8 は答え待ちの A）/ 依頼（CH21.md:36「残すべきものは『違和感』の issue にする」）。本人が Q8 で第 2 案（別ラベルで自動）を選ぶと、この行は依頼にも答えにも反する。`spec --answers` の確かめ（観点 6）は specs / design / tasks を見るので、CH21.md の行が推奨のまま残っても拾われにくい
- kind: technical
- 提案: CH21.md:54 の行に「（Q7・Q8 の推奨。答えで書き直す）」の印を付けるか、`spec --answers` の段で CH21.md も写し直す対象として呼び出し元に伝える
- 処置: fixed D12 （CH21.md の該当の行に「Q7・Q8 の推奨で書いた。答えが違えば spec --answers が書き直す」の印を付けた）

## R23. `review/deep.md` の処置の形が review_triage を通らない（17 件）
- 成果物: openspec/changes/ch21-trial-use-before-verify/review/deep.md
- 根拠: `python3 scripts/review_triage.py . ch21-trial-use-before-verify` rc=1。R1・R2 は kind が irreversible / premise で loss=exported なのに `fixed`（答え待ちの A は escalated か、失わない側の仮なら provisional.jsonl）。R4（daily）と R10（conflict）は `fixed D<n> 仮` でしか閉じられない。全 11 件の `fixed` の指す先が `Q7` `premises.md（…` などで、D<n> / <x.y> / change 直下のファイル名の形でない
- kind: technical
- 提案: 処置を `review_triage.py` の冒頭（2〜41 行）の形に直す: R1・R2 は `escalated`（deep.md に R の id を書く）、R4・R10 は対応する D に（仮）と反転条件を書いて `fixed D<n> 仮` か `escalated`、他は `fixed <change 直下のファイル名>`（`deep-questions.json` `premises.md` など、実在するもの）
- 処置: fixed deep.md （review/deep.md の処置を review_triage の形に直した。A の R1・R2・R10 は escalated とし、deep.md の末尾に問いへ回したことを書いた）

## R24. D2 の「`git archive HEAD` で写して `package.sh windows`」は、`.git` の無い写しで組み立てが落ちる
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md
- 根拠: design.md:36-37（D2: `git archive HEAD` で `C:\dev\ashiato2-trial-build\src\` に写し、そこで `tools/release/package.sh windows`）/ `tools/release/build.sh:48`（`git -C "$repo" show HEAD:LICENSE … || die 'Git の LICENSE blob を読めない'`）・`build.sh:56`（`copy_canonical_license "$root" …` を server-windows / collector-windows でも呼ぶ）。`git archive` の写しには `.git` が無いので、ここで必ず die する。D2 が「同じ型」と呼ぶ録画の `C:\dev\ashiato2-rec-build`（`tools/record-env.sh:196-201`）は rsync の写しで `cargo build` だけを走らせ、`package.sh` は通していない。
  このままだと Task 1.1 の `probe.sh build` は設計の理由で FAIL し、1.2 が D2 を CI の artifact の案へ倒す（前提の確かめではなく作り方の誤りで倒れる）。spec.md:43-44 の「呼ばれた木の commit を書き留め」も、commit を知る木を前提にしている
- kind: technical
- 提案: D2 の写し方を `git clone --local --no-hardlinks`（`build.sh:343` の自己検査と同じ型）か `git worktree` にする。写し方を変えないなら、build.sh が `.git` の無い木でも組める根拠を D2 に書く
- 処置: fixed D2 （写し方を `git clone --local --no-hardlinks` にした。1.1 も同じ）

## R25. 材料の置き場（`C:\trial\in`）と頼みの箱の場所が、spec・design・tasks の間で食い違う
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md
- 根拠: spec.md:93（材料の置き場にあるのは目的の一覧・2 つの zip・合成の書き出し「だけ」）/ design.md:73・tasks.md:77（`in/` に置くのは同じ 3 種だけ）/ design.md:113・tasks.md:66（`C:\trial\in`（読みだけ）に `runner.ps1` も置く）。`runner.ps1` を `in` に置けば spec.md:91-94 の Scenario が落ち、置かなければ LogonCommand の置き場が無い。
  頼みの箱も design.md:46 は `C:\trial\box\`、design.md:113・tasks.md:68 は `C:\trial\out\box\`
- kind: technical
- 提案: `runner.ps1` を材料と別の読みだけの置き場（例: `C:\trial\bin`）に分けるか、spec の「材料の置き場」の定義を「Claude に渡す置き場」として `runner.ps1` を含まない場所に限る。頼みの箱は 1 つの場所にそろえる

---

観点 6: 該当なし（呼び出しは `spec --draft`。名指しの Q は無い）

観点 7: 該当なし（spec_overlap.py candidates が 0 件）
- 処置: fixed D11 （`runner.ps1` は別の読みだけの置き場 `C:\trial\bin` に置き、頼みの箱は `C:\trial\out\box\` の 1 つにそろえた）

