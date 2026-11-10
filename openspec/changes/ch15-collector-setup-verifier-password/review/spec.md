# CH15 spec-review（成果物どうしの整合）

対象: `openspec/changes/ch15-collector-setup-verifier-password/`（proposal・design・tasks・lane.json・lane-verdict.json・deep.md・.openspec.yaml）、`docs/changes/CH15.md`
照らした正典: `openspec/specs/data-sensitivity/spec.md` の Requirement「画面は、ログインして画面の合言葉を入れ直したときだけ、収集アプリの設定の文字列を出す」（:603-654）
照らした実物: `crates/server/src/web_session.rs`（`WebLogin`・`collector_setup_post`・`session_post`）・`lib.rs`（`App::with_login` / `with_web_login` / `for_test`・起動の組み立て）・`st55_tests.rs`・`st56_tests.rs`・`web_session_tests.rs`・`scripts/lanes.py`（`red_green`）・`scripts/evidence.py`
確かめた日: 2026-11-10（worktree `ashiato2-up-ch15`、HEAD ab6f1cd27、`origin/main` = 04a618b3b）。軽い路（`lane.json` の `reason: bug`）

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate ch15-collector-setup-verifier-password --strict` | valid（rc=0。`skip_specs: true` で delta 0 を受ける） |
| `python3 scripts/check_chain.py .` | chain: OK（0 件 / 未回収 0 件 / warn 0 件） |
| `python3 scripts/check_scenarios.py . ch15-collector-setup-verifier-password` | scenarios: OK（warn はこの change に関わらない既存の印だけ） |
| `python3 scripts/review_triage.py . ch15-collector-setup-verifier-password` | triage: OK（指摘 0 / 仮決め 0 / 要件へ戻すもの 0。この review を書く前） |
| `python3 scripts/spec_overlap.py candidates . ch15-collector-setup-verifier-password` | `[]`（0 件） |
| `lanes._items_naming(tasks.md, "合言葉を入れ直すと、設定の文字列とコピーのボタンが出る")` | `['1.1']`（名指しの項目は 1.1 だけ。9.1 は当たらない） |

原因の見立ては実物で正しい: `WebLogin::new_with_verifier` は `password: String::new()`（web_session.rs:335-345）、`collector_setup_post` は `given.is_empty() || !token_matches(&given, &app.login.password)`（web_session.rs:598）なので、照らす形では空でない合言葉はすべて 403。
照らす形で壊れている口はほかに無い: `login.password` を読む本番の式は :598 だけ（:350 `accepts` の開発の形・:357/:369 は `#[cfg(test)]`）。`lib.rs:2870` の `export_body` が読む `password` は書き出しの暗号の合言葉で、画面の合言葉と比べない。ログインの印は `secret_tag_for_verifier`（:243）で作り、`session_valid` は `tag` を見る（:421）。

## R1. 1.1 の検証は `-- --list` の数えを前に置くので、試験が無い・組み立てが落ちた木の FAIL も「直す前に落ちた」と数えられる
- 成果物: openspec/changes/ch15-collector-setup-verifier-password/tasks.md（1.1）
- 根拠: tasks.md:25 の検証は `test "$(cargo test … ch15_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3 && scripts/quiet-run ct -- cargo test … ch15_`。ch15_ の試験がまだ無い木、または組み立てが落ちる木（`2>/dev/null` で見えない）では `test` が rc=1 を返す。evidence.py:495-498 は rc≠0 で出力に印が無ければ `("FAIL", None)`。試験が 0 本のときに「1 本も走っていない」と理由を付ける `ran_no_tests` は、コマンドに `--list` があると見ない（evidence.py:347）。`red_green` が飛ばすのは理由に「試験が無い」「1 本も走っていない」を含む記録だけ（lanes.py:366）。なので、本番のコードが base のままの木でこのコマンドを走らせれば、試験を 1 本も書いていなくても red の証跡になる。tasks の「`ch15_verifier_collector_setup_returns_token` が 403 で落ちる」（tasks.md:24）は文で書いてあるだけで、機械は見ない
- kind: technical
- 提案: 1.1 の検証を、数えの前置きを外した `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server ch15_'` rc=0 にする（`.env` の包みは残す。tasks.md:15）（0 本なら rc=0 になり、`ran_no_tests` が「1 本も走っていない」を付ける。`red_green` はその記録を red に数えない）。「3 本以上」の数えは 1.2 の検証に移し、CH15.md:34 の完了の判定（「`-- --list` で 3 本以上（tasks 1.1）」）も同じに直す。組み立てが落ちた FAIL はそれでも red に数えられるので、code_verify が FAIL のログに `ch15_verifier_collector_setup_returns_token … FAILED` と 403 の不一致が出ているかを見る、と 1.1 に書く
- 処置: fixed 1.1 — 1.1 の検証から `-- --list` の数えの前置きを外し（`.env` の包みは残す）、3 本以上の数えを 1.2 と CH15.md の完了の判定へ移した。直す前の FAIL のログに `ch15_verifier_collector_setup_returns_token` の 403 の不一致が出ていることを確かめる、と 1.1 に書いた

