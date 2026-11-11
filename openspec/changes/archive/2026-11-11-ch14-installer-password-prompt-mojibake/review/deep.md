# CH14 深掘りの問いの独立レビュー（deep-review）

対象: `openspec/changes/ch14-installer-password-prompt-mojibake/deep-questions.json`（Q1〜Q4。4 問とも `kind: open`・推奨あり・`loss` なし）と `premises.md`（7 行）。
手順 1〜5 は依頼（`docs/changes/CH14.md`）・正典（`openspec/specs/distribution/spec.md`）・コード（`crates/installer/src/`）・試験（`tools/release/install-check.sh`・`.github/workflows/ci.yml`）・rpassword 7.5.4 の source だけで先にやり、そのあとで問いの一覧と突き合わせた。問いの JSON は触っていない。
走らせたのは grep・`scripts/upstream_misses.py report` と、WSL から本人の PC の `sc.exe` / `icacls.exe` / `whoami.exe` / `chcp.com`（管の出力のバイト列を読む）だけ。Windows のコンソールの上では何も走らせていない。

## 手順ごとの確かめ

- **手順 1（要件どうしの衝突）**: CH14 は `satisfies: []`。触る正典の Requirement 3 つを読んだ —「インストールの最後に、端末で画面の合言葉を決め…」（spec.md:1773-1781）・「画面の合言葉を忘れたら…」（:1809-1814）・「DB はあるのに秘密が無くて止まったら…repair」（:1733「インストールの最後と同じ検査で画面の合言葉を決めさせる」）。
  依頼の 2 点（問いを WriteConsoleW の経路で出す・条件を最初の問いの前に示す）は「端末に表示せず」（:1777）・「端末でないときは標準入力の 2 行を読む」（:1779）・「理由を示して入れ直させる」（:1776）のどれとも衝突しない。条件を前に示すのは振る舞いの追加なので Requirement の MODIFIED が要るが、衝突ではない。repair が 3 つ目の口であることを問いが見落としている → R1
- **手順 2（扉の幅）**: 該当なし（確かめた範囲: CH14.md に doors の欄が無く、change は Story を持たない。触る Requirement の「導出元」（FR-111・深掘り Q3・design D23）に、この change の文脈で幅の残る扉は無い）
- **手順 3（新たに立つ一方通行）**: 該当なし（確かめた範囲: 合言葉の照らすための値の作り方 `password.rs:53-58`（Argon2id・UTF-8 のバイト）と検査 `password.rs:28-50` は変えない。設定のファイルの形 `secrets.rs:38-62` も変えない。直すのは表示の経路と試験だけで、捨てる・変換して凍る・外に出す新しいものは無い。配った版の exe は既に外に出ているが、この change の選択で新しく出るものではない）
- **手順 4（日常に影響する選択）**: 該当なし（確かめた範囲: インストーラはインストール・`password`・`repair`・`uninstall` のときだけ開く。CI の Windows の分の増加は Q4 の context に書いてある）
- **手順 5（既存コードが要件を満たしていない箇所）**: 依頼の原因は source で確かめた（rpassword `lib.rs:275-292` が問いを `CONOUT$` へ `write_all`、`windows.rs:16`）。そのうえで:
  (a) `repair` は `install()` を通って合言葉を聞く（Q1 は「聞かない」と書く）→ R1。
  (b) 子の命令の出力を `from_utf8_lossy` で読むのは `output_tail` だけでなく、値として使う口（`whoami` の利用者名 → repair の pg_ident、`reg.exe` のダウンロードの場所）もある → R2。
  (c) `output_tail` の化けは本人の PC で実際に起きる（`sc.exe`・`icacls.exe` の管の出力は cp932）→ R3。
  (d)「入れた合言葉を端末に表示せず」（spec.md:1777）は、本物のコンソールで一度も確かめていない（CI は管で流す）→ R5
- **手順 6（過去の取りこぼし）**: 「拾えたはずの観点」を 1 つずつ当てた。当てはまったのは 3 つ。
  「OS のコマンドを情報源にする決定では、出力を実機で確かめ、前提欄に書く」→ 子の命令の出力の符号ページ（R2・R3）。
  「検証コマンドが 0 本でも通る」→ 窓の試験を足すときの数え方（R5）。
  「既存関数の出力に乗せる Scenario は、例を通して成り立つことを確かめる」→ runner で cp932 を再現する前提（R6）。
  時計・保持期限・送り先・削除・移行・許諾の観点は、表示と試験だけのこの change には当たらない
