# ST29 上流の成果物の独立レビュー（spec --draft・第 1 回の先読み）

対象: `openspec/changes/st29-satellite-credentials/`（proposal / specs 3 本 / design / tasks / deep / premises / provisional.jsonl / review/deep.md）、
`docs/stories/ST29.md`、`docs/requirements.md`（FR-77 / FR-90 / FR-96 / FR-101 / PERM-8 / PERM-10 / PERM-14・扉 #30）、`docs/stories/INDEX.md`、
`openspec/specs/`（`data-sensitivity` / `data-durability`）、走っている change（`st09-media-capture` / `st13-scheduled-account-fetch` / `ch01-st17-review-fixes` / `ch02-export-next-record-window`）。
Q1〜Q8 は答え待ちなので、推奨の答えを本人の答えとみなして突き合わせた。観点 6 は今回は対象外。

## 機械の検査

```
openspec validate st29-satellite-credentials --strict   → Change 'st29-satellite-credentials' is valid（rc=0）
python3 scripts/check_chain.py .                         → chain: OK (0 件 / 未回収 0 件 / warn 0 件)（rc=0）
python3 scripts/check_scenarios.py . st29-satellite-credentials
                                                         → scenarios: FAIL (担保なし 41 件)（rc=1）
python3 scripts/review_triage.py . st29-satellite-credentials
                                                         → 指摘 10 件 / 仮決め 4 件 / 要件へ戻すもの 0 件 / triage: OK（rc=0）
```

- check_scenarios の FAIL 41 件は、この change が足した Scenario（satellite-platform 36・data-durability 3・data-sensitivity 2）で、実装前なので想定どおり。
  warn（spec に無い Scenario を指す印）はどれも ST29 と無関係の既存の印
- **数え方の注意**: 担保なしが 41 件で、この change の Scenario は 53 本（tasks.md:17 の申告と一致）。差の 12 本は MODIFIED の Requirement が既存の名前のまま持つ Scenario で、
  名前が同じなので**既存の印で緑になっている**。うち中身を変えた 2 本は R12
- review_triage の「要件へ戻すもの 0 件」は R2 に関わる

## R1. Q1（A・rewrite-all）の決定そのもの（衛星の名前の形）を固定する Scenario が無い
- 成果物: openspec/changes/st29-satellite-credentials/specs/satellite-platform/spec.md
- 根拠: spec.md:79-95 —— Requirement「衛星が登録する名前は、衛星の識別子を前置した形をとる」の SHALL 4 つ（`<衛星の識別子>:<名前の部分>` の形・名前の部分は英小文字・数字・`-` の 1〜64 文字・本体の名前に `:` を含めない・`:` より前の識別子が持ち主）のうち、Scenario があるのは「本体の論理ソースの名前は区切りを含まない」（3 つ目）だけ。前置の形・名前の部分の 1〜64 文字（数値）・持ち主の決め方は Scenario に無い。tasks.md:55 は `name_owner` を単体テストで試すが、どの Scenario の印でもない（Global Constraints の「印の無い Scenario を FAIL にする」の外）。さらに名前を検査する口を作る ST38 / ST39 への申し送り（docs/handoff/ST38.md・ST39.md）に、この形で検査することが書かれていない（`grep -ln "name_owner\|名前の形\|識別子を前置" docs/handoff/*.md` → ST40.md だけ）
- kind: technical
- 提案: 名前の形を観測できる言葉で Scenario にする（例: 「`dev.example.booklog:reading` は `dev.example.booklog` の衛星の名前として扱われ、`dev.example.booklog:` ・名前の部分 65 文字・`c03-x:reading` は衛星の名前として扱われない」）。ST29 に口が無いので観測点が無いなら、`docs/handoff/ST38.md`・`ST39.md` に「名前の登録の口でこの Requirement の形を検査し、Scenario を置く」を書く。
- 処置: fixed specs/satellite-platform/spec.md —— Scenario「名前の持ち主は区切りより前の識別子で決まる」を足し（前置の形・名前の部分の 1〜64 文字・大文字・本体の前置き・持ち主）、tasks 2.1 が `name_owner` の表で印を付ける。口での検査は docs/handoff/ST38.md・ST39.md に st29-satellite-credentials R1 として書いた

