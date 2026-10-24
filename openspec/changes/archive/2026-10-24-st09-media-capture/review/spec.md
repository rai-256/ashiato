# ST09 上流成果物の独立レビュー（spec-review）

**やり方**: `deep.md` の本人の答え（Q1〜Q17）と C1〜C19、「spec の段へ渡すもの」の 5 項目を 1 件ずつ取り、
`specs/` / `design.md` / `tasks.md` / `docs/requirements.md`（FR-3 / FR-4 / FR-5 / FR-6 / FR-32 / FR-50 / FR-78 / FR-79 / FR-88 / NFR-6 / NFR-9 / NFR-16 の現在の本文）/
`docs/stories/ST09.md` / `docs/stories/INDEX.md`（2026-10-17 の訂正）/ `docs/handoff/ST38.md`・`ST45.md`・`ST31.md` まで追った。
問いの context は `deep-questions.json` の文面そのもの（Q7 / Q8 / Q9 / Q12 / Q14 / Q15 / Q16）を読んだ。
MODIFIED の 4 本は正典 `openspec/specs/{device-collection,collection-coverage}/spec.md` と Requirement 単位で diff を取った。
tasks が名指しする道具・クラス・既存の試験は repo で実在を確かめた（`tools/*.sh`・`crates/server/src/export/excluded.rs`・`coverage.rs`・`collector-android/app/src/test`）。
**成果物は編集していない。**

## 機械の検査（先に走らせた。2026-10-17）

```
$ openspec validate st09-media-capture --strict
Change 'st09-media-capture' is valid                                     rc=0

$ python3 scripts/check_chain.py .
  要件 142 件 / Story 47 本 / 扉 31 項
  [ok]   どれかの Story に拾われた要件: 140/142 件
  [ok]   INDEX の「Story の対象外」: NFR-11, NFR-8
chain: OK (0 件 / 未回収 0 件 / warn 0 件)                                rc=0

$ python3 scripts/check_scenarios.py . st09-media-capture
  [FAIL] 担保の無い Scenario: …（106 件。device-collection / collection-coverage / data-durability の ST09 が足したもの）
scenarios: FAIL (担保なし 106 件 / 名無しの確認待ち 0 件)                 rc=1

$ python3 scripts/review_triage.py . st09-media-capture
  指摘 15 件 / 仮決め 0 件 / 要件へ戻すもの 9 件
triage: OK                                                               rc=0
```

- Scenario は 148 本（device-collection 96・collection-coverage 46・data-durability 6）。tasks.md:17-18 の数と一致する
- FAIL 106 = ST09 が新しく足した 108 本 − 2 本（`写真を撮るとメタが D-01 に、原本が D-02 に入る` / `動画を撮るとメタだけが入り原本は送られない` は
  tasks.md:220-227 の「人間の確認待ち」で担保ありに数えられている）。担保あり 42 = MODIFIED で写した既存 40 本 + その 2 本。上流ではテストが 0 本なので想定どおり
- ST09 が足した 108 本は、どれも tasks.md のどこかの Task に名前が出る（突き合わせた）
- **MODIFIED 4 本は正典の本文と Scenario を 1 行も落としていない**（diff で確認。追加と、既存 Scenario 3 本の WHEN に「（メディアのソースでない）」を足した変更と、
  :215 の「例外は集計の 1 ソースだけで、写真・…」から写真を外した変更だけ）。見出しは正典と一字一句一致する
  - `device-collection`「端末は想定間隔ごとに生存信号を送る」: 本文 1 文・理由 4 行・Scenario 2 本の追加だけ
  - `collection-coverage` の 3 本: 本文の追加・既存 Scenario 3 本の WHEN の限定・Scenario 11 本の追加だけ
- INDEX の割当て（collection-coverage は取得可否・収集開始日・格子の見え方、data-durability は書き出しの分類、record-deletion に触らない）は超えていない。
  `record-deletion` の delta は無い。proposal の Capabilities と `specs/` のディレクトリと INDEX の capability 表（INDEX:81 / :84 / :92）は一致する
- `specs/` に実装の名前（関数名・crate 名・列名）は入っていない（バッククォートの中身は論理ソース名・日付・パスの例・`ACCESS_MEDIA_LOCATION` だけ）
- `check_chain` の観点 8（再生成との一致）は通っている
- `review_triage` の OK はこのファイルを書く前の値。このファイルの指摘には処置が無いので、次に走らせると FAIL になる

---

## R1. 出来事の時刻の 2 段目（端末の一覧の撮影時刻）を design だけが決めていて、spec の Scenario と食い違う（Q14 は `loss: rewrite-all`）

- 成果物: openspec/changes/st09-media-capture/design.md
- 根拠: 本人の答えは「撮影時刻。無い・狂っているときは端末に保存された時刻」（deep.md:105）。spec `specs/device-collection/spec.md:99-100` も
  「出来事の時刻を撮影時刻にする。IF 撮影時刻が無い… THEN 端末に保存された時刻」の 2 段で、`:101` は「撮影時刻（ファイルのメタ）・端末の一覧の撮影時刻・保存された時刻」を
  別の 3 つとして並べる。一方 `design.md:90-91` D4 は「撮影時刻 = `taken_at_exif` を `offset_exif` で読んだもの、**無ければ `date_taken_store`**」と、
  端末の一覧の `DATE_TAKEN` を 2 段目に挟み、それを採っても `time_basis = taken` になる。Q14 の context（`deep-questions.json` Q14）は
  `DATE_TAKEN` を「EXIF から作られるか、無ければ保存の時刻」と書いており、どれを骨格に採るかを本人に聞いた当の問い。
  Scenario `撮影時刻が無い画像は保存された時刻になる`（`:119-122`）は「メタに撮影時刻の無いスクリーンショット → 保存された時刻・保存の印」で、
  `DATE_TAKEN` を持つスクリーンショット（端末が書く）では design の実装だと出来事の時刻が `DATE_TAKEN`・印が `taken` になり、この Scenario と逆になる。
  出来事の時刻は書き換え禁止（`specs/device-collection/spec.md:111`、C7）。
  `DATE_TAKEN` が「EXIF から作られるか、無ければ保存の時刻」で埋まるなら、D4 の 3 段では `date_added` に落ちて `time_basis = stored` になる経路がほぼ無くなり、
  本人が選んだ「保存された時刻 + 印」の枝が実装から消える
