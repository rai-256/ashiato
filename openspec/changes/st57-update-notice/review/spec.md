# ST57 の spec の検査（spec --draft・第 1 回の先読み）

## 機械の検査（2026-11-11、docs/st57-upstream の f76d59aa0 の上）

- `openspec validate st57-update-notice --strict` → `Change 'st57-update-notice' is valid`
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（観点 8 の再生成との一致も通っている）
- `python3 scripts/check_scenarios.py . st57-update-notice` → `scenarios: FAIL (担保なし 71 件 / 名無しの確認待ち 0 件)`。
  71 件は ADDED の 67 本と、MODIFIED で足した 4 本で、tasks.md の Global Constraints の数と合う。下流の前なので担保が無いのは予定どおり。
  spec の 94 本の Scenario の名前は、MODIFIED で文を変えていない ST55 の 23 本を除き、どれも tasks.md のどこかの Task が名指ししている（突き合わせの python で確かめた）
- `python3 scripts/review_triage.py . st57-update-notice` → `triage: FAIL (12 件)`。`escalated なのに deep.md に R1 が無い` が R1〜R4・R6〜R8・R11〜R15 で出る（R1 を見る）。
  `要件へ戻すもの 0 件`（R2 を見る）
- `python3 scripts/spec_overlap.py candidates . st57-update-notice` → `[]`

## R1. deep.md に review/deep.md の R の出所が 1 つも無く、triage が 12 件で落ちる
- 成果物: openspec/changes/st57-update-notice/deep.md
- 根拠: `python3 scripts/review_triage.py . st57-update-notice` → `[FAIL] deep.md R1: escalated なのに deep.md に R1 が無い` ほか 11 件（R2・R3・R4・R6・R7・R8・R11・R12・R13・R14・R15）/ `grep -c "R[0-9]" deep.md` → 0 / review/deep.md の処置は `escalated Q1`・`escalated Q4`・`escalated Q9` などで問いを名指ししている / 前例の archive の `2026-11-07-st55-server-installer/deep.md:19,35,41,49` は各問いに `出所: review/deep.md R<n>` の行を持つ
- kind: technical
- 提案: deep.md の各 Q に、review/deep.md の処置が名指しした R を `出所: review/deep.md R<n>` の形で書く（Q1←R1・R3、Q2←R6・R15、Q3←R12、Q4←R2・R14、Q5←R11、Q7←R13、Q9←R4・R7、Q10←R8）。record の段で書き直すなら、その段で triage を通す。
- 処置: fixed deep.md —— 各 Q に `出所: review/deep.md R<n>` を書いた（Q1←R1・R3、Q2←R6・R15、Q3←R9・R12、Q4←R2・R14、Q5←R11、Q7←R13、Q9←R4・R7・R11、Q10←R8）

## R2. Q5・Q9 の推奨は FR-113 に無い振る舞いを足すのに、deep.md に「要件へ戻すもの」が無く、★ の検査が効かない
- 成果物: openspec/changes/st57-update-notice/deep.md・openspec/changes/st57-update-notice/specs/distribution/spec.md
- 根拠: triage の出力 `要件へ戻すもの 0 件`（review_triage.py の検査 4 は、deep.md の「要件へ戻すもの」に挙がった FR だけに ★ を求める）/ spec の導出元 425 行目（携帯の通知）と 472 行目（確かめられていない知らせ）は「推奨のままなら FR-113 を ★ で改訂する」とだけ書く。要件の側の根拠は Q5・Q9 の推奨だけ / `docs/requirements.md:1027` の FR-113 は「V-01 と C-01 の画面、C-02 の通知に『更新あり』」だけで、携帯の通知も「確かめられていない」も持たない / proposal.md:28 は改訂を record の段に預けている / 処置の預け先が散文だけなので、本人が推奨のままにした後に ★ が入らなくても、どの検査も落ちない
- kind: conflict
- 提案: draft は止めない。record の段が、Q5・Q9 が推奨のまま（または通知や知らせを足す側）に決まったら、deep.md に「要件へ戻すもの: FR-113（携帯の通知・確かめられていない知らせ）」を置く。続けて `docs/requirements.md` の FR-113 に ★ を入れ、`docs/stories/ST57.md` を作り直す。これで triage の検査 4 と check_chain の観点 8 が戻し漏れを止める。本人が足さない側を選んだら、spec の 419〜443 行目と 464〜509 行目を削る。
- 処置: fixed D22 仮 —— design D22（仮）に、推奨のまま決まったら record が deep.md の「要件へ戻すもの: FR-113」と ★ と ST57.md の作り直しを行うこと、足さない側なら spec の該当を削ることを反転条件つきで書いた

## R3. 確かめた番号・数が成果物どうしで食い違う（見出しの数・D の番号）
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md・proposal.md・defaults.md
- 根拠: defaults.md:8 は見出しを「`Host`・`User-Agent`・`Accept`・`Accept-Encoding: gzip` の 4 つだけ」とし、spec:27 は `Connection` を加えた 5 つにしている。design の D2 は defaults C2 を写したと言うが、5 つめを足したのは D13（仮）である / spec:35 は `Connection` の根拠を「design D6（仮）」とするが、D6 は更新係の置き場で仮でもない。見出しの集合は D13（仮）（design.md:122）/ proposal.md:53 は TLS を「design D5（仮）」とするが、D5 は時期の判定で、TLS は D12（仮）（design.md:116）
- kind: technical
- 提案: spec:35 を「design D13（仮）」に、proposal.md:53 を「design D12（仮）」に直す。defaults.md の C2 は聞かない既定の原文として残すなら、design D2 か D13 に「C2 の 4 つに `Connection` を足した（仮）」と書き、どちらが今の決まりかを 1 か所で読めるようにする。
- 処置: fixed D13 ——（spec.md ほか） spec の `Connection` の根拠を D13（仮）に、proposal の TLS を D12（仮）に直し、design D2 に C2 の 4 つと D13（仮）の関係を書いた

