import "./global.css";
import { RootProvider } from "fumadocs-ui/provider";
import { Inter } from "next/font/google";
import type { ReactNode } from "react";
import { AxiomWebVitals } from "next-axiom";

const inter = Inter({
  subsets: ["latin"],
});

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={inter.className}>
      <AxiomWebVitals />
      <body>
        <RootProvider>{children}</RootProvider>
      </body>
    </html>
  );
}