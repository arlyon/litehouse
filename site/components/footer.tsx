/**
 * v0 by Vercel.
 * @see https://v0.dev/t/WIFIRsYvLgz
 * Documentation: https://v0.dev/docs#integrating-generated-code-into-your-nextjs-app
 */
import { Logo } from "@/app/page";
import Link from "next/link";
import { ThemeToggle } from "./theme-toggle";

export function Footer() {
  return (
    <footer className="w-full bg-primary-foreground py-12 text-sm border-t">
      <div className="container mx-auto grid grid-cols-2 lg:grid-cols-[auto_1fr_1fr_max-content] gap-8 sm:gap-16 px-8">
        <div>
          <h4 className="font-semibold mb-2">Resources</h4>
          <ul className="text-muted-foreground">
            <li>
              <Link
                href="https://github.com/arlyon/litehouse"
                className="hover:text-primary transition-colors"
                prefetch={false}
              >
                Source Code
              </Link>
            </li>
            <li>
              <Link
                href="/docs"
                className="hover:text-primary transition-colors"
                prefetch={false}
              >
                Documentation
              </Link>
            </li>
            <li>
              <Link
                href="/registry"
                className="hover:text-primary transition-colors"
                prefetch={false}
              >
                Registry
              </Link>
            </li>
          </ul>
        </div>
        <div>
          <h4 className="font-semibold mb-2">Community</h4>
          <ul className="text-muted-foreground">
            <li>
              <Link
                href="https://github.com/arlyon/litehouse/issues"
                className="hover:text-primary transition-colors"
                prefetch={false}
              >
                Raise an Issue
              </Link>
            </li>
            <li>
              <Link
                href="https://github.com/arlyon/litehouse/discussions"
                className="hover:text-primary transition-colors"
                prefetch={false}
              >
                Open a Discussion
              </Link>
            </li>
          </ul>
        </div>
        <div className="lg:-order-1 max-w-96">
          <Logo />
          <h3 className="text-lg font-semibold mb-4 mt-2">Litehouse</h3>
          <p className="text-muted-foreground">
            Litehouse is a lightweight OS for your home, designed from the
            ground up to be easy to use, resource efficient, open, and secure.
          </p>
        </div>
        <div className="flex flex-col items-start gap-4">
          <p className="text-muted-foreground">
            Made by{" "}
            <Link
              href="https://github.com/arlyon"
              className="text-primary hover:underline"
              prefetch={false}
            >
              @arlyon
            </Link>
          </p>
          <ThemeToggle />
        </div>
      </div>
    </footer>
  );
}
