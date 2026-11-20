# CH21 の spec の検査（spec --draft・第 1 回の先読み。第 3 版）

## 機械の検査（2026-11-20、docs/ch21-upstream の ad69ff50e の上。spec・design・tasks は未コミット）

- `openspec validate ch21-trial-use-before-verify --strict` → `Change 'ch21-trial-use-before-verify' is valid`
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（change なので観点 8 は無い）
- `python3 scripts/check_scenarios.py . ch21-trial-use-before-verify` → `scenarios: FAIL (担保なし 51 件 / 名無しの確認待ち 0 件)`。下流の前なので担保が無いのは予定どおり（呼び出し元の指示で指摘にしない）。
  tasks.md が名指しする `# Scenario:` / `== Scenario:` の印は 51 本で、spec の `#### Scenario:` 51 本と名前が一字一句合う（python の正規表現で両方の名前を抜き出して突き合わせた）
- `python3 scripts/review_triage.py . ch21-trial-use-before-verify` → `triage: OK`（指摘 16 件 / 仮決め 0 件 / 要件へ戻すもの 0 件。review/deep.md の分）
- `python3 scripts/spec_overlap.py candidates . ch21-trial-use-before-verify` → `[]`

## R1. 公開の題の台本の「検査を走らせられなければ全部伏せる」と、伏せる形の半分が Scenario を持たず、Task 2.1 の自己試験は gitleaks が無いと期待を変えて緑になる
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・tasks.md
- 根拠: spec.md:170 は伏せる形に「この PC の利用者名・ホスト名・私設網の名前・`.env` の 8 字以上の値・`/mnt/`・`/Users/`・`\\`・IPv4」を挙げ、spec.md:172 は「検査を走らせられない…ときは、全部の題を伏せる」と書く。
  Scenario（spec.md:176-189）が見るのは `/home/`・`C:\`・メール・ホスト名・私設網の IPv4・私設網のホスト名・秘密の形・`@`/`#数字` だけで、利用者名・`.env` の値・`/mnt/`・`/Users/`・`\\`・私設網でない IPv4・「走らせられないとき全部伏せる」は Scenario に無い（Requirement の本文だけが言う）。
  さらに tasks.md:44 は「`tools/check-secrets.sh` が gitleaks を見つけられない木では、飛ばしたことを 1 行出して秘密の形の題だけを外す」と自己試験の期待を変える。spec.md:172 どおりなら、そのとき台本は 4 つとも伏せるはずで、spec.md:184 の「出力は 4 行、伏せは 3 つ」とも合わない。
  検査を走らせられないときに素通しで返す（fail-open）実装でも、自己試験はこの分岐で緑になる。`.env` の値は、tasks.md:40 が「cwd の木の `.env`」、spec.md:170 が「確認バッチの作業ツリーの `.env`」で、処置（harness2）が別の cwd で呼べば何も伏せない。
  同じ中身の写し（spec.md:220-221）は、前の回の索引（design.md:104 の `titles`）から題を写すが、写す題が公開の題の台本を通ったものか（索引に伏せる前と後のどちらを置くか）が決まっておらず、spec.md:224-227 の Scenario は私的な形の無い題（「並びがおかしい」）でしか見ない。
  Q4 は A（loss: exported）で、Q4 の context（deep-questions.json:140）は推奨の守りを「題も…公開に出す前に伏せる検査を通す。…通らなければ「題は手元の結果に」に置き換える」としている
- kind: irreversible
- loss: exported
- 提案: Scenario を足す:「検査を走らせられなければ、全部の題を伏せる」（差し込みで gitleaks の場所を無い所に向け、出力が全部「（題は手元の結果に）」）・「確認バッチの作業ツリーの .env の値と利用者名を含む題は伏せる」（`.env` は自己試験が作る）。
  写した回の Scenario の題に `/home/someone/x` を足し、写した段落でも伏せられていることを見る。
  tasks.md:44 の「秘密の形の題だけを外す」を消し、gitleaks が無い木では自己試験そのものを落とす（CI の `rust` の job は取れる）。`.env` を読む場所は「`--env <ファイル>` か cwd」と台本の口で決め、処置にも渡させる（D15 の 6）
- 処置: fixed D19 仮 —— 伏せる側に倒した。spec に Scenario「利用者名と .env の値を含む題は伏せる」「検査を走らせられなければ、全部の題を伏せる」を足し、`/mnt/`・`/Users/`・`\\` を「私的な形を含む題は伏せる」に入れた。Task 2.1 は gitleaks を飛ばさない（CI は取れる）。写した題も通し直す（D10・Scenario「写した題も、公開の題の台本を通し直す」）

