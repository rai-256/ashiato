// SPDX-License-Identifier: AGPL-3.0-only
import { describe, expect, it } from "vitest";
import { webBuildStamp, type Git } from "../../build-stamp";
import { buildClaim, emptyInput, ingestItem } from "../attributes";
import { WEB_VERSION } from "../buildStamp";
import { buildFeeling, ingestFeelingItem } from "../feelings";
import { buildPlaceRecord, ingestPlaceItem } from "../places";

/** D5 の正規表現（4 つの組み立てが同じ式で見る） */
const STAMP = /^[0-9][^+]*\+([0-9a-f]{40}|unknown)(\.dirty)?(\.debug)?$/;
const HEAD = "0123456789abcdef0123456789abcdef01234567";

function fakeGit(opts: { head?: string | Error; status?: string | Error }): Git {
  return (args) => {
    const r = args[0] === "rev-parse" ? opts.head : opts.status;
    if (r instanceof Error) throw r;
    return r ?? "";
  };
}

describe("web build stamp", () => {
  const base = { version: "0.1.0", env: {} };

  it("clean production: no suffix", () => {
    const s = webBuildStamp({ ...base, production: true, git: fakeGit({ head: `${HEAD}\n`, status: "" }) });
    expect(s).toBe(`0.1.0+${HEAD}`);
    expect(s).toMatch(STAMP);
  });

  it("uncommitted change of web/ adds .dirty, then .debug when not production", () => {
    const s = webBuildStamp({ ...base, production: false, git: fakeGit({ head: HEAD, status: " M web/src/App.tsx\n" }) });
    expect(s).toBe(`0.1.0+${HEAD}.dirty.debug`);
    expect(s).toMatch(STAMP);
  });

  it("production never gets .debug even when dirty", () => {
    const s = webBuildStamp({ ...base, production: true, git: fakeGit({ head: HEAD, status: "?? web/x\n" }) });
    expect(s).toBe(`0.1.0+${HEAD}.dirty`);
  });

  it("no git: unknown commit, not dirty", () => {
    const boom = new Error("no git");
    const s = webBuildStamp({ ...base, production: true, git: fakeGit({ head: boom, status: boom }) });
    expect(s).toBe("0.1.0+unknown");
    expect(s).toMatch(STAMP);
  });

  it("a malformed HEAD is unknown", () => {
    expect(webBuildStamp({ ...base, production: true, git: fakeGit({ head: "HEAD\n", status: "" }) })).toBe("0.1.0+unknown");
  });

  it("ASHIATO_BUILD_COMMIT wins over git", () => {
    const env = { ASHIATO_BUILD_COMMIT: "f".repeat(40) };
    const s = webBuildStamp({ version: "0.1.0", env, production: true, git: fakeGit({ head: HEAD, status: "" }) });
    expect(s).toBe(`0.1.0+${"f".repeat(40)}`);
  });

  it("an empty ASHIATO_BUILD_COMMIT falls through to git", () => {
    const s = webBuildStamp({ version: "0.1.0", env: { ASHIATO_BUILD_COMMIT: "" }, production: true, git: fakeGit({ head: HEAD, status: "" }) });
    expect(s).toBe(`0.1.0+${HEAD}`);
  });
});

describe("the three assemblies carry the embedded stamp", () => {
  it("the embedded stamp has the stamp shape", () => {
    expect(WEB_VERSION).toMatch(STAMP);
  });

  it("feeling, place, claim items put __ASHIATO_WEB_VERSION__ in client_version", () => {
    const now = new Date("2026-10-01T00:00:00Z");
    const id = "11111111-1111-4111-8111-111111111111";
    const feeling = ingestFeelingItem(buildFeeling({ kind: "day", date: "2026-10-01" }, [], null, now, id)) as { client_version: string };
    const place = ingestPlaceItem(buildPlaceRecord({ field: "name", name: "x" }, now, id, id)) as { client_version: string };
    const claim = ingestItem(
      buildClaim(emptyInput("k"), now, id, "n"),
    ) as { client_version: string };
    for (const item of [feeling, place, claim]) {
      expect(item.client_version).toBe(__ASHIATO_WEB_VERSION__);
    }
  });
});
