// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 表面のトークン。**`docs/ui-direction.md` の「★ 2026-10-12 表面を作り直した」から引く 1 か所**（design D12）。
 *
 * 色・形・密度・文字の値は、本人が `docs/ui-surface-playground.html` で選んだ設定から
 * playground と**同じ式**（`buildTones` / `sizes`）で導いた値をそのまま置く。導出をここで
 * やり直さないのは、描く色を 1 つの表に固定し、検査（`surface-limits.test.ts`）がその表の
 * **全部の組**を測るため —— 測る色と描く色を別々の場所で決めない（UIR-13 / UIR-61）。
 *
 * **画面ごとに色を足さない** —— 足すと ST25（S-2 主表現）が別のトークン系を持つ。
 */

/** 色の 1 つ。`[色相, 彩度(%), 明るさ(%)]`。描くときは `tone()` で文字列にする。 */
export type Hsl = readonly [hue: number, sat: number, light: number];

/**
 * 色は 3 本（本人の決定 2026-10-12）。
 * - 地と面と文字の色み: 若葉〜青みの緑。**文字の色みも同じ彩度**
 * - アクセント: はっきりした紫。**量は見出しまで**（題・リンク）と主ボタンの塗り
 * - 差し色: 若葉〜青みの緑。**「確定まであと N 日」のような小さな注意と移動のしるしにだけ**
 */
export const SURFACE_HUE = 140;
export const SURFACE_SAT = 18;
export const ACCENT_HUE = 307;
export const ACCENT_SAT = 42;
/** 差し色の彩度はアクセントと同じ（playground の式）。 */
export const SPARK_HUE = 150;

const surface = (l: number): Hsl => [SURFACE_HUE, SURFACE_SAT, l];
const accent = (l: number): Hsl => [ACCENT_HUE, ACCENT_SAT, l];
const spark = (l: number): Hsl => [SPARK_HUE, ACCENT_SAT, l];
/** 塗りの上の文字（アクセントの塗りの上で 4.5:1 を満たす側として導かれた白）。 */
const WHITE: Hsl = [0, 0, 100];

/**
 * 明暗の 2 組（NFR-17）。**OS の明暗に追従し、取得できないときはダーク**（`scheme.ts`）。
 *
 * - 面は 3 段（地 → 行・節 → 詳細・ボタン）。ライトは地 99 から 4 ずつ沈め、ダークは地 13 から 6 ずつ浮かせる
 * - 文字（本文・控えめ・アクセント・差し色）は**選んだ色相と彩度のまま、明暗だけを寄せて**
 *   3 段の面すべてに 4.5:1 を満たす最初の明るさ（NFR-18）。寄せたのは
 *   ライトの差し色（40 → 32.5）とダークのアクセント（70 → 74.5）
 * - `line` は区切りの線（装飾。意味は担わせない —— 意味を担う縁は `muted` で引く）
 * - `cell` は稼働状況の格子の 3 段（下の「格子の 3 段」）
 */
export const SCHEMES = {
  light: {
    ground: surface(99),
    surface1: surface(95),
    surface2: surface(91),
    line: surface(82),
    text: surface(14),
    muted: surface(30),
    accent: accent(36),
    accentFill: accent(38),
    onAccent: WHITE,
    spark: spark(32.5),
    cell: { other: accent(95), alive_no_record: accent(56), recorded: accent(22) },
  },
  dark: {
    ground: surface(13),
    surface1: surface(19),
    surface2: surface(25),
    line: surface(31),
    text: surface(92),
    muted: surface(72),
    accent: accent(74.5),
    accentFill: accent(32),
    onAccent: WHITE,
    spark: spark(72),
    cell: { other: accent(9), alive_no_record: accent(49.5), recorded: accent(92) },
  },
} as const satisfies Record<string, SchemeColors>;

/** 1 組の色の形。 */
export type SchemeColors = {
  ground: Hsl;
  surface1: Hsl;
  surface2: Hsl;
  line: Hsl;
  text: Hsl;
  muted: Hsl;
  accent: Hsl;
  accentFill: Hsl;
  onAccent: Hsl;
  spark: Hsl;
  cell: Record<"other" | "alive_no_record" | "recorded", Hsl>;
};

