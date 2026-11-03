# リリース手順

本物の作業は ST53 が main に入った後に本人が行う。ST53 の確認バッチでは下書きだけを確認し、本物の印や正式な鍵は扱わない。

## 1. 版番号と準備

版番号の出所は `Cargo.toml` の workspace の `version` だけとする。`tools/release/version.sh source` で読み、他のファイルに版番号を手書きしない。ベータ版は `v<出所>-beta.N` の印、開発版は出所に日付を足した `<出所>-dev.YYYYMMDD` とする。

コードを変更した PR では、CI の `package` job が同じ版番号の Linux / Windows のサーバ、PC の収集アプリ、携帯の APK を組み立てる。配布物は `tools/release/check-artifacts.sh` で確認し、1つでも欠けたら公開しない。

正式版を公開した後は、次の開発を始める前に `Cargo.toml` の workspace の `version` を次の版へ上げて commit する。これにより、開発版が直前に公開した正式版より古く見えないようにする。

## 2. 鍵を作る

### 正式版

1. 安全な作業場所で、合言葉を標準入力から渡して `tools/release/make-key.sh official <keystore>` を実行する。合言葉をコマンド引数、ログ、標準出力、チャットに書かない。
2. `printf '%s\n' "$PASSWORD" | tools/release/make-key.sh verify <keystore>` で RSA 4096 bit かつ25年以上の有効期限を確かめる。合言葉の値は出力しない。
3. 正式な鍵の控えを、扉34のとおり USB に 1 部、拠点外の保管先に 1 部置き、紙の合言葉を物理保管する。GitHub Environment の値は取り出せないため、控えにはならない。

### 開発版

開発版の鍵は正式な鍵と別にする。開発版に携帯の APK を載せ始めるのは ST54 からなので、ST52 では作らず、`nightly` にも置かない。ST54 で `tools/release/make-key.sh dev <keystore>` を実行し、控えを USB に 1 部置く。

## 3. エージェントの資格情報

本物の作業は ST53 が main に入った後に本人が行う。最初の印を打つ前に、ハーネスの token を fine-grained token に差し替える。持たせる権限は Contents の読み書き、Pull requests と Issues の読み書き、Workflows の書き、Actions の読みだけとし、Administration・Environments・Deployments・Secrets は持たせない。

**正式な鍵を `release` に置く前に、ST53 で「受け入れて進める（リリースの前に判断し直す）」とした IMP-01・IMP-02 を判断し直す**（`python3 scripts/imp_ledger.py risks .` に出る。印を push できる人が main を通さずに鍵を読む経路と、本人の管理者の資格情報がエージェントと同じ OS の利用者に残る件。鍵は一度読まれると取り返せないので、期限に依らず、鍵を置く前にここで必ず判断する）。

差し替えた資格情報で、管理権限が無いことを確認する。

```bash
tools/release/check-guards.sh agent-credential --repo rai-256/ashiato
```

このコマンドが `rc=0` でなければ鍵を置かない。ハーネスの `gh pr merge --merge` と PR 本文の PATCH は管理権限を使わない。workflow の中の確かめは、印の commit の workflow を変更できれば守りの本体にならない。

## 4. ルールセットと承認者

最初の印の前に、印 `v*` の削除と付け替えを拒む ST52 のルールセットを入れる。`release` Environment は tag `v*` だけを対象にし、本人を必須の承認者として付ける。管理者の飛ばしを外し、自分の承認を禁じない。`nightly` Environment は branch `main` だけを対象にする。

```bash
tools/release/check-guards.sh rules --repo rai-256/ashiato
tools/release/check-guards.sh approvals --repo rai-256/ashiato
```

`check-guards.sh approvals` が `rc=0` でなければ公開を止める。承認するときは、印の commit が main の祖先で、PR を通って入ったことを見る。workflow の中の `check-guards.sh approvals` は守りの本体ではなく、印の commit の workflow で走る補助の止めである。

## 5. 鍵と Secrets

§3 の資格情報と §4 の承認者を確認した後で、`release` Environment に正式な鍵を置く。`ASHIATO_SIGNING_KEYSTORE_B64`、`ASHIATO_SIGNING_STORE_PASSWORD`、`ASHIATO_SIGNING_KEY_ALIAS`、`ASHIATO_SIGNING_KEY_PASSWORD` を同じ名前で登録し、正式な証明書の SHA-256 を `ASHIATO_CERT_SHA256` に置く。keystore の値は `base64 -w0 <正式なkeystore>`（`-w` が無い環境では改行を除いた base64）から作る。