- **手順 7（族の網羅）**: Q1 の family（「日本語の窓でインストーラの文字が化ける口」）の兄弟を数え直した。
  repair の口は「聞かない」と誤って決まっている（R1）。子の命令の出力を値として使う口が無い（R2）。標準出力を向け替えた窓で問いが消える場合が無い（R4）。窓で入れた文字が出ないことの確かめが無い（R5）。CI が通らない昇格して開き直した窓（本人が化けを見た窓）が試験の置き場の問いに出ていない（R6）
- **手順 8（前提の確かさ）**: premises.md はある。`loss` のある決定は無い（4 問とも open）ので、確かさ 4 未満の前提に `loss` が乗る指摘は無い。
  6 行目（子の出力が cp932 で化ける。確かさ 2）は本人の PC で走らせて確かめた → 4 に上げられる（R3）。
  7 行目（runner の窓の符号ページ。確かさ 1）は Q4 の推奨に乗るが、検出に 932 は要らない（R6）。
  4 行目（Rust の標準出力は WriteConsoleW）は確かさ 2 だが、本人が同じ窓でほかの行が化けないのを見ている（依頼の逐語）。低く付けているだけで、決定を誤らせる向きではない

## R1. `repair` も合言葉を聞く。Q1 の family と context の「repair は聞かない」はコードと合わない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/deep-questions.json
- 根拠: `crates/installer/src/main.rs:1475`（repair は `secrets::new_install_secrets()` で作り直す）/ `crates/installer/src/secrets.rs:138`（`web_password_verifier: String::new()`）・`:58`（それを設定に書く）/ `main.rs:1539`（repair の最後に `install(…)` を呼ぶ）/ `main.rs:1193-1199`（install は照らすための値が空なら `read_web_passwords` を呼ぶ）/ `tools/release/install-check.sh:1072`（`install_with "$PW1" repair` —— 試験も repair に合言葉を 2 行渡している）/ spec.md:1733「インストールの最後と同じ検査で画面の合言葉を決めさせる」・:1745 / `docs/install.md:25`「画面の合言葉を作り直します」/ 依頼の逐語「password / repair など合言葉を聞く他の口も同じ」
- kind: premise
- 提案: Q1 の family の兄弟「repair の口」の decided と context の「合言葉を聞く口は install の最後と password の 2 つで、repair は聞きません」を「repair も install の最後を通って聞く（同じ `read_web_passwords`）」に直す。直し方は同じ関数 1 か所で 3 つの口に効くが、spec の段で条件の表示を Requirement に足すときは repair の Requirement（:1733）も同じ検査を指していることを写す。docs/install.md:25・31 の「管理者の端末で repair」はインストーラを昇格せずにその窓で動かす道（`main.rs:1610`）なので、その窓も試験か確認バッチの範囲に入れる（R6）
- 処置: fixed deep-questions.json — （Q1 の family・context・選択肢を「install・password・repair の 3 つの口が同じ read_web_passwords を通る」に直した。昇格済みの端末の窓は Q4 の確認バッチの範囲に入れた）

## R2. 子の命令の出力を、表示でなく値として使う口がある。Q2 は表示（失敗の理由）だけを問うている
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/deep-questions.json
- 根拠: `crates/installer/src/main.rs:250`（`whoami` を `from_utf8_lossy` で読む）→ `:1456`（repair の `account`）→ `:1490`・`commands.rs:1097-1117`（`repair_ident_contents`。`U+FFFD` は英数字でないので `\uFFFD` になり、SSPI が渡す本当の名前と一致しない）/ `main.rs:71`（`reg.exe` のダウンロードの場所を `from_utf8_lossy` で読む）→ `:1588` → `:1182`・`:593-602`（場所が化けると `ensure_real_directory` が読めずにインストールが止まる）、設定の `ASHIATO_DOWNLOADS_DIR`（`secrets.rs:54`）にも入る / `main.rs:380`（`tasklist` の名前。表示だけ）/ 本人の PC で WSL から `sc.exe` と `icacls.exe` の管の出力を読むと cp932 のバイト列だった（R3）。`whoami.exe` は本人の利用者名が ASCII（`<PC 名>\<利用者名>`）なので化けない
- kind: technical
- 提案: Q2 に兄弟として「値として使う口」（repair の利用者名・ダウンロードの場所）を足し、表示の口（`output_tail`・`tasklist`）と分けて書く。context の「インストールが成功する限り出ません」は値の口には当たらない —— 利用者名が ASCII でない PC では repair が DB に入れず止まり、spec.md:1741 のとおり残る手は DB の置き場を消して入れ直すことになる、と選択肢「含めない」の detail に書く。本人の PC は ASCII なので今は当たらない、も添える。含めないなら、別の change に出す先を proposal の Impact でなく `hx change` で立てるかを決める
- 処置: fixed deep-questions.json — （Q2 を表示の口と値の口の 2 種類に分け、選択肢「値の口だけ含める」を足した）