## R2. FR-101 の [NEEDS CLARIFICATION] と扉 #30 の決着を要件へ戻す節が deep.md に無い
- 成果物: openspec/changes/st29-satellite-credentials/deep.md
- 根拠: docs/requirements.md:870-873（FR-101 の本文に `[NEEDS CLARIFICATION: 名前に衛星の識別子を前置する / 登録の時に衝突を断る …衛星の登録を作る Story の深掘りで本人に問う]`）/ docs/requirements.md:1534-1537（扉 #30 は「後回し可（期限: 衛星の登録を作る Story の深掘り）」）/ deep.md に「要件へ戻すもの」の節が無く、`review_triage.py` は「要件へ戻すもの 0 件」で OK を返す（scripts/review_triage.py:233-246 は節がある時だけ ★ を確かめる）
- kind: defer
- 提案: Q1 の答えを受けた時点で FR-101 の未決を本文に置き換え、扉 #30 を「決定済」にする改訂（★ 日付）が要る。deep.md に「要件へ戻すもの」の節を置いて FR-101 を挙げておけば、record / triage が戻し漏れを機械で落とせる。
- 処置: fixed deep.md —— Q1 の節に record への覚え（FR-101 の [NEEDS CLARIFICATION] を置き換え、扉 #30 を決定済に ★、「要件へ戻すもの」の節に FR-101）を書いた。答えの前に節を立てると、まだ ★ の無い FR-101 で triage が落ちるため、節は record が立てる

## R3. 「本人の答え待ち」「推奨で先に書いた」の印が specs の本文（導出元）にあり、archive で正典に入る
- 成果物: openspec/changes/st29-satellite-credentials/specs/satellite-platform/spec.md
- 根拠: spec.md:44（「深掘り Q1（推奨で先に書いた。**本人の答え待ち**）」）・spec.md:86（同じ）・spec.md:109・155・189（「推奨で先に書いた」）・spec.md:271（「推奨の形で先に書いた」）
- kind: technical
- 提案: 答えを写す段（record / `spec --answers`）が deep.md だけでなく specs の導出元のこの印も書き換えることを確かめる。推奨どおりの答えでも specs の差分が要る（答え以外の変更として扱われないよう、呼び出し元で扱いを決める）。
- 処置: fixed specs/satellite-platform/spec.md —— kind は日常の操作に効かないので technical。導出元の「推奨で先に書いた」「本人の答え待ち」の印を specs から外し、深掘りの Q 番号だけにした（答えを写す段で specs を直す要は、推奨と違う答えのときだけになる）

## R4. 読み出しの記録に「識別子の文字列でなく本体の中の番号」を入れる決定（D4（仮））が Scenario で固定されていない
- 成果物: openspec/changes/st29-satellite-credentials/specs/data-sensitivity/spec.md
- 根拠: design.md:59-60（「欄に入れるのは本体の中の番号（uuid）で、識別子の文字列（`dev.example.booklog`）ではない —— 書き換えられない表に識別子の形を焼き込まない（Q1 の答えが推奨と違っても…書き直さずに済む）」）/ data-sensitivity spec.md:74-75 は「本体の中の番号で持たせる」と書くが、Scenario（spec.md:105-108・115-118）が見るのは「番号を持つ」ことと「利用者や記録の識別子の文字列」が無いことだけで、**衛星の識別子の文字列が無いこと**を見る Scenario が無い。読み出しの記録は追記のみで、書いた行は直せない（spec.md:78）
- kind: technical
- 提案: 「読み出しの記録には中身も合言葉も残らない」か「衛星の資格情報で断った求めは…」の THEN に「行のどの列にも衛星の識別子の文字列（`dev.example.booklog`）も資格情報の値も無い」を足すか、独立の Scenario にする。
- 処置: fixed specs/data-sensitivity/spec.md —— SHALL の「持たせない」に衛星の識別子の文字列を足し、Scenario「衛星の資格情報で断った求めは、衛星と 1 本の番号つきで残る」と「読み出しの記録には中身も合言葉も残らない」の THEN に識別子の文字列が無いことを足した（tasks 3.2）

