# 整合の確認（2026-10-25・ST49 の深掘り第 1 回の答えを FR-122 / FR-123 と扉 32 へ戻した改訂）

- 基準: `814761dec02ac05b32def528ca326f968793f108`..`67a2b3fa1412be0734be3c5dcc21980c348b3e97`

対象は commit 67a2b3fa が足した 3 つの ★（`docs/requirements.md:1020-1021` FR-122 ★ 2026-10-25 追加「版は版番号・commit・未 commit の変更の印・組み立ての種類を持つ」「版を持つ『記録』には、`core.event` の外で収集アプリが作る追記のみの行（破棄の報告・原本の欠け）も含める」、
`:1026-1028` FR-123 ★ 2026-10-25 訂正「S-01 の版は、受け取って D-01 に書いたすべての記録に持たせる」「衛星の記録には衛星自身の版を付けない」「V-01 から本人が書く記録は…画面の組み立ての版も持つ」、
`:1724` 扉 32 の ★ 2026-10-25 訂正）と、`docs/stories/stories.json` の ST49 の完了の判定 2 行・`docs/stories/ST49.md` の貼り直し。文書は head（`67a2b3fa`）の内容で読んだ。

機械の入力:
- `req_impact.py --base 814761de`: 改訂 2 件（FR-122・FR-123）・扉 1 項（#32）。名指しは `docs/stories/INDEX.md:72,468`・`docs/stories/stories.json:1155,1156` と、走っている `st49-record-component-version` の文書（deep・design・premises・proposal・review・specs 8 capability・go.json）だけ
- `check_chain.py`: 要件 171 / Story 58 / 扉 35・拾われた要件 169/171・未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった: FR-122, FR-123」（この監査が受けるもの）

走っている change のうち、この改訂を受けるのは `st49-record-component-version`（出どころ）と、ST49 の版を持たせる先の表を足す `st23-record-erasure`。どちらへの指摘も**印を付けるだけ**にした（C4・C6）。
失われるもの: C4 は uncaptured（版を持たずに書いた行には後から版を足せない —— 扉 32 の理由そのもの）。ほかの印に失われるものは無い。

## C1. FR-123 の SHALL の本文が改訂前の範囲（C-03・派生・V-01）のままで、同じ項の ★ が「すべての記録」へ広げている
- 行き先: requirements（FR-123 の SHALL の本文を書き換え、★ 2026-10-25 訂正に従来の文を残した）
- 重大度: 中
- 文書: docs/requirements.md:1023
- 要件: FR-123
- 根拠: 本文（:1023）「THE SYSTEM SHALL **C-03 と派生が書く記録と、V-01 から本人が書く記録**の 1 件ごとに、書いた時点の S-01 の版を持たせる」、直後の ★ Q13（:1024）「衛星の記録は対象にしない」。
  同じ項の ★ 2026-10-25 訂正（:1026）「**S-01 の版は、受け取って D-01 に書いたすべての記録に持たせる**（C-01 / C-02 の記録・衛星の記録・…）」。本文と ★ Q13 の 2 か所が、最後の ★ と反対のことを言ったまま残る。
  同じ日の FR-122 の C7 訂正は本文そのものを書き換えた（:1015「その記録を**作った時の**収集アプリの版」、★ :1019「従来は『送った収集アプリの版』」）。出どころの design も本文の改訂を申し送っていた（`openspec/changes/st49-record-component-version/design.md:125`「FR-123 の**本文**（S-01 の版を持つ記録の範囲）と Story の完了の判定…を ★ で改訂する」）。
  `docs/stories/ST49.md:28` は本文を逐語で写し、最後の ★ の終わりの行（「収集アプリの版だけでは…」）は「…（続きは requirements.md）」で切れる。SHALL の行だけを読む席（Story の切り出し・satisfies の判定・衛星の Story の上流）は、衛星と収集アプリの記録に S-01 の版が要らないと読む