## R2. Q4 の推奨が名指す「足す試験の名前」が、D15・harness2 への依頼・spec のどこにも無く、処置の Scenario は根拠とログが公開に出ないことを見ていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md・harness2-request.json・specs/verify-environment/spec.md
- 根拠: Q4 の context（deep-questions.json:140）は「直しの commit の本文・Story の issue へのコメント・足す試験の名前も同じ」と書き、review/deep.md R1 の処置もこの 3 つを足したと言う。
  design.md:150（D15 の 6）・harness2-request.json:12（scope）・spec.md:348 は「手順書の問い・違和感の issue・直しの commit の本文・Story の issue へのコメント」までで、試験の名前（と試験の中身に入る文字列）が落ちている。spec.md:348 は Story の issue へのコメントも挙げていない。
  また spec.md:362-365 の Scenario が見るのは `.local/state`・`RESULT.md`・`/home/` の 3 つの字面だけで、候補の根拠（evidence）の文やログの抜き書きが commit・手順書・`gh` の本文に入っても落ちない（spec.md:126-129 の目録の Scenario は根拠の字面「画面の写し a.png」で見ているのに、公開に出る側では見ていない）
- kind: irreversible
- loss: exported
- 提案: D15 の 6・harness2-request.json の scope・spec.md:348 に「足す試験の名前と、試験の中の文字列」と「Story の issue へのコメント」を足す。spec.md:365 の THEN に「用意した候補の根拠の文（自己試験が決めた字面）がどこにも無い」を足す
- 処置: fixed D20 仮 —— Q4 の推奨（公開に出すものを減らす側）のまま、D12 の処置の題の規則に「足す試験の名前と試験の中の文字列」を足し、D15 の 6・harness2-request.json の scope に「足す試験の名前と試験の中の文字列」を足した。受け入れの Scenario は用意した根拠の文がどこにも無いことを見る

## R3. Q12 の推奨の「選別や直しが落ちた回も 1 行書く」が、harness2 への依頼に写っていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md・harness2-request.json
- 根拠: Q12 の points（deep-questions.json:431）「回さなかった回（範囲が空・前の結果を写した）と、選別や直しが落ちた回も 1 行書く」。前半は spec.md:144・:221 にある。
  後半（選別・直しは harness2 の処置）は design.md:143-154（D15 の 1〜9）にも harness2-request.json の what・scope にも無い。処置が落ちた回は、手順書に何も出ず「通らなかった 0」のバッチとして機械が merge しうる（Q3 の context が「前から」の誤りで心配しているのと同じ型）
- kind: technical
- 提案: D15 に 10 として「選別・直し・確かめ直しが落ちたか上限で止まった回は、決まった文言の 1 行を手順書に書く（理由は手元だけ）」を足し、harness2-request.json の what にも写す
- 処置: fixed design.md D15 —— 10 として足し、harness2-request.json の what にも写した

## R4. Q3 の推奨の数（直すのは 3 件まで・同じ候補は 1 回まで）と「迷えば本人へ」が、どの Scenario にも無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md
- 根拠: spec.md:350 は導出元に Q3（「迷えば本人へ・直すのは 3 件まで・1 回まで」）を挙げるが、spec.md:347-370 の本文にも Scenario にも 3・1・「迷えば本人へ」が無い。数は harness2-request.json:12 にだけある。
  ashiato2 の正典の側では、処置が上限なしで直しても、迷ったものを「前から」に倒しても、どの Scenario も落ちない
- kind: technical
- 提案: どちらかに揃える。(a) 数と「迷えば本人へ」は harness2 の約束と割り切り、spec.md:350 の導出元から Q3 を外して「Q3 の数は harness2 への依頼（design D15 の 3）が持つ」と書く。
  (b) real-run の Scenario に「同じ候補を 2 度目に見つけても直さない」を足す。R5 の方針と合わせて選ぶ
- 処置: fixed specs/verify-environment/spec.md —— (a) にした。最後の Requirement で、数と「迷えば本人へ」は harness2 への依頼（D15 の 2・3）が持ち、ashiato2 の正典は約束しないと書いた

