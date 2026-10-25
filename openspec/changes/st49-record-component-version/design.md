# ST49 design — 記録ごとに、作った部品の版が残る

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q1〜Q4（どれも A）の推奨は仮でない D として書き（Q1 → D5・D6、Q2 → D10、Q3 → D14・D15、Q4 → D20）、
> 本人の答えが推奨と違えば `spec --answers` がその D と Scenario だけを書き直す。
> 深掘りの「聞かずに決めるもの」C1〜C14（deep.md の後ろの premises.md）は D に写した: C1 → D13、C2 → D8、C3 → D11、C4 → D12、C5 → D1（仮）・D2（仮）、C6 → D17・D4（仮）、
> C7 → D9、C8 → D16・D21（仮）、C9 → D3（仮）・D18・Migration Plan、C10 → D6、C11 → D5、C12・C13 → D18、C14 → D19。
> 仮の決定 D1〜D4 は深掘りの独立検査（review/deep.md R1・R9・R6・R11）で、D21 は spec の独立検査（review/spec.md R7。C8 を改めた）で載せたもの（`provisional.jsonl`）。

## Context

動機は proposal.md の Why。いまの形（前提の出所は `premises.md`）:

- 記録（`core.event`）と前の版（`core.event_version`）はスキーマ版・端末・TZ を持つが、部品の版を持たない。どの部品も自分の版を実行時に読んでいない（review/deep.md の手順 5）
- 取り込み口 `POST /ingest` は C-01・C-02・画面が同じ形（`IngestRequest`）で送る。資格情報は `authorize_quiet` が `Caller::ApiToken` / `Caller::WebSession` に分ける（前提 6）。
  書庫の取り込み（C-03）は S-01 の中で `store_one` → `ingest_request` を呼び、HTTP の判定を通らない（前提 8）
- `ingest_one` は 2 つの INSERT（外部の識別子の有無）で書き、外部サービス由来の更新は `apply_external_update` が前の版を列を名指しで写してから本表を書き換える。
  滞在は `stay_store.rs` が `insert_stay` / `update_stay` / `push_version` で書き、`schema_version` を `1` に固定している
- 収集した記録の門は列を名指しで見る。端末・TZ は「設定ミスを直せる余地」で門の外、前の版の錠 `core.reject_version_rewrite` は全列を名指しで凍結（前提 9）
- 収集アプリ 2 本は記録を作った時点で要求の形にして溜め、送るときはその形を送る（前提 1）。読み戻しは型に通すので、今の型が知らない欄は落ちる（C-01 `ignoreUnknownKeys = true`、C-02 は `serde` の既定）
- 書き出し（ST33）は表ごとに列を名指しした一覧（`EVENT_COLUMNS` など）を持ち、記録・前の版・破棄の報告・気分の通知の台帳を出す。原本の欠けと削除の台帳は出さない（`Class::Exclude`）
- ST48（下流が走っている）は `crates/server/build.rs` に `build_commit()`（`ASHIATO_BUILD_COMMIT` → `git rev-parse HEAD` → `unknown`）を足し、DB の台帳に `build_version` と `build_commit` を残す（ST48 design D8（仮））

## Goals / Non-Goals

**Goals**: 記録と部品が作る追記のみの行が、作った部品の版と D-01 に書いた S-01 の版を持つ。版は組み立てのたびに作り直した印で、未送信を更新の後に送っても作った時の版のまま。
版を足しても、重複の判定・収集した記録の門・書き出しの今の列・今の収集アプリとの組み合わせ（記録が入ること）が変わらない。

