# impact-review — ch14-installer-password-prompt-mojibake

seen: IMP-01 medium, IMP-02 medium, IMP-03 medium, IMP-04 medium

IMP-01 の根拠は実物で支えられた。`tools/check-unsafe-scope.sh:13-15` の perl は `\b(allow|expect|warn)\s*\(([^)]*,)?\s*unsafe_code` で、`find crates -name '*.rs' -not -path "$ALLOWED"` の全部を見る。D4 の `#![cfg_attr(windows, allow(unsafe_code))]` は `allow(unsafe_code` を含むので落ちる。`ci.yml:134` の lint の job で毎回走り、merge 前に CI が止めるので、medium でよい。`high` にする理由は見つからなかった。
`3 つの口が read_web_passwords 1 つ`（D2）と、`受け付けない: 短い` の字面の grep（D3）も実物で合う（main.rs:1199・:1400、install-check.sh:975・:996）。

## R1 — IMP-01 の options が直しの費用を小さく見せ、unknowns が答えの出せる問いを残している
- 対象: impact.findings[IMP-01].undefined / options / unknowns
- 種類: 根拠
- 指摘:
  1. `undefined` は「ST56 の決定も扱っていない」とあるが、`tools/check-unsafe-scope.sh:2-4` は「許してよいのは secret.rs だけ（review/code.md R32）」と明示している。広げる案は、前の決定を覆す案として options に書く必要がある。
  2. options 1 の「tasks に 1 項目」は足りない。`ALLOWED` は文字列 1 つで、`-not -path "$ALLOWED"`（:14）と `--self-test`（:39-41）がその 1 つ前提で組まれている。`ci.yml:133` の注記「unsafe を許すのは PC の DPAPI（secret.rs）だけ」も変わる。
  3. `unknowns` の「unsafe を使わずに書く手があるか未確認」は、確かめれば答えが出る。Rust の std は stdout / stderr が窓のとき `WriteConsoleW` で書く。依頼文の「ほかの行は Rust の標準出力を通るので化けない」（docs/changes/CH14.md:15）も同じことを言っている。窓が向け替えられていないときは std で足りる。`CONOUT$` を `std::fs` で開いて書くと `WriteFile` の経路になり化けるので、向け替え対応（Q5 案 1）だけが unsafe を要する。
- 直し方の案: options に「前の決定（R32）を覆す」費用を書く。「窓が向け替えられていなければ std、向け替え時だけ CONOUT$」の案を足す。「広げる」を選ぶなら、tasks に script・self-test・ci.yml の注記の 3 点を書く。
- 処置: fixed: IMP-01 の undefined に R32 の決定を覆すことを書き、options を 3 案（広げるなら直す 3 点・向け替えないときは std で書き unsafe を使わない・crate）に直し、unknowns を std では向け替え時に足りないことへ絞った

## R2 — 窓の Scenario が、折り返す長い条件の行を「1 行」として探す。この unknown が IMP-02 に無い
- 対象: impact.findings[IMP-02].unknowns（と tasks 4.2 / design Risks）
- 種類: 新しい帰結
- 指摘:
  - D3 の条件の行は表示幅で約 168 桁（全角を 2 桁で数えた）。ConPTY は probe.ps1:30 のとおり 120 桁で作る。折り返しの扱いは ConPTY の版しだいで、行の途中で改行や桁位置の移動（VT）が入りうる。
  - tasks 4.2 は「VT を除いた窓の文字に、`16 文字以上`・`change-me で始めない`・`API の合言葉`・`ログインの印の鍵`・`表示されません` を全部含む行がある」ことを求めている。折り返しが入ると runner の CI で落ちる。
  - design Risks は「短い行（問いの文）だけを探す」と書いているが、4.2 は逆に長い行を探す。矛盾している。
  - premise-probe/result.txt は短い文（『画面の合言葉: 』）しか通していない。本人の PC でも長い行は確かめていない。
  - IMP-02 の unknowns は「runner で ConPTY が動くか」「コード頁」の 2 つだけ。折り返しは入っていない。
- 直し方の案: 条件の語ごとに別々に探す（行にまとめない）か、条件を 120 桁以内の複数行にする。IMP-02 の unknowns に「長い行の折り返し」を足し、本人の PC で長い行を通して確かめる。
- 処置: fixed: IMP-03（medium・derived）を足し、open と test.5 に載せた。tasks 4.2 と design Risks の食い違いは reading では直さず open に出した

## R3 — install-check は CI で 2 回走る。窓の節は 2 倍の時間を使い、`--probe-console` は 1 回しか守らない
- 対象: impact.dependencies[D5] / targets[T3]
- 種類: 依存の見落とし
- 指摘:
  - `ci.yml:453-454` は release の zip と test の zip で install-check を続けて 2 回走らせる。窓の Scenario 4 本は 2 回とも通り、`password` の口（サービスを起こし直して最大 180 秒待つ。design D6）が 1 回あたり 2 回、計 4 回増える。
  - T3 / D5 はこれを数えていない。2 回目（test zip）の窓の節が、1 回目と別の理由（test build の配置）で落ちる場合の扱いも書いていない。
  - `--probe-console` の step は 1 回しか守らない。
