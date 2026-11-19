# CH22 spec の独立検査（spec-review・`--draft`）

対象: `openspec/changes/ch22-soak-install-online-ci/`（proposal・design・tasks・specs/distribution・deep.md・deep-questions.json・premises.md・review/deep.md）、
`docs/changes/CH22.md`、正典 `openspec/specs/distribution/spec.md:2515-2579`。`--draft` なので deep.md の推奨を答えとして読んだ。

## 機械の検査

- `openspec validate ch22-soak-install-online-ci --strict` → `Change 'ch22-soak-install-online-ci' is valid`（rc=0）
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0）
- `python3 scripts/check_scenarios.py . ch22-soak-install-online-ci` → `scenarios: FAIL (担保なし 21 件 / 名無しの確認待ち 0 件)`。
  21 件は全部この change の新しい Scenario で、実装の前なので担保が無いのは当たり前。Task 5.1 が閉じる。この件では指摘を立てない。
  tasks.md の印の名前（自己試験 15＋2＋2・実地 2）と spec の 21 本の `#### Scenario:` は、字面が全部一致する（突き合わせて確かめた）
- `python3 scripts/review_triage.py . ch22-soak-install-online-ci` → `triage: OK`（指摘 13 件 / 仮決め 1 件 / 要件へ戻すもの 0 件）
- `python3 scripts/spec_overlap.py candidates . ch22-soak-install-online-ci` → `[]`

## 観点 1: deep の決定が正典に写っているか

- Q1（PC の収集アプリを設定せずに置き、1 回起こし直す）→ Requirement「置く段は、PC の収集アプリを設定せずに起こし…」と、その 3 本の Scenario。写っている
- Q2（1 回でも起こし直したら落とす・置き終わりに実行中で healthz が 200）→ Requirement「置く間にサービスが…」と 4 本の Scenario。写っている
- Q3（同じ誤りが 3 回以上）→ 数の 3 が Scenario に無い。R4 を見る
- Q4（確かめた行が 1 行は要る・上限だけなら警告）→ 4 本の Scenario。写っている
- Q5（消す → 外す → 入れる）→ Scenario「置く段は、前の段の入れたものと…」。字義の古い順（入れた後に外す）はどこにも残っていない
- Q7（段を緩めない）→ Requirement の本文「外れを許す一覧を持たない」と design D12。Scenario は無い（R7）
- MODIFIED（呼び出し元の特に見るところ (1)）: 正典の元の Requirement と `diff` した。違いは 3 か所だけだった
  - 最後の SHALL が 2 文に分かれた
  - 導出元に CH22 の 1 段落が足された
  - Scenario「本物の配布元へ繋ぐ自動の試験は、握手の試験と置く段だけである」が足された
  - ほかの 6 つの SHALL、ほかの導出元の段落、既存の 9 本の Scenario は、1 字も変わっていない
  - ただし、最後の SHALL の中身は「外への接続」から「更新係の外への接続」に狭まった（R12）

## R1. Task 2.2 の actionlint の検証は、この change が触る前の ci.yml でもう落ちる
- 成果物: openspec/changes/ch22-soak-install-online-ci/tasks.md
- 根拠: tasks.md:70 のコマンドを、手を入れていない今の木でそのまま走らせた。結果は rc=1（`actionlint: command not found` の後に docker の `rhysd/actionlint:1.7.7`）
  - `.github/workflows/ci.yml:487:9: shellcheck reported issue in this script: SC2155:warning:7:9`
  - 同じ形の `:12:10` がもう 1 件
  - 487 行は package-windows の job の手順で、この change の触る範囲の外にある
- kind: technical
- 提案: 落ちる道が 2 つある。下流が範囲の外の手順を直すか、検証が通らないまま Task が止まるか。どちらかを選ぶ:
  - 新しい指摘だけを見る形にする（base の出力との差を見る・`-ignore 'SC2155'`・`-shellcheck=`）
  - 487 行の直しを触ってよい範囲として明記する
- 処置: fixed tasks.md 2.2 — actionlint を `-ignore SC2155` で走らせ、範囲の外の既存 2 件（`ci.yml:487`）だけを除き、ほかの指摘は新しいものとして落とす形にした

