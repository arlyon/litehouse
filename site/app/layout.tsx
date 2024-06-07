import { Toaster } from "@/components/ui/sonner";
import "./global.css";
import { RootProvider } from "fumadocs-ui/provider";
import type { Metadata } from "next";
import { AxiomWebVitals } from "next-axiom";
import { Inter } from "next/font/google";
import type { ReactNode } from "react";
import { Footer } from "@/components/footer";

const inter = Inter({
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "Litehouse",
  description: "Liteweight house automation system, build on web assembly.",
};

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={inter.className}>
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
