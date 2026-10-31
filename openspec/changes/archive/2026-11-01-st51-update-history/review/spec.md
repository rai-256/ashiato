# ST51 spec の独立検査（spec-review）

対象: `openspec/changes/st51-update-history/` の proposal.md・specs/（distribution・collection-coverage・device-collection・desktop-collection・data-durability）・design.md・tasks.md・provisional.jsonl、
`docs/stories/ST51.md`・`docs/stories/INDEX.md` の末尾の訂正の節。deep.md は第 1 回の推奨で先に書いた版で、推奨を答えとして読んだ。
照合に使ったコード: `crates/server/src/lib.rs`（`run_migrate`・`migrate_after`・`run`）・`migration_ledger.rs`・`grants.sql`・`heartbeat.rs`・`export.rs`・`main.rs`、
`crates/server/tests/server_startup.rs`、`crates/collector-windows/src/heartbeat.rs`・`runtime.rs`・`outbox.rs`、ST49 の delta（`origin/main` と `feat/st49-record-component-version` の両方）。

## 機械の検査（先に写す）

- `openspec validate st51-update-history --strict` → valid。INFO 2 件:
  `desktop-collection` / `device-collection` の MODIFIED「…記録を未送信に積む時点で自分の版を焼き込み、溜めた行の欄を落とさない」が「not found」で、**今は archive が断る**（ST49 の ADDED がまだ正典に無い）→ R15
- `python3 scripts/check_chain.py .` → chain: OK（観点 8 の再生成との一致を含む）
- `python3 scripts/check_scenarios.py . st51-update-history` → FAIL（担保なし 54 件）。内訳は ST51 が印を置く 41 本（`台帳と導出の表は書き出しに入らない` は既存の印あり）と、ST49 の delta から写した 13 本（ST49 の下流が置く）。実装前なので想定どおり
- `python3 scripts/review_triage.py . st51-update-history` → **FAIL 1 件**「deep.md R1: escalated なのに deep.md に R1 が無い」→ R1
- `python3 scripts/spec_overlap.py candidates . st51-update-history` → 3 件（すべて `st49-record-component-version`）→ 観点 7
- 追加で `scripts/evidence.py` の `declared()` で tasks.md の各項目から実際に走るコマンドを取り出した → R12・R13

## R1. review/deep.md の R1 を escalated にしたのに、deep.md に R1 が書かれておらず triage が落ちる
- 成果物: openspec/changes/st51-update-history/deep.md
- 根拠: `python3 scripts/review_triage.py . st51-update-history` → `[FAIL] deep.md R1: escalated なのに deep.md に R1 が無い` / review/deep.md:26（R1 の処置「escalated —— Q2 … を足した」）/ deep.md:10-13（Q2 の見出し・問い・答えのどこにも `R1` が無い）
- kind: technical
- 提案: deep.md の Q2 の見出しか問いの行に `（review/deep.md R1）` を書き足す。グラフを止めるのはこの 1 件だけなので最初に直す
- 処置: fixed deep.md —— Q2 の問いの行に「（review/deep.md R1）」を書き足した

## R2. 「DB の誤りの本文を S-01 のログにも残さない」を確かめる Scenario が無い。今のコードは標準エラーへ出しており、23505 を選んだ Scenario では行への漏れも見分けられない
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md
- 根拠: specs/distribution/spec.md:11（「更新の履歴にも S-01 のログにも残さない」）と :50-55（THEN は更新の履歴の列だけを見る）/ crates/server/src/main.rs:5-9（`run_migrate` の `Err` をそのまま `main` が返し、anyhow がエラーの連なりを標準エラーへ出す）/ crates/server/src/lib.rs:334（`return Err(e)`）/ crates/server/src/migration_ledger.rs:96-98（`Version` の Display が `{source}`、つまり DB の誤りの本文を含む）/ tasks.md:41・2.1（「ログにも出さない」は Global Constraints の散文だけで、検証が無い）。
  あわせて、一意の制約に反したとき（23505）の誤りの本文 `duplicate key value violates unique constraint "…"` は値を含まず、値は detail（`Key (x)=(値)`）にしか出ない。本文を行に書いてしまう実装でも、:55 の「入れた行の値の文字列が無い」は通る。また行の形は D4 で固定されていて、本文を入れられる自由な文字列の列がそもそも無い
- kind: technical
- 提案: Scenario を 1 つ足す。「移行が落ちた回、migrate の標準エラーと標準出力に入れた行の値の文字列が無い」とし、bin の試験（ST48 の `st48_migrate_content_failure_exits_1` と同じ組み方。server_startup.rs:787-803）で見る。値が誤りの本文そのものに出る誤り（例: 型の入力の誤り 22P02 `invalid input syntax for type integer: "<値>"`）で試し、detail を持つ 23505 も別に試す。Task 2.1 に、`run_migrate` が `Version` の誤りを出すときの文言を種別・版の名前・SQLSTATE だけにする、と書く
- 処置: fixed specs/distribution/spec.md —— Requirement に「標準出力・標準エラーにも残さない。示すのは種別・落ちた版の名前・SQLSTATE だけ」を足し、bin の Scenario「移行が落ちた migrate は終了コード 1 で終わり、標準エラーに行の値を出さない」を足した。行の Scenario は値が本文に出る 22P02 に変えた。D8 と tasks 2.1・2.3 に書いた

## R3. 「移行が落ちた回」の Scenario は終了コードと行の中身と本文の不在を束ねていて、置き場（同じプロセスの中の試験）では終了コードを見られない
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md, openspec/changes/st51-update-history/tasks.md
- 根拠: specs/distribution/spec.md:50-55（THEN「終了コードは 1 で…」+ AND「どの列にも…無い」）・:20-24（「終了コードは 0 で…」）/ tasks.md:69-75（2.2 は `update_history_tests.rs`。「試験用の並び = `MIGRATIONS` + 試験用の版。印は引数で渡す」—— 終了コードを決める `run_migrate` を通らない）/ tasks.md:81-86（2.3 の bin の試験は退避の失敗・db_ahead・印の 3 本だけで、落ちた回の配線を見ない）
- kind: technical
- 提案: 終了コードと行の書き込みの配線は bin の試験で見る（server_startup.rs:787-803 の「表を置き換えて版の行を消す」で落ちる版を作れる。その場合 SQLSTATE は 23505 にならないので、Scenario の値はその組み方に合わせる）。同じプロセスの中の試験に残すなら、THEN から終了コードを外し、`run_migrate` と試験が同じ関数（並び・印・退避を引数に取る）を通ることを Task 2.1 に書く。AND の 2 つ目は R2 の Scenario に分ける
- 処置: fixed specs/distribution/spec.md —— 行の Scenario から終了コードを外して当てる処理の返り値にし、終了コードと標準エラーは bin の Scenario に分けた（tasks 2.3）

## R4. 「退避に失敗した回」の GIVEN は、置き場とした bin の試験では作れない
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md
- 根拠: specs/distribution/spec.md:65（GIVEN「試験用の版を 1 つ並びの末尾に足した D-01」）/ tasks.md:82（`st51_bin_migrate_backup_failed_row` は bin の試験。bin の `MIGRATIONS` は `build.rs` が組み込む並びで、試験から版を足せない）/ crates/server/src/migration_ledger.rs:372（退避 `before_apply` は当てる版の有無に関わらず呼ばれる）・server_startup.rs:691（`st48_migrate_backs_up_even_with_nothing_to_apply`）
- kind: technical
- 提案: GIVEN から「試験用の版を足した」を外す（当てる版が無くても退避は走るので、Scenario の主張は変わらない）。THEN の「当てた版の名前は空」はそのまま成り立つ
- 処置: fixed specs/distribution/spec.md —— GIVEN を「別の印の通った行を置いた D-01 と退避が失敗する設定」にした

## R5. 「表が無い D-01 で落ちても」の Scenario は、名前が終了コードを言い、THEN は関数の返り値と警告の文言を見ている
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md
- 根拠: specs/distribution/spec.md:87-91（名前「migrate の終了コードは変わらない」、THEN「当てる処理は失敗を返し、警告に「更新の履歴を書けなかった」があり」）/ design.md:88（警告は `kind = "update_history_unwritten"`。Scenario の日本語の文言と、どちらで判定するかが決まっていない）/ tasks.md:77（同じプロセスの中の試験）
- kind: technical
- 提案: 名前を THEN に合わせる（「…落ちても、当てる処理は落ちた版の失敗を返し、更新の履歴を書けなかったことを警告する」）か、THEN を終了コードに合わせて bin の試験へ移す。警告の判定は種別の文字列（`update_history_unwritten`）にし、Scenario か Requirement に書く
- 処置: fixed specs/distribution/spec.md —— Scenario の名前を THEN に合わせ、警告を種別 `update_history_unwritten` で判定すると Requirement と Scenario に書いた

