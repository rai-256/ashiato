# spec-review — st55-server-installer（spec --draft。第 1 回の推奨で先に書いた版）

## 機械の検査（2026-11-03）

| コマンド | 結果 |
|---|---|
| `openspec validate st55-server-installer --strict` | valid。INFO 1 件: `data-sensitivity` の MODIFIED「記録を読むには画面の合言葉のログインが要り、API の合言葉では読めない」が正典に無い（ST44 の ADDED。ST44 の archive が先なら解ける。想定どおり） |
| `python3 scripts/check_chain.py .` | OK（要件 174 / Story 60 / 拾われた要件 172/174。対象外 NFR-8・NFR-11） |
| `python3 scripts/check_scenarios.py . st55-server-installer` | FAIL 67 件（spec 段では想定どおり）。内訳は ST55 の 66 本（distribution 60・data-sensitivity 3・data-durability 2・device-collection 1）と、ST44 の「API の合言葉では記録を送れるが読めない」1 本（ST44 の試験はまだ main に無い）。tasks の「66 本」と食い違ってはいない |
| `python3 scripts/review_triage.py . st55-server-installer` | **FAIL 3 件**: review/deep.md の R3・R6・R11 は escalated なのに deep.md に R3・R6・R11 が無い（→ R1） |
| `python3 scripts/spec_overlap.py candidates . st55-server-installer` | 候補 3 件（どれも st44-collector-credential-scope）。判定は末尾の「観点 7」 |

## R1. escalated の R3・R6・R11 が deep.md に名指しされておらず、triage が落ちている
- 成果物: openspec/changes/st55-server-installer/deep.md
- 根拠: `python3 scripts/review_triage.py . st55-server-installer` → `[FAIL] deep.md R3/R6/R11: escalated なのに deep.md に R3 が無い`。review/deep.md:87（R3 → Q6）・:121（R6 → Q7）・:170（R11 → Q3）。deep.md:5-38 の Q3・Q6・Q7 に出典の R が無い
- kind: technical
- 提案: deep.md の Q6 に「review/deep.md R3」、Q7 に「R6」、Q3 に「R11」の出典を 1 行ずつ足す（triage が読む形に合わせる）。直さないとグラフの triage が止まる
- 処置: fixed deep.md —— Q3・Q6・Q7 に review/deep.md の R11・R3・R6 を出所として足した

## R2. Windows の CI が試すのは配る組み立ての zip だが、配る組み立ては試験の差し込み（`ASHIATO_SETUP_TEST_*`）を読まないので、差し込みに乗る Scenario が CI で走らない
- 成果物: openspec/changes/st55-server-installer/design.md（D24）・tasks.md
- 根拠: design.md:184「`ci.yml` の `package-windows` が作った zip を受け」・:185「試験の差し込みは…配る組み立て（`ASHIATO_RELEASE=1`）では読まない（`check-artifacts.sh` が配る `ashiato-setup.exe` の中にその名前の文字列が無いことを見る）」。`tools/release/package.sh:20` は `ASHIATO_RELEASE=1 … "$build" server-windows`（CI の `package-windows` は `.github/workflows/ci.yml:383` でこれを走らせる）。差し込みに乗る Scenario: 「サービス専用のアカウントで登録できなければ…」（tasks.md:76 `REGISTER_FAIL`）・「移行が失敗すると…」（:97 `MIGRATE_FAIL`）・「DB を作る途中で落ちても…」（:104 `STOP_AT`）・「空きが見積もりより少なければ…」（:137 `FREE_GB`）・「最後に Tailscale の転送先を示し…」（:139 `TRACE`）・「起動を拒んだ理由が…」（:374 差し替え）ほか
- kind: technical
- 提案: 差し込みを読む試験用の組み立て（`ASHIATO_RELEASE` 無し）を同じ `package.sh` の経路で別に作り、差し込みに乗る Scenario だけをそちらで走らせる。配る zip はそれ以外の Scenario を走らせる。どちらで何を見るかを D24 に書く。でなければ「配る zip から」という Scenario の言い方を直す
- 処置: fixed D24 —— 配る zip と試験の zip（`package.sh windows --test-build`）の 2 つを作り、差し込みか命令の記録に依る Scenario は試験の zip で走らせる。spec に「試験の差し込みは試験の組み立てでだけ読む」と Scenario「配る組み立ては試験の差し込みを持たない」を足した

