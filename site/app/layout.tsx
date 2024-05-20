import { Toaster } from "@/components/ui/sonner";
import "./global.css";
import { RootProvider } from "fumadocs-ui/provider";
import { AxiomWebVitals } from "next-axiom";
import { Inter } from "next/font/google";
import type { ReactNode } from "react";

const inter = Inter({
  subsets: ["latin"],
});

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={inter.className}>
      <AxiomWebVitals />
      <body>
        <Toaster />
        <RootProvider>{children}</RootProvider>
      </body>
    </html>
  );
}