## R2. Task 4.1 は Task の順で走れない。検証が CI の run を見ず、CH22 の完了の判定の P2・P7 を確かめない
- 成果物: openspec/changes/ch22-soak-install-online-ci/tasks.md
- 根拠:
  - tasks.md:82-87 の 4.1 は「PR の CI の install-windows の置く段の出力を読み」と書き、終了の条件を「PR の CI の install-windows が、置く段を含めて緑」としている
  - 4.1 の後に Task 5 が続く。一方で tasks.md:96-98 の「人間の確認待ち」は「なし」
  - 検証（tasks.md:86）は premises.md の P8・P11 の行の末尾が `| [2-5] |` かを grep で見るだけ。数字を書き換えれば、run を 1 度も読まずに通る（run の番号も要らない）
  - CH22.md の「完了の判定」は、起こし直しの回数（P2）と、PC の収集アプリが確かめたか（P7）の記録も挙げている。4.1 の本文は 4 つを名指しするが、検証が見るのは 2 つだけ
  - Task 2.2 の「終了の条件: PR の CI の install-windows が緑」（tasks.md:72）も、コマンドを持たない
- kind: technical
- 提案: 4.1 を PR の後の工程の Task として書き分ける（誰が・いつ走らせるか。CI を待つ・ci-fix の席など）。検証は次の 2 つを見る形にする:
  - premises の出所の欄に run の番号がある
  - `gh run view <番号> --log` に実地の 2 つの `== Scenario:` の行がある
  - P2・P7 の行も P8・P11 と同じく確かめる
  - ただし、検証に `gh` を呼ぶ Task には、今のグラフでは誰も `[x]` を付けず、merge_gate が落ちる（CH11/CH13 の gate_help の実例）。
    `gh` を使うなら、付ける席を名指す。使わないなら、4.1 を「人間の確認待ち」（CI の run を読む工程）に移し、そう書く
- 処置: fixed tasks.md 4.1 — 「PR の CI の後の工程」と題に書き、走らせる席（CI を待つ段・ci-fix）を名指しした。検証は P2・P7・P8・P11 の 4 行の出所に `run <番号>` があることと、P8・P11 の確かさが 2 以上であることを見る（`gh` は使わない。run の番号を書かずには通らない）。人間の確認待ちの節にもその旨を書いた

## R3. D4・D6・D7 の反転条件の処置は、D12・Q7・完了の判定と食い違う。どれも CH22 自身の Requirement の書き直しを要する
- 成果物: openspec/changes/ch22-soak-install-online-ci/design.md
- 根拠:
  - design.md:72-73（D6）と design.md:81-82（D7）は、反転したら「…を直す change を立てる」と書く
  - 一方で design.md:111-115（D12）・tasks.md:85 は、段の側の原因なら「台本を直す」と書く
  - CH22.md の完了の判定は「install-windows が置く段を含めて緑」で merge する（Q7）
  - CH22 が merge する前は、その Requirement は正典に無い。別の change からは直せないので、最初の走りで反転に当たると CH22 が止まったままになる
  - 反転の処置には、spec の本文を書き直すものがある:
    - D7: PC の収集アプリの「確かめた」の判定を外す。Requirement「置く間に更新係と PC の収集アプリが…」の SHALL（spec.md:115）と、Scenario「配る形の PC の収集アプリは、設定せずに起こし直すと本物の配布元に確かめ、動き続ける」（spec.md:167-171）を書き直す
    - D4: システムの記録を読まない。spec.md:55 の IF が「サービスの異常な終わりの記録（Windows のシステムの記録）」を名指ししている。D4 は「自己試験の Scenario は変えない」とするが、読まないなら本文が偽になる
  - 加えて、P7 は通知の差出人の登録を確かめていない（P7 の確かめ方は ci.yml:230 だけ）。登録に失敗すると、PC の収集アプリは警告の行 `outage_notice_unavailable` を書き、確かめる係を付けない（crates/collector-windows/src/main.rs:222-253）。この読み方が、premises と Task 4.1 のどちらにも無い