- 直すところ: requirements（FR-123 の SHALL の本文を「受け取って D-01 に書いたすべての記録に S-01 の版を、V-01 から本人が書く記録には画面の組み立ての版も」に書き換え、★ 2026-10-25 に従来の文を残す）
- 機械: できない（★ が本文の範囲を変えたかは散文）

## C2. 扉 32 の決着状態の本文に「衛星の記録は対象にしない」が残り、直下の ★ が反対を言う
- 行き先: requirements（扉 32 の決着状態の本文を書き換え、★ に従来の文を残した）
- 重大度: 中
- 文書: docs/requirements.md:1723
- 要件: FR-122, FR-123
- 根拠: 決着状態（:1723）「持つ —— C-01 / C-02 の記録は作った時の収集アプリの版、**C-03・派生・V-01 から本人が書く記録は S-01 の版。衛星の記録は対象にしない**」、直下の ★（:1724）「S-01 の版は受け取って書いたすべての記録に持つ（**衛星の記録も**。衛星自身の版は付けない）」。
  前の訂正（:1725 ★ C7 / C8）は決着状態の本文を書き換えた（「送った」→「作った時」、本人が書く記録を足した）。今回は本文を残して ★ を足したので、決着状態の 1 行と ★ が食い違う。
  `docs/stories/ST49.md:34-35` はこの 2 行を逐語で並べて写す。`docs/stories/INDEX.md:343` の扉の表は ST49 を指すだけで中身を写していない
- 直すところ: requirements（扉 32 の決着状態の本文を、S-01 の版はすべての記録・衛星自身の版は付けない・画面の記録は画面の版も、に書き換える）
- 機械: できない

## C3. 「受け取って D-01 に書いたすべての記録」と完了の判定の「すべて」が、design が外した行（生存信号・原本の受け取り・書庫の取り込みの台帳など）まで含む
- 行き先: requirements（FR-123 の本文を design D7 の範囲 —— S-01 が書く記録と、収集アプリと画面の操作が作る追記のみの行 —— に絞り、S-01 の内部の台帳と生存信号を外す理由を FR-123 / FR-122 の ★ に書いた。stories.json の完了の判定も同じ範囲に直して ST49.md を貼り直した。追記のみの表を列挙する機械の試験は、合意した st49 の tasks を変えることになるので、ここでは置かない）
- 重大度: 中
- 文書: docs/requirements.md:1026
- 要件: FR-123, FR-122
- 根拠: FR-123 ★（:1026）「S-01 の版は、受け取って D-01 に書いた**すべての**記録に持たせる（C-01 / C-02 の記録・衛星の記録・破棄の報告・原本の欠け・画面の操作の台帳（…）も）」、FR-122 ★（:1021）「版を持つ『記録』には、`core.event` の外で収集アプリが作る追記のみの行（破棄の報告・原本の欠け）も含める」、
  完了の判定（`docs/stories/stories.json:1171`）「受け取って書いた**すべての**記録（収集アプリの記録・破棄の報告・原本の欠け・書庫の取り込み・滞在の派生・画面の操作の台帳を含む）に、書いた時点のサーバの版が残り」。
  一方 design は版の列を 8 表に限る（`design.md:105-109` D7: `core.event`・`core.event_version`・`core.drop_report`・`core.media_missing`・`core.deletion_ledger`・`core.feeling_reminder`・`core.stay_criteria`・`core.attribute_kind_name`）。
  外したもの: `design.md:33-34`「`core.stay_absorbed`・`core.outage_ledger`・`core.export_ledger`・`core.media_original`・`core.web_session`・`core.access_log`」、`design.md:32`・D18（:172）・premises C13「生存信号（`core.heartbeat`）には版を付けない」。
  このうち `core.heartbeat` は C-01 / C-02 から受け取って書く行で、追記のみ（`migrations/202609111112_immutable_heartbeat.sql:33-34` `BEFORE UPDATE OR DELETE ON core.heartbeat`）—— FR-122 ★ の「`core.event` の外で収集アプリが作る追記のみの行」にも FR-123 ★ の「受け取って…書いたすべて」にも当たる。
  `core.media_original` は C-01 から受け取った画像の原本の受け取りの行（`migrations/202610181200_media_capture.sql:52` の注記「画像の原本を受け取った行」）。書庫の取り込みの台帳（`core.archive_ledger` ほか 8 表。`crates/server/src/archive/` が INSERT）は C-03 が書く行で、design の表にも除外の一覧にも名前が無い。
  また完了の判定の「受け取って書いた…（…滞在の派生…を含む）」は、S-01 が自分で作る滞在を「受け取って」に入れている
