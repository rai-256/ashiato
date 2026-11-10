# CH14 spec レビュー（spec --draft。第 1 回の推奨で書いた版）

対象: `openspec/changes/ch14-installer-password-prompt-mojibake/`（proposal・specs/distribution/spec.md・design・tasks・premises・deep.md・review/deep.md）と `docs/changes/CH14.md`。
照らしたもの: 正典 `openspec/specs/distribution/spec.md:1773-1807`、`crates/installer/src/{main.rs,password.rs,lib.rs}`・`Cargo.toml`・`Cargo.lock`、rpassword 7.5.4 の source、`tools/release/install-check.sh`、`.github/workflows/ci.yml`、走っている ch11・ch12・ch13（`origin/main` 95cba8c4d と `origin/feat/ch1[1-3]-*`）。

## 機械の検査

```
openspec validate ch14-installer-password-prompt-mojibake --strict   → valid（rc=0）
python3 scripts/check_chain.py .                                     → chain: OK（rc=0。観点 8 の再生成は change に無い）
python3 scripts/check_scenarios.py . ch14-…                          → FAIL（rc=1）: 担保の無い Scenario 5 件 =
    この change が足した 5 本（合言葉の条件は、最初の問いの前に示す／短い・雛形で断るときは、理由に条件を添える／
    コード頁が UTF-8 でない窓でも…／窓で打った合言葉は、窓に出ない／標準出力と標準エラーを向け替えても…）。
    印は Task 4 が置く。上流の段では想定どおり（ほかの FAIL は無い）
python3 scripts/review_triage.py . ch14-…                            → triage: OK（指摘 8 件・仮決め 0・要件へ戻すもの 0）
python3 scripts/spec_overlap.py candidates . ch14-…                  → []（0 件）
```

第 2 回（0d2eedfcf を読んで。2026-11-09）: `openspec validate --strict` valid / `check_scenarios.py` は FAIL 6 件で、どれも足した 6 本（印は Task 4 が置く。想定どおり）/ `review_triage.py` は「R1: kind=premise は fixed にできない」の 1 件だけ FAIL（R1 の kind を technical にしたので解ける）/ `spec_overlap.py candidates` は `[]`。R2〜R9 の処置は成果物で確かめ、指摘を満たしている（R2: design D6 `:76-77`・tasks 3.2 `:49` が `CONSOLE-OK` と化けた並びの**存在**を見る。R3: `-ExecutionPolicy Bypass`。R4: 窓の Scenario の THEN に順・管の Scenario を改名。R5: 管の Scenario の WHEN が 3 つの口・tasks 4.1 が 3 つの出力。R6: SHALL 2 行を足し、(c) は約束しないと design D4 `:61`。R7: `D7（仮）`。R8: 語を名指し・ログインを別の Scenario に分けた・backtick を外した。R9: `capability: [distribution]`）。新しく見つけたのは R10。R9 の処置の行が観点の節の後ろ（ファイルの末尾）に付いていたので、字面を変えずに R9 の節の中へ移した

正典との突き合わせ: `diff <(sed -n 1773,1807p openspec/specs/distribution/spec.md) <(sed -n 3,41p specs/distribution/spec.md)` の差は、SHALL 3 行・導出元の 1 句・理由の 1 行の**追加だけ**。既存の SHALL 5 行と Scenario 5 本は逐語で残っている（Scenario は落ちていない）。

## R1. ConPTY が runner で使えるかは「spec の段で確かめる」と書いたまま未確認で、D6 の反転条件は今の Task では起きない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/premises.md・design.md
- 根拠: `review/deep.md:71`（R6 の処置「ConPTY が runner で使えるかは spec の段で確かめる」）/ `premises.md:16`（確かめていない・確かさ 1 のまま。しかもこの行は `:15` の段落の後にあり、表の外に落ちている）/ `design.md:76`（反転条件「ConPTY が windows-latest の runner の step から使えない…と **Task 3 の確かめで**分かれば A として人間に返す」）/ `tasks.md:49`（Task 3.2 の確かめは「WSL から本人の PC の PowerShell で」—— runner では走らない）。runner で初めて走るのは PR の CI の `install-windows`（`ci.yml:453`）で、Task 1〜4 を全部書いた後。下流から spec へ戻る辺も無い。
  あわせて、CI はインストールの確かめを `| tee` で流す（`ci.yml:453`）ので、台本の PowerShell の標準の handle は管になっている。子が疑似コンソールにつながるか（`STARTUPINFOEX` の std handle をどう渡すか）は design にも tasks にも無く、つながらなければ子の `is_terminal()` が偽で管の道を通る。これも runner の上でしか分からない
- kind: technical
- kind の判定し直し（第 2 回。0d2eedfcf を読んで）: premise から technical に変えた。理由: (1) 前提のうち本人の PC の分（Windows PowerShell 5.1 の `Add-Type` の C# で `CreatePseudoConsole`、親の出力が `| tee` の管、`STARTF_USESTDHANDLES` を立てて std handle を空にすれば子の `IsInput/OutputRedirected` が偽、cp932 の窓で UTF-8 のバイトが本人の見た『逕ｻ髱｢縺ｮ蜷郁ｨ闡』になる）は実物で確かめられ、`premises.md` の表の 9 行目（確かさ 5）にある。(2) 残る runner の分（表の 8 行目、確かさ 2）が乗る決定は D6（仮）だけで、失うもの（`loss`）が無い（試験の置き場で、外れたら置き場を替えれば戻る）。確かさ 4 未満の前提が人間に返るのは失うもののある決定に乗るときなので、これは人間に聞く前提ではない。(3) 指摘の残り（反転条件にたどり着けるか・std handle の形）は技術の問題で、どちらも直っている: tasks 3.3 が `install-windows` のインストールの前に `--probe-console` の step を足すので、runner で使えないことは PR の CI の最初の step で、`CreatePseudoConsole` の失敗か `CONSOLE-OK` が無いことで見分けて落ちる（design D6 `:81` の反転条件の入口と同じ印）。std handle の形は design D6 `:73`・tasks 3.1 `:44` に「立てて空にする」と書いてある。なお、処置の行の「`STARTF_USESTDHANDLES` を立てない・継がない」は成果物（design `:73`・tasks `:44`・premises の表の 9 行目の実測）と逆の字面。成果物の側が実測どおりで正しく、下流は成果物を読む
- 提案: GO の前に、本人の PC と（できれば）runner で `CreatePseudoConsole` → 子が窓につながる（子の `[Console]::IsInputRedirected` が偽）ことを 1 回確かめて premises の確かさを上げるか、確かさ 1 のまま進むと明記する。反転条件は「PR の CI の install-windows で probe が落ちたら」に書き直し、そのときの行き先（窓の Scenario 3 本を外して別の change に出す、など）を到達できる形で書く。premises の行は表の中へ戻す。
- 処置: fixed D6 仮 — 失うものが無い（試験の置き場で、後から替えれば戻る）ので確かさ 1 のまま進むと D6 と premises に明記し、premises の行を表の中へ戻した。runner で先に確かめる step（`install-check.sh --probe-console`）を `install-windows` のインストールの前に足し（tasks 3.3）、反転条件を「PR の CI のその step が落ち、原因が runner で ConPTY が使えないことなら A として人間へ」に書き直した。子の std handle を疑似コンソールにつなぐ形（`STARTF_USESTDHANDLES` を立てて 3 つの handle を空にし、継がない。※最初に「立てない」と書いたが、本人の PC の確かめで逆と分かり成果物は「立てる」にした —— premises.md 9 行目）と、子が窓につながった印 `CONSOLE-OK` を D6・tasks 3.1 に足した

