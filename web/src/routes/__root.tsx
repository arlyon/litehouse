import {
  HeadContent,
  Scripts,
  createRootRoute,
} from "@tanstack/react-router";
import { createElement, type ReactNode } from "react";
import appCss from "../styles.css?url";

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      {
        name: "viewport",
        content: "width=device-width, initial-scale=1",
      },
      {
        title: "Litehouse capability renderer gate",
      },
      {
        name: "description",
        content:
          "Greenfield Litehouse Worker scaffold and typed capability renderer reference.",
      },
    ],
    links: [{ rel: "stylesheet", href: appCss }],
  }),
  shellComponent: RootDocument,
});

function RootDocument({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <html lang="en">
      <head>
        <HeadContent />
      </head>
      <body>
        {createElement(
          "a",
          { className: "skip-link", href: "#main-content" },
          "Skip to capability controls",
        )}
        {children}
        <Scripts />
      </body>
    </html>
  );
}
