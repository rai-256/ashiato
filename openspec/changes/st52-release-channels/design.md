# ST52 design — 3 つの部品を 1 つの版で、3 つのチャンネルに配る

## Context

動機は `proposal.md` の Why。いまの状態（premises の P1・P9・P10・P11）:

- 版番号は 3 か所に手書き（`Cargo.toml` の workspace `0.1.0`・`build.gradle.kts` の `versionName = "0.1.0"` と `versionCode = 1`・`web/package.json` の `0.1.0`）。
  組み立ての印（ST49）は `crates/build-stamp`・gradle・`web/build-stamp.ts` がそれぞれの出所から作る。版の記録（ST48）の `build_version` は `env!("CARGO_PKG_VERSION")`
- サーバは API だけを持ち、道は根から（`/ingest`・`/events`…）。画面は `vite preview` が別の番号で配り、`/api/…` を剥がしてサーバへ渡す（`web/vite.config.ts`）。画面は道で振り分けない（client の router が無い）
- CI（`.github/workflows/ci.yml`）は `contents: read` の 1 本で、`changes` の job が出し分ける。サーバと PC の収集アプリは Windows で組めることを確かめてある（P1・P9）
- 配った印の検査（`tools/check-released-migrations.sh`・`tools/check-contract-version.sh`）は `git tag --list 'v[0-9]*'` の全部を数え、後者は「最後の印」を作った日時で選ぶ
- 配布元 `rai-256/ashiato` は公開リポジトリで、`v*` の印は 0 個（P6・P11）

## Goals / Non-Goals

**Goals**: 印を付けるだけで 3 部品が同じ版で並ぶ仕組みを、本物の印を打たずに（下書きで）通す。配れる形を PR ごとに確かめる。正式な鍵の置き場と作り方を決め、台本と手順を用意する。

**Non-Goals**:
- 最初の本物の印 `v0.1.0-beta.1` を打つこと・開発版の公開を始めること（Q5 の推奨。ST53 の後に本人が打つ。`docs/handoff/ST53.md`）
- 本人の携帯を配布版へ切り替えること。Q1 の答え（本人の携帯にはまだ入っておらず、記録も集めていない）で、切り替えも入れ直しも署名の引き継ぎも要らない。最初に入れるのは ST56 の後の正式な鍵の版。本人のサーバを配布版へ移すこと（D4（仮）。印を打った後の確認）
- 開発版のリリースに携帯の APK を並べること・開発版の鍵を作って置くこと（Q4・Q11。ST54。ST52 が用意する範囲は D23（仮））
- インストーラ（ST55）・開発版と正式版を並べて入れる仕組みと開発版の DB の扱い（ST54）・更新の確認と適用（ST57・ST58）・許諾の同梱（ST53）
- 鍵そのものを作ること・GitHub の Environment とルールセットを入れること（本人の手。手順は `docs/release.md`）

## Decisions

### D1. 配った印の検査は、今の木の祖先にある印だけを数え、最後の印は版番号で選ぶ（未決。Q12 の答え待ち）

> 仮で進めたが、別の席（`provisional-review.md` の D1）が否とした: CI の浅い checkout では `git tag --merged HEAD` が空になり検査が黙って通る（`.github/workflows/ci.yml:68-69`・`:93`）、かつ「祖先でない印は凍結に入れなくても失われない」は D6（仮）に乗る。台帳から外し、`deep.md` の Q12 で人間に返した。下の本文は推奨で書いた案のまま