## R2. 1.1 で lib.rs に触れた木の FAIL は red に数えられない。lib.rs は要らないので、1.1 では触らないと言い切る
- 成果物: openspec/changes/ch15-collector-setup-verifier-password/tasks.md（Global Constraints・1.1）・proposal.md（Impact）
- 根拠: `red_green` は、FAIL の木の base との差分に本番のコードがあればその記録を red にしない（lanes.py:368-373）。lane-verdict.json:34-38 は `crates/server/src/lib.rs` を `code` に分類している（`web_session_tests.rs` は入っていない）。tasks.md:9 と proposal.md:33 は lib.rs を「試験の口が要るときだけ」触ってよいとしているが、試験の口を足した木で 1.1 の FAIL を記録すると、その記録は red に数えられない。lib.rs は要らない: `App::with_login` は crate の根の非公開の関数（lib.rs:518）で、`web_session_tests` は crate の根の子（lib.rs:287、web_session_tests.rs:10 の `use super::*`）なので呼べる。`WebLogin::failure_delay` は `pub(crate)`（web_session.rs:316）、`password_verifier_for_test` も `pub(crate)`（:184）。逆に、手近な口は照らす形を組まない: `App::with_web_login` は `WebLogin::new`（合言葉そのものの形）に組み直す（lib.rs:603-611）。照らす形の `App::web_password_for_test()` は空を返す（web_session.rs:339・lib.rs:621）ので、`testdb::web_cookie(&app, app.web_password_for_test())`（testdb.rs:58 の型）でログインすると 401 になる
- kind: technical
- 提案: tasks.md の Global Constraints と 1.1 に「1.1 の FAIL を記録するまで、lib.rs と web_session.rs には触れない（本番のコードと数えられ、red の証跡にならない）」と書く。照らす形の App は `with_login` で組み、`with_web_login`・`web_password_for_test`・st56 の `login_cookie` の型を使わず、試験の中で作った合言葉でログインする、と添える
- 処置: fixed 1.1 — Global Constraints に「1.1 の FAIL を記録するまで lib.rs と web_session.rs に触れない」「lib.rs は触らない」と言い切り、照らす形の App は `with_login` で組み、`with_web_login`・`web_password_for_test`・st56 の `login_cookie` の型を使わないと添えた

