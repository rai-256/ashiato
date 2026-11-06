// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
import { getConfig } from "@testing-library/dom";
import { describe, expect, test } from "vitest";

describe("画面の単体試験の上限", () => {
  // Scenario: 画面の単体試験の待ちの上限は 10 秒、試験の上限は 30 秒
  // 設定ファイルの字面ではなく、この試験に効いている上限（task.timeout）を読む（review/code.md R16）
  test("待ちと試験の上限を設定している", ({ task }) => {
    expect(getConfig().asyncUtilTimeout).toBe(10_000);
    expect(task.timeout).toBe(30_000);
  });
});