- kind: technical
- loss の見直し: 当初 rewrite-all（下の対応のとおり、直した後に失われるものは残らない）
- 提案: 2 段目を挟むかを本人に返す（推奨を付けて）か、spec の Requirement に「撮影時刻 = ファイルのメタ、無ければ端末の一覧の撮影時刻」を書いて
  「端末の一覧の撮影時刻を採ったときの印」を決め、Scenario を 1 本足す（メタに無く一覧にだけある画像）。どちらでも spec と design を同じ形にする
- 処置: fixed D4 —— loss は design の側の逸脱にあった（本人は `DATE_TAKEN` を派生の値として見たうえで「撮影時刻 → 無ければ保存の時刻」を選んだ）。design D4 を spec と Q14 の逐語に合わせ、端末の一覧の撮影時刻は持つが採らない形にした。spec に Scenario `端末の一覧にだけ撮影時刻がある画像は保存された時刻になる` を足した。新しい選択肢は無いので本人には返さない
## R2. Q14 の「狂っている」の境界が spec と design で違い、Scenario が境界を踏まない

- 成果物: openspec/changes/st09-media-capture/specs/device-collection/spec.md
- 根拠: 本人が選んだ選択肢の detail と FR-3 ★（requirements.md:146）は「端末に保存された時刻より **1 日以上後**」、spec `:100` も「1 日以上後」。
  `design.md:91` D4 は「`date_added` より **24 時間を超えて**後」、tasks.md:37 の定数は `TAKEN_AT_MAX_AHEAD = 24 時間`。ちょうど 24 時間後の 1 枚で答えが分かれる。
  Scenario `保存より 1 日以上後の撮影時刻は採らない`（`:130-133`）は「2 日後」、`2000 年より前の撮影時刻は採らない`（`:124-128`）は 1980 年で、
  どちらも境界から遠い。本人の決定の数値（1 日・2000-01-02）が Scenario に無いので、実装が閾値を 2 日や 1999 年にしても spec は落ちない
- kind: technical
- 提案: design を「24 時間以上」に揃え、境界の Scenario を 2 本足す（ちょうど 24 時間後 → 保存の時刻 / `2000-01-01T23:59:59` → 保存の時刻・`2000-01-02T00:00:00` → 撮影時刻）。
  `2000-01-02` をどの地域で読むか（design は `Z`）も Scenario に書く
- 処置: fixed D4 —— 境界を「24 時間以上後（ちょうど 24 時間を含む）」「`2000-01-02T00:00:00Z` より前」に揃え、境界の Scenario を 2 本足した（`2000-01-02 ちょうどの撮影時刻は採り、その直前は採らない` / `撮影時刻が保存のちょうど 24 時間後なら保存された時刻になる`）
## R3. 動画の撮影位置が、消した滞在の時間でも衛星と AI の読み出しで渡りうる —— 要件と ST45 の申し送りが「写真と画像」だけを挙げる

- 成果物: docs/handoff/ST45.md（と docs/requirements.md FR-50 ★ 2026-10-17）
- 根拠: requirements.md:682「衛星と AI への読み出しでも、消した滞在の時間に入る**写真と画像**の撮影位置は渡さない」。FR-3 ★（:152）は
  「範囲の全部を言うときは『写真と画像』と書く」で、これは画像の全部（動画を含まない）。`docs/handoff/ST45.md:17` も
  「写真と画像（`c01-photo` と ST09 が足す画像のソース）」。一方、動画も削除の印が付かず（requirements.md:672 の「写真・動画・…」、spec `:587`）、
  緯度経度を持つ（spec `:155-158`）、FR-88 ★（2026-10-17）で動画の撮影位置は「写真の位置」の種類。書き出しは `c01-video` を緯度経度を持つ側に置いて除く
  （`specs/data-durability/spec.md:5` / `:23-26`）ので、外へ出る経路のうち衛星と AI だけが動画を漏らす
- kind: defer
- loss の見直し: 当初 exported（下の対応のとおり、直した後に失われるものは残らない）
- 提案: FR-50 ★ の文を「写真と画像と動画の撮影位置」に直し（record の段の扱い）、`docs/handoff/ST45.md` に `c01-video` を足す。
  ST45 の試験に「消した滞在の時間の動画の撮影位置を渡さない」を 1 本置くよう書く
- 処置: deferred ST45 —— 衛星と AI の読み出しの経路はまだ無く（ST29 が衛星の資格情報ではどの口も返さない）、ST45 の上流も始まっていない。`docs/handoff/ST45.md` に `c01-video` を足す節（st09-media-capture R3）と試験 1 本を書いた。厳しい側の既定を動画へ広げるだけで扉は閉じたまま。要件の★ はこの段で直さない（整合の確認の後に要件を変えると鎖が落ちる。ST45 の上流で本人に見せる）
## R4. 原文の 16 KB の上限で長いタグを落とす形が design だけにあり、spec の「EXIF の全部の欄」と食い違う —— 動画は原本が無いので戻らない

