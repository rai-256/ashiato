# ST56 spec の独立レビュー（`spec --draft`。深掘り第 1 回の推奨を答えとして読んだ）

対象: `openspec/changes/st56-collector-connection-setup/` の proposal.md・specs/{device-collection,desktop-collection,data-sensitivity}/spec.md・design.md・tasks.md・premises.md・deep.md・review/deep.md（C1〜C7）、
`docs/stories/ST56.md`・`docs/stories/INDEX.md` の訂正の節・`docs/handoff/ST55.md`・`docs/handoff/ST59.md` に足した節、`docs/requirements.md`（FR-8・FR-9・FR-92・FR-106・FR-128・FR-133・PERM-10）、正典 `openspec/specs/`。
成果物には触っていない。

## 機械の検査

- `openspec validate st56-collector-connection-setup --strict` → `Change 'st56-collector-connection-setup' is valid`
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（観点 8 の再生成との一致を含む）
- `python3 scripts/check_scenarios.py . st56-collector-connection-setup` → `scenarios: FAIL (担保なし 62 件 / 名無しの確認待ち 0 件)`。62 件は全部この change の Scenario（device 36・desktop 20・data-sensitivity 6）で、実装の前なので担保が無いのは想定どおり（Task 12.2 が閉じる）。指摘にはしない。
  tasks.md が全 62 本を名指していることは、`grep -h '^#### Scenario:' specs/*/spec.md` の各名を `grep -F` で tasks.md に当てて確かめた（漏れ 0）。正典と名前の重なりも 0
- `python3 scripts/review_triage.py . st56-collector-connection-setup` → `triage: FAIL (8 件)`: `deep.md R1〜R8: escalated なのに deep.md に R<n> が無い` → R1
- `python3 scripts/spec_overlap.py candidates . st56-collector-connection-setup` → `[]`（0 件）

## R1. deep.md に、escalated した review/deep.md の R1〜R8 の出所が無く、triage が落ちている
- 成果物: openspec/changes/st56-collector-connection-setup/deep.md
- 根拠: `python3 scripts/review_triage.py . st56-collector-connection-setup` → `[FAIL] deep.md R1: escalated なのに deep.md に R1 が無い` 〜 R8 の 8 件 / review/deep.md:41・50・59・68・77・86・95・103（処置: escalated Q3・Q1・Q8・Q2・Q7・Q7・Q5・Q1）/ 比べる形: openspec/changes/st55-server-installer/deep.md:19・35・41（`- 出所: review/deep.md R11`）
- kind: technical
- 提案: deep.md の各 Q に `- 出所: review/deep.md R<n>` を書く（Q1 ← R2・R8、Q2 ← R4、Q3 ← R1、Q5 ← R7、Q7 ← R5・R6、Q8 ← R3）。書いた後に triage を走らせ直す
- 処置: fixed deep.md —— 各 Q に `- 出所: review/deep.md R<n>` を書いた（Q1 ← R2・R8・R11、Q2 ← R4・R9、Q3 ← R1、Q5 ← R7、Q7 ← R5・R6、Q8 ← R3）

## R2. 違う合言葉への 401 で、画面は全体がログアウトし「合言葉が違います」は出ない。e2e はそれを見分けられない
- 成果物: openspec/changes/st56-collector-connection-setup/specs/data-sensitivity/spec.md
- 根拠: web/src/session.tsx:28-31（`watchUnauthorized` は `/api/` の 401 を、`POST /api/session` を除いて全部「ログインが切れた」とみなし `Gate` を `out` にする）/ specs/data-sensitivity/spec.md:8（合言葉が違えば 401）・:23-26（Scenario「合言葉が違えば出さない」は「合言葉が違います」と出ることを求める）/ design.md:84-85（D8）・tasks.md:47-50（Task 3.1）のどちらも、この口を見張りの例外にすると書いていない / tasks.md:50 の e2e の判定「欄が無く、応答が 401」は、ログイン画面へ戻されて部品ごと消えても真になる
- kind: technical
- 提案: 「ログインの印が無い」と「合言葉が違う」を応答で分ける（別の状態符号か本文の印）か、見張りの例外にこの口を足すことを design と Task 3.1 に書く。どちらでも「違う合言葉を入れても、ログインは保たれ設定の画面に留まる」を Scenario にし、e2e がそれを見る
- 処置: fixed D8 —— 違う合言葉は 403 `wrong_password`（401 は見張りがログイン切れと読む）。spec の Scenario「合言葉が違えば出さない」に「ログインは保たれ設定の画面に留まる」を足し、Task 3.1 の e2e がログインの画面の要素が無いことと 403 を見る