## R3. `output_tail` の化けは「見込み」でなく、本人の PC で起きることを確かめた
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/premises.md
- 根拠: WSL から `/mnt/c/Windows/System32/sc.exe query ashiato-nonexistent-xyz` の管の出力は cp932 で `指定されたサービスはインストールされたサービスとして存在しません。`、UTF-8 として置換付きで読むと `�w�肳�ꂽ�T�[�r�X�̓C���X�g�[�����ꂽ…`。`icacls.exe 'C:\nonexistent-ashiato-xyz'` も同じ（`0 �̃t�@�C��…`）。`chcp.com` は 932 / `crates/installer/src/main.rs:81-90`（`output_tail` は `from_utf8_lossy`）・`:106`・`:159`・`:892`・`:922` が失敗の行に添える / 既に同じことを ST05 が実測している: `openspec/changes/archive/2026-10-05-st05-clock-skew/design.md:198`（「出力はコンソールの符号ページ（cp932）のバイト列で、`from_utf8_lossy` では見出しが化ける」）
- kind: premise
- 提案: premises.md の 6 行目を確かさ 4（確かめ方: 本人の PC で sc.exe・icacls.exe の管の出力を cp932 と UTF-8 で読み比べた）にし、Q2 の why「実地ではまだ見ていません」と context「なる見込み」を「なる（本人の PC で確かめた）」に直す。Q2 の選択肢に、直し方の候補（`MultiByteToWideChar(CP_OEMCP)` か、子に UTF-8 で書かせる）が子ごとに効くか（`psql`・`pg_ctl` は符号ページでなく自分の lc_messages で書く）は spec の段で確かめる、と添える
- 処置: fixed premises.md — （6 行目を確かさ 4・本人の PC で確かめた、に直した）・deep-questions.json（Q2 の why）

## R4. 問いを標準出力へ移すと、標準出力を向け替えた窓では問いが見えなくなる。どの流れに書くかが問いに無い
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/deep-questions.json
- 根拠: いまは rpassword が標準出力に依らず `CONOUT$`（窓そのもの）へ書く（`~/.cargo/registry/src/*/rpassword-7.5.4/src/lib.rs:283`・`src/windows.rs:16`）。インストーラが窓かどうかを見るのは標準入力だけ（`crates/installer/src/main.rs:286`）。標準入力が窓で標準出力をファイルへ向けた場合（昇格済みの管理者の端末で `ashiato-setup.exe repair > log.txt` など。`main.rs:1610` で起こし直さずにその窓で動く。docs/install.md:14・25 は管理者の端末で動かす手順）、問いと条件の一覧はファイルへ行き、本人は何も出ない窓に合言葉を打つ。spec.md:1775「インストーラの端末で画面の合言葉を 2 回入れさせ」/ 依頼の逐語は「標準出力（WriteConsoleW を通る経路）」で、流れまでは決めていない
- kind: technical
- 提案: B（推奨つき）の問いか、聞かない既定として 1 行足す。選択肢は「標準出力」「標準エラー」「窓（`CONOUT$`）へ `WriteConsoleW` で直に書く」。どれも後から直せば戻る。あわせて、Rust の標準出力は行ごとにまとめて書くので、改行の無い問いは読みの前に `flush` が要ることを design への申し送りに書く（無いと問いが入力の後に出る）
- 処置: fixed deep-questions.json — （Q5「問いの文をどこへ書くか」を足した。推奨は窓へ直に書く。flush の申し送りは spec の段の design に書く）

## R5. 「入れた合言葉を端末に表示しない」は、本物の窓で一度も確かめていない。Q4 の試験の中身に入っていない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/deep-questions.json
- 根拠: spec.md:1777「入れた合言葉を端末に表示せず」/ 試験はどれも管で流す（`tools/release/install-check.sh:597-598`・`:973`・`:994`）ので `main.rs:286` の `is_terminal()` が偽になり、窓で読む道（rpassword の `SetConsoleMode(ENABLE_PROCESSED_INPUT)`、`windows.rs:377-386`）を通らない。Scenario「合言葉の元の値は、どこにも残らない」の「インストーラの端末の出力」（install-check.sh:1004）も管の出力を見ている。この change は rpassword の入口を `prompt_password` から読みだけの関数へ替えるので、エコーを止める道が壊れても今の試験は気づかない / 試験の数え方: install-windows は `--list` と実際に出た `== Scenario:` を突き合わせる（ci.yml:452-460）
- kind: technical
- 提案: Q4 の context か推奨の detail に、窓の試験で確かめる中身を書く —— 問いの文字（化けない）・条件の一覧・断りの行（標準エラー）・**打った合言葉が窓に出ないこと**。窓の試験の Scenario 名を `install-check.sh` の `scenario_names`（`--list`）に足し、0 本で通らない形にする、と添える
- 処置: fixed deep-questions.json — （Q4 の context に「入れた合言葉が窓に出ないこと」を確かめる中身として足した。--list の Scenario 名は spec の段で tasks に書く）