- 成果物: openspec/changes/st09-media-capture/design.md
- 根拠: spec `:106` は「原文に、…ファイルから読めるメタ（写真は EXIF の全部の欄、動画は入れ物のメタ）を端末が返した値のまま持たせる」、
  `:107` で除くのは「縮小画像などのバイナリの塊」だけ。C5（deep.md:141-143）も「捨てるより持つ。動画は原本を送らないので、原文に持たなかったメタは端末から消えたら取れない」。
  `design.md:95-97` D4 は (a) 型が UNDEFINED / BYTE で 256 バイトを超えるタグを落とし、(b) **原文が 16 KB を超えたら長いタグから `omitted` へ移す**。
  (b) はバイナリでない文字のタグも落としうる。Scenario `原文に埋め込みの縮小画像が入らない`（`:170-173`）は「16 KB 以下」を THEN にしており、
  16 KB を超えたときに何を落とすかを spec は言わない。D4 の反転条件（`:103`）は「写真は後から読み直せる。動画は戻らない」と自分で書いている
- kind: technical
- loss の見直し: 当初 uncaptured（下の対応のとおり、直した後に失われるものは残らない）
- 提案: (b) の切り詰めを本人に返すか、spec に「16 KB を超えるときに落とすのはバイナリの型のタグだけ・文字のタグは落とさない（上限は D20 の 20 KiB の門で見る）」を
  SHALL と Scenario で書く。どちらでも `omitted` に名前が残ることを Scenario にする
- 処置: fixed D4 —— 16 KB での切り詰めを外し、文字のタグは長さで落とさない形にした（C5「捨てるより持つ」の側）。spec に「文字の欄は長くても原文から落とさない」と Scenario `文字の欄は長くても原文から落ちない` を足し、縮小画像の Scenario の THEN を「入れなかった欄の名前を持つ」に直した。大きさが門（20 KiB）を超えたら Task 2.3 で本人に返す
## R5. 消えた事実の記録の外部識別子が、spec（中身のハッシュ）と design / tasks（`vanished:…`）で違う

- 成果物: openspec/changes/st09-media-capture/specs/device-collection/spec.md
- 根拠: spec `:177-178`「メディアのソース（4 つ）の記録の外部識別子を、中身のハッシュ（SHA-256）にする」。消えた事実は同じ論理ソースの記録（`design.md:105` D5）。
  `design.md:98` D4 と tasks.md:101 は消えた事実の識別子を `vanished:<sha256>:<volume>:<media_id>` にする。spec のとおりハッシュにすると、
  消えた事実が写真の記録と同じ外部識別子になり、FR-22 の更新の経路で**写真の記録を消えた事実で上書きする**。
  Scenario `消えた事実を 2 度見ても 1 件のまま`（`:223-226`）が 1 件に保たれる理由も spec に無い
- kind: technical
- 提案: spec の Requirement を「写真・画像・動画の記録は中身のハッシュ、消えた事実は中身のハッシュと置き場と項目の番号から作る別の識別子」に直し、
  Scenario「消えた事実を作っても写真の記録は前の版にならない」を 1 本足す
- 処置: fixed D4 —— spec の Requirement を「写真・画像・動画の記録は中身のハッシュ、消えた事実はハッシュ・置き場・項目の番号から作る別の識別子」に直し、Scenario `消えた事実を作っても写真の記録は前の版にならない` を足した
## R6. 原本の欠けの報告が、未送信の 90 日・2 GB で捨てられないことが spec に無い

- 成果物: openspec/changes/st09-media-capture/specs/device-collection/spec.md
- 根拠: spec `:522`「原本の欠けの報告を、記録と同じ未送信の仕組みに乗せて再送し…」だけ。正典の保持の Requirement には
  `生存信号は上限を超えても捨てられない`・`破棄の報告は上限を超えても捨てられない`（openspec/specs/device-collection/spec.md:364-372）があるが、
  欠けの報告には無い。「捨てない」は `design.md:182` D11 の「`DropReport` と同じ扱い」にしか無い。捨てると FR-4 ★（requirements.md:157）の
  「失ったことは件数で残す」が消え、ST10 の突き合わせ（`docs/handoff/ST10.md`）が欠けと不一致を分けられない
- kind: technical
- 提案: spec に「原本の欠けの報告を 90 日・2 GB の上限で捨てない」の SHALL と Scenario `原本の欠けの報告は上限を超えても捨てられない` を足す
- 処置: fixed D11 —— spec に「原本の欠けの報告を 90 日・2 GB の上限で捨てない」と Scenario `原本の欠けの報告は上限を超えても捨てられない` を足した
## R7. 記録が恒久的に断られたとき、原本の送り待ちの行が `awaiting_record` のまま残り、欠けにもならない

- 成果物: openspec/changes/st09-media-capture/design.md
- 根拠: `design.md:143` D8 は「記録が S-01 に受理（重複を含む）されたと分かったら `ready`」、`:146` は「未送信の上限で破棄されたら欠け `record_dropped`」。
  正典の `恒久的に断られた記録は未送信から消える`（openspec/specs/device-collection/spec.md:139-142）の経路で記録が消えたときの扱いが無く、行は永久に送られず、
  欠けにも数えられない（spec `:521` の理由の一覧にも無い）。また `design.md:156` は `409 record_missing` を「試し直す」にしていて、
  記録が後で生きていなくなった（`design.md:165` の「生きている記録」の判定）ときも 1 時間ごとに読み直して送り続ける（1 枚ずつ全体を読むので電池と通信を食う）
- kind: technical
- 提案: D8 に「記録が恒久的に断られたら行を消し、欠け（理由: 記録が断られた）を報告する」を足し、spec の理由の一覧と Scenario に 1 本足す。
  `409` を試し直す回数か期間に上限を置くかを（仮）にする
- 処置: fixed D8 —— 恒久的に断られたら行を消して欠け `record_rejected`、409 は 7 日続いたら欠け `record_missing`（D9。仮）。spec の理由の一覧と Scenario `恒久的に断られた記録の原本は欠けになる` を足した
## R8. 原本の大きさの上限（1 GiB）と、超えたときの欠け `too_large` が spec に無い