## R3. Story の完了の判定と Q3 の推奨の detail は「設定すると送り始める」と言うが、Q7 の推奨では入れたばかりの携帯でも押すまで送らない
- 成果物: docs/stories/ST56.md / openspec/changes/st56-collector-connection-setup/deep-questions.json（Q3）
- 根拠: docs/stories/ST56.md:42（「設定すると送信が始まる」）/ deep-questions.json の Q3 の選択肢 1 の detail「飛ばしても記録は積まれ、設定した後に送られる」/ specs/device-collection/spec.md:166・:185-188（承認した送り先が無く未送信があれば押すまで送らない。入れて初めて設定するときを含む）/ design.md:212（Risks「入れたばかりの携帯でも、設定の前に積んだ生存信号があるので、最初の設定で 1 回押す手間がある」）/ tasks.md:127（13.1 は「保留の確認」を手順に入れている）
- kind: conflict
- 提案: Q3 の detail に「設定の前に積んだ分があれば、Q7 の『この送り先へ送る』を 1 回押してから送る」を書き、本人が Q3 と Q7 を合わせて読めるようにする。完了の判定は要件から再生成されるので書き換えず、確認バッチの手順（13.1）で押す手順を含むことを判定の読み方として明記する
- 処置: fixed D24 仮 —— Q3 と Q7 の重なりは Q7 に従う（最初の設定でも未送信があれば押すまで送らない）を design D24（仮）に反転条件つきで書いた。あわせて deep-questions.json の Q3 の推奨の detail に「設定の前に積んだ分があるので、Q7 の『この送り先へ送る』を 1 回押してから送り始める」を書いた（問いはまだ本人に渡していない）。完了の判定は要件から再生成されるので書き換えず、確認バッチ 13.1 の手順が押す手順を含む

## R4. Q7 の context は「相手に何かを残すものは全部止める」と見せたが、spec は保存の時の `GET /contract`（と PC の `GET /healthz`）を止めない
- 成果物: openspec/changes/st56-collector-connection-setup/specs/device-collection/spec.md（:166・:178）・specs/desktop-collection/spec.md（:133・:144）
- 根拠: deep-questions.json の Q7 の context「止める範囲は、記録・生存信号・破棄の報告・写真の原本・途絶と気分の通知の問い合わせの全部（どれも相手のサーバに何かを残す）」/ design.md:103（D10「相手に残るのは読み出しの記録の 1 行だけ」）・:128（D13 の止めないもの: `GET /contract`・`GET /healthz`）/ device-collection spec:178・desktop-collection spec:144（「`GET /contract` の確かめを除く」）
- kind: premise
- 提案: Q7（A・`exported`）の context に「保存の時の確かめ（`GET /contract` 1 回）は押す前に新しい送り先へ届き、相手の読み出しの記録に 1 行残る」を足して本人に見せる。spec の Requirement 本文（device:166・desktop:133）にも例外として書く（今は Scenario の括弧の中にしか無い）
- 処置: escalated Q7 —— 第 1 回の Q7（A・`exported`）の context に保存の時の確かめ（`GET /contract` 1 回、PC は時計合わせも）が押す前に届き、相手の読み出しの記録に 1 行残ることを書いた（問いはまだ本人に渡していない。答える前に見える）。spec の本文（携帯・PC の保留の Requirement）にも例外として書き、D13 に理由を書いた