## R3. 管理者の権限が要る確かめは PR の CI でしか走らないので、P19（確かさ 1）が外れたかは全部のタスクを終えた後まで分からない
- 成果物: openspec/changes/st55-server-installer/tasks.md（3.1・3.2）・design.md（D3（仮）の反転条件）
- 根拠: tasks.md:71「この段の `install-check.sh` は、仮想アカウントで `pg_ctl register` が通り…を最初に見る。外れたら D3 の反転条件を上流へ返す」。`.githooks/pre-push:30-47` は `review/code.md` が無い `feat/st*` の push を止める（whole-branch review の後でしか CI が走らない）。tasks.md:12 の下流の手元の検証は Linux の `cargo test`・cross clippy・偽の `sc.exe` を置いた `--self-test` だけ。premises.md P17（runner が管理者で動く）は確かさ 2、P19 は確かさ 1
- kind: technical
- 提案: P19 と P17 を Task 4 より前に実物で確かめる手段を tasks に置く（例: Windows サンドボックスで `install-check.sh` の最初の項目だけを走らせる・人間の確認待ちに先出しする）。`install-check.sh` の最初に昇格の確かめ（High Mandatory Level）を置いて、P17 が外れたら全部が落ちる前に止める。手段を置かないなら、D3 の反転が PR の CI で初めて出うることを Risks に書く
- 処置: fixed 3.0 —— WSL から Windows サンドボックスを起こして P19 を機械で先に見る `sandbox-probe.sh p19`。design の Risks に書いた。install-check.sh は始めに上がった札を見て P17 が外れたら全部の前に落ちる。D24

## R4. いくつかの Scenario の WHEN は windows-latest の CI では作れず、試験は別の主張を確かめている
- 成果物: openspec/changes/st55-server-installer/specs/distribution/spec.md・tasks.md
- 根拠:
  - 「別の管理者の資格情報で上がっても、本人は開いた利用者になる」（spec:155-158。WHEN は UAC で B の資格情報を入れる）↔ tasks.md:78 は SID を `--owner-sid` で渡す。design.md:173「上がった方が既に管理者のとき（CI）は、起こし直さずにその場で続ける」ので、上がる前に SID を控える経路（D22）は CI で一度も通らない
  - 「最後に Tailscale の転送先を示し…」（spec:428-429。WHEN「Tailscale が入った Windows」・THEN `tailscale serve status` が同じ）↔ tasks.md:139「runner に Tailscale が無いので…命令の一覧で見る」
  - 「インストールごとに秘密が違う」（spec:177。WHEN「2 つの使い捨ての Windows」）↔ tasks.md:96「同じ runner で消して入れ直し」
  - 「配る DB は OS に依らない並べ方で…」（spec:100。WHEN「地域の設定が日本語の Windows」）。tasks に runner の地域の設定を変える手順が無い
  - 「配る zip からのインストールで…」（spec:19。WHEN「Docker も WSL も入っていない Windows」）。runner に入っていないことを確かめる手順が tasks に無い
  - 「管理者に上がらなければ決め直せない」（spec:314）↔ tasks.md:116 は管理者でない利用者で `password` の口を開く。D22 の形では `Start-Process -Verb RunAs` が答えられない資格情報の確認を出しうる。どう「通らない」を作るかが書かれていない
- kind: technical
- 提案: Scenario ごとに、CI で作れる WHEN に書き直す（例: 「`--owner-sid` に別の利用者を渡しても、ACL はその利用者で管理者が無い」と、上がる前に SID を控える命令の純粋な関数の試験に分ける）か、作れない部分を「人間の確認待ち」（12.x）に物理の印つきで移す。「通らない」を作る差し込み（昇格を断ったときと同じ戻り値）を置く
- 処置: fixed specs/distribution/spec.md —— 別の管理者は「上がった方が別の管理者でも…」と「上がる前の段は…控える」に分けた・Tailscale は命令の記録で見る・秘密は消して入れ直す 2 回で比べる・地域の設定は `Set-Culture ja-JP`（D24）・Docker と WSL は命令の記録で見る Scenario に分けた・上がらない利用者は差し込み `ELEVATION=deny` で断ったときと同じ戻り値