## R5. 資格情報の行を消さずに取り消しの時刻を付けて残す（C5 / D1（仮））が SHALL にも Scenario にも無い
- 成果物: openspec/changes/st29-satellite-credentials/specs/satellite-platform/spec.md
- 根拠: spec.md:158 は導出元の説明文の中で「取り消した 1 本と外した衛星の資格情報は、行を消さずに取り消しの時刻を付けて残す（D1（仮））」と書くだけで、SHALL は「取り消した資格情報を一覧に出さない」（spec.md:148）。登録簿の行を消さない SHALL（spec.md:15）にも Scenario が無い。tasks.md:44 の check-immutable の (a) は DELETE / TRUNCATE を拒むことを試すが、どの Scenario の印でもない（印を付けるのは既存の「読み出しの記録の行は書き換えも削除もできない」）。D1 の失い方は uncaptured（provisional.jsonl D1・review/deep.md R1）で、data-sensitivity spec.md:105-108 の 1 本の番号が指す先もこの行
- kind: technical
- 提案: 「THE SYSTEM SHALL 取り消した資格情報と外した衛星の資格情報の行を消さず、取り消しの時刻を持たせる」を SHALL に上げ、「取り消した 1 本と外した衛星の 1 本は、取り消しの時刻を持って残り、消す求めは拒まれる」の Scenario を置く（登録簿の行も同様）。
- 処置: fixed specs/satellite-platform/spec.md —— 資格情報の行を消さない SHALL と Scenario「外した衛星と取り消した資格情報の行は消えずに残る」を足した（tasks 1.2）

## R6. 取り消した衛星の資格情報 + 有効なログインの印の求めの扱いが、spec（401 / 衛星の求め）と design（印で通る）で食い違う
- 成果物: openspec/changes/st29-satellite-credentials/specs/satellite-platform/spec.md / design.md
- 根拠: spec.md:147（取り消した 1 本は「資格情報の無い求めと同じく 401 で断る」）・spec.md:149（外した衛星も 401）・spec.md:186（衛星の資格情報を付けた求めは、ログインの印を併せて持っていても衛星の求め）/ design.md:118（「取り消した値 + 有効な印は、印で通る」）・design.md:222（Risks に同じ）。有効な値 + 印は 403（D18）なのに、取り消した値 + 印は 2xx で通り、spec の本文はそれを言っていない。どの Scenario もこの組を試していない
- kind: conflict
- 提案: どちらに寄せるかを決めて spec に書く（厳しい側なら「`ashsat_` で始まる値を持つ求めは、引けなくても印で通さず 401」、今の design のままなら spec.md:147 の括弧を「ほかの資格情報が無ければ 401」に直す）。どちらでも Scenario を 1 本置く。
- 処置: fixed D19 仮 —— 厳しい側。`ashsat_` で始まる値が引けなければ印を見ずに 401 を SHALL にし、Scenario「取り消した資格情報はログインの印を併せて持っていても通らない」を足した。design D8 と Risks を合わせた（tasks 3.1）

## R7. 管理の口の状態の番号と断りの種別（D11）が design にしかなく、Scenario の一部は番号を持たない
- 成果物: openspec/changes/st29-satellite-credentials/specs/satellite-platform/spec.md / design.md
- 根拠: design.md:164-174（登録 201・登録し直し 200・外す 204・発行 201・取り消し 204・404・外した衛星を外す / 外した衛星への発行 409・資格情報の名前 400・本文の種別 `malformed` / `reserved` / `body_prefix` / `name_invalid` / `identifier_taken` / `name_taken` / `builtin` / `removed`）。spec が番号を持つのは登録の 400 / 409・最初の 2 つの 409・権限の 401 / 403 だけ。spec.md:140-143「外した衛星には資格情報を発行できない」の THEN は「求めは断られ」で番号が無い（同じ Requirement の他の Scenario はどれも番号を書く）。資格情報の名前 40 文字まで（spec.md:103）は Scenario が無い。識別子の上限は 65 文字を断る側だけで、64 文字を通す側が無い。design.md:108 の「登録し直しで `registered_at` は最初の登録の時刻のまま」も一覧に出る値なのに spec に無い。design は archive で正典に入らない
- kind: technical
- 提案: 発行の断り（外した衛星 → 409）を Scenario の THEN に書く。資格情報の名前 41 文字 → 400、識別子 64 文字は通る、登録し直しで登録の時刻が変わらない、を Scenario にする。種別の語は画面の理由（spec.md:305-308）が依るので、少なくとも「形 / 本体が使う / 本体の前置き / 重なり」で理由が分かれることを spec に置く。
- 処置: fixed specs/satellite-platform/spec.md —— 断りの種別を見分ける SHALL、発行の 409・資格情報の名前 41 文字の 400（Scenario を足した）・64 文字の識別子が通る Scenario・登録し直しの 200 と登録の時刻が変わらないことを THEN に足し、登録の Scenario の THEN に種別を書いた（tasks 4.1・5.1）

