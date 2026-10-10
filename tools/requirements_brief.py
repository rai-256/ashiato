#!/usr/bin/env python3
"""要件定義書（docs/requirements.md）の「わかりやすい版」を HTML で出す。

    python3 tools/requirements_brief.py                  # docs/briefs/requirements.html を書く
    python3 tools/requirements_brief.py -o out.html

2 種類の中身を混ぜずに出す（scripts/story_brief.py の考え方を借りる）:
- **機械で引くもの**: 各項目の原文（EARS の本文）と経緯（★ の注記）、どの Story が満たすか
  （docs/stories/INDEX.md）、その Story の進み具合（openspec/changes/ と archive/ の有無）
- **AI が書いたもの**: 平易な一行（PLAIN）とまとまりの説明（GROUPS）。
  画面の上で「AI の要約」と明示し、原文を開けば確かめられるようにする

「今回の改訂」は比べる版（既定 origin/main。`--base`）の requirements.md と項目ごとに突き合わせて機械で付ける
（無い = 新規、本文か経緯が違う = 改訂）。
要件の本文が変わっても原文は自動で追従する。一行の要約が無い項目は「要約なし」と出す（黙って落とさない）。
"""
import argparse
import html
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REQ = ROOT / "docs/requirements.md"
INDEX = ROOT / "docs/stories/INDEX.md"

# ---------------------------------------------------------------- AI が書いたもの