## R5. Requirement の本文と深掘りの推奨にあって、Scenario が 1 つも言っていない主張がある
- 成果物: openspec/changes/st55-server-installer/specs/distribution/spec.md
- 根拠:
  - 管理者の確認（UAC）を「1 回だけ」（spec:5・:289。FR-108「管理者の権限を 1 回求める」・Q3 の推奨・ST55 の完了の判定 1 つ目）。Scenario は「管理者の権限で走らせ」から始まる（spec:19）。tasks.md:166 は 12.1 を「Scenario ではない」としている
  - Q1 の推奨「PC の収集アプリは入れない」（spec:11）。zip の目録の Scenario（spec:449）は収集アプリが無いことを見ない
  - Q7 の推奨「別の場所を選ばせない」（spec:421）。選ぶ口を足しても落ちる Scenario が無い
  - 「既定のブラウザで画面を開く」（spec:261）
  - ログを日ごとに分け、DB は 7 日・サーバは 30 日（spec:366。数値）。Task 2.4 の試験は 30 日を見るが Scenario の印が無い。7 日はどこも見ない
  - DB のサービスも回復で起こし直す（spec:10）。Scenario はサーバだけ（spec:37-40）
  - 秘密を「状態の置き場の外のファイルに出さない」（spec:168）。`--pwfile` の一時のファイル（design.md:35）が消えたかを見る Scenario が無い
- kind: technical
- 提案: 機械で見られるもの（収集アプリが zip に無い・場所を選ぶ引数を受けない・7 日 / 30 日・DB の回復・`--pwfile` が残らない）は Scenario を立てる。UAC 1 回とブラウザは Scenario を立てて 12.1 の物理の印で担保するか、本文から外して design に置く
- 処置: fixed specs/distribution/spec.md —— Scenario を足した: 管理者に上がる起こし直しは 1 回だけ・DB のサービスの回復・一時の合言葉のファイルが残らない・既定のブラウザで開く・ログの 7 日 / 30 日・置き場を選ぶ口を持たない・zip に収集アプリが無い。本物の UAC の画面は人間の確認待ち 12.1

## R6. 「インストールした形」の決め方が spec と D8 で違い、照らすための値が無いときの振る舞いが食い違う
- 成果物: openspec/changes/st55-server-installer/design.md（D8）・specs/data-sensitivity/spec.md
- 根拠: data-sensitivity spec:11-12「WHERE サーバがインストールした形（サービス）で動く … 照らすための値が無いか形が違えば起動しない」・Scenario「インストールした形で照らすための値が無いと起動しない」（spec:122-125）。design.md:79「`WEB_PASSWORD_VERIFIER` があれば照らす形、無ければ今の `WEB_PASSWORD`（元の値）の形で動く。2 つが両方あれば起動しない」。D4（design.md:56）はサービスの口も設定を環境変数に置いて同じ `run()` を通る。サービスの設定に `WEB_PASSWORD` だけがあると、spec では起動しないが D8 では元の値の形で起きる。「両方あれば起動しない」は design にだけあり（tasks.md:110 は試すが Scenario が無い）。インストーラの検査（distribution spec:259・data-sensitivity spec:12）はログインの印の鍵と同じ値を断らないが、D8（design.md:81）では起動時にその一致でサーバが止まる（ST44 の「画面のログインの印の鍵はサーバだけが持つ」）
- kind: technical
- 提案: 照らす形かどうかを「サービスの口で動くか」で決めるか、spec の WHERE を「照らすための値が設定にあるとき」に書き直すかのどちらかに揃える。「両方あれば起動しない」を Scenario にする。インストーラの検査に「ログインの印の鍵と同じ値は受け付けない」を足す
- 処置: fixed D8 —— 照らす形かどうかは口で決める。service の口は照らす形だけで元の値があれば起動しない。インストーラはログインの印の鍵と同じ値も断る。specs/data-sensitivity/spec.md: Scenario「インストールした形の設定に画面の合言葉の元の値があると起動しない」