- 直し方の案: T3 の change か coverage に「2 回走る」と増える時間を書く。窓の節は release の回だけにするかを決める。
- 処置: fixed: T3 の change と IMP-02 の text に 2 回走ること・起こし直しが増えることを書いた。install-windows に時間の上限は無い（ci.yml:438-442）。窓の節を release の回だけにするかは仕様の段に任せる

## R4 — situations の `can` が spec の Scenario を写しただけで、`cannot` の「見つけていない」が確かめた範囲を超える
- 対象: impact.situations[normal].can / cannot、situations[mixed].cannot、overview
- 種類: 言い切り
- 指摘:
  - `normal.can` の「日本語の Windows の窓で問いと条件を読め、決められる」は `basis: spec` で、Scenario を写している。条件の行（168 桁）の窓での読みやすさ（折り返し）は、どの実物でも通していない。
  - `normal.cannot` と `update.cannot` の「見つけていない」は、`not_examined` にある Windows Terminal や conhost 以外の窓を調べていないことと、同じ口調で並んでいる。
  - `overview` の「本人ができなくなることを見つけていない」は、折り返し（R2）を探していない結果である。
- 直し方の案: `normal.cannot` に「条件の行の折り返しは確かめていない」を書く。`can` は「本人の PC の conhost で（疑似コンソールの再現で）」と範囲を付ける。
- 処置: fixed: normal.can に範囲を付け、normal.cannot・mixed.cannot・overview に確かめていない範囲（折り返し・conhost 以外）を書いた

## R5 — 閉じ方の材料が無いまま acceptance が「IMP-01 の直し方を仕様の段で決めてから」と置いている
- 対象: impact.acceptance[2] / findings[IMP-01]
- 種類: 閉じ方
- 指摘:
  - findings に `if_accepted` / `if_deferred` の欄が無い。acceptance の「直し方を決めてから」は予定を書いただけで、いま進めて安全と言える手当てが実在しない。
  - tasks 2.1 は D4 のとおりに console.rs を作る指示なので、そのまま下流が実装すると IMP-01 の帰結（CI の lint が落ちる）が確実に起きる。R1 の 3 案のどれにするかが、spec / tasks のどこにも書かれていない。
- 直し方の案: 上流で option を 1 つ選び、tasks 2.1 と design D4 に書く。選ばないなら、下流が先に `tools/check-unsafe-scope.sh` を走らせる Task（検証の行）を 2.1 に足す。
- 処置: kept: reading の段は仕様（design D4・tasks 2.1）を直さない。acceptance に、合意の前に IMP-01 の直し方を選び違和感として返す道を書き、scope.3 を highlights に上げて人間に見せる。medium なので if_accepted / if_deferred は要らない



## R6 — R2 の `blocks_legit: false` は、「いまも行は名前と合わない」を前提にしているが、この前提が成り立たない利用者が design の中に残る
- 対象: impact.rules[R2].legit / blocks_legit、lost[3]、findings[IMP-04].unknowns
- 種類: 根拠
- 指摘:
  1. ANSI のコード頁が 65001 の PC（「Beta: Use Unicode UTF-8」を入れた Windows）では、SSPI が渡す名前のバイト列は UTF-8 で、いまの行（UTF-8 のまま。commands.rs:1097-1110 は `is_alphanumeric()` の文字を生で書く）と同じになりうる。一方 D9 の `encode_with_code_page(…, WC_NO_BEST_FIT_CHARS, …)` は、`WideCharToMultiByte` が CP_UTF8 に対して 0 か `WC_ERR_INVALID_CHARS` 以外の旗を `ERROR_INVALID_FLAGS` で断る（`lpUsedDefaultChar` も NULL でなければならない）ので、`GetACP()` が 65001 のとき全部の英字でない名前が「表せない」で止まる。design D9・tasks 5.1 はこの場合を扱っていない。R2 の legit に無く、IMP-04 の unknowns にも無い。
  2. 「表せない名前」側も、premises.md の行（確かさ 2）は PostgreSQL が UTF-8 の行と ANSI のバイト列を突き合わせると読んだだけで、英語の Windows で日本語の名前を ANSI の API が `?` に替えたとき、いまの行（`?` は `\u003F` の正規表現になる）が偶然合うかは試していない。「見込み」と書いてあっても `blocks_legit: false` は言い切りで、`basis: derived` の根は premises の 1 行（確かさ 2）にしか無い。
- 直し方の案: D9 に ACP 65001 の扱い（旗なし・往復で表せるかを確かめる）を足して IMP-04 の unknowns と R2 の legit に書く。表せない名前の止め方は、いまの行で通る場合が無いと確かめるまで `blocks_legit` を `unknown` 相当に書き、basis を `guess` にする。
- 処置: fixed: R2 を blocks_legit: unknown・basis: guess にし、legit にいまの行が偶然合う場合を試していないことと 65001 の PC を書いた。65001 の筋書き S1 を足し、IMP-04 の text・unknowns・options に 65001 と WC_NO_BEST_FIT_CHARS を書いた。lost の 3 件目を guess にした。design D9 の旗の扱いは reading では直さず open に出した