## R4. 閾値の Scenario が片側にしか無く、7 日・48 時間・64 KB は値を変えても spec が落ちない
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md
- 根拠: 24 時間は 23 時間（spec:175）と 25 時間（spec:181）の両側がある。7 日は 8 日の例だけ（spec:328・475・487）で、3 日の実装でも全部通る。7 日より手前で帯が出ない Scenario が無い（spec:311「まだ 7 日たたないときは帯を出さない」は本文だけ）/ 48 時間（spec:469）は 9 日の例だけ（spec:505）で、7 日の実装でも通る / 64 KB（spec:316）は 65 KB の例だけ（spec:383）で、1 KB の実装でも通る / Q9 の推奨の数値は「7 日」で、本人が決める数値がこの形では固定されない
- kind: technical
- 提案: 手前の側を 1 本ずつ足す。7 日は「最後に確かめられたのが 6 日前で、その後届かない → 帯は無く、PC の知らせも出ない」。48 時間は「更新係の最後に確かめた時刻が 47 時間前 → 理由は届かないのまま」。64 KB は「63 KB の正しい結果は読める」。
- 処置: fixed 3.1 ——（spec.md ほか） 手前の側の Scenario を足した（「6 日確かめられていないだけなら帯は出ない」「6 日確かめられていないだけなら知らせない」「更新係の時刻が 48 時間たたなければ、理由は最後の回のまま」（47 時間）「64 KB より小さい正しい結果のファイルは読む」（63 KB））。tasks 3.1・4.1・8.1 が名指しする

## R5. 「確かめられていない」の語と状態の一覧が spec と design で割れ、読めない結果のファイルで 7 日たたずに帯が出る
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md・design.md
- 根拠: spec:313 の設定の状態は 6 つで、「有効・7 日確かめられていない」はあるが、7 日の条件を持たない「確かめられていない」は無い。一方、spec:316 と spec:385 は読めない結果のファイルを状態「確かめられていない」（理由: 読めない答え）にする / design D14（design.md:129・131）はこれを `stale`（理由 `unreadable`）に写し、tasks 4.1 の帯は `stale` を出す。そのため、結果のファイルが壊れた日のうちに帯「更新を 7 日確かめられていない」が出る。spec:311「まだ 7 日たたないときは帯を出さない」と食い違う / 置き場はあるのに `status.json` が無い（入れた直後で、まだ確かめていない）とき、`pending` になるのか「読めない答え」になるのかを、spec も D14 も書いていない / spec:313 の「有効・まだ確かめていない」と「有効・新しい版なし」を画面で見る Scenario が無い（e2e は available・off・not_running・stale だけ）
- kind: technical
- 提案: 状態の語を 1 つの一覧にそろえ、spec に置く（D14 の 7 つの符号と、画面の語の対応）。読めない結果を「7 日の数えに入る失敗」にするのか、別の状態（例「結果を読めない」。帯は出さず、設定にだけ出す）にするのかを決め、Scenario を 1 本足す。`status.json` の無い置き場は「まだ確かめていない」にする Scenario も足す。
- 処置: fixed D14 ——（spec.md ほか） `state` の 8 つ（`unreadable` を足した）と画面の語の対応を spec に置いた。読めない結果は `unreadable` で帯を出さず設定にだけ出す。`status.json` の無い置き場は 7 日まで `pending`。Scenario「結果を読めないときは帯を出さず、設定にだけ出す」「まだ確かめていない更新係は、7 日たつまで帯を出さない」「新しい版なしは設定にだけ出る」を足した

## R6. 確かめられない理由が「6 つ」と言いながら 7 つあり、spec と design で中身が違う
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md・design.md
- 根拠: spec:245 の 6 つは「更新係が動いていない（サーバだけ）」を含み、「宛先なし」を含まない。ところが spec:82 は記録に「宛先なし」を書かせる / design D4（design.md:58）の `reason` は `no_destination` を含み、`not_running` を含まない。D14（design.md:129）は `not_running` を足す / 理由の Scenario（spec:263）は `403`・`404`・`500`・読めない `200` だけで、「時間切れ」と `429` は Scenario を持たない
- kind: technical
- 提案: spec:245 を、記録に書く理由（宛先なしを含む）と、サーバが読む時に足す理由（更新係が動いていない）に分けて数を直す。時間切れ（受け手が応答を返さない）と `429` の Scenario を足す。
- 処置: fixed 2.1 ——（spec.md ほか） 記録の理由 6 つ（宛先なしを含む）と、サーバが読むときに足す「更新係が動いていない」を分けた。時間切れ（10 秒・30 秒）と転送 3 回を spec に書き、`429` を理由の Scenario に、「答えない配布元は時間切れになる」を足した（tasks 2.1）

