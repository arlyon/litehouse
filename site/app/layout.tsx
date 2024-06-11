import { Toaster } from "@/components/ui/sonner";
import "./global.css";
import { Footer } from "@/components/footer";
import { RootProvider } from "fumadocs-ui/provider";
import type { Metadata } from "next";
import { AxiomWebVitals } from "next-axiom";
import { Inter } from "next/font/google";
import type { ReactNode } from "react";

const inter = Inter({
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "Litehouse",
  description: "A lightweight home automation server.",
  metadataBase: new URL("https://litehouse.arlyon.dev"),
  creator: "@arlyon",
  openGraph: {
    type: "website",
  },
  viewport: "width=device-width, initial-scale=1",
  robots: "index, follow",
  keywords: ["home automation", "home", "automation", "server", "wasm"],
};

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={inter.className}>
      <head>
        <link rel="icon" href="/favicon.svg" />
      </head>
      <AxiomWebVitals />
      <body>
        <Toaster />
        <RootProvider>
          <main className="min-h-screen flex flex-col">
            <div className="flex-1">{children}</div>
            <Footer />
          </main>
        </RootProvider>
      </body>
    </html>
  );
}