## R7. data-sensitivity の写した 2 つの Scenario はインストールした形に限定されておらず、インストールした形では成り立たない
- 成果物: openspec/changes/st55-server-installer/specs/data-sensitivity/spec.md
- 根拠: spec:10 で起動の検査を「元の値で受ける形（開発の台本で起こすサーバ）」に絞ったが、Scenario「画面の合言葉が API の合言葉と同じだと起動しない」（spec:92-95）と「画面の合言葉が無いか短いと起動しない」（spec:97-100）の WHEN はどちらの形とも書いていない。インストールした形では、API の合言葉と同じ値から作った照らすための値でもサーバは起きる（spec:12 は決める口でだけ見る）。D8（design.md:81）は印の鍵を照らすための値と照らしているので、同じことは API の合言葉でもできる
- kind: technical
- 提案: 起動の時に API の合言葉も照らすための値と照らして、同じなら起動しない（Argon2id 1 回）。そうすれば写した Scenario は両方の形で成り立つ。そうしないなら、2 つの Scenario の WHEN を「開発の台本で起こすサーバで」に絞る
- 処置: fixed D8 —— 照らす形の起動で API の合言葉とログインの印の鍵を照らすための値と照らし、合えば起動しない。specs/data-sensitivity/spec.md: 写した 2 つの Scenario の WHEN を形ごとに書いた。短い合言葉は開発の台本の形に絞った

## R8. data-durability・device-collection の MODIFIED が今の既定の文を限定しないまま残し、インストールした形で 2 つの既定が両方当たる
- 成果物: openspec/changes/st55-server-installer/specs/data-durability/spec.md・specs/device-collection/spec.md
- 根拠: data-durability spec:19「WHEN 置き場が指定されていない THE SYSTEM SHALL …サーバを動かしている利用者のローカルのアプリのデータのフォルダの下」は限定が無く、spec:20 の「WHEN インストールした形で…指定されていない → `C:\ProgramData\ashiato\export`」と両方がインストールした形に当たる。Scenario「置き場の指定が無ければローカルのアプリのデータのフォルダの下に書く」（spec:107-110）も同じ。導出元 spec:34「Windows は利用者のプロファイルの既定の権限に任せる」は、新しい WHERE（spec:23。置き場の権限を受け継がせる）と食い違ったまま残っている。device-collection spec:5（`%LOCALAPPDATA%\ashiato\media`）と spec:6、Scenario spec:33-36 も同じ形。既にある試験は開発の形で通るので、壊してはいない。なお D4（design.md:55）は設定に `ASHIATO_EXPORT_DIR`・`ASHIATO_MEDIA_DIR` を書くので、インストーラが入れた形では「指定が無い」状態にならない
- kind: technical
- 提案: 今の文と Scenario の WHEN を「開発の台本で起こす（サービスでない）とき」に限定し、spec:34 も「開発の形では」に絞る。D4 の設定に置き場を書くのか、サーバの既定に任せるのかをどちらか一方にする（両方なら、設定の値が優先されることを design に書く）
- 処置: fixed D4 —— specs/data-durability/spec.md と specs/device-collection/spec.md で、今の既定の文と Scenario をサービスの口でない形に限った。導出元の Windows の権限の行も形で分けた）・D4（インストーラは media・export・archive-copies を設定に書かず、service の口の既定に任せる

## R9. 状態の置き場の ACL を付けた後に、上がったインストーラが設定のファイルをどう書き・どう読むかが設計に無い
- 成果物: openspec/changes/st55-server-installer/design.md（D3（仮）・D10（仮）・D21）
- 根拠: D3（design.md:47-48）で書けるのは `NT SERVICE\ashiato-server`（と `db` に `ashiato-db`）だけ。`Administrators`・`SYSTEM` は付けない。D10 の段（design.md:94）は (1) ACL の後に (2) 秘密を設定に書き、(11) で照らすための値を書く。`password` の口（spec:289-291）も設定を書く。D21（design.md:166）が一時的に書きを足すのは `db\17` と `backups` だけ。また本人と違う管理者 B で上がった走らせ直し・`password`・`uninstall` は設定を読めない（B に読みの項目が無い。spec:128 で本人にしない）ので、「既にあるか」（D10）を判断できない
- kind: technical
- 提案: 設定を書く段（(2)・(11)・`password`）でも一時的に書きを足して外す（D21 と同じ形。CI の「書きの権限が残らない」で見る）か、置き場の持ち主が持つ ACL を書き換える権利を使う手順を D21 に書く。管理者 B で走らせ直したときの振る舞い（止まる・本人で開き直すよう示す）を決めて、Scenario を 1 つ足す
- 処置: fixed D21 —— 書く段の間だけ上がった利用者に一時の書きを足して外す。別の管理者 B の走らせ直しも同じ形で、本人は設定の `ASHIATO_OWNER_SID` から読んで変えない。specs/distribution/spec.md: Scenario「別の利用者が走らせ直しても、本人は変わらない」