- kind: technical
- 提案: 反転の処置を「CH22 の中で spec・design・tasks を直す」に揃え、そのとき合意を引き継ぐかどうかを書く（仮の D だけか、Requirement の本文を変えるか）。D7 の反転を判定する観測（`collector.log` の `outage_notice_unavailable`）を、premises の行と Task 4.1 の記録に足す
- 処置: fixed design.md — D15 を足し、反転の処置を「CH22 の中で spec・design・tasks を直す」に揃えた（D と定数だけなら合意を引き継ぎ、Requirement の本文か Scenario を直すなら合意を聞き直す）。D4・D6・D7・D12 の反転条件を D15 に寄せ、D7 に `outage_notice_unavailable` の読み方を、premises P7 と Task 4.1 に記録の対象を足した

## R4. Q3 の推奨の数「3 回」が Scenario に無い。正典の本文は、archive の後に残らない design の D 番号を指す
- 成果物: openspec/changes/ch22-soak-install-online-ci/specs/distribution/spec.md
- 根拠:
  - deep.md の Q3 の答えは「同じ誤りが 3 回以上続いたら落とす」
  - spec.md:90 は「決まった回数（置く段の台本の定数。design D6（仮））」と書き、Scenario の spec.md:95-111 も「決まった回数」とだけ書く。台本の定数を 10 にしても、どの Scenario も落ちない
  - spec.md:149 も「決まった時間（design D7（仮））」と書く。`openspec archive` は main specs しか更新しないので、正典の本文が、そこに無い設計を指すことになる
  - D11 はこの形を意図して選び、note の「聞かずに決めたこと」に書いてある。ただ、本人が Q3 に推奨のまま答えると、3 は本人の答えになる。それでも D6 が「（仮）」のままだと、後で数を変えても合意の聞き直しが起きない
  - `SOAK_SAME_ERROR_LIMIT=3`（tasks.md:20）を確かめる検証も無い
- kind: technical
- 提案: 次のどちらかにする。どちらでも、正典の本文から「design D6（仮）」「design D7（仮）」を外す:
  - 3 を Scenario に書く（3 行で落ちる・2 行で通る）
  - D11 を残すなら、D6 の 3 を本人の答えの写しとして仮の印から外し、Task 1.1 に定数の値を見る検証を足す
- 処置: fixed D11 — Q3 の答えの数 3 を Requirement と Scenario に書き（3 行で落ち・2 行で通る）、正典の本文から design の D 番号への参照を外し、答えでない閾値は「台本の定数」と書いた（specs/distribution/spec.md）。D11 を「深掘りの答えの数は正典に書く」に改め、Task 1.1 に `SOAK_SAME_ERROR_LIMIT=3` を見る検証を足した

## R5. 「置く段の始まり」の定義が spec に無い。数える窓の境（段の始まりから healthz が 200 になるまで）を、どの Scenario も見ない
- 成果物: openspec/changes/ch22-soak-install-online-ci/specs/distribution/spec.md
- 根拠:
  - 「置く段の始まり = 塞ぎの規則を消す直前の時刻」は design.md:46（D3）にだけある。spec は「置く段の始まり」を 17 か所で使うが（`grep -c`）、どこにも定義しない
  - 確かめた記録の判定はこの時刻で決まる。更新係は入れた直後に確かめる（D5、design.md:60）ので、始まりを入れ終わりと読めば、正常な run でも赤になる
  - 誤りの行と起こし直しを数える窓は「healthz が 200 になってから 5 分」（D5、spec.md:54）
  - spec.md:55 の 1 つの IF に「置く間に」と「置く段の始まりより後に」の 2 つの窓が混ざっている
  - Scenario（spec.md:80-84・107-111）が置く行は「置く段の始まりより前」だけ。始まりと healthz の間（入れる最中の DB の起動の行）を数えないことを見る入力が無い。段の始まりで切る台本でも通る
- kind: technical
- 提案: 第 1 の Requirement の本文に「置く段の始まり」を観測できる形で定義する（塞ぎの規則を消す直前）。誤りの行の Scenario に、段の始まりと healthz の 200 の間に書かれた行を数えない入力を足す
- 処置: fixed specs/distribution/spec.md — 第 1 の Requirement に「置く段の始まり」（塞ぎの規則を消す直前）と「置く間」（healthz が 200 になってから 5 分）を定義し、何をどちらの窓で数えるかを書いた。サービスの異常な終わりの記録も置く間で数える形に揃えた。Scenario「置く段の始まりと置く間の始まりの間に書かれた誤りの行は数えない」を足した