- 成果物: openspec/changes/st09-media-capture/specs/device-collection/spec.md
- 根拠: `design.md:164` D10 は受け口の上限を 1 GiB（仮）、`:157` D9 は `413` を原本の欠け `too_large` にする。`design.md:180` D11 の理由の一覧も `too_large` を持つ。
  spec `:521` の理由は「端末から消えた・中身が変わった・読めなかった・記録が未送信から破棄された」の 4 つで、大きすぎて受け取られない原本は spec のどこにも出ない。
  受け取る / 断るの境は観測できる振る舞いで、design に置くと archive で正典から落ちる
- kind: technical
- 提案: spec の受け口の Requirement に「1 件の原本は 1 GiB まで受け、超えたら送り直せない失敗として返す」、欠けの理由に「大きすぎた」を足し、Scenario を 1 本ずつ置く
  （数値は（仮）のまま D10 の反転条件を残す）
- 処置: fixed D10 —— spec の受け口に 1 GiB と「送り直せない失敗」、欠けの理由に「大きすぎた」を足し、Scenario `1 GiB を超える原本は送り直せない失敗になる` / `大きすぎて受け取られない原本は欠けになる` を置いた
## R9. 入れ直す（台帳が消える）と、D-02 に置き済みの原本も全部読み直して送り直す（約 29 GB）

- 成果物: openspec/changes/st09-media-capture/design.md
- 根拠: `design.md:55` D2「台帳が無い（入れ直した・消えた）ときは全部が初めて見るものになり、全部を読んで送る。S-01 は外部識別子で畳むので記録は増えない」は記録の話。
  原本は `:143` D8 で記録が受理（重複を含む）されれば `ready` になり、`:153-154` D9 で 2 回読んで本文を全部 `PUT` し、
  `:167` D10 の「受け取りの行がすでにある → `.part` を消して `200`」は**本文を全部受けた後**。Scenario `同じ原本が 2 度届いても 1 つだけ置かれる`（`:451-454`）はこれを許す。
  Q1 の答え（約 29 GB）では、入れ直しのたびに Wi-Fi で数日ぶんの送信と端末の全ファイルの読み直しが起きる。Risks（`:268-280`）に無い
- kind: daily
- 提案: 送る前に原本があるかを聞く形（受け口に存在の問い合わせを足す、または `/ingest` の重複の応答に「原本あり」を載せて `ready` にせず行を消す）を（仮）の D にし、
  Scenario「受け取り済みの原本は送り直さない」を足す
- 処置: fixed D9 仮 —— 送る前に `HEAD` で受け取り済みかを問い合わせ、受け取り済みなら読まずに行を消す（D9 / D10）。spec に問い合わせの SHALL と Scenario `受け取り済みかを問い合わせられる` / `受け取り済みの原本は送り直さない` を足した
## R10. Q7「前の版の原本も D-02 に残す」が Scenario に無い

- 成果物: openspec/changes/st09-media-capture/specs/device-collection/spec.md
- 根拠: 本人の答え（deep.md:98）「変わった後の版も取る（メタも原本も。前の版も残す）」、deep.md:118「前の版の原本も D-02 に残す」。
  spec `:181` は記録だけ（「前の版の記録も残す」）、Scenario `中身が変わった写真は新しい記録になり前の記録も残る`（`:196-200`）も記録と読み出しだけを見る。
  D-02 の原本が上書き・置き換えされないこと、新しい版の原本も置かれることは、どの Scenario も観測しない
- kind: technical
- 提案: Scenario `中身が変わった写真は新旧どちらの原本も D-02 に残る`（WHEN: 原本を置いた後に上書きして見直す / THEN: D-02 にファイルが 2 つあり、前のファイルのバイトは変わらない）を足す
- 処置: fixed 6.2 —— Scenario `中身が変わった写真は新旧どちらの原本も D-02 に残る` を足し、6.2 に名指しした
## R11. Q8 の「消えたと数えないもの」のうち 3 つが Scenario に無い

- 成果物: openspec/changes/st09-media-capture/specs/device-collection/spec.md
- 根拠: deep.md:119「問いの context の『消えたと数えないもの』も spec の Scenario にする」。Q8 の context（`deep-questions.json` Q8）は
  ごみ箱 / 置き場が外れた / 「選んだ写真だけ」に変わった・**外された** / **一覧が作り直されて番号が振り直された** / 別のフォルダへ動かした・**名前を変えた** の 5 つを挙げる。
  spec の Scenario（`:228-251`）はごみ箱・外れた置き場・選んだ写真だけ・動かした の 4 本。アクセスを全部外した、一覧の作り直し（`:42-45` は記録が増えないことだけを見る）、
  名前を変えた は無い。本文 `:210-211` が言うだけ
- kind: technical
- 提案: Scenario を 3 本足す（写真と動画へのアクセスを外して戻す / 一覧の版が変わり番号が振り直される / 名前を変える —— いずれも消えた事実は作られない）
- 処置: fixed 3.3 —— Scenario を 3 本足した（`写真と動画へのアクセスを外して戻しても消えたと数えない` / `一覧が作り直されて番号が振り直されても消えたと数えない` / `名前を変えた写真は消えたと数えない`）
## R12. 生存信号が 1 件も無いメディアのソースで、記録のある日の格子の状態が design D14 にしか無い

- 成果物: openspec/changes/st09-media-capture/specs/collection-coverage/spec.md
- 根拠: spec `:262` の例外は「(1) 収集開始日より前なら『導入前』（ただしメディアのソースは、その日に観測の記録が 1 件以上あれば『記録あり』）」で、
  収集開始日がまだ無い（`:85` の「まだ開始していない」）ときに「収集開始日より前」に当たるかを言わない。`design.md:200` D14 だけが「収集開始日が `NULL` なら全日」と決めている。
  Scenario `生存信号の無い写真のソースは記録があっても開始していない`（`:234-237`）は開始日だけを見て、その日の状態を見ない。
  正典の `1 件も届いていないソースは開始していない`（`:140-143`）は「どの日も導入前」で、記録があるときの答えは spec から決まらない。格子の見え方は INDEX が ST09 に許した範囲