## R5. 最後の Requirement は、harness2 の確認バッチの振る舞いを ashiato2 の正典に約束させ、まだ無い台本の名前を書いている。Scenario の 1 つは条件つきで空でも通る
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md
- 根拠: spec.md:346-349 は「THE SYSTEM SHALL 候補が、…別の席の選別を経て…Story のブランチで直って…手順書の問いになり…issue になる」と、選別・直し・問い・issue（すべて harness2 の `verify_tsukau.py` の振る舞い。design.md:131-154）を ashiato2 の SHALL で書く。
  archive すると `openspec/specs/verify-environment/spec.md` に残るが、それを確かめるのは CI に載らない `tools/tsukau/real-run.sh`（本物の claude・Sandbox）だけで、harness2 が後で処置を変えても ashiato2 の正典は落ちない。
  spec.md:346・:368 は `.harness2/scripts/verify_tsukau.py` という、まだ無く、design.md:178 自身が「別の名前・別の入口で入ったとき」を反転条件に挙げている名前を Scenario に固定している（観点 3: specs に実装の名前）。
  spec.md:360 の THEN は「判断が要ると選別された候補が**あれば**…問いがあり」で、そう選別された候補が 0 件なら何も見ずに通る。同じ THEN に「前から」→ issue の主張を束ねている（観点 2）。
  spec.md:355 は、席（LLM）が崩した文言を見つけ、選別の席（LLM）が「直す」に分けることを前提にするが、design.md:176 の「本物の候補が出なければ用意した候補で通す」の逃げ道は tasks.md:168 では「前から」の側にしか書かれていない
- kind: conflict
- 提案: 最後の Requirement を「ashiato2 は処置との契約（目録・公開の題の台本・`TSUKAU_RECHECK`）を守り、処置まで通ることを実地の台本 1 本で受け入れる」に絞り、選別・直し・問い・issue の SHALL は harness2-request.json（と harness2 の試験）に置く。
  台本の名前は「harness2 の確認バッチの処置の入口（依頼 D15 の 1）」と書き、名前は real-run.sh の頭の 1 か所に持つ。spec.md:357-360 を「判断が要る候補は問いになる」と「前からの候補は issue になる」の 2 本に分け、どちらも用意した候補（選別の席に渡る前に種類が決まっている形）で必ず 1 件ずつ起こす。
  直す側にも「席が崩した文言を見つけなければ、用意した候補を足す」を tasks.md:168 に書く
- 処置: fixed D18 仮 —— 最後の Requirement を「処置との契約を守り、処置まで通ることを実地の 1 回で受け入れる」に絞った。台本の名前は spec から外し（design D15 の 1 と tasks だけ）、用意した候補を 2 つ足して条件つきの THEN を無くした

## R6. 実地の確かめで「直す」の道は、偽の feat/ch99 に Story の作業ツリーも PR の CI も無いので、依頼の 7（remote と gh の差し替え）だけでは通らない
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md・harness2-request.json・tasks.md
- 根拠: 直しの道は統合の CI の直しと同じ（design.md:146、D15 の 3）。その台本 `.harness2/scripts/verify_ci_route.py:15-19` は「載せる先は、その Story の作業ツリー（`git worktree list` で feat/st<NN>-* を checkout しているもの）。作業ツリーが無い・汚れている・手元が origin と違うときは載せない」、:7 は「その Story の PR の CI が緑になるのを待つ（scripts/ci_wait.py）」。
  tasks.md:166-168 の real-run は bare repo と merge は作るが、`feat/ch99-tsukau-real` の作業ツリーも、PR も、その CI の結果を返す偽の `gh` も作らない。`docs/changes/CH99.md` も作らないが、表題の文書そのものが無いときの範囲の行は spec.md:29-30 に決まっていない（spec.md:30 は価値の段落が無いときだけを決める）。
  このままでは処置は D15 の 3「載せられなければ本人に聞く」に倒れ、spec.md:355 の「一時の bare repo の feat/ch99 に直しの commit があり」は満たせない。harness2-request.json:12 の「push 先の remote と gh をその場で差し替えて単独で走らせられること」も、この 2 つ（作業ツリーと PR の CI）に触れていない
- kind: technical
- 提案: tasks.md の 10.1 に、`feat/ch99-tsukau-real` の作業ツリーを一時に作る・`docs/changes/CH99.md` を偽の木に置く・偽の `gh` が `pr list`/`pr checks` に緑を返す、を書く。
  harness2-request.json の scope の 7 に「Story の作業ツリーと PR の CI の待ちも差し替えられること（または、実地の口では待たないこと）」を足す
- 処置: fixed tasks.md 10.1 —— 偽の Story の作業ツリー・`docs/changes/CH99.md`・`pr checks` に緑を返す偽の `gh` を足し、D15 の 7 に Story の作業ツリーと CI の結果の差し替えを足した