PLAIN = {
    "FR-1": "携帯が 60 秒ごとに位置（緯度経度・精度・時刻）を 1 件記録する。どの時計の時刻かも残す",
    "FR-2": "携帯のアプリ利用を 30 分ごとに取りに行く",
    "FR-84": "収集を始めた日に、OS に残っている過去のアプリ利用の集計（最大 2 年）を取る。放っておくと消える",
    "FR-85": "OS が既に消していて取れなかった期間は「残っていなかった」と記録する",
    "FR-3": "新しい写真ごとに、撮影日時・位置・機種・ハッシュを記録する",
    "FR-4": "写真の原本も自宅 PC（D-02）へ送る",
    "FR-5": "動画も同じように記録する（情報だけ）",
    "FR-6": "動画の原本は送らない（容量のため）",
    "FR-7": "携帯と PC の時計のずれを 1 時間ごとに測って残す。外部の時刻サーバには問い合わせない",
    "FR-8": "送れていない記録は携帯の中に 90 日・2 GB まで溜めておく",
    "FR-9": "上限を超えたら古い順に捨て、捨てた期間と件数を残す。上限の 7 日前に 1 回知らせる",
    "FR-10": "つながったら未送信を送る。断られたものは捨てて、捨てた事実を残す",
    "FR-11": "初回起動で健康データの履歴を読む権限を求める（遅れると 30 日より前が永久に読めない）",
    "FR-12": "PC で前面のアプリ・ウィンドウ題名・URL が変わるたびに 1 件記録する。URL は見えているまま",
    "FR-13": "ブラウザの履歴を 1 日 1 回、全プロファイルから訪問ごとに取る。消えた訪問は「消えた」と残す",
    "FR-81": "離席・画面ロック・スリープへの出入りを記録する（見ていた時間と離席を分けるため）",
    "FR-82": "PC が止まっていた期間を、次に起動したときに記録する",
    "FR-83": "除外に登録したもの（パスワード管理ソフトなど）は中身を記録せず、除外した事実と件数だけ残す",
    "FR-14": "置き場に Google Takeout の zip やマップのタイムラインを置くと読み込む。元のファイルは消さない",
    "FR-15": "アカウント系のソースを、ソースごとの間隔で定期的に取りに行く",
    "FR-16": "ソースごとに「どこまで取り込んだか」を持つ",
    "FR-17": "過去の書き出しを置いても、既にある記録と重複させずに取り込む",
    "FR-18": "受け取った原文をそのまま保存する。更新されたら前の版も残す",
    "FR-19": "記録ごとに「起きた時刻」と「入った時刻」の 2 つを持つ",
    "FR-20": "時刻にタイムゾーンも持たせる",
    "FR-21": "記録ごとに、送る側で一意な ID を振る",
    "FR-22": "同じ記録が再び届いても行を増やさない。更新なら前の版を残して書き換える",
    "FR-23": "外部サービスの ID を持たせる。ソースごとに「記録ごとの ID か、対象ごとの ID か」を宣言する",
    "FR-24": "記録ごとに、どのソース・端末・衛星が書いたかを持つ",
    "FR-25": "記録を「集めた／本人が書いた／計算で作った」に分ける。衛星の記録は、衛星の宣言どおり",
    "FR-26": "記録ごとに、書いたときのデータ形式の版を持つ",
    "FR-27": "文字は正規化（NFC）してから溜める",
    "FR-28": "数値には単位と座標系を持たせる",
    "FR-29": "すべての表に利用者 ID を持たせる",
    "FR-30": "集めた記録は書き換えない。例外は「版を残す更新」と「台帳つきの完全消去」だけで、DB が守る",
    "FR-31": "計算で作ったもの（滞在など）は原文から作り直せる。作り直しても同じものは同じ ID を保つ",
    "FR-32": "画面の文字抽出をするなら、使ったエンジンと版を残す",
    "FR-76": "半径 100 m に 10 分以上いたら「滞在」を 1 件作る。基準は後から変えて作り直せる",
    "FR-33": "記録が来たら、そのソースが動いていたことを残す",
    "FR-34": "自分で収集を止めたら、止めたソースと期間を残す",
    "FR-35": "想定の 3 倍の時間なにも来なければ知らせる",
    "FR-78": "収集が動いていて取れる状態か（権限・センサ・接続）を定期的に残す（生存信号）",
    "FR-79": "ソースごとに「収集を始めた日」を残す",
    "FR-80": "記録も生存信号も来ない期間を「途絶」として残す",
    "FR-36": "主観の記録は快–不快の 5 段階（-2〜+2）が必須",
    "FR-37": "主観は「日」か「滞在」に紐づける",
    "FR-38": "紐づけ先の種類は後から増やせる",
    "FR-39": "出来事の前に書いたか、後に書いたかを区別する",
    "FR-40": "追加の尺度を何個でも、尺度の ID と版つきで持てる",
    "FR-41": "対象の日とは別に、書いた日時を持つ",
    "FR-42": "過去の日付の主観も書ける",
    "FR-43": "決めた時刻に 1 日 1 回、書くよう促す通知を出す",
    "FR-44": "個人属性（住まいなど）は上書きせず追記する",
    "FR-45": "属性ごとに「言った日時」と「いつから有効か」を持つ",
    "FR-46": "人物を独立したものとして持つ",
    "FR-47": "滞在に「誰と一緒だったか」を紐づけられる",
    "FR-48": "場所（自宅など）を独立したものとして持つ。滞在がどの場所かは、読むたびに座標から計算する",
    "FR-49": "場所の ID は名前や座標が変わっても変えない。座標を変えるときは「直す」か「移った（いつから）」かを選ぶ",
    "FR-50": "削除は「消した印」を付ける。消した記録は、同じ内容が再び届いても入れない",
    "FR-51": "本文の完全消去もできる。消した事実・時刻・件数・範囲は台帳に残る",
    "FR-52": "完全消去の前に件数を見せて確認する",
    "FR-53": "収集をソースごと・期間指定で止められる",
    "FR-54": "ソースごと・日ごとに 8 つの状態（記録あり・記録なし・取れない状態・停止・破棄・途絶・導入前・退役）を見せる",
    "FR-55": "書庫のソースは「どこまで取り込んだか」と、直近に置いた書庫の結果を見せる",
    "FR-56": "日付を選ぶと、その日を滞在の単位で時刻順に見せる。アプリやサイトは要約と中身で",
    "FR-57": "主観が書かれていない日を区別して見せる",
    "FR-58": "語で検索できる（主観・場所名・人物・アプリ・題名・URL・写真の情報など）。日本語の区切り方は未決",
    "FR-59": "MCP サーバを同梱する。AI は衛星の 1 つで、つなぎ方はいったん MCP",
    "FR-60": "AI には、その AI の衛星に承認された種類のうち「出さない」印の無い記録だけを返す",
    "FR-61": "登録簿にソースを 1 行足せば、API を変えずにそのソースの記録を受け付ける",
    "FR-62": "衛星を登録するとき、読む種類・書く種類・書く記録の分類を宣言させる",
    "FR-63": "承認していない種類の記録は、その衛星に返さない（承認は持ち出しの許可も兼ねる）",
    "FR-64": "衛星が書けるのは、承認された種類で、自分が書いた記録の追加・変更・削除だけ",
    "FR-77": "衛星（AI を含む）とは、別プロセスから HTTP でだけやり取りする（ライセンスの境界）",
    "FR-65": "衛星の追加は、登録と承認だけで済む（本体の再起動は要らない）",
    "FR-66": "毎日、増分バックアップを取る",
    "FR-67": "毎週、完全バックアップを取る",
    "FR-68": "完全バックアップは暗号化して拠点外に送る",
    "FR-69": "暗号化は設定で切り替えられる",
    "FR-70": "拠点内に 30 日分、拠点外に 12 か月分を持つ",
    "FR-71": "6 か月ごとに復元テストを促す",
    "FR-72": "毎週、写真の原本と記録が食い違っていないか突き合わせる",
    "FR-73": "健康データを 1 日 1 回取り込む",
    "FR-74": "画面を一定間隔で撮る。間隔・保持期間・文字抽出のエンジンは未決",
    "FR-75": "意味で探す（埋め込み）。後から足しても過去の分が埋まる",
    "PERM-1": "利用者は D-01 ごとに 1 人。権限は人ではなく衛星ごとに分かれる",
    "PERM-2": "記録ごとに「どの衛星にも出さない」の印を付けられる。今ある記録は印なしで始める",
    "PERM-3": "外部 AI とローカル AI の 2 つの衛星を最初から置き、集めた記録・滞在・場所はどちらにも承認済みで始める",
    "PERM-4": "主観と個人属性は、最初はローカル AI にだけ承認",
    "PERM-5": "人物は、最初はどの衛星にも承認しない",
    "PERM-6": "写真・写真の位置・スクリーンショットは、最初はローカル AI にだけ承認",
    "PERM-7": "外からの接続は、許可した私設網（Tailscale など）の内側だけ",
    "PERM-8": "新しい衛星は、何も承認していない状態で始まる",
    "PERM-9": "承認を与えるときと「出さない」印を外すときは確認を求める",
    "PERM-10": "すべての API 呼び出しに資格情報を求める。資格情報は衛星ごとに発行する",
    "NFR-1": "位置とアプリ利用は 1 時間以内に D-01 に入る",
    "NFR-2": "健康データは 24 時間以内に入る",
    "NFR-3": "Takeout 系は遅延の目標を置かない（2 か月周期のため）",
    "NFR-4": "検索は 2 秒以内に返る",
    "NFR-5": "記録（位置・アプリ・健康・ウィンドウ・履歴・写真の情報）は年 3〜6 GB",
    "NFR-6": "写真の原本は年 6 GB 前後（動画は含めない）",
    "NFR-7": "携帯の溜め置きは 90 日・2 GB まで",
    "NFR-8": "開発費は月 2,000 円以内",
    "NFR-9": "運用コストは月 100 円以内（拠点外バックアップの分だけ）",
    "NFR-10": "6 か月ごとに復元テストをする",
    "NFR-11": "開発に使える時間は週 15 時間",
    "NFR-12": "手作業は Google 系の 2 か月に 1 回だけ。他はすべて自動",
    "NFR-13": "成功条件 1 の判定: 主要 5 ソースすべてが、365 日のうち 95 % 以上の日で取れていること",
    "NFR-14": "成功条件 2 の判定: 固定の 10 問を 6 か月ごとに AI に投げ、データが揃っているかと答えの満足度を見る",
    "NFR-17": "明暗は OS の設定に従う（取れなければダーク）",
    "NFR-18": "文字と背景のコントラストは 4.5:1 以上",
    "NFR-19": "指で触れる対象は 24×24 px 以上",
    "NFR-20": "取り返しのつかない操作のボタンは 44×44 px 以上",
    "NFR-23": "状態を表す色も、隣とのコントラストを 3:1 以上",
    "NFR-22": "キーボードで操作するとき、フォーカスの位置が見える",
    "NFR-21": "原色・派手な配色・角張った四角を使わない",
    "NFR-15": "データは自宅の中だけ。例外は暗号化したバックアップと、承認した種類を衛星が持ち出すこと",
    "NFR-16": "全記録を、外部の道具で読める形に書き出せる",
    "FR-91": "睡眠や読書のような幅のある出来事に、終わりの時刻も持たせられる",
    "FR-86": "本・旅程・費目のような衛星が持ち込むモノを、表を足さずに 1 つの汎用の台帳に入れる",
    "FR-87": "衛星が送ってきた自分の都合のデータ（積読の順番など）を取っておく。既定ではその衛星だけが読める",
    "FR-88": "承認の単位になる「種類」の一覧と、外部 AI・ローカル AI への最初の承認を表で持つ",
    "FR-89": "衛星向け API に版を付け、同じ版の中では衛星を壊さない",
    "FR-90": "衛星の登録を外すと、その資格情報は無効になる",
    "PERM-11": "衛星ごと・種類ごと・読む／書くごとに承認する（スマホの権限と同じ形）",
    "PERM-12": "承認の画面に「取り消しても、持ち出された写しは戻らない」と出す",
    "PERM-13": "どの衛星が・いつ・どの記録を読んだかを台帳に残す",
    "PERM-14": "資格情報はどれか 1 つの衛星に結び付け、その名前を出す（外部 AI とローカル AI を取り違えにくくする）",
    "NFR-24": "成功条件 3 の判定: 読書ログ・家計簿・睡眠の 3 衛星が、本体の表と列を変えずに読み書きできることを自動テストで確かめる",
    "FR-92": "書き手と利用者は、送られた申告でなくサーバが資格情報から決める",
    "FR-98": "衛星が場所の座標を変えるとき「直す／移った」の指定が無ければ断る",
    "FR-97": "衛星が更新で種類を増やしたら、増えた分だけ承認し直す",
    "FR-93": "モノの中身が登録した形に合わなければ断る",
    "FR-94": "衛星が足した新しい種類は、最初はその衛星だけが読める",
    "FR-95": "壊す変更は新しい版で出し、前の版を 6 か月以上動かす",
    "FR-96": "衛星の登録を外しても、その衛星が書いた記録は残る",
    "FR-99": "衛星の記録にも、時刻・識別子・文字の正規化の決まりを課す",
    "PERM-15": "印の付いた記録は、どの承認があっても衛星に返さない",
}