- kind: technical
- 提案: (1) の例外に「収集開始日が無いときも同じ」を書き、Scenario「生存信号の無い写真のソースでも、記録のある日は記録あり」を足す
- 処置: fixed D14 —— 状態の判定 (1) に「まだ開始していなければ全日」を書き、Scenario `生存信号の無い写真のソースでも記録のある日は記録あり` を足した
## R13. 消えた事実の記録が格子で「記録あり」に数えられることが design D5 にしか無い

- 成果物: openspec/changes/st09-media-capture/design.md
- 根拠: `design.md:105` D5「（仮）消えた事実は同じ論理ソースの記録で、観測の記録として数える」、`:110` 出来事の時刻は気づいた時刻。
  本人が「空き容量を増やす」でまとめて消すと（Q2）、写真を撮っていない日（気づいた日）が `c01-photo` で「記録あり」になる。
  状態の判定（spec `collection-coverage:274-275`「格子で数えない記録を除いたもの」）は格子の見え方で、INDEX が ST09 に許した範囲。
  spec の Scenario はこの日の状態を見ないので、archive で正典から落ち、反転条件（D5 `:111-112`）の「格子の数えから外す」を変えても spec は落ちない
- kind: conflict
- 提案: `collection-coverage` の状態の判定に「消えた事実の記録は観測の記録に数える」（または数えない）を書き、Scenario を 1 本足す。D5 の（仮）と反転条件はそのまま残す
- 処置: fixed D5 仮 —— `collection-coverage` の状態の判定に「消えた事実の記録を観測の記録に数える」を書き、Scenario `消えた事実の記録がある日は記録ありになる` を足した。D5 の（仮）と反転条件はそのまま
## R14. 「見始めた日より前の記録ありの日を途絶の判定に入れない」が Scenario に無い

- 成果物: openspec/changes/st09-media-capture/specs/collection-coverage/spec.md
- 根拠: spec `:278`「メディアのソースの収集開始日より前の『記録あり』の日を、達成の分母と分子と、**途絶の判定**に入れない」、FR-79 ★（requirements.md:549-550）
  「窓（NFR-13）と途絶（FR-80）の判定には入れない」。Scenario は `見始めた日より前の記録ありの日は達成の分母に入らない`（`:414-417`）だけで、途絶を見ない。
  (9)「直前の観測の記録・生存信号からの経過が想定間隔を超えていれば途絶」の「直前の観測の記録」に見始めた日より前の写真が入るかで、開始日の直後の日の状態が変わりうる
- kind: technical
- 提案: Scenario を 1 本足す（例: 見始めた日の前日に撮影日の写真があり、見始めた日に生存信号が届く → 見始めた日より前の日は「途絶」にならない・途絶の期間の始まりに使われない）
- 処置: fixed 8.3 —— Scenario `見始めた日より前の写真は途絶の判定に使われない` を足し、8.3 に名指しした
## R15. 「選んだ写真だけ」の許可で通知を出さないことが design D6 にしか無く、spec の WHILE と読み分けられない

- 成果物: openspec/changes/st09-media-capture/specs/device-collection/spec.md
- 根拠: spec `:258`「WHILE 写真の位置の権限か、写真と動画へのアクセスが無い THE SYSTEM SHALL 端末の通知で…」、`:260` は「選んだ写真だけ」を別の WHILE で扱い、通知に触れない。
  Android 14 の部分の許可は `READ_MEDIA_IMAGES` が無い状態で、「写真と動画へのアクセスが無い」にも読める。`design.md:130` D6 が「『選んだ写真だけ』では出さない（生存信号だけ）」と決め、
  tasks.md:120 も design に従う。通知を出す / 出さないは観測できる振る舞い
- kind: daily
- 提案: spec の `:260` の Requirement に「選んだ写真だけのときは通知を出さない」（または出す）を書き、Scenario を 1 本足す
- 処置: fixed D6 仮 —— spec に「選んだ写真だけで写真の位置の権限があるときは通知を出さない」と Scenario `選んだ写真だけの許可では通知を出さない` を足した。D6 の見出しに（仮）を付けた
## R16. 過去ぶんの原本の Scenario が定義（保存の時刻で 24 時間）を観測せず、順の半分が Scenario に無い

- 成果物: openspec/changes/st09-media-capture/specs/device-collection/spec.md
- 根拠: spec `:389`「過去ぶんの画像（端末に**保存された時刻**が、収集がそれを見つけた時刻より 24 時間以上前のもの）」。Scenario `過去ぶんの原本はモバイル通信では送られない`
  （`:400-403`）の WHEN は「**撮影日が** 3 年前の写真を初めて取り」で、保存の時刻を言わない。撮影は 3 年前・保存は今日（受け取った画像・カメラから移した写真）の画像は
  定義では「新しく保存された」側なので、fixture の作り方でどちらにも転ぶ。24 時間の境界の Scenario も無い。
  `:390` の「過去ぶんは保存された時刻の新しい順に送る」は Scenario が無い（`:410-413` は新しいものが先、だけ）。
  24 時間の閾値（tasks.md:38 の `BACKLOG_AGE`）は tasks が「仮決めの値」に数えるのに、`design.md:140` D8 の見出しに（仮）が無く、`design.md:291` の Open Questions の列挙にも D8 が無い
- kind: technical
- 提案: `:400-403` の WHEN を「端末に保存された時刻が 3 年前の写真」に直し、境界（保存の 23 時間後 / 25 時間後に見つける）と、
  「過去ぶんどうしは保存の新しいものが先」の Scenario を足す