- 直すところ: requirements と stories（FR-123 ★ と完了の判定の「すべての記録」を、design の範囲 —— 記録の表・前の版と、収集アプリと画面の操作が作る追記のみの台帳 —— に合わせて書くか、生存信号・原本の受け取り・書庫の取り込みの台帳を外す理由を ★ に書く。逆に要件の「すべて」を採るなら design D7・D18 の側を広げる）
- 機械: 安く捕まえられる（`core` の追記のみの表 —— `BEFORE UPDATE OR DELETE` のトリガを持つ表 —— を列挙し、`server_version` を持つか除外の一覧に載るかを見る試験。いまは無い）

## C4. ST23 の新しい操作の台帳（`core.erasure_operation`・`core.erasure_span`）と ST23 が初めて書く `core.erasure_ledger` に版を持たせる側がどこにも無く、ST49 の機械の止めも捕まえない（印のみ）
- 行き先: handoff ST23（R5 の項を Q3 の答え「持たせる」に合わせ、3 表の列は後から main に入る側の移行が足すと書いた。st49 の design D19・tasks 11.1 は合意した仕様なので印のまま）
- 重大度: 中
- 文書: docs/handoff/ST23.md:148
- 要件: FR-123
- 根拠: FR-123 ★（:1026）「画面の操作の台帳（削除・…）も」S-01 の版を持つ。handoff（`docs/handoff/ST23.md:148`）「新しい操作の台帳（`core.erasure_operation` など）に版を持たせるかは、**ST49 の Q3 の答えに従う**」、同じ文が `openspec/changes/st49-record-component-version/tasks.md:209`。
  Q3 の答えは「持たせる（…画面の操作の台帳（削除・気分の通知・滞在の基準・個人属性の種類の名前）には S-01 の版）」（`deep.md:27`）で出たが、handoff と tasks は条件のまま。
  ST23 の design の表（`openspec/changes/st23-record-erasure/design.md:110-112`）は `core.erasure_operation`（1 操作 1 行・追記のみ）・`core.erasure_span` を作り `core.erasure_ledger` に `operation_id` を足すが、`server_version` の列は無い（st23 の change に `server_version` / ST49 の語は 0 件）。
  ST49 の D7（`design.md:105-109`）はこの 3 表に列を足さない（`core.erasure_operation` は ST49 の木に無い）。
  ST49 の design D19（`design.md:184`）「ST23 が足す…新しい操作の台帳は `server_version` を書く必要がある —— **D10 の機械の止めが rebase の後に落とす**」は、D10（:123）の止めが「`server_version` を持つ表への INSERT の文」しか見ないので、列の無い表への INSERT では落ちない。
  どちらが先に main に入っても、本文ごと消す操作の行は版を持たずに書かれる（後から足せない）
- 直すところ: その文書（`docs/handoff/ST23.md` の st49 R5 の項を Q3 の答えに合わせ、`core.erasure_operation`・`core.erasure_span`・`core.erasure_ledger` の列をどちらの移行が足すかを書く。走っている st49 の design D19・tasks 11.1 と st23 の design D4 は印のみ）
- 機械: 安く捕まえられる（C3 と同じ試験。列の無い追記のみの表を落とす）