## R2. 陰性の対照は「化けた窓に問いの文字が**無い**」だけを見るので、書きが窓へ届かなくても通る
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/design.md・tasks.md
- 根拠: `design.md:74`・`tasks.md:47`（`console_probe` は `.raw.txt` に『画面の合言葉』が**無く**、`.console.txt` に**ある**ことを確かめる）。`.raw.txt` 側の子が何も窓に出さなかった場合（`[IO.File]::OpenWrite('CONOUT$')` が窓でなく別の物を開いた・失敗した・子が窓につながらなかった）も「無い」は真になり、試験は化けを見分けたことにして本物の Scenario へ進む。Windows PowerShell 5.1（.NET Framework）の `FileStream` が `CONOUT$` を窓として開くかは premises に無い（rpassword は `CreateFileW` で開く。`rpassword-7.5.4/src/windows.rs:34-53`）
- kind: technical
- 提案: `.raw.txt` に**化けた並びがある**ことを確かめる形にする（期待値は bash 側で `'画面の合言葉'.encode('utf-8').decode('cp<chcp の番号>', errors='replace')` から作る。本人が見た『逕ｻ髱｢縺ｮ蜷郁ｨ闡』と同じ）。子の書き方は、`OpenWrite('CONOUT$')` で窓が開けなければ `[Console]::OpenStandardOutput().Write(<UTF-8 のバイト>)`（WriteFile の経路）に倒す、と design に 1 行足す。
- 処置: fixed D6 — 陰性の対照を「`.raw.txt` に化けた並び（chcp のコード頁で UTF-8 のバイトを読んだ文字列。bash 側で python から作る）がある」「両方に `CONSOLE-OK` がある」まで見る形にし、`CONOUT$` を開けないときは `OpenStandardOutput` へ同じバイトを書く倒し方を足した（tasks 3.1・3.2）

## R3. Task 3.2 の検証コマンドは、本人の PC の既定の実行ポリシーでは書いたまま走らない見込み
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/tasks.md
- 根拠: `tasks.md:48`（WSL では `powershell.exe` に台本のパスを `wslpath -w` で渡す —— 作業木は WSL の中なので `\\wsl.localhost\…` の UNC になる）。`.ps1` を `-File` で走らせる口に実行ポリシーの指定が無い。repo の前例は `tools/release/sandbox-probe.sh:17`（`powershell.exe -NoProfile -ExecutionPolicy Bypass -File …`）。Windows のクライアントの既定（Restricted）や RemoteSigned で UNC の未署名の台本を走らせると、台本は読まれずに失敗する
- kind: technical
- 提案: Task 3.2（と 4.2 の呼び出し）に `powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File <台本>` の形を名指す。
- 処置: fixed 3.2 — `powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File <台本>` を tasks 3.2 と design D6 に名指した

## R4. 「最初の問いの前に」示すことを、問いに対しては一度も測らない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/specs/distribution/spec.md
- 根拠: `spec.md:10`（SHALL「最初の問いの前に…条件を示す」）。Scenario「合言葉の条件は、最初の問いの前に示す」（`spec.md:43-46`）の WHEN は管で、管では問いの文を出さない（`design.md:57`・`tasks.md:16`）ので、THEN が見る順は「断り・決め直しの結果より前」。窓の Scenario（`spec.md:53-56`、`tasks.md:59`）は `画面の合言葉: ` と `16 文字以上` が**ある**ことだけを見て、順を見ない。窓で条件を問いの後に出す実装（本人が困った場面そのもの）でも全 Scenario が緑になる
- kind: technical
- 提案: 窓の Scenario の THEN に「条件の一覧は、最初の『画面の合言葉: 』より前に出る」を入れ、Task 4.2 で VT を除いた窓の文字の中の位置を比べる。管の Scenario は名前を中身に合わせる（例:「端末でないときも、断りより前に合言葉の条件を示す」）。
- 処置: fixed specs/distribution/spec.md — 窓の Scenario の THEN に「条件の行は最初の『画面の合言葉: 』より前にある」を入れ（tasks 4.2 で位置を比べる）、管の Scenario を「端末でないときも、断りより前に合言葉の条件を 1 回だけ示す」に改名した

## R5. SHALL は 3 つの口（インストールの最後・password・repair）を名指すが、Scenario は全部 `password` の口だけ
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/specs/distribution/spec.md・tasks.md
- 根拠: `spec.md:10`（3 つの口のどれも）/ `docs/changes/CH14.md:31`（壊してはいけないもの 5「3 つの口が同じ問い方・同じ検査」）/ 足した 5 本の WHEN はどれも `password` の口（`spec.md:45`・`:55`・`:60`・`:65`。`:50` は口を書いていない）。3 つの口が同じになる根拠は `fn read_web_passwords` が 1 つあること（`tasks.md:39` の `grep -c … | grep -qx 1`）だけで、install・repair がそれを呼ぶことは確かめない。一方、管で流した install の最後の出力（`install-check.sh` の `$WORK/install.out`、`:1003` が読む）と repair の出力（`install_with "$PW1" repair >"$out"`、`:1072` 付近・`:1277`）は既に CI で取っている
- kind: technical
- 提案: Task 4.1 で、install の最後と repair の出力にも条件の行が最初の `受け付けない:`／決定より前にあることを grep する（新しい Scenario でなく、「合言葉の条件は、最初の問いの前に示す」の確かめを 3 つの出力に当てる）。Scenario の WHEN を「install の最後・password・repair のどれで開いても」に直す。
- 処置: fixed 4.1 — 管の Scenario の WHEN を 3 つの口にし、install の最後（`install.out`）・password・repair の 3 つの出力に同じ確かめ（関数 `assert_requirements_line`）を当てる形にした