`check-released-migrations.sh` は `git tag --merged HEAD --list 'v[0-9]*'` を数える。`check-contract-version.sh` は同じ集合から「最後の印」を、作った日時（`--sort=-creatordate`）ではなく版番号の順で選ぶ。
順は D11 の `tools/release/version.sh order`（versionCode の式と同じ並び。ベータ < 正式、ベータは N の順）で決め、`git` の `versionsort.suffix` の設定に頼らない（手元の設定で順が変わる）。
- 選ばなかった案: 全部の印を数える（今のまま）。修正の枝の上では、後から main で打ったベータの印の移行が「消した」で落ちる。main の上では、後から打った修正の印が「最後」になり、ベータで上げた送信契約の版との比べが抜ける（review/deep.md R9）
- 反転条件: 修正の枝を merge して main に戻す運用にするなら（FR-105 は「一時的に切る」で戻さない）、祖先の判定は変わらないが、修正の枝の印も main の祖先に入るので、版番号の順で選ぶことだけが効く
- ST48・ST50 の申し送り「型を変えるなら一緒に変える」に当たるので、ST52 が 2 本の台本と自己試験を直す（印の型 `v[0-9]*` 自体は変えない）

### D2（仮）. 配った印 `v*` は消さない・動かさない。ルールセットで拒み、公開の前に確かめる

配布元のリポジトリのルールセット（対象: 印 `refs/tags/v*`、規則: 削除・更新の禁止。迂回する者を置かない）を本人が入れる（`docs/release.md` の手順）。`release.yml` は公開の前に
`tools/release/check-guards.sh rules` で `gh api repos/{owner}/{repo}/rules/tags/<印>` を読み、`deletion` と `update`（non_fast_forward を含む）の規則が当たっていなければ公開しない。
- 選ばなかった案: 規約で守る。消した時点で凍結が黙って外れ、気づく手段が無い
- 反転条件: GitHub の API でルールセットを読めない（権限の不足）と分かったら、公開の前の確かめを、本人が入れた後に 1 回だけ人間が見る手順に替える

### D3（仮）. 正式な鍵と開発版の鍵は RSA 4096・有効期限 25 年以上で作る

`tools/release/make-key.sh <official|dev> <出力の置き場>` が `keytool -genkeypair -keyalg RSA -keysize 4096 -validity 10000` で作り、`keytool -list -v` を読み直して種類・長さ・有効期限を確かめる。
合言葉は台本の引数に取らず、標準入力から読む（履歴に残さない）。Play App Signing に持ち込める条件の一次情報は tasks で premises に足す。
- 反転条件: 一次情報が「RSA 2048 以上・期限 25 年以上」より厳しい条件を示したら、作る前に引数を変える（作った後は替えない）

### D4（仮）. 本人のいまのサーバは、印を打った直後にその印の版へ移す

印を打つのは ST53 の後なので、ST52 の確認バッチでは手順書（`docs/release.md` の「本人の環境を移す」）を読むだけにする。DB が配布版より先に進んでいれば、配布版のサーバは知らない版で止まり DB を書き換えない（ST48）。そのときは次の印まで移らない。

### D5（仮）. PC の収集アプリを新しい版から古い版へ入れ直すことを ST52 では止めない

ST52 は PC の収集アプリを入れる操作を持たない。止めるか知らせるかは、入れる操作と一緒に ST58 が決める（`docs/handoff/ST58.md`）。

### D6（仮）. 緊急の修正の枝 `release/<MAJOR>.<MINOR>` は、配った正式版の印から切り、移行と封筒の形を変えない

`tools/release/version.sh plan <印>` は、印の commit が main の祖先でないとき、その commit が `release/<MAJOR>.<MINOR>` の枝の上にあり、印が正式版で、枝の元の印
（`git describe --tags --abbrev=0 --match 'v<MAJOR>.<MINOR>.*' <印>^`。正式版の印に限る）と `migrations/ORDER`・`docs/contract/envelope.json` が同じときだけ公開を許す。
- 失わない側: 枝で移行や封筒を変えられないだけで、main では今どおり変えられる。直したい欠陥が移行に要るなら、main で直して次の版を出す
- 反転条件: 移行を含む緊急の修正が本当に要る事態が起きたら、その移行を main にも同じ名前・中身で入れる規則を足して許す
- 台帳: `provisional.jsonl` の D6（source: review/deep.md#R9）