## R10. インストールした形では、消された取り込み待ちをサーバが作り直せず、正典の external-ingestion と合わない
- 成果物: openspec/changes/st55-server-installer/specs/distribution/spec.md・design.md（D9）
- 根拠: 正典 `openspec/specs/external-ingestion/spec.md:22`「WHEN 取り込み器が起動し、専用のフォルダが無い THE SYSTEM SHALL 専用のフォルダを作る」。design.md:89 で足すのは `%USERPROFILE%\ashiato\取り込み待ち` への `M` と、ダウンロードへの `RX` だけ。親の `%USERPROFILE%\ashiato` にはサーバのアカウントの項目が無いので、本人が取り込み待ちを消すとサーバは作れない。ST55.md の「壊してはいけないもの」に external-ingestion の取り込み待ちがある
- kind: technical
- 提案: 親の `%USERPROFILE%\ashiato` にサーバのアカウントの「フォルダを作る」権限を足す（`uninstall` で外す）か、作れないときの振る舞い（生存信号に出す）を Scenario にする
- 処置: fixed D9 —— 親の `%USERPROFILE%\ashiato` にそのフォルダだけの AD と RX を足す。specs/distribution/spec.md: Scenario「取り込み待ちを消しても、サーバが作り直す」

## R11. Task 8.1 が使う `ASHIATO_ARCHIVE_SCAN_SEC` を設定すると、サーバは起動を拒む
- 成果物: openspec/changes/st55-server-installer/tasks.md（8.1）
- 根拠: tasks.md:123「`ASHIATO_ARCHIVE_SCAN_SEC` を短くした設定で待ち」。`crates/server/src/archive/config.rs:28-31` は、この環境変数があると `bail!("見回りの間隔は登録簿の s01-archive-inbox の fetch_interval_sec で設定する…")`。登録簿の上限は 300 秒（`migrations/202610211000_source_acquisition.sql:74`）
- kind: technical
- 提案: 見回りの間隔は同梱の `psql` で登録簿の `fetch_interval_sec` を書き換えて短くする、と書き直す。環境変数は使わない。Scenario の「10 分待つ」は CI の待ち時間の上限として残す
- 処置: fixed 8.1 —— 見回りの間隔は同梱の psql で登録簿の fetch_interval_sec を短くする。環境変数は使わない

## R12. Task 9.1 の検証 `tools/check-ports.sh` rc=0 は、今の台本では引数が無いので rc=2 になる
- 成果物: openspec/changes/st55-server-installer/tasks.md（9.1）
- 根拠: tasks.md:136 `tools/check-ports.sh` rc=0。`tools/check-ports.sh:12` は `[[ ${1:-} == --self-test ]] || { echo "使い方: $0 --self-test" >&2; exit 2; }`
- kind: technical
- 提案: 検証を `tools/check-ports.sh --self-test` にする（Scenario「配る番号は開発の台本の割り当てと被らない」はそこに足す）。引数なしの口を新しく作るなら、そう書く
- 処置: fixed 9.1 —— `tools/check-ports.sh --self-test` に足し、検証もそれにした

## R13. Task 5.1 の `cargo test -p ashiato-server db_role` は 0 本に当たり、rc=0 でも何も確かめない
- 成果物: openspec/changes/st55-server-installer/tasks.md（5.1）
- 根拠: tasks.md:93。`grep -rn "fn [a-z0-9_]*db_role" crates/` で試験の名前に `db_role` を含むものは 0 件（役割の試験は `app_role_tests.rs` の `app_role_*`・`server_startup.rs` の `server_startup_refuses_privileged_role`・`migrate_as_owner`）。`-- --list` の下限も無い
- kind: technical
- 提案: 当たる名前（`app_role_` と `--test server_startup server_startup_refuses_privileged_role`・`migrate_as_owner`）に替え、`-- --list` の本数の下限を同じコマンドで見る
- 処置: fixed 5.1 —— `app_role_` と `--test server_startup server_startup_refuses_privileged_role` に替え、`-- --list` の下限を同じコマンドで見る