- 処置: fixed D8 —— Scenario の WHEN を保存の時刻に直し、境界（23 / 25 時間）・撮影が古く保存が新しい画像・過去ぶんどうしの順の 3 本を足した。D8 の見出しに（仮）と 24 時間の反転条件を付け、Open Questions の列挙に入れた
## R17. メディアの試行に保存の知らせを数える形が、正典「ソースごとに独立して収集する」の「取得契機の間隔を満点の刻みとする」を MODIFIED せずに外れる

- 成果物: openspec/changes/st09-media-capture/specs/device-collection/spec.md
- 根拠: 正典 openspec/specs/device-collection/spec.md:862-863「取得の試行と成功の数えを、ソースごとに別に持ち、**そのソースの取得契機の間隔を満点の刻みとする**」。
  ST09 の Scenario `メディアの生存信号の試行は見直しと知らせの回数で数える`（`:712-715`）は 6 時間の区間で試行 4（見直し 1 + 知らせ 3）で、
  取得契機の間隔 6 時間（`design.md:252` D22）を満点の刻みにすると 1 になる。`design.md:252-253` は「保存の知らせの分だけ試行が満点の刻みより多くなる」と認めている。
  整合の確認の「未確認」（docs/audit/2026-10-17-st09-consistency.md:133）が「spec の段で決まる」と送った件。決めた先が生存信号の Requirement の Scenario だけで、
  刻みを定める正典の条項は変わらないまま残るので、archive 後の正典が自分と食い違う
- kind: conflict
- 提案: 「ソースごとに独立して収集する」を MODIFIED に加え、「メディアのソースは、保存の知らせによる取得も試行に数える（満点の刻みの外）」を本文に書く。
  D22 の（仮）と反転条件はそのまま
- 処置: fixed D22 仮 —— 正典「ソースごとに独立して収集する」を device-collection の MODIFIED に加え、メディアのソースは保存の知らせによる取得も試行に数える、を本文に書いた
## R18. 新しく足す 2 つの口（原本の欠けの受け口・量の読み出し）に資格情報の Requirement と Scenario が無い

- 成果物: openspec/changes/st09-media-capture/specs/device-collection/spec.md
- 根拠: 原本の受け口は `:418` と Scenario `資格情報の無い原本の要求は断られる`（`:468-471`）を持つ。原本の欠け（`:519-554`）と量の読み出し（`:556-583`）は
  資格情報に触れない。量の読み出しは利用者ごとの記録の件数・バイト・欠けの理由を返す。いまのサーバの口はどれも `authorize` を通る（`crates/server/src/lib.rs` の各 handler）ので
  実装は従うだろうが、spec が言わないと試験が置かれない。正典 `data-durability` も読み出しに `資格情報の無い件数と状態の求めは断られる`（openspec/specs/data-durability/spec.md:527）を置いている
- kind: technical
- 提案: 2 つの Requirement に「資格情報を要求する」を足し、Scenario を 1 本ずつ置く
- 処置: fixed D12 —— 原本の欠けの受け口と量の読み出しに資格情報の SHALL と Scenario（`資格情報の無い原本の欠けの報告は断られる` / `資格情報の無い量の読み出しは断られる`）を足した
## R19. 「4 つの論理ソースを登録簿に…利用が主語で持つ」は、登録簿に主語の欄が無く、Scenario も主語を見ない

- 成果物: openspec/changes/st09-media-capture/specs/device-collection/spec.md
- 根拠: spec `:61`「4 つの論理ソースを登録簿に、想定間隔 6 時間・**利用が主語**で持つ」。主語はいま `crates/server/src/coverage.rs:52-60` の定数
  （`DEVICE_SUBJECT` / `USAGE_SUBJECT`）で、Must の 5 ソース（`MUST_ORDER`）にしか使われない。`design.md:211` D15 は定数を変えないとし、移行（D15）も主語の欄を足さない。
  Scenario `4 つのソースが登録簿に 6 時間の間隔で並ぶ`（`:87-90`）は間隔だけを見る。新しい 3 ソースが「利用が主語」であることは、どこにも観測できる形が無い
- kind: technical
- 提案: `:61` から「利用が主語」を外す（Must の 5 つにだけ主語がある、と `:62` に寄せる）か、主語を読み出す経路と Scenario を足す
- 処置: fixed D15 —— spec から「利用が主語」を外した（主語は Must の 5 ソースにだけあり、D15 は定数を変えない）
## R20. design に観測できる振る舞いが残っている（archive で正典から落ちる）

- 成果物: openspec/changes/st09-media-capture/design.md
- 根拠:
  - `design.md:217` D16「説明書の『入らないもの』に 3 つの表の名前を挙げる」—— 書き出しの説明書は本人が読む成果物。`specs/data-durability/spec.md:38-41` は zip にファイルが無いことだけを見る
  - `design.md:173-174` D10「置き場が消えていても作り直さない。既定の置き場だけは S-01 の起動時に作る」—— spec `:480-481` / `:504-507` は既定の場所だけで、作る / 作らないを言わない
  - `design.md:187` D12「`last_storage_error` は S-01 の実行中だけ持つ」—— spec `:560` は「直近の失敗の種別と時刻を読み出せる」で、再起動で消えることを言わない
  - `design.md:170-171` D10a の 120 文字・予約名 —— spec `:484` は置き場の外に書かないことだけ（これは spec に置かなくてもよい側。挙げるだけ）
- kind: technical
- 提案: 上 3 つを該当する Requirement の SHALL と Scenario に移す（D の側は理由と反転条件だけを残す）
- 処置: fixed D10 —— 3 つを spec に移した: 説明書の「入らないもの」（data-durability に SHALL と Scenario）/ 既定の置き場だけ起動時に作り、設定された置き場は作らない（Scenario `設定した置き場が無ければ作らずに書けない失敗にする`）/ 直近の失敗は起動し直すと消える（Scenario `書けなかった失敗は起動し直すまで読み出しに出る`）。D10a の 120 文字・予約名は design のまま
## R21. deep の「spec の段へ渡すもの」の「ソースの種類を Scenario にする」を、design D19 が ST38 への申し送りに変えた —— deep にも監査の行き先にも記録が無い

