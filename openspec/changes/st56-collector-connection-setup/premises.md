# ST56 の前提（外の形式・振る舞い）

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| Android の自動バックアップは、除外しない限りアプリの SharedPreferences を Google のクラウドへ写す（`allowBackup` の既定は true） | 携帯に置く資格情報を外へ出さない既定（C3：設定をバックアップと端末の移し替えから外す） | 公式文書を読む。manifest は `allowBackup` を書かず、`health_data_extraction_rules.xml` は `health_permission.xml` だけを外している | 外さなければ写る | 公式文書 https://developer.android.com/identity/data/autobackup ・ `collector-android/app/src/main/AndroidManifest.xml:74-75` | 2 |
| Windows の DPAPI（`CryptProtectData`、利用者の範囲）は、管理者の権限なしで、ログオンした同じ利用者だけが戻せる形に暗号化する | PC の資格情報の置き方（Q2 の推奨） | 公式文書を読む | 同じ利用者のログオンの中でだけ戻せる。同じ利用者として動くほかのプログラムは戻せる | 公式文書 https://learn.microsoft.com/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata | 2 |
| 配る形のサーバは画面と API を同じ口で出し、`/api` を剥がして API に渡す | 画面が示す設定の文字列に入れる送り先（画面を開いている口から作れる） | ST52 の archive の design D13 を読む | 同じ口。剥がした後の道は今の router と同じ | `openspec/changes/archive/2026-11-02-st52-release-channels/design.md:118-123` | 2 |
| 配る形のサーバの収集アプリの資格情報（`API_TOKEN`）は 43 文字の乱数で、インストーラが作り、画面に出さない | 手で打つ設定が成り立たない（Q1）。画面で示すと ST55 の「画面に出さない」と重なる | 走っている ST55 の design を読む | 43 文字の base64url。インストールした本人だけが読める設定のファイルにだけ置く | `openspec/changes/st55-server-installer/design.md:75` | 2 |
| 収集アプリは 401 を一時的な失敗として、未送信を 1 件も取り除かない | 「記録を捨てずに設定し直しを促す」の土台 | コードを読む | 携帯・PC とも取り除かない | `collector-android/.../Sender.kt:345-349`・`crates/collector-windows/src/sender.rs:254` | 2 |

失われるもの（`loss`）のある決定（Q7・Q8）は、外の形式に依らない。Q7 の推奨は「前の送り先が分からないまま未送信がある時も、押すまで送らない」で、自動バックアップと端末の移し替えが未送信（`filesDir`）を写すかどうかに依らずに守る（review/deep.md R6）。