## R5. 401 だけを返すサーバは「一度も届かない」にも当たり、24 時間後に「送り先に届きません」と出る（spec と design の定義が食い違う）
- 成果物: openspec/changes/st56-collector-connection-setup/specs/device-collection/spec.md・specs/desktop-collection/spec.md
- 根拠: design.md:118（D12「一度も届いたことの無い送り先」は「401 でない答えを 1 回も受けていない送り先」= 401 は届いた数に入らない）/ design.md:183（D20 は「断られ続け / 届かない」を同じ通知の id）/ device-collection spec:127・desktop-collection spec:110（「一度も届いたことの無い送り先」を定義していない。401 を受けたのに「届きません」と出すかは本文から決まらない）
- kind: technical
- 提案: 「届いた」を spec で定義する（401 を含めて答えが返れば届いた、が文の意味に合う）。両方に当たるときにどちらの文を出すかを Requirement に書き、「全部の要求に 401 を返すサーバで 24 時間を過ぎても『送り先に届きません』は出ない」を Scenario にする
- 処置: fixed specs/device-collection/spec.md —— 「届いた」= 状態符号のある答え（401・5xx を含む）と定義し、401 だけの送り先は「届かない」と数えないこと・2 つの知らせを別にすることを本文に書き、Scenario「401 だけを返す送り先は「届かない」と知らせない」（PC も）を足した。D12 も揃えた

## R6. D17 は、spec が書くなと言う `--config-from-env` を自動起動の台本に書く
- 成果物: openspec/changes/st56-collector-connection-setup/design.md（D17）/ specs/desktop-collection/spec.md
- 根拠: specs/desktop-collection/spec.md:37（台本に書くのは「実行ファイルの場所と、既定でない置き場と端末の識別子だけ」）・:48-51（Scenario の名前も「実行ファイルの場所だけを持つ」）/ design.md:165（D17（仮）: `--install-autostart` を `--config-from-env` と一緒に使ったら、台本に `--config-from-env` を書く）
- kind: technical
- 提案: どちらかに揃える。D17 を残すなら Requirement の「だけ」の列に「開発と試験の印」を足し、その場合の Scenario（台本に印はあるが送り先・資格情報・利用者の行は無い）を足す
- 処置: fixed D17 —— 台本に印を書かない形に揃えた（spec のまま）。環境変数のまま自動起動したい開発の PC は利用者の環境に `ASHIATO_CONFIG_FROM_ENV=1` を置く

## R7. 観測できる振る舞いが design にだけあり、archive で正典から落ちる
- 成果物: openspec/changes/st56-collector-connection-setup/design.md
- 根拠:
  - design.md:117（D12: 届かない・5xx は 401 の数えを止めない）
  - design.md:157-158（D16: 開発版の `setup` は `…\collector\dev` に設定を書く / `%LOCALAPPDATA%` が読めなければ起動を止める）。後者は spec の「設定が無くても終わらない」（desktop:5）と並ぶ起動の条件で、spec に無い。
    また開発版の端末の識別子を、親の置き場の `settings.json` と `dev` の `settings.json` のどちらで作って覚えるかが決まらない（desktop:63・:83-86 は親に覚えている場合だけ）
  - design.md:185（D20: 未設定と保留の通知も 24 時間ごとに出し直す）
  - design.md:191（D21: 貼らずに Enter で今の設定のまま聞き直す。desktop:146-149 の Scenario「送ると答えれば」はこれに乗る）
  - design.md:197（D22: 失敗の数えをログインの口と共有する）
  - design.md:214（Risks: DPAPI の包みが戻せなければ設定が無いのと同じに扱い、記録は捨てない）