## R7. 時期の Scenario は「部品」を主語にしながら、担保は更新係の 1 本だけで、携帯の別の実装を見ない
- 成果物: openspec/changes/st57-update-notice/tasks.md・specs/distribution/spec.md
- 根拠: spec:167〜169 は「各部品に」24 時間・未来の時刻・失敗の数えを課す。その Scenario（spec:175〜197）の印は tasks 2.1（`ashiato-updater` の試験）にだけ置かれ、8.1（PC）と 9.1（携帯）は名指ししない / 携帯の時期の判定は Kotlin の別の実装（tasks 9.1「`UpdateCheck.kt`（選び方・時期・7 日…）」）で、共有の材料（design D3、tasks 1.1）は選び方だけを持ち、`due` の場合を持たない / `check_scenarios.py` は名前ごとに印が 1 つあれば担保ありとみなすので、Kotlin の `due` が「未来の時刻なら確かめない」と書かれても緑になる
- kind: technical
- 提案: 共有の材料に時期の場合（23 時間・25 時間・失敗の後 1 時間・3 年先の時刻・7 日・未来の最後に確かめられた時刻）を足し、Rust と Kotlin の両方で全件を通すと tasks 1.1・9.1 に書く。または、9.1 に spec:175〜197 の 4 本の印を携帯の側でも置くと書く。
- 処置: fixed 1.1 ——（spec.md ほか） 共有の材料に `timing`（due・stale・48 時間の両側）を足し、Rust と Kotlin が全件を通すと書いた

## R8. 2 つの部品にまたがる Scenario を 2 つの Task に分けて印を置くので、片方だけでも緑になる
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md・tasks.md
- 根拠: spec:12「収集アプリは新しい版をサーバに問い合わせない」と spec:199「収集アプリは送り先を設定するまで確かめない」は、THEN で PC と携帯の両方を主張する。tasks 8.1 と 9.1 は同じ名前の印を「PC の側」「携帯の側」として置く（tasks.md:100・111）/ `check_scenarios.py` は 1 つの名前に印が 1 つあれば担保ありとみなす。9.1 の印が抜けても、10.2 の `check_scenarios.py` は rc=0 になる
- kind: technical
- 提案: 2 本ずつに分ける（「PC の収集アプリは新しい版をサーバに問い合わせない」と「携帯の収集アプリは…」、送り先の設定も同じ）。名前を部品ごとにして、tasks の印も分ける。
- 処置: fixed 8.1 ——（spec.md ほか） 「収集アプリは新しい版をサーバに問い合わせない」「収集アプリは送り先を設定するまで確かめない」を PC と携帯の 2 本ずつに分け、印を 8.1 と 9.1 に分けた

## R9. 「インストールの間、ネットワークに出ない」が残ったまま、インストールの最後に更新係が外へ問い合わせる
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md（MODIFIED「サーバのインストールは…」）
- 根拠: spec:541「THE SYSTEM SHALL インストールの間、ネットワークに出ない。」と spec:542「インストールが最後に達する…その後に更新係のサービスを開始する（更新係の最初の問い合わせは、示した後にだけ出る）」が同じ Requirement にある。インストーラが起こした問い合わせが「インストールの間」に入るかどうかを、どちらの文も決めていない / この Scenario の確かめの遮断の一覧（`tools/release/install-check.sh:757-759`）は、setup・server・PostgreSQL の実行ファイルだけを塞ぎ、`ashiato-updater.exe` を持たない。tasks 7.1 はこの確かめを直す対象に挙げていない（直すのは `配る zip からの…`・`消すと…`・`消しても…` の 3 本）/ CI では D18（仮）が別に更新係を塞ぐので、どちらの読み方でも緑になる
- kind: technical
- 提案: spec:541 を「インストーラの段（更新係のサービスを開始する前まで）は、ネットワークに出ない」に直し、更新係の最初の問い合わせは範囲の外だと書く。または「インストーラ自身は出ず、出るのは開始した更新係だけ」と書く。どちらにしても、install-check.sh のこの確かめを、更新係を含めた形に合わせると tasks 7.1 に足す。
- 処置: fixed 7.1 ——（spec.md ほか） MODIFIED の文を「インストーラの段（更新係のサービスを開始する前まで）は出ない。開始した後の更新係の問い合わせは範囲の外」に直し、install-check.sh の遮断の一覧に `ashiato-updater.exe` を足すと tasks 7.1 に書いた

## R10. design に観測できる振る舞いが残り、archive で正典から落ちる（D14・D2・D7・D21・D5）
- 成果物: openspec/changes/st57-update-notice/design.md
- 根拠: 次のものが design にだけあり、spec の Requirement にも Scenario にも無い。
  - D14（design.md:129〜131）: `GET /update-status` の応答の形（`state` の 7 つの符号と欄の名前）・`Cache-Control: no-store`・読み出しの記録に残さないこと。ST58 はこの形の上に積む（proposal.md:58「後続の ST58 は…`update-status` の結果…の上に」）。spec は 401・403 と状態の日本語の名前だけを持つ。正典 data-sensitivity:203 は、API の合言葉で許可の一覧の外の口に届いた 403 を記録に残すので、「残さない」の範囲（成功した読みだけか）も design だけでは決まらない
  - D2（design.md:42）: 時間切れ（接続 10 秒・全体 30 秒）と、転送を追う上限 3 回
  - D7（design.md:86）: 「`repair` の命令の一覧は変えない」。spec では MODIFIED の導出元の注記（spec:657「`repair` は更新係に触れない」）だけで、SHALL でも Scenario でもない。tasks 5.1 の試験に印が無い
  - D21（design.md:168〜171）: 開発版の土台の版が違うとき（`0.4.0-dev.20261110` と `0.5.0-dev.20261101`）の更新ありの判定
  - D5（design.md:69）: 「組み立ての時に `-P` で渡した送り先も設定済みに数える」。spec:170 の「送り先の設定（設定の文字列）を保存した後」に例外を足している