証明書の指紋は、次の完全なコマンドで計算し、値は手順・ログ・チャットに出さない。

```bash
keytool -exportcert -keystore <正式なkeystore> -alias "$ASHIATO_SIGNING_KEY_ALIAS" -storepass:env ASHIATO_SIGNING_STORE_PASSWORD | sha256sum | cut -d' ' -f1
```

`nightly` には正式な鍵、Secret、証明書指紋を置かない。携帯の開発版鍵を作って置くのは ST54 の作業とする（`sign-apk.sh … dev` は開発版の鍵の指紋を `ASHIATO_EXPECT_DEV_CERT_SHA256` で受け取り、無ければ署名しない）。

開発版の公開設定はリポジトリ変数 `ASHIATO_NIGHTLY_PUBLISH=on` とし、設定が無い間は公開しない。

## 6. 最初の印と開発版の公開

§3 の資格情報・§4 のルールセットと承認者・§5 の鍵を確認した後で、本人が次の順に進める。正式版・ベータ版は印でソースを示し、それ以外は組み立てた commit で示す。

1. main の CI が緑で、配布物の組み立てが完了していることを確認する。
2. main の公開する commit を checkout し、`git tag v<版番号> <commit>` で最初の印を main の commit に打つ。`tools/release/version.sh plan v<版番号>` が通ることを確認してから `git push origin v<版番号>` を行う。
3. release workflow が `check-guards.sh rules`、`check-guards.sh approvals`、正式版の署名、`ASHIATO_CERT_SHA256` による指紋確認、`check-artifacts.sh`、GitHub Release 作成の順で進むことを確認する。
4. 公開された配布物を、印 `v<版番号>` を checkout した手元で検査する。

   ```bash
   dir="$(mktemp -d)"
   gh release download v<版番号> -D "$dir"
   tools/release/check-artifacts.sh "$dir" --channel <beta|release> --expect-version <版番号>
   ```

5. 開発版の公開を始めるときは、main の変更がある日に nightly workflow を動かし、`ASHIATO_NIGHTLY_PUBLISH=on` のときだけ公開する。nightly の公開先は main に開いた `nightly` Environment である。ST54 までは携帯の APK を含めず、S-01 と C-02 の配布物を公開する。
   最初の公開では workflow がリリース `nightly` を先行公開として作る（手で作らなくてよい）。公開は、新しい配布物を先に載せ（同じ名前は置き換え）、載った後で今回の配布物に無い名前（前日の版の配布物）だけを消し、最後に印 `nightly` を動かす。載せるのに失敗したときは前の配布物が残る。

PR の成果物は ST53 が main に入った後に本人が掃除する。ST53 より前の commit の `package-linux`・`release-package` だけを対象にし、ST53 後の成果物は消さない。

```bash
gh api --method DELETE repos/rai-256/ashiato/actions/artifacts/<artifact-id>
```

## 7. 本人の環境を移す

★ 2026-11-03 訂正（`docs/requirements.md` FR-108 の ★ 2026-11-03 補足・整合の確認 `docs/audit/2026-11-03-native-postgres-consistency.md` の C3）。
**本人の環境を切り替えるのは ST55（同梱の PostgreSQL のインストーラ）が済んでから、そのインストーラで入れて行う。** ST53 の後に Docker の DB のまま切り替えない ——
Docker の DB の記録は配る形へ移さないと決めた（FR-108。移す口が無い）。以下は旧い手順（2026-11-02）で、ST55 がインストーラの手順に書き直す。

ST53 が main に入り、正式版の公開が完了してから本人が行う。まず現在のサーバを止め、DB のバックアップを取り、新しい正式版のサーバを配置する。新しい版で `ashiato-server migrate` を実行し、成功した場合だけ新しいサーバを起動する。DB が配布版より先に進んでいる場合は書き換えず、次の版まで移さない。起動後に画面、取り込み、稼働状況を確認し、異常があれば旧版へ戻す手順（ST48 の終了コード3は復元しない）に従う。

## 8. 壊れた版と修正の枝

壊れた正式版は削除・付け替えせず、次の版で修正する。緊急修正が必要なときは正式版から `release/<MAJOR>.<MINOR>` の修正の枝を切り、移行 (`migrations/ORDER`) と封筒 (`docs/contract/envelope.json`) を元の正式版から変更しない。`tools/release/version.sh plan` でその条件を確認してから正式版の印を打つ。修正の枝からベータ版は出さない。