### D7. 配布物の検査は `tools/release/check-artifacts.sh` の 1 本に集め、PR と公開の両方が通す

検査すること: チャンネルの配布物がそろう（名前は D17。正式版とベータ版は 4 つ、開発版は ST54 まで携帯の APK を除く 3 つで、APK があれば落とす。`--channel nightly` で切り替える。`--channel nightly` は APK を期待しないので、署名の指紋と `BuildConfig` の検査を当てず、`ASHIATO_EXPECT_CERT_SHA256` が空でよい。Q4・D23（仮））・どれも組み立ての印を持ち、版番号が 1 つで、`.dirty` / `.debug` が無い（C8）・APK の `BuildConfig` の `BASE_URL`・`API_TOKEN`・`USER_ID` が空（C1）。
印の読み方は `tools/check-build-stamp.sh` と同じ（Rust は実行ファイルを `strings`、APK は `apkanalyzer`／`aapt2` で `versionName` と、dex から `BuildConfig` の定数を読む）。
空でない資格情報は項目の名前だけを示し、値を出さない。PR の job はこれを `--allow-debug-key` で走らせる（署名の鍵だけを見ない）。

### D8. 印の判定は `tools/release/version.sh plan <印>` が決め、workflow は判定に従うだけ

`plan` は印の名前・印の commit・出所の版から、`channel`（`release` / `beta` / `nightly`）・`version`・`version_code`・`prerelease`（true/false）を出力し、次のどれかで 0 でない終了コードを返す:
形が `v<数>.<数>.<数>` か `v<数>.<数>.<数>-beta.<1〜98>` でない・MAJOR.MINOR.PATCH が出所の版と違う・印の commit が main の祖先でない（D6 の例外を除く）。
workflow の YAML に判定の論理を書かない（YAML は手元で試せない。台本は `--self-test` で使い捨ての git の木に当てる）。

### D9. 開発版は日本時間 0:00 に起き、`nightly` と main の HEAD の違いで作り直す

`nightly.yml` の `schedule` は `cron: "0 15 * * *"`（UTC。日本時間 0:00）と `workflow_dispatch`。最初の手順で `tools/release/version.sh nightly-plan` が
`nightly` の印の commit（無ければ空）と `origin/main` の HEAD を比べ、同じなら job を終える（C6）。版番号の日付は日本時間の今日（`TZ=Asia/Tokyo date +%Y%m%d`）。
予定の実行は混むと遅れる・落ちる（P7）が、窓でなく commit の違いで決めるので次の回で拾う。時刻は後から変えれば戻る。
出所の版の正式版の印 `v<出所の版>` が main の祖先にあれば、`nightly-plan` は `build=no` と「出所の版を上げる」を返す（Q6 の推奨「正式版を出した後は出所を次の版に上げる」を、手順書でなく機械で守る）。
上げ忘れで止まるのは開発版だけで、PR の検査（`check-version.sh`）は落とさない —— 関係の無い PR を止めない側。止めるのを PR に広げるのは、検査に 1 行足せば戻る

### D10. 3 部品がそろったときだけ公開する

`release.yml` と `nightly.yml` は、組み立ての job（サーバの Linux 向け・サーバの Windows 向け・PC の収集アプリ・携帯の APK。`nightly.yml` は ST54 まで携帯の APK を組まない。Q4）が全部成功した後の 1 つの `publish` の job だけが配布元に書く（`needs` で全部を待つ）。
`publish` は `check-artifacts.sh` を通してから `gh release create`（正式版・ベータ版）か、`nightly` の配布物の入れ替え（D15）をする。1 つでも落ちれば `publish` は走らず、開発版は前の版のまま残る（C7）。

### D11. 版番号の計算は `tools/release/version.sh` の 1 か所、gradle は同じ式を持ち、試験で突き合わせる