## R7 — 「英字だけ／英字でない」の境目が定義されておらず、既存の試験と今の行の書き方とぶつかる
- 対象: impact.dependencies（D6〜D8 に無い）/ targets[T5] / scope.preconditions[2]
- 種類: 依存の見落とし
- 指摘:
  - 仕様は「英字でない文字を含むときは文字どおりの行、英字だけの名前の行はいまと同じ」（spec.md:82）。tasks 5.1 も同じ書き方で、境目（ASCII か・Unicode の英数字か・記号を含むか）を決めていない。
  - いまの `repair_ident_contents` は `is_alphanumeric()` で分けており、`足` は生のまま正規表現に入る。既存の試験 `crates/installer/src/lib.rs:1349-1356` は `alice.b`・`a^b c`（ASCII の記号）に `\u002E`・`\u005E` の行を期待している。「英字でない」を字面どおりに実装すると、`.`・`-`・空白を含むふつうの ASCII の名前（`john.doe`、CI の `ashiato-ci-é` の `-` も）まで文字どおりの行に変わり、この試験と「壊してはいけないもの」7 が落ちる。「非 ASCII を含むとき」と実装すれば試験は残るが、spec・scope.preconditions の言葉とは違う。
  - この読み手（lib.rs の既存試験と、ASCII の記号入りの名前の管理者）が dependencies に無い。
- 直し方の案: spec と tasks に境目を「名前に非 ASCII を含むとき」と書き、`alice.b`・`a^b c` の行が変わらないことを Scenario か試験に足し、dependencies に載せる。
- 処置: fixed: 依存 D9（lib.rs の alice.b・a^b c の試験と壊してはいけないもの 7。E13）と IMP-05（medium・derived。境目が仕様に無い）を足し、open に出した。spec と tasks の境目は reading では直さない

## R8 — IMP-04 と recovery.can は、CI が通れば日本語の名前も確かめたと読めるが、CI が通るのは `é` だけである
- 対象: impact.findings[IMP-04].text / options、situations[recovery].can、acceptance[3]
- 種類: 言い切り
- 指摘:
  - 文字どおりの行にした理由（design D9 R24）は、cp932 の 2 バイト目に `\`（`表`＝`95 5c`）が入ること。しかし PostgreSQL まで届く確かめは `ashiato-ci-é`（1252 で `e9`）だけで、2 バイト目が `\` になる日本語の名前は単体の試験が行のバイト列を比べるだけ。PostgreSQL がその行で認証するかは、どこにも走らない。
  - recovery.can の「日本語の利用者名でも…対応表の行を書く」と acceptance[3] の「英字でない名前の管理者の repair が通る」は、緑の CI でこの残りが消えたように読める。IMP-04 の unknowns・options・scope.residual も、「`é` で通れば足りる」としか書かない。
  - IMP-04 の options 2（CI の Scenario を外す）に、そのあいだ安全と言える手当てが無い。
- 直し方の案: IMP-04 に「CI は 1252 の `é` だけ。日本語（2 バイト目が `\`）の認証は走らない」を unknowns と residual に書き、acceptance[3] と recovery.can の範囲を `é` に限る。
- 処置: fixed: recovery.can を CI で PostgreSQL まで通すのは é だけ・日本語の名前は行のバイト列の比べだけ、に限った。acceptance の 4 件目に日本語の名前の認証は確かめないことを書き、IMP-04 の text・unknowns と options 2（その間の手当ては無い）に書いた

## R9 — mixed.can の「UTF-8 で書く子も窓のコード頁で書く子も読める」が、D8 自身の反転条件と合わない
- 対象: impact.situations[mixed].can / cannot
- 種類: 言い切り
- 指摘: D8 の反転条件は「OEM や ANSI のコード頁で書く子が見つかったら」「UTF-8 として正しい cp932 の出力が見つかったら」順を替えると書き、`psql`・`pg_ctl` の lc_messages は窓のコード頁と同じか UTF-8 でなければ置換の文字になる（design D8 の最後から 2 つ目）。つまり読めない組み合わせは分かっているのに、mixed.cannot は「conhost 以外」しか挙げず、can は限定なし。coverage.not_examined の psql・pg_ctl の行と、この can/cannot が結ばれていない。
- 直し方の案: mixed.can に「この PC で調べた tasklist・reg.exe・sc.exe・whoami」と範囲を付け、cannot に「psql・pg_ctl の lc_messages が窓のコード頁でも UTF-8 でもない組み合わせは置換の文字になる」を書く。
- 処置: fixed: mixed.can を「UTF-8 で書く子と、窓のコード頁で書く子なら読める」に限り、mixed.cannot に lc_messages の違う psql・pg_ctl は置換の文字になることを書いた