## R6. 観測できる振る舞いが design にだけあり、archive で正典から落ちる
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/design.md
- 根拠: (a) `design.md:50`「一覧は断るたびには出し直さない」—— Q3 の案 3 を採らない決定（deep.md Q3）で、出す回数は出力で数えられる。spec は「最初の問いの前に示す」だけで、断るたびに出し直す実装も通る。(b) `design.md:48`「端末でないときは『入れた文字は表示されません』を付けない」—— Task 1.1 の単体試験 `ch14_requirements_line_mentions_hidden_only_on_terminal`（`tasks.md:29`）が確かめるのに、spec に無い。(c) `design.md:61`「窓を開けない・`WriteConsoleW` が失敗したら標準エラーへ書いて続ける」—— 問いが出ないまま止まるか否かの振る舞いで、Scenario が無い
- kind: technical
- 提案: (a) は Q3 の答えそのものなので SHALL と Scenario に写す（「条件の一覧は 1 回だけ示し、断るたびには示さない」。管の出力で条件の行が 1 つ）。(b)(c) は SHALL に 1 行ずつ写すか、振る舞いとして約束しない（単体試験からも外す）かを決める。
- 処置: fixed specs/distribution/spec.md — (a) 「1 回だけ示し、断るたびには示し直さない」を SHALL と管の Scenario（条件の行がちょうど 1 つ）に写した。(b) 「端末のときは表示されないことを添える」を SHALL に写し、窓の Scenario（ある）と管の Scenario（無い）で確かめる。(c) 窓を開けないときの倒し方は約束しないと design D4 に書いた（標準入力が窓なのに窓を開けない場面を CI で作れない）

## R7. D7 は Q4 の推奨に依るのに（仮）の印が無い
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/design.md
- 根拠: `design.md:78`（`### D7. 確認バッチで本人が日本語の窓で 1 回見る（Q4）`）。同じ Q4 に依る D6 は `（仮）`（`:70`）。deep.md Q4 は答え待ちで、推奨は「Windows の CI ＋ 確認バッチで 1 回見る」。本人が「CI だけ」を選べば D7・`tasks.md:80` の 7.1・`CH14.md:42` の確認バッチの行が落ちるが、`（仮）` が無いと `spec --answers` の反転の対象として見えず、合意の引き継ぎ（仮でない D を含む変更は聞き直し）の判定も変わる
- kind: technical
- 提案: 見出しを `D7（仮）.` にし、反転条件（Q4 で「CI だけ」なら 7.1 と CH14.md の確認バッチの行を外す。「確認バッチだけ」なら D6 と窓の Scenario 3 本を外す）を書く。
- 処置: fixed D7 仮 — 見出しを `D7（仮）.` にし、反転条件（CI だけ → 7.1 と完了の判定の行を外す / 確認バッチだけ → D6・Task 3・4.2 と窓の Scenario を外す）を書いた

## R8. 足した Scenario の言葉が、測り方を決めきっていない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/specs/distribution/spec.md
- 根拠: (a) `spec.md:56` THEN「問いと条件の一覧が**そのまま読め**、決め直した合言葉で画面にログインできる」—— 「そのまま読め」と「条件の一覧」の中身が書かれていない（管の Scenario `:46` は 4 つの条件を名指すが、窓の Task 4.2 は `16 文字以上` だけを探す。`tasks.md:59`）。さらに「読める」と「ログインできる」の 2 つの主張を束ねている（ログインは `:18-21`・正典の password の Scenario が別に持つ）。(b) `spec.md:11`・`:51` の字面「雛形（`change-me` で始めない）」は、引用の中に backtick がある。design（`:49`）と tasks（`:55`）は backtick の無い `雛形（change-me で始めない）` を探すので、spec を逐語で読むと別の文字列になる
- kind: technical
- 提案: (a) THEN を「VT の制御の並びを除いた窓の文字に、『画面の合言葉: 』『画面の合言葉（もう一度）: 』と、16 文字以上・change-me で始めない・API の合言葉・ログインの印の鍵の語がある」と書き、ログインは「窓で入れた合言葉が読めている」ことの別の Scenario に分けるか、THEN から外す。(b) 「」の中は backtick を外して実際の字面だけにする。
- 処置: fixed specs/distribution/spec.md — (a) THEN に条件の 5 つの語を名指し、ログインは新しい Scenario「窓で打った合言葉で決め直せる」に分けた（tasks 4.2）。(b) 「」の中の backtick を外した

## R9. `docs/changes/CH14.md` の `capability: []` が、specs の `distribution` の MODIFIED と合わない
- 成果物: docs/changes/CH14.md
- 根拠: `docs/changes/CH14.md:6`（`capability: []`）/ `proposal.md:28`（Modified Capabilities: `distribution`）/ `specs/distribution/spec.md`。正典の delta を持った change の前例 `docs/changes/CH01.md`・`CH08.md` は capability を書いている（CH10 は空で揃っていない）。`scripts/board.py:13` は change の capability を front matter から読む（着手は止めないが、何を触るかの表示と照合の入力）
- kind: technical
- 提案: `capability: [distribution]` にする。
- 処置: fixed proposal.md — proposal の Modified Capabilities（`distribution`）に合わせ、`docs/changes/CH14.md` の front matter を `capability: [distribution]` にした

## R10. D1 の反転条件が、改名前の Scenario の名前と古い SHALL の数を指している
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/design.md
- 根拠: `design.md:36`（D1（仮）の反転条件「spec の追加の SHALL 2 つと Scenario『合言葉の条件は、最初の問いの前に示す』『短い・雛形で断るときは、理由に条件を添える』を落とす」）。R4 の処置でこの Scenario は「端末でないときも、断りより前に合言葉の条件を 1 回だけ示す」に改名された（`specs/distribution/spec.md`）。条件に関わる SHALL も R6 の処置で 3 つ（1 回だけ示す・端末のとき「表示されない」を添える・断りに条件を添える）になり、窓の Scenario「コード頁が UTF-8 でない窓でも…」の THEN にも条件の行と順の確かめが入った。Q1 で「文字化けだけ」が選ばれたとき、`spec --answers` がこの反転条件のとおりに書き直すと、無い名前を探し、窓の Scenario の THEN と「表示されない」の SHALL・tasks 4.1 の `assert_requirements_line` を残す
- kind: technical
- 提案: D1 の反転条件を今の名前と中身に合わせる（落とす: SHALL 3 つ・Scenario「端末でないときも、断りより前に合言葉の条件を 1 回だけ示す」「短い・雛形で断るときは、理由に条件を添える」・tasks 1.1 の `requirements_line` と 4.1。直す: 窓の Scenario の THEN から条件の語と順を外し、問いの文だけにする）。
- 処置: fixed D1 仮 — D1 の反転条件を今の名前と中身に合わせた（落とす SHALL 3 つ・改名後の Scenario 2 本・窓の Scenario 2 本の THEN から条件を外す・tasks 1.1 の requirements_line と 4.1・4.2 の条件の確かめ）

## 観点ごとの該当なし