- kind: technical
- 提案: D14 の応答の形を、`GET /update-status` の Requirement（spec:307 の節）に SHALL として写す。ST58 が当てにする欄と、`no-store` は Scenario を 1 本ずつ持たせる。D7 の repair は、正典の repair の Requirement を MODIFIED にして SHALL と Scenario（「repair は更新係のサービスと置き場に触れない」）にする。D21 は「新しい版かどうかは…」に Scenario を 1 本足す。D5 の `-P` の例外は spec:170 に書くか、例外を外す。D2 の数値は、spec に「時間切れ」の Scenario（R6）を足すときに一緒に入れる。
- 処置: fixed D14 ——（spec.md ほか） D14 の応答の欄と `state` の 8 つと `no-store` を SHALL と Scenario（「更新の状態の口は応答を残させない」）にした。repair は ADDED「インストーラの `repair` の口は、更新係のサービスと置き場に触れない」と Scenario にした（tasks 5.1・7.1）。D21 は Scenario「開発版は土台の版が上がった版を新しいとみなす」。D5 の `-P` の例外は spec の本文に写した。D2 の数値は R6 と一緒に spec に入れた

## R11. 更新係の依存の許諾が、サーバの Windows の zip の依存の一覧に載る道が無い
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md・tasks.md
- 根拠: 正典 distribution:2223〜2224 は、サーバの `THIRD-PARTY-NOTICES.txt` に「実行ファイルに入る Rust の部品」を入れると決めている / `tools/release/notices.sh:177` は `server` の種類で `ashiato-server` の依存の木だけを辿る（`crate=ashiato-server`）/ 更新係は `ureq` + `rustls` + `platform-verifier`（design D12（仮））を持ち、サーバはこれらに依らない（D1）。だから、どれもサーバの zip の一覧に載らない / tasks 6.1 は `tools/check-licenses.sh` を走らせるが、notices.sh や配布物の一覧を直す Task は無い。spec はこの Requirement を MODIFIED にしていない
- kind: technical
- 提案: 正典の「配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ」を MODIFIED にし、サーバの Windows の配布物の一覧に更新係の実行ファイルの Rust の部品を足す。Scenario は「Windows の zip の `THIRD-PARTY-NOTICES.txt` に、更新係だけが持つ TLS の部品の項がある」。tasks 6.1 に notices.sh の直しと、その確かめを足す。
- 処置: fixed 6.1 ——（spec.md ほか） 正典「配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ」を MODIFIED にし、サーバの Windows 版に更新係の Rust の部品を足し、Scenario「サーバの Windows 版の依存の一覧は、更新係だけが持つ部品を含む」を足した。tasks 6.1 に notices.sh の直しと確かめを書いた

## R12. OS の根で確かめることと、根を同梱しないことは本文だけで、TLS は人間の確認待ちまで一度も通らない
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md
- 根拠: spec:30「暗号化した接続の証明書を OS の持つ根で確かめる（部品に根を同梱しない）」に Scenario が無い / どの試験の受け手も loopback の HTTP（tasks Global Constraints・2.1・8.1・9.2）で、CI の Windows は D18（仮）で更新係の外への接続を塞ぐ。TLS の握手が通るかを最初に見るのは 11.1（人間・本物の配布元・リリースが出た後）/ D12（仮）の反転条件「`NT SERVICE\ashiato-updater` で OS の根を読めない」も、確かめる Task が無い / 「同梱しない」は依存の木で機械が見られる（例: `webpki-roots` が `cargo tree -p ashiato-updater` と `-p ashiato-collector-windows` に無い）
- kind: technical
- 提案: 「部品に根を同梱しない」を Scenario にし、`tools/check-updater-deps.sh`（tasks 2.2）で依存の木に同梱の根の部品が無いことを見る。OS の根で確かめることは、試験で作った認証局を OS に入れた loopback の https の受け手で 1 本見るか、それが CI で無理なら、人間の確認待ちの 11.1 に「TLS が通る」を明記する。
- 処置: fixed 2.2 ——（spec.md ほか） 「根の一覧の部品を同梱しない」を Scenario にし、check-updater-deps.sh が依存の木に `webpki-roots`・`webpki-root-certs` が無いことを見る。OS の根で通ることは人間の確認待ち 11.1 に明記した（CI は本物の TLS を通らない）

## R13. ベータ版の規則の `<版>` が、自分と同じ土台の版だけを指すとも読める
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md
- 根拠: spec:88「ベータ版（`<版>-beta.<N>`）の部品は、印が `v<版>` か `v<版>-beta.<N>` のリリースを数える」は、自分の版の `<版>` と同じ記号を使う。字のとおりに読むと、`0.4.0-beta.2` の部品は `v0.4.0` と `v0.4.0-beta.*` だけを数え、`v0.5.0-beta.1` と `v0.4.1` を数えない / Scenario（spec:106〜110）は同じ土台（`0.4.0-beta.3`・`0.4.0`）だけを見るので、どちらの読み方でも通る / Q2 の推奨は「ベータ版はベータ版と正式版」
- kind: technical
- 提案: 記号を分ける（「印が `v<X.Y.Z>` か `v<X.Y.Z>-beta.<N>` の形のリリース」）。Scenario を 1 本足す: 「版 `0.4.0-beta.2` の部品に、一覧の `v0.5.0-beta.1` が最も新しい → 更新あり 0.5.0-beta.1」。
- 処置: fixed 1.1 ——（spec.md ほか） 記号を `<X.Y.Z>` と `<A.B.C>` に分け、Scenario「ベータ版の部品は、土台の版の上がったベータ版も更新ありにする」を足した（tasks 1.1）