GROUPS = [
    ("collect-phone", "携帯で集める", "Android の収集アプリ（C-01）が、位置・アプリ利用・写真・動画を自動で集め、自宅 PC に届くまで端末に溜めておく。",
     ["FR-1", "FR-2", "FR-84", "FR-85", "FR-3", "FR-4", "FR-5", "FR-6", "FR-8", "FR-9", "FR-10", "FR-11", "FR-73"]),
    ("collect-pc", "PC で集める", "Windows の収集アプリ（C-02）が、前面のウィンドウ・ブラウザの履歴・離席を記録する。",
     ["FR-12", "FR-13", "FR-81", "FR-82", "FR-83"]),
    ("clock", "時計のずれ", "端末の時計が狂っていても後から補正できるよう、ずれそのものを測って残す。",
     ["FR-7"]),
    ("import", "過去のデータを取り込む", "Google のエクスポートなど、自動では取れないものを置き場から読み込む（C-03）。",
     ["FR-14", "FR-15", "FR-16", "FR-17"]),
    ("store", "記録の溜め方", "どの記録にも共通する決まり。後から取り返せない情報（原文・時刻・出所・版）を最初から持つ。",
     ["FR-18", "FR-19", "FR-20", "FR-91", "FR-21", "FR-22", "FR-23", "FR-24", "FR-92", "FR-25", "FR-26", "FR-27", "FR-28", "FR-29", "FR-30", "FR-32"]),
    ("stay", "滞在（計算で作るもの）", "位置の記録から「どこにどれだけいたか」を計算する。基準を変えれば作り直せる。",
     ["FR-76", "FR-31"]),
    ("coverage", "動いていたか（稼働状況）", "「記録が無い」が、何もしていなかったのか、壊れていたのか、自分で止めたのかを後から区別できるようにする。",
     ["FR-33", "FR-34", "FR-35", "FR-78", "FR-79", "FR-80"]),
    ("mood", "気分・主観", "その日や滞在に「どう感じたか」を毎日短く残す。",
     ["FR-36", "FR-37", "FR-38", "FR-39", "FR-40", "FR-41", "FR-42", "FR-43"]),
    ("entities", "属性・人物・場所", "自分の属性の移り変わり、一緒にいた人、名前を付けた場所を持つ。",
     ["FR-44", "FR-45", "FR-46", "FR-47", "FR-48", "FR-49", "FR-98"]),
    ("delete", "消す・止める", "記録を消したことにする・本当に消す・収集を止める。",
     ["FR-50", "FR-51", "FR-52", "FR-53"]),
    ("view", "画面で見る・探す", "本体の画面（V-01）。稼働状況・1 日の並び・検索。",
     ["FR-54", "FR-55", "FR-56", "FR-57", "FR-58", "FR-75"]),
    ("connect", "衛星をつなぐ", "本体の外で作る衛星アプリが、承認された分だけデータを読み書きする仕組み。衛星が増えても本体の表は変えない。",
     ["FR-61", "FR-62", "FR-97", "FR-63", "FR-64", "FR-65", "FR-77", "FR-99", "FR-86", "FR-93", "FR-87", "FR-88", "FR-94", "FR-89", "FR-95", "FR-90", "FR-96"]),
    ("ai", "AI から問う", "AI も衛星の 1 つ。MCP で問い合わせ、その AI に承認された分だけ返す。",
     ["FR-59", "FR-60"]),
    ("backup", "バックアップ", "毎日・毎週の写しと、拠点外への暗号化した写し。",
     ["FR-66", "FR-67", "FR-68", "FR-69", "FR-70", "FR-71", "FR-72"]),
    ("future", "将来（Could）", "いまは作らないが、取り返せない部分だけ先に決めてあるもの。",
     ["FR-74"]),
    ("perm", "承認と権限", "衛星ごと・種類ごとに承認し、記録ごとには「出さない」の印だけを持つ。新しい衛星は何も承認しない状態で始まる。",
     ["PERM-1", "PERM-11", "PERM-2", "PERM-15", "PERM-8", "PERM-3", "PERM-4", "PERM-5", "PERM-6", "PERM-9", "PERM-12", "PERM-13", "PERM-10", "PERM-14", "PERM-7"]),
    ("nfr-speed", "速さと容量", "",
     ["NFR-1", "NFR-2", "NFR-3", "NFR-4", "NFR-5", "NFR-6", "NFR-7"]),
    ("nfr-cost", "お金と時間", "",
     ["NFR-8", "NFR-9", "NFR-10", "NFR-11", "NFR-12"]),
    ("nfr-success", "成功の判定", "",
     ["NFR-13", "NFR-14", "NFR-24"]),
    ("nfr-look", "見た目と触れやすさ", "",
     ["NFR-17", "NFR-18", "NFR-19", "NFR-20", "NFR-23", "NFR-22", "NFR-21"]),
    ("nfr-keep", "保全", "",
     ["NFR-15", "NFR-16"]),
]