## R6. Q4 の推奨は「runner で窓の符号ページを 932 にする」（確かさ 1）に乗るが、検出には 932 は要らない。CI は本人が化けを見た「昇格して開き直した窓」を通らない
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/deep-questions.json
- 根拠: premises.md 7 行目（確かさ 1・確かめていない）/ 化けの原因は UTF-8 のバイトを 65001 以外の符号ページで読むことなので、runner の既定（英語の OS）のままでも直す前は化け、直した後は化けない。932 は「日本語の Windows と同じ」に寄せる意味しか無い / CI の runner は管理者として動くので `crates/installer/src/main.rs:1610`（`elevation_count(is_elevated()) == 1`）で起こし直さず、新しい窓（本人が見た窓）を開かない。install-check.sh も `"$SETUP" password` を直に呼ぶ（:974）/ ConPTY が runner の step から使えることも premises に無い
- kind: premise
- 提案: premises.md 7 行目の決定への依り方を「試験の検出は符号ページに依らない（65001 以外なら化ける）。932 にするのは本人の窓に寄せるため」に書き直し、ConPTY が runner で使えるかを 1 行足す（確かめるまで確かさ 1〜2）。Q4 の選択肢「Windows の CI だけ」の detail に「昇格して開き直した窓（本人が化けを見た窓）と、昇格済みの管理者の端末で直に動かす窓（R1 の repair）は、機械はどちらも通らない」と書く
- 処置: fixed premises.md — （7 行目を「検出は 65001 以外なら化けるので 932 に依らない」に直した）・deep-questions.json（Q4 の context と推奨の detail に、CI は昇格して開き直した窓を通らないことを書いた。ConPTY が runner で使えるかは spec の段で確かめる）

## R7. 条件の一覧の文字（16 文字・change-me）を、検査の定数と別に書くと再びずれる
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/deep-questions.json
- 根拠: `crates/installer/src/password.rs:4-5`（`MIN_LEN = 16`・`PLACEHOLDER = "change-me"` は非公開の定数）/ 依頼そのものが一度「12 文字」と誤って登録された（`docs/changes/CH14.md:15`「2026-11-09 訂正: 登録時の「12 文字」は誤り」、commit `ef5055778`）/ Q3 の推奨は一覧の文と断りの「（16 文字以上）」を新しく書く
- kind: technical
- 提案: Q3 の推奨の detail に「一覧と断りの文は `password.rs` の定数から組み、単体の試験で `MIN_LEN` と表示の数が合うことを見る」と添える（聞かない既定でよい。後から直せば戻る）
- 処置: fixed deep-questions.json — （Q3 の context に「一覧の数字と語は password.rs の定数から組む」を聞かない既定として書いた）

## R8. Q1 は依頼の逐語で既に決まっていて幅が無い
- 成果物: openspec/changes/ch14-installer-password-prompt-mojibake/deep-questions.json
- 根拠: `docs/changes/CH14.md:21`（依頼の逐語が「問いを標準出力で出し、rpassword は読みだけにする」と「合言葉の要件を最初の問いの前に示す」の両方を名指しし、検査の中身（16 文字以上ほか）は「訂正」で確かめ直したうえで変えないと読める）/ Q1 の選択肢 2（条件の表示を別の change に）は依頼に逆らい、選択肢 3（検査も見直す）は依頼の外。どれを選んでも失うものは無い（premises.md 末尾）
- kind: defer
- 提案: Q1 を問いから外し、依頼どおりの 2 点を聞かない既定（C）として deep.md に写す。Q1 が持つ family は Q2 へ移す（R1・R2・R4〜R6 で兄弟を直したうえで）。人間に残す問いは Q2〜Q4（と R4 を問いにするならその 1 つ）
- 処置: rejected: change の第 1 回は範囲を問う決まり（.claude/skills/story-upstream/SKILL.md:51「第 1 回で範囲（何を変え、何を変えないか）を問う」）。選択肢 2・3 は依頼から外れる側だが、本人が依頼の後で範囲を狭める・広げる余地を残すため問いに残す。推奨は依頼どおりで、触らなければ依頼どおりに進む