## R14. Task 10.1 の `install-check.sh --list | wc -l` ≥ 55 は、tasks が install-check.sh に置く Scenario の数（52〜53）より多い
- 成果物: openspec/changes/st55-server-installer/tasks.md（10.1）
- 根拠: tasks.md:150。tasks が `install-check.sh` に置く Scenario は 3.2 で 12・4.1 で 6（同じメジャー版も印を置くなら 7）・5.2 で 9・6.1 で 4・7.2 で 9・8.1 で 3・8.2 で 1・9.1 で 1・9.2 で 2・9.3 で 4・10.1 で 1、計 52（53）。残りは cargo test（2.2・2.3・7.1・8.2）・`pgsql.sh`・`check-artifacts.sh`・`check-ports.sh` が持つ
- kind: technical
- 提案: 下限を、tasks が install-check.sh に割り当てた本数（数え直した値）にする。数え違いを避けるなら、spec の Scenario の一覧から cargo test などの持ち分を除いた一覧と、`--list` を突き合わせる
- 処置: fixed 10.1 —— 本数の下限をやめ、`install-check.sh --check-coverage` で spec の Scenario からほかの道具の持ち分を除いた一覧と `--list` を突き合わせる

## R15. タスクの依存の順が逆になっている（1.3 が 3.1 の crate を、3.1 が 3.2 の台本を当てにしている）
- 成果物: openspec/changes/st55-server-installer/tasks.md
- 根拠: tasks.md:43「1.3 … `ashiato-setup.exe`（Task 3 の crate を同じ版番号で組む）… を zip に入れる」。crate を作るのは 3.1（:68）。3.1（:70-71）は CI の job に `install-check.sh` を走らせ、P19 を「この段の install-check.sh」で見るが、台本を作るのは 3.2（:75）
- kind: technical
- 提案: crate の骨（3.1 の前半）を Task 1 より前に出すか、1.3 を Task 3 の後へ移す。CI の job を足すのは 3.2 の後にする
- 処置: fixed tasks.md —— 同梱の 1.3・1.4 を Task 3 の 3.3・3.4 へ移し、crate の骨 3.1 と install-check.sh・CI の job 3.2 の後に置いた

## R16. D11 の待つ上限（120 秒）と回復の間隔は観測できる振る舞いなのに design にだけあり、spec は上限なしと読める
- 成果物: openspec/changes/st55-server-installer/design.md（D11）・specs/distribution/spec.md
- 根拠: design.md:102「最長 120 秒待つ…120 秒で繋げなければ理由をログに書いて終わる」。spec:54「DB を待つ間は起動を失敗にしない」（上限なし）。Scenario は 20 秒の遅れだけ（spec:72）。`openspec archive` は design を正典に写さない
- kind: technical
- 提案: spec:54 に「120 秒まで待ち、繋げなければ理由をログに書いて終わる」を足し、上限を越えたときの Scenario を 1 つ立てる（差し込みで DB を開かない）
- 処置: fixed specs/distribution/spec.md —— 120 秒まで待ち、繋げなければ理由を書いて終わるを本文に足し、Scenario「DB が 120 秒開かなければ、理由を書いて終わる」を立てた

## R17. 「設定のファイルが無いと…」と「DB の起動が遅れても…」は、サービスの段の THEN を関数の段の試験でしか見ない
- 成果物: openspec/changes/st55-server-installer/tasks.md（2.2）
- 根拠: spec:65-68 の THEN は「サービスは実行中にならず、状態の置き場のサーバのログに…」、spec:70-73 の THEN は「DB が接続を受けた後に `/healthz` に 200」。tasks.md:56 はどちらも Linux の `st55_tests.rs` の関数の段（「関数の段: 無い・読めない・形が違う」・「偽の接続で待つ」）。design.md:185 は差し込みに「DB の遅れ」を挙げるが、install-check.sh に置くタスクが無い
- kind: technical
- 提案: 2 つを install-check.sh にも置き（設定を退けて開始する・DB の遅れの差し込み）、関数の段の試験は印の無い下支えにする。または Scenario の言い方を関数の段に合わせる
- 処置: fixed 2.2 —— 関数の段の試験は印を置かない下支えにし、2 つの Scenario は install-check.sh のサービスの段（3.2）で見る