修正の枝の例:

```bash
git switch -c release/0.1 <正式版の印>
# Cargo.toml の workspace version を 0.1.1 に上げて commit
# （版番号を上げた commit を修正の枝に先に置く）
<エディタで Cargo.toml の workspace version を 0.1.1 に変更>
git add Cargo.toml
git commit -m "chore: 次の版番号へ上げる"
# 修正、テスト、commit
git add <修正したファイル>
git commit -m "fix: 正式版の不具合を修正"
git push -u origin release/0.1
git tag v0.1.1
tools/release/version.sh plan v0.1.1
git push origin v0.1.1
# release workflow の完了と配布物を確認してから枝を消す
run_id="$(gh run list --workflow release.yml --limit 1 --json databaseId --jq '.[0].databaseId')"
gh run watch "$run_id" --exit-status
git switch main
git branch -d release/0.1
git push origin --delete release/0.1
```

実際の版番号と commit は対象のリリースに置き換える。削除前に修正が正式版へ取り込まれていることを確認し、印そのものは削除しない。

## 9. ST53 の確認バッチ（人間の確認待ち）

PR の CI が緑になった後、使い捨ての値と下書きで次を本人が確認する。本物の印、正式な鍵、本人のサーバの切り替えは行わない。**準備の間は本人の携帯を繋がない。**

### 11.1 下書きのリリースで許諾の 3 点を見る

この Story の PR の `package` の run で、`tools/release/draft.sh <run-id> 0.1.0-beta.1` を、その PR の **head の commit を checkout** した手元（`git rev-parse HEAD` が run の `head_sha` と同じ）で走らせる。作る前と後で印の数が同じことを見る。

```bash
before="$(git ls-remote --tags origin 'v*' | wc -l)"
tools/release/draft.sh <run-id> 0.1.0-beta.1
after="$(git ls-remote --tags origin 'v*' | wc -l)"
test "$before" = "$after"
```

4 つの配布物を落として、tar / zip の根に `LICENSE`・`THIRD-PARTY-NOTICES.txt`・`SOURCE.txt` の 3 点があること、APK は `unzip -l` で `assets/licenses/` に 3 点があることを見る。`SOURCE.txt` が `Commit:` を持つこと、`gh api repos/rai-256/ashiato/actions/runs/<run-id> --jq .head_sha` の値と配布物の組み立ての印の commit が同じで `.dirty` が無いことを見る。見た後に下書きと不要な成果物を削除する。

### 11.2 承認者が job を止めることと、権限を絞った token が承認できないこと

使い捨ての Environment `st53-probe` に本人を必須の承認者として付け、使い捨ての Secret を置く。main に無い枝の commit に、その Environment を使う workflow を足して 1 回だけ走らせ、承認するまで job が始まらず Secret が読まれないことを見る。さらに §3 の権限で作った token で、保留中の実行の承認（`gh api -X POST repos/rai-256/ashiato/actions/runs/<id>/pending_deployments`）を試み、断られることを見る。最後に Environment と枝と token を消す。

### 11.3 公開の job から承認者の設定を読めるか

11.2 の workflow の中で `tools/release/check-guards.sh approvals --repo rai-256/ashiato --reviewer <本人>`（`GITHUB_TOKEN`）を走らせ、`st53-probe` を `release` の代わりに読ませて rc を見る。読めなければ D11（仮）の反転条件で上流へ返し、結果を P15 として書く。

### 11.4 GitHub の許諾の表示

この Story の枝の `LICENSE` を GitHub の画面で開き、許諾の判定の表示（AGPL-3.0 か Other か）を見る。Other でも止めない（README が名指しする）。結果を P16 として書く。

### 11.5 PR の head を後から取れるか

使い捨ての枝に 1 commit を足して PR を開き、`git rev-parse HEAD` で head を控える。枝を rebase して force-push し直し、新しい head も控える。PR を merge せずに閉じ、別の空の clone で控えた 2 つ（push し直す前の head と最後の head）を `git fetch origin <commit>` で取れるかを見る。最後に枝を消す。取れれば P14 の確かさを 4 にし、取れなければ D9 の反転条件で上流へ返す。

使い捨ての Environment で、PR の枝から `release` 相当の Secret が読めないことも確認してから Environment を削除する。11.2 で承認できた・承認の前に job が始まった・Secret が読めた場合は、鍵を置かず、ST53 の深掘りの Q3（正式な鍵を置く前に、印から鍵を読める経路をどう閉じるか）を人間に聞き直す。