- 観点 1: R7 のほかは該当なし（確かめた範囲: deep.md Q1〜Q5 の推奨が D1〜D6 と spec の SHALL 3 行・Scenario 5 本・tasks の Global Constraints `:12` に写っている。review/deep.md R1（repair も聞く）→ `spec.md:10`・`design.md:9`、R2（値の口）→ `proposal.md:38`・`design.md:40`、R3 → premises `:12` 確かさ 4、R4 → Q5・D4（flush の申し送りは `design.md:55` で不要と書いた）、R5 → Scenario「窓で打った合言葉は、窓に出ない」と `scenario_names` への追加（`tasks.md:63`）、R6 → D6 の `chcp`・65001 の検査と D7（ConPTY の確かめだけ未了 → R1）、R7 → D5。要件へ戻すものは無い（satisfies 空・`docs/requirements.md` に触れない）。数値 16・`change-me` は Scenario にある）
- 観点 2: R4・R5・R8 のほかは該当なし（確かめた範囲: 写した既存 5 本は正典と逐語。足した 5 本の WHEN / THEN）
- 観点 3: R6・R9 のほかは該当なし（確かめた範囲: specs に関数名・crate 名・列名は無い（`password`・`repair` は利用者が打つ口の名前）。proposal の Modified `distribution` と `specs/distribution/` は一致し、`docs/stories/INDEX.md:108` に `distribution` がある。change なので INDEX の前倒しの理由は要らない）
- 観点 4: R1・R2・R3 のほかは該当なし（確かめた範囲: 全 `- [ ]` にコマンドと rc の終了条件がある。Task 1.1 の `-- --list` で 3 本以上。名前だけ足して印を出さない形は、Linux の `--self-test`（`install-check.sh:1506-1516`。実地の確かめの区間に印がちょうど 1 つ）と Windows の `comm -23`（`ci.yml:452-460`）の両方で落ちる。Task 2.1 の `Cargo.lock` の確かめは `^\+name = ` を見るので、既にある `windows-sys 0.61.2` を足しても通る。依存順は 1→2、3.1→3.2→4.2 で合う。窓の節を API の Scenario（`install-check.sh:991`）の前に置き `$PW2` で決め直す形は、前後の前提（`:978` で `$PW2` に決め直し済み）と合う。7.1 は `> 物理: device` つき。`--probe-console` の口は末尾の分岐（`:1528-1532`）より前に足す必要があり、Task 3.2 が名指している）
- 観点 5（change）: R9 のほかは該当なし（確かめた範囲: 「完了の判定」は空でなく、D1〜D7・Task 1〜4・7.1 と一致する。「壊してはいけないもの」1〜4・6 は写した既存 Scenario と足した向け替えの Scenario が持つ（5 は R5）。specs は依頼（逐語）の 2 点と「password / repair など他の口も同じ」「本物の Windows のコンソールで確かめる」を越えも欠きもしない）
- design の前提と実物: 該当なし（確かめた範囲: rpassword 7.5.4 `lib.rs:270-292`（`prompt_password` → `OpenOptions::open(CONOUT$)` に `as_bytes()` を `write_all`）・`windows.rs:15-16`（既定の入出力は `CONIN$`／`CONOUT$`）・`:380`（`SetConsoleMode(ENABLE_PROCESSED_INPUT)`）・`read_password()` は既定の設定で `CONIN$` から読む（`lib.rs:265`・`config.rs:125-132`）。rpassword の `windows-sys` は `0.61` で `Win32_Foundation`・`Win32_System_Console`・`Win32_Storage_FileSystem` を既に有効にしている（rpassword の `Cargo.toml`）、`Cargo.lock` は `windows-sys 0.61.2`。`Cargo.toml:27` の `unsafe_code = "deny"` と `crates/collector-windows/src/secret.rs:3` の前例。`crates/installer/Cargo.toml` に `[target.'cfg(windows)'.dependencies]` の節は今無く、Task 2.1 が新しく作る。windows-gnu の clippy は `ci.yml:165` が `-p ashiato-setup` を含む）
- 観点 6: 該当なし（`spec --draft`。`--answers` でない）
- 観点 7: 該当なし（spec_overlap.py candidates が 0 件）。参考: ch13 は `tools/workbox.json` の `host_only` に `tools/release/(build|install-check)\.sh` と `powershell\.exe` を持つ（`origin/feat/ch13-workbox-config`）。Task 3.2・4.2 の検証は `install-check.sh` を名指すのでホストで走り、`tasks.md:49` の「作業箱の外で走らせる」と矛盾しない。ch11 は `tools/check-licenses.sh` に 4 行足すが、Task 2.1 は呼ぶだけ

## 第 3 回（spec --answers Q2。HEAD 58c723f64 からの未 commit の差分を読んで。2026-11-10）

対象は Q2 の写しで足した部分（ADDED Requirement と Scenario 5 本・tasks Task 5・6・design D2・D8・D9・Context・Risks・premises.md の 2 行・proposal・`docs/changes/CH14.md` の完了の判定）。既存の R1〜R10 には触れない。観点 6 の結果は `review/answers.md`。

```
openspec validate ch14-installer-password-prompt-mojibake --strict   → valid（rc=0）
python3 scripts/check_chain.py .                                     → chain: OK（rc=0）
python3 scripts/check_scenarios.py . ch14-…                          → FAIL（rc=1）: 担保の無い Scenario 11 件 =
    MODIFIED で足した 6 本（印は Task 4）＋ ADDED の 5 本（印は Task 5.2 の 4 本・Task 6.2 の 1 本）。上流の段では想定どおり
python3 scripts/review_triage.py . ch14-…                            → triage: OK（指摘 18 件・仮決め 3 件・要件へ戻すもの 0）
python3 scripts/spec_overlap.py candidates . ch14-…                  → []（0 件。観点 7 は上の「該当なし」のまま）
```

## R11. `cargo test --lib ch14_` は main.rs の試験を数えないので、Task 6.1 の「7 本以上」が落ちる
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/design.md・tasks.md（と docs/changes/CH14.md の完了の判定）
- 根拠: design D9（`design.md:103`）は試験の置き場を「`crates/installer/src/console.rs`・`main.rs` の `#[cfg(all(test, windows))]`」とする。`output_tail`（`main.rs:81`）・`reg_query_value`（`:48`）・`expand_downloads_shell_folder`（`:41`）は `main.rs`（bin の target。`mod tests` は `main.rs:1638`）の private な関数で、Task 5.1（`tasks.md:76`）は「分ける」とだけ書き、lib へ移すとは書いていない。Task 5.2 の `ch14_output_tail_reads_console_code_page`・`ch14_account_and_downloads_read_in_console_code_page` はこれらを呼ぶので、置き場は main.rs になる。一方 Task 6.1（`tasks.md:89`）と完了の判定（`docs/changes/CH14.md` の「`-- --list` で 7 本以上」）は `cargo test -p ashiato-setup --lib ch14_` で数える。`--lib` は bin の試験を含まないので、数は 3（Task 1）＋ console.rs に置いた分にとどまり、CI の step が落ちる。Task 5.2 の Linux の検証 `grep -rq "fn $n" crates/installer/src` は、どちらに置いても通るので気づけない
- kind: technical
- 提案: 6.1 と完了の判定の数え方から `--lib` を外す（`cargo test -p ashiato-setup ch14_`。フィルタがあるので、ほかの試験は巻き込まない）か、D9・Task 5.1 に「`output_tail` の組み立てと reg の出力からの場所の組み立てを lib（`console.rs` か `commands.rs`）へ移し、試験も lib に置く」と書く。
- 処置: fixed 6.1 — 数え方から `--lib` を外し（`cargo test -p ashiato-setup ch14_`。main.rs（bin）の試験も数える）、下限を 8 本にした（5.2 を 5 本にしたため）。D9・docs/changes/CH14.md の完了の判定も同じに直した