**Non-Goals**:
- 版で選び出す口（画面・API の絞り込み。D18）、版の変化の検知（ST51 / FR-121）、送信契約の版と古い S-01 の拒否（ST50 / FR-124 / FR-125 / FR-130）
- 生存信号の版（ST51 が動いている版を申告する。D18）
- 衛星の資格情報と、衛星の版（ST29 / FR-123 ★ Q13。D9）
- 版番号の出所を 1 つにすること・タグで配ること（ST52）。ST49 の版番号は各部品の今の手書き（`Cargo.toml`・`build.gradle.kts`・`web/package.json`）を読む
- 中身を持たない器（`core.place`・`core.attribute_kind`。中身は記録の表と種類の名前の台帳が持つ）と、S-01 が自分の判断で書く内部の台帳（`core.stay_absorbed`・`core.outage_ledger`・`core.export_ledger`・`core.media_original`・`core.web_session`・`core.access_log`）。
  部品（収集アプリ・画面の操作）が作る行ではない（Q3 の範囲は「収集アプリと画面が作る追記のみの行」。滞在の基準と個人属性の種類の名前は画面で本人が選んだ値なので含める —— review/spec.md R4）

## Decisions

### D1（仮）. 外部サービス由来の更新では、版は今の中身を作った版に動き、前の値は前の版に残る

- `apply_external_update` は本表の 3 つの版を、届いた要求の部品の版（D9 で振り分けたもの）と書いた S-01 の版（D10）に替える。前の版の INSERT は 3 つの版を名指しで写す
- 中身が変わらない到着（`advance_watermark`・重複）と、古い到着で退けた更新（`SkippedStale`）・削除済み（`SkippedDeleted`）では版を変えない
- 前の版の表・前の版の読み口 `core.event_version_live`・書き出しの `EVENT_VERSION_COLUMNS` に 3 列を足す。列が本表と揃うことを試験で固定する（足し忘れると更新のたびに前の値が消える）
- 反転条件: 「作った時の版」を「最初に作った版」と読むべき事例（最初の版で選び出したいのに、更新で版が替わって選べなかった）が出たら、本表の版を据え置き、前の版の表から最初の版を読む読み口を足す（前の値は前の版に残っているので、どちらにも戻せる）

### D2（仮）. 滞在の作り直しで原文が変わったら、S-01 の版を作り直した版に替える

- `insert_stay` は `server_version` に D10 の印を入れる。`update_stay` は原文が変わったとき（`e.raw != raw`。`push_version` を呼ぶのと同じ条件）だけ `server_version` を今の印に替える
- 印だけの書き換え（`e.raw == raw` で削除の印だけが違う）は版を変えない。`push_version` は 3 つの版を名指しで写す
- 反転条件: 作り直しの版を「最初に滞在を書いた版」で選び出したい事例が出たら、本表の版を据え置き、前の版の表から読む

### D3（仮）. 収集アプリの溜めは、読み戻した行の知らない欄を捨てずに送る。入れ替えは S-01 を先にする

- C-01: `SegmentStore` の読み戻しは行の JSON を保ち、送り出し（`Sender`）は保った JSON をそのまま送る（型に通して作り直さない）。C-02: `IngestRequest` に `#[serde(flatten)]` の知らない欄の置き場を持たせ、書き出すときに戻す
- 入れ替えの順: 本人の環境では **S-01 を先に、収集アプリを後に** 入れ替える（Migration Plan）。ST49 より前の S-01 は `client_version` を読み飛ばす（前提 3。記録は 200 で入り、版だけが落ちる）
- 代償: ST49 より前の版の収集アプリへ戻した間は、その版が欄を知らないので版が落ちる（直せない）。期限（扉 32）の後は ST50（FR-125 / FR-130）が古い組み合わせを止める（`docs/handoff/ST50.md`）
- 反転条件: 知らない欄を送り返したことで S-01 が要求を断る事例（`deny_unknown_fields` を足した型など）が出たら、送る前に S-01 の契約の版で欄を落とす（ST50 の契約の版に任せる）

### D4（仮）. 版の説明を「部品の申告」と「S-01 が自分で入れた」に書き分ける

- 書き出しの説明書（`EVENT_COLUMNS` などの説明）: `collector_version` は「記録を作った収集アプリの組み立ての印（収集アプリの申告。S-01 は確かめていない）」、`web_version` は「記録を組んだ画面の組み立ての印（画面の申告。S-01 は確かめていない）」、
  `server_version` は「記録を D-01 に書いた S-01 の組み立ての印（S-01 が自分で入れた）」。前の版・破棄の報告・気分の通知の台帳も同じ文で書く
