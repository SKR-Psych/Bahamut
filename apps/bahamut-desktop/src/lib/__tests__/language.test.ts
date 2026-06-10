import { describe, expect, it } from "vitest";
import { languageForFile } from "../language";

describe("languageForFile", () => {
  it("maps common extensions to Monaco language ids", () => {
    expect(languageForFile("main.rs")).toBe("rust");
    expect(languageForFile("App.tsx")).toBe("typescript");
    expect(languageForFile("index.js")).toBe("javascript");
    expect(languageForFile("styles.css")).toBe("css");
    expect(languageForFile("README.md")).toBe("markdown");
    expect(languageForFile("config.yaml")).toBe("yaml");
    expect(languageForFile("script.ps1")).toBe("powershell");
  });

  it("is case-insensitive", () => {
    expect(languageForFile("MAIN.RS")).toBe("rust");
    expect(languageForFile("Index.HTML")).toBe("html");
  });

  it("handles special file names without extensions", () => {
    expect(languageForFile("Dockerfile")).toBe("dockerfile");
    expect(languageForFile("Makefile")).toBe("makefile");
  });

  it("falls back to plaintext", () => {
    expect(languageForFile("LICENSE")).toBe("plaintext");
    expect(languageForFile("data.unknownext")).toBe("plaintext");
    expect(languageForFile("trailingdot.")).toBe("plaintext");
  });
});
