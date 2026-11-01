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
- `release` の Secret に正式な keystore と合言葉を置く。Environment の値の名前だけを手順やログに出し、値は出さない。
- `release` の変数 `ASHIATO_CERT_SHA256` に正式な証明書の SHA-256 指紋を置く。workflow はこれを `ASHIATO_EXPECT_CERT_SHA256` として署名検査に渡す。
- `nightly` Environment は branch `main` **だけ**に開く。ST52 では鍵、Secret、`ASHIATO_CERT_SHA256` を置かない。携帯の開発版鍵を作って置くのは ST54 の作業とする。
- 開発版の公開設定はリポジトリ変数 `ASHIATO_NIGHTLY_PUBLISH=on` にする。設定が無い間は nightly の公開を行わない。

## 4. ルールセット

最初の印の前に、印 `v*` の削除と付け替え（update / non-fast-forward）を拒むルールセットを入れる。`tools/release/check-guards.sh rules --repo <owner/name>` で削除と付け替えの両方を確認する。ルールセットを確認できない場合は公開を進めない。

## 5. 最初の印と開発版の公開

1. main の CI が緑で、配布物の組み立てが完了していることを確認する。
2. main の公開する commit を checkout し、`git tag v<版番号> <commit>` で最初の印を main の commit に打つ。`tools/release/version.sh plan v<版番号>` が通ることを確認してから `git push origin v<版番号>` を行う。
3. release workflow が `check-guards.sh rules`、正式版の署名、`ASHIATO_CERT_SHA256` による指紋確認、`check-artifacts.sh`、GitHub Release 作成の順で進むことを確認する。
4. 開発版の公開を始めるときは、main の変更がある日に nightly workflow を動かし、`ASHIATO_NIGHTLY_PUBLISH=on` のときだけ公開する。nightly の公開先は main に開いた `nightly` Environment である。ST54 までは携帯の APK を含めず、S-01 と C-02 の配布物を公開する。

## 6. 壊れた版と修正の枝

壊れた正式版は削除・付け替えせず、次の版で修正する。緊急修正が必要なときは正式版から `release/<MAJOR>.<MINOR>` の修正の枝を切り、移行 (`migrations/ORDER`) と封筒 (`docs/contract/envelope.json`) を元の正式版から変更しない。`tools/release/version.sh plan` でその条件を確認してから正式版の印を打つ。修正の枝からベータ版は出さない。

修正の枝の例:

```bash
git switch -c release/0.1 <正式版の印>
# 修正、テスト、commit
tools/release/version.sh plan v0.1.1
git tag v0.1.1
git push origin v0.1.1
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
tools/release/draft.sh <run-id> 0.1.0-beta.1
before="$(git ls-remote --tags origin 'v*' | wc -l)"
# 下書きの配布物と先行公開の印を確認し、確認後に下書きを削除する
after="$(git ls-remote --tags origin 'v*' | wc -l)"
test "$before" = "$after"
tools/release/check-guards.sh rules --repo <owner/name>
```

使い捨ての Environment で、PR の枝から `release` 相当の Secret が読めないことも確認してから Environment を削除する。読めた場合は鍵を置かず、Q2 の判断へ戻す。