## R12. Task 4.2 と 5.2 の検証の `check_scenarios.py … rc=0` は、書いた順に進めると通らない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/tasks.md
- 根拠: `check_scenarios.py` は change の spec の全 Scenario の印を見る（今の FAIL 11 件の内訳は上の機械の検査）。Task 4.2 の検証（`tasks.md:72`）と Task 5.2 の検証（`tasks.md:85`）は、どちらも `python3 scripts/check_scenarios.py . ch14-installer-password-prompt-mojibake` rc=0 を求める。ADDED の 5 本の印は Task 5.2（4 本）と Task 6.2（1 本）で初めて置かれるので、4.2 の時点では 5 件、5.2 の時点では 1 件（「英字でない名前の管理者が repair しても、記録を残したままログインできる」）が担保なしで残り、rc=1 になる。HEAD の版では 4.2 が最後に印を置くタスクだったので通っていた
- kind: technical
- 提案: `check_scenarios.py` rc=0 を、印を置く最後のタスク（6.2）と 8.1 にだけ残す。4.2・5.2 は「そのタスクが置いた名前が担保なしの一覧に無い」（`check_scenarios.py … | grep -F "<名前>"` が無いこと）に替える。
- 処置: fixed 4.2 — 4.2・5.2 の検証を「その Task が置いた名前が check_scenarios.py の担保なしの一覧に無い」に替えた。全体の rc=0 は印を置く最後の 6.2 と 8.1 に残した

## R13. 単体の Scenario の WHEN「インストーラの窓のコード頁が 932」は、試験の中で作られない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/specs/distribution/spec.md・tasks.md
- 根拠: ADDED の Scenario「窓のコード頁で書いた子の命令の失敗の理由が、化けずに読める」「UTF-8 で書く子の出力は、窓のコード頁が 932 でもそのまま読める」「窓のコード頁で書いた日本語の利用者名とダウンロードの場所を、同じ文字として読む」の WHEN は「インストーラの窓のコード頁が 932 である」。Task 5.2（`tasks.md:79-84`）の試験は `decode_with_code_page(bytes, 932)` のようにコード頁を引数で渡し、`decode_child_output` が窓のコード頁（`GetConsoleOutputCP`）を選ぶ部分を通らない。`decode_child_output` が `GetOEMCP` や固定の 932 で読む実装でも、この 3 本は緑になる。窓のコード頁を選ぶ部分を通るのは install-check の repair の Scenario（runner の OEM のコード頁は 932 でない見込み。`premises.md` の表の 7 行目）だけで、そちらは R14 の前提にも乗っている
- kind: technical
- 提案: 単体の 3 本の WHEN を「コード頁 932 を指して読む」に直し、窓のコード頁を選ぶことは別の 1 本で確かめる（例: Windows の試験で `SetConsoleOutputCP(932)` にした窓から `decode_child_output` を呼ぶ。窓が無ければ `AllocConsole`）か、repair の Scenario がそれを担うと D9 に明記して、その Scenario の失敗が「読み違え」か「DB の認証」かを見分けられるようにする（R14）。
- 処置: fixed 5.2 — 失敗の理由の試験を、`SetConsoleOutputCP(932)` にした窓（無ければ `AllocConsole`）から `decode_child_output` を呼ぶ形にし、窓のコード頁を選ぶ部分を通すようにした。ほかの Scenario の WHEN は「コード頁 932 を指して読む」に直した（spec・D9）

## R14. D9 の反転条件の入口が、Task 6.2 の失敗の出力から見分けられない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/design.md・tasks.md・premises.md
- 根拠: D9 の反転条件（`design.md:107`）は (a)「runner で `chcp 932` が断られたら」と (b)「読みは正しい（…repair の一時のログに書いた利用者名が Windows の名前と同じ）のに PostgreSQL が英字でない名前を認証できないと分かったら」で分かれる。Task 6.2（`tasks.md:91-94`）は `.cmd` の頭で `chcp 932 >nul` とし（断られても何も出ない。D6 は `chcp` の出力を読む）、外れたら「`.out` を出して `die`」だけで、一時の利用者規則（`repair.pg_ident.conf`。`main.rs:1490`）も `repair-db.log` も出さない。どちらに当たったかは CI の出力から決められず、反転の行き先（(a) は文字を替えて続ける・(b) は A として人間へ）が選べない。
  あわせて、(a) の替え方（`é` などの窓のコード頁で表せる文字）は窓のコード頁の問題だけを避ける。pg_ident の行は `fs::write` で UTF-8 のまま書かれ（`main.rs:1490`）、英字でない文字は `is_alphanumeric` ならそのままの文字で入る（`commands.rs:1097-1110`）。PostgreSQL が SSPI から得た名前をどのコード頁のバイト列で比べるかは、premises の新しい 2 行目（確かさ 1）が「SSPI の名前の文字」と 1 語でまとめているだけで、どちらの反転条件も扱っていない
- kind: technical
- 提案: Task 6.2 の `.cmd` を `chcp 932 >nul || exit /b 90` のように断りを別の rc で返す形にし、`die` の前に `repair.pg_ident.conf` と `repair-db.log` の中身（利用者名の行）も出す。premises の行を「runner の `chcp 932`」「`net user` の日本語の名前」「PostgreSQL が SSPI の名前と pg_ident の行をどのバイト列で比べるか」に分け、最後のものは (a) でも避けられないことを D9 の反転条件に書く（知識の上では、Windows の ANSI のコード頁の名前と UTF-8 で書いた行が食い違う見込みがあり、確かめていない）。
- 処置: fixed D9 仮 — `.cmd` を `chcp 932 >nul || exit /b 90` にし、外れたら `repair.pg_ident.conf` と `repair-db.log` の末尾を出してから止める形にした（tasks 6.2）。premises の行を runner の chcp 932・net user の日本語の名前・PostgreSQL の突き合わせのバイト列の 3 つに分け、(a) の文字の替え方では (b) を避けられないことを D9 の反転条件に書いた