## R6. migrate が付与で落ちた回（版は当たった・終了コード 1）と、問い合わせそのものが落ちた回に、何を書くかが決まっていない
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md, openspec/changes/st51-update-history/design.md
- 根拠: crates/server/src/migration_ledger.rs:386-391（版を当て終えた後の付与の失敗は `MigrateError::Grants`）・:114-118（`MigrateError::Db`）/ specs/distribution/spec.md:5・:8-9（結果は 4 つで、付与の失敗は無い）/ design.md:74-81（D8 の表にも無い）
- kind: technical
- 提案: 付与の失敗は「D-01 は新しい版に動いた・終了コード 1」なので、`applied` にすると成功に見え、書かないと版の変化が残らない。D8 と Requirement に 1 行足す（例: `failed` に `failed_version` を空・`sqlstate` を付与の誤りの符号で書く、など）。`Db`（錠・BOOTSTRAP・指紋の読み出しの失敗）も、書くか書かないかを決めて Requirement に書く
- 処置: fixed D8 —— 付与・錠・版の記録の失敗も、表を書けるなら `failed`（落ちた版は空・SQLSTATE は取れれば）で書くと Requirement と D8 の表に足し、Scenario「付与で落ちた migrate は、落ちた版の名前を持たない failed の行を残す」を足した

## R7. 版の変化の並びの分け方が、`c01-`・`c02-` 以外の論理ソースと端末の無い信号について決まっていない。分け方の Scenario は端末と部品を同時に変えるので、片方を忘れた実装でも通る
- 成果物: openspec/changes/st51-update-history/specs/collection-coverage/spec.md
- 根拠: specs/collection-coverage/spec.md:43（部品は `c01-`・`c02-` の頭だけで決める）/ crates/server/src/heartbeat.rs:16（`device_id: Option<String>`）/ 正典 collection-coverage:53（登録簿にあるソースなら受ける。`s01-archive-inbox` など `s01-` のソースもある —— `grep -rhoE '"s01-[a-z0-9-]+"' crates/server/src`）/
  specs/collection-coverage/spec.md:90-94（端末 `d1` の C-01 と端末 `d2` の C-02 —— 端末だけで分けても、部品だけで分けても行は 0 になる）
- kind: technical
- 提案: Requirement に「頭が `c01-`・`c02-` でない論理ソースの信号は並びに入れない」（か、部品を何とするか）と「端末の無い信号の扱い」を書く。分け方の Scenario を 2 つに分ける: 同じ端末 `d1` で C-01 と C-02 が違う版を交互に申告 → 0 行（部品で分けている）、同じ部品 C-01 で端末 `d1`・`d2` が違う版を交互に申告 → 0 行（端末で分けている）。利用者で分けることも 1 本で見る
- 処置: fixed specs/collection-coverage/spec.md —— `c01-`・`c02-` 以外の信号は並びに入れない・端末の無い信号は「端末が空」の 1 つの並び、を Requirement に書き、分け方の Scenario を部品・端末・利用者・収集アプリでないソースの 4 本に分けた（tasks 5.1）

## R8. 生存信号の版の列を `client_version` で凍結すると、同じ「収集アプリの版」が D-01 と書き出しで 2 つの名前になる
- 成果物: openspec/changes/st51-update-history/specs/collection-coverage/spec.md, openspec/changes/st51-update-history/specs/data-durability/spec.md
- 根拠: specs/collection-coverage/spec.md:5・:13（列 `client_version` を「ST49 の取り込み口の欄と同じ」名前で固定し、版をまたいで変えない）/ ST49 の移行 `git show feat/st49-record-component-version:migrations/202610251000_record_component_version.sql`:6・:12・:14（記録・破棄の報告・原本の欠けの列は `collector_version`。`client_version` は要求の欄の名前）/ ST49 specs/data-durability:5（書き出しは「表の列と同じ名前」）/ design.md:48-53（D4（仮）: 列の名前は後から変えない）
- kind: technical
- 提案: 凍結する前に、列の名前を ST49 の表の列と同じ `collector_version` に揃えるか（要求の欄は `client_version` のまま）、`client_version` にする理由を Requirement に書く。D4 で後から名前を変えられないので、ここで決める
- 処置: fixed specs/collection-coverage/spec.md —— 行の列と書き出しの列を ST49 と同じ `collector_version` にし、要求の欄と原文の鍵は `client_version` のままにした（ST49 と同じ分け方）。design D4・D6・D11・D14、tasks を合わせた

## R9. 観測できる振る舞いが design / tasks にだけあり、archive で正典から落ちる
- 成果物: openspec/changes/st51-update-history/design.md, openspec/changes/st51-update-history/tasks.md, openspec/changes/st51-update-history/specs/
- 根拠:
  - design.md:70（D7「2 つの S-01 が同時に起動しても 2 行にならない」）—— 試験はある（tasks.md:79 `st51_migrate_concurrent_one_row`）が Scenario が無い
  - design.md:84（D8「印が取れない組み立て（`unknown`）も、その文字列のまま書く」）—— spec に無い
  - tasks.md:101（4.1「U+0000 は取り除く」）—— 受け口が送られた値を変える振る舞いで、spec にも design にも無い。:6 の「値の形を検査せず」と並べると読み手には逆に見える
  - specs/collection-coverage/spec.md:6（「空の文字列なら列を空（NULL）にする」）—— Requirement の本文だけで Scenario が無い
- kind: technical
- 提案: 同時の書き込みは distribution の Requirement 1 に Scenario を足す（「同じ印の 2 つの書き込みが同時に走っても行は 1 つ」）。`unknown` の扱いと U+0000 の扱いを Requirement に書き、それぞれ Scenario を足す。空の文字列 → NULL の Scenario を足す
- 処置: fixed specs/distribution/spec.md —— 同時の書き込みで 1 行・`unknown` の印をそのまま書く、を Requirement と Scenario に足し、collection-coverage に U+0000 の除去と空 → NULL の Requirement と Scenario を足した

## R10. 書き出しの説明書の Scenario は節があるかしか見ず、Requirement が決めた 3 つの説明の中身を確かめない
- 成果物: openspec/changes/st51-update-history/specs/data-durability/spec.md
- 根拠: specs/data-durability/spec.md:49-50（説明の中身を 3 つ決めている: 「前の版が空なら記録の始まり」「落ちた回の行は、退避から D-01 を戻すと消えることがある」「S-01 は確かめていない」「空なら申告なし」）/ :74-77（THEN は節と説明が「ある」ことだけ）。ST49 の同種の Scenario は文言を名指す（ST49 specs/data-durability:30-38「「S-01 は確かめていない」が書かれている」）
- kind: technical
- 提案: THEN に、決めた語句（少なくとも「戻すと消えることがある」「S-01 は確かめていない」「空なら申告なし」）が各節に書かれていることを足す。書き出しは外に出て後から直せないので、文言は試験で固定する
- 処置: fixed specs/data-durability/spec.md —— 説明書の Scenario の THEN に 3 つの語句を足した

## R11. MODIFIED の「溜めた行が知らない欄を持っていても捨てずに送る」が生存信号の溜めにも及ぶのかが読めない。C-02 の生存信号の溜めは型で読み戻すので、及ぶなら今は満たさない
- 成果物: openspec/changes/st51-update-history/specs/desktop-collection/spec.md, openspec/changes/st51-update-history/specs/device-collection/spec.md
- 根拠: specs/desktop-collection/spec.md:6・:9（生存信号の文を足したので、:7-9 の「未送信」「溜めた行」が生存信号の溜めも指すように読める）/ crates/collector-windows/src/runtime.rs:228（`Outbox<HeartbeatRequest>`）・outbox.rs:44（`serde_json::from_str::<T>` —— 型に無い欄は読み戻しで落ちる）/ ST49 design.md:145-148（D13: ST49 が欄を守るのは記録の溜め `events`）/ tasks.md:141（7.1 に生存信号の型へ知らない欄を持たせる作業が無い）
- kind: technical
- 提案: :7-9 の主語を「記録の未送信」と「生存信号の未送信」に書き分ける。生存信号にも「知らない欄を捨てない」を課すなら、Scenario（PC・携帯の両方）と Task 7.1 / 8.1 の作業を足す。課さないなら、文を記録の未送信に限る（ST49 の本文の範囲に戻す）
- 処置: fixed specs/device-collection/spec.md —— 「知らない欄を捨てずに送る」を記録の行に限り、生存信号の溜めには求めないと書いた（desktop-collection も同じ）

