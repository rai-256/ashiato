# 網の手順書（ST28）

本システムは**許可した私設網の内側からしか届かない**。サーバと画面は 127.0.0.1 で待ち受け、網の外へは
`tailscale serve` の HTTPS だけが出す（Q2 / design D12）。網の名前は雛形（`<machine>.<tailnet>.ts.net`）で書く。実値は `.env` と手元にだけ置く。

## 1. 口を出す（`tailscale serve` の HTTPS）

画面と API の 2 本。サーバは `BIND`（~~既定~~ `.env.example` の値 `127.0.0.1:18787`）、画面は `WEB_PORT`（既定 `5180`）。

> ★ 2026-10-11 訂正（2026-10-11 の整合の確認）。`18787` は**雛形（`.env.example`）の値**で、コードの既定ではない。
> `BIND` を設定しないとサーバは `127.0.0.1:8787` で待ち受ける（`crates/server/src/net_guard.rs` の `DEFAULT_BIND`）。
> C-02 の起動例（`crates/collector-windows/README.md`）も `8787` を指すので、`.env` どおり 18787 で立てたサーバに
> README どおりの C-02 は繋がらない。**この食い違いは fix で直す（コードの既定と雛形のどちらに揃えるかは fix で決める）。**
> この手順書は `.env` から立てる前提なので、下の口は 18787 のまま。

```bash
tailscale serve --bg --https=18787 http://127.0.0.1:18787   # API
tailscale serve --bg --https=5180  http://127.0.0.1:5180    # 画面
```

- 既存の http の口は消す: `tailscale serve --http=18787 off` / `tailscale serve --http=5180 off`
- 開発用の画面（`DEV_WEB_PORT`、既定 5173）の口は網へ出さない。出していたら消す: `tailscale serve --http=5173 off` / `--https=5173 off`
- 接続先は**ホスト名**（`https://<machine>.<tailnet>.ts.net:<port>`）。IP 宛は `tailscale serve` が 404 を返す
- 出したあとは `tools/check-exposure.sh` で確かめる（`ss` の待ち受けが loopback だけ・網の外への公開が無い・平文の口が無い・開発用の口が無い）。落ちても自動では直さない

## 2. 証明書ログに名前が載る（消せない）

HTTPS を有効にすると、**機械の名前と網の名前が公開の証明書ログ（Certificate Transparency）に載る。一度載ると消せない**（Q2。本人は受け入れた）。
配布先の利用者が手順書どおりにすると、その人の機械名と網の名前も載る。**利用者に先に見せて、了承を得てから有効にする。**

## 3. 収集アプリの入れ直し

`~/.gradle/gradle.properties` の `ashiato.baseUrl` を `https://<machine>.<tailnet>.ts.net:18787` にして APK を入れ直す
（収集アプリの平文の例外は外してある。`http://` の loopback 以外は組み立てが落ちる）。
C-02 の `ASHIATO_BASE_URL` は loopback のままでよい。

## 4. 電話を落としたとき

収集側の合言葉（`API_TOKEN`）は ST29 まで全読みのまま（Q4）。★ 2026-10-11（2026-10-11 の整合の確認）: 要件の改訂で ST29 の範囲が変わる（資格情報は衛星ごと。PERM-10★ / PERM-14）。中身はそのまま有効。収集アプリは衛星ではないので（要件 §1.3）、宛先の「ST29」をどの Story に付け替えるかは `/stories` で決める。落とした電話・古い APK の合言葉で読まれた記録は戻らない。次の順で締め出す:

1. 網（管理画面）から**その電話のノードを外す**
2. `.env` の `API_TOKEN` を変えて、APK と C-02 を入れ直す
3. `.env` の `WEB_PASSWORD`（画面の合言葉）を変えて、サーバを起動し直す

ログインは期限で切れない（Q6）。**画面の合言葉を変えると、すべての端末のログインが無効になる**。`API_TOKEN` を変えた時点でも全端末のログインは切れる（design D3）。
端末を落としたときの締め出しの手段は、期限の代わりにこれだけ。

## 5. 画面を出す機械名に信頼できないサービスを同居させない

ログインの印（cookie）は port で分かれない（RFC 6265 §8.5）。**同じ機械名の別の口（操作盤など）にも送られる。**
画面を出す機械名に、信頼できないサービスを同居させない（同居させると、その口の持ち主にログインの印が渡る）。

## 6. 本人の機械での移行（design D20。merge の後に本人が行う）

実データの入った DB に触るので、**退避を先に**。この順で行う:

1. `pg_dump` で退避する（`tools/db-roles.sh` は所有を移すだけで行を変えないが、実データの DB に対する初めての操作なので）
2. `.env` に 4 つの秘密（`POSTGRES_PASSWORD` / `OWNER_DB_PASSWORD` / `APP_DB_PASSWORD` / `WEB_PASSWORD`。`openssl rand -hex 24`）を足し、
   `DATABASE_URL` をアプリの役割に、`DATABASE_OWNER_URL` を足す（`.env.example` を見る）
3. `tools/db-roles.sh` → `ashiato-server migrate` → サーバ
4. `tailscale serve` の口を https に置き換え、http と開発用の画面の口を消す（§1）→ `tools/check-exposure.sh`
5. `~/.gradle/gradle.properties` の `ashiato.baseUrl` を `https://` にして APK を入れ直す（§3）

確認バッチ（`tools/verify-prep.sh`）は、`ashiato.baseUrl` が平文で loopback でなければ APK を作らずに「移行の 5 を済ませる」と出す。