## R15. 足した Scenario のうち 3 本が、THEN に別々の主張を束ねている
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/specs/distribution/spec.md
- 根拠: 「英字だけの子の出力は、いまと同じ値として読む」は WHEN に 3 つの出力 × 2 つのコード頁、THEN に「値が同じ」と「repair の DB の認証の対応表の行も同じ」。「窓のコード頁で書いた日本語の利用者名とダウンロードの場所を、同じ文字として読む」は THEN に利用者名・ダウンロードの場所・対応表の行の 3 つ。「英字でない名前の管理者が repair しても、記録を残したままログインできる」は THEN に rc=0・記録の件数と DB が変わらない・ログインできるの 3 つ（「DB は変わらず」は Task 6.2 では `pg_database` の oid。Scenario の字面には無い）。どれも試験 1 本に 1 つの印で、片方の主張だけを確かめる試験でも印は立つ
- kind: technical
- 提案: 利用者名（と対応表の行）とダウンロードの場所を別の Scenario に分ける。repair の Scenario は THEN を「rc=0 で終わり、決めた合言葉で画面にログインできる」に絞り、記録と DB が変わらないことは今の「別の管理者が repair しても、本人は変わらない」と同じ扱いにするか、「DB は変わらず」を「DB が作り直されない（同じ DB のまま）」と測れる字面にする。
- 処置: fixed specs/distribution/spec.md — 日本語の利用者名とダウンロードの場所を 2 本の Scenario に分けた。英字だけの Scenario の THEN から対応表の行を外した。repair の Scenario は THEN を rc=0 とログインに絞り、名前を「英字でない名前の管理者でも repair でき、決めた合言葉でログインできる」にした（DB と記録の件数は tasks 6.2 があわせて見る）

## R16. D8 の「読めない並びは置換の文字にして止まらない」は観測できる振る舞いで、design にだけある
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/design.md
- 根拠: `design.md:93`（D8: 読めない並びは置換の文字にする。いまの `from_utf8_lossy` と同じく止まらない）・`:98`（`psql`・`pg_ctl` が UTF-8 でも窓のコード頁でもない文字で書けば置換の文字が出る。止まらない）。ADDED Requirement の SHALL（spec の「子が書いた文字として読む（…化けさせない）」）は、読めない出力のときにインストーラが止まらず置換の文字で続けることを言っておらず、Scenario も無い。archive で正典から落ちる
- kind: technical
- 提案: SHALL に「どのコード頁でも読めない並びは置換の文字にして読み、それで止まらない」を 1 行足すか、約束しない（いまの振る舞いのまま、spec に書かない）と D8 に明記する。
- 処置: fixed specs/distribution/spec.md — 「UTF-8 としても窓のコード頁としても読めない並びは置換の文字にして読み、それを理由に止まらない（いまと同じ）」を SHALL に足し、D8 から参照した

## R17. Task 5.2 の期待するバイト列の出所（premises.md の実物）に、そのバイト列が無い
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/tasks.md・design.md
- 根拠: Task 5.2（`tasks.md:79`）と D9（`design.md:103`）は、期待するバイト列を「`premises.md` の本人の PC の実物の出力から取る」とする。premises.md の新しい 1 行目にあるバイト列は `tasklist` の文の先頭 `8f ee 95 f1 3a 20` だけで、試験が使う「指定されたサービスはインストールされたサービスとして存在しません。」・`pc\足跡太郎`・`%USERPROFILE%\ダウンロード` の値の `reg.exe query` の出力は無い（本人の利用者名は英字。`proposal.md` の Impact）。下流は書いたとおりには取れない
- kind: technical
- 提案: 出所を「cp932 で符号化したバイト列（例: `python3 -c "print('…'.encode('cp932').hex(' '))"`。Linux の Python は cp932 の表を持つ）」に直すか、使う 3 つのバイト列を premises.md に実物から足す。
- 処置: fixed 5.2 — 期待するバイト列の出所を「Linux の Python の `'…'.encode('cp932')` で作る」に直した（D9 も同じ）

## R18. 足した後の数と参照が、proposal・design・tasks の古い字面と合わない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/proposal.md・design.md・tasks.md
- 根拠: (a) `proposal.md` の Impact の試験の行は「`scenario_names` に 6 つの名前」のまま（Task 6.2 で 7 つ）。走っている change との重なりの行は「この change は `ci.yml` の `install-windows` に step を 1 つ足すだけ」のままで、`package-windows` の step（Task 6.1）が入っていない。(b) design Context（`design.md:17`）の `Get-Acl` の読みは `main.rs:752` と書くが、実物は `:745`。(c) tasks の Global Constraints（`tasks.md:11`）は「足した 6 本は Task 4 が印を置き（ADDED の 5 本は、4 本を Task 5.2 の Rust の試験の `// Scenario:`、1 本を Task 6.2 の install-check が置く）、`scenario_names`（`--list`）に名前を足す」と読め、Rust の試験の 4 本の名前も `scenario_names` に足すように読める。足すと `install-windows` の `comm -23`（`ci.yml` の install-windows の step）が「印が無い」で落ちる
- kind: technical
- 提案: (a) proposal の数を 7 にし、重なりの行に `package-windows` の step を足す（ch11・ch12 が `package-windows` に触るかも確かめる）。(b) `:745` に直す。(c) Global Constraints を「`scenario_names` に足すのは install-check が印を出す 7 本だけ（Rust の試験の 4 本は足さない）」と書き分ける。
- 処置: fixed proposal.md — (a) scenario_names を 7 つ（Rust の試験の 5 本は足さない）にし、重なりの行に package-windows の step を足した（ch11・ch12 は install-windows・package-windows に触らない）。(b) design の `:752` を `:745` に直した。(c) tasks の Global Constraints を、scenario_names に足すのは install-check が印を出すものだけ、Rust の試験の 5 本は足さない、と書き分けた

## R19. 日本語の Windows で実物の子の命令の出力を読むことを、CI も確認バッチも見ない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/design.md・tasks.md・docs/changes/CH14.md
- 根拠: 足した Risks の行（`design.md` の Risks の 2 つ目）は「日本語で実地に通るのは確認バッチ（tasks 9.1）の窓だけ」と書く。9.1（`tasks.md:110`）の本文は番号が 7.1 から変わっただけで、問いと条件の表示・打った文字が出ないこと・15 文字の断りしか見ない。`docs/changes/CH14.md` の完了の判定の Q2 の 3 行にも確認バッチの行が無い。CI の runner は英語の OS で日本語の文を出す子が無く（同じ Risks の行）、単体の試験は 16 進のバイト列を読むだけ。日本語の Windows で `sc.exe` などの実物の出力を `decode_child_output` が読むことは、どの確かめも通らない
- kind: technical
- 提案: 9.1 に 1 項目足す（例: 本人の PC で、`ashiato-server` のサービスが無い状態で失敗の理由に `sc.exe` の出力が添わる口を 1 つ通し、日本語の文が化けずに出ることを見る。通せる口が無ければ、その旨と代わりの確かめ）か、Risks の行を「日本語の Windows で実物の子の出力を読むことは確かめない」と事実どおりに直し、完了の判定にもそう書く。
- 処置: fixed D9 仮 — Risks の行を「日本語の Windows で実物の子の日本語の出力を読むことは CI も確認バッチも確かめない。子が窓のコード頁で書くことは本人の PC の実物（確かさ 5）、その符号化の読みは単体の試験で確かめ、2 つを合わせて確かめたことにする」と事実どおりに直し、docs/changes/CH14.md の完了の判定にも同じことを書いた