## R18. 「配る DB はログに文と値を書かない」は 3 つの設定を見るだけで、失敗した文は既定でログに書かれる
- 成果物: openspec/changes/st55-server-installer/specs/distribution/spec.md
- 根拠: spec:113-116 の THEN は `log_statement`・`log_min_duration_statement`・`log_error_verbosity` の 3 つの値だけを見る。PostgreSQL の `log_min_error_statement` は既定が `error` で、エラーになった文をその文の字面ごとサーバのログに書く。D17 で DB のログは `logs\db` のファイルに出る（design.md:141）。spec:379 の Scenario も、構文の誤った文を DB に送っている
- kind: technical
- 提案: Scenario の名前を「配る DB はログの 3 つの設定が製造準備の既定値」に替えるか、`log_min_error_statement=panic` を D6 に足して、送った文の字面が `logs\db` に無いことを THEN にする（製造準備の棚卸し `docs/production-prep.md:235` の「SQL ごとログに落ちるのを止める」との整合は本人へ）
- 処置: fixed D6 —— `log_min_error_statement=panic` を足し、specs/distribution/spec.md の Scenario の THEN に、送った文の目印が logs\db に無いことを足した

## R19. ST55 の完了の判定の「再起動してもログオンせずに動いている」は、Q5 の推奨の逃げ道を取ると何も確かめない
- 成果物: docs/stories/ST55.md・openspec/changes/st55-server-installer/tasks.md（12.2）
- 根拠: ST55.md の完了の判定 1 つ目「PC を再起動するとログオンしなくても動いている」。deep.md:28（Q5 の推奨）と tasks.md:171「残らなければ、CI の『サービスは利用者でなくサービス専用のアカウントで動く』まで」。CI の runner は再起動できない（tasks.md:172）。premises.md P16 で、再起動の後に状態が残るかは確かさ 1
- kind: conflict
- 提案: Q5 の答えを受けるとき、逃げ道を取ると完了の判定の 1 項が確かめられないまま残ることを GO 資料に明記する。完了の判定を「開始の種類が自動で、利用者でないアカウントで動く」に読み替えるかを本人に聞く
- 処置: fixed D26 仮 —— サンドボックスで状態が残るときだけ再起動を実物で見る。残らなければ CI の自動・サービスのアカウントで代え、確認バッチに見ていないと書く。反転条件つき。provisional.jsonl

## R20. 1 つの Scenario に独立した主張を束ねたもの・spec の中の repo の名前
- 成果物: openspec/changes/st55-server-installer/specs/distribution/spec.md
- 根拠: 「消すとサービスとプログラムが消え、記録の置き場は残る」（spec:404-405。サービス 2 つ・プログラムの置き場・登録簿・DB と設定が残る・出力の場所の 5 つ）・「配る zip からのインストールで…」（spec:20。登録・自動・実行中・依存の 4 つ）・「配る DB は OS に依らない並べ方で…」（spec:101。5 つ）。repo の名前: `tools/db-roles.sh`（spec:192）・`migrations/ORDER`（spec:204）・`tools/ports.sh`（spec:346・:354）・`docker-compose.yml`（spec:92・:120）
- kind: technical
- 提案: 削除と保持は別々に壊れうるので「消すと…消える」と「消しても…残る」に分ける。repo の名前は振る舞いの言葉に替える（例: 「開発の DB と同じ役割の形」「当てる版の全部」「開発の台本が使う番号」）
- 処置: fixed specs/distribution/spec.md —— 消す Scenario を「消える」と「残る」に分けた。repo の名前（`tools/db-roles.sh`・`migrations/ORDER`・`tools/ports.sh`・`docker-compose.yml`）を振る舞いの言葉に替えた。ほかの束ねた Scenario は同じ 1 つの状態を見るので残した