## R3. D1 の要（`LOGIN_CHECK` の印・`spawn_blocking`）をどの検証も見ないので、`accepts` を async の側で呼ぶだけの実装でも全部通る
- 成果物: openspec/changes/ch15-collector-setup-verifier-password/tasks.md（1.2）・design.md（D1）
- 根拠: tasks.md:11 の Global Constraints は「`LOGIN_CHECK` の印・`spawn_blocking` の側での `WebLogin::accepts`・同じ側での `record_failure()`」を求めている。しかし 1.1・1.2 の検証（tasks.md:25-31）が見るのは、ch15_ / st55_ / st56_ / web_session の試験、`login\.password\b` が無いこと、clippy、openapi だけ。`collector_setup_post` の :598 を `!app.login.accepts(&given)` に替えるだけの直しでも、`login.password` は消え、試験は順に送るのでどれも緑になる。並んだ求めが失敗の数えをすり抜けないこと（ST55 review R23・R78）を確かめる試験は無い（`grep -n "join!\|JoinSet\|LOGIN_CHECK\|join_all\|spawn(" crates/server/src/st55_tests.rs crates/server/src/st56_tests.rs crates/server/src/web_session_tests.rs crates/server/tests/server_startup.rs` に当たるのは、st55_tests.rs:562-585 の停止の知らせの試験と、server_startup.rs の子のプロセスの起動だけ。ログインの口を並べて送る試験は無い）。なので、`session_post` の照合を関数へ移すときに「印を blocking の側へ渡す」（web_session.rs:680-691）が抜けても、どの検証も落ちない
- kind: technical
- 提案: 1.2 に機械で見る検査を足す。例: `bash -c 'test "$(grep -c "\.accepts(" crates/server/src/web_session.rs)" -eq 1 && test "$(grep -c "LOGIN_CHECK" crates/server/src/web_session.rs)" -eq 2'`（`accepts` を呼ぶのは共通の関数の 1 か所だけ。印は宣言と取る場所の 2 か所）。この 2 つの数は base でも成り立つ（今は `.accepts(` が :685 に 1 か所、`LOGIN_CHECK` が :303・:656 の 2 か所）。だからこれは赤から緑への確かめではなく、近道の直し（async の側で `accepts` を呼ぶ・`collector_setup_post` に 2 つ目の `.accepts(` を置く）を止める検査になる。数を固定するのが脆いなら、`collector_setup_post` の本体を awk で切り出して `token_matches(` と `.accepts(` が無いことを見る。並んだ求めの試験（例: 数えの上限の手前で `tokio::join!` で 2 本送り、合わせて上限を超えない）を ch15_ に足せるなら、それが一番強い
- 処置: fixed 1.2 — `.accepts(` が 1 か所・`LOGIN_CHECK` が 2 か所であることを見る grep を 1.2 の検証に足した（近道を止める検査で、赤から緑の確かめではないと明記）。並べて送る試験は、試験の時刻に依る揺れを持ち込むので足さない

## R4. D1 は空の合言葉の判定と断る数の判定の順を決めていない。今は数えを超えた窓では空でも 429 だが、読みようによっては 403 になり失敗にも数える
- 成果物: openspec/changes/ch15-collector-setup-verifier-password/design.md（D1）・docs/changes/CH15.md（壊してはいけないもの 1）
- 根拠: 今の `collector_setup_post` は、`throttled()` で 429 を返した（web_session.rs:579-588）**後で**本文を読み、空なら失敗に数えて 403（:590-611）。数えを超えた窓では空の合言葉は 429 で、失敗にも数えない。design.md:20 は、断る数の判定（`throttled`）を共通の関数の中に置く。design.md:23 は「空の合言葉は、今と同じく照合の前に失敗に数えて 403」「空の判定を口に残す」と書く。「照合の前」を共通の関数を呼ぶ前と読めば、数えを超えた窓で空の合言葉は 429 から 403 に変わり、印の外で失敗に数える。この順はどの試験も固定していない（st56_tests.rs:241-260 は数えの下での 403 だけ）。CH15.md:25 の「失敗の数えを超えたら正しい合言葉でも 429」も、空のときを言っていない
- kind: technical
- 提案: D1 に「空の判定は、印を取り断る数を見た後に置く（共通の関数が空を断る、または『断る数を超えた』を受けた後で口が空を見る）。数えを超えた窓では空の合言葉も 429」と書く。ch15_ か 1.2 に、照らす形で数えを超えた後の空の合言葉が 429 になる試験を 1 本足す
- 処置: fixed D1 — 順を D1 に書いた（断る数の判定が先、空の判定は共通の関数の中で後。数えを超えた窓では空も 429・数えない）。1.1 の `ch15_verifier_collector_setup_shares_login_failure_limit` に空の合言葉が 429 になる確かめを足し、CH15.md の壊してはいけないもの 1 と完了の判定に写した