## R7. Task 10 は harness2 の処置が入るまで検証が通らず、CH21 の PR はそれまで main に入らない。「それまでの確認バッチでは席を呼んで段落と目録を書く」は成り立たない
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md・proposal.md・design.md・docs/changes/CH21.md
- 根拠: tasks.md:165 は「`.harness2/scripts/verify_tsukau.py` が無ければ…rc≠0（飛ばさない）」、:175 の検証は `tools/tsukau/real-run.sh` rc=0。harness2 の処置は Task 9.3 が依頼を置いてから改良グラフで作られるので、同じ下流の流れの中で Task 10 の検証は落ち続ける。
  `.harness2/docs/flow-gates.md` と `.harness2/skills/story/task.md` を `外の待ち|harness2 の改良を待|待つ Task|requires.*HI` で探したが、Task が外の repo の完了を待つ仕組みは見つからなかった。
  下流の次の Task は `.harness2/graph/harness_graph/story.py:1386-1399` が「impl の未完がある Task のうち、A を積んで保留にしたもの以外の最初の 1 つ」と決め、保留は本人の答え待ちの間だけ（:1390）。最後の段（PR を作る final）へは、回せる Task が無くなってから進む（:1402-1411）。
  だから Task 10 が残る間、CH21 の PR は作られない。`完了の判定` を読んで merge と archive の間で待つ関門も `.harness2/scripts`・`graph` に見つからなかった（読むのは stories_json.py・story_brief.py・verify_checklist.py だけで、verify_checklist.py:327 は「完了の判定からは問いを作らない」）。
  一方 proposal.md:45・design.md:137・:182 は「それまでの確認バッチでは、席を呼んで段落と目録を書くところまでが動く」と書き、Q1 の推奨の代償（deep-questions.json の Q1 options[0].cost）も「CH21 は席を呼んで結果を載せるところまでしか動かない」と本人に見せている。
  `tools/verify-prep.sh` の変更が main に入らなければ確認バッチは席を呼ばないので、本人が読む代償より実際の代償が大きい（harness2 の処置が入るまで、席は確認バッチで 1 度も回らない）。docs/changes/CH21.md の完了の判定「それまで CH21 は archive しない」も、merge と archive の間で待つ前提に立っている
- kind: conflict
- 提案: 本人が答える前に形を決める。(a) Task 10 を CH21 から外し、harness2 の処置が入った後に始める別の change（CH21 を requires に持つ）に移す。CH21 の完了の判定は Task 8・9.3 まで。Q13 の推奨の言い方（「処置まで実地で通す」）が変わるので、Q13 の推奨の detail を直してから本人に見せる。
  (b) CH21 のまま待つなら、Q1・Q13 の代償に「harness2 の処置が入るまで、席は確認バッチで回らない（CH21 の PR が main に入らない）」を書き、proposal.md:45・design.md:137・:182 を直す
- 処置: fixed D18 仮 —— (b) にした（Q13 の推奨は「待つ」のまま）。代償を「harness2 の処置が入るまで CH21 は main に入らず、その間は確認バッチで席が回らない」に直し、proposal・design D14・Risks・deep-questions.json の Q1 と Q13 の代償（と Q13 の 2 つ目の案の得）も同じに直した。本人は正しい代償で Q13 を選べる

## R8. 候補の番号が 3 通りある（手順書は題の並び、`TSUKAU_RECHECK` は candidates.jsonl の行目、席の台本は `id` の C<n>）
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・design.md
- 根拠: spec.md:140 は手順書の番号を「`候補 1` から」、design.md:113 は「候補 <n>。`candidates.jsonl` の何行目か」とし、spec.md:140-142 は「候補の番号つきの題の一覧」を最後の行の `titles` から組む。
  spec.md:242 は `TSUKAU_RECHECK` の `candidate` を「`candidates.jsonl` の `candidate` 行目」とする。ところが `.harness2/scripts/tsukau.py` は、`titles` を `result["candidates"]`（壊れた行と題の空の行を飛ばしたもの。:168-185・:575）から組み、候補ごとに `id`（`C<n>`。:314）を振る。
  壊れた行か空行が 1 つあれば、手順書の「候補 2」・`titles` の 2 つ目・`candidates.jsonl` の 2 行目・`C2` が別の候補を指しうる。Q4 は公開に「候補の番号」を出し、処置は番号で直す候補を名指すので、取り違えると別の候補を直したと公開に書く
- kind: technical
- 提案: 番号は席の台本の `id`（`C<n>`）に揃える。手順書は「候補 C1」、`TSUKAU_RECHECK` の項目は `{"target","run","id"}`、題は `result.json` の `candidates` から `id` で引く。Scenario「確かめ直しは…」の GIVEN に壊れた行を 1 つ混ぜる
- 処置: fixed design.md D11 —— 番号は席の台本の `id`（`C<n>`）に揃えた。`TSUKAU_RECHECK` の項目は `{target, run, id}`、題は `result.json` の `candidates` から引く。確かめ直しの Scenario の GIVEN に壊れた行を混ぜた