export type Scheme = keyof typeof SCHEMES;

/**
 * 格子のセルが担う **3 段**（design D10 / 深掘り 第 5 回 Q21）。色は `SCHEMES[scheme].cell`。
 *
 * **7 段にしない。** WCAG SC 1.4.11（NFR-23）は隣接する色に 3:1 を求めるが、
 * 7 段を隣接 3:1 で並べると 3^6 = 729:1 が要り、**sRGB の理論最大は 21:1**。
 * 8 状態の区別は**週を選んだときの文字**が担う。
 *
 * 段はアクセントの色相で、両端（ライト 95 / 22、ダーク 9 / 92）の間に中の段を
 * 輝度の幾何平均で置く（playground の式）。隣接比はライト 3.33 / 3.35、ダーク 3.82 / 3.89
 * （`surface-limits.test.ts` が値から計算して確かめる。ここに書いた数を信じない）。
 * 「それ以外」の段は節の面に近いので、**`muted` の 1 px の縁**を付けて広がりを読ませる。
 */
export const BAND_NAMES = ["other", "alive_no_record", "recorded"] as const;

/** HSL の文字列。**色はここを通してしか描かない。** */
export function tone([h, s, l]: Hsl): string {
  return `hsl(${h} ${s}% ${l}%)`;
}

/**
 * 文字（本人の決定 2026-10-12）。本文 15.5 px・行間 1.70・見出しと本文の比 1.18・太さ 600。
 * 書体は 9/8 から変えていない（角ゴシック。端末にあるものだけを使う —— 外から読み込まない）。
 * `Noto Sans CJK JP` は `Noto Sans JP` と同じ書体の、Linux・Android（と CI の e2e）での名前。
 * 無いと system-ui に落ちて欧文が DejaVu Sans になり、要約が折り返して行が伸びる（ST25 design D24（仮））。
 */
const BODY_PX = 15.5;
const RATIO = 1.18;
const round1 = (n: number): number => Math.round(n * 10) / 10;
export const TYPE = {
  family: '"Hiragino Sans", "Hiragino Kaku Gothic ProN", "Noto Sans JP", "Noto Sans CJK JP", "Yu Gothic UI", Meiryo, system-ui, sans-serif',
  body: BODY_PX,
  lineHeight: 1.7,
  ratio: RATIO,
  /** 見出しの太さ */
  weight: 600,
  /** 題（h1）= 本文 × 比² */
  h1: round1(BODY_PX * RATIO * RATIO),
  /** 見出し（h2）= 本文 × 比 */
  h2: round1(BODY_PX * RATIO),
  /** 添える文字（ボタン・表・日付）= 本文 ÷ 比（12 px を下回らない） */
  caption: round1(Math.max(12, BODY_PX / RATIO)),
} as const;

/** `font` の短縮記法。行の高さは数値で書く（`one-scroll.test.tsx` が `px/行間` を読んで勘定する）。 */
export function font(size: number, weight: 400 | 600 = 400, lineHeight: number = TYPE.lineHeight): string {
  return `${weight} ${size}px/${lineHeight} ${TYPE.family}`;
}

/**
 * 密度 66（ゆったりした側。9/8 は 75）。余白と行の間隔を**まとめて比例で**動かす（playground の `sizes()`）。
 * **横に並ぶ操作の間は 8 px を下回らない**（`targetGap`）。
 */
export const DENSITY = 66;
const GAP = 6 + DENSITY * 0.24;

/** 角（本人の決定 2026-10-12）。節・カードは 12 px、行はその 0.55 倍の 7 px。 */
export const RADIUS = {
  section: 12,
  row: Math.max(4, Math.round(12 * 0.55)),
  /** ボタン・入力欄（角の 0.55 倍。丸い四角） */
  control: round1(Math.max(4, 12 * 0.55)),
  /** 格子のセル */
  cell: 3,
} as const;