## R12. tasks の「何本以上」は機械が確かめない。jq と XML の件数の検証は走らない
- 成果物: openspec/changes/st51-update-history/tasks.md
- 根拠: `scripts/evidence.py` の `declared()` で取り出したコマンド: `CT`・`CW`・`GT` の項目は略記の定義の最初の backtick（試験を走らせるコマンド）だけに展開され、`-- --list | grep -c ': test$'` の本数は括弧の中の散文（tasks.md:21-26 の定義・:59・:80・:110・:124・:137・:148・:158・:165）。
  別の backtick に書いた 2.3・3.2 の `grep -c` は 1 本以上で rc=0 になるので「3 以上」は見ない。4.2 の `jq -e …` と 8.2 の `cat … | grep -c '<testcase '` は `CMD_LIKE`（evidence.py:67）に当たらず走らない（4.2 は 2 本・8.2 は 2 本だけが取り出された）
- kind: technical
- 提案: 本数を 1 つのコマンドに畳む（ST49 の `VT` の形）。例: `bash -o pipefail -c '<試験を走らせる> && test "$(cargo test -p ashiato-server <名前> -- --list 2>/dev/null | grep -c ": test$")" -ge <本数>'`。略記の定義を `CT <名前> <本数>` の 2 引数にできないなら、各項目に畳んだコマンドを書く。`jq` と XML の件数は `bash -c '…'` で包む
- 処置: fixed tasks.md —— 略記をやめ、各検証を試験と本数の下限を 1 つにした `bash -c '… && test "$(… -- --list | grep -c …)" -ge N'` で書いた。jq と XML の件数も bash -c で包んだ

## R13. 実装の項目 2.1・3.1・4.1・6.1・7.1・8.1 に検証が無く、証跡なしで [x] にできる
- 成果物: openspec/changes/st51-update-history/tasks.md
- 根拠: `evidence.py` の `names_verification()` が 2.1・3.1・4.1・6.1・7.1・8.1 で False（evidence.py:161-163「名指さない項目は証跡を要らない」）/ tasks.md:66-68（2.1 は `run_migrate` の配線で、その配線を通る試験は 2.3 の 3 本だけ —— 落ちた回の配線は R3 のとおり誰も見ない）/ :90（3.1）
- kind: technical
- 提案: 各実装の項目に、直後の試験の項目と同じ検証を書く（2.1 は 2.2・2.3、3.1 は 3.2 …）。少なくとも 2.1 には、落ちた回を bin で見る試験（R3）を検証として名指す
- 処置: fixed tasks.md —— 実装の項目 1.1・2.1・3.1・4.1・6.1・7.1・8.1 に、組み立てと直後の試験の検証を書いた

## R14. D2（仮）の反転条件は今の振る舞いと同じで、反転にならない。D4（仮）の反転条件は形の足し方の規則で、反転の条件ではない
- 成果物: openspec/changes/st51-update-history/design.md
- 根拠: design.md:40（D2 の反転条件「起動の行を migrate の後でない入れ替えのときだけにする（今もそうなっている…）」）/ design.md:53（D4 の反転条件「列を足して古い列を残す」—— 固定をやめる条件ではなく、固定したまま変える手順）/ provisional.jsonl:2・:4（D2・D4 は仮決めとして確認バッチに出る）
- kind: technical
- 提案: D2 は「何が分かったら、どちらの口をやめる（狭める）か」を書く（例: 起動の行が書けないことで起動が遅れる・実行時の役割に INSERT を渡すことが問題になったら起動の口をやめる、など）。D4 は「形を固定しない側に戻すのはどの場合か」を書くか、戻す場合が無いなら仮でなく確定の D にして台帳から外す。provisional.jsonl の D1〜D5 と design の D1〜D5（仮）の対応・出所（R2・R5・R3・R11・R6）は一致していた
- 処置: fixed D2 —— 反転条件を「起動の読み書きが起動を遅らせる・実行時の役割の INSERT が守りの穴になると分かったら、起動の口をやめる」にした。D4 は「列の追加で表せない詳細が要ると分かったら、jsonb の列を 1 つ足す側に変える」にした

## R15. ST51 の delta は ST49 の archive の後でしか archive できないが、proposal / design はそれを書いていない
- 成果物: openspec/changes/st51-update-history/proposal.md, openspec/changes/st51-update-history/design.md
- 根拠: `openspec validate st51-update-history --strict` の INFO「Archive would refuse this delta: … MODIFIED failed for header … - not found」（2 件）/ proposal.md:44-46・design.md:128（spec が変わったときの写し直しは書いてあるが、archive の順は無い）
- kind: technical
- 提案: design の Risks に「ST51 の archive は ST49 の archive が main に入った後（MODIFIED の相手が正典に要る）」と書く。グラフの requires が archive の順まで縛らないなら、ST51 の archive の前に `openspec validate --strict` の INFO が消えたことを見る
- 処置: fixed design.md —— Risks に「ST51 の archive は ST49 の archive の後。archive の前に validate の INFO が消えたことを見る」を足した

## R16. C-01 の版の変化は、上書きの更新で端末識別子が残る前提（確かさ 2）に載るが、tasks が確かめない
- 成果物: openspec/changes/st51-update-history/tasks.md, openspec/changes/st51-update-history/premises.md
- 根拠: premises.md:21（前提 8「上書きで更新しても残り」—— 知識・公式文書の URL は未確認・確かさ 2）/ docs/stories/ST51.md:48（完了の判定「収集アプリを更新すると…版の変化が 1 行残り」）/ specs/collection-coverage/spec.md:43（端末ごとに並べる。端末識別子が変われば「申告の始まり」になり、変化の行は出ない）/ tasks.md（前提 1・2・3・4 は試験で確かさ 4 へ上げるが、8 を扱う項目は無い）
- kind: technical
- 提案: 前提 8 の出所（Android の公式文書）を premises.md に書いて確かさ 3 にするか、エミュレータの計測テスト（`adb install -r` の後に `resolveDeviceId` が同じ値を返す）を 1 本足して 4 に上げる。後者は人間の確認待ちでなくエミュレータで走る
- 処置: fixed premises.md —— 前提 8 に載る決定は導くビュー（D5。計算し直せば戻る）だけで、外れても生存信号の行と版は残り失うものが無いことを書いた。`loss` のある決定はこの前提に載らないので、確かさ 2 のまま試験を足さない

## 観点 1: deep の決定が正典に写っているか — 該当なし
確かめた範囲（推奨を答えとして読んだ）:
- Q1（1 行の中身・誤りの本文は残さない）→ specs/distribution/spec.md:5-11・:99（列の表）・:20-91 の Scenario。結果の 4 つの値と SQLSTATE・当てた版の名前まで写っている（ただし本文を残さないことのログ側の検証は R2）
- Q2（D-01 の中だけ・戻した後の書き直しは ST58）→ specs/distribution/spec.md:127・:143-147、specs/data-durability/spec.md:49（説明書「戻すと消えることがある」）、docs/handoff/ST58.md の st51 の節
- Q3（生存信号から導くビュー・書き出しに出す）→ specs/collection-coverage/spec.md:42・:47、specs/data-durability/spec.md:47
- Q4（端末・部品ごと・発信の時刻の順・版なしを外す・最初は申告の始まり・原文からも読む）→ specs/collection-coverage/spec.md:43-45 と :54-100 の 8 本（分け方の Scenario の弱さは R7）
- Q5（画面・API・格子・途絶の判定を変えない）→ specs/collection-coverage/spec.md:104-120
- 数値を含む決定は無い。「当初案を覆したもの」「要件へ戻すもの」は無い（deep.md・review/deep.md の処置）