## R8. 「登録している衛星と同じ名前の衛星は登録されない」は主張を 3 つ束ね、名前の重なりの比べ方（空白）が決まっていない
- 成果物: openspec/changes/st29-satellite-credentials/specs/satellite-platform/spec.md
- 根拠: spec.md:74-77（同じ名前 → 409・空白だけ → 400・41 文字 → 400 を 1 本に束ね、見出しは「同じ名前」だけ）/ spec.md:40（「前後の空白を除いて 1〜40 文字」）/ design.md:86（長さの CHECK は `btrim(name)`）・design.md:92（重なりの索引は生の `name`）。名前を前後の空白ごと保存するなら `" 外部 AI"` は重なりを素通りする。spec はどちらとも言っていない
- kind: technical
- 提案: 名前の長さの 2 件を別の Scenario（「名前が空白だけか 41 文字の衛星は登録されない」）に分ける。前後の空白を除いて保存するか、除いた後で重なりを比べるかを SHALL に書き、「前後に空白を付けた同じ名前も 409」の Scenario を置く。
- 処置: fixed specs/satellite-platform/spec.md —— 名前を前後の空白を除いて保存する SHALL を足し、重なりは除いた後で比べる。Scenario を「同じ名前」（前後に空白の名前も 409）と「名前が空白だけか 41 文字」に分けた。design D7 の CHECK を `name = btrim(name)` に

## R9. 最初の 2 つの衛星について、Scenario どうしと試験の隔離の規則が噛み合わない
- 成果物: openspec/changes/st29-satellite-credentials/specs/satellite-platform/spec.md / tasks.md
- 根拠: spec.md:26（最初の 2 つは「どちらも資格情報を 0 本持ち」）/ spec.md:173（外す求めの後「資格情報も効いたままである」—— 0 本なら空の主張で、確かめるには最初の 2 つに発行するしかない）/ design.md:196・tasks.md:30（「最初の 2 つを外す試験は 409 だけを見て状態を変えない」）/ tasks.md:133（e2e は「最初の 2 つ + 3 つ」に資格情報 0〜2 本ずつを積む）。テスト DB は worktree ごとに 1 つ（crates/server/src/testdb.rs:18-28）で、資格情報の行は消せない（D7）ので、最初の 2 つに 1 本でも発行する試験・e2e・縦串が走った後は spec.md:26 の「0 本」が成り立たない（取り消せば一覧からは消えるが、並走する試験とは順序に依る）
- kind: technical
- 提案: spec.md:26 を「移行は最初の 2 つに資格情報を入れない」の形（DB を作り直した直後に限る、または「移行が入れた資格情報は 0 本」）にするか、spec.md:173 の「資格情報も効いたまま」を外す。Q3 の推奨の後半「資格情報は取り消せる」を確かめる Scenario（最初の AI に発行して取り消せる）を置くなら、取り消しまでを 1 本で閉じる。
- 処置: fixed specs/satellite-platform/spec.md —— 「0 本」を Scenario「移行は最初の 2 つに資格情報を入れない」（空の schema に当てて確かめる。design D14・tasks 1.1）に分け、「最初の AI の 2 つは外せない」から「資格情報も効いたまま」を外し、Q3 の後半を Scenario「最初の AI にも資格情報を発行して取り消せる」（1 本で閉じる。tasks 5.2）にした

