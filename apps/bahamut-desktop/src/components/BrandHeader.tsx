import type { ReactNode } from "react";
import logoUrl from "../assets/bahamut-logo.png";

interface BrandHeaderProps {
  /** Optional right-aligned controls (mode toggles etc.). */
  children?: ReactNode;
}

/**
 * Compact application header: icon-only Bahamut logo (derived 256px asset,
 * never the 1536px master) + wordmark text. Works in glassmorphic and solid
 * accessibility modes — the logo is a transparent PNG over the app surface.
 */
export function BrandHeader({ children }: BrandHeaderProps) {
  return (
    <header className="brand-header">
      <div className="brand-identity">
        <img src={logoUrl} alt="Bahamut logo" className="brand-mark" width={28} height={28} />
        <h1 className="app-title">Bahamut</h1>
        <span className="badge-beta">MVP</span>
      </div>
      <div className="brand-actions">{children}</div>
    </header>
  );
}

export { logoUrl };