## R9. 1 回の呼び出しの SIGTERM（1,560 秒）は、立てる上限と席の時間に片付け（300 秒）を数えていないので、正しく終わる途中の回を「打ち切った」にして結果を捨てる
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・design.md
- 根拠: spec.md:72・design.md:88 は「900 + 600 + 60 = 1,560 秒で SIGTERM、片付けを 360 秒まで待つ」。席の台本は席の時間の後に、ログを写し（`.harness2/scripts/tsukau.py:548-556`、1 つ 200 MB まで）、`down` を最長 `down_timeout_s`（既定 300。:558-568）待つ。
  立てるのに 890 秒かかった回は、席の 600 秒の後の片付けの途中で SIGTERM を受ける。台本は SIGTERM で印を立てるだけなので（:647-649）、`down` を終えて `stop` が `time` の正しい最後の行を出す。
  それでも spec.md:144 は「打ち切ったときは（最後の行の JSON に依らず）「時間の上限で打ち切った」」、spec.md:115 は `not_run` にするので、席が残した候補は目録で `run` を持たず、処置にも手順書の件数にも届かない
- kind: technical
- 提案: SIGTERM までを 900 + 600 + 300 + 60 = 1,860 秒、待ちを 360 秒（合わせて 2,220 秒）にし、準備の全体の上限（6,540 秒）も数え直す。または「打ち切った後でも、最後の行が読めて `stop` が `done`/`time`/`tokens` なら `ran` として件数と題を写す」に変え、Scenario を足す
- 処置: fixed design.md D8 —— SIGTERM まで 1,860 秒（片付け 300 を数える）・待ち 360・合わせて 2,220、全体 7,140 に直し、打ち切った後でも最後の行が読めて終えていれば写す Scenario を足した

## R10. PC の相手でインストーラに渡す合言葉が決まっていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md・tasks.md
- 根拠: 画面の相手は「その場で作った乱数の合言葉」「`--repo` の `.env` は読まず」（spec.md:312-313）と Scenario（:319-327）を持つが、PC の相手の Requirement（spec.md:256-266）は合言葉に触れない。
  tasks.md:131 は「合言葉の渡し方は `tools/release/install-check.sh` の `install_with` と同じ」で、渡し方（標準入力に 2 行）だけを決め、値の出所を決めていない。
  確認バッチから呼ぶ回は `--repo` が `.env` の無い使い捨ての作業ツリー（spec.md:68-69・D9）なので本物は届かないが、手で呼ぶ `/tsukau`（spec.md:7。`--repo` は本人の checkout で、本物の `.env` がある）では、env-hook が `--repo` の `.env` から合言葉を読めば、本物の合言葉が Sandbox と案内（席が読み、モデルへ渡る）に入る。
  画面の相手はこの経路を spec.md:313 で塞いでいるが、PC の相手には同じ文が無い。docs/changes/CH21.md の「壊してはいけないもの」は本人の合言葉を席に触らせないことを挙げている
- kind: technical
- 提案: PC の Requirement に「その場で作った乱数の合言葉で入れ、`--repo` と確認バッチの作業ツリーの `.env` を読まない」を足し、Scenario「PC の相手は、本人の合言葉でなく、その場の合言葉で入れる」（偽の `wsb exec` が受けた標準入力に `.env` の値が無い）を足す
- 処置: fixed design.md D4 —— その場の乱数の合言葉で入れ、`.env` を読まないことを Requirement と Scenario「PC の相手は、本人の合言葉でなく、その場の合言葉で入れる」に足した

## R11. 時間の上限の数（1,560・360・6,540・2,400 秒）が、どの Scenario にも無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md
- 根拠: spec.md:71-72・:199 は 2,400・1,560・360・6,540 秒を SHALL で書くが、Scenario（spec.md:101-109・:208-211）は自己試験で 2 秒に縮めた値で見るだけで、本番の値を見るものが無い。tasks.md:21 は台本の頭に定数で置くとだけ言う。
  実装が 1,560 を 600 にしても spec は落ちない（D8 は仮でない決定。2,400 は D5（仮））
- kind: technical
- 提案: Scenario を 1 本足す:「打ち切りの秒数は、立てる上限・席の時間・片付けの和で決まる」（`verify-batch.sh --print-limits` の出力か、台本の頭の定数を読んで 1560/360/2400/6540 と、`tools/verify-prep.sh` の席の部分の既定 6540 を見る）。R9 で値を変えるならその値で
- 処置: fixed specs/verify-environment/spec.md —— Scenario「打ち切りの秒数は、立てる上限・席の時間・片付けの和で決まる」（`--print-limits`）を足した

