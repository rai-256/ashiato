// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

/**
 * 画面（V-01）の組み立ての印（ST49 / design D5・D6）。`<版番号>+<commit 40 桁|unknown>[.dirty][.debug]`。
 * `vite.config.ts` と `vitest.config.ts` が読み、`define: { __ASHIATO_WEB_VERSION__ }` に埋める。
 * Rust の `crates/build-stamp` と同じ規則を、vite の道具の中で書き直したもの（形は同じ正規表現で見る）。
 */
declare const process: { env: Record<string, string | undefined> };

/** この部品のソースの道（リポジトリの根から。cwd に依らない pathspec）。 */
const SOURCE_PATHS = [":/web", ":/Cargo.toml"];

/** git を呼ぶ関数。引数を渡して標準出力を返し、失敗したら投げる。試験は偽のものを渡す。 */
export type Git = (args: string[]) => string;

export const execGit: Git = (args) =>
  execFileSync("git", args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
    // `git status` が index を書き換えて次の組み立てを呼ばないように（D5）
    env: { ...process.env, GIT_OPTIONAL_LOCKS: "0" },
  });

export interface StampInput {
  /** 根の Cargo.toml の内容。未指定時はリポジトリのファイルを読む。 */
  cargoToml?: string;
  /** `tools/release/MAJOR` の内容（左の数字）。未指定時はリポジトリのファイルを読む。渡した版番号があるときだけ読む。 */
  major?: string;
  /** `vite build`（production）だけ `.debug` を付けない */
  production: boolean;
  git?: Git;
  env?: Record<string, string | undefined>;
}

/** 根の Cargo.toml の `[workspace.package]` の版（画面の側の唯一の読み方。e2e もこれを使う。/code-review R40） */
export function sourceVersion(cargoToml: string): string {
  const match = /^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m.exec(cargoToml);
  if (!match?.[1]) throw new Error("Cargo.toml の [workspace.package] version が無い");
  return match[1];
}

interface VersionShape {
  numbers: [number, number, number];
  suffixFree: boolean;
}

/** 数字 3 つと後ろの形（`-beta.<1〜98>`・`-dev.<YYYYMMDD>`）。決めた形でなければ undefined。 */
function versionShape(value: string): VersionShape | undefined {
  const match = /^(\d+)\.(\d+)\.(\d+)(?:-beta\.([1-9]|[1-8]\d|9[0-8])|-dev\.\d{8})?$/.exec(value);
  if (!match) return undefined;
  const numbers = [Number(match[1]), Number(match[2]), Number(match[3])] as [number, number, number];
  if (!numbers.every(Number.isSafeInteger)) return undefined;
  return { numbers, suffixFree: !value.includes("-") };
}

/** 左の数字・真ん中・右の上限（versionCode の式 D3 の内側）。 */
const LIMITS: [number, number, number] = [209, 999, 9999];

/** 左の数字のファイル（`tools/release/MAJOR`）の内容を数字にする。数字 1 つでなければ投げる。 */
export function parseMajor(text: string): number {
  const value = text.trim();
  if (!/^\d+$/.test(value)) throw new Error(`tools/release/MAJOR が数字 1 つでない: ${JSON.stringify(value)}`);
  return Number(value);
}

/**
 * 版の規則（design D9。bash・Rust・gradle と同じ）。渡した版の 3 つの数字が出所の版と同じなら「番号を持たない版」、
 * 左が `tools/release/MAJOR` の値で `<左>.<真ん中>` が `0.0` でなければ「番号」。
 */
export function checkVersion(source: string, given: string, major: number): string {
  const context = `左の数字のファイル ${major}、出所 ${source}、渡した版番号 ${given}`;
  const sourceParsed = versionShape(source);
  if (!sourceParsed || !sourceParsed.suffixFree || sourceParsed.numbers[0] !== 0 || sourceParsed.numbers[1] !== 0 || sourceParsed.numbers[2] < 1) {
    throw new Error(`版番号の出所が 0.0.<N>（N は 1 以上）の形でない: ${context}`);
  }
  const givenParsed = versionShape(given);
  if (!givenParsed) throw new Error(`ASHIATO_BUILD_VERSION の形が不正: ${context}`);
  if (givenParsed.numbers.join(".") !== sourceParsed.numbers.join(".")) {
    const [left, middle] = givenParsed.numbers;
    if (left !== major) throw new Error(`左の数字が左の数字のファイルと違う: ${context}`);
    if (left === 0 && middle === 0) throw new Error(`出所の版と違う 0.0.x は番号ではない: ${context}`);
  }
  if (givenParsed.numbers.some((n, i) => n > LIMITS[i])) {
    throw new Error(`数字が上限（左 ${LIMITS[0]}・真ん中 ${LIMITS[1]}・右 ${LIMITS[2]}）を超えた: ${context}`);
  }
  return given;
}

function resolvedVersion(source: string, readMajor: () => string, env: Record<string, string | undefined>): string {
  const given = env.ASHIATO_BUILD_VERSION?.trim();
  if (!given) {
    if (!versionShape(source)) throw new Error(`Cargo.toml の版番号が不正: ${source}`);
    return source;
  }
  return checkVersion(source, given, parseMajor(readMajor()));
}

/** commit の出所は環境変数 `ASHIATO_BUILD_COMMIT`（空でなければ）→ `git rev-parse HEAD` → `unknown`。 */
function commitOf(git: Git, env: Record<string, string | undefined>): string {
  const fromEnv = env.ASHIATO_BUILD_COMMIT?.trim();
  if (fromEnv) return fromEnv;
  try {
    const head = git(["rev-parse", "HEAD"]).trim();
    return /^[0-9a-f]{40}$/.test(head) ? head : "unknown";
  } catch {
    return "unknown";
  }
}

function isDirty(git: Git): boolean {
  try {
    return SOURCE_PATHS.some((path) => git(["status", "--porcelain", "--", path]).trim() !== "");
  } catch {
    return false;
  }
}

export function webBuildStamp({ cargoToml, major, production, git = execGit, env = process.env }: StampInput): string {
  const source = cargoToml ?? readFileSync(new URL("../Cargo.toml", import.meta.url), "utf8");
  const readMajor = () => major ?? readFileSync(new URL("../tools/release/MAJOR", import.meta.url), "utf8");
  const version = resolvedVersion(sourceVersion(source), readMajor, env);
  const commit = commitOf(git, env);
  return `${version}+${commit}${isDirty(git) ? ".dirty" : ""}${production ? "" : ".debug"}`;
}