## 観点 5: Story が要件の現在の本文と一致しているか — 該当なし
確かめた範囲: check_chain.py は OK（再生成との一致を含む）。ST51.md:5・:7 の requires / layer と stories.json・INDEX.md:13-14・:74 が一致。完了の判定（ST51.md:47-49）の「1 行残る」「書き換えられない」と D5（導くビュー）の読み替えは review/deep.md R6 → D5（仮）・Q3 で扱い済み。
specs が満たそうとする要件は FR-121（device / desktop の写しは ST49 の FR-122 の本文）で、satisfies を越えない。capability の前倒しは無く、device-collection・desktop-collection・data-durability に ST51 を足した理由は INDEX.md の末尾の訂正の節にある

## 観点 6: 答えが写ったか — 該当なし（`spec --answers` ではない。第 1 回の推奨で先に書いた版）

## そのほか、確かめて指摘にならなかったもの
- **ST49 の写し**: `openspec/changes/st49-record-component-version/specs/{device,desktop}-collection/spec.md` は `origin/main` と `feat/st49-record-component-version` で同じ（`git diff` が空）。ST51 の MODIFIED の本文は、「生存信号には書き込まない」を外して生存信号の文（★ ST51）を足したことと、導出元・理由の行（design の D 番号に「（ST49）」を添えた）のほかは一字一句同じ。
  Scenario は ST49 の「…生存信号には版が書き込まれない」を外し、2 本足した（device 8 → 9・desktop 7 → 8）。外していない Scenario の本文も同じ
- **distribution の成立**: 落ちた版は `apply_one` の取引ごと戻り、接続は錠を持ったまま使えるので、別の取引で書ける。`backup_failed` の時点では BOOTSTRAP 前で何も当たっていないが、ST51 の移行が前の回に当たっていれば表はある。起動の口は版の検査を通った後なので ST51 の移行は必ず当たっている。
  実行時の役割の INSERT は `grants.sql:6`（全部の表）と `:8`（sequence の USAGE）で足りる（`seq` は IDENTITY）。D10 の REVOKE は付与の SQL が流れる成功の回だけ当たるが、書き換えの門が全部の役割で断るので Scenario は崩れない
- **data-durability の MODIFIED**: 正典 openspec/specs/data-durability/spec.md:365-401 の本文 5 文と Scenario 5 本を落とさずに写し、★ ST51 の 1 行と、Scenario「台帳と導出の表は書き出しに入らない」のファイルの一覧への 2 つだけが違う。
  ビュー `core.collector_version_change` は `read_inventory_in` の表の一覧（`table_type = 'BASE TABLE'`。export.rs:631）に載らず、`unclassified()`（export.rs:652）は分類が `Include` の表の列しか見ないので、ビューの列が分類を求めない（D11 が成り立つ）
- **Scenario の本数の勘定**: spec の `#### Scenario:` は 59 本（distribution 18・collection-coverage 15・device-collection 9・desktop-collection 8・data-durability 9）。tasks.md が名指す Scenario の名前は 42 本で、spec の名前と一字一句一致（`comm` で突き合わせ）。残りの 17 本は ST49 の 13 本と正典から写した data-durability の 4 本で、Global Constraints（tasks.md:15-19）と一致
- **検証のコマンドの名前**: `tools/check-openapi.sh`・`tools/check-immutable.sh`・`tools/check-shared-appends.sh`・`crates/server/tests/server_startup.rs`・`export_file_excludes_ledgers_and_derived_tables`（export_file_tests.rs:1197）・`heartbeat_raw_has_no_private_content`（collector-windows heartbeat.rs:457）・`story_deps.py included` は実在。
  絞り込みの既存の本数: `migration_ledger_`（migration_ledger_tests.rs 23 本）・`app_role_tests`（3 本）・`export_`（175 本前後）・`outage_`（78 本前後）・`*Heartbeat*`（Android 29 本）で、書いた下限を満たす
- **design の D1〜D5（仮）と provisional.jsonl**: 5 行が D1〜D5 に 1 対 1 で対応し、出所（review/deep.md R2・R5・R3・R11・R6）も一致。反転条件は D1・D3・D5 は意味を持つ（D2・D4 は R14）
- **`pg_input_is_valid`**: docker-compose.yml:3 は `postgres:17-alpine`（16 以降で使える）
- **ST49 の「版を原文に入れない」との関係**: ST49 specs/record-envelope:114 と design D11 は**記録**の原文の話で、ST51 が生存信号の原文に版を入れる（D6。冪等キーが原文から作られるため）のとは別のもの。食い違わない

## 観点 7: 走っている change との前提関係
- st49-record-component-version / PC の収集アプリは、記録を未送信に積む時点で自分の版を焼き込み、溜めた行の欄を落とさない: 前提（ST49 の ADDED の本文を MODIFIED で書き換える。相手の本文が変われば写し直し、相手が正典に入るまで archive できない —— R15）
- st49-record-component-version / 携帯の収集アプリは、記録を未送信に積む時点で自分の版を焼き込み、溜めた行の欄を落とさない: 前提（同上）
- st49-record-component-version / 組み立ての印は、版番号・commit・その部品のソースの未 commit の変更・組み立ての種類を持ち、組み立てのたびに作り直す: 前提（S-01・C-01・C-02 が焼き込む印をそのまま更新の履歴と生存信号に書く。印の形が変われば Scenario の値も変わる）

3 件とも ST49 は requires にある（docs/stories/ST51.md:5）ので、requires を足す指摘は立てない。

---

# 第 2 回の答え（Q6）の写しの確かめ（spec --answers Q6、2026-10-29）

対象: `git diff HEAD -- openspec/changes/st51-update-history docs/handoff/ST58.md`（推奨で書いた版 47fb7381 からの未 commit の差分）と `docs/briefs/ST51-pr.md`（git の外）。
照合に使ったコード: `crates/server/src/migration_ledger.rs`（`startup_refusal`・`apply_migrations_with`・`apply_locked`）・`migrate_backup.rs`・`crates/server/tests/server_startup.rs`・`crates/server/src/testdb.rs`・`tools/pg_dump-docker.sh`。
観点 6 の結果は `review/answers.md`（Q6: 写った。答え以外の変更は D16（仮））。

## 機械の検査（第 2 回）

- `openspec validate st51-update-history --strict` → valid。INFO 2 件（ST49 の MODIFIED の相手が正典に無い。R15 と同じ。今回の差分に依らない）
- `python3 scripts/check_chain.py .` → chain: OK（0 件 / 未回収 0 件 / warn 0 件）
- `python3 scripts/check_scenarios.py . st51-update-history` → FAIL（担保なし 64 件）。実装前なので想定どおり。distribution の Scenario は 24 本で tasks の「69 本・distribution 24」と一致
- `python3 scripts/review_triage.py . st51-update-history` → triage: OK（指摘 27 件 / 仮決め 6 件 / 要件へ戻すもの 0 件）
- `python3 scripts/spec_overlap.py candidates . st51-update-history` → 3 件（すべて `st49-record-component-version`。第 1 回と同じ）→ 観点 7 は上の節のまま
- `python3 scripts/provisional.py check openspec/changes/st51-update-history` → FAIL 1 件「D16: provisional-review.md に別の席の確かめが無い」（グラフの provisional_review が書く。想定どおり）/ `pending` → `["D16"]`。台帳の形（follows・rests_on・重複）の FAIL は無い