## R12. Q9 の推奨の「画面も触る」に、席が PC の画面を操作する口が無く、画面の写しの口も Scenario を持たない
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md・specs/verify-environment/spec.md
- 根拠: Q9 の推奨は「窓を開いて画面も触る」（deep-questions.json の Q9）。design.md:60 の `tools/tsukau/pc.sh` は `exec <PowerShell の文>` と `shot <置き場>` の 2 つだけで、クリック・入力の口が無い。
  spec.md:263 は案内に「コマンドを走らせて出力を受け取る口と画面の写しを撮る口」を書かせるだけで、`pc.sh` の `exec` が出力を返すこと・`shot` が写しを撮れることはどの Scenario も見ない（spec.md:305-307 の本物の確かめも healthz とプロセスの有無だけ）。
  `pc.sh` の振る舞い（席が PC に辿り着く唯一の口）は design.md:60 にだけあり、archive で正典から落ちる（観点 3）
- kind: technical
- 提案: design D4 に操作の口（例: `pc.sh click <x> <y>`・`pc.sh type <文>`。`ExistingLogin` の PowerShell から SendInput）を決めるか、「操作は PowerShell の UI Automation を `exec` で」と案内に書くと決める。
  spec の PC の Requirement に `pc.sh` の口を書き、Scenario を足す:「`pc.sh exec` は Sandbox の中の出力を標準出力に返す」（偽の `wsb`）・本物の確かめに「`pc.sh shot` で画面の写しが 1 枚撮れる」
- 処置: fixed design.md D4 —— 操作は `exec` から UI Automation と `SendKeys`、案内に例を載せると決めた（反転条件つき）。`pc.sh exec` の Scenario を足し、案内の Scenario に `pc.sh shot` と操作の仕方を足した

## R13. Requirement の本文だけが言い、Scenario が言っていないことがある
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md
- 根拠:
  - spec.md:71「PC の配る zip の置き場を絶対パスで渡させる（`ASHIATO_TSUKAU_PC_ZIPS`）」—— 画面の置き場は spec.md:91-94 が見るが、PC の置き場を見る Scenario が無い
  - spec.md:69「（harness2 を）繋げなければ、その相手の席を呼ばない」と spec.md:143 の「台本が無い・harness2 を繋げなかったときは「席を呼べなかった」」—— Scenario が無い
  - spec.md:242「項目の無い相手は呼ばない」—— 呼ばない相手の段落の文言と目録の `state` が決まっていない（`skipped`・`copied` のどれでもなく、spec.md:115 の 4 つにも当たらない）。確かめ直しの結果を D10 の索引に書くかも決まっていない
  - spec.md:314「渡されていなければ（手で呼んだとき）`<state>/tree` で組む」・spec.md:315 の案内の「その回の画面の合言葉・本物のブラウザで開いて写しを撮るコマンド・サーバのログの場所」—— Scenario が無い（:334-337 は URL と「触らない」だけ）
  - spec.md:27「`web/index.html`・`web/public/`」—— 振り分けの Scenario（:34-52）が使わない
- kind: technical
- 提案: 上のそれぞれに Scenario を 1 本ずつ足す。確かめ直しで項目の無い相手は「回さなかった（直した候補が無い）」と `state: skipped` に決め、Requirement と Scenario に書く
- 処置: fixed specs/verify-environment/spec.md —— PC の置き場の絶対パス・harness2 を繋げなかったとき・確かめ直しで項目の無い相手（`skipped`・「回さなかった（直した候補が無い）」）・手で呼んだときに別の木で組む・案内の合言葉と写しのコマンドとログ・`web/index.html` と `web/public/` の振り分けに Scenario を足した

## R14. 1 つの Scenario が 2 つ以上の主張を束ねている
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md
- 根拠: spec.md:149-153（段落の形と、パスが無いこと。AND）・:229-232（main が進んだ回と PR のコミットが変わった回）・:294-298（ログを写してから止める順と、自分の id だけを止めること。AND）・
  :319-322（起動の場所・`.env` の中身・`--repo` が変わらないこと）・:329-332（渡した置き場が届く回と、無い回に断ること）・:357-360（R5）。どれも片方だけが通っても、印の試験の名前は同じで緑になる
- kind: technical
- 提案: それぞれ 2 本に分ける（例:「main が進めば呼び直す」「PR のコミットが変われば呼び直す」）。tasks.md の印の数（4.1 の 23・7.1 の 15）も合わせて直す
- 処置: fixed specs/verify-environment/spec.md —— 段落の形と手元のパス・main と PR・ログの順と自分の id・起動の場所と --repo・置き場が届く回と無い回、を 2 本ずつに分け、tasks の印の数を直した（最後の Requirement は R5 で作り直した）