## R14. 「確かめられていない」の Scenario の引用が、決めた文の部分の文字列になっていない
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md
- 根拠: spec:466・479・487・491 は、通知と行を「更新を確かめられていない」と引く。spec:310 の帯の文は「更新を 7 日確かめられていない（…）」で、design D20（design.md:165）の通知の文は「更新を 7 日確かめられていません（…）」。どちらにも「更新を確かめられていない」という部分の文字列は無い / 一方、spec:16・421・442 の「更新あり」「新しい版 <版> が出ています」は、文そのものを引いている / 試験が spec の引用で文を探すと落ち、札だけで探すと文の中身を見ない
- kind: technical
- 提案: spec:466 に携帯の行と通知の文を書き（D20 から写す）、Scenario の THEN の引用を、その文か、文の部分の文字列（「確かめられていな」など）にそろえる。
- 処置: fixed D20 ——（spec.md ほか） 携帯と PC の文「更新を 7 日確かめられていません（最後に確かめた <月>/<日>・<理由>）」を spec の本文に置き、Scenario の引用をこの文にそろえた

## R15. 携帯の「確かめた記録」をバックアップから外す Scenario が、記録の置き場を見ていない
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md
- 根拠: spec:246 は「確かめた記録と『知らせた版』の置き場」を外すと言う。Scenario（spec:268〜271）の題は「携帯の確かめた記録はバックアップから外れる」だが、THEN が見るのは `update_check.xml`（design D4 では結果と知らせた版）だけ / 記録は design D4 の `noBackupFilesDir/update_checks.jsonl` に置き、OS が外すことに頼る。記録が `filesDir` に置かれても、この Scenario は通る
- kind: technical
- 提案: Scenario の THEN に「確かめた記録のファイルは、バックアップの対象にならない置き場（`noBackupFilesDir`）にあるか、規則で外されている」を足す。携帯の計測テストか Robolectric で、記録のファイルの置き場を読む。
- 処置: fixed 9.1 ——（spec.md ほか） THEN に「確かめた記録のファイルは OS がバックアップしない置き場にある」を足し、9.1 の Robolectric の試験が置き場を見ると書いた

## R16. 送り先を既に設定した収集アプリは、新しい版に入れ替えた直後に、切る口を見ないまま最初の問い合わせを出す
- 成果物: openspec/changes/st57-update-notice/design.md（Migration Plan）・specs/distribution/spec.md
- 根拠: design.md:186「新しい版に入れ替えた後、既に送り先を設定していれば次の機会に確かめる（確認は既定で有効）」/ spec:170 は、最初の問い合わせを「送り先の設定（設定の文字列）を保存した後」とする。入れ替えた人は前の版で保存済みなので、字のとおりなら入れ替えた直後に出る。本人は更新の確認ができたことも、スイッチと `update-check off` も、まだ見ていない / review/deep.md R2（kind: irreversible、loss: exported）が塞いだのは、入れたての部品の族だけ。入れ替えの族が抜けている。サーバは走らせ直しの最後の表示の後に出す（design.md:185）ので、守られている / design D5（design.md:69）の `-P` の送り先の例外も、同じく本人が切る口を見る前に出る
- kind: irreversible
- loss: exported
- 提案: 失わない側は安い。入れ替えの後の最初の起動で「更新の確認: 有効（切り方）」を見せる。携帯は最初の画面の行か通知 1 回、PC はトースト 1 回。最初の問い合わせはその後の機会にする。仮で進めるなら provisional.jsonl に載せる。spec:170 には「確認の口を持たない前の版から入れ替えた部品も、切り方を示した後」と書き、Scenario を 1 本足す。Q1 の答え（直に聞くか）に従属するので、Q1 の context に 1 行足して本人に見せてもよい。
- 処置: escalated Q11 —— はじめ失わない側（切り方を通知で 1 回示してから確かめる）を仮の D23 で置いたが、別の席（`provisional-review.md` D23）が「否」とした（通知を出せない端末での向きが無い）。台帳から外し、第 2 回の Q11 で本人に返した。答え（示す口を広げ、どれでも示せない間は問い合わせない）を design D23 と spec の SHALL・Scenario に写した

## R17. specs に実装の名前が入っている（DB の部品の crate 名・携帯の SharedPreferences の名前・言語）
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md
- 根拠: spec:208 は DB に繋ぐ部品を crate 名（`sqlx`・`tokio-postgres`・`postgres`・`diesel`・`sea-orm`）で並べる。正典 data-sensitivity:354 の同じ形の Requirement は「一覧は検査の側が持ち、1 つ以上の名前を持つ」とし、名前を spec に置かない / spec:271 の `update_check.xml` は SharedPreferences のファイルの名前（design D11（仮）の名前）/ spec:162 は「（Rust）」「（Kotlin）」と実装の言語で主語を書く
- kind: technical
- 提案: spec:208 は「DB に繋ぐ部品（一覧は検査の側が持つ）」にし、名前は D1 と `tools/check-updater-deps.sh` に置く。spec:271 は「更新の確認の結果と知らせた版の置き場」と書き、名前は design に置く。spec:162 は「更新係と PC の収集アプリの選び方と、携帯の収集アプリの選び方」にする。
- 処置: fixed D1 ——（spec.md ほか） DB の部品の名前は「一覧は検査の側が持つ」にして design D1 へ、言語の主語は部品の名前に直した。`update_check.xml` は Scenario から外し「結果と知らせた版の置き場」と書いた