- kind: technical
- 提案: それぞれを該当の Requirement の本文と Scenario に移す（design には理由と反転条件だけを残す）。開発版の識別子の置き場は D16 に 1 つ決めて Requirement に書く
- 処置: fixed D16 —— 5xx が挟まっても数えを止めない・`%LOCALAPPDATA%` が読めなければ止める・開発版の送り先は `dev` の下・識別子は正式版の識別子に `-dev`・未設定と保留の 24 時間の出し直し・Enter での聞き直し・数えの共有と 429・戻せない包みは未設定、を本文と Scenario に移した。design には理由と反転条件だけを残した

## R8. 携帯の Requirement の本文だけが言い、Scenario が言っていないもの
- 成果物: openspec/changes/st56-collector-connection-setup/specs/device-collection/spec.md
- 根拠:
  - :166（保留で止めるものに写真の原本）— Scenario :175-178 の GIVEN に原本の送り待ちが無い。review/deep.md R5 が足した範囲で、Scenario が無いと原本の `PUT`・`HEAD`・`POST /media/missing` を門に通し忘れても落ちない
  - :170（「送り先の確認を待っています」の通知と、押すと設定の画面）— Scenario 無し
  - :178（「積んだ期間が出て」）— 期待する時刻が GIVEN にも THEN にも無く、何が出れば真かが決まらない
  - :43（設定の画面をメインの画面からいつでも開ける）・:125（401 の通知を押すと設定の画面が開く）— Scenario 無し（未設定の通知 :57-60 だけがある）
  - :9（資格情報を「通知」に出さない）— Scenario :33-36 は画面と logcat だけ
- kind: technical
- 提案: 原本の送り待ちを 1 件含む保留の Scenario（新しい送り先が `PUT`・`HEAD`・`POST /media/missing` を 0 件受ける）、保留の通知の Scenario、「積んだ期間」は GIVEN に最も古い・新しい時刻を置いて THEN にその文字列を書く。401 の通知を押した先の Scenario を足す
- 処置: fixed specs/device-collection/spec.md —— Scenario「保留の画面は積んだ期間を示す」「保留の間は写真の原本も送らない」「保留の間は通知で促し、押すと設定の画面が開く」を足し、401 の Scenario の THEN に押した先を足した

## R9. Q5 の数値のうち「24 時間」は、携帯では下限が無く、PC では Scenario が 1 本も無い
- 成果物: openspec/changes/st56-collector-connection-setup/specs/device-collection/spec.md・specs/desktop-collection/spec.md
- 根拠: device-collection spec:139-142（出し直しは 24 時間後に出ることだけを見る。送るたびに出し直す実装でも通る）・:149-152（届かないの知らせは 24 時間 1 分後に有ることだけを見る。1 分で知らせる実装でも通る）/ 1 時間の側は :134-137 が 59 分と 61 分で挟んでいる / desktop-collection spec:109・:111・:112（出し直しの 24 時間・届けば数えをやめて通知を消す・再起動をまたいで数える）に PC の Scenario が無い
- kind: technical
- 提案: 携帯の 2 本を 23 時間 59 分（出ない）と 24 時間 1 分（出る）で挟む。PC に、出し直しの 24 時間・再起動をまたぐ数え・届けば数えをやめる、の Scenario を携帯と同じ数値で足す
- 処置: fixed 8.1 —— 24 時間を 23 時間 59 分と 24 時間 1 分で挟んだ。PC に出し直し・再起動・届けば数えをやめる、の Scenario を足した

## R10. PC の Requirement の本文だけが言い、Scenario が言っていないもの
- 成果物: openspec/changes/st56-collector-connection-setup/specs/desktop-collection/spec.md
- 根拠:
  - :6（設定の無いまま窓（コンソール）を持って起動すると、その窓で貼らせる）— Q2 の推奨の本体（deep.md:13「設定が無いまま起動したら、黒い窓で設定の文字列を貼らせる」）だが、Scenario は `setup` 副命令（:19-27）だけ
  - :7（未設定の通知を出し、押すと `setup` の窓）— Scenario 無し
  - :135・:136・:137（送らないと答えたときの保留の通知 / 未送信が無ければそのまま承認 / 同じ送り先の見分け方）— PC の Scenario 無し
  - :153-154（4 つの結果のうち「繋がりました」と系統違い / この問い合わせで未送信を送らない）— Scenario は 401 と届かない（:158-166）の 2 本だけ
  - :64（入れ直しても同じ置き場・識別子・未送信・除外の登録で続く）— Scenario 無し