`version.sh` の口: `source`（出所の版を `Cargo.toml` から読む）・`check <版番号>`（形と出所との一致）・`code <版番号>`（versionCode）・`order`（標準入力の印を版番号の順に並べる）・`plan <印>`（D8）・`nightly-plan`（D9）・`--self-test`。
gradle（Kotlin DSL）は組み立ての中で shell を呼ばずに同じ式を持つ（`build.gradle.kts` の関数。configuration cache を壊さない）。2 つの式が同じ値を出すことは、版番号と versionCode の表 `tools/release/version-table.tsv` を `version.sh --self-test` と gradle の task `verifyVersionCodeTable` の両方が読んで確かめることで固定する。
- 選ばなかった案: gradle から `version.sh` を呼ぶ。Windows の組み立てで bash を要求し、configuration cache の入力が増える

### D12. 版番号は環境変数 `ASHIATO_BUILD_VERSION` で渡し、4 つの部品の組み立てが同じ規則で受ける

- `crates/build-stamp` に `version(source: &str) -> Result<String, String>` を足す。`ASHIATO_BUILD_VERSION` が空でなければ形と出所との一致（Requirement「3 つの部品の版番号は、1 つの出所から決まる」）を見て返し、違えば `build.rs` が `panic!` で組み立てを止める。
  `crates/server/build.rs` と `crates/collector-windows/build.rs` はこれを使い、`cargo:rustc-env=ASHIATO_VERSION=<版番号>` を出す。`migration_ledger::BUILD_VERSION` は `env!("ASHIATO_VERSION")` にする（印と版の記録が同じ版番号になる）
- gradle は出所の版を根の `Cargo.toml` の `[workspace.package]` から読み（`providers.fileContents` で入力に載せる）、`ASHIATO_BUILD_VERSION` を `providers.environmentVariable` で読み、同じ形の検査をする
- 画面は `web/build-stamp.ts` が出所の版を根の `Cargo.toml` から読み（`package.json` は読まない）、`ASHIATO_BUILD_VERSION` を同じ規則で受ける。`web/package.json` の `version` は消す（`private: true` なので npm は求めない）
- OpenAPI の `info.version`（utoipa が `CARGO_PKG_VERSION` を引く）は出所の版のまま（`docs/openapi.json` は checked-in で、渡した版番号を入れると PR の検査がずれる）
- `tools/check-version.sh`（`--self-test` つき）は、`build.gradle.kts` に文字の `versionName = "` / `versionCode = <数>` が無い・`web/package.json` に `version` が無い・出所の版が `0.<数>.<数>` の形、を見る。CI の `chain` の job で走らせる（速い）
- `ASHIATO_BUILD_VERSION` は `rerun-if-env-changed` に載せる（渡した版が変われば組み直す）

### D13. サーバは画面の組み立て済みのファイルを組み立ての時に取り込み、`/api` を剥がして今の router に渡す

- `crates/server/build.rs` は環境変数 `ASHIATO_WEB_DIST`（`vite build` の出力の置き場）が空でなければ、その中のファイルを全部読み、`OUT_DIR` に「道 → 中身・種類」の表の Rust のソースを書き出す（`include_bytes!`）。空なら空の表。
  新しい依存の crate は足さない（`rust-embed` などを入れない。表は数十ファイル）。`ASHIATO_RELEASE=1`（配る組み立て）で `ASHIATO_WEB_DIST` が空なら `panic!`