## R18. tasks の検証と順の穴（9.2 の本数・6.1 が後の Task の性質を先に検査する）
- 成果物: openspec/changes/st57-update-notice/tasks.md
- 根拠: 9.2 の検証は `scripts/quiet-run emu -- tools/android-emulator.sh` rc=0 だけで、`St57UpdateQueryTest` が走ったかと、その本数を見ない。0 本でも rc=0 になる（tasks.md:8 の決まりと食い違う）/ 6.1 は `check-artifacts.sh` に、配る `ashiato-collector-windows.exe` に宛先の文字列があることと、差し込みの名前が無いことを足す（tasks.md:81）。その文字列を入れるのは後の Task 8.1 で、6.1 の検証は `--self-test` だけなので、6.1 の時点の本物の配布物では落ちる検査を、印のついたまま先に置く。9.3 の APK の宛先の検査も同じく 6.1 の後
- kind: technical
- 提案: 9.2 の検証に、計測テストの結果の XML から `St57UpdateQueryTest` の testcase が 2 本以上あることを足す。6.1 の収集アプリと APK の検査の追加は、8.1・9.1 の後に動かす（または 6.1 を 8.1 の後へ並べ替える）。
- 処置: fixed 9.2 ——（spec.md ほか） 9.2 の検証に計測テストの結果の XML の `St57UpdateQueryTest` が 2 本以上を足した。PC の収集アプリの配布物の宛先の検査は 6.1 から、宛先を入れた後の 8.2 に動かした（APK は 9.3 のまま 9.1 の後）

## R19. Q11 の写しで足した「示す」口のうち、D25 の定期の試み・D24 の設定の画面・携帯の後から出せるようになった時は、どの Scenario も落とさない
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md
- 根拠: spec.md:196「PC は次の起動の時（とその後の機会。design D25（仮））に示し直す。携帯は通知を出せるようになった時か画面を開いた時に示す」。tasks.md:101 は `入れ替えた PC の収集アプリは、トーストを出せた後の機会に確かめる` に「（D25（仮）。定期の時にも試みる）」を付けるが、その Scenario の WHEN（spec.md:258）は「成功を返すようになってから起動し」で起動の時しか見ず、spec.md:253 の THEN も「起動し直した時にもう一度…試みる」だけ —— 定期の時に試みない実装でも全部緑になる。design.md:190（D24（仮））と tasks.md:112 の「`SetupActivity` を開いてスイッチが見えた時も印を立てる」は Scenario が無く、design にだけある観測できる振る舞い（archive で正典から落ちる）。spec.md:195 の 3 条件のうち「アプリの通知が切られている」と、携帯で通知の許可を後から与えた時に通知が出て印が立つことにも Scenario が無い。spec.md:196 の本文は `design D25（仮）` を名指すが、design は archive で正典に入らない
- kind: technical
- 提案: Scenario を足す —— 「入れ替えた PC の収集アプリは、起動し直さなくても定期の時に切り方のトーストを試み直す」（起動の時は `Err`、次の定期の時に `Ok` → `update-intro` が 1 回出て、その時には問い合わせない）・「入れ替えた携帯は、設定の画面を開くと示したとみなす」・「入れ替えた携帯は、通知の許可を後から与えると次の機会に切り方を通知する」（とアプリの通知を切った場合の 0 件）。tasks 8.1 / 9.1 の名指しと下限をそれに合わせ、spec.md:196 の `design D25（仮）` は振る舞いの文（「定期の時にも」）に置き換える
- 処置: fixed 8.1 —— spec に Scenario 3 本を足した（`入れ替えた PC の収集アプリは、起動し直さなくても定期の時に切り方のトーストを試み直す`・`入れ替えた携帯は、設定の画面を開くと示したとみなす`・`入れ替えた携帯は、通知を後から許すと次の機会に切り方を通知してから確かめる`）。SHALL の画面に「最初の画面の行か設定の画面のスイッチ」を書き、`design D25（仮）` の名指しを振る舞いの文（「起動し直さなくても定期の時に」）に置き換えた（導出元に D25（仮）を残す）。「アプリの通知が切られている」は Scenario を分けず、tasks 9.1 の `入れ替えた携帯は、通知を出せず画面も開かなければ問い合わせない` を 2 通り（許可なし・`areNotificationsEnabled()` が false）で確かめると書いた。tasks 8.1 / 9.1 の名指しと下限（20・16）、Scenario の数（ADDED 95・計 100）を合わせた

## R20. Q12 の写しの SHALL のうち、確認の窓が閉じた時に数えること（D27（仮））と、示した後も開いている画面から行を消さず次に組み直した時から出さないことは、どの Scenario も落とさない
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md
- 根拠: spec.md:195「同じ回に確認の窓を重ねたときはその窓が閉じた時に数える」・spec.md:196「示した後も開いている画面から行を消さず、次に画面を組み直した時から出さない」。`grep -n '確認の窓\|組み直\|行を消さ' spec.md tasks.md` は SHALL（:195・:196）と Scenario「同じ回に別の画面を前に出したときは…」の THEN（:314。使用状況のアクセスの設定の分かれだけ）にしか当たらない。tasks 9.1（tasks.md:112-114）も窓の分かれの試験を名指さない。窓の回にも `setContentView` の時に印を立てる実装、印が立った後も組み直した画面に行を出し続ける実装のどちらでも、Scenario は全部緑になる（R19 と同じ型）
- kind: technical
- 提案: Scenario を 2 本足す —— 「入れ替えた携帯は、同じ回に確認の窓を重ねたときは窓が閉じた時に画面に出したとみなす」（未使用のアプリの制限の対象で窓が出る → 窓が出ている間は印が無い → 「あとで」で閉じると立つ）・「入れ替えた携帯は、示した後に画面を組み直すと入れ替えの行を出さない」。tasks 9.1 の名指しと下限（19→21）、ADDED と計の本数を合わせる
- 処置: fixed 9.1 —— Scenario「入れ替えた携帯は、同じ回に確認の窓を重ねたときは窓が閉じた時に画面に出したとみなす」と「入れ替えた携帯は、示した後に新しく開いた最初の画面には入れ替えの行を出さない」を足し、tasks 9.1 の名指しと下限（19→21）、ADDED 103・計 108 に合わせた