## R10. tasks 5.3 と design の Risks が、`tools/check-log-private.sh` に実行時のログを探させる前提で書かれているが、この道具は書き方の静的な検査
- 成果物: openspec/changes/st29-satellite-credentials/tasks.md / design.md
- 根拠: tools/check-log-private.sh:2-10（「ログに私的データを出す**書き方**を止める」「追跡ファイル全部を見る」—— tracing の書き方・構造化フィールドの名前を見る）/ tasks.md:98-99（「`ashsat_` の値の形をログから探す…発行・取り消し・衛星の求めを流したサーバのログで 0 件」）/ design.md:225。正規表現を足しても見るのは追跡ファイルで、走らせたサーバのログは見ない。検証の「rc=0」は、値がログに出ても通る
- kind: technical
- 提案: 実行時のログは縦串（tasks 10.2 の `tools/smoke.d/…`）の終わりでサーバのログを `ashsat_[a-z0-9.-]+_[A-Za-z0-9_-]{43}` で grep して 0 件を確かめる形にする。追跡ファイルに値が入ることを止めたいなら `check-secrets.sh`（gitleaks）の規則に足す。
- 処置: fixed 5.3 —— 実行時のログは縦串（tasks 10.2）の終わりでサーバのログを値の形と発行した値で grep して 0 件を確かめる形にし、5.3 は静的な検査が通ることだけにした。design の Risks も合わせた

## R11. 全 route の試験の「`lib.rs` の `.route("` の字面の数と契約の path の数を突き合わせる」は、今の router で既に合わない
- 成果物: openspec/changes/st29-satellite-credentials/design.md / tasks.md
- 根拠: design.md:194・tasks.md:105 / `grep -c '\.route("' crates/server/src/lib.rs` → 25（`/selftest/panic` を含む）。`router()`（lib.rs:3420-3475）は 27 の path を持ち、うち `/feelings/reminder`（lib.rs:3435-3436）・`/attributes/kinds/{id}/names`（lib.rs:3450-3451）・`/session`（lib.rs:3454-3455）の 3 つは rustfmt で `.route(` と path が別の行に分かれていて字面に掛からない。ST09 は `PUT` / `HEAD /media/originals/{sha256}` を経路ごとの本文の上限（`DefaultBodyLimit`。st09 design.md:168-170）つきで足すので、別の Router を `merge` / `nest` する形になれば lib.rs の字面からは見えない
- kind: technical
- 提案: 字面の数ではなく、複数行に渡る `.route(\s*"…"` を拾う抽出にするか、契約に載らない route を落とす手段を router 自身から取る形（例: 契約の全 path を試した後、未知の path が 404 であることと突き合わせる、など）にする。tasks 6.1 に、字面で取る場合の正規表現と `merge` / `nest` を使う口の扱いを書く。
- 処置: fixed 6.1 —— 複数行にまたがる `\.route\(\s*"…"` の抽出（今の lib.rs で 27）に直し、`.merge(` / `.nest(` が lib.rs に現れたら落とすことを design D14 と tasks 6.1 に書いた

## R12. 中身を変えた MODIFIED の Scenario 2 本は、既存の印で緑になり、テストを直さなくても検査が通る
- 成果物: openspec/changes/st29-satellite-credentials/tasks.md
- 根拠: tasks.md:18（「中身を変えた Scenario（`読み出しの記録には中身も合言葉も残らない`・`台帳と導出の表は書き出しに入らない`）はテストも直す」）/ check_scenarios の結果（担保なし 41 件。この 2 本は既存の印があるので FAIL に出ない）/ tasks.md:73 の検証 `CT access_log 9`・tasks.md:117 の `CT export_ 60` は既存のテストが通れば rc=0（`export_*` のテストは今 120 本超: export_file_tests.rs 43・export_job_tests.rs 38・export_excluded_tests.rs 24・export_tests.rs 18…）
- kind: technical
- 提案: 2 本それぞれに「直したことが落ちる形」の終了条件を付ける。例: 3.2 は衛星の求めを足したテストの名前で `CT <その名前> 1`、7.1 は `export_file_tests.rs` の `ENTRIES` に `satellites.jsonl` があることを `grep -c` で確かめる（D13 の 15 → 16）。
- 処置: fixed 3.2 —— 3.2 に `CT access_log_secrets_satellite 1`、7.1 に `grep -c '"satellites.jsonl"' export_file_tests.rs` を置き、直したことが落ちる形にした

