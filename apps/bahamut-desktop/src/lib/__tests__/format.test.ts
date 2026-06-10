import { describe, expect, it } from "vitest";
import { formatBytes, shortHash } from "../format";

describe("formatBytes", () => {
  it("formats byte ranges", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(2 * 1024 * 1024)).toBe("2.0 MB");
  });

  it("handles invalid input", () => {
    expect(formatBytes(-1)).toBe("—");
    expect(formatBytes(Number.NaN)).toBe("—");
  });
});

describe("shortHash", () => {
  it("truncates long hashes and keeps short ones", () => {
    expect(shortHash("abcdef0123456789abcdef")).toBe("abcdef012345");
    expect(shortHash("abc")).toBe("abc");
  });
});
