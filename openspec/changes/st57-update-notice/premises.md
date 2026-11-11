# ST57 の前提（外の形式・振る舞い）

問いを立てる前に確かめたもの。確かさは story-upstream の 1〜5（1 言っただけ / 2 出所を示した / 3 辿った / 4 走らせた / 5 実物で再現した）。
2026-11-11 に確かめた。

**失われるもの（`loss`）のある問いは Q1 の 1 つ（`exported`: 問い合わせの元の IP と時刻）。** 問い合わせの中身は FR-116 が「版の一覧の問い合わせだけ」に固定しており、
問い合わせの形は P1〜P3（確かさ 4〜5）に載せた。

| # | 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|---|
| P1 | 配布元（GitHub Releases、`docs/production-prep.md` の既定）のリポジトリは公開されていて、認証なしでリリースの一覧を取れる | FR-112 の問い合わせが認証なしで成り立つ | `gh repo view --json visibility` と、認証なしの `curl https://api.github.com/repos/rai-256/ashiato/releases`（見出し `User-Agent: ashiato-update-check`） | `PUBLIC`。HTTP 200、本文は `[]`（まだリリースが 1 つも無い） | 実物 | 5 |
| P2 | 認証なしの問い合わせの上限は IP ごとに 1 時間 60 回 | FR-112（1 日 1 回なら当たらない。EXT-P） | P1 の応答の見出し | `x-ratelimit-limit: 60`・`x-ratelimit-resource: core` | 実物 | 5 |
| P3 | GitHub の API は `User-Agent` の見出しの無い問い合わせを断る | FR-116（送るものを「版の一覧の問い合わせ」に絞っても、見出しを 1 つは送らざるを得ない） | 空の `User-Agent` で P1 と同じ問い合わせ | HTTP 403「Please make sure your request has a User-Agent header」 | 実物 | 5 |
| P4 | リリースの一覧の各項目は `tag_name`・`prerelease`・`draft`・`assets[].name` を持つ | 版とチャンネルの読み取り（正式版 = 先行公開でない、ベータ版 = 先行公開の `v*-beta.N`、開発版 = 印 `nightly` の配布物の名前の版） | 公開のリポジトリ（`cli/cli`）に認証なしで同じ問い合わせ | 4 つの欄がある。配布物の名前が並ぶ | 実物（別のリポジトリ） | 4 |
| P5 | 下書きのリリースは認証なしの一覧に出ない | 手で起こした確かめの組み立て（下書き。正典 distribution）を「更新あり」と取り違えない | 公式文書 https://docs.github.com/en/rest/releases/releases#list-releases（"Only users with push access will receive listings for draft releases."） | 出ない | 公式文書 | 2 |
| P6 | 開発版の版番号は印 `nightly` のリリースの配布物の名前からしか読めない（印の名前は版を持たない） | 開発版の「新しい版」の判定 | 正典 `openspec/specs/distribution/spec.md` の「印（タグ）の名前で、3 つのチャンネルに分けて配り…」（配布物の名前は版をまたいで変えない・後の Story が名前を読む）と `tools/release/publish-nightly.sh:31-42`（初回だけ作り、以後は配布物を入れ替える） | 名前は `ashiato-server-<版>-windows-x86_64.zip` などで固定 | 実物（repo） | 3 |
| P7 | 開発版のリリースは一度作ったら作り直さないので、一覧の中の位置が古いまま沈みうる | 一覧を 1 ページ（100 件）だけ取る形で開発版を見落とさないか | `tools/release/publish-nightly.sh:31-35`（無いときだけ `gh release create`）。一覧の並びの規則は確かめていない | 作り直さない。並びは未確認 | 実物（repo）/ 知識 | 2 |
| P8 | PC の収集アプリは、いま暗号化した接続（TLS）の部品を持たない | PC の収集アプリが配布元（https）に直に聞くなら、TLS の部品と証明書の根を足す（依存の許諾・脆弱性の検査に載る） | `crates/collector-windows/Cargo.toml:21`（`ureq = { version = "3", default-features = false }`） | TLS の機能を有効にしていない | 実物（repo） | 2 |
| P9 | 携帯の収集アプリの通信の設定は、平文を loopback（と組み立てに使った接続先）だけに許し、https の宛先は制限しない | 携帯が配布元に https で聞けるか | `collector-android/app/build.gradle.kts:288-293`（`base-config cleartextTrafficPermitted="false"` を生成）・`AndroidManifest.xml:11`（`INTERNET`） | https は許される | 実物（repo） | 2 |
| P10 | Android の `HttpURLConnection` は既定の `User-Agent` に端末の機種名と OS の版を入れる（`http.agent`） | FR-116（見出しを固定しないと、版の一覧の問い合わせ以外の情報が出る） | 知識。下流の試験（届いた見出しを数える）で 4 に上げる | 見出しは固定する形にする（どの答えでも同じ。決定は載せない） | 知識 | 1 |
| P11 | サーバの部品の検査（`tools/check-offsite.sh`）は `cargo tree -p ashiato-server` だけを見る | 更新係を別の crate にすれば、サーバの守りを開けずに問い合わせの部品を持てる | `tools/check-offsite.sh:47` | `ashiato-server` の依存の木だけを見る | 実物（repo） | 2 |
| P12 | 状態の置き場（FR-109）は 2 つのサービスのアカウントとインストールした本人だけが読める。更新係のアカウントはそこを読めない | 更新係の設定・結果の置き場を FR-109 の外に別に作る | 正典 distribution の「状態の置き場は、2 つのサービスのアカウントだけが書け…」・`crates/installer/src/layout.rs`（申し送り ST55 D14） | 別の置き場が要る | 実物（repo） | 2 |