- `crates/server/src/web_assets.rs` は表を引く handler。`/` は `index.html`、ほかは表の道そのまま。無ければ 404。応答に `Content-Security-Policy: default-src 'self'; frame-ancestors 'none'` と `Cache-Control: no-store`、種類は拡張子から
- `router()` に `/api` の前置きを剥がす層を足し（剥がした道を今の router に渡す。`nest` で router を 2 回組まない）、画面の表は `fallback` に置く（今の API の道が先に当たる）。認可・読み出しの記録・`net_guard` は剥がした後の道で今と同じに当たる
- 画面の道の `.dirty`: `web/dist` は gitignore で、S-01 のソースの道（ST49 の D5: 自分の crate・印の crate・移行・workspace の宣言）に `web/` は入らないので、画面を取り込んでも S-01 の印は `.dirty` にならない。画面の版は画面自身の印（ST49）が持つ
- 選ばなかった案: 実行ファイルの隣のフォルダから配る（Q8 の選択肢 2）。画面とサーバの版がずれうる

### D14. workflow は 3 本に分け、書く権限は公開の job だけが持つ

- `ci.yml`（今の 1 本）に `package` の job を足す（Q9）: `changes` の `code` が true のときだけ。Linux の runner でサーバ（Linux 向け）・携帯の APK（`ASHIATO_RELEASE=1` の release を debug 鍵で署名）、Windows の runner でサーバ（Windows 向け）・PC の収集アプリを、
  `ASHIATO_BUILD_VERSION` を渡さず（出所の版で）`tools/release/build.sh` で組み、`check-artifacts.sh --allow-debug-key` を通し、`actions/upload-artifact` で置く。`permissions` は `contents: read` のまま
- `release.yml`: `on: push: tags: ['v*']`。組み立ての job は `contents: read`、`publish` の job だけ `contents: write` と `environment: release`
- `nightly.yml`: `on: schedule` と `workflow_dispatch`。`publish` の job だけ `contents: write` と `environment: nightly`。ST54 まで携帯の APK を組まず、署名もしない（Q4・D23（仮））
- Windows の組み立ては 1 回あたり数分で、公開リポジトリでは標準の実行は課金されない（P6）。`package` は Windows の 2 つを 1 つの job にまとめる（runner の起動を 1 回に）

### D15. 開発版の公開はリポジトリの変数 `ASHIATO_NIGHTLY_PUBLISH` が `on` のときだけ

`nightly-plan` は、`ASHIATO_NIGHTLY_PUBLISH` が `on` でなく、起きたのが予定の実行（`schedule`）なら `build=no`（何も組まない。FR-107「変更の無い日は作らない」と Actions の枠。review/spec.md R4）、手で起こした実行（`workflow_dispatch`）なら `build=yes`・`publish=no` を返す。
`nightly.yml` の `publish` の job は `if: vars.ASHIATO_NIGHTLY_PUBLISH == 'on'` も持つ（判定が外れても公開しない二重の守り）。入っていない間は印 `nightly` も動かさない（Q5 の推奨: 開発版の公開は ST53 の後に本人が始める）。
公開するときは、`nightly` のリリース（先行公開・1 つだけ）の配布物を `gh release upload --clobber` で入れ替え、要らなくなった名前の配布物を消し、印を `git push --force origin <commit>:refs/tags/nightly` で動かす。
`nightly` は D2 のルールセットの対象（`v*`）の外。
- 反転条件: Q5 の答えが「ST52 で公開を始める」なら、`ASHIATO_NIGHTLY_PUBLISH` を ST52 の確認で入れる手順に替える

### D16. ST52 の確かめは、PR の組み立ての配布物で下書きのリリースを作って見る

`tools/release/draft.sh <run-id> <版番号>` は、`gh run download` で PR の `package` の配布物を落とし、`check-artifacts.sh --allow-debug-key` を通し、`gh release create --draft [--prerelease] --target main` を出す。
印は作らない（下書きは公開するまで印を作らない。P16。tasks で走らせて確かめる）。確認バッチで本人が下書きの並び（4 つの配布物・先行公開）を見て、下書きを消す。
`--self-test` は偽の `gh`（`PATH` の先に置いた台本）で、出す要求を記録して突き合わせる。

### D17. 配布物の名前