## R6. 緑の run からは P11 が分からない。システムの記録を読めたか・何件読んだかを出す約束が無い
- 成果物: openspec/changes/ch22-soak-install-online-ci/specs/distribution/spec.md
- 根拠:
  - premises.md P11 は確かさ 1。design.md:54 の反転条件は「runner で wevtutil が読めない・7031/7034 が出ないと分かったとき」
  - 正常な run では 7031/7034 は 0 件のはずで、「読めて 0 件」と「読めなかった」が出力で見分けられない
  - spec.md:55 の SHALL が出させるのは「どのサービスが何回起こし直された・止まったか」だけで、読めなかったときに落とす・出す約束も無い
  - Task 4.1 は P11 を記録させるが、記録の元になる出力が無い
- kind: technical
- 提案: 次の 2 つを足す。どちらも D4 の仮の扱いのままでよい:
  - 置く段は、システムの記録を読んだ結果（読めた / 読めない・置く段の始まりより後の件数）を、落ちない回にも 1 行出す（Requirement の本文と Scenario）
  - 読めないときに落とすか警告にするかを決める
- 処置: fixed specs/distribution/spec.md D4 — システムの記録を読めたか・置く間の件数を落ちない回にも 1 行出し、読めなければ警告にして状態と番号だけで判定する SHALL と Scenario「システムの記録を読めなければ、警告を出し、状態とプロセスの番号で判定する」を足した（D4（仮））

## R7. Requirement の本文だけが言い、Scenario が言っていないことがある
- 成果物: openspec/changes/ch22-soak-install-online-ci/specs/distribution/spec.md
- 根拠:
  - spec.md:7「前の段が入れたものを消せない…THEN 配る zip を入れずに置く段を落とす」: Scenario は塞ぎの規則の側（spec.md:29-33）だけ。消した後に `ashiato-server.exe`・状態の置き場の `settings`・`status.json` が残る場合（D2、design.md:37）が無い
  - spec.md:10「外れを許す一覧を持たない」と spec.md:11「送り先を設定せず、配布元の答えの形を見本と比べない」: どちらも Scenario が無い（Q7・依頼の「やらないこと」の担保）
  - spec.md:88 の「サーバの日付のログは…panic の行」: panic の行を数える入力がどの Scenario にも無い。DB のログのプロセスの番号 `[1234]` を除く規則（D6、design.md:66）も同じで、spec.md:89 は「日時を除いた」とだけ書く
  - spec.md:117「ほかの理由…の行がある」: 上限の行と「届かない」の行が混ざる入力（落ちるべき）が無い。Scenario は「上限だけ」（spec.md:141-145）と「届かない・読めないだけ」（spec.md:135-139）の 2 つで、全部上限かどうかを見ない台本でも、2 つとも通りうる
- kind: technical
- 提案: 次の 3 本を自己試験の Scenario に足す:
  - 消した後に残りがあれば入れずに落ちる
  - 上限と届かないが混ざれば落ちる
  - サーバのログの panic の行と、DB の番号違いの同じ行を数える
  「許す一覧を持たない」「見本と比べない」は検査で見る形（台本に一覧の読み込みが無い・送り先の設定の命令が無い）を足すか、design に置き場を移す
- 処置: fixed specs/distribution/spec.md D12 — Scenario を 4 本足した（消した後に残りがあれば入れずに落ちる・上限と届かないが混ざれば落ちる・プロセスの番号だけが違う DB の行を同じ誤りとして数える・panic の行）。「外れを許す一覧を持たない」「送り先を設定しない」「見本と比べない」は観測できる振る舞いでなく台本の作りの決まりなので、Requirement の本文から外して D12 に移した（code-verify と review が台本を読んで見る）

