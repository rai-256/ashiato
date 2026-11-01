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
  const base = { cargoToml: '[workspace.package]\nversion = "0.1.0"\n', env: {} };

  it("clean production: no suffix", () => {
    const s = webBuildStamp({ ...base, production: true, git: fakeGit({ head: `${HEAD}\n`, status: "" }) });
    expect(s).toBe(`0.1.0+${HEAD}`);
    expect(s).toMatch(STAMP);
  });

  // Scenario: 根の Cargo.toml を書き換えて commit していなければ、携帯と画面の印は .dirty を持つ
  it("uncommitted source-of-truth change adds .dirty", () => {
    const s = webBuildStamp({ ...base, production: false, git: fakeGit({ head: HEAD, status: " M Cargo.toml\n" }) });
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
    const s = webBuildStamp({ ...base, env, production: true, git: fakeGit({ head: HEAD, status: "" }) });
    expect(s).toBe(`0.1.0+${"f".repeat(40)}`);
  });

  it("an empty ASHIATO_BUILD_COMMIT falls through to git", () => {
    const s = webBuildStamp({ ...base, env: { ASHIATO_BUILD_COMMIT: "" }, production: true, git: fakeGit({ head: HEAD, status: "" }) });
    expect(s).toBe(`0.1.0+${HEAD}`);
  });

  // Scenario: 携帯と画面の印の版番号も、根の出所の版から読む
  it("reads the version from the workspace Cargo.toml", () => {
    const s = webBuildStamp({
      cargoToml: '[workspace.package]\nversion = "0.9.0"\n',
      production: true,
      git: fakeGit({ head: HEAD, status: "" }),
    });
    expect(s).toBe(`0.9.0+${HEAD}`);
  });

  it("ASHIATO_BUILD_VERSION accepts a matching prerelease", () => {
    const s = webBuildStamp({
      ...base,
      env: { ASHIATO_BUILD_VERSION: "0.1.0-beta.2" },
      production: true,
      git: fakeGit({ head: HEAD, status: "" }),
    });
    expect(s).toBe(`0.1.0-beta.2+${HEAD}`);
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