## R13. tasks の依存順: Task 3 の Scenario が、Task 4 / 5 で足す一覧と発行の口を WHEN に持つ。1.1 は WHEN と違う確かめ方に印を付ける
- 成果物: openspec/changes/st29-satellite-credentials/tasks.md
- 根拠: tasks.md:66（3.1 が受ける「使われた資格情報には最後に使われた時刻が残る」の WHEN は「衛星の一覧を読み出す」= spec.md:132、一覧の口は 4.1・発行は 5.1）・tasks.md:71（3.2 が受ける「衛星「読書ログ」の 2 本の資格情報のうち 1 本…」= data-sensitivity spec.md:107。発行は 5.1）/ tasks.md:41（1.1 は「最初から外部 AI とローカル AI が登録されている」に「DB の行で確かめ」て印を付ける。Scenario の WHEN は「画面のログインで衛星の一覧を読み出す」= spec.md:25。4.3 を飛ばしても 1.1 の印で緑になる）
- kind: technical
- 提案: 3.1 / 3.2 の段では資格情報を SQL で直に入れて試すことを本文に書き、一覧での確かめは 5.x に移すか、Task 3 を Task 5 の後ろに置く。1.1 の印は「移行を当て直しても最初の 2 つは増えない」だけにし、「最初から…」の印は 4.3 だけに置く。
- 処置: fixed 3.1 —— 3.1 / 3.2 は衛星と資格情報の行を SQL で直に入れ、`使われた資格情報には最後に使われた時刻が残る` の印は一覧の口がある 5.1 へ移した。1.1 から `最初から外部 AI とローカル AI が登録されている` の印を外し、4.3 にだけ置いた

## R14. tasks の終了条件が条件を測っていないもの 2 件
- 成果物: openspec/changes/st29-satellite-credentials/tasks.md
- 根拠: tasks.md:153（`git diff main -- crates/server/src/lib.rs | grep -c 'Caller::ApiToken'` の「削除行が 0」—— `grep -c` は文脈行・足した行も数えるので、削除行の数ではない）/ tasks.md:107（`authorize` の `Satellite` を `Ok` に戻し middleware の 403 を外した木でテストが落ちることを「1 回確かめ、結果を PR 本文に書く」—— コマンドも終了条件も無い）
- kind: technical
- 提案: 前者は `git diff main -- crates/server/src/lib.rs | grep -c '^-[^-].*Caller::ApiToken'` が 0。後者は変異を当てるコマンド（`git stash` を使わない一時 commit か patch）と「`CT every_route_refuses_satellite 1` が rc≠0」を書く。
- 処置: fixed 10.5 —— 10.5 を `grep -c '^-[^-].*Caller::ApiToken'` が 0 に、6.1 の変異は一時 commit に置いて `cargo test … every_route_refuses_satellite` が rc≠0 を見てから戻す、と書いた

## R15. Story の完了の判定「衛星を登録すると…資格情報が発行される」と、spec（登録では 0 本・発行は別の操作）の読みがずれる
- 成果物: docs/stories/ST29.md / openspec/changes/st29-satellite-credentials/specs/satellite-platform/spec.md
- 根拠: docs/stories/ST29.md:83（「衛星を登録すると、その衛星の名前の付いた資格情報が発行される」）・ST29.md:3（題も同じ）/ spec.md:51-52（登録の後「資格情報 0 本」）・spec.md:99（発行は利用者が衛星を選んで求める）・spec.md:302-303。PERM-14（「WHEN 利用者が衛星の資格情報を発行する」）は別の操作と読めるので要件とは合うが、確認バッチで完了の判定を字面どおりに確かめると「登録しても発行されない」になる
- kind: conflict
- 提案: proposal に「完了の判定の『登録すると発行される』は、登録した衛星を選んで発行できることで満たす」を 1 行書くか、Q8 の画面の問いの context に、登録と発行が別の操作であることを明示して本人に見せる。
- 処置: fixed D20 仮 —— 登録と発行は別の操作（登録しただけでは 0 本）を design D20（仮）にし、proposal に「完了の判定の読み方」を書いた