## 第 3 回の観点ごとの該当なし（足した部分について）

- 観点 1: R14・R19 のほかは該当なし（確かめた範囲: deep.md Q2 の「表示も値の口も含める」が ADDED Requirement の SHALL 4 行（表示 = 失敗の末尾、値 = 利用者名・ダウンロードの場所、英字だけは同じ値）と Scenario 5 本・design D2・tasks の Global Constraints に写っている。数値（コード頁 932・437）は Scenario にある。Q2 の古い内容（「直さない」・Non-Goals の行・proposal の「見つけたが直さないもの」・D2（仮）の反転条件）は残っていない。要件へ戻すものは無い）
- 観点 2: R13・R15 のほかは該当なし（確かめた範囲: ADDED の 5 本の WHEN / THEN と、Requirement の本文にだけある主張（R16 を除く））
- 観点 3: R16 のほかは該当なし（確かめた範囲: ADDED の Requirement と Scenario に関数名・crate 名・列名は無い（`sc.exe`・`whoami`・`reg.exe`・`repair` は利用者が見る命令と口の名前）。proposal の Modified Capabilities は `distribution` のままで、`specs/distribution/` と一致する）
- 観点 4: R11・R12・R14・R17 のほかは該当なし（確かめた範囲: Task 5.1〜6.2 の全 `- [ ]` にコマンドと rc の終了条件がある。Task 6.2 が使う `make_users`・`remove_users`・`run_as_admin_b`・`with_settings_write`・`wait_healthz`・`app_sql`（oid と `core.source` の件数。`install-check.sh:1033`・`:1077`）は実在する。依存順は 5.1→5.2→6.1、5.1→6.2 で合う（R12 の検査の順を除く））
- 観点 5（change）: R19 のほかは該当なし（確かめた範囲: 完了の判定に Q2 の 3 行があり、壊してはいけないもの 7 は Scenario「英字だけの子の出力は、いまと同じ値として読む」が持つ。依頼（逐語）を越える範囲は deep.md Q2 の本人の決定に依る）
- 観点 6: `review/answers.md` に書いた
- 観点 7: 上の「該当なし」のまま（spec_overlap.py candidates が 0 件）

## 第 4 回（R11〜R19 の処置を読んで。2026-11-10）

処置はどれも成果物で確かめた。R11（6.1・D9・完了の判定から `--lib` を外し、下限 8）・R12（4.2・5.2 はその Task の名前が担保なしの一覧に無いことを見る。全体の rc は 6.2・8.1）・R14（`.cmd` の rc=90、失敗時に `repair.pg_ident.conf` と `repair-db.log` の末尾を出す。premises を 3 行に分けた。(a) では (b) を避けられないと D9 に書いた）・R15（Scenario を分け、THEN を絞った）・R17（cp932 のバイト列は Python で作る）・R18（proposal の 7 つ・`package-windows` の step・`:745`・Global Constraints の書き分け）・R19（Risks と完了の判定を「確かめない」に直した）は、指摘を満たしている。R13 と R16 は、直したことで次の R20・R21 が出た。

```
openspec validate ch14-installer-password-prompt-mojibake --strict   → valid（rc=0）
python3 scripts/check_chain.py .                                     → chain: OK（rc=0）
python3 scripts/check_scenarios.py . ch14-…                          → FAIL（rc=1）: 担保の無い Scenario 12 件 =
    MODIFIED で足した 6 本 ＋ ADDED の 6 本（R15 で 1 本増えた）。印は Task 4・5.2・6.2 が置く。想定どおり
python3 scripts/review_triage.py . ch14-…                            → triage: OK（指摘 27 件・仮決め 5 件・要件へ戻すもの 0）
python3 scripts/spec_overlap.py candidates . ch14-…                  → []（0 件。観点 7 は「該当なし」のまま）
```

## R20. 窓のコード頁を選ぶ試験（R13 の処置）は `unsafe` を呼ぶが、置き場の main.rs では `unsafe` を書けない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/tasks.md・design.md
- 根拠: Task 5.2 の `ch14_output_tail_reads_console_code_page`（`tasks.md` の 5.2）は、試験の中で `AllocConsole`・`SetConsoleOutputCP(932)`（どちらも `windows-sys` の FFI で `unsafe`）を呼び、`decode_child_output` で読んで「組んだ出力の末尾」を確かめる。出力の末尾を組む関数は Task 5.1 が `main.rs` の `output_tail` から分けるもので、置き場は書かれていない。5.2 は「置き場は、呼ぶ関数のあるモジュール（`console.rs` か `main.rs` の `mod tests`）」とする。main.rs に置くと、作業場の lint は `unsafe_code = "deny"`（`Cargo.toml:27`）で、`allow(unsafe_code)` は `console.rs` の 1 行だけ（Global Constraints・Task 5.1 の検証の `wc -l | grep -qx 1`・`tools/check-unsafe-scope.sh`）なので組めない。console.rs に置くと、lib の試験から bin（main.rs）の関数は呼べない。console.rs に試験だけの安全な包み（`#[cfg(test)]`）を置いても、bin の試験を組むときの lib は `cfg(test)` なしで組まれるので見えない
- kind: technical
- 提案: Task 5.1 に「出力の末尾を組む関数（文字列から、空行を除いた末尾 5 行を ` / ` で繋ぐ）は lib（`console.rs`）に置き、main.rs の `output_tail` はそれを呼ぶ」と書き、この試験を console.rs に置くと 5.2 に名指す。または、窓のコード頁を一時的に替える安全な関数を console.rs の公開の関数として置く（`cfg(test)` にしない）と書く。
- 処置: fixed 5.1 — 出力の末尾を組む中身を lib の `console.rs` の `pub fn output_tail_text(stdout, stderr)` へ移し（main.rs の `output_tail` は呼ぶだけ）、窓のコード頁を切り替える試験を `console.rs` に置くと Task 5.1・5.2 と D9 に名指した