## R21. D27（仮）の窓の分かれは「設定を開く」で窓が閉じた時にも印を立て、同じ D27 の「別の画面を前に出した回は立てない」と食い違う
- 成果物: openspec/changes/st57-update-notice/design.md
- 根拠: design.md:220「未使用のアプリの制限の確認の窓を出した回は、その窓が閉じた時（`setOnDismissListener`）に…印を立てる」。`MainActivity.kt:444-452` の窓は、肯定のボタン（`OPEN_SETTINGS_LABEL`）が `openUnusedAppRestrictionsSettings()`（:463-466。`startActivity` で設定を前に出す）を呼び、その後に窓が閉じて `setOnDismissListener` が走る（:437 のコメント「応じた・あとで・閉じた」のどれでも走る）。この道では、窓が閉じた時に前にあるのは設定の画面で、入れ替えの行は窓の後ろから設定の後ろへ移るだけ。D27 の前半（design.md:220「使用状況のアクセスの設定を前に出した回は印を立てない」）と同じ形なのに、印を立てる
- kind: technical
- 提案: 窓が「設定を開く」で閉じた回は印を立てず、設定から戻って画面を組み直した時に立てる（D27 の前半と揃える）。遅らせる側の直しなので仮のままでよい。R20 の窓の Scenario は「あとで」で閉じる形にし、「設定を開く」の形も 1 本足すかは直しの側で決める
- 処置: fixed D27 仮 —— 窓が「設定を開く」で閉じた時は印を立てず、設定から戻って組み直した時に立てる（D27 の前半と揃えた）。SHALL を「設定を前に出さずに閉じた時に数える」にし、tasks 9.1 の窓の試験で肯定のボタンで閉じた時に立たないことも見る

## R22. ロック中の定期の時の Scenario は入力の経過を固定しておらず、ロックを見ない実装でも緑になる
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md
- 根拠: spec.md:316-320「画面がロックされたまま定期の時を 3 回迎える（24 時間ずつ時計を進める）」は、その回の観測の最後の入力からの経過を言わない。tasks 8.1（tasks.md:101）も「偽の出し手が呼ばれた回数 0」だけ。試験の観測の経過が 300 秒を超えるか `IdleRead::Unavailable`（`engine.rs:85`）なら、`obs.locked`（`runtime.rs:493`）を見ずに経過だけを見る実装でも 0 回になる。Q13 の答えは「ロックしておらず」と「5 分以内」の 2 つの条件で、ロックの条件を単独で固定する Scenario が無い
- kind: technical
- 提案: WHEN を「画面がロックされたまま、最後の入力からの経過が 60 秒と読める定期の時を 3 回迎える」にする（経過の条件を満たしたうえでロックだけで止まることを見る）。tasks 8.1 の括弧にも経過 60 秒を書く
- 処置: fixed 8.1 —— ロック中の Scenario の WHEN に「最後の入力からの経過が 60 秒と読める」を足し、tasks 8.1 の括弧に `locked` が真で経過 60 秒と書いた

## R23. 通知から開く Scenario の GIVEN と WHEN は、本物の端末では同時に起きない
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md
- 根拠: spec.md:304-308 は GIVEN「通知を出せずに止まっている」・WHEN「本人がアプリの通知を押して最初の画面を開く」。今の `MainActivity` を開く入口はランチャー（`AndroidManifest.xml:83-86` の `ACTION_MAIN`）だけで、今ある通知はどれも `MainActivity` を開かない（常駐の通知は使用状況のアクセスの設定へ。`LocationService.kt:1343`・`:1452`。送り先の知らせは `SetupActivity` へ。`SetupNotifier.kt:93-96`・`LocationService.kt:1388-1392`・`:1417-1421`。気分の通知は `ACTION_VIEW` の URL へ。`FeelingNotification.kt:51-58`）。ST57 が足すチャンネル `update` の通知（tasks 9.1「押すと `MainActivity`」）は、示すまでは問い合わせないので「更新あり」「確かめられていない」が出ず、切り方の通知を出せた時には印が立っている。つまり「印が無いまま通知から開く」は試験の合成の Intent でしか起きない。deep-questions-r3.json の Q12 の context（「通知を押して開いた時は…すぐ閉じる」）の前提も同じ所を読んでいない。分かれ自体は答えの選択肢の本文にあり、守りとして残すのは害が無い。THEN も「閉じずに出す」と「印が立つ」の 2 つを束ねる
- kind: technical
- 提案: GIVEN と WHEN を「`ACTION_MAIN` でない Intent で最初の画面が開かれた」（どの入口からでも）に書き直し、tasks 9.1 の括弧に `isFinishing` と印を別々にアサートすると書く。答え（閉じずに出す）は変えない
- 処置: fixed 9.1 —— Scenario を「ランチャー以外から開かれて…」に改め、GIVEN/WHEN を `ACTION_MAIN` でない Intent で開かれる形にし、THEN を閉じないことと印の 2 つに分けた。design D24 に、今は印の無いまま `MainActivity` を開く通知が無く守りとして置くことを書いた（答えの「閉じずに出す」は変えない）