## R17. 試みの行を書いた回の結果の行が比べ直しで落ちうるので、通った回が「結果を書く前に終わった」と読まれる。D16 の「錠を持ったまま試みから結果まで書く」は今のコードの錠の範囲と合わない
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md、design.md（D16・D8・D7）、tasks.md（2.1）
- 根拠:
  - 結果の行は、試みとは別に、終わる時にもう一度「直前の通った行の版と違えば」で書く（specs/distribution/spec.md:6）。読み方は「試みの後に、次の試みより前に `via = migrate` の結果の行が無ければ結果を書く前に終わった」（spec.md:17）
  - 起動は migrate の錠を取らない（`migration_ledger.rs:277-305` の `startup_refusal` は錠なしで版の記録を読む）。更新の履歴の鍵は比べて書く取引の頭の `pg_advisory_xact_lock` だけ（design.md:71）
  - 当てる処理の錠は `apply_migrations_with` が `apply_locked` の後に外してから返す（`migration_ledger.rs:335-345`）。結果の行は `run_migrate` が終わりに書く（tasks.md:61、design.md:78「当てて付与した後・終了コード 0 の前」）ので、錠の外になる。design.md:97 の「migrate は錠（`pg_advisory_lock`）を持ったまま試みから結果まで書く」はこれと合わない
  - 手順 1（起動が間に入る）: 通った行 A の D-01 で、当てる版の無い印 B の migrate が試み B を書き、退避（`pg_dump`）の間に印 B の S-01 が起動する → 版の記録は揃っている（`Verdict::Even`）ので起動の行 B（`nothing_to_apply` = 通った行）を書く → migrate の終わりに比べ直すと直前の通った行が B なので結果の行を書かない → 試み B の後に結果の行が無く、spec.md:17 の読み方で「通らなかった回」と読まれる（実際は通った）
  - 手順 2（2 つの migrate）: 同じ印 B の migrate M1・M2 が続けて走る。M1 は錠を外してから `applied` を書く。その間に M2 が錠を取り、直前の通った行（まだ A）と比べて試み B を書く → 行は 試み(M1)・試み(M2)・`applied`(M1) の順で、M1 が「結果を書く前に終わった」と読まれる。spec.md:14 の WHILE は「migrate と起動」「2 つの起動」だけを名指し、2 つの migrate を名指さない
  - design.md:87 は試みの行を「取引の外（autocommit）か自分の取引で」書くとしていて、D7（比べる・書くを 1 つの取引にし、頭で `pg_advisory_xact_lock`）と揃っていない。autocommit なら比べてから書くまでの間に起動の行が入る
- kind: technical
- 提案: 「試みの行を書いた migrate は、結果の行を比べ直さずに必ず書く（書くかは試みの時に 1 回だけ決める）」を Requirement に足し、起動の行が間に入っても結果の行が書かれる Scenario を 1 本足す。
  結果の行を当てる処理の錠の中（錠を外す前）で書くか、D16 の錠の文を消して「2 つの migrate が交ざっても読み方が崩れない」形を別に決める。design.md:87 の「autocommit」は D7 の 1 つの取引にそろえる
- 処置: fixed D16 仮 —— 書くかは試みの時に 1 回だけ決め、試みを書いた回は結果の行を比べ直さずに必ず書き、前の版は試みの行の前の版にする・結果の行は当てる処理の錠を外す前に書く、を D16（仮）と Requirement に書いた。試みの行は D7 と同じ 1 つの取引で書く（design D8 の autocommit を消した）。Scenario「試みと結果の間に同じ組み立ての起動の行が入っても、結果の行は書かれる」と、2 つの migrate の試験（tasks 2.2）を足した

## R18. Requirement の題「1 行残る」と WHILE「同じ版の変化を 1 行だけ書く」が、migrate の 1 回の版の変化に試みと結果の 2 行を書く仕様と食い違う
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md
- 根拠: spec.md:3（題「S-01 の版が変わると、更新の履歴に 1 行残る」）/ spec.md:14（「同じ版の変化を 1 行だけ書く」）/ spec.md:5-6（migrate は試みと結果の 2 行を書く）/
  spec.md:112（Scenario「更新の履歴の初めの行は、前の版を空にする」の THEN が「2 行」）/ spec.md:33 の Scenario 題「当てる版が無い入れ替えの migrate も 1 行残る」は THEN が「最後の行」だけを見て、試みの行を見ない
  （「新しい組み立ての migrate が版を当てると…」と「退避に失敗した回は…」は 1 つ前の試みの行を AND で見るようにしたが、この 1 本だけ直っていない）。
  WHILE を字面どおりに実装すると、試みの行を書いた後の結果の行を「同じ版の変化の 2 行目」として書かない実装でも本文に合う（R17 と同じ穴）
- kind: technical
- 提案: WHILE を「同じ版の変化を、migrate なら試みと結果の 1 組、起動なら 1 行だけ書く」の形に直し、題の「1 行」を外すか言い直す。
  Scenario「当てる版が無い入れ替えの migrate も 1 行残る」は題を直すか、1 つ前の行が試みであることを別の Scenario で見る（1 Scenario 1 主張を崩さないなら別にする）
- 処置: fixed specs/distribution/spec.md —— WHILE を「migrate なら試みと結果の 1 組、起動なら 1 行」にし、2 つの migrate も名指した。題の「1 行」は結果の行のことと本文に書いた（題は go.json の 35 か所の参照を保つため変えない）。Scenario「当てる版が無い入れ替えの migrate も 1 行残る」に、1 つ前の行が試みであることを AND で足した（他の 2 本と同じ形）

## R19. D16（仮）の反転条件は、選んだ側ですでに試みの行を書く回を名指していて反転にならない。書かない回は別にある
- 成果物: openspec/changes/st51-update-history/design.md（D16）、provisional.jsonl（D16）
- 根拠: design.md:99 の反転条件「同じ組み立てで落ちた migrate を走らせ直した回（前の回の退避から戻した後の再試行）も、試みとして数えたいと分かったら」。
  戻した D-01 の行は 通った A・試み B なので、直前の通った行は A。印 B で走らせ直すと B ≠ A で、D16 の選んだ側でも試みの行を書く（provisional.jsonl の D16 の keeps も「同じ版の再試行も含め…2 行になり」と書いている）。
  D16 が書かないのは「直前の通った行と同じ組み立ての走らせ直し」（そのときは D2 の条件で結果の行も書かない）だけ。review/spec.md R14（D2 の反転条件が今の振る舞いと同じ）と同じ形
- kind: technical
- 提案: 反転条件を、書かない側に実際に入る回で書き直す（例:「最後に通った組み立てと同じ組み立てで migrate を走らせ直し、退避や付与で通らなかった回の跡が要ると分かったら」）。D16 の本文の「版の変わらない走らせ直しは、もともと版の変化ではない」とそろえる
- 処置: fixed D16 仮 —— 反転条件を、書かない側に入る回（最後に通った組み立てと同じ組み立ての走らせ直しで、通らなかった跡が要る）で書き直し、戻した後の再試行は選んだ側でも書くことを添えた

## R20. design D8 の「上の表の 2〜6 行目」が、起動の行（6 行目）を含み、付与・錠・版の記録の失敗（7 行目）を外している
- 成果物: openspec/changes/st51-update-history/design.md
- 根拠: design.md:75-84 の表は 1 試み・2 `applied`・3 `nothing_to_apply`・4 `failed`（版が落ちた）・5 `backup_failed`・6 起動・7 付与などの失敗の `failed`・8 書かない。design.md:86「試みの行の後に、その migrate の結果の行（上の表の 2〜6 行目）を書く」
- kind: technical
- 提案: 「2〜5 行目と 7 行目」に直す。7 行目のうち錠・版の記録の読みの失敗は版の検査より前なので試みの行を伴わないことも 1 行書き添える
- 処置: fixed D8 —— 「上の表の 2〜5 行目と 7 行目」に直し、錠・版の記録の読みの失敗は試みの行を伴わないことを書き添えた

## R21. 「退避から D-01 を戻しても試みの行が残る」の試験は本物の `tools/pg_dump-docker.sh` と db のコンテナの `pg_restore` を使うが、置き場の bin の試験に前例が無く、そのままでは繋ぎ先も PATH も届かない
- 成果物: openspec/changes/st51-update-history/tasks.md（2.3）
- 根拠: tasks.md:87-88（`st51_bin_migrate_attempt_survives_restore`。`PG_DUMP` は本物の `tools/pg_dump-docker.sh`、空の別の DB へ db のコンテナの中の `pg_restore`）/
  `crates/server/tests/server_startup.rs:577-580` の `migrate_with` は `env_clear()` で PATH を落とす（`pg_dump-docker.sh` は `docker compose exec` を呼ぶ）/ 既存の退避の bin の試験は偽の `pg_dump`（同ファイル:558-569）だけで、`crates/server/tests`・`src/*tests*.rs` に `docker`・`pg_restore` を呼ぶ試験は 0 件（grep）/
  `pg_dump-docker.sh:16` はその worktree の compose の `db` の中の `127.0.0.1:5432` へ繋ぐが、試験の DB は `testdb.rs:236` の `db_port()` で繋ぎ先を決めるので、両者が同じサーバである前提が書かれていない /
  空の別の DB は `testdb.rs:18`「`CREATE DATABASE` はこのファイルの中だけに書く」により testdb の口から取る必要がある