- 反転条件: 収集アプリ・画面の版を S-01 が確かめる手段（署名など）ができたら、説明を「確かめた」に変える

### D5. 組み立ての印の形と、3 つの Rust / Kotlin / TypeScript の組み立てが共有する作り方

- 版番号の出所: S-01・C-02 は `Cargo.toml` の workspace の版（`CARGO_PKG_VERSION`）、C-01 は `build.gradle.kts` の `versionName`、V-01 は `web/package.json` の `version`（ST52 が 1 つにするまで。spec は「その部品の宣言した版番号」と書く）
- 形: `<版番号>+<commit>[.dirty][.debug]`。`<commit>` は 40 桁の 16 進（短縮しない）か `unknown`。`.dirty` は**その部品のソース**に未 commit の変更（`git status --porcelain -- <部品のソースの道>` が 1 行以上）があるとき（S-01・C-02 は `crates/`・`migrations/`・`Cargo.toml`・`Cargo.lock`、C-01 は `collector-android/`、V-01 は `web/`。ほかの部品の木の変更はその組み立てに入らない —— review/spec.md R2）、
  `.debug` は release でない組み立てのとき。順は固定（`.dirty` が先）
- commit の出所は ST48 の D8（仮）と同じ順: 環境変数 `ASHIATO_BUILD_COMMIT`（空でなければ）→ `git rev-parse HEAD` → `unknown`。値は ST48 の台帳の `build_commit` と同じ 40 桁になる（C11）
- Rust の 2 つ（S-01・C-02）は新しい workspace の crate `crates/build-stamp`（`ashiato-build-stamp`。`build-dependencies` だけで使う。依存の追加なし・`std::process::Command` で git を呼ぶ）を共有する。
  関数は「印を組む（版番号・profile・git の木・部品のソースの道）」「作り直しの印を出す（`cargo:rerun-if-changed` / `rerun-if-env-changed`）」の 2 つで、単体試験を crate に置く。git は `GIT_OPTIONAL_LOCKS=0` で呼ぶ（`git status` が `index` を書き換え、`index` を見張る次の組み立てがもう 1 回走るのを止める。review/spec.md R2 の実測）
- C-01 と V-01 は同じ規則を自分の組み立ての道具の中で書く（gradle と vite は Rust の crate を呼べない）。形が揃うことは 3 つの試験が同じ正規表現 `^[0-9][^+]*\+([0-9a-f]{40}|unknown)(\.dirty)?(\.debug)?$` で見る
- ST48 の D8（仮）が否になっても、印の作り方は ST49 の `crates/build-stamp` が持ち、形は変えない

### D6. 印は組み立てのたびに作り直す（差分の組み立てで古い commit を残さない）

- S-01・C-02: `build.rs` が `crates/build-stamp` で印を組み、`cargo:rustc-env=ASHIATO_BUILD_STAMP=<印>` で渡す。作り直しの印として `rerun-if-env-changed=ASHIATO_BUILD_COMMIT`・
  `rerun-if-changed=<git-dir>/HEAD`・HEAD が指す参照のファイル（`<git-dir>/refs/heads/<枝>`。worktree では `git rev-parse --git-dir` と `--git-common-dir` の先）・`<git-common-dir>/packed-refs`・`<git-dir>/index`、
  と**部品のソースの道そのもの**（`crates/`・`migrations/`・`Cargo.toml`・`Cargo.lock`。cargo は道が dir なら中を全部見る）を出す。cargo は build script が `rerun-if-changed` を 1 つでも出すとパッケージのファイルを見なくなる（`crates/server/build.rs` は今 `migrations/` だけを出している）ので、
  ソースの道を出さないと add していない変更の後に `.dirty` が付かない（review/spec.md R2）。`target/` は道に入れない（組み立てのたびに変わる）。
  費用: Rust のソースが変わった組み立てで build script が 1 回走る（git を 3 回。数十〜数百 ms）。印が前と同じなら crate は組み直されない