# ---------------------------------------------------------------- 機械で引くもの

HEAD = re.compile(r"^(#{2,4}) (.*)")
ITEM = re.compile(r"^- \*\*((?:FR|NFR|PERM)-\d+)\*\*[:：]\s*(.*)")
DOOR = re.compile(r"^(\d+)\. \*\*(.+?)\*\*(.*)")


def parse_requirements(text):
    items, cur, sec, in_doors = {}, None, [], False
    order = []
    for no, line in enumerate(text.splitlines(), 1):
        m = HEAD.match(line)
        if m:
            sec = sec[: len(m.group(1)) - 2] + [m.group(2).strip()]
            in_doors = sec[-1].startswith("一方通行の扉")
            cur = None
            continue
        m = ITEM.match(line)
        if m:
            cur = {"id": m.group(1), "line": no, "body": [m.group(2)], "notes": [], "note": False}
            items[cur["id"]] = cur
            order.append(cur["id"])
            continue
        if in_doors:
            m = DOOR.match(line)
            if m:
                cur = {"id": f"扉{m.group(1)}", "line": no, "title": m.group(2),
                       "body": [m.group(3).strip(" —-")], "notes": [], "note": False}
                items[cur["id"]] = cur
                order.append(cur["id"])
                continue
        if cur is None:
            continue
        if line.startswith("- ") or line.startswith("#") or line.strip() == "---":
            cur = None
            continue
        s = line.strip()
        if not s:
            continue
        if s.startswith("★") or cur["note"]:
            cur["note"] = True
            cur["notes"].append(s)
        else:
            cur["body"].append(s)
    return items, order


