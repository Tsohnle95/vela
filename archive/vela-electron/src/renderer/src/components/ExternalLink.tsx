import type { ComponentPropsWithoutRef, MouseEvent, ReactNode } from "react";
import { safeExternalUrl } from "@shared/url-policy";

type ExternalLinkProps = ComponentPropsWithoutRef<"a"> & { node?: unknown };

export function ExternalLink({ href, children, node: _node, ...props }: ExternalLinkProps): ReactNode {
  const external = safeExternalUrl(href);
  const activate = (event: MouseEvent<HTMLAnchorElement>): void => {
    event.preventDefault();
    if (external) window.open(external, "_blank", "noopener,noreferrer");
  };
  return (
    <a
      {...props}
      href={external ?? undefined}
      target={external ? "_blank" : undefined}
      rel={external ? "noreferrer" : undefined}
      onClick={activate}
    >
      {children}
    </a>
  );
}