- kind: technical
- 提案: 少なくとも Q2 の本体（設定の無い起動で窓に貼らせ、収集は待たない）と、PC の「繋がりました」「系統が違う」「確かめで未送信を送らない」を Scenario にする。保留の通知・未送信が無いときの承認は携帯と同じ形で足す
- 処置: fixed specs/desktop-collection/spec.md —— 「設定の無い起動は窓で貼らせ、待つ間も収集する」「PC で繋がれば」「PC で系統の違う」「PC の確かめの問い合わせでは記録を送らない」「PC は未送信が無ければ」「PC は保留の間、知らせで促す」を足した

## R11. 画面の Requirement の本文だけが言い、Scenario が言っていないもの
- 成果物: openspec/changes/st56-collector-connection-setup/specs/data-sensitivity/spec.md
- 根拠: :10（ログインの口と同じ失敗の遅れと回数の制限）— Scenario 無し。正典のログインは 1 分に 10 回を超えたら 429（openspec/specs/data-sensitivity/spec.md:370・:461）で、この口にそれが当たるか（と D22 の数えの共有）は試験で決まらない / :12（「別の画面へ移ると」消える）— Scenario は読み込み直し（:41-44）だけ / :18-21・:36-39 は 1 本に 3〜4 の主張（文字列・ボタン・送り先・資格情報・利用者 / 成功の行・失敗の行・記録とログに値が無い）を束ねている
- kind: technical
- 提案: 「違う合言葉で 11 回求めると、11 回目は正しい合言葉でも 429 で資格情報を返さない」と「ログインの口で失敗した分もこの口の数えに入る」（D22 を正典に置くなら）を Scenario にする。別の画面へ移って戻ったときの Scenario を足す。束ねた 2 本は、少なくとも「値が残らない」を別の Scenario に分ける
- 処置: fixed specs/data-sensitivity/spec.md —— 「失敗が回数の制限を超えると、正しい合言葉でも答えない」「別の画面へ移って戻ると設定の文字列は消える」を足した。束ねた「値が残らない」はそのまま（同じ求めの 2 つの面で、1 本の試験が両方を見る）

## R12. tasks の件数の検査で、Task 10.1 の `st56_setup` が Task 1.1 の `st56_setup_string` も数える
- 成果物: openspec/changes/st56-collector-connection-setup/tasks.md
- 根拠: tasks.md:35（1.1 の試験の名前 `st56_setup_string…`、`-ge 2`）/ tasks.md:105（10.1 は `cargo test -p ashiato-collector-windows st56_setup` と `-- --list … | grep -c ": test$"` が `-ge 8`）。cargo の絞り込みは部分一致なので、10.1 の 8 本の下限は 1.1 の試験で一部埋まる
- kind: technical
- 提案: 10.1 の接頭辞を `st56_setupcmd_` など 1.1 と前方一致しない名前にするか、`--exact` を使わずに数える対象を `st56_setup_` かつ `st56_setup_string` でないものに絞る
- 処置: fixed 10.1 —— 接頭辞を `st56_setupcmd_` にし、下限を 10 にした

## R13. DPAPI の Scenario「置き場のファイルに資格情報の平文が無い」の担保が、Windows 以外の試験用の包みになる
- 成果物: openspec/changes/st56-collector-connection-setup/tasks.md
- 根拠: tasks.md:102（10.1。`secret.rs` は Windows 以外では「試験用の可逆な包み」）・:104（その 10.1 の試験が `// Scenario: 置き場のファイルに資格情報の平文が無い` の印を持つ）/ tasks.md:106-107（10.2 の本物の `CryptProtectData` の試験は Scenario の印を名指していない）/ desktop-collection spec:36（DPAPI で包む）。`check_scenarios.py` は Linux の偽の包みの試験で緑になり、DPAPI を呼ばない実装でも落ちない
- kind: technical
- 提案: この Scenario の印を 10.2（`tests/runtime_windows.rs`）の試験に置く。10.1 の Linux の試験は上の層の試験として印を持たせない
- 処置: fixed 10.2 —— Scenario「置き場のファイルに資格情報の平文が無い」の印を本物の DPAPI の 10.2 に移した