## R8. 依頼の「panic…を機械が判定する」に対し、プロセスを止めないサーバの panic が 1 回なら段は通る
- 成果物: openspec/changes/ch22-soak-install-online-ci/specs/distribution/spec.md
- 根拠:
  - CH22.md の依頼（逐語）は「サービスの再起動の繰り返し・panic・誤りの記録の連続・クラッシュを機械が判定する」
  - サーバの panic の hook は、誤りの段の行を 1 行書くだけ（crates/server/src/lib.rs:4583-4586 `tracing::error!(kind = "panic", …)`）
  - `Cargo.toml` に `panic = "abort"` は無い（`grep -n panic Cargo.toml crates/server/Cargo.toml` の結果は lint と注記だけ）
  - サーバは `tokio::spawn` で作業を分けている（crates/server/src/lib.rs:4817 など）。分けた作業の中の panic はその作業だけを終え、プロセスは残る（tokio の既定）
  - そのため、主の作業の panic は Requirement「置く間にサービスが…」の起こし直しで捕まるが、分けた作業の panic は捕まらない。後者は「誤りの段の行」として、spec.md:88-90 の「決まった回数（3）以上」に当たるまでは出力に出すだけ（spec.md:91）
  - deep の Q3 は panic を分けて問うていない。review/deep.md R8 の処置は、更新係と PC の収集アプリに hook が無いことを理由に「panic はどこでも落とす」を外した。hook のあるサーバまで一緒に外れた
- kind: technical
- 提案: サーバのログの `kind="panic"` の行は 1 行で落とす（D6 の仮に足す）か、1 回は落とさない理由を design に書いて依頼との差を明示する。どちらでも Scenario を 1 本足す
- 処置: fixed D6 仮 — サーバのログの panic の行は 1 行で落とす SHALL と Scenario「サーバのログの panic の行は 1 行でも落ちる」を足した。反転条件は D6 の数え方と同じ（D15）

## R9. 自己試験で実地の口を Linux で走らせる継ぎ目を、tasks が指していない。実地の口が判定を全部呼ぶことを、どの検証も見ない
- 成果物: openspec/changes/ch22-soak-install-online-ci/tasks.md
- 根拠:
  - Scenario「置く段は、前の段の入れたものと更新係の置き場を消し、塞ぎを外してから配る zip を入れる」（spec.md:23-27）の WHEN は「置く段を走らせる」。Task 2.1（tasks.md:57）は、偽の命令の記録でこれを見るとしている
  - 写し元には前例がある。update-apply-install.sh の自己試験は、`cd "$work"` の中で `C:/…` の字面を作業の置き場の中の相対の道にし、待ちを `reset_wait_seconds=0` で縮め、偽の `taskkill.exe`・`reg.exe` を置いて `reset_machine` を走らせる（tools/release/update-apply-install.sh:899-915）。
    加えて `declare -f run_scenario` に `reset_machine` があることで、実地の口が掃除を呼ぶことを見る（同 :916）
  - Task 2.1 と Global Constraints はこの継ぎ目を指していない:
    - 偽の命令の一覧（tasks.md:17）に `reg.exe`・`icacls.exe`・zip を開く命令・PC の収集アプリの実行ファイルが無い
    - 実地の口の 150 秒・60 秒・300 秒の待ち（tasks.md:54）をどう縮めるかを書いていない
    - tasks.md:19 は時刻を変える道を禁じているので、待ちの秒数を `reset_wait_seconds` の形で差し替えてよいかを下流が読めない
  - このままだと「順を記録する別の関数」を試す形でも、Scenario が緑になりうる
  - 実地の Scenario「配る形のサーバを…5 分置くと…」（spec.md:46-50）は、THEN に 5 つの主張を束ねる。印は判定の前に 1 回 echo する（tasks.md:55）
    - 実地の口が判定の 1 つ（例: 置き終わりの healthz）を呼び忘れても、`comm` の突き合わせは通る
    - CI の Windows でしか走らないので、Linux の自己試験も気づかない
- kind: technical
- 提案: Task 2.1 に、update-apply-install.sh:899-916 の形を写すことを書く:
  - 相対の道の置き場で掃除を走らせる
  - 待ちの秒数は、自己試験の関数の中からだけ変数で縮める（tasks.md:19 の禁止との関係も書く）
  - 偽の `reg.exe`・`icacls.exe` などを置く
  - 実地の口が Task 1 の判定の関数を全部呼ぶことを、`declare -f` で見る（または、判定ごとに 1 か所だけ壊した入力で実地の口が落ちることを見る）
- 処置: fixed tasks.md Global Constraints・2.1 — `update-apply-install.sh:899-916` の継ぎ目（`cd "$work"` の相対の道・待ちの秒数を自己試験の関数の中だけで縮める・偽の `reg.exe`・`icacls.exe`・zip を開く命令・PC の収集アプリ）を書き、時刻の差し込みの禁止との関係（秒数の差し込みは別）を書いた。実地の口が判定の関数を全部呼ぶことを `declare -f` で見る確かめを 2.1 に足した

