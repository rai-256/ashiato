# ST54 — 開発版を正式版と並べて入れられる

## Why

開発版を試すには、正式版と同じ携帯・同じ PC に並べて入れる必要がある。しかし今は並べられない。
携帯の収集アプリは、組み立ての仕組み（ST52）こそ `.dev` の識別子を持つが、開発版のリリースに携帯の APK が無く、開発版の鍵もまだ無い。
PC の収集アプリは、自動起動の項目（`ashiato-collector.cmd`）と通知の差出人（`dev.ashiato.collector`）が固定で、開発版を入れると正式版の分を上書きする（premises P4）。
サーバは、DB の繋ぎ先も待ち受けも正式版と同じ既定を持ち、取り違えた先で移行を当てると正式版の DB を先へ進めてしまう（P6・P12・P13）。
開発版の不具合が正式版の記録・自動起動・通知・DB に及ばないように、並べたときに取り合うものを全部分ける（FR-106・扉 34）。

## What Changes

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q4・Q6・Q9 は A（失うものがある）で、本人の答えが推奨と違えば `spec --answers` が書き直す。

- **開発版かどうかは、組み立てのときの版番号で決まる**（Q1 の推奨・D1）: 版番号に `-dev.` を持つ組み立てが開発版。携帯（ST52 の `applicationIdSuffix`）・PC の収集アプリ・サーバが同じ規則で読む。手で組んだ版（版番号に `-dev.` が無い）は正式版として振る舞う
- **携帯の開発版を開発版のリリースに載せる**（ST52 Q4・Q11 の申し送り・C3・C4・C5）: 開発版の作り直しが携帯の APK を組み、開発版の鍵（`nightly` の Environment）で署名し、指紋を `ASHIATO_EXPECT_DEV_CERT_SHA256` で確かめる。
  開発版のリリースも 4 つの配布物になり、`distribution` の「ST54 まで」の例外を外す。開発版の鍵が無ければ作り直しは落ち、前の開発版が残る
- **携帯の開発版は名前で見分ける**（Q7・Q10 の推奨・C1）: アプリの名前を「あしあと。開発版」にする。識別子が違うので、端末内の記録の置き場・権限・通知の設定は Android が分ける。動かす収集は正式版と同じ全部
- **PC の収集アプリの開発版は、取り合う名前を全部分ける**（Q1・Q2・Q7 の推奨・C6）: 自動起動の項目 `ashiato-collector-dev.cmd`、通知の差出人 `dev.ashiato.collector.dev`（表示名「ashiato 開発版」）、実行ファイル `ashiato-collector-windows-dev.exe`。
  置き場は渡された置き場の下の `dev`、端末識別子は末尾に `-dev` を付ける。除外の登録は、渡された置き場（正式版と同じ値を渡せば正式版のもの）を読む
- **収集アプリは、自分と同じ系統のサーバにだけ送る**（Q6 の推奨・D7（仮））: サーバが `GET /contract` の応答に見出し `Ashiato-Channel` で自分の系統（開発版か正式版か）を載せる（答えの本文の形は変えない）。開発版の収集アプリは正式版のサーバへ、正式版の収集アプリは開発版のサーバへ、何も送らずに手元に残す。系統を答えない古いサーバは正式版とみなす
- **DB の系統の守り**（Q3 の推奨・D3）: DB が開発版の DB か正式版の DB かを、最初に当てた移行の版の記録（組み立ての版番号）で決める。違う系統のサーバと `migrate` は、何も書かずに終了コード 4 で止まる。版の記録の無い DB に移行を当てる・起動するのは正式版だけ
- **開発版の DB が main とずれたときの扱い**（Q4 の推奨・D4・D5（仮））: DB は書き換えずに止まり、原因（知らない版の名前と、当てた組み立ての版番号）を示す。main 側で直す道を 2 つ用意する ——
  移行の名前を変えた・消したら `migrations/LINEAGE` に旧名の対応を書き、サーバはそれを知っている版として読む。開発版に載った移行の中身を変えたら、直しの移行を足して LINEAGE に書く。開発版に載った移行を対応なしに変えた PR は CI が落とす
