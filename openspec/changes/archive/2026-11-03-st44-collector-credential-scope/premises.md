# ST44 の前提

外のサービスや OS の書き出しの形式に依る決定はない。依るのは、道具（curl・Playwright）がログインの印をどう付けるかだけ（Q1 案 1 の費用。どの案でも失うものは無い）。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| 道具はログインの印（`Secure` の cookie）を `http://127.0.0.1` のサーバへ付けて読める | Q1 案 1 の、道具をログインで読む形へ直す費用 | 独立レビュー（review/deep.md R4）が curl 8.18.0 と小さな http サーバで、`Set-Cookie: s=abc; Secure` の次の求めに cookie が付くのを見た | 付いた（curl） | 実物 | 4 |
| Playwright の `request` は `Secure` の印を http の求めに付けないことがある | 同上（e2e） | `web/e2e/feel.spec.ts:16` の注記（「画面の外の要求には付かない」） | 付かない（注記） | 既存のテスト材料 | 2 |
| 付かなくても、道具は `Set-Cookie` から値を取り、`cookie:` の見出しとして自分で付ければ読める（サーバは見出しの値だけを見る） | 同上 | `crates/server/src/web_session.rs` の `session_valid` が求めの見出しを読むところまで辿る | 成り立つ | repo のコード | 3 |

失うもの（`loss`）のある決定はこれらの前提に載せない（どれも費用だけに効く）。

問いの context に書いた repo の中の事実（外の形式ではない。確かめた場所だけを控える）:

| 事実 | 確かめた場所 |
|---|---|
| 資格情報の判定は `authorize_quiet` が 1 か所で行い、`API_TOKEN` に一致すれば `Caller::ApiToken`（全部の口が通る）。Bearer をログインの印より先に見る | `crates/server/src/lib.rs` の `authorize_quiet` / `authorize` |
| `API_TOKEN` はログインの印を無効にする鍵（`secret_tag`）にも使っている | `crates/server/src/web_session.rs` の `secret_tag` |
| 携帯の収集アプリが呼ぶ口: `/ingest` `/heartbeat` `/drops` `/media/missing` `/media/originals/{sha256}`（PUT・HEAD）`/contract` `/outages` `/feelings/reminder`（GET） | `collector-android/app/src/main/kotlin/dev/ashiato/collector/`（`LocationService.kt` `OriginalAndroid.kt` `FeelingReminderSource.kt`） |
| PC の収集アプリが呼ぶ口: `/ingest` `/heartbeat` `/contract` `/outages` と、資格情報の要らない `/healthz` | `crates/collector-windows/src/sender.rs` `runtime.rs` `clock.rs` |
| 画面は記録を `/ingest` で送る（気分・場所） | `web/src/*.ts*` の `/api/ingest` |
| 道具と e2e が `API_TOKEN` で記録を読んでいる | `tools/smoke.sh` と `smoke.d/`（48 か所）・`web/e2e/*.spec.ts`（8 ファイル）・`tools/recording`（5 ファイル）・`tools/verify-prep.sh`・`tools/st25-seed-check.sh` |
| 携帯の APK は `API_TOKEN` を `BuildConfig` に焼き込む | `collector-android/app/build.gradle.kts` の `buildConfigField("String", "API_TOKEN", …)` |