- 成果物: openspec/changes/st09-media-capture/design.md
- 根拠: deep.md:241「登録簿に足すソースの種類（C11）: …各ソースが FR-88 のどの種類に属するかを Scenario にする」。監査 C11 の「機械」（docs/audit/2026-10-17-st09-consistency.md:92）も
  「それより前は、st09 の spec で登録簿に足すソースの種類を Scenario にする」。`design.md:231-235` D19 は「登録簿に種類の欄がまだ無い。欄を作ると ST38 の承認の器を先に決める」で
  欄も Scenario も作らず `docs/handoff/ST38.md` へ送る。理由は筋が通るが、いまの `docs/handoff/ST38.md` に ST09 の割当ては無く（tasks 11.2 が下流の最後に書く）、
  deep.md の「渡すもの」は書き換えられていない。spec の段が deep の指示を変えたことが、どこにも残っていない
- kind: technical
- 提案: deep.md の「spec の段へ渡すもの」に「C11 は D19 で ST38 へ送った（理由）」を書き足し、ST38 の申し送りを上流のこの段で書く（下流の最後に置かない。R26 も参照）
- 処置: fixed deep.md —— C11 は AI の既定で、決定は変えていない（記録の不備）。deep.md に「spec の段で受けたところ」を足し、`docs/handoff/ST38.md` に割当て（st09-media-capture R21）を上流のこの段で書いた
## R22. deep が spec の段に求めた 2 つ（衛星と AI の Scenario・Q16 の差を GO 資料に）の行き先が成果物に無い

- 成果物: openspec/changes/st09-media-capture/proposal.md
- 根拠:
  - deep.md:216 / :243「消した滞在の時間に入る撮影位置は書き出しと**衛星・AI の読み出し**に出さない、を Scenario にする」。spec `:592` は「衛星と AI の読み出しで渡さないことは、
    その門を作る Story が受ける（`docs/handoff/ST45.md`）」で Scenario を置かない。読み出しの経路がまだ無いので検証できず、送るのは妥当だが、deep の側は直っていない（R3 の動画の穴もここ）
  - deep.md:128-129「画面と検索には写真が残り、書き出しからは除かれる、という差が残る…**spec の段は、この差を GO 資料に出して本人に見せる**」。proposal `:31` は
    「書き出しからは今のまま除く（NFR-16）」だけで、差（同じ写真が中では消えず、外へは出ない）を本人に見せる形になっていない。`docs/briefs/` に ST09 の spec の段の資料は無い（`ST09.html` / `ST09-deep.html` / `ST09-deep-r2.html` だけ）
  - deep.md:265「ST22 の画面 —— 消した時間の写真が日の画面に残る。表示は ST09 の spec の段で決める」。`design.md:25` の Non-Goals「1 日の画面にも写真を足さない」が答えになっているが、deep の項目を閉じたとは書いていない
- kind: technical
- 提案: proposal の「深掘りで本人が決めたこと」か GO の資料に Q16 の差を 1 行で出し、deep.md の 3 項目に「どこで受けたか」を書き足す
- 処置: fixed deep.md —— 3 項目の行き先を deep.md の「spec の段で受けたところ」に書き、proposal の What Changes に Q16 の差を 1 行で出した。GO 資料（reading の段）に「直感注意」として出すことを design の Open Questions に書いた
## R23. tasks の `GT` / `AT` の判定は 1 本以上しか見ず、各項目の「N 本以上」が判定に入っていない

- 成果物: openspec/changes/st09-media-capture/tasks.md
- 根拠: tasks.md:25-26 `GT <クラス>` は gradle の rc=0 かつ `grep -Eq 'tests="[1-9]'`（1 本以上）、`:27-28` `AT` も `tests="[1-9]`。
  一方、各項目は `GT MediaRecordTest（12 本以上）`（`:76`）・`GT MediaScannerTest（12 本以上）`（`:95`）・`GT MediaVanishTest（7 本以上）`（`:104`）・
  `GT MediaCapabilityTest（11 本以上）`（`:119`）・`GT OriginalUploaderTest（9 本以上）`（`:135`）と下限を書く。`CT` は `-- --list` で本数を数えるのに、`GT` は 1 本でも通る
- kind: technical
- 提案: `GT` の定義を「`TEST-*<クラス>*.xml` の `tests="` の値の和が N 以上」にする（例: `grep -ho 'tests="[0-9]*"' … | tr -dc '0-9\n' | paste -sd+ | bc` が N 以上）
- 処置: fixed tasks.md —— `GT` / `AT` の定義を、結果の XML の `tests=` の和が項目の本数以上、に直した
## R24. Task 2.2 は計測テストだけで閉じ、tasks 自身の制約に反する —— 2.3 の門と 11.2 の確かさの検査がエミュレータに依存する

- 成果物: openspec/changes/st09-media-capture/tasks.md
- 根拠: tasks.md:30「計測テスト（`AT`）だけで閉じる Task を作らない」。2.2（`:77-81`）の検証は `AT MediaPremisesInstrumentedTest` だけ。
  2.3（`:82-84`）の検証にも `AT` があり、`:65` は「2.3 が落ちたら Task 3 以降に進まない」。11.2（`:217-218`）は premises.md の P1 / P3 / P12 / P13 の確かさが 4 か 5 であることを grep する。
  手元の `/dev/kvm` が無い（MEMORY の 10/15〜 の記録）と、2.2 が `BLOCKED_INFRA` のまま 2.3 と 11.2 が閉じず、Task 3 以降も止まる