## R15. 第 2 版の Task 番号が 2 か所に残っている
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md・design.md
- 根拠: tasks.md:22「本物の Sandbox を使う確かめ（Task 1・7・10）」。第 2 版（`git show e2c9031f8^:…/tasks.md`:19）は「Task 1・7・8」で、第 3 版で本物の Sandbox を使うのは Task 8（8.1）と Task 10。Task 1 は組み立てだけ、Task 7 は偽の `wsb` の自己試験。
  design.md:73「網を切った Sandbox で…通るか…Task 7 で確かめ」も第 2 版の番号（第 2 版の Task 7 が本物の Sandbox）で、第 3 版では Task 8（tasks.md:149 が同じことを書く）
- kind: technical
- 提案: tasks.md:22 を「Task 8・10」に、design.md:73 を「Task 8 で確かめ」に直す
- 処置: fixed tasks.md —— Global Constraints を「Task 8・10」に、design D6 を「Task 8 で確かめ」に直した

## R16. Task 8 が使う「Task 1 で組んだ配る zip」の置き場が HEAD のコミットで決まり、Task 8 の時には見つからない
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md
- 根拠: tasks.md:30 は build-pc.sh の既定の置き場を `dist/tsukau-pc/<HEAD のコミット>/`、tasks.md:129 は env-hook がそこを探すとする。Task 1 の検証（tasks.md:32）は `$TMPDIR/pc` に組み、verify-run の `TMPDIR` は検証ごとに消える。
  Task 2〜7 の commit で HEAD が進むので、tasks.md:147「Task 1 で組んだ配る zip を置いて」は、既定の置き場でも Task 1 の検証の置き場でも見つからず `UNAVAILABLE=` になる。`real-check-pc` が組み直すのか、置き場をどう渡すのかが書かれていない
- kind: technical
- 提案: tasks.md:147 に「`real-check-pc` は始めに `tools/tsukau/build-pc.sh` で一時の置き場に組み、`ASHIATO_TSUKAU_PC_ZIPS` で渡す」と書く（組み立ての時間の分、検証が長くなることも書く）
- 処置: fixed tasks.md 8.1 —— 始めに build-pc.sh で一時の置き場に組んで `ASHIATO_TSUKAU_PC_ZIPS` で渡すと書いた

## R17. build-pc.sh の写しの除外に `.env` と `dist/` が無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md
- 根拠: tasks.md:28 の `rsync -a --delete` の除外は `target`・`.git`・`node_modules`・`web/dist`・`collector-android/local.properties` だけ。verify-batch.sh は確認バッチの作業ツリー（tools/verify-prep.sh:26-30 が main の checkout の本物の `.env` を写した木）で build-pc.sh を呼ぶ（tasks.md:66）ので、本物の合言葉が Windows 側の `C:\dev\ashiato2-tsukau-build\src` に写る。
  `dist/` にはその回の `ashiato-server`・`web/`・APK と、組んでいる最中の `dist/verify-<tag>/tsukau-pc` 自身があり、毎回写して組み立ての時間（上限 2,400 秒）を食う
- kind: technical
- 提案: 除外に `.env`・`.env.*`・`dist/` を足し、自己試験か 1.1 の検証で「写した木に `.env` が無い」を見る
- 処置: fixed tasks.md 1.1 —— 除外に `dist/`・`.env`・`.env.*` を足し、検証で写しの木に `.env` と `dist` が無いことを見る

## R18. Task 5.1 と 6.1 の検証が、足した印の数を数えていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md
- 根拠: 5.1（tasks.md:106）の検証は `--self-test` rc=0 と `bash -n` だけで、5.1 が足す 3 本の印（tasks.md:103-105）を数えない（4.2 の後で 25、5.1 の後は 28 のはず）。
  6.1（tasks.md:124）も `--self-test` rc=0 と `tsukau.py check` だけで、8 本の印を数えない（7.1 の 15 で後から数えるが、6.1 の時点では 0 本でもチェックできる）
- kind: technical
- 提案: 5.1 に `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/verify-batch.sh)" -ge 28'` rc=0、6.1 に同じ形で `tools/tsukau/env-hook.sh` が 8 以上を足す
- 処置: fixed tasks.md 5.1 —— 5.1 に 37 本以上、6.1 に 11 本以上の数えを足した

## R19. 「壊してはいけないもの」は fix の道を「CH21 は触らない」とするが、harness2 への依頼の 9 はその fix の後の振る舞いを変える
- 成果物: docs/changes/CH21.md・design.md・harness2-request.json
- 根拠: docs/changes/CH21.md の「壊してはいけないもの」2 つ目は「本人が手順書で「通らなかった」と答えた後の直し（確認バッチの `fix`）…（harness2 の確認バッチ。CH21 は触らない）」。
  design.md:154（D15 の 9）と harness2-request.json の what は「本人の「通らなかった」の後の fix の後にも、席の結果の段落を手順書に差し替える（今は録画などだけを差し替える）」で、CH21 の依頼で fix の後の手順書が変わる
