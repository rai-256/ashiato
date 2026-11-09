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

正典との突き合わせ: `diff <(sed -n 1773,1807p openspec/specs/distribution/spec.md) <(sed -n 3,41p specs/distribution/spec.md)` の差は、SHALL 3 行・導出元の 1 句・理由の 1 行の**追加だけ**。既存の SHALL 5 行と Scenario 5 本は逐語で残っている（Scenario は落ちていない）。

## R1. ConPTY が runner で使えるかは「spec の段で確かめる」と書いたまま未確認で、D6 の反転条件は今の Task では起きない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/premises.md・design.md
- 根拠: `review/deep.md:71`（R6 の処置「ConPTY が runner で使えるかは spec の段で確かめる」）/ `premises.md:16`（確かめていない・確かさ 1 のまま。しかもこの行は `:15` の段落の後にあり、表の外に落ちている）/ `design.md:76`（反転条件「ConPTY が windows-latest の runner の step から使えない…と **Task 3 の確かめで**分かれば A として人間に返す」）/ `tasks.md:49`（Task 3.2 の確かめは「WSL から本人の PC の PowerShell で」—— runner では走らない）。runner で初めて走るのは PR の CI の `install-windows`（`ci.yml:453`）で、Task 1〜4 を全部書いた後。下流から spec へ戻る辺も無い。
  あわせて、CI はインストールの確かめを `| tee` で流す（`ci.yml:453`）ので、台本の PowerShell の標準の handle は管になっている。子が疑似コンソールにつながるか（`STARTUPINFOEX` の std handle をどう渡すか）は design にも tasks にも無く、つながらなければ子の `is_terminal()` が偽で管の道を通る。これも runner の上でしか分からない
- kind: premise
- 提案: GO の前に、本人の PC と（できれば）runner で `CreatePseudoConsole` → 子が窓につながる（子の `[Console]::IsInputRedirected` が偽）ことを 1 回確かめて premises の確かさを上げるか、確かさ 1 のまま進むと明記する。反転条件は「PR の CI の install-windows で probe が落ちたら」に書き直し、そのときの行き先（窓の Scenario 3 本を外して別の change に出す、など）を到達できる形で書く。premises の行は表の中へ戻す。
- 処置: fixed D6 仮 — 失うものが無い（試験の置き場で、後から替えれば戻る）ので確かさ 1 のまま進むと D6 と premises に明記し、premises の行を表の中へ戻した。runner で先に確かめる step（`install-check.sh --probe-console`）を `install-windows` のインストールの前に足し（tasks 3.3）、反転条件を「PR の CI のその step が落ち、原因が runner で ConPTY が使えないことなら A として人間へ」に書き直した。子の std handle を疑似コンソールにつなぐ形（`STARTF_USESTDHANDLES` を立てない・継がない）と、子が窓につながった印 `CONSOLE-OK` を D6・tasks 3.1 に足した

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

## 観点ごとの該当なし