- kind: technical
- 提案: 2.2 を「前提の確かめは計測テストでしかできないので例外」と明記し、`BLOCKED_INFRA` のときの扱い（CI の `android-instrumented` の結果を証跡にする）を書く。
  2.3 は `GT MediaRecordSizeTest` だけで門を判定できるようにし、`AT` は追加の確かめにする
- 処置: fixed 2.2 —— 2.2 を計測テストの例外と明記し、`BLOCKED_INFRA` のときは CI の結果を証跡にする。2.3 の門は `GT MediaRecordSizeTest` だけで判定する
## R25. Task 4.2 が既存の Scenario `どのソースも生存信号の区間に取得契機が 1 回以上入る` を違うクラスに置いている

- 成果物: openspec/changes/st09-media-capture/tasks.md
- 根拠: tasks.md:119「既存の `GT CollectionSourceTest` と `GT SourceIndependenceTest`（`どのソースも生存信号の区間に取得契機が 1 回以上入る` を含む）が緑のまま」。
  その印は `collector-android/app/src/test/kotlin/dev/ashiato/collector/HeartbeatCountersTest.kt:107` にあり、`SourceIndependenceTest` には無い（`grep -rn` で 1 か所だけ）。
  `GT SourceIndependenceTest` が緑でも、4 つのメディアのソースを足した後のこの不変条件は走らない
- kind: technical
- 提案: 検証に `GT HeartbeatCountersTest` を足す（4 つのメディアのソースが不変条件の対象に入ることを、その試験が見るように 4.2 の本文に書く）
- 処置: fixed 4.2 —— 検証に `GT HeartbeatCountersTest` を足し、4 つのメディアのソースが不変条件の対象に入ることを 4.2 の本文に書いた
## R26. Task 11.2 の ST38 への申し送りの検証が、割当ての中身を見ない

- 成果物: openspec/changes/st09-media-capture/tasks.md
- 根拠: tasks.md:218 `grep -c "st09-media-capture" docs/handoff/ST38.md が 1 以上`。D19（`design.md:234-235`）の割当て
  （`c01-photo` → 写真 / `c01-image` → 写真 / `c01-screenshot` → スクリーンショット / `c01-video` → 動画 / 撮影位置は「写真の位置」）が書かれていなくても通る
- kind: technical
- 提案: 4 つのソース名がそれぞれ 1 回以上出ることを grep する。R21 のとおり、申し送りそのものは上流のこの段で書く
- 処置: fixed 11.2 —— ST38 への申し送りは上流で書いた。11.2 の検証を 4 つのソース名がそれぞれ出ることの grep にした
## R27. tasks の「全部をこのファイルのどこかの Task が名指しする」は、MODIFIED で写した既存の 37 本で成り立たない

- 成果物: openspec/changes/st09-media-capture/tasks.md
- 根拠: tasks.md:17-19「残る 40 本は MODIFIED で写した既存分で、印は既存のテストにある。**全部をこのファイルのどこかの Task が名指しする**」。
  spec の Scenario 名を tasks.md に `grep -F` すると、既存の 37 本（`記録が 0 件でも生存信号があれば稼働が残る` ほか。名指しされているのは 8.2 の 3 本だけ）が出ない。
  既存の印で担保されている（`check_scenarios` で担保あり）ので害は無いが、記述が実態と違う
- kind: technical
- 提案: 「ST09 が足した 108 本は全部名指しする。既存の 40 本は既存の印のまま」に直す
- 処置: fixed tasks.md —— 「新しく足した 136 本は全部名指しする。既存の 47 本は既存の印のまま」に直した（本数も数え直した）
## R28. 軽微 2 件 —— 一時ファイルの残り、知らせの Scenario の緩さ

- 成果物: openspec/changes/st09-media-capture/specs/device-collection/spec.md
- 根拠:
  - `design.md:164` D10 は `<置き場>/.incoming/<uuid>.part` に書き、失敗で消す。S-01 が書いている途中で落ちると `.part` は残り続ける（片付ける規則が無い）。
    Scenario `書いている途中で止まっても壊れたファイルは並びに残らない`（`:456-460`）は年/月の並びだけを見るので、置き場の容量を食い続けても落ちない。ST30 のバックアップにも載る
  - Scenario `保存の知らせで 6 時間の見直しを待たずに取られる`（`:27-30`）の THEN「次の 6 時間の見直しより前に」は、5 時間 59 分後に取っても緑。D1 の 10 秒（`design.md:40`）に対して緩い
- kind: technical
- 提案: 起動時に `.incoming` の古い `.part` を消す規則を D10 に足し、Scenario の THEN に「`.incoming` にも残らない（起動後）」を足す。知らせの Scenario は「1 分以内に」など D1 から決まる上限で書く
- 処置: fixed 6.1 —— 起動時に `.incoming/` の `.part` を消す（D10）と Scenario `起動すると書きかけの一時のファイルが消える` を足した。知らせの Scenario の THEN を「撮ってから 1 分以内」にした

---

観点 5（Story と要件の現在の本文）: 該当なし（確かめた範囲: `check_chain` OK（観点 8 を含む）。`docs/stories/ST09.md` の満たす要件の逐語は requirements.md の FR-3 / FR-4 / FR-5 / FR-6 / FR-32 / NFR-6 の現在の本文と一致。
完了の判定 3 本（写真はメタと原本 / 動画はメタだけ / 文字抽出の版の列）は Q4 / Q6 / C15 と矛盾しない。「壊してはいけないもの」（ST04 のバッファ）は Scenario `原本は未送信の置き場の大きさを増やさない` が受ける。
satisfies に無い FR-50 / FR-78 / FR-79 / FR-88 は INDEX:385-392 の表に理由つきで載っている。spec の導出元に出る FR-22 / FR-72 / FR-102 / NFR-9 / NFR-13 は、満たすのではなく引く・入力にするだけ）