- S-01 は ST48 の `build_commit()` を `crates/build-stamp` の関数に置き換え、`ASHIATO_BUILD_COMMIT`（ST48 の台帳が読む env）も同じ値で出し続ける（ST48 の振る舞いを変えない）
- C-01: `app/build.gradle.kts` が `providers.exec`（`git rev-parse HEAD`・`git status --porcelain -- collector-android`）で印を組み、variant ごとに `BuildConfig.COMPONENT_VERSION` に入れる（`androidComponents.onVariants` の `buildConfigFields.put` に provider で渡す。
  release の variant だけ `.debug` を付けない）。`providers.exec` の値は構成キャッシュの入力として毎回確かめ直されるので、キャッシュに古い印が残らない。ビルドキャッシュも BuildConfig の入力が変われば作り直す
- V-01: `web/build-stamp.ts`（設定から読む関数。`child_process.execFileSync` で git を呼ぶ）を `vite.config.ts` と `vitest.config.ts` の両方が読み、`define: { __ASHIATO_WEB_VERSION__: JSON.stringify(<印>) }` で埋め込む。
  `vite build`（`mode === "production"`）だけ `.debug` を付けない
- 退けた案: ST48 の D8 のまま HEAD の変化で作り直さない —— 記録の版は後から直せない記録に焼き込むので、古い印を許さない（C10）

### D7. 列と移行

- 列（どれも `text NULL`。既定値なし）:

  | 表 | `collector_version` | `web_version` | `server_version` |
  |---|---|---|---|
  | `core.event`・`core.event_version` | ○ | ○ | ○ |
  | `core.drop_report`・`core.media_missing` | ○ | — | ○ |
  | `core.deletion_ledger`・`core.feeling_reminder`・`core.stay_criteria`・`core.attribute_kind_name` | — | — | ○ |

- 移行 `migrations/<作成時刻>_record_component_version.sql`（名前の時刻は `202610250000` 以後）と `.down.sql`。`ADD COLUMN IF NOT EXISTS`・`CREATE OR REPLACE` で 2 回当てても落ちない。
  ST48 の D10（仮）の規則に従う: 前進側に `BEGIN` / `COMMIT` を文として持たず、`.down.sql` は `DELETE FROM migration_ledger.applied WHERE name = '<版の名前>'` を持つ（ST48 は requires にあり、下流の入口が ST48 の実装を取り込むので、台帳と `tools/check-migrations.sh` の規則は下流の木にある）
- 移行がすること: 上の列を足す / `core.event_live` を作り直す（`SELECT *` の展開は作った時点で固まる）/ `core.event_version_live` を作り直して 3 列を名指しで足す /
  `core.reject_version_rewrite` と `core.require_version_or_ledger` に 3 列を足す（D16・D21（仮））。追記のみの 6 表のトリガは `UPDATE` を全部拒むので、新しい列は足すだけで凍結される
- CHECK は置かない（値の形を検査しない。D8）。空の文字列は取り込み口で NULL にする（D8）ので、DB に `''` は入らない（CHECK `<> ''` も置かない —— 取り込み口の外の書き込みを断らないため）
- 移行は既存の行を埋めない（D12）

### D8. 取り込み口の欄は `client_version` の 1 つ。形を検査せず、空は NULL

- `IngestRequest`・`DropReportRequest`・`MediaMissingRequest` に `#[serde(default)] client_version: Option<String>` を足す。検査は足さない（断る理由を増やすと、収集アプリは FR-10 で記録を捨てる）
- 空の文字列（`""`）だけを NULL にし、ほかは前後の空白も含めて文字列のまま持つ（「文字列のまま」。review/spec.md R8）。U+0000 を含む値は格納できない（SQLSTATE 22021 でまとめ送り全体が 500 になる）ので、U+0000 を取り除いてから持つ（断らない。取り除いて空になれば NULL）。長さは切り詰めない
- `docs/collector-contract.md` に任意の欄として足し、`docs/openapi.json` を再生成する
- 退けた案: 収集アプリと画面に別の欄（`collector_version` / `web_version`）を送らせる —— 送り手が欄を選べると、画面のログインで収集アプリの版を名乗れる。欄は 1 つにし、どの列に入れるかは S-01 が資格情報で決める（D9）