- **開発版のサーバは、正式版と別の既定を持つ**（C2・C6・C7・C8・D8（仮））: 既定の待ち受け `127.0.0.1:18788`、画面のログインの名前 `ashiato_session_dev`、移行の前の退避の既定の置き場 `backups/pre-migrate-dev`。開発版の DB は手順書で別の器（ポート 55433）に立てる
- **開発版のリリースに「サポートしない」を書く**（Q8 の推奨・D9）: 公開のたびに説明文を決まった文で書き直し、先頭にその文があることを公開の手順が確かめる
- **開発版の印 `nightly` は動かす方式のまま**（Q5 の推奨）: 手順書に、手元の clone では `git fetch --tags --force` を使うと書く
- **完了は下書きのリリースと使い捨ての鍵で確かめる**（Q9 の推奨・D10）: 本物の開発版の公開と本物の開発版の鍵は、ST53 の後に本人が始める

**BREAKING ではない**: 正式版の識別子・自動起動の項目・通知の差出人・既定の待ち受け・ログインの名前・退避の置き場は変えない。収集アプリが送る道と封筒の形も変えない。
正式版の収集アプリが新しく断るのは「開発版と答えたサーバへ送る」ことだけで、今ある正式版のサーバは系統を答えないので正式版とみなされ、今どおり送る。

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `distribution`: ST52 の 3 つの Requirement から「ST54 まで」の例外を外す（開発版も 4 つの配布物・開発版の作り直しが携帯の APK を組んで開発版の鍵で署名する）。
  「携帯の APK は、チャンネルごとに決めた識別子と鍵で署名する」は MODIFIED。「印（タグ）の名前で、3 つのチャンネルに分けて配る」「開発版は、幹にその日の変更があった日だけ作り直す」は、例外を固定していた Scenario 3 本が偽になり、OpenSpec が MODIFIED で Scenario を落とせないので、
  REMOVED にして名前を変えた ADDED に移した（ST51・ST21 の前例。条項と Scenario は写し、変えた所だけを REMOVED の Migration に書いた）。
  「手で起こした確かめの組み立ては、下書きのリリースにだけ置き、印を作らない」を MODIFIED（開発版の下書き。Q9・D10）。
  ほかの ADDED は 10: 開発版の判定・携帯の開発版を並べる・PC の収集アプリの開発版の名前・PC の収集アプリの開発版の置き場と端末識別子・同じ系統のサーバにだけ送る・DB の系統の守り・旧名の対応・開発版に載った移行の検査・開発版のサーバの既定・開発版のリリースの「サポートしない」

**capability の照合**: INDEX.md の capability 表は ST54 を `distribution` に割り当てている。PC の収集アプリの自動起動・通知・置き場（`desktop-collection`）、携帯の送信（`device-collection`）、画面のログイン（`data-sensitivity`）にも触れるが、
どれも「版番号が `-dev.` のときだけ」か「相手のサーバの系統が違うときだけ」に働く分岐で、正式版の振る舞いを書き換える Requirement は無い（それぞれの正典に、その名前を固定した Requirement も Scenario も無い。`grep -n "ashiato-collector.cmd\|ashiato_session\|18787" openspec/specs/*/spec.md` は 0 件）。
送信契約の問い合わせの答えの形（正典 record-envelope。最初の配った印の後は変えない）に触れないよう、系統は本文の欄でなく応答の見出しで運ぶ（review/spec.md R1）。収集アプリの「送らずに手元に残す」理由（desktop-collection・device-collection の契約の版の Requirement）は、列挙を閉じた形で書いていないので、「系統が違う」を distribution の Requirement が足しても食い違わない。
  そこで、開発版と正式版を並べることに関わる規則を `distribution` の 1 か所に置き、読む人が系統の規則を 1 つの正典で見られるようにした。前倒しは無く、INDEX.md の表は変えない。