## R5. proposal は `session_post` を書き直すことを言っていない。申告（lane.json・deep.md・依頼）の「accepts に替えるだけ」とも食い違う
- 成果物: openspec/changes/ch15-collector-setup-verifier-password/proposal.md（What Changes・Impact）
- 根拠: lane.json:4・deep.md:7・CH15.md:21 は「`WebLogin::accepts` に替えるだけ」と言う。design.md:19-24 は「替えるだけにしない」と決め、`session_post` の照合も共通の関数へ移す。ログインの口のコードに手が入るので、design.md:29 自身がそれを危うさとして挙げている。proposal.md:15-16 は「ログインの口と同じ照合に替える」「同じ通し方にする」とだけ書き、Impact（proposal.md:33-35）も `session_post` に手が入ることを言っていない。予定のパスの中に収まり、正典の振る舞いも変えないので、軽い路の判定は変わらない。ただ、merge の前に読む人は「1 行の直し」と読みうる（壊してはいけないもの 4 はログインの口）
- kind: technical
- 提案: proposal の What Changes か Impact に、「`session_post` の照合も同じ関数へ移す（ログインの口のコードに触れる。振る舞いは変えない。確かめは 1.2）」と 1 行足す。`session_post` に触れずに、`collector_setup_post` に同じ形を書き写す道もある。採らないなら、理由（2 つの口の道を 1 つにする。design の Goals）を D1 に 1 行残す
- 処置: fixed proposal.md — What Changes に `session_post` の照合も共通の関数へ移すこと（振る舞いは変えない・確かめは 1.2・予定のパスの中）を書き、D1 に書き写す道を採らない理由（2 つの口の道を 1 つにする）を残した

## R6. 壊してはいけないもの 5 は画面の e2e が通るままを挙げるが、tasks にも完了の判定にも e2e のコマンドが無い
- 成果物: openspec/changes/ch15-collector-setup-verifier-password/tasks.md（1.2）・docs/changes/CH15.md（完了の判定）
- 根拠: CH15.md:29 は「いまの試験 `st56_` と画面の e2e が通るまま」と言う。tasks.md:27-31 と CH15.md:36 が挙げるのは cargo test・grep・clippy・check-openapi だけ。`web/e2e/st56-collector-setup.spec.ts:18-37` は、この change が書き直す `collector_setup_post` を合言葉そのものの形のサーバで通っている（正典の Scenario「合言葉を入れ直すと、…」「合言葉が違えば出さない」の印を持つ）。走らせても `web/` に触れないので、予定のパスの外には出ない
- kind: technical
- 提案: 1.2 に `bash -c 'cd web && npm run test:e2e -- st56-collector-setup'` rc=0（docs/testing.md §4.5）を足し、CH15.md の完了の判定にも写す。CI の e2e の job に任せるなら、5 の「画面の e2e」に「CI の e2e の job で見る」と書き添える
- 処置: fixed 1.2 — `scripts/quiet-run e2e -- bash -c 'cd web && npm run test:e2e -- st56-collector-setup.spec.ts'` rc=0 を 1.2 に足し、CH15.md の完了の判定に写した

## R7. 9.1 は、この直しを入れた版のサーバで確かめることを言っていない
- 成果物: openspec/changes/ch15-collector-setup-verifier-password/tasks.md（9.1）・docs/changes/CH15.md（完了の判定の確認バッチ）
- 根拠: tasks.md:35 と CH15.md:37 は「インストーラで入れた本人の PC のサーバで」とだけ書く。いま本人の PC に入っているのは、不具合が起きた版（CH15.md:21「2026-11-09 本人の実地で発生」）。どの版を、どの道具で入れ直すか・上げるかが書かれていないので、確認バッチで古い版をそのまま見て「直っていない」と答えうる
- kind: technical
- 提案: 9.1 に「この change の頭で組んだ配る道具（`tools/release/`）で入れ直す、または上げたサーバで」と、版の見分け方（例: 起動の記録の版・commit）を 1 行足す
- 処置: fixed 9.1 — この change を含む組み立ての配る道具（`tools/release/`）で上げたサーバで確かめること、上げた版の commit を確認バッチの手順書で示すことを 9.1 と CH15.md に書いた