- 観点 1: R7 のほかは該当なし（確かめた範囲: deep.md Q1〜Q5 の推奨が D1〜D6 と spec の SHALL 3 行・Scenario 5 本・tasks の Global Constraints `:12` に写っている。review/deep.md R1（repair も聞く）→ `spec.md:10`・`design.md:9`、R2（値の口）→ `proposal.md:38`・`design.md:40`、R3 → premises `:12` 確かさ 4、R4 → Q5・D4（flush の申し送りは `design.md:55` で不要と書いた）、R5 → Scenario「窓で打った合言葉は、窓に出ない」と `scenario_names` への追加（`tasks.md:63`）、R6 → D6 の `chcp`・65001 の検査と D7（ConPTY の確かめだけ未了 → R1）、R7 → D5。要件へ戻すものは無い（satisfies 空・`docs/requirements.md` に触れない）。数値 16・`change-me` は Scenario にある）
- 観点 2: R4・R5・R8 のほかは該当なし（確かめた範囲: 写した既存 5 本は正典と逐語。足した 5 本の WHEN / THEN）
- 観点 3: R6・R9 のほかは該当なし（確かめた範囲: specs に関数名・crate 名・列名は無い（`password`・`repair` は利用者が打つ口の名前）。proposal の Modified `distribution` と `specs/distribution/` は一致し、`docs/stories/INDEX.md:108` に `distribution` がある。change なので INDEX の前倒しの理由は要らない）
- 観点 4: R1・R2・R3 のほかは該当なし（確かめた範囲: 全 `- [ ]` にコマンドと rc の終了条件がある。Task 1.1 の `-- --list` で 3 本以上。名前だけ足して印を出さない形は、Linux の `--self-test`（`install-check.sh:1506-1516`。実地の確かめの区間に印がちょうど 1 つ）と Windows の `comm -23`（`ci.yml:452-460`）の両方で落ちる。Task 2.1 の `Cargo.lock` の確かめは `^\+name = ` を見るので、既にある `windows-sys 0.61.2` を足しても通る。依存順は 1→2、3.1→3.2→4.2 で合う。窓の節を API の Scenario（`install-check.sh:991`）の前に置き `$PW2` で決め直す形は、前後の前提（`:978` で `$PW2` に決め直し済み）と合う。7.1 は `> 物理: device` つき。`--probe-console` の口は末尾の分岐（`:1528-1532`）より前に足す必要があり、Task 3.2 が名指している）
- 観点 5（change）: R9 のほかは該当なし（確かめた範囲: 「完了の判定」は空でなく、D1〜D7・Task 1〜4・7.1 と一致する。「壊してはいけないもの」1〜4・6 は写した既存 Scenario と足した向け替えの Scenario が持つ（5 は R5）。specs は依頼（逐語）の 2 点と「password / repair など他の口も同じ」「本物の Windows のコンソールで確かめる」を越えも欠きもしない）
- design の前提と実物: 該当なし（確かめた範囲: rpassword 7.5.4 `lib.rs:270-292`（`prompt_password` → `OpenOptions::open(CONOUT$)` に `as_bytes()` を `write_all`）・`windows.rs:15-16`（既定の入出力は `CONIN$`／`CONOUT$`）・`:380`（`SetConsoleMode(ENABLE_PROCESSED_INPUT)`）・`read_password()` は既定の設定で `CONIN$` から読む（`lib.rs:265`・`config.rs:125-132`）。rpassword の `windows-sys` は `0.61` で `Win32_Foundation`・`Win32_System_Console`・`Win32_Storage_FileSystem` を既に有効にしている（rpassword の `Cargo.toml`）、`Cargo.lock` は `windows-sys 0.61.2`。`Cargo.toml:27` の `unsafe_code = "deny"` と `crates/collector-windows/src/secret.rs:3` の前例。`crates/installer/Cargo.toml` に `[target.'cfg(windows)'.dependencies]` の節は今無く、Task 2.1 が新しく作る。windows-gnu の clippy は `ci.yml:165` が `-p ashiato-setup` を含む）
- 観点 6: 該当なし（`spec --draft`。`--answers` でない）
- 観点 7: 該当なし（spec_overlap.py candidates が 0 件）。参考: ch13 は `tools/workbox.json` の `host_only` に `tools/release/(build|install-check)\.sh` と `powershell\.exe` を持つ（`origin/feat/ch13-workbox-config`）。Task 3.2・4.2 の検証は `install-check.sh` を名指すのでホストで走り、`tasks.md:49` の「作業箱の外で走らせる」と矛盾しない。ch11 は `tools/check-licenses.sh` に 4 行足すが、Task 2.1 は呼ぶだけ
- 処置: fixed proposal.md — proposal の Modified Capabilities（`distribution`）に合わせ、`docs/changes/CH14.md` の front matter を `capability: [distribution]` にした