## R16. satisfies に無い要件の引用と、data-durability を同時に触る change の扱いが INDEX に書かれていない
- 成果物: docs/stories/INDEX.md / openspec/changes/st29-satellite-credentials/specs/satellite-platform/spec.md
- 根拠: (a) FR-96 は ST39 の要件（INDEX.md:62）で ST29 の satisfies（ST29.md:4）に無いが、spec.md:17・21・155 と data-durability spec.md:48・50 が導出元に引き、「登録簿の行を消さない」の根拠にしている。(b) NFR-17 / NFR-18 / NFR-19 / NFR-22 は spec.md:271 が導出元に引くが、他の画面を持つ Story は INDEX の「satisfies に無い条項」の表に 1 行足している（INDEX.md:198・279・371）。ST29 の行は無い。(c) INDEX.md:95 の訂正は data-durability に ST29 を足したが、同じ capability を走っている ST09（INDEX.md:386 の代償の注記）と CH02 が触っていることと、触る Requirement が別であることを書いていない（前例 INDEX.md:403 は書いている。proposal.md:88-94 には書いてある）
- kind: technical
- 提案: (a)(b) は INDEX の「satisfies に無い条項」の表に ST29 の行を足す（FR-96 は「行を残す土台だけ。記録を残す振る舞いは ST39」）。(c) は INDEX.md:95 の訂正に、ST09 / CH02 と触る Requirement が別で `衝突待ち` にならない理由を 1 行足す。
- 処置: fixed proposal.md —— kind は INDEX の書き漏れなので technical。docs/stories/INDEX.md の ST29 の訂正に、ST09 / CH02 と触る Requirement が別で衝突待ちにならない理由と、satisfies に無い条項（FR-96 の土台・NFR-17〜22）の表を足し、proposal の Impact に書いた

## R17. data-durability の MODIFIED が、ST29 の決定の外にあるファイル（場所・気分）を一覧に書き足している
- 成果物: openspec/changes/st29-satellite-credentials/specs/data-durability/spec.md
- 根拠: data-durability spec.md:14・19（「場所（ST21）・気分の対象の種類と気分の通知（ST17）のファイルが書かれていなかったのを、実際の書き出しに合わせて書き足した」）/ 正典 openspec/specs/data-durability/spec.md:332 には無く、crates/server/src/export_file_tests.rs:33-35 の `ENTRIES` にはある（正典の Scenario が今の実装と食い違っていた）
- kind: conflict
- 提案: 正典の取りこぼしを直す変更で、ST29 は同じ Scenario を MODIFIED するので一緒に直すのは筋が通る。ただし C / D のどれにも当たらない変更なので、第 2 回（`spec --answers`）の「答え以外の変更」の判定で拾われないよう、proposal の What Changes に 1 行で載せておく。
- 処置: fixed D21 仮 —— 正典の一覧の書き漏れを一緒に直すことを design D21（仮）にし、proposal の What Changes に 1 行書いた

## R18. 403 の本文の「固定の語」が測れる形になっておらず、衛星の求めに印を出し直さないこと（D8）が spec に無い
- 成果物: openspec/changes/st29-satellite-credentials/specs/satellite-platform/spec.md
- 根拠: spec.md:184・198（「どの応答の本文も固定の語だけで記録を含まない」—— 何の語か・どう比べるかが無い。design.md:120 は `forbidden`）/ design.md:119（「`refresh_cookie` は…衛星の求めには出さない」）。有効な印 + 衛星の値の求め（spec.md:215-218）で `Set-Cookie` が出るかは観測できるが、Scenario も SHALL も言っていない
- kind: technical
- 提案: spec.md:198 の THEN を「どの 403 の応答の本文も同じ 1 つの文字列で」にする。spec.md:218 の THEN に「応答に印を出し直す `Set-Cookie` が無い」を足す（印の寿命を衛星の求めで延ばさない）。
- 処置: fixed specs/satellite-platform/spec.md —— 403 の本文を「どの口でも同じ 1 つの文字列」にし、衛星の求めの応答でログインの印を出し直さない SHALL と、「ログインの印を併せて持っていても衛星の求めとして断る」の THEN に `Set-Cookie` が無いことを足した