## R10. 1 つの Scenario が主張を 2 つ以上束ねている
- 成果物: openspec/changes/ch22-soak-install-online-ci/specs/distribution/spec.md
- 根拠:
  - spec.md:17-21「置く段は、ほかのインストールと更新の試験の後に…」の THEN には主張が 3 つある: 3 つの試験より後・塞ぐ手順が最初のインストールより前・時間の上限が 2 か所
  - spec.md:242-247（MODIFIED に足した Scenario）は THEN に 2 つ（網に出る試験の手順・塞ぎを外す手順）と、AND に壊した写しの 2 つを持つ
  - spec.md:23-27 の THEN は 2 つ（命令の順・インストールの時に更新係の置き場が無い）
  - 「最後の段」の言い方も揃っていない: Requirement の題と spec.md:5 は「最後の段」と言う。tasks.md:11 は「harness2 の check_scenarios の手順より前でもよい」とし、Scenario は 3 つの試験より後しか見ない
- kind: technical
- 提案: 主張ごとに Scenario を分ける（少なくとも、D18 の塞ぐ位置・時間の上限・握手の試験の手順・塞ぎを外す手順）。「最後」は「インストールと更新の試験の手順より後」と書き直して、tasks と揃える
- 処置: fixed specs/distribution/spec.md — 第 1 の Requirement の CI の定義の Scenario を 4 本に分け（後に走る・塞ぐ手順の位置・時間の上限・突き合わせ）、MODIFIED の Scenario を 2 本に分け、順と置き場の有無を別の Scenario にした。題と本文の「最後の段」を「インストールと更新の試験の手順より後」に直し、tasks の Global Constraints と揃えた

## R11. 「壊してはいけないもの」の今の 3 つの台本の `--list` の突き合わせを、どの Scenario も検証も固定しない
- 成果物: openspec/changes/ch22-soak-install-online-ci/tasks.md
- 根拠:
  - CH22.md の「壊してはいけないもの」の 3 つ目は「install-windows の今の 3 つの台本の `--list` と走った `== Scenario:` の突き合わせ（欠ければ落ちる）」
  - Task 2.2 は ci.yml の install-windows を書き換える。しかし CI の定義を読む自己試験（tasks.md:64-68）と grep の検証（tasks.md:71）が見るのは、置く段の `--list`・`--self-test`・`--block-outbound` だけ
  - ci.yml:618-626・:629-637・:639-648 の 3 つの `comm` の行が消えても、どの検証も落ちない
- kind: technical
- 提案: CI の定義を読む自己試験に、3 つの台本の `--list` と `comm -23` の手順が残っていることを足す。足した確かめは、Scenario「置く段は、ほかのインストールと更新の試験の後に、最後の段として走る」か、新しい 1 本の担保にする
- 処置: fixed tasks.md — 2.2 と specs/distribution/spec.md を直した。 Scenario「インストールと更新の 3 つの台本の Scenario の突き合わせは残る」を足し、CI の定義を読む自己試験が 4 つの台本の `--list` と `comm -23` を見る形にした

## R12. MODIFIED の SHALL は、塞ぐ対象を「外への接続」から「更新係の外への接続」に狭めた。D10 と proposal はそれを名指ししていない
- 成果物: openspec/changes/ch22-soak-install-online-ci/specs/distribution/spec.md
- 根拠:
  - 正典 spec.md:2523 の文は「配る組み立てを動かす CI は、外への接続を塞いで走らせる」
  - MODIFIED の spec.md:184 の文は「置く段より前の全部の段を、更新係の外への接続を塞いで走らせる」
  - proposal.md:25-27 と design.md:99-104（D10）が言う改めは、例外を 2 つ名指しすることだけ。塞ぐ対象を更新係に限ることは書いていない
  - 今の CI の塞ぎは更新係だけ（ci.yml:606-610）なので、今の振る舞いとは合っている
- kind: technical
- 提案: D10 に「塞ぐ対象を、実際の D18 と同じ更新係に揃えた」と 1 行書く。CH22.md の「どこまで」の範囲（最後の SHALL を合わせて改める）に入ることも明示する
- 処置: fixed D10 — 塞ぐ対象を今の D18 と同じ更新係の外向きに揃えたこと、本人の決定の範囲（同じ SHALL）に入ることを D10 に書き、MODIFIED の文の括弧と proposal にも書いた

