# リリース手順

ST52 で用意した配布手順。実際に配布元へ設定を入れたり、本物の印を打ったりするのは、ST53 の確認後に本人が行う。ST52 の確認では使い捨ての鍵と下書きのリリースを使い、本物の資格情報を扱わない。

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

## 3. GitHub の Environment と Secrets

ST53 の確認後、最初の正式版の印を打つ前に本人が設定する。

- `release` Environment は deployment branch policy を tag `v*` **だけ**にする。`main` には開かない（D21（仮））。
- `release` の Secret に、workflow が読む次の4つを同じ名前で置く: `ASHIATO_SIGNING_KEYSTORE_B64`（keystore を base64 にした値）、`ASHIATO_SIGNING_STORE_PASSWORD`、`ASHIATO_SIGNING_KEY_ALIAS`、`ASHIATO_SIGNING_KEY_PASSWORD`。値は手順やログに出さない。
- keystore は `base64 -w0 <正式なkeystore>` の出力を `ASHIATO_SIGNING_KEYSTORE_B64` に登録する（macOS など `-w` が無い環境では改行を除いた base64 を登録する）。
- `release` の変数 `ASHIATO_CERT_SHA256` に正式な証明書の SHA-256 指紋を置く。keystore と alias を用意した安全な端末で、`keytool -exportcert -keystore <正式なkeystore> -alias "$ASHIATO_SIGNING_KEY_ALIAS" -storepass:env ASHIATO_SIGNING_STORE_PASSWORD | sha256sum | cut -d' ' -f1` の出力（値そのものはログやチャットに貼らない）を登録する。workflow はこれを `ASHIATO_EXPECT_CERT_SHA256` として署名検査に渡す。
- `nightly` Environment は branch `main` **だけ**に開く。ST52 では鍵、Secret、`ASHIATO_CERT_SHA256` を置かない。携帯の開発版鍵を作って置くのは ST54 の作業とする。
- 開発版の公開設定はリポジトリ変数 `ASHIATO_NIGHTLY_PUBLISH=on` にする。設定が無い間は nightly の公開を行わない。

## 4. ルールセット

最初の印の前に、印 `v*` の削除と付け替え（update / non-fast-forward）を拒むルールセットを入れる（対象は tag、include に `refs/tags/v*`、状態は active）。`tools/release/check-guards.sh rules --repo <owner/name>`（`--repo` は前後どちらに置いてもよい）で削除と付け替えの両方を確認する。台本はルールセットの一覧と各ルールセットの詳細（`GET /repos/<owner>/<name>/rulesets` と `/rulesets/<id>`）を読むので、印を打つ前でも確かめられ、印の名前は要らない。ルールセットを確認できない場合は公開を進めない。

## 5. 最初の印と開発版の公開

1. main の CI が緑で、配布物の組み立てが完了していることを確認する。
2. main の公開する commit を checkout し、`git tag v<版番号> <commit>` で最初の印を main の commit に打つ。`tools/release/version.sh plan v<版番号>` が通ることを確認してから `git push origin v<版番号>` を行う。
3. release workflow が `check-guards.sh rules`、正式版の署名、`ASHIATO_CERT_SHA256` による指紋確認、`check-artifacts.sh`、GitHub Release 作成の順で進むことを確認する。
4. 開発版の公開を始めるときは、main の変更がある日に nightly workflow を動かし、`ASHIATO_NIGHTLY_PUBLISH=on` のときだけ公開する。nightly の公開先は main に開いた `nightly` Environment である。ST54 までは携帯の APK を含めず、S-01 と C-02 の配布物を公開する。
   最初の公開では workflow がリリース `nightly` を先行公開として作る（手で作らなくてよい）。公開は、新しい配布物を先に載せ（同じ名前は置き換え）、載った後で今回の配布物に無い名前（前日の版の配布物）だけを消し、最後に印 `nightly` を動かす。載せるのに失敗したときは前の配布物が残る。

## 6. 壊れた版と修正の枝

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

## 7. 本人の環境を移す

ST53 の確認後、正式版の公開が完了してから本人が行う。まず現在のサーバを止め、DB のバックアップを取り、新しい正式版のサーバを配置する。新しい版で `ashiato-server migrate` を実行し、成功した場合だけ新しいサーバを起動する。DB が配布版より先に進んでいる場合は書き換えず、次の版まで移さない。起動後に画面、取り込み、稼働状況を確認し、異常があれば旧版へ戻す手順（ST48 の終了コード3は復元しない）に従う。

## 8. ST52 の確認バッチ（人間の確認待ち）

PR の CI が緑になった後、使い捨ての値と下書きで次を本人が確認する。本物の印、正式な鍵、本人のサーバの切り替えは行わない。

```bash
before="$(git ls-remote --tags origin 'v*' | wc -l)"
tools/release/draft.sh <run-id> 0.1.0-beta.1
# 下書きの配布物と先行公開の印を確認し、確認後に下書きを削除する
after="$(git ls-remote --tags origin 'v*' | wc -l)"
test "$before" = "$after"
tools/release/check-guards.sh rules --repo <owner/name>
```

`draft.sh` は PR の `package` の run（`<run-id>`）の配布物を取り寄せ、渡した版番号の名前（`v0.1.0-beta.1`）で下書きを作る。版番号が `-beta.` を含むので先行公開の下書きになる。PR の組み立ては出所の版（`tools/release/version.sh source`）で組むので、配布物の検査は出所の版と比べ、下書きの本文にその旨を書く。渡す版番号の `MAJOR.MINOR.PATCH` は出所の版と同じにする。印は下書きを作る前（`before=`）に数え、作った後と比べる。

使い捨ての Environment で、PR の枝から `release` 相当の Secret が読めないことも確認してから Environment を削除する。読めた場合は鍵を置かず、Q2 の判断へ戻す。