`ashiato-server-<版番号>-windows-x86_64.zip`・`ashiato-server-<版番号>-linux-x86_64.tar.gz`・`ashiato-collector-<版番号>.apk`（開発版は `ashiato-collector-dev-<版番号>.apk`。開発版に並べるのは ST54 から。名前は ST52 が決めておく）・`ashiato-collector-windows-<版番号>-x86_64.zip`。
サーバの zip にはサーバの実行ファイルだけを入れる（画面は中に取り込んである。D13）。名前は ST57・ST58 が版の確認で読むので、版をまたいで変えない（spec の「印（タグ）の名前で、3 つのチャンネルに分けて配る」に置いた）。

### D18. 携帯の APK の署名は、Environment の付いた公開の job の中だけで当てる

組み立ての job は鍵を持たない。携帯の release の APK を、PR の `package` では debug 鍵で、`release.yml` の組み立てでは**署名せずに**作る（`nightly.yml` は ST54 まで携帯の APK を組まない。Q4）。署名しない切り替えは `ASHIATO_APK_UNSIGNED=1` で、`release.yml` の組み立ての job だけが立てる。
`ASHIATO_RELEASE=1`（配る組み立て。画面の取り込みが要る・資格情報を渡さない）とは分ける —— PR の `package` は `ASHIATO_RELEASE=1` で配るときと同じ手順を通し、署名だけ debug 鍵にする（review/spec.md R9）。
署名は `publish` の job（Environment の付いた job）の中だけで、`apksigner sign` を当てる。鍵は Environment の Secrets（鍵の置き場を base64・`ASHIATO_SIGNING_STORE_PASSWORD`・`ASHIATO_SIGNING_KEY_ALIAS`・`ASHIATO_SIGNING_KEY_PASSWORD`）から `$RUNNER_TEMP` に戻し、job の終わりに消す。
鍵の Secrets が空なら `tools/release/sign-apk.sh` は 0 でない終了コードで終わり、`publish` はリリースを作らない（署名しない APK を公開しない）。署名した後に `check-artifacts.sh`（署名の証明書の指紋が鍵のものか・識別子がチャンネルのものか）を通してから公開する。指紋の期待値は秘密でないので、Environment の変数 `ASHIATO_CERT_SHA256`（ST52 は `release` に。`nightly` の変数は開発版の鍵と一緒に ST54 が置く）に置き、`check-artifacts.sh` は環境変数 `ASHIATO_EXPECT_CERT_SHA256` で受ける。
開発版の識別子は `ASHIATO_BUILD_VERSION` が `-dev.` を持つときだけ `applicationIdSuffix = ".dev"`（扉 34）。この設定と `sign-apk.sh` の開発版の側は ST52 が作って使い捨ての鍵で確かめ、開発版のリリースに載せるのは ST54（Q4・D23（仮））。これで鍵は Environment の付いた job の中にだけ現れる。

### D19. 最初の印と開発版の公開を始める手順は `docs/release.md` に置き、ST53 に申し送る

`docs/release.md` は次を持つ: 鍵を作る（D3）・控えを USB と拠点外に置く（扉 34）・Environment `release`（配る対象: 印 `v*` だけ。D21（仮））を作り Secrets と指紋の変数 `ASHIATO_CERT_SHA256` を置く・`nightly`（枝 `main`）を作る（ST52 では鍵を置かない）・開発版の鍵（正式な鍵と別。控えは USB に 1 部。Q11）は ST54 が開発版に携帯の APK を載せるときに `make-key.sh dev` で作って `nightly` に置く（手順だけを書く）・ルールセット（D2）を入れる・最初の印を main の commit に打つ・
開発版の公開を始める（D15）・壊れた版を出したら次の版で直す（D2）・修正の枝の切り方と消し方（D6）・本人の環境を移す（D4）。`docs/handoff/ST53.md` に「最初の印と開発版の公開は ST53 の確認の後に本人が始める（ST52 Q5）」を書く。