## C5. 衛星の記録に S-01 の版を持たせることを、衛星の書き込みとモノ・都合のデータの Story（ST39・ST40・ST46）へ送る宛先が無い
- 行き先: handoff ST40（ST39・ST46 にも同じ項を置いた。ST46.md は新しく作った）
- 重大度: 中
- 文書: docs/requirements.md:1026
- 要件: FR-123, FR-86, FR-87
- 根拠: FR-123 ★（:1026）「S-01 の版は…すべての記録に持たせる（…**衛星の記録**…も）」、完了の判定（`stories.json:1172`）「衛星が書いた記録には衛星自身の版を付けない（**受け取って書いたサーバの版は付く**）」。
  ST49 の design D10（`design.md:124`）「衛星の記録（ST29 / ST39 の後）も**同じ INSERT を通るので** S-01 の版を持つ」—— 根拠は `core.event` への INSERT が同じになるという見込みだけ。
  衛星のモノは汎用の台帳（FR-86 :844「1 つの汎用の台帳に保持する」。ST40）、衛星の都合のデータは衛星の保管場所（FR-87 :857。ST46）に入り、どちらも `core.event` の外の新しい表になる。
  `docs/handoff/ST39.md`・`docs/handoff/ST40.md` に ST49 / `server_version` の項は無く、`ST46.md` は無い。ST49 の tasks 11.1（`tasks.md:209-212`）の申し送り先は ST23・ST50・ST29・ST52 だけ。stories.json の ST39 / ST40 / ST46 の完了の判定に版の語は無い。
  D10 の機械の止めは `server_version` を持つ表しか見ないので、ST40 / ST46 が列の無い表を作っても落ちない
- 直すところ: 凍結した Story への申し送り（`docs/handoff/ST39.md`・`ST40.md`・`ST46.md` に、衛星の書く行が S-01 の版を持つこと・衛星自身の版は持たないことを送る。出どころの tasks 11.1 の申し送り先にも足す）
- 機械: 安く捕まえられる（C3 と同じ試験が、ST40 / ST46 が足す追記のみの表を列か除外の一覧のどちらかへ寄せさせる）

## C6. 走っている st49 の文書が、FR-123 の「衛星を外す」★ Q13 と、2 つある「★ 2026-10-25」を出典に挙げたまま（印のみ）
- 行き先: leave: 出典の表記だけで、仕様の振る舞い（衛星自身の版を付けない・画面の版を持つ）は改訂後の FR-123 と一致する。直すと合意した仕様の digest が変わり、答えが推奨どおりなのに合意を聞き直すことになる
- 重大度: 低
- 文書: openspec/changes/st49-record-component-version/specs/data-durability/spec.md:11
- 要件: FR-123
- 根拠: data-durability spec.md:11「収集アプリの版も申告で、S-01 は確かめられない（**FR-123 が衛星を外す理由**と同じ）」、record-envelope spec.md:100「衛星の記録には衛星の版を付けない（**FR-123 ★ Q13**）」、premises.md:46 C7「（FR-123 の★ Q13）」、design.md:31「衛星の版（ST29 / FR-123 ★ Q13。D9）」。
  改訂後の FR-123 は衛星の記録を外さない（★ :1026。★ Q13 の「対象にしない」は狭められた —— deep.md:43）。衛星自身の版を付けない根拠は ★ 2026-10-25 訂正（:1027「★ Q13 の理由はそのまま」）にある。
  record-envelope spec.md:197「導出元: FR-123 ★ 2026-10-25（V-01 から本人が書く記録）」は、FR-123 に ★ 2026-10-25 が 2 つ（:1025 の C8 と :1026 の ST49 Q2-Q4）になり、画面の版の出典（Q4）は後者なのにどちらか読めない
- 直すところ: その文書（走っている st49 の specs data-durability・record-envelope、premises C7、design の Non-Goals の出典を ★ 2026-10-25（ST49 Q2 / Q4）に。印のみ）
- 機械: できない

## 未確認