## R24. 印を遅らせた回の後、権限の結果と直後の `onResume` で `start()` が続けて走ると、行は描かれる前に組み直されて消え、印だけが残る
- 成果物: openspec/changes/st57-update-notice/design.md
- 根拠: design.md:194「印が立った後も、開いている画面からは行を消さない。次にその画面を組み直した時（次の `onResume` の読み直し）からは行を出さない」。`onRequestPermissionsResult`（`MainActivity.kt:145-147`）は `inFlight = false` にして `proceed()` を同期で呼び、`healthDone` が真で求めるものが無ければそのまま `start()`（:209-210）へ進む。直後の `onResume`（:128-131）は `inFlight` が偽なのでもう 1 回 `start()` を通る。同じ Activity で D27（仮）が印を遅らせた（使用状況のアクセスの設定を前に出した）後、戻った時に新しく求められるようになった権限（前景の位置を後で許した後の背景の位置。:165-169）を求めた回に当たる: 1 回目の `start()` が行を組んで印を立て、2 回目が行の無い画面に組み直す（他の行が無く通知から開いた画面なら `finish()`）。本人が画面を離れないまま、行が 1 フレームも描かれずに消えうる。狭い道だが、design.md:194 の「開いている画面からは行を消さない」と「次の `onResume` の読み直しから出さない」がこの道で食い違う
- kind: technical
- 提案: 行を出すかを「印が無い、またはこの Activity が既に行を出した」で決める（Activity が生きている間は行を残し、新しい Activity から出さない）と design D24 の 1 行と tasks 9.1 に書く。外へ出るものは変わらない（印を立てる時は答えのまま）
- 処置: fixed D24 —— 行を出すかを「印が無いか、この Activity が既に行を出した」で決めると design D24 と SHALL に書き、tasks 9.1 の新しい Scenario の試験で同じ Activity の中で `start()` が 2 回続いても行が残ることを見る

## R25. R21・R24 の直しで SHALL に入った 2 つ（窓が設定を開いて閉じた時は数えない・同じ画面の組み直しでは行を残す）は、tasks の試験の括弧にだけあり、Scenario の THEN に無い
- 成果物: openspec/changes/st57-update-notice/specs/distribution/spec.md
- 根拠: spec.md:195「その窓が設定を前に出さずに閉じた時に数える」・spec.md:196「示した印が無いか、その画面が開いている間に既に行を出したときに出し」。Scenario :311 の WHEN/THEN は「あとで」で閉じる形だけで、:317 の THEN は「新しく開いた最初の画面に入れ替えの行は無い」だけ。「肯定のボタンで閉じた時は立たない」と「同じ Activity の中で `start()` が 2 回続けて走っても行が残る」は tasks.md:114 の括弧にしか無い。tasks は archive で正典に入らないので、正典の Scenario からは、窓のどの閉じ方でも印を立てる実装と、組み直しで行を消す実装のどちらも落ちない（R19・R20 と同じ型）。:317 の試験は 2 つの主張を 1 本に束ねる
- kind: technical
- 提案: Scenario を 2 本足す —— 「入れ替えた携帯は、確認の窓を『設定を開く』で閉じた回は画面に出したとみなさない」（THEN: その回には印が無く、設定から戻って組み直した時に立つ）・「入れ替えた携帯は、同じ画面が開いている間に組み直しても入れ替えの行を残す」（権限の結果と直後の `onResume` で `start()` が続けて走る形）。tasks 9.1 の名指しと下限、ADDED と計の本数を合わせる
- 処置: fixed 9.1 —— Scenario「入れ替えた携帯は、確認の窓を「設定を開く」で閉じた回は画面に出したとみなさない」と「入れ替えた携帯は、同じ画面を組み直しても入れ替えの行を残す」を足し、窓の試験と新しく開いた画面の試験から束ねた主張を外した。tasks 9.1 の下限 21→23、ADDED 105・計 110

## 観点ごとの該当なし

- 観点 1: deep の 10 問の推奨は、どれも specs の Requirement と Scenario に写っている（Q1→spec:3、Q2→84、Q3→273、Q4→387、Q5→419、Q6→445、Q7→307、Q8→352、Q9→464、Q10→511）。当初案を覆したもの（旧 Q10 → C10）は、spec の画面の Requirement の「動いていない」と Linux の配布物を変えない文に、覆した後の形だけが残っている。数値の欠けは R4、要件へ戻すものは R2
- 観点 3（capability の照合）: proposal の Modified Capabilities は `distribution` だけで、`specs/` のディレクトリと INDEX.md:81・108（ST57 → `distribution`）と一致する。前倒しは無い
- 観点 5: check_chain は OK（観点 8 を含む）。ST57.md の「価値」と「完了の判定」は、deep の推奨と矛盾しない（「設定で部品ごとに確認を切る」のサーバの口がインストーラの命令であることは、Q3 で本人に問うている）。satisfies の外の要件を満たす振る舞いは、R2 の FR-113 の拡張だけ。MODIFIED の 4 つは FR-108 の Requirement の文を変えるが、FR-112 ★ 2026-11-03（更新係を別のサービスにする）から来る（spec:547・614・657・694 に ★ ST57 の注記）。MODIFIED は正典の Scenario を 1 本も落としていない（正典との突き合わせで missing 0）
- 観点 6: 対象外（spec --draft。`--answers` ではない）

## 観点 7: 走っている change との前提関係

観点 7: 該当なし（spec_overlap.py candidates が 0 件）。`openspec/changes/` で archive でないのは st57-update-notice だけ。