### D20. 縦串（`tools/stack.sh`）は画面を先に組み、サーバにも取り込ませる

`tools/stack.sh` は画面の `vite build` をサーバの `cargo build` より先に走らせ、`ASHIATO_WEB_DIST=<画面の出力>` を渡してサーバを組む。サーバは `$BIND` の `/` でも画面を配り、
`vite preview` は今の `WEB_PORT` で今どおり画面を配る（壊してはいけないもの。e2e の既存の spec と確認バッチの `run.sh` は `WEB_PORT` を開く）。
ST52 の e2e（`web/e2e/st52-served-ui.spec.ts`）だけが `$BIND` の `/` を開き、「配ったサーバだけで画面が開く」を本物のブラウザで見る。
- 選ばなかった案: e2e が自分でサーバを別に組んで立てる。縦串と別の DB・資格情報を用意することになり、e2e の土台が 2 つになる

### D21（仮）. 配布用の Environment `release` は印 `v*` にだけ開く（枝 `main` に開かない）

正式な鍵を使うのは `release.yml`（印の push で走る）の `publish` だけで、main の上の実行には正式な鍵の要る job が無い。`main` にも開くと、main に入った workflow の変更（AI が書いた PR を本人が merge したもの）が次の main の実行で正式な鍵を読める。
開発版の Environment `nightly` は、予定の実行が main の上で走るので枝 `main` に開く。
- Q2 の推奨の文面は「印と main にだけ開いた Environment」で、その狭い側（印だけ）にした。失うものは無い（`release.yml` は印の ref で走る）
- 反転条件: main の上で正式な鍵を使う job が要るとわかったら（例: 正式版の再署名）、その job の理由を書いて `main` を足す
- 台帳: `provisional.jsonl` の D21（source: review/spec.md#R10。Q2 に依る）

### D22. ST52 の完了の判定は、下書きのリリースと使い捨ての鍵で確かめた形で見る（未決。Q13 の答え待ち）

> 仮で進めたが、別の席（`provisional-review.md` の D22）が否とした: この読み替えは Q5 だけでなく Q2（正式な鍵で署名）と Q1（携帯に入っていない）にも乗るのに、台帳と反転条件が Q5 しか挙げていなかった。台帳から外し、`deep.md` の Q13 で人間に返した。下の本文は推奨で書いた案のまま

Q5・Q2 の答えのもとでは、ST52 の確認バッチで本物の印は打たれず、正式な鍵はまだ作られず、本人のサーバは切り替わらない（本人の携帯には Q1 の答えのとおり収集アプリがまだ入っていない）。そこで ST52 の完了の判定（`docs/stories/ST52.md`）を次のように読む:
「印を付けると並ぶ」→ 印の判定（`version.sh plan`）と 4 つの配布物の組み立てが通り、下書きのリリースに 4 つが並ぶ（D16）／「正式な鍵で署名されている」→ 署名の手順が使い捨ての鍵で通り、鍵を Environment に置けば印の push で署名される／
「自分の環境をその版で動かす」→ ST53 の確認の後に本人がサーバを移す手順が `docs/release.md` にある（D4・D19。携帯は ST56 の後に最初から正式な鍵の版を入れる。Q1）。本物の公開・正式な鍵・本人の環境の切り替えは ST53 の確認で見る（`docs/handoff/ST53.md`）。
- 反転条件: Q5 で「ST52 の確認バッチで本人が打つ」が選ばれたら、この読み替えを外し、本物の印の公開を確認バッチで見る。Q1・Q2 の答えが変わったときも同じく読み直す。答えが推奨のままなら record の段が `stories.json` の完了の判定をこの形に書き直す
- 台帳: なし（`provisional.jsonl` の D22 は resolve 済み。Q13 の答え待ち。source: review/spec.md#R7。Q1・Q2・Q5 に依る）

### D23（仮）. 開発版の携帯の APK の道具は ST52 が作って確かめ、開発版のリリースに載せるのは ST54

