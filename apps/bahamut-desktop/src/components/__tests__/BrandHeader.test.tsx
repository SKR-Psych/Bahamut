import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { BrandHeader } from "../BrandHeader";

describe("BrandHeader", () => {
  it("renders the Bahamut logo with accessible alt text and the wordmark", () => {
    render(<BrandHeader />);
    const logo = screen.getByRole("img", { name: "Bahamut logo" });
    expect(logo).toBeInTheDocument();
    expect(logo).toHaveAttribute("src");
    expect(screen.getByRole("heading", { name: "Bahamut" })).toBeInTheDocument();
  });

  it("renders header actions passed as children", () => {
    render(
      <BrandHeader>
        <button>Mode toggle</button>
      </BrandHeader>,
    );
    expect(screen.getByRole("button", { name: "Mode toggle" })).toBeInTheDocument();
  });

  it("remains functional inside the accessibility solid mode wrapper", () => {
    render(
      <div className="accessibility-solid-mode">
        <BrandHeader />
      </div>,
    );
    expect(screen.getByRole("img", { name: "Bahamut logo" })).toBeInTheDocument();
  });
});