## R19. specs が change の中の場所（`premises.md`・`review/deep.md`・`openspec/changes/st29-satellite-credentials/proto.html`）を指し、archive の後に指す先が無くなる
- 成果物: openspec/changes/st29-satellite-credentials/specs/satellite-platform/spec.md / specs/data-durability/spec.md
- 根拠: satellite-platform spec.md:111（`premises.md` P3）・spec.md:245（`premises.md` P7）・spec.md:272（`openspec/changes/st29-satellite-credentials/proto.html`）/ data-durability spec.md:48（`review/deep.md` R7）/ `grep -rn "openspec/changes/\|premises.md" openspec/specs/` → 0 件（正典に前例が無い）
- kind: technical
- 提案: 「深掘り P3」「独立レビュー R7」の形にするか、archive 後の場所（`openspec/changes/archive/<日付>-st29-satellite-credentials/…`）を指さない書き方にする。
- 処置: fixed specs/satellite-platform/spec.md —— 導出元を「深掘りの前提 P3」「前提 P7」「深掘り Q8 の画面の試作」「深掘りの独立レビュー R7」の形にし、change の中のパスを指さないようにした（data-durability / data-sensitivity も）

## 観点ごとのまとめ

- **観点 1**: Q2〜Q8 と C1〜C14 は対応する Scenario があり、推奨と同じことを言っている。抜けは R1（Q1 の形）・R4（D4）・R5（C5 / D1）・R7（D11 の番号・名前の 40 文字）・R9（Q3 の「資格情報は取り消せる」）。数値は識別子 64 / 65・名前 40 / 41・乱数 43 文字・400 日が Scenario にある。名前の部分の 64 文字と資格情報の名前の 40 文字は無い（R1 / R7）。当初案を覆したものは無い（第 1 回）。要件へ戻すものは R2
- **観点 2**: R6・R7・R8・R9・R18。束ねた Scenario はほかにも「外した衛星を同じ識別子で登録し直すと前の衛星の続きになる」（spec.md:175-178。主張 4 つ）・「衛星と資格情報の節は触れる大きさと幅の下限を守る」（spec.md:310-313。主張 4 つ）があるが、いずれも見出しが全体を言っているので指摘にしない
- **観点 3**: design の観測できる振る舞いは R6・R7・R18。specs に関数名・列名は無い。crate 名（`libloading` `wasmtime` `clang-sys`）は FR-77 の検査の入力として Scenario に出る（spec.md:247・256）が、検査の対象そのものなので指摘にしない。proposal の Capabilities と specs のディレクトリは一致（satellite-platform / data-sensitivity / data-durability）。INDEX の capability 表とも一致（data-durability への追加は INDEX.md:95 に理由あり。足りない点は R16）
- **観点 4**: R10・R11・R12・R13・R14。人間の確認待ちは「無し」と書かれ（tasks.md:155-160）、P6 は ST27 へ送ってある
- **観点 5**: check_chain は OK（観点 8 を含め 0 件）。完了の判定とのずれは R15。satisfies の外は R16。ST29.md の「壊してはいけないもの」（ST28 のログインと API の合言葉・資格情報を求めない口）は既存の正典の Scenario と spec.md:200-203・tasks.md:153 が受けている
- **走っている change との重なり**: ST09（`PUT` / `HEAD /media/originals`）・ST13（`GET /sources`）は全 route の試験が拾う設計（R11 の抽出の弱さだけ）。CH02 / ST09 が触る data-durability の Requirement は ST29 の MODIFIED と別（確かめた: ch02 は「書き出しは本人が消した場面を出さない」、st09 は ADDED 1 つ）。CH01 は重ならない