Q4 の答え（ST54 まで開発版に携帯の APK を入れない）と Q11 の答え（開発版は正式な鍵と別の鍵）を合わせて、ST52 の範囲を次のように切る:
- ST52 が作る: gradle の `.dev`（`-dev.` の版番号のときだけ）・`sign-apk.sh` の開発版の側と取り違えの止め・`make-key.sh dev`・配布物の名前 `ashiato-collector-dev-<版番号>.apk`（D17）。どれも使い捨ての鍵の `--self-test` で閉じ、配布元には何も出さない
- ST52 がしない: `nightly.yml` で携帯の APK を組む・署名する。開発版の鍵を作る・`nightly` の Environment に鍵と指紋の変数を置く（ST54。`docs/handoff/ST54.md`）
- 開発版の配布物の検査（`check-artifacts.sh --channel nightly`）は、携帯の APK を除く 3 つを期待し、APK があれば落とす（FR-104 ★ 2026-11-01 の例外を機械で守る。ST54 が外す）
- 失わない側: 何も配らないので、開発版の鍵も識別子もまだ外に出ていない（Q11 の `exported` はまだ起きない）。道具を ST54 へ回す側へも、作った台本を使わなければ移れる
- 反転条件: ST54 の上流が開発版の携帯の APK の作り方（識別子・鍵の形）を変えると決めたら、ST52 の道具をその形に直すか外す
- 台帳: `provisional.jsonl` の D23（source: deep.md#Q4。Q4・Q11 に依る）

## Risks / Trade-offs

- [Environment の配る対象の規則が、同じリポジトリの枝からの PR の実行に効くこと（P12）は公式文書だけ（確かさ 2）] → 本物の鍵を置く前に、使い捨ての値を Environment に置き、PR の実行から読めないことを確かめる（tasks 9.2）。読めたら鍵を置かず、Q2 の答えを人間に聞き直す
- [組み立ての job と `publish` の job の間で配布物を受け渡すので、組み立ての job を書き換えた PR が `publish` に別物を渡す] → `publish` は印の push と main の予定の実行でだけ走り、PR からは起きない。`publish` は受け取った配布物を `check-artifacts.sh` で見てから署名・公開する
- [下書きのリリースは書き手には見え、`gh release create --draft` が印を作るかは試していない（P16。確かさ 2）] → tasks 10.1 で下書きを作り、`git ls-remote --tags origin 'v*'` が増えないことを確かめてから確認バッチに出す。増えたら、その印を消す前に人間に返す（D2 のルールセットはまだ入っていない）
- [APK の `BuildConfig` の定数を dex から読む道具（`apkanalyzer dex code`）の出力の形が SDK で変わる] → `check-artifacts.sh --self-test` は本物の組み立てでできた APK（資格情報あり・なし）に当てる
- [ルールセットを読む API（`rules/tags/<印>`）を `GITHUB_TOKEN` で読めるか未確認] → D2 の反転条件。tasks 9.3 で確かめる
- [Windows でサーバを DB に繋いで画面を配るところは試していない（premises の触らなかったもの）] → ST52 の Windows の確かめは組み立てと起動の入口まで（ST55 が受ける）。画面が開くことは Linux の組み立てで e2e が見る

## Migration Plan

- 版の出所の切り替え（gradle・vite が `Cargo.toml` を読む）は 1 つの PR で入る。切り替えの前後で印の版番号は `0.1.0` のまま変わらない
- 配った印が 0 個なので、D1 の型の書き換えは今の main の検査結果を変えない
- 戻すとき: `release.yml`・`nightly.yml` を消し、`ci.yml` の `package` を外せば今に戻る。版の出所は手書きに戻さない（戻すと K3 に戻る）

## Open Questions

（無し。Q1〜Q11 の答えは `deep.md`。推奨と違った Q1・Q4 はこの版に写した。Q11 は推奨と同じ）
