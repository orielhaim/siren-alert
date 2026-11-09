import type { ButtonHTMLAttributes } from "react";
import WarmTooltip from "./WarmTooltip";
export function TooltipButton({
  title,
  children,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement>) {
  const content = title || props["aria-label"];
  return (
    <WarmTooltip
      content={content}
      surfaceColor="#353130"
      inkColor="#f2e9e5"
      size="sm"
    >
      <button {...props}>{children}</button>
    </WarmTooltip>
  );
}
