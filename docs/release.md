# リリース手順

本物の作業は ST53 が main に入った後に本人が行う。ST53 の確認バッチでは下書きだけを確認し、本物の印や正式な鍵は扱わない。

## 1. 版番号と準備

版番号の出所は `Cargo.toml` の workspace の `version` とし、`tools/release/version.sh source` で読む。コードを変更した PR では CI が配布物を組み立て、`tools/release/check-artifacts.sh` で確認する。

## 2. 鍵を作る

正式版と開発版の鍵は分ける。正式な鍵は安全な作業場所で `tools/release/make-key.sh official <keystore>` を使い、合言葉を引数・ログ・標準出力・チャットに書かない。控えと紙の合言葉は物理保管する。

## 3. エージェントの資格情報

本物の作業は ST53 が main に入った後に本人が行う。最初の印を打つ前に、ハーネスの token を fine-grained token に差し替える。持たせる権限は Contents の読み書き、Pull requests と Issues の読み書き、Workflows の書き、Actions の読みだけとし、Administration・Environments・Deployments・Secrets は持たせない。

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

§3 の資格情報と §4 の承認者を確認した後で、`release` Environment に正式な鍵を置く。`ASHIATO_SIGNING_KEYSTORE_B64`、`ASHIATO_SIGNING_STORE_PASSWORD`、`ASHIATO_SIGNING_KEY_ALIAS`、`ASHIATO_SIGNING_KEY_PASSWORD` を同じ名前で登録し、正式な証明書の SHA-256 を `ASHIATO_CERT_SHA256` に置く。keystore の値は `base64 -w0 <正式なkeystore>`（`-w` が無い環境では改行を除いた base64）から作る。証明書の指紋は `keytool -exportcert ... | sha256sum` で計算し、値は手順・ログ・チャットに出さない。`nightly` には正式な鍵、Secret、証明書指紋を置かない。

開発版の公開設定はリポジトリ変数 `ASHIATO_NIGHTLY_PUBLISH=on` とし、設定が無い間は公開しない。携帯の開発版鍵は ST54 から扱う。

## 6. 最初の印と開発版の公開

main の CI と配布物が緑になった後、本人が main の公開する commit に最初の印を打つ。正式版・ベータ版は印でソースを示し、それ以外は組み立てた commit で示す。

最初の印の最後に、印 `v<版>` を checkout した手元で実行する。

```bash
gh release download v<版> -D "$dir"
tools/release/check-artifacts.sh "$dir" --channel <beta|release> --expect-version <版>
```

PR の成果物は ST53 が main に入った後に本人が掃除する。ST53 より前の commit の `package-linux`・`release-package` だけを対象にし、ST53 後の成果物は消さない。

```bash
gh api --method DELETE repos/rai-256/ashiato/actions/artifacts/<artifact-id>
```

## 7. 壊れた版と修正の枝

壊れた正式版は削除・付け替えせず、次の版で修正する。修正の枝では移行と契約を元の正式版から変更せず、`tools/release/version.sh plan` で確認してから次の印を打つ。

## 8. ST53 の確認バッチ（人間の確認待ち）

PR の CI が緑になった後、その PR の **head の commit を checkout** した手元で `tools/release/draft.sh` を走らせる。配布物は head から組まれ、`draft.sh` は手元の木の版番号と `LICENSE` で比べる。本物の印・正式な鍵・本人の環境変更は行わない。

準備の間は本人の携帯を繋がない。APK はエミュレータにだけ入れ、`tools/verify-prep-device.sh --self-test`、許諾の検査、表示の検査を走らせる。確認後に下書きと不要な成果物を削除し、印が増えていないことを確認する。