- kind: technical
- 提案: 2.3 に前提（`PATH` と docker の接続を渡す・試験の DB が compose の db と同じサーバであること・空の DB は testdb から取る）を書くか、同じプロセスの試験（2.2）で、試みの行を commit した後・当てる前の D-01 の写し（`before_apply` の中で testdb の口から複製する等）を取って戻す形に置き換える。どちらでも検証のコマンドの本数の条件（`-ge 5`）はそのまま使える
- 処置: fixed 2.3 —— 前提（`migrate_with` に PATH などを渡す口・戻し先は `testdb::scratch()`・試験の DB と compose の db が同じサーバであることを頭で確かめ、違えば失敗）を tasks 2.3 に書いた

## R22. 本文の「ST51 の移行を当てる前の試み（表がまだ無い）」に Scenario が無い。新しく入れた D-01 の最初の migrate は毎回これに当たる
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md
- 根拠: spec.md:18（IF 表がまだ無い … ST51 の移行を当てる前の試み THEN 書かずに `update_history_unwritten` の警告）。これを見る Scenario は無い ——
  「更新の履歴の表が無い D-01 で落ちても…」（spec.md:114-118）は落ちた回だけ、「更新の履歴の初めの行は…」（spec.md:108-112）は GIVEN が「表が空の D-01」（表はある）。
  空の DB に初めて migrate を当てる回（新しく入れた環境・ST51 を初めて当てる本人の D-01）は、試みが書けず警告が出て、結果の `applied` の 1 行だけが残る。どれも Requirement の本文にしか無い
- kind: technical
- 提案: 「更新の履歴の表が無い D-01 に初めて当てて通った migrate は、警告を出し、`applied` の 1 行だけを残す」の Scenario を足す（tasks 2.2 に 1 本足し、`-ge 15` を `-ge 16` にする）
- 処置: fixed specs/distribution/spec.md —— Scenario「更新の履歴の表が無い D-01 に初めて当てて通った migrate は、警告を出し、結果の 1 行だけを残す」を足し、tasks 2.2 に `st51_migrate_first_install_no_attempt` を足した（下限 -ge 18）

## 観点 1〜5・7（第 2 回の差分の範囲）

- 観点 1: Q6 の「仕様へ写すもの」の各文は specs に Scenario を持って写っている（`review/answers.md`）。数値の決定は無い。Q2 の答えは覆していない（覆した後の内容だけが残る、の点検は不要）。
  要件へ戻すもの: なし（deep.md:52-53。FR-121 の★ 2026-10-29 は「どこで・いつ書くかは ST51 の深掘りで決める」と委ねている —— docs/requirements.md:1030-1032 で確かめた）
- 観点 2: R17・R18・R22。新しい Scenario 2 本（退避の時点で読める・戻しても残る）は WHEN / THEN が観測できる言葉で書かれている。「戻しても残る」の THEN は最後の行と「版の記録に落ちた版が無い」を AND で束ねるが、後者は戻しの前提の確認で主張は 1 つと読んだ
- 観点 3: specs の差分に関数名・crate 名は入っていない（`before_apply`・`apply_locked` は design と tasks だけ）。`attempted` は Requirement の表が固定する値の一覧の値で、実装の名前ではない。proposal の Capabilities は変わっていない
- 観点 4: R21。依存順は 1.1（CHECK に `attempted`）→ 2.1（`record_attempt`）→ 2.2・2.3 で崩れていない。人間の確認待ちの要るタスクは足されていない
- 観点 5: check_chain OK。ST51 の完了の判定「移行が通らずに退避から戻したときも『通らなかった』として 1 行残り、戻しで消えない」（docs/stories/ST51.md）と Q6 の写しは合う。
  ただし R17 が直るまでは、通った回が同じ「試みの後に結果が無い」形で残りうる（完了の判定の「通らなかった」の読み違い）
- 観点 7: 候補は第 1 回と同じ 3 件（ST49）で、差分は ST49 と重なる Requirement に触れていない。上の「観点 7」の節のまま
- 指摘にしなかったもの: premises.md:44（C10）が「Q2 の推奨」「落ちた試みと戻したことの行を書き直す」の字面のまま。premises は深掘りの前の前提の表で、申し送りの正本は docs/handoff/ST58.md（今回直っている）なので挙げない

---

# R17〜R22 の直しの確かめ（2026-10-29、第 2 回の 2 巡目）

対象: R17〜R22 の処置の後の `git diff HEAD`（spec・design・tasks・provisional.jsonl・proposal・docs/handoff/ST58.md）と `docs/briefs/ST51-pr.md`。

- R17: 2 つの手順はどちらも解けている。2 つの migrate は M2 が錠を待ち、取った時には直前の通った行が B なので何も書かない。起動が間に入る手順は、試みを書いた回は結果の行を必ず書くので、試みだけが残る行は出ない。
  ただし前の版の決め方で新しい食い違いが出た → R23・R24
- R18: WHILE と Scenario「当てる版が無い入れ替えの migrate も 1 行残る」の AND で解けている。2 つの migrate の Scenario が無い → R25
- R19・R20・R22: 指摘どおりに直っている。Scenario「…初めて当てて通った migrate は、警告を出し、結果の 1 行だけを残す」の GIVEN と THEN は観測できる
- R21: tasks 2.3 に前提（PATH などを渡す・`testdb::scratch()`・同じサーバかを頭で確かめ、違えば失敗にして飛ばさない）が書かれ、解けている
- 本数: distribution 26・全体 71・印を置く 54・2.2 の `st51_migrate_` 18 本（`-ge 18`）・2.3 は `-ge 5` で、実数と合う
- 指摘にしなかったもの: `docs/briefs/ST51-pr.md:35` の D16 の説明は R17 の直し（書くかを試みの時に 1 回だけ決める・結果の行を錠の中で書く）を含まず古い。GO 資料と PR 本文は確かめの後に書き直されるので挙げない。
  D16 の見出し「結果の行と同じ条件でだけ書く」は、結果の行がもう比べ直さないので字面がずれる。見た目だけなので挙げない

## R23. 結果の行の前の版を「試みの行の前の版」にする文（spec.md:7）が、D7 の文（spec.md:11）と食い違う。Scenario は spec.md:7 の側に立つので、D7 どおりの実装は落ちる
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md、design.md（D16・D7）、provisional.jsonl（D16）
- 根拠:
  - spec.md:7「試みの行を書いた回の結果の行の前の版を、試みの行の前の版と同じにし」/ design.md:97（D16）
  - spec.md:11「『前の版』を、更新の履歴の中で書いた順に最後の、移行の結果が `applied` か `nothing_to_apply` の行の今の版にする」/ design.md:68-71（D7。`seq` の最大の通った行。D7 は仮でない）
  - Scenario「試みと結果の間に同じ組み立ての起動の行が入っても、結果の行は書かれる」（spec.md:91-95）の THEN は、試み（A→B）・起動の行（A→B。`nothing_to_apply` = 通った行）・結果の行（前の版 **A**）の順。
    結果の行を書く時、書いた順で最後の通った行は起動の行（今の版 B）なので、spec.md:11 は前の版 B を、spec.md:7 は A を求める。tasks 2.1 は前の版を読む口を「D7。`seq` の最大」と書いており（tasks.md:58）、それに従うと Scenario が落ちる
  - `from_version` は追記のみの凍結した行の値で、書いた後に直す口が無い。D16 の書く条件（書く・書かない）と違い、この値は「計算し直せば戻る」型でも、D2 の書く条件と同じ型でもない。provisional.jsonl の D16 は `rests_on` に仮でない D7 を持つ
- kind: technical
- 注: `from_version` は後から戻せない値なので、処置を反射的に `fixed D16 仮` にしないこと
- 提案: どちらを取るかは呼び出し元が決める。帰結は次のとおり。
  (1) 前の版 A を保つなら、spec.md:11 に「試みの行を書いた回の結果の行は除く（試みの時の前の版）」の例外を書き、D7 の節もそれに合わせる。これは仮でない D7 の述べた規則を変えるので、合意の引き継ぎで D7 を名指すことになる（聞き直し）
  (2) D7 どおりにするなら、結果の行の前の版は B になり（B→B の結果の行）、Scenario の THEN を直す。D7 は変わらないが、版の変化を読む側は前の版と今の版の同じ行を読み飛ばすことになり、それを Requirement に書く必要がある