### D9. 呼び出し元の区別で部品の版を振り分ける

- `ingest_request` に呼び出し元の種類（`Writer::Collector`（API の合言葉）/ `Writer::Web`（画面のログイン）/ `Writer::Internal`（S-01 の中の格納））を渡す。HTTP の `ingest` は `decided()` の `Caller` から決め、`store_one` は `Internal`
- `Collector` → `collector_version`、`Web` → `web_version`、`Internal` → どちらにも入れない。破棄の報告・原本の欠けは `Collector` だけ `collector_version` に入れる
- 衛星の資格情報（ST29 が `Caller` に足す）は `Collector` にも `Web` にもしない（`docs/handoff/ST29.md`）。`Caller` の種類を足したときに振り分けを決めないとコンパイルが落ちるよう、`match` に `_` を置かない
- ST23 も `ingest_request` に「HTTP か内部か」の引数を足す（ST23 design D7 / D8）。後から main に入る側が 1 つの引数に畳む（同じ区別の細かい側に寄せる）

### D10. S-01 の版はすべての記録に S-01 が自分で入れる

- `crates/server` の定数 `BUILD_STAMP: &str = env!("ASHIATO_BUILD_STAMP")` を、`ingest_one` の 2 つの INSERT・`apply_external_update`・`insert_stay` / `update_stay`・破棄の報告・原本の欠け・削除の台帳（`deletion.rs`・`stay_store.rs` の 2 か所）・気分の通知の台帳（`feelings.rs`）・滞在の基準（`stay_store.rs` の `set_criteria`）・個人属性の種類の名前（`attributes_store.rs` の 3 か所）の INSERT が `server_version` に入れる
- 機械の止め: `server_version` を持つ表への INSERT の文（`crates/server/src` の試験でないファイル）が列の並びに `server_version` を名指すことを、文字列で見る試験 `st49_ledger_inserts_name_server_version` を置く（列は NULL 可なので、書き忘れた INSERT は何も落ちない。走っている ST23 が足す削除の台帳の INSERT も、rebase の後にこれが落として知らせる —— review/spec.md R5）
- 要求の中身から決めない（送り手が S-01 の版を名乗る欄は無い）
- 衛星の記録（ST29 / ST39 の後）も同じ INSERT を通るので S-01 の版を持つ。衛星自身の版は持たない（Q2 の推奨）。**record の段への申し送り**: Q2 の推奨が採られたら、FR-123 の本文（S-01 の版を持つ記録の範囲）と
  Story の完了の判定「衛星が書いた記録には版を付けない」（→「衛星の版を付けない」）を ★ で改訂する

### D11. 版は内容の鍵・破棄の報告の鍵・原文・解析済みの内容に入れない

- `content_hash_of`（論理ソース・出来事の時刻・原文）と `hash_is_pinned` の固定値を変えない。破棄の報告の鍵（`logical_source + raw`）と原本の欠けの鍵も変えない
- 収集アプリは `client_version` を原文（`raw`）に入れない（要求の別の欄）。C-01 の破棄の報告の `raw` も今の形のまま
- 同じ内容の到着は `ON CONFLICT … DO NOTHING` で畳まれるので、先に入った行の版が残る

### D12. 版の無い記録は「不明」のまま。推定で埋めない

- 移行は既存の行を NULL のまま残す。取り込みの時刻と ST51 の更新の履歴から推定する列・読み口は作らない（推定と本物の版を区別できなくなる）
- 収集アプリは読み戻した行に欄が無ければ、今の版を入れない（D13）

### D13. 収集アプリは、未送信に積む 1 か所で版を焼き込む