## R13. MODIFIED の Scenario の GIVEN は「この repo の CI の定義」だが、Task 2.2 の自己試験が読むのは ci.yml だけ。ci.yml の注記にも `--ignored` がある
- 成果物: openspec/changes/ch22-soak-install-online-ci/tasks.md
- 根拠:
  - spec.md:244 の GIVEN は「この repo の CI の定義」で、THEN は「網に出る試験…を走らせる手順は…1 つだけ」と言う
  - tasks.md:64 は「読むのは `$root/.github/workflows/ci.yml`」とする
  - `.github/workflows/` には `nightly.yml`・`release.yml`・`vulns-nightly.yml` もある。今の 3 つには `--ignored`・`--include-ignored`・`block-outbound`・インストールの台本・`cargo test` が無い（grep で 0 件）ので、今は偽にならない
  - 将来 nightly.yml などに網に出る手順が足されても、Scenario は緑のまま落ちない
  - ci.yml:255 の注記の行にも `--ignored` の字面がある。行で数える実装は 2 件と数える（手順の `run:` の中に絞らないと、正しい定義で落ちる）
- kind: technical
- 提案: Task 2.2 の自己試験は `.github/workflows/*.yml` を全部読み、手順の `run:` の中だけを数える（注記の行を除く）と書く。Scenario の GIVEN と揃える
- 処置: fixed tasks.md 2.2・D10 — 自己試験は `.github/workflows/*.yml` の全部を読み、手順の `run:` の命令だけを数える（注記の行を除く）と書いた。Scenario の GIVEN も「`.github/workflows/` の全部の定義」・WHEN を「注記の行を除く」に揃えた

## 観点 2: 上の R4・R5・R7・R9・R10 のほかは、該当なし
確かめた範囲: spec.md の 21 本の Scenario の WHEN / THEN が観測の言葉（終了コード・標準エラーの語・`::warning::`・命令の順）か。「正しく」「適切に」の類は無い。

## 観点 3: 上の R4（正典が design の D を指す）・R5（D3 の定義）・R7（D2 の残りの確かめ）のほかは、該当なし
- 確かめた範囲（D 番号）: D1〜D13 のうち、観測できる振る舞いを述べるもの。D13 の `::warning::`、D10 の 2 つの例外、D5 の置く間は spec にある
- 確かめた範囲（実装の名前）: spec に出る名前は次のとおりで、CI の定義・OS・ログの置き場として観測できる名前の範囲だった
  - サービスの名前 `ashiato-db` など
  - `logs/updater.log`
  - `--ignored`・`tls_`
  - `tls_online_*`（本人の決定の字面）
- 確かめた範囲（Capabilities）: proposal の Capabilities は distribution の 1 つで、specs/ のディレクトリと CH22.md の `capability: [distribution]` に一致する

## 観点 4: 上の R1・R2・R9・R11・R13 のほかは、該当なし
確かめた範囲:
- 1.1・2.1・2.2・3.1・5.1 の検証コマンドの形
- 依存の順（判定 → 実地の口 → CI の配線 → 文書 → 最初の走り → 全体）
- 1.1 の印の数 15 と、spec の自己試験の Scenario の数
- 5.1 の予定のパスの外の検査（`docs/briefs/` は gitignore の対象）

## 観点 5: change として、上の R2（完了の判定の P2・P7）・R3（D7 の反転と完了の判定）・R8（依頼の panic）・R11（壊してはいけないもの）のほかは、該当なし
確かめた範囲:
- CH22.md の「完了の判定」は空でない
- 「壊してはいけないもの」の D18 は Scenario（spec.md:21 の後半）が持つ
- `tls_online_*` は MODIFIED の Scenario が持つ（握手の試験の手順が `tls_` に絞った 1 つ）
- 既存の 9 本の Scenario は変わっていない（diff で確かめた）
- 依頼の「やらないこと」（毎晩の試験・答えの形の比べ）を越える Requirement は無い
- 製品のコードに触る Task は無い（tasks.md:7-9・:94）

## 観点 6: 該当なし（`--draft`。`spec --answers` でない）

## 観点 7: 該当なし（spec_overlap.py candidates が 0 件）
