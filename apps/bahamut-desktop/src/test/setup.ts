import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";

// Vitest runs without injected globals, so testing-library's automatic
// cleanup hook does not self-register — do it explicitly.
afterEach(() => {
  cleanup();
});