- C-01: `Outbox.add` が、積む行に `client_version` が無ければ `BuildConfig.COMPONENT_VERSION` を書き込む。印を書くのは記録・破棄の報告（D15 で下書きが先に持つ）・原本の欠けの溜めだけで、生存信号の溜めには書かない
  （`Outbox` の組み立てに「印を書くか」を渡す）。`migrateLegacyOutbox`（ST01 / ST02 の JSONL の取り込み）と読み戻しは印を書かない
- C-02: 記録の溜め（`events`）に積む 1 か所（`Outbox::add_many` の呼び出し元の組み立て。生存信号の `beats` には書かない）で `client_version` に `env!("ASHIATO_BUILD_STAMP")` を入れる。読み戻し（`Outbox::open`）は入れない
- ブラウザ履歴の読み直しは新しい要求を組んで積むので、読み直した時点の版になる（D1（仮）の「今の中身を作った版」）
- 退けた案: まとめ送りの見出しや HTTP のヘッダで送る —— 送った時の版になり、FR-122 ★ 2026-10-25 の取り違えが戻る（C1）

### D14. 破棄の報告と原本の欠けは、部品の版と S-01 の版を持つ

- `drops.rs`・`media_missing.rs` の INSERT に `collector_version`（`Collector` のときだけ。D9）と `server_version`（D10）を足す。重複の判定の問い合わせは変えない（D11）
- 書き出しの破棄の報告の列に 2 列を足す（D17）。原本の欠けは書き出しに入らない（今と同じ）

### D15. 破棄の報告の下書きは、作り始めた版を持ち、版が変わったら閉じる

- C-01 の `DropDraft` に `clientVersion` を足し、下書きを始めるときに今の印を入れる。足そうとした下書きの `clientVersion` が今の印と違えば、その下書きを閉じて（`closed = true`）次の下書きを始める
- 保存済みの下書きに `clientVersion` が無ければ（ST49 より前の下書き）、版の違う下書きとして閉じ、閉じた報告は版の欄の無いまま送る（今の版を付けない。D12）

### D16. 前の版の版は凍結する。本文の消去は版に触らない

- `core.reject_version_rewrite`（前の版の錠）に 3 列を足す（TZ も前の版では凍結している先例と同じ）
- 本表の版の列の扱いは D21（仮）
- 本文の消去（ST23）は版の列に触らない（版は私的な中身ではない）

### D21（仮）. 収集した記録の版の書き換えは、本表の門の対象にする（前の版を積むときだけ直せる）

- premises の C8 は「門の外（端末・TZ の先例。誤った印を直す余地を残す）」で、同時に「列を書き換えるのは前の版を残す経路だけ」と書いていたが、門の外では後者を強制する仕組みが無く、直に書き換えると前の値がどこにも残らない（review/spec.md R7）。D1（仮）の「どちらの読み方にも戻せる」もそれに依る
- `core.require_version_or_ledger` の変化を見る列に 3 つの版を足し、更新前の版の照合にも 3 列を足す。版を書き換えるには、同じまとまりに前の版の行（D1（仮）・D2（仮）の経路、または直すための `push_version` と同じ形の INSERT）が要る。誤った印は直せる（前の版を積めば通る）
- 反転条件: 誤った印を前の版を残さずに直したい事例（前の版の表の行数が問題になる量の直し）が出たら、門の外へ戻す（前の値は前の版に残っているので、戻しても失うものが無い）

### D17. 書き出しの列

- `EVENT_COLUMNS` / `EVENT_VERSION_COLUMNS` に 3 列を `schema_version` の次に足す（ST41 も同じ一覧に足すので、末尾を取り合わない）。`DROP_REPORT_COLUMNS` に 2 列、`FEELING_REMINDER_COLUMNS`・`STAY_CRITERIA_COLUMNS`・`ATTRIBUTE_KIND_NAME_COLUMNS` に 1 列。説明は D4（仮）
- 今の列は消さず・名前を変えない。書き出しの列の一覧の試験（表の列と一覧が一致する）が新しい列を一覧に足し忘れたら落ちる（`CATALOG` の今の検査）

### D18. この Story で作らないもの