## 観点 1: 該当なし（確かめた範囲: deep.md は機械の申告の写しで「本人の答え」も「効く先」も無い。申告の 4 項目（理由 bug・根拠・予定のパス・名指しの Scenario）は lane.json / lane-verdict.json の declared と一字一句一致し、名指しの Scenario は正典の data-sensitivity/spec.md:618 に逐語で在る。数値の決定は無い。「要件へ戻すもの」も無い）

## 観点 2: 該当なし（確かめた範囲: この change は spec の delta を持たない（`skip_specs: true`）ので、検証可能かを見る新しい Scenario は無い。名指しの Scenario は正典のまま。注記: 1.1 の `ch15_verifier_collector_setup_returns_token` は、画面の Scenario の THEN のうち、口が資格情報を返すところだけを router で確かめる。文字列とボタンが見えること・送り先が `/api` であること・利用者を持たないことは、合言葉そのものの形のサーバで e2e（`web/e2e/st56-collector-setup.spec.ts`）が見る。照らす形の画面は 9.1 が見る。design.md:30 がこの分担を書いており、軽い路の予定のパス（`web/` の外）の制約とも合う）

## 観点 3: 該当なし（確かめた範囲: design の D は D1 だけ。D1 が述べる振る舞い（429 / 403 / 記録の行）は正典の Requirement がもう言っており、新しい観測できる振る舞いを design だけに置いてはいない。R4 の順は、今の振る舞いを保つ書き方の不足として R に挙げた。proposal の Capabilities は New・Modified ともに「なし」で、`specs/` は無い。CH15.md の `capability: []` とも一致する）

## 観点 4: R1・R2・R3・R6 に挙げた。ほかの項目は確かめた: 1.1 は、試験を書いた後なら正しい理由で落ちる。base の木で落ちるのは `ch15_verifier_collector_setup_returns_token` だけ（照らす形のログインは通り、`/collector-setup` が :598 で 403 を返す）。ほかの 2 本は base でも通る（違う合言葉と空の合言葉はもう 403。9 + 1 回の失敗でもう 429）。1.2 の各コマンドは書いたまま走る形（`st56_` は 7 本で `-ge 7` が成り立つ。st56_tests.rs:91,103,115,135,155,215,241。`login\.password\b` は今は :598 にだけ当たり、直せば 0 件。`tools/check-openapi.sh` は在る）。試験の名前は `ch15_` の接頭辞で、`web_session_tests.rs` に置けば crate の試験の一覧に出る。依存の順（試験 → FAIL の記録 → 直す → PASS の記録）は 1.1 の中で正しく並んでいる。数えの上限は 10（web_session.rs:48）で、9 + 1 回の失敗で上限に届く。9.1 の物理（realdata）は書かれている

## 観点 5: 該当なし（確かめた範囲: change なので check_chain の観点 8 は無い。CH15.md の「完了の判定」は空でなく、design D1・tasks 1.1/1.2/9.1 と対応する。「壊してはいけないもの」1〜6 は、ST56 の試験 7 本・ST55 の照らす形のログイン・openapi の検査で固定されている。漏れは R4（空の合言葉と 429 の順）・R6（e2e）。照らす形の記録の行と、ログに値を出さないこと（3）を見る試験は合言葉そのものの形にだけある。ただし D1 の後は 2 つの形が同じ道を通るので、指摘にはしない。「依頼（逐語）」の範囲は越えていない。D1 で `session_post` に手が入る点は R5）

## 観点 6: 該当なし（`spec --answers` でない。軽い路で、名指しの Q は無い）

観点 7: 該当なし（spec_overlap.py candidates が 0 件）

## A（失う側がある問い）の有無
潜んでいない。D1 は照合の道をログインの口に揃えるだけで、応答の形・状態の番号・遅れ・上限・記録の行を変えず、何も失わない。照らす形の画面を e2e でなく 9.1 で見る分担は、軽い路の予定のパス（`web/`・`tools/` の外）から来る技術的な制約で、利用者が失うものは無い。`session_post` に触れずに書き写す道（R5）も、どちらを選んでも振る舞いは同じなので A ではない。
