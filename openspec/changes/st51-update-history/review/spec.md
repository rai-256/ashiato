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