- 処置: fixed D16 仮 —— (1)(2) のどちらも取らず、D7 を変えずに直した: 起動の行の取引が頭で当てる処理の錠を待つ（D16（仮））。試みと結果の間に通った行が入らないので、D7 のまま読んで結果の行の前の版は試みの行の前の版と同じになる（凍結する値は D7 の規則どおりで、例外を作らない）。spec の文は D7 の例外でなく錠の帰結として括弧に書き、Scenario を「当てる処理の途中で同じ組み立ての S-01 が起動しても、試みと結果の組は崩れない」（起動の行は無い）に書き直した

## R24. 起動が試みと結果の間に入る回は、同じ A→B の変化が通った行として 2 回残る（起動の行と migrate の結果の行）。WHILE はそれを防ぐ文だったが、新しい字面では防がず、そのことも書いていない
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md、docs/handoff/ST58.md
- 根拠: spec.md:14 の WHILE「同じ版の変化を、migrate なら試みと結果の 1 組、起動なら 1 行だけ書く（題の『1 行』は、版の変化 1 回につき結果の行が 1 つ残ること…）」。書き手ごとには満たせるが、
  spec.md:91-95 の Scenario は、起動の行（A→B・`nothing_to_apply`）と migrate の結果の行（A→B・`nothing_to_apply`）の両方を求める。「結果の行が 1 つ」が成り立つのは、spec.md:17 が結果の行を `via = migrate` に限っているからにすぎない。
  通った行を版の変化 1 回 = 1 行として読む側（書き出しを読む本人・`docs/handoff/ST58.md` の「前の版で起動し直しても行は増えない」の読み方）には、同じ変化が 2 回に見える。直し前の WHILE（「同じ版の変化を 1 行だけ書く」）はこの重なりを防ぐ文だった
- kind: technical
- 提案: Requirement に「この競り合いでは、同じ変化が起動の行と migrate の組の 2 つで残る。版の変化として数えるときは、同じ今の版を持つ起動の行の後に続く migrate の組を 1 回と読む」のように重なりと読み方を書く（R23 の (2) を取ると読み方が変わるので一緒に決める）。
  代わりに、起動の側が結果の行の無い同じ今の版の試みを見て書かない形もある。ST58 の申し送りが通った行を 1 変化 1 行と読んでいるなら、そこも合わせる
- 処置: fixed D16 仮 —— R23 と同じ直し（R24 の提案の「起動の側が書かない形」に近い）。起動は migrate の結果の後に比べるので、同じ版の変化が 2 回残らない。待ちの費用と反転条件を D16 に書いた

## R25. WHILE が新しく名指した「2 つの migrate」に Scenario が無い。spec.md:6 は「試みを書かなかった回（同じ版）」の結果の行を言っていない
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md、tasks.md（2.2）
- 根拠: spec.md:14 の WHILE は「または 2 つの migrate」を足したが、それを見る `#### Scenario:` は無い。tasks.md:83 の `st51_migrate_two_migrates_pairs_intact` は `// Scenario:` の印を持たず、`check_scenarios.py` は結び付けない（Requirement の本文だけが言っていることになる）。
  spec.md:6 は「試みの行を書いた回は必ず」と「書けなかった回（表がまだ無かった）は違えば」の 2 つだけを言う。試みを**書かなかった**回（直前の通った行と同じ組み立て）の結果の行は本文に無く、Scenario「同じ組み立てで migrate を走らせ直しても、行は増えない」（spec.md:41-45）だけが決めている
- kind: technical
- 提案: 「同じ印の 2 つの migrate を同時に走らせても、試みと結果の 1 組だけが残る」の Scenario を足し、tasks 2.2 の試験に印を付ける。spec.md:6 に「試みを書かなかった回（直前の通った行と同じ組み立て）は結果の行も書かない」を 1 句足す
- 処置: fixed specs/distribution/spec.md —— Scenario「同じ組み立ての当てる処理を 2 つ同時に走らせても、試みと結果は 1 組だけ」を足し、tasks 2.2 の `st51_migrate_two_migrates_pairs_intact` に印を付けた。「表があって試みの行を書かなかった回は、結果の行も書かない」を Requirement に足した


---

# R23〜R25 の直しの確かめ（2026-10-29、第 2 回の 3 巡目）

対象: R23〜R25 の処置の後の `git diff HEAD`。直しは、起動の行の取引が頭で当てる処理の錠（`migration_ledger.rs:13` の `LOCK_KEY` = 4_820_251）を `pg_advisory_xact_lock` で待つ形（D16（仮）、design.md:99-103、spec.md:7、tasks 2.1）。
照合に使ったコード: `crates/server/src/lib.rs:224-238`（`migrate` / `migrate_after`）・`:3862`（`run` の `startup_refusal`）・`migration_ledger.rs:13・:277-305・:335-345`。

- R23: 解けている。試みと結果の間に起動の行が入らないので、D7 の文（spec.md:12）のままで、結果の行の前の版は試みの行の前の版と同じになる。spec.md:7 は D7 の例外ではなく錠の帰結として書かれていて、D7 の節も変わっていない
- R24: 同じ組み立ての起動について解けている。migrate の後に起動の取引が比べると、直前の通った行はもう B なので、2 回目は残らない。ただし別の組み立ての起動では、錠を待つことで新しい食い違いが出る → R26
- R25: 解けている。Scenario「同じ組み立ての当てる処理を 2 つ同時に走らせても、試みと結果は 1 組だけ」（spec.md:97-101）があり、tasks 2.2 の試験に印がある。spec.md:6 に「試みを書かなかった回は結果の行も書かない」が入った
- 錠の順（呼び出し元が名指したデッドロックの点検）:
  - 起動は「`LOCK_KEY` → 更新の履歴の鍵」、migrate の試みの取引は「`LOCK_KEY`（セッション）→ 更新の履歴の鍵」で、順が同じなので両者の間に循環は無い
  - advisory lock は database ごとなので、試験の DB どうしでも取り合わない
  - `run` は migrate を呼ばない（lib.rs:221-222 の注）ので、同じプロセスの中で起動が自分の migrate を待つことも無い
  - 残るのは結果の行を書く接続が決まっていない点 → R28
- 本数: distribution 27・全体 72・印を置く 55・2.2 の `st51_migrate_` 18 本（`-ge 18`）で、実数と合う
- 既存の起動の Scenario（「migrate を走らせずに入れ替えた S-01 は…」「migrate の後に同じ組み立ての S-01 を起動しても…」「版の検査で断られた起動は…」「同じ版の変化を 2 つ同時に…」）は、migrate が走っていなければ待たないので変わらない

## R26. 起動は版の検査の後で錠を待つので、待っている間に migrate が新しい版を当てると、検査が古くなったまま古い S-01 が新しい D-01 に起動の行を書き、待ち受けまで開く
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md、design.md（D16）
- 根拠:
  - 起動の順は「版の検査（`startup_refusal`。錠を取らない、`migration_ledger.rs:277-305`・`lib.rs:3862`）→ 起動の行を書く取引の頭で `LOCK_KEY` を待つ（spec.md:7・design.md:99）→ 書く → 待ち受け（spec.md:8）」
  - 手順: D-01 は組み立て A の版の並びで揃っている。組み立て B（版を 1 つ足した）の migrate が試みを書き、退避（`pg_dump`）を取っている間に、古い組み立て A の S-01 が起動する（自動の再起動・止め忘れ）。
    このとき版の記録はまだ A の並びなので、A の検査は通る（`Verdict::Even`）。A は錠を待つ。その間に B の migrate が版を当て、結果の行（A→B）を書き、錠を外す。
    A は錠を取り、直前の通った行（B）と自分（A）を比べて、起動の行 B→A を書く。そのまま、自分の知らない版の当たった D-01 で待ち受けを開く
  - その結果、更新の履歴に偽の戻り（B→A）が凍結され、次の前の版（D7）は A になる。さらに spec.md:24「起動の行は版の検査の後に書く —— 前に書くと、古い S-01 が新しい D-01 に書くことになる（FR-117）」と、spec.md:17 の「起動が版の検査で断られる THEN 何も書かない」の両方が崩れる
  - design.md:101「起動は版の検査を通った後なので、待つのは版の揃った DB での入れ替えのときだけ」は、検査が待ちの前にあることを見落としている。検査と書き込みの間の隙は直す前からあったが、ミリ秒の幅だった。錠を待つ直しで、その幅が migrate の全体（退避を含む）に広がった