## R21. R16 の処置で足した SHALL（読めない並びは置換の文字にして止まらない）に Scenario が無い
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/specs/distribution/spec.md
- 根拠: ADDED Requirement の 5 つ目の SHALL「UTF-8 としても窓のコード頁としても読めない並びは置換の文字にして読み、それを理由に止まらない（いまと同じ）」に当たる `#### Scenario:` が無い（Scenario 6 本の WHEN は、どれも読める出力）。validate は Requirement の本文だけが言っていることを落とさないので、実装が読めない並びで止まっても（`MultiByteToWideChar` の失敗を `Err` で返すなど）spec は緑のまま
- kind: technical
- 提案: Scenario を 1 本足す（例: WHEN 子の出力がコード頁 932 でも UTF-8 でも読めない並び（後ろが欠けた 2 バイト文字の先頭のバイト `0x82` で終わる、など）で、それを 932 を指して読む / THEN 読んだ文字列に置換の文字（U+FFFD）があり、読みが失敗を返さない）。Task 5.2 に `ch14_` の試験を 1 本足し、6.1 の下限を 9 にする。
- 処置: fixed specs/distribution/spec.md — Scenario「どのコード頁でも読めない並びは、置換の文字にして止まらない」と試験 `ch14_unreadable_bytes_become_replacement`（tasks 5.2）を足し、6.1 と完了の判定の下限を 9 にした

## R22. Task 4.2・5.2 の新しい検証は、`check_scenarios.py` が走らなくても通る
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/tasks.md
- 根拠: R12 の処置で 4.2・5.2 の検証は `o=$(python3 scripts/check_scenarios.py … 2>&1); for n in …; do grep -F "担保の無い Scenario: $n" <<<"$o" && exit 1; done; exit 0` になった。台本が見つからない・例外で止まる・change の名前を誤るなどで `担保の無い Scenario:` の行が 1 行も出なくても rc=0 になる（「無い」ことだけを見ている）
- kind: technical
- 提案: 同じ出力に要約の行 `scenarios: ` があることも求める（例: `grep -q "^scenarios: " <<<"$o" || exit 1` を先に置く）。
- 処置: fixed 4.2 — 4.2・5.2 の検証に、check_scenarios.py の出力に要約の行 `scenarios: ` があることを足した

## 第 5 回（R20〜R22 の処置を読んで。2026-11-10）

R20（出力の末尾を組む `output_tail_text(stdout: &[u8], stderr: &[u8])` を lib の `console.rs` へ移し、窓のコード頁を切り替える試験も `console.rs` に置くと Task 5.1・5.2・D9 に名指した）・R22（4.2・5.2 の検証が、要約の行 `scenarios: ` が無ければ rc=1）は解けている。`check_scenarios.py` は今も要約の行 `scenarios: FAIL (担保なし 13 件 …)` を出すので、R22 の確かめの形は今の台本で通る。R21 は Scenario「どのコード頁でも読めない並びは、置換の文字にして止まらない」と試験 `ch14_unreadable_bytes_become_replacement` が足され、6.1・完了の判定の下限が 9 になった。ただし THEN の期待する置換の文字の出どころが決まっていないので、R23 を立てる。担保の無い Scenario は 13 件（MODIFIED で足した 6 本と ADDED の 7 本）で、想定どおり。

## R23. 読めない cp932 の並びが U+FFFD になることは、`MultiByteToWideChar` の既定の振る舞いではない見込みで、作り方が決まっていない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/specs/distribution/spec.md・design.md・tasks.md
- 根拠: 新しい Scenario「どのコード頁でも読めない並びは、置換の文字にして止まらない」の THEN と Task 5.2 の `ch14_unreadable_bytes_become_replacement` は、「指定された」の cp932 のバイト列の後ろに `0x82` を足した並びを `decode_with_code_page(…, 932)` で読むと「`U+FFFD` で終わる」ことを求める。D8（`design.md:93`）と Task 5.1 は「`MultiByteToWideChar(code_page, …)`。読めない並びは置換の文字」とだけ書き、フラグ（`MB_ERR_INVALID_CHARS`）も、壊れた並びを U+FFFD に置き換える手順も書いていない。本人の PC の実物（`premises.md` の表の 9 行目。確かさ 5）では、cp932 の窓が壊れた 2 バイトの並び（`画面の合言葉: ` の UTF-8 の末尾の `0x89 0x3A`）を `闡・ ` と出した。つまり cp932 の変換は、壊れた並びを U+FFFD でなく、コード頁の既定の文字 U+30FB『・』にした（`:` が消えている）。Windows の変換が既定のまま U+FFFD を返す根拠は成果物に無い。末尾の欠けた 1 バイト目をどう扱うか（『・』にする・落とす）も確かめていない。既定のフラグで書くと試験が Windows の CI で落ちるか、試験に合わせて THEN を後から緩めることになる。
  あわせて、Scenario「窓のコード頁で書いた子の命令の失敗の理由が、化けずに読める」の THEN は「置換の文字（U+FFFD）が無い」で化けていないことを測る。cp932 の変換が壊れた並びを『・』にするなら、読み違えても U+FFFD は出ず、この確かめは化けを見分けない（同じ THEN の「その文が同じ文字のまま入る」の側は見分ける）
- kind: technical
- 提案: D8 と Task 5.1 に作り方を名指す（例: `MB_ERR_INVALID_CHARS` を付けて呼び、失敗したら `IsDBCSLeadByteEx` で文字の切れ目を数えながら読めない所だけ U+FFFD にする）。Windows の既定の変換に任せるなら、Scenario の THEN を「止まらずに文字列を返し、その前の文字はそのまま残る」と、置換の文字の種類を決めない形に直す。失敗の理由の Scenario の THEN は、「置換の文字が無い」より「同じ文字のまま入る」を主にする（U+FFFD と U+30FB のどちらも見ないなら、その句を外す）。
- 処置: fixed D8 仮 — 作り方を D8 と Task 5.1 に名指した（`MB_ERR_INVALID_CHARS` で全体を読み、失敗したら `IsDBCSLeadByteEx` で 1 文字ずつ読んで読めない文字だけを U+FFFD にする。旗の無い既定の読みは使わない）。失敗の理由の Scenario は THEN の主が「その文が同じ文字のまま入る」で、読み違えはそこで落ちるので変えない

## 第 6 回（R23 の処置を読んで。2026-11-10）

R23 は解けている。D8 と Task 5.1 は、まず `MB_ERR_INVALID_CHARS` を付けて `MultiByteToWideChar` で全体を読む。失敗したら `IsDBCSLeadByteEx` で 1 文字の長さを決めて 1 文字ずつ同じ旗で読み、読めない文字（末尾で 2 バイト目が欠けたものを含む）だけを U+FFFD にする。旗の無い既定の読みは使わないと、理由（`premises.md` 9 行目の `・`）つきで名指した。名指した API と定数（`MultiByteToWideChar`・`MB_ERR_INVALID_CHARS`・`IsDBCSLeadByteEx`・`GetOEMCP` は `Win32_Globalization`、`GetConsoleOutputCP`・`SetConsoleOutputCP`・`AllocConsole` は `Win32_System_Console`）は、`Cargo.lock` の `windows-sys 0.61.2` の source に実在する。失敗の理由の Scenario は THEN の主が「同じ文字のまま入る」で、別のコード頁で読み違えればそこで落ちるので、提案のうち「主にする」を満たしている。新しい指摘は無い。