export const SPACE = {
  gap: round1(GAP),
  /** 画面の内側の余白 */
  page: round1(Math.max(12, GAP * 0.8)),
  /** 行の内側の余白（縦・横）。角の丸みより細くしない */
  rowY: round1(Math.max(GAP * 0.42, RADIUS.row * 0.45)),
  rowX: round1(Math.max(GAP * 0.55, RADIUS.row * 0.7)),
  /** 行どうしの間 */
  rowGap: 6,
  /** 行の中のしるしと文字の間 */
  inRow: round1(GAP * 0.45),
  /** 横に並ぶ操作の間（8 px を下回らない） */
  targetGap: round1(Math.max(8, GAP * 0.35)),
  /** 題の下 */
  underTitle: round1(GAP * 0.35),
  /** 操作の並びの下 */
  underTools: round1(GAP * 0.5),
} as const;

/** 種類のしるし（線の輪の中に記号）の直径。本文の 1.5 倍、24 px を下回らない。 */
export const MARK_PX = Math.max(24, BODY_PX * 1.5);
/** フォーカスの輪の太さ（NFR-22。本文の色・3 px のまま）。 */
export const FOCUS_RING_PX = 3;

/**
 * 操作できるものの最小の大きさ（NFR-19）。
 *
 * **週の帯がこれを満たす。セルは操作対象にしない**（深掘り 第 4 回 Q16）——
 * 1 週 7 日を横に並べると 1 セルが 24 px に満たない幅になりうる。
 * 選ぶ単位を週にすれば、セルの大きさに関わらず NFR-19 を満たせる。
 */
export const MIN_TARGET_PX = 24;

/** 消去のような破壊的操作の最小の大きさ（D9）。 */
export const DESTRUCTIVE_TARGET_PX = 44;

/**
 * 日の区切り。**サーバの `DAY_TZ`（`crates/server/src/coverage.rs`）と同じでなければならない**
 * （深掘り Q2 / design D1）。
 *
 * 画面が UTC で日を切っていたときは、**JST の 00:00〜09:00 のあいだ今日が格子に出ず**、
 * 格子の窓と達成の窓が 1 日ずれた（review/code.md の R10）。
 * 「日を引く場所がアプリと SQL に割れると、片方だけずれても誰も気付かない」——
 * サーバ側の `facts()` が書いているのと同じ割れ方が、画面側に残っていた。
 */
export const DAY_TZ = "Asia/Tokyo";

/**
 * 開いた直後に見せる週数（深掘り 第 7 回 Q28 は「直近 4〜5 週」）。
 *
 * **5 ではなく 4**（第 8 回 Q30）。Q28 の根拠にした 600 px は
 * `5 ソース × 5 行 × 24 px` で**セルだけを積んだ勘定**で、見出し・余白・ボタン・
 * 達成の表を数えていなかった。実際に宣言されている箱を積むと約 1,491 px あり、
 * **ひとスクロール（2 画面 = 1,280 px）に収まらない**。
 * 5 → 4 で 5 ソースぶん 130 px を返す。`one-scroll.test.tsx` が勘定を固定する。
 */
export const INITIAL_WEEKS = 4;

/**
 * 基準の画面の高さ（CSS px）。**NFR-19 が幅に使っている 360 px の端末**に、
 * その端末の高さを合わせたもの（360 × 640）。
 */
export const VIEWPORT_H_PX = 640;

/** ひとスクロール = **2 画面ぶん**（第 8 回 Q30）。開いて 1 画面、1 回スクロールで残り。 */
export const ONE_SCROLL_PX = VIEWPORT_H_PX * 2;

/** ソースの節どうしの間隔。**勘定に効くので `one-scroll.test.tsx` が見ている。** */
export const SECTION_GAP_PX = 12;
/** ソースの節の内側の余白。同上。密度 66 の式（角の半分を下回らない）。 */
export const SECTION_PAD_PX = round1(Math.max(GAP * 0.45, RADIUS.section * 0.5));

/** 伸ばしたときの週数（1 年）。 */
export const YEAR_WEEKS = 53;