- 選び出す口（画面・API の絞り込み）: 版は書き出しと D-01 の問い合わせで選べる。後から作っても失うものは無い（C12）
- 生存信号の版（`core.heartbeat`）: 動いている版の申告は ST51（FR-121）が生存信号に足す（C13）
- 版の変化の検知（ST51）・送信契約の版（ST50）（C9）

### D19. 走っている change と同じ箇所を触る

- ST41: 移行は**当てる時点の** `core.event_live`・`core.event_version_live`・`core.reject_version_rewrite` の本体を写して版の列を足す。ST41 が先に main に入っていれば終わりの列を含む本体を写す（rebase のときに写し直す）。
  `IngestRequest`・2 つの INSERT・`apply_external_update`・前の版の INSERT・`push_version`・書き出しの一覧は、列を足す箇所が隣り合うだけで、意味は重ならない
- ST23: D9 の引数。ST23 が足す削除の台帳の INSERT（本文ごと消す `user:purged` の行）と新しい操作の台帳は `server_version` を書く必要がある —— D10 の機械の止めが rebase の後に落とす。`docs/handoff/ST23.md` に申し送る（review/spec.md R5）
- ST48: D5・D6（`build.rs`）と D7（移行の規則）。requires に入れてある

### D20. 画面は、送る 1 件に読み込まれた画面の印を載せる

- `web/src/buildStamp.ts` が `__ASHIATO_WEB_VERSION__` を 1 か所で読み、`ingestFeelingItem`（`feelings.ts`）・場所の 1 件（`places.ts`）・個人属性の主張の 1 件（`attributes.ts`）・`MasterView.tsx` の取り込みの 1 件が `client_version` に入れる
- S-01 は画面のログインの要求のときだけ `web_version` に入れる（D9）。画面の表示と操作は変えない
- 退けた案: S-01 の版だけで済ませる —— 更新をまたいで開いたままのタブと ST52 より前の別のプロセスの画面が組んだ記録を分けられない（Q4）

## Risks / Trade-offs

- [ST49 より前の S-01 に、新しい収集アプリが送ると版だけが落ちる] → 入れ替えは S-01 を先にする（Migration Plan）。期限の後は ST50 が止める（D3（仮）・`docs/handoff/ST50.md`）
- [ST49 より前の版の収集アプリへ戻した間の版は落ちる（直せない）] → 代償として書く。以後の版どうしの行き来では、知らない欄を送り返すので落ちない（D3（仮））
- [`.dirty` はその部品のソースだけを見る] → ほかの部品の木の変更は印に乗らない（その組み立てに入らないので、同じ中身の組み立てを別のものに見せない。D5）。部品のソースの道の外にある組み立ての入力（`~/.cargo` の依存・gradle の設定など）の変更も乗らない —— 依存は `Cargo.lock` / gradle の宣言で commit に入る
- [申告の版は偽れる] → 書き出しの説明で「申告（確かめていない）」と書く（D4（仮））。S-01 の版は S-01 が入れる
- [記録 1 件あたり 40〜60 バイト増える] → C-02 のウィンドウは 1 日数千件で 1 日 300 KB 未満。未送信の上限（2 GB）に対して無視できる（review/deep.md の手順 4）
- [ST41・ST23 と同じ関数と移行の本体を触る] → D19。後から入る側が rebase で写し直す
- [D21（仮）で、版を直すたびに前の版の行が 1 つ増える] → 版を直すのは古い組み立ての誤った印を見つけたときだけで、量は小さい

## Migration Plan

1. S-01 を ST49 の版にする（移行を当てる。既存の行の版は NULL のまま）。画面も同じ組み立てで入れ替わる
2. その後で C-02、C-01 を ST49 の版に入れ替える（逆の順だと、その間に送った記録の版が落ちる。D3（仮））
3. 戻すとき: `.down.sql` が列を落とす（版の値は失われる —— 戻す前に書き出しを取る）。収集アプリを先に古い版へ戻すと、その間に送る記録の版は落ちる

この順は `docs/collector-contract.md` の `client_version` の節に書き、`docs/handoff/ST50.md` に期限の後の止め方を申し送る。

## Open Questions

（無し）