## R21. tasks の `-p ashiato-setup` は package の名前を当てにしているが、design は package の名前を決めていない
- 成果物: openspec/changes/st55-server-installer/design.md（D20）・tasks.md
- 根拠: design.md:158 と tasks.md:26 は「crate `crates/installer`（bin `ashiato-setup`）」とだけ書き、`[package] name` を決めていない。tasks.md:73-74・:88・:98・:105・:117・:125・:136・:140・:144・:161 は `cargo test -p ashiato-setup` / `cargo clippy -p ashiato-setup`（`-p` は package の名前を取る）。今ある crate は置き場の名前から `ashiato-<置き場>` で名付けている（`crates/server` → `ashiato-server`、`crates/collector-windows` → `ashiato-collector-windows`）ので、下流が `ashiato-installer` にすると全部の検証が package の名前の違いで落ちる
- kind: technical
- 提案: D20 と tasks.md:26 に `[package] name = "ashiato-setup"`（bin の名前と同じ）を書く

## 観点 2: Scenario が検証可能か
- R4（CI で作れない WHEN）・R5（本文だけの主張）・R17（関数の段だけで見る THEN）・R18（名前が言うことより THEN が狭い）・R20（主張を束ねたもの）

## 観点 4: tasks が検証を持つか
- R2・R3・R11〜R15・R17・R21。人間の確認待ち（12.1〜12.3）は物理の印（device）つきで書かれている

## 観点 1: deep の決定が正典に写っているか
- Q1〜Q7 は推奨のまま（答え待ち）。Q2・Q3（2 回入れる・`password` の口）・Q4・Q6（18700）・Q7（40 GB）は Scenario にある。Q1（収集アプリを入れない）・Q7（選ばせない）・Q3（管理者の確認 1 回）は本文だけ（→ R5）。Q5 は tasks の 12.1・12.2（→ R19）
- 仮の台帳の D3・D10・D16・D17・D18 は design に `（仮）` つきで在り、premises の C<n> = D<n> の約束どおり（C1〜C18 → D1〜D18 を突き合わせた。D19〜D25 は spec 段で足したもの）
- 要件へ戻すものは 0 件（triage）

## 観点 3: 置き場の誤り
- design にだけある観測できる振る舞いは R6（両方あれば起動しない）・R16（120 秒）。spec の中の実装の名前は R20
- proposal の Capabilities（distribution・data-sensitivity・data-durability・device-collection）は specs/ のディレクトリと一致する。INDEX.md の capability 表にも ST55 を足し、訂正の節に理由がある（INDEX.md 末尾）

## 観点 5: Story が要件の現在の本文と一致しているか
- `check_chain.py` OK（観点 8 の再生成との一致を含む）。satisfies（FR-108〜FR-111・FR-132）の外の要件を specs は満たそうとしていない。data-sensitivity・data-durability・device-collection の MODIFIED は、FR-109・FR-111 の置き場と合言葉の持ち方の写しで、INDEX.md に理由がある
- FR-108〜FR-111・FR-132 の逐語と Scenario の対応で抜けているのは、FR-108 の「管理者の権限を 1 回」（→ R5）と「PC の起動時から・ログオンしていなくても」（→ R19。CI はアカウントと開始の種類まで）。FR-132 の「17 のあいだはマイナー版の差し替えだけ」は D19 の `pgsql.lock` が持ち、Scenario は無い（扉を閉じる振る舞いではないので指摘にしない）

## 観点 6: 該当なし（spec --draft。`--answers` ではない）

## 観点 7: 走っている change との前提関係
- st44-collector-credential-scope / 記録を読むには画面の合言葉のログインが要り、API の合言葉では読めない: 前提（ST55 の MODIFIED は ST44 が ADDED で置き直した本文と 17 の Scenario をそのまま写して書き換える。ST44 が変われば書き直し。ST44 は requires に在る）
- st44-collector-credential-scope / API の合言葉で通る口は、送る口と通知に要る読みの許可の一覧だけ: 前提（ST55 の MODIFIED の本文 spec:15 がこの名前の一覧を当てにしている。ST44 は requires に在る）
- st44-collector-credential-scope / すべての API 要求は資格情報を要求する: 前提（写した導出元 spec:19 がログインの口をこの Requirement の例外として名指しし、ST55 の Scenario はログインの口と資格情報の無い `/healthz` が ST44 の形であることを当てにしている。ST44 は requires に在る）
- 処置: fixed D20 —— `[package] name = "ashiato-setup"` を書き、tasks.md の名前の行にも書いた