**requires**: ST52 は archive 済み（`openspec/changes/archive/2026-11-02-st52-release-channels`）で、MODIFIED する 3 つの Requirement は正典 `openspec/specs/distribution/spec.md` の今の本文から写した。ST53（許諾）は requires に無い —— ST54 は本物の開発版を公開しない（Q9）。

**申し送りの受け先**（`docs/handoff/ST54.md` の全部の節）:
- ST51 premises 9（並べた PC の収集アプリの端末識別子）→ Q2・D2
- ST52 Q4・Q11（開発版に携帯の APK と開発版の鍵）→ C3・C4・Q9・D6。鍵の形（ST52 D3（仮））は P15 で確かめた（`make-key.sh --self-test` rc=0。RSA 4096・10000 日）
- ST52 R11（開発版の DB の食い違いと `nightly` の印）→ Q4・Q5・D4・D5（仮）
- ST52 code_verify R26（開発版の指紋 `ASHIATO_EXPECT_DEV_CERT_SHA256`）→ C4・D6

## Impact

- 系統の判定: `crates/build-stamp`（`channel(版番号)` を足す）・`collector-android/app/build.gradle.kts`（アプリの名前を開発版だけ替える `manifestPlaceholders`）・`collector-android/app/src/main/AndroidManifest.xml`（`android:label`）
- サーバ: `crates/server/src/contract_version.rs`（`GET /contract` の応答に見出し `Ashiato-Channel`）・`crates/server/src/migration_ledger.rs`（DB の系統の守り・`migrations/LINEAGE` の対応）・`crates/server/build.rs`（LINEAGE を組み込む）・`crates/server/src/lib.rs`（終了コード 4・既定の待ち受け）・`net_guard.rs`・`web_session.rs`・`migrate_backup.rs`・`docs/openapi.json`
- PC の収集アプリ: `crates/collector-windows/src/{autostart,toast,config,main,runtime,sender,contract_gate}.rs`・`tests/runtime_windows.rs`・`tools/release/build.sh`（開発版の実行ファイルの名前）
- 携帯の収集アプリ: `ContractGate.kt`（系統を読む）と、送る 5 つの口（`Sender.kt` ほか。ゲートの答えに従うだけ）
- 移行: 新しい `migrations/LINEAGE`（空で始まる）・`tools/check-migrations.sh`（LINEAGE の形）・新しい `tools/check-nightly-migrations.sh`（開発版に載った移行の変更）・`.github/workflows/ci.yml`
- 配る道具: `.github/workflows/nightly.yml`（携帯の APK・署名・説明文）・`tools/release/check-artifacts.sh`（開発版も 4 つ）・`tools/release/check-guards.sh`（workflow の検査を反転）・`tools/release/draft.sh`（開発版の下書き）・新しい `tools/release/nightly-notes.md`
- 新しい台本: `tools/android-side-by-side.sh`（エミュレータに 2 つを並べて入れる）
- 文書: `docs/release.md`（開発版の鍵・`nightly` の Environment・開発版の DB・`git fetch --tags --force`・LINEAGE の書き方）
- **走っている change との重なり**: `openspec/changes/` で archive 以外はこの change だけ（`spec_overlap.py candidates` は空）。重なりは無い
- **要件へ戻すもの（record の段が本人の答えの後に判断する）**: FR-104 の ★ 2026-11-01 の「開発版（nightly）のリリースには ST54 まで C-01 を並べない」は、この Story の後は当たらない。Q9 を推奨のまま答えるなら、完了の判定に 3 つ（開発版のリリースに携帯の APK が並ぶ・開発版の鍵で署名されている・「サポートしない」の文）を足す（D10）