- kind: technical
- 提案: 起動の取引が `LOCK_KEY` を取った後に、版の検査をもう一度その中で行い、断られたら書かずに終了コード 3 で止まる、を Requirement と D16 に書く（ST48 の検査を錠の中に移すか、錠の中で 2 回目を引くか）。
  「当てる処理の途中で古い組み立ての S-01 が起動し、当てる処理が新しい版を当てると、起動は終了コード 3 で止まり、起動の行を書かない」の Scenario を足す
- 処置: fixed D16 仮 —— 起動の取引は錠を待たずに取ろうとし（`pg_try_advisory_xact_lock`）、取れたらその中で版の記録を読み直して、知らない版があれば書かない。待たないので検査が古くなる幅は錠の前の数ミリ秒に戻り、その幅も読み直しで塞ぐ。Scenario「版の検査の後に知らない版が当たった D-01 には、起動の行を書かない」を足した（古い S-01 を止めるかは ST48 の起動の検査の受け持ちで変えない）

## R27. 起動の錠の待ちに上限が無い。migrate が止まったままだと S-01 は待ち受けを開かず、D9 と spec.md:20「更新の履歴を書けなかったことで…起動の成否も変えない」に反する
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md、design.md（D16・D9）
- 根拠:
  - spec.md:7・design.md:99 の `pg_advisory_xact_lock(LOCK_KEY)` は、取れるまで待つ。サーバの接続に `lock_timeout`・`statement_timeout` は無い（`crates/server/src/lib.rs`・`grants.sql`・`migrations/*.sql` を grep して 0 件）
  - migrate がセッションの錠を持ったまま止まると（`pg_dump` が固まる・手で止めた端末の migrate・ネットワークの切れで接続が残る）、起動はその接続が切れるまで待ち受けを開かない
  - design.md:110（D9）の「書けないことで移行や起動を止めると、本人の記録を受け取れない時間が延びる」は、更新の履歴のために起動を止めない決定で、D9 は仮でない。D16 の費用（design.md:101）と反転条件（待ち）は「目に見えて遅らせると分かったら」としか書かず、上限を決めていない
- kind: technical
- 提案: 待ちに上限（`SET LOCAL lock_timeout`、または `pg_try_advisory_xact_lock` を短く何度か）を置き、超えたら書かずに `update_history_unwritten` の警告を出して起動を続ける、を D16 と Requirement に書く。
  超えた回に起動の行が残らないこと（その変化は走っている migrate の結果の行が持つかどうか）も書き添える。上限は Scenario で値を持たせる（無ければ、実装が値を変えても spec が落ちない）
- 処置: fixed D16 仮 —— 待たない形にしたので上限は要らない。取れなければ書かずに D9 の警告を出して起動を続ける（D9 の文は変えない）。反転条件を「上限つきで待つ側へ」にした

## R28. 結果の行をどの接続で書くかが決まっておらず、錠を持たない別の接続で起動と同じ口（`LOCK_KEY` を待つ）を通すと、migrate が自分の錠を待って止まる
- 成果物: openspec/changes/st51-update-history/specs/distribution/spec.md、design.md（D16・D8）、tasks.md（2.1）
- 根拠:
  - 試みの行は「錠を持った同じ接続で」と書かれている（design.md:87）。一方、結果の行は「`apply_migrations_with` が錠を外す前に、結果の行を書く口（試みの口と同じ引数の形）を呼ぶ」（design.md:98）、「錠を外す前に書く口から書く」（tasks.md:61）だけで、接続を名指さない
  - `run_migrate` / `migrate_after` は pool を持っている（lib.rs:229-237）。結果の行を pool の別の接続で、`record_startup` と同じく頭で `LOCK_KEY` を取る取引にすると、`LOCK_KEY` はセッションの錠として当てる処理の接続が持っているので、その取引は待ち続ける。当てる処理は結果の行を書き終えるまで錠を外さないので、互いを待つ
  - advisory lock の待ちは PostgreSQL のデッドロック検出にかかるが、同じプロセスの中の非同期の待ちと DB の錠の待ちの組は検出されない（DB から見ると片方が待っているだけ）
- kind: technical
- 提案: 「試みの行と結果の行は、当てる処理の錠を持つ接続で書き、`LOCK_KEY` を待つのは起動の行の取引だけ」を D16 と tasks 2.1 に書く。2.2 の試験（`st51_migrate_two_migrates_pairs_intact` など）に時間の上限（`tokio::time::timeout`）を付け、止まったら失敗にする（cargo test は止まっても落ちない）
- 処置: fixed D16 仮 —— 「試みと結果の行は錠を持つ接続で書き、`LOCK_KEY` を取ろうとするのは起動の取引だけ」を Requirement・D16・tasks 2.1 に書き、2.2 の同時の試験を `tokio::time::timeout` で包む


---

# R26〜R28 の直しの確かめ（2026-10-29、第 2 回の 4 巡目）

対象: R26〜R28 の処置の後の `git diff HEAD`。今回の直しは次の 3 つ（D16（仮）、spec.md:7-8、tasks 2.1・2.2）。
- 起動の取引は `pg_try_advisory_xact_lock(LOCK_KEY)` で錠を取ろうとし、待たない
- 取れなければ起動の行を書かずに `update_history_unwritten` の警告を出して起動を続ける。取れたらその中で版の記録を読み直し、知らない版があれば書かない
- 試みと結果の行は、錠を持つ接続で書く

呼び出し元の指示により、今回は実装の前に直す必要のある指摘だけを R にした。**新しい R は無い。** triage: OK。

- R26: 解けている。古い組み立ての起動が migrate の途中に来た場合は、錠が取れないので書かない。migrate の後に来た場合は、錠の中の読み直しで知らない版を見て書かない。
  どちらも偽の戻りの行は凍結されない。その S-01 を止めるかは ST48 の受け持ちとして、範囲の外に置くと明記された（design D16）。
  Scenario「版の検査の後に知らない版が当たった D-01 には、起動の行を書かない」（spec.md:98-102）が、錠の中の読み直しを観測できる形で見る
- R27: 解けている。待たないので、止まった migrate が起動を止めない。D9（仮でない）は今の文のまま満たされる。Scenario「当てる処理の途中で同じ組み立ての S-01 が起動しても…」（spec.md:92-96）の THEN が「待たずに終わって警告」を見て、試験は `tokio::time::timeout` で包む（tasks 2.2）
- R28: 解けている。「試みと結果の行を、当てる処理の錠を持つ接続で書く」が spec.md:7 と D16 と tasks 2.1 にある。2 つの migrate の試験も `timeout` で包む
- デッドロック: 起動は錠を待たないので、待ちの循環はもう作れない。錠を取れた起動が更新の履歴の鍵を待つ相手は、`LOCK_KEY` を持つ書き手だけで、その書き手は起動が `LOCK_KEY` を持つ間は動けないので、取り合わない
- 本数: distribution 28・全体 73・印を置く 56・2.2 の `st51_migrate_` 19 本（`-ge 19`）で、実数と合う
- D7・D9・D8 の節と ST48 の起動の検査（`startup_refusal` の位置）は変わっていない

## 指摘にしなかったもの（4 巡目。軽いもの）

- **D16 の費用の文の「同じ組み立てなら版の変化は migrate の組が持つ」は、その migrate が通らなかった回には成り立たない。**
  当てる版の無い組み立て B の migrate の退避の間に、S-01 B が起動したとする。錠は取れないので起動の行は書かれない。その後に migrate が `backup_failed`（または付与の `failed`）で終わると、S-01 B は動いているのに、通った行の今の版は A のままになる。
  その変化は、次に B を起動したとき（または migrate を通し直したとき）に、その時刻で 1 行残る。起きるのは同じ組み立ての起動と通らない migrate が重なった回だけなので、R にしなかった。確認バッチで D16 の費用を見るときに添える値打ちはある
- spec.md:8 の「起動し、版の検査を通った THE SYSTEM SHALL … 違えば 1 行を書く」は、spec.md:7 の 2 つの例外（錠が取れない・読み直しで知らない版）を書いていない。2 文を並べて読めば決まるので、字面だけの問題として挙げなかった
- 錠が取れずに書かなかった起動と、表が無くて書けなかった起動が、同じ警告の種別 `update_history_unwritten` になる。区別は警告の本文に任せられるので挙げなかった
- design D2（仮）の費用（design.md:35 の節「起動のときの 1 行を読んで比べる問い合わせ 1 本」）に、D16 が足した錠の試しと版の記録の読み直しが入っていない。D16 の費用の文が持っているので挙げなかった
