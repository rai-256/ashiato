// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 各検査のあとに DOM を片付ける。
 *
 * **これが無いと描いたものが積み上がり**、2 本目以降の検査が前の検査の DOM も一緒に見る
 * （`Found multiple elements` になるか、**もっと悪いことに前の結果で通ってしまう**）。
 */
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

afterEach(cleanup);