def section(text, start, stop):
    out, on = [], False
    for line in text.splitlines():
        if line.startswith(start):
            on = True
            continue
        if on and stop(line):
            break
        if on:
            out.append(line)
    return "\n".join(out).strip()


def base_items(ref):
    """比べる版の requirements.md を項目に分ける。取れなければ None（印を付けない）。"""
    try:
        text = subprocess.run(["git", "show", f"{ref}:docs/requirements.md"], cwd=ROOT,
                              capture_output=True, text=True, check=True).stdout
    except subprocess.CalledProcessError:
        return None
    return parse_requirements(text)[0]


def diff_kind(it, base):
    if base is None:
        return None
    old = base.get(it["id"])
    if old is None:
        return "new"
    if old["body"] != it["body"] or old["notes"] != it["notes"]:
        return "changed"
    return None


def story_map():
    """INDEX.md の一覧表から「要件 ID → Story」と Story の表題を引く。"""
    req2st, titles = {}, {}
    for line in INDEX.read_text(encoding="utf-8").splitlines():
        m = re.match(r"^\| \[(ST\d+)\]\(ST\d+\.md\) \| (.+?) \| \d+ \| (.*?) \|", line)
        if not m:
            continue
        st, title, sat = m.groups()
        titles[st] = title
        for rid in re.findall(r"(?:FR|NFR|PERM)-\d+", sat):
            req2st.setdefault(rid, []).append(st)
    return req2st, titles


def story_status():
    """origin/main の openspec/changes/ を見る。archive にあれば済み、changes にあれば作業中。"""
    try:
        names = subprocess.run(
            ["git", "ls-tree", "--name-only", "origin/main", "openspec/changes/", "openspec/changes/archive/"],
            cwd=ROOT, capture_output=True, text=True, check=True).stdout.split()
    except subprocess.CalledProcessError:
        names = [str(p.relative_to(ROOT)) for p in (ROOT / "openspec/changes").glob("**/st*")]
    status = {}
    for n in names:
        m = re.search(r"(?:^|/)(?:\d{4}-\d{2}-\d{2}-)?st(\d+)-", n)
        if not m:
            continue
        st = f"ST{int(m.group(1)):02d}"
        status[st] = "done" if "/archive/" in n else status.get(st, "wip")
    return status


# ---------------------------------------------------------------- 書く

def inline(s):
    s = html.escape(s)
    s = re.sub(r"\*\*(.+?)\*\*", r"<b>\1</b>", s)
    s = re.sub(r"`(.+?)`", r"<code>\1</code>", s)
    return s


ST_LABEL = {"done": "済", "wip": "作業中", None: "未着手"}


def render_item(it, req2st, titles, status, base):
    rid = it["id"]
    plain = PLAIN.get(rid)
    sts = req2st.get(rid, [])
    states = [status.get(s) for s in sts]
    if sts and all(x == "done" for x in states):
        st_key = "done"
    elif any(x in ("done", "wip") for x in states):
        st_key = "wip"
    elif sts:
        st_key = "todo"
    else:
        st_key = "none"
    rev = diff_kind(it, base)
    chips = []
    for s in sts:
        lab = ST_LABEL.get(status.get(s))
        chips.append(f'<span class="st st-{status.get(s) or "todo"}" title="{html.escape(titles.get(s, ""))}">{s} {lab}</span>')
    if not sts:
        chips.append('<span class="st st-none">Story なし</span>')
    body = "<br>".join(inline(b) for b in it["body"])
    notes = ""
    if it["notes"]:
        notes = (f'<details class="hist"><summary>経緯 {len(it["notes"])} 行</summary>'
                 f'<div>{"<br>".join(inline(n) for n in it["notes"])}</div></details>')
    rev_html = ""
    if rev:
        chips.insert(0, f'<span class="tag tag-rev">{"新規" if rev == "new" else "改訂"}</span>')
    plain_html = html.escape(plain) if plain else '<span class="missing">要約なし（原文を開いて読む）</span>'
    filt = " ".join(filter(None, [f"s-{st_key}", "s-rev" if rev else ""]))
    return f'''<li class="item {filt}" id="{rid}">
<div class="row"><span class="rid">{rid}</span><p class="plain">{plain_html}</p></div>
<div class="meta">{"".join(chips)}</div>
<details class="orig"><summary>原文（requirements.md:{it["line"]}）</summary><div class="verbatim">{body}</div>{notes}</details>
</li>'''


