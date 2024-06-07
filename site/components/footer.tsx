/**
 * v0 by Vercel.
 * @see https://v0.dev/t/WIFIRsYvLgz
 * Documentation: https://v0.dev/docs#integrating-generated-code-into-your-nextjs-app
 */
import { Logo } from "@/app/page";
import Link from "next/link";

export function Footer() {
  return (
    <footer className="w-full bg-neutral-900 py-12 text-sm">
      <div className="container mx-auto grid grid-cols-2 lg:grid-cols-[auto_1fr_1fr_max-content] gap-16 px-8">
        <div className="max-w-96">
          <Logo />
          <h3 className="text-lg font-semibold text-white mb-4 mt-2">
            Litehouse
          </h3>
          <p className="text-neutral-400">
            Litehouse is a lightweight OS for your home, designed from the
            ground up to be easy to use, resource efficient, open, and secure.
          </p>
        </div>
        <div>
          <h4 className="text-neutral-200 font-semibold mb-2">Resources</h4>
          <ul className="text-neutral-400">
            <li>
              <Link
                href="https://github.com/arlyon/litehouse"
                className="hover:text-white transition-colors"
                prefetch={false}
              >
                Source Code
              </Link>
            </li>
            <li>
              <Link
                href="/docs"
                className="hover:text-white transition-colors"
                prefetch={false}
              >
                Documentation
              </Link>
            </li>
            <li>
              <Link
                href="/registry"
                className="hover:text-white transition-colors"
                prefetch={false}
              >
                Registry
              </Link>
            </li>
          </ul>
        </div>
        <div className="lg:order-3">
          <p className="text-neutral-400">
            Made by{" "}
            <Link
              href="#"
              className="text-white hover:underline"
              prefetch={false}
            >
              @arlyon
            </Link>
          </p>
        </div>
        <div>
          <h4 className="text-neutral-200 font-semibold mb-2">Community</h4>
          <ul className="text-neutral-400">
            <li>
              <Link
                href="https://github.com/arlyon/litehouse/issues"
                className="hover:text-white transition-colors"
                prefetch={false}
              >
                Raise an Issue
              </Link>
            </li>
            <li>
              <Link
                href="https://github.com/arlyon/litehouse/discussions"
                className="hover:text-white transition-colors"
                prefetch={false}
              >
                Open a Discussion
              </Link>
            </li>
          </ul>
        </div>
      </div>
    </footer>
  );
}