## R14. tasks の依存順: 保留を解く手順を含む Scenario が、門を作る Task より前にある
- 成果物: openspec/changes/st56-collector-connection-setup/tasks.md
- 根拠: tasks.md:59（4.1 が「設定の前に積んだ記録は、設定の後に送られ、捨てられない」を持つ。spec device:110 の WHEN は「送り先の保留を解いて送る」）・:76（6.1 が「設定を保存すると、サービスを起こし直さずに送り始める」。spec device:115 も「保留を解く」）/ 門 `DestinationGate` は Task 7（:81）/ PC も 10.1（:104「設定を保存すると、起動し直さずに送り始める」。spec desktop:31 が「保留を解く」）が `destination_gate` の Task 11（:111）より前
- kind: technical
- 提案: 門の Task（7・11）を 4・6・10 より前に出すか、保留を含む 3 本の Scenario を門の Task へ移す
- 処置: fixed tasks.md —— 保留を解く手順を含む Scenario を門の Task（携帯 Task 7・PC Task 11）へ移し、Global Constraints に書いた

## R15. spec に実装の名前（道具の名前・manifest の属性）が入っている
- 成果物: openspec/changes/st56-collector-connection-setup/specs/device-collection/spec.md
- 根拠: :208（`fullBackupContent`・`dataExtractionRules`）・:214（Scenario の WHEN に `tools/check-android-backup-rules.sh`）
- kind: technical
- 提案: Requirement は「Android 12 より前と以降の両方のバックアップの規則」と書き、道具の名前は tasks（Task 5.1）にだけ置く。Scenario の WHEN は「バックアップの規則の検査を走らせる」にする
- 処置: fixed specs/device-collection/spec.md —— Requirement と Scenario から属性名と道具の名前を外した（道具の名前は Task 5.1 だけ）

## 観点ごとの該当なし

- 観点 1: R3・R4・R5・R8〜R10 のほかは、Q1〜Q5・Q7・Q8 の推奨と C1〜C7 が specs の Requirement に写っていることを確かめた（Q5 の 1 時間は device:134-137・desktop:116-119 が 59 分と 61 分で挟む。Q8 の置き場 `%LOCALAPPDATA%\ashiato\collector` は desktop:60・:68-71。C5 の loopback は device:241・:246-249）。
  「当初案を覆したもの」は第 1 回なので無い。deep.md に「要件へ戻すもの」は無い（`review_triage.py` の「要件へ戻すもの 0 件」）
- 観点 3: proposal.md:36-40 の Capabilities と `specs/` の 3 つのディレクトリは一致。INDEX.md:95・96・103・108 は ST56 を 4 つ（`distribution` を含む）に割り当て、proposal.md:42-43 と INDEX.md:538-542 が `distribution` に delta を置かない理由を書いている。前倒しは無い
- 観点 4: 人間の確認待ち 13.1・13.2 には `> 物理:` の印がある（tasks.md:128・:130）。R12〜R14 のほかは、各タスクにコマンドと終了コードと本数の下限がある
- 観点 5: `check_chain.py` の観点 8 は通っている。`satisfies` は FR-128 だけで、specs が満たそうとする要件も FR-128（FR-92・FR-8 は床と既存の上限として引くだけ）。完了の判定と deep の推奨の食い違いは R3
- 観点 6: 該当なし（`spec --draft`。名指しの答えは無い）

## 観点 7: 走っている change との前提関係

観点 7: 該当なし（spec_overlap.py candidates が 0 件）