def build(out, ref):
    text = REQ.read_text(encoding="utf-8")
    items, order = parse_requirements(text)
    base = base_items(ref)
    req2st, titles = story_map()
    status = story_status()

    known = {rid for _, _, _, ids in GROUPS for rid in ids}
    stray = [r for r in order if not r.startswith("扉") and r not in known]
    missing = [r for r in known if r not in items]

    groups_html = []
    for key, name, desc, ids in GROUPS:
        lis = "\n".join(render_item(items[r], req2st, titles, status, base) for r in ids if r in items)
        d = f'<p class="gdesc">{html.escape(desc)}</p>' if desc else ""
        groups_html.append(f'<section class="group" id="g-{key}"><h3>{html.escape(name)} <span class="count">{len(ids)}</span></h3>{d}<ul class="items">{lis}</ul></section>')
    if stray:
        lis = "\n".join(render_item(items[r], req2st, titles, status, base) for r in stray)
        groups_html.append(f'<section class="group"><h3>まとまりに入っていない項目 <span class="count">{len(stray)}</span></h3><ul class="items">{lis}</ul></section>')

    doors = [items[r] for r in order if r.startswith("扉")]
    door_lis = []
    for d in doors:
        body = " ".join(d["body"])
        m = re.search(r"決着状態[:：]\s*(\S+?)（", body) or re.search(r"決着状態[:：]\s*(\S+)", body)
        state = m.group(1) if m else "—"
        cls = "done" if state.startswith("決定済") else "open"
        door_lis.append(f'<li><span class="rid">{d["id"]}</span><span class="dt">{inline(d["title"])}</span>'
                        f'<span class="tag tag-{cls}">{html.escape(state)}</span>'
                        f'<details class="orig"><summary>原文（:{d["line"]}）</summary><div class="verbatim">{inline(body)}</div></details></li>')

    purpose = section(text, "### 1.1", lambda l: l.startswith("### 1.2"))
    success = re.findall(r"^\d\. \*\*(.+?)\*\*", purpose, re.M)
    qs = re.findall(r"^\| (QS-\d+) \| (.+?) \|", text, re.M)
    out_of_scope = section(text, "### 範囲外", lambda l: l.startswith("### "))
    oos = [re.sub(r"\*\*", "", l[2:]).split("。")[0] for l in out_of_scope.splitlines() if l.startswith("- ")]

    counts = {k: sum(1 for r in order if r.startswith(k + "-")) for k in ("FR", "NFR", "PERM")}
    revs = [(r, diff_kind(items[r], base)) for r in order]
    revs = [(r, k) for r, k in revs if k]
    n_rev = len(revs)
    total = counts["FR"] + counts["NFR"] + counts["PERM"]
    done_ids = sum(1 for r in order if not r.startswith("扉") and req2st.get(r) and all(status.get(s) == "done" for s in req2st[r]))

    new_lis = "".join(
        f'<li><a class="rid" href="#{r}">{r}</a><span><span class="tag tag-rev">{"新規" if k == "new" else "改訂"}</span>'
        f'{html.escape(PLAIN.get(r, items[r].get("title", "")))}</span></li>'
        for r, k in revs) or "<li><span>比べる版との違いはありません</span></li>"
    qs_lis = "".join(f'<li><span class="rid">{a}</span>{html.escape(b)}</li>' for a, b in qs)
    oos_lis = "".join(f"<li>{html.escape(x)}</li>" for x in oos)
    succ_lis = "".join(f"<li>{html.escape(x)}</li>" for x in success)
    warn = ""
    if stray or missing:
        warn = f'<p class="note warn">生成の注意: まとまりに無い項目 {", ".join(stray) or "なし"} ／ 要件に無い ID {", ".join(missing) or "なし"}</p>'

    page = TEMPLATE.format(
        total=total, fr=counts["FR"], nfr=counts["NFR"], perm=counts["PERM"], doors=len(doors),
        n_rev=n_rev, done=done_ids, ref=html.escape(ref), lines=len(text.splitlines()),
        succ=succ_lis, qs=qs_lis, new=new_lis, groups="\n".join(groups_html),
        door_list="\n".join(door_lis), oos=oos_lis, warn=warn,
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(page, encoding="utf-8")
    print(f"{out}  項目 {total}・扉 {len(doors)}・{ref} からの改訂 {n_rev}・要約なし {sum(1 for r in order if not r.startswith('扉') and r not in PLAIN)}")


TEMPLATE = """<title>あしあとの要件</title>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Zen+Kaku+Gothic+New:wght@500;700&family=Noto+Sans+JP:wght@400;500;700&family=IBM+Plex+Mono:wght@400;500&display=swap">
<style>
/* 1 段組。上から「何のためか → 何が変わるか → 機能のまとまり → 扉 → 範囲外」。項目は畳んだ原文を持つ行 */
:root{{
  --bg:#f3f6f4; --surface:#ffffff; --fg:#1d2621; --muted:#5d6b63; --line:#d5ddd8;
  --accent:#2e6b4f; --accent-soft:#e2efe8;
  --done:#2e7d4f; --done-soft:#e1f1e6; --wip:#2f5fa8; --wip-soft:#e3ebf8;
  --todo:#6b7570; --todo-soft:#eceeed; --rev:#a5560a; --rev-soft:#fbecd8;
  --display:"Zen Kaku Gothic New","Hiragino Sans","Noto Sans JP",sans-serif;
  --body:"Noto Sans JP","Hiragino Sans","Yu Gothic",sans-serif;
  --mono:"IBM Plex Mono",ui-monospace,Menlo,monospace;
}}
@media (prefers-color-scheme: dark){{:root:not([data-theme="light"]){{
  --bg:#121815; --surface:#1a221e; --fg:#e2ebe6; --muted:#97a69e; --line:#2b3731;
  --accent:#6cc79a; --accent-soft:#1b3328;
  --done:#6fcf97; --done-soft:#183024; --wip:#8db3f0; --wip-soft:#1d2a40;
  --todo:#a3aea8; --todo-soft:#252d29; --rev:#f0a85a; --rev-soft:#3a2a14; color-scheme:dark}}}}
:root[data-theme="dark"]{{
  --bg:#121815; --surface:#1a221e; --fg:#e2ebe6; --muted:#97a69e; --line:#2b3731;
  --accent:#6cc79a; --accent-soft:#1b3328;
  --done:#6fcf97; --done-soft:#183024; --wip:#8db3f0; --wip-soft:#1d2a40;
  --todo:#a3aea8; --todo-soft:#252d29; --rev:#f0a85a; --rev-soft:#3a2a14; color-scheme:dark}}
*{{box-sizing:border-box}}
body{{background:var(--bg);color:var(--fg);font-family:var(--body);font-size:15px;line-height:1.75;margin:0}}
.wrap{{max-width:780px;margin:0 auto;padding-inline:16px;padding-block:28px 72px}}
h1,h2,h3{{font-family:var(--display);text-wrap:balance;line-height:1.4}}
h1{{font-size:1.7rem;margin:0 0 .3em}}
h2{{font-size:1.25rem;margin:2.2em 0 .6em;padding-bottom:.3em;border-bottom:1px solid var(--line)}}
h3{{font-size:1.05rem;margin:0}}
p{{margin:.45em 0}}
.lead{{color:var(--muted)}}
.note{{font-size:.84rem;color:var(--muted);background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:10px 14px}}
.warn{{color:var(--rev);border-color:var(--rev)}}
code{{font-family:var(--mono);font-size:.85em;background:var(--accent-soft);padding:0 4px;border-radius:4px}}
.stats{{display:flex;flex-wrap:wrap;gap:8px 18px;margin:14px 0;font-size:.88rem;color:var(--muted)}}
.stats b{{font-family:var(--mono);font-size:1.05rem;color:var(--fg);font-variant-numeric:tabular-nums}}
.cols{{display:grid;grid-template-columns:1fr 1fr;gap:12px}}
@media (max-width:600px){{.cols{{grid-template-columns:1fr}}}}
.card{{background:var(--surface);border:1px solid var(--line);border-radius:10px;padding:14px 16px;min-width:0}}
.card h3{{font-size:.95rem;margin-bottom:.4em}}
.card ol,.card ul{{margin:0;padding-left:1.2em}}
.revbox{{background:var(--rev-soft);border:1px solid var(--rev);border-radius:10px;padding:14px 16px}}
.revbox h3{{color:var(--rev)}}
.revbox ul{{list-style:none;margin:8px 0 0;padding:0;display:grid;gap:8px}}
.revbox li{{display:grid;grid-template-columns:5.2em 1fr;gap:10px;font-size:.9rem}}
.revbox a.rid{{color:var(--rev)}}
@media (max-width:600px){{.revbox li{{grid-template-columns:1fr;gap:0}}}}
.qs{{list-style:none;padding:0;margin:0;display:grid;gap:4px;font-size:.88rem}}
.qs .rid{{margin-right:8px}}
.filter{{position:sticky;top:env(safe-area-inset-top,0px);z-index:2;background:var(--bg);padding-block:10px;display:flex;flex-wrap:wrap;gap:6px;border-bottom:1px solid var(--line)}}
.filter button{{font:inherit;font-size:.82rem;padding:4px 12px;border-radius:999px;border:1px solid var(--line);background:var(--surface);color:var(--fg);cursor:pointer;min-height:32px}}
.filter button[aria-pressed="true"]{{background:var(--accent);border-color:var(--accent);color:var(--bg)}}
.filter button:focus-visible,summary:focus-visible{{outline:2px solid var(--accent);outline-offset:2px}}
.toc{{display:flex;flex-wrap:wrap;gap:4px 12px;font-size:.84rem;margin:10px 0 0}}
.toc a{{color:var(--accent)}}
.group{{margin-top:26px}}
.group h3 .count{{font-family:var(--mono);font-size:.8rem;color:var(--muted);font-weight:400}}
.gdesc{{color:var(--muted);font-size:.9rem}}
.items{{list-style:none;margin:8px 0 0;padding:0;display:grid;gap:8px}}
.item{{background:var(--surface);border:1px solid var(--line);border-radius:10px;padding:10px 14px}}
.row{{display:grid;grid-template-columns:5.2em 1fr;gap:8px;align-items:baseline}}
.rid{{font-family:var(--mono);font-size:.78rem;color:var(--muted);white-space:nowrap}}
.plain{{margin:0;min-width:0}}
.missing{{color:var(--rev)}}
.meta{{display:flex;flex-wrap:wrap;gap:4px;margin:6px 0 0 5.2em}}
@media (max-width:480px){{.row{{grid-template-columns:1fr;gap:0}}.meta{{margin-left:0}}}}
.st,.tag{{font-size:.72rem;font-weight:700;padding:1px 8px;border-radius:999px;white-space:nowrap}}
.st-done{{background:var(--done-soft);color:var(--done)}}
.st-wip{{background:var(--wip-soft);color:var(--wip)}}
.st-todo,.st-none{{background:var(--todo-soft);color:var(--todo)}}
.tag-rev{{background:var(--rev);color:var(--bg);margin-right:8px}}
.tag-done{{background:var(--done-soft);color:var(--done)}}
.tag-open{{background:var(--rev-soft);color:var(--rev)}}
.rev{{font-size:.86rem;color:var(--rev);margin:8px 0 0;padding:6px 10px;background:var(--rev-soft);border-radius:6px}}
details.orig{{margin-top:6px}}
details summary{{cursor:pointer;font-size:.8rem;color:var(--accent);min-height:24px}}
.verbatim{{font-size:.86rem;background:var(--bg);border-radius:6px;padding:8px 10px;margin-top:4px;overflow-wrap:anywhere}}
details.hist summary{{color:var(--muted)}}
details.hist div{{font-size:.8rem;color:var(--muted);padding:6px 10px;overflow-wrap:anywhere}}
.doors{{list-style:none;padding:0;margin:0;display:grid;gap:6px}}
.doors li{{background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:8px 12px;display:flex;flex-wrap:wrap;gap:4px 10px;align-items:baseline}}
.doors .dt{{flex:1 1 16em;min-width:0}}
.doors details{{flex-basis:100%}}
.oos li{{margin:.3em 0}}
</style>

<div class="wrap">
<h1>あしあとの要件</h1>
<p class="lead">要件定義書（<code>docs/requirements.md</code>・{lines} 行）を、まとまりごとに一行で読めるようにしたものです。</p>
<p class="note"><b>読み方</b>：一行の要約とまとまりの説明は <b>AI が書いた</b>ものです。正しいのは原文で、各項目の「原文」を開くと逐語で確かめられます。どの Story が満たすか（INDEX.md）・その進み具合（main の openspec）・「新規／改訂」の印（<code>{ref}</code> の本文との突き合わせ）は機械で引いています。</p>
{warn}
<div class="stats"><span>要件 <b>{total}</b>（機能 {fr}・非機能 {nfr}・権限 {perm}）</span><span>一方通行の扉 <b>{doors}</b></span><span>満たす Story が全部済み <b>{done}</b></span><span>今回の改訂 <b>{n_rev}</b></span></div>

<h2>何のためのものか（いまの本文）</h2>
<div class="cols">
<div class="card"><h3>目的</h3><p><b>衛星アプリを作るための基盤。</b>AI はその衛星の 1 つ。</p><p>その上で、全記録を AI に読ませ、自分についてメタ分析し、理解を深め、行動し、幸せになること。意思決定の例は旅行・買い物・遊び。</p></div>
<div class="card"><h3>成功の定義</h3><ol>{succ}</ol></div>
</div>
<details class="card" style="margin-top:12px"><summary>成功条件 2 の質問 10 問（2026-09-07 固定）</summary><ul class="qs" style="margin-top:8px">{qs}</ul></details>

<h2>今回変わったところ</h2>
<div class="revbox">
<h3>「衛星アプリの基盤」を主にする改訂（2026-10-11）</h3>
<p style="font-size:.88rem">従来の本文は「AI に読ませる記録庫。衛星アプリは範囲外」という向きでした。<code>{ref}</code> の本文と比べて、足した・書き換えた項目が次のとおりです（一行は AI の要約。押すと項目へ飛びます）。下の一覧でも同じ印が付いています。</p>
<ul>{new}</ul>
</div>

<h2>中身</h2>
<div class="filter" role="group" aria-label="絞り込み">
<button type="button" data-f="all" aria-pressed="true">すべて</button>
<button type="button" data-f="s-done" aria-pressed="false">済</button>
<button type="button" data-f="s-wip" aria-pressed="false">作業中</button>
<button type="button" data-f="s-todo" aria-pressed="false">未着手</button>
<button type="button" data-f="s-none" aria-pressed="false">Story なし</button>
<button type="button" data-f="s-rev" aria-pressed="false">今回の改訂</button>
</div>
<nav class="toc">{toc}</nav>
{groups}

<h2>一方通行の扉（後から変えられない決定）</h2>
<p class="gdesc">一度決めて溜め始めると、後から変えても過去の分が取り返せない決定です。題名と決着は原文から引いています。</p>
<ul class="doors">{door_list}</ul>

<h2>やらないこと（範囲外）</h2>
<ul class="oos">{oos}</ul>
</div>

<script>
(function(){{
  var btns=document.querySelectorAll('.filter button');
  function apply(f){{
    btns.forEach(function(b){{b.setAttribute('aria-pressed', String(b.dataset.f===f));}});
    document.querySelectorAll('.item').forEach(function(li){{li.hidden = !(f==='all' || li.classList.contains(f));}});
    document.querySelectorAll('.group').forEach(function(g){{
      g.hidden = !g.querySelector('.item:not([hidden])');
    }});
  }}
  btns.forEach(function(b){{b.addEventListener('click', function(){{apply(b.dataset.f);}});}});
}})();
</script>
"""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-o", "--out", default=str(ROOT / "docs/briefs/requirements.html"))
    ap.add_argument("--base", default="origin/main", help="「今回の改訂」の印を付けるために比べる版")
    a = ap.parse_args()
    global TEMPLATE
    toc = " ".join(f'<a href="#g-{k}">{html.escape(n)}</a>' for k, n, _, _ in GROUPS)
    TEMPLATE = TEMPLATE.replace("{toc}", toc.replace("{", "{{").replace("}", "}}"))
    build(Path(a.out), a.base)


if __name__ == "__main__":
    main()