- kind: conflict
- 提案: CH21.md の 2 つ目を「fix で直す道と、違和感の問いを issue にする道は残す（CH21 の依頼は fix の後に席の段落を差し替えることだけを足す。D15 の 9）」に直す

## 観点ごとの該当なし・確かめた範囲

- 観点 1（上の R1〜R4・R7・R12 のほか）: Q1（spec.md:111-134・design D14）・Q2（spec.md:347・D15 の 3）・Q5（spec.md:7・:18-21）・Q6（spec.md:23-62。依頼を狭める点は D2 と proposal の持ち越しにある）・
  Q7（spec.md:71・:96-99）・Q8（spec.md:261・:279-282。外につなぐ案に変わったときの直し方は design.md:74）・Q9 の窓（spec.md:262・:284-287）・Q10（spec.md:6・:67・:79・:242）・Q11（spec.md:217-237。D10（仮）は Q11 の「コミット」を木に読み替えた理由を持ち、台帳に載る）・
  Q12 の前半（spec.md:142-145・:155-164）・Q13（spec.md:344-370・tasks Task 10・CH21.md の完了の判定）は、推奨と同じことを言う Scenario がある。当初案を覆した印・要件へ戻すものは deep.md に無い
- 観点 3（上の R5・R12 のほか）: proposal.md の Capabilities（`verify-environment` を MODIFIED・Requirement 11 + 1）と `specs/` のディレクトリ・CH21.md の `capability` は合う（spec の Requirement は 12 本）。
  INDEX.md の capability 表は Story だけを載せ、走っている CH19・CH22・CH23 も載っていないので、CH21 が無いのは慣例どおり。design の D のうち観測できるもの（D2・D3・D6・D7・D10・D11・D12・D14・D16・D17）は spec に写っている（D4 の `pc.sh` を除く。R12）
- 観点 5: CH21.md の「完了の判定」は空でなく、deep の推奨・spec・tasks（Task 8・9.3・10・人間の確認待ち）と合う（R7 の待ちの仕組みを除く）。「依頼（逐語）」の範囲（差し込み口・画面と PC・処置の経路・止めない・公開に出さない・席の中身を変えない）を specs は越えていない。
  依頼の「fix で直す」を ci_fix の道に変えるのは Q2（premise）が本人に聞いている。第 2 版の古い参照は R15 の 2 か所のほかに見つからなかった（spec・design・tasks の Q1〜Q13・review/deep.md の R1〜R16 の番号は第 3 版の中身を指す。「issue も作らない」は席を呼ぶ台本についてだけ言い、第 3 版の依頼と食い違わない）。
  provisional.jsonl の 9 件（D1・D3・D5・D7・D9・D10・D12・D16・D18）は design の（仮）の 9 件と合う。lane.json の `paths` は第 1 版の `tools/trial-use/**` のまま。`.harness2/scripts/lanes.py` の冒頭（:1-30）と :329-342 では、申告の paths は軽い路（same / look / bug / recipe）に当たるかの裏取りにだけ使い、`reason: none` の申告はそのまま通常の路にする。通常の路で触るファイルを paths で縛る箇所は `paths` の grep で見つからなかったので指摘にしていない
- 観点 6: 該当なし（`spec --draft` の第 1 回の先読みで、本人の答えはまだ無い。answers.md は書かない）

## 観点 7: 走っている change との前提関係

観点 7: 該当なし（spec_overlap.py candidates が 0 件）。CH19（`emulator-startup`）・CH22（`distribution`）・CH23（`dependency-vulnerability-scan`）は `verify-environment` を持たない。
ファイルの上では、CH22（`origin/feat/ch22-soak-install-online-ci`）は `.github/workflows/ci.yml` の `rust` の job の `update-apply-install.sh --self-test`（149 行）の後に 1 行足し、CH21 が足す `check-private.sh --self-test`（158 行）の後とは別の位置。
CH23 は `docs/testing.md` を 2 行、CH22 は 7 行変える。CH21 の Task 9.2 は 1 節を足すだけで、置く位置は書かれていないが、同じ行を取り合うことは無い。Q8 で外につなぐ案が選ばれたときだけ CH22 が前提になる（design.md:74・proposal.md:49-51）
- 処置: fixed D15 仮 —— docs/changes/CH21.md を 「fix の道と違和感の issue の道は残す。依頼が足すのは fix の後に席の段落を差し替えることだけ（D15 の 9）」に直した