- 衛星の書き込み（ST39）が `core.event` へ書くとき、`ingest_one` の 2 つの INSERT を通るか別の INSERT を持つか。ST39 は上流が始まっていないので確かめていない（C5 の D10 の見込みはこれに依る）
- `core.erasure_operation`・`core.erasure_span` の列を ST23 と ST49 のどちらの移行が足すのがよいか（後から main に入る側の rebase の順）。グラフの状態を見ていないので、どちらが先に入るかは確かめていない（C4）
- `agreement.json`（untracked）の中身と、合意がこの改訂（67a2b3fa）の後の状態を指しているか。この監査の範囲外として読んでいない

## 触らなくてよいと判断したもの（理由つき）

- **FR-122 ★ の事実**: 例の版番号 `0.1.0` は 3 つの宣言と一致（`Cargo.toml:6`・`collector-android/app/build.gradle.kts:27`・`web/package.json:4`）。破棄の報告・原本の欠けが `core.event` の外の追記のみの行であることは `migrations/202609151546_drop_reports.sql:61-68` のトリガと `crates/server/src/drops.rs:275`・`media_missing.rs:139` の INSERT で確かめた。
  画面の操作の台帳 4 つ（削除・気分の通知・滞在の基準・個人属性の種類の名前）の INSERT は `deletion.rs:417`・`stay_store.rs:911`・`feelings.rs:791`・`stay_store.rs:116`・`attributes_store.rs:109,188,230` にあり、design D10 の挙げる箇所と一致する。文書とコードの食い違い（K）は見つからなかった
- **NFR-25 / FR-104 の版番号の形との両立**: 印 `<版番号>+<commit>[.dirty][.debug]` は `-beta.N` の後ろに build の付記として付くので、FR-104 の「MAJOR.MINOR.PATCH、ベータ版は `-beta.N`」と食い違わない。版番号の出所を 1 つにするのは ST52（tasks 11.1 の ST52 への申し送り）
- **FR-130（欄を足したら送信契約の版を上げる）と ST49 の `client_version`**: 改訂前からの関係で、FR-130 ★（「最初の正式版を出すまでの足し方…は版 1 に含める」）が受けている。この改訂は変えない
- **FR-121 ★（版の変化は生存信号で知る）**: 生存信号に動いている版を足すのは ST51 のまま。生存信号に S-01 の版を付けるかは C3 で扱った
- **ST49 の価値・表題・壊してはいけないもの**: 「その版で作られた記録だけを選び出したい」は改訂後の範囲（サーバの版・画面の版が足された）でも成り立ち、壊してはいけないもの 3 つ（重複判定・門・書き出しの列）を崩す決定は無い
- **`docs/stories/INDEX.md:469` の「Q3 で『記録の表だけ』…が選ばれたら、`spec --answers` が…割当を外す」**: 答えが推奨どおりで、条件が成り立たないまま残るだけ。capability の割当（INDEX.md:91-103 の ST49）は答えと合っている
- **走っている st49 の proposal / design の「本人の答えが推奨と違えば `spec --answers` が書き直す」・go.json:172 の未解決**: 答えが推奨と同じで、要件と完了の判定への戻しはこの改訂で済んだ。条件が成り立たない文が残るだけで実装を誤らせない
- **正典の Scenario の印**: この改訂は正典を変えない。版を持たせる先の 8 capability（record-envelope・device-collection・desktop-collection・derived-records・subjective-log・personal-entities・record-deletion・data-durability）の全 Scenario（1086 本）を機械で見て、欠けは device-collection の「数か月操作しなくても権限が剥がれない」1 本だけ（3 か月の実時間の物理の確認。`docs/verify/20261024-0521.md:9` が「確認できない（まだ）」で持つ）。この改訂と無関係
- **`docs/production-prep.md:47`（バージョンの単一情報源）・`:103`（FR-32 の版の列）**: 前者は OpenAPI の `info.version` の話で ST52 が受け、後者は文字抽出のエンジンの版で FR-122 / FR-123 と別
