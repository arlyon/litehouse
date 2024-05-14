/**
 * This code was generated by v0 by Vercel.
 * @see https://v0.dev/t/pmh0DXSxRC9
 * Documentation: https://v0.dev/docs#integrating-generated-code-into-your-nextjs-app
 */

/** Add fonts into your Next.js project:

import { Archivo } from 'next/font/google'
import { Gabarito } from 'next/font/google'

archivo({
  subsets: ['latin'],
  display: 'swap',
})

gabarito({
  subsets: ['latin'],
  display: 'swap',
})

To read more about using these font, please visit the Next.js documentation:
- App Directory: https://nextjs.org/docs/app/building-your-application/optimizing/fonts
- Pages Directory: https://nextjs.org/docs/pages/building-your-application/optimizing/fonts
**/
import { Button } from "@/components/ui/button";
import { Database, ExternalLink } from "lucide-react";
import Link from "next/link";
import type { SVGProps } from "react";
import { AddButton } from "./add-button";
import { ManifestButton } from "./manifest-button";
import { ManifestEditor } from "./manifest-editor";
import { ThemeToggle } from "./theme-toggle";

export function RegistryPage(props: {
  totalDownloads: number;
  pluginCount: number;
  users: number;
  packages: {
    title: string;
    description: string;
    version: string;
    downloads?: number;
  }[];
}) {
  return (
    <div>
      <div className="font-mono text-muted-foreground my-4">
        4 out of 4 results
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-6">
        {props.packages?.map((p) => (
          <Package key={p.title} {...p} />
        ))}
      </div>
    </div>
  );
}

function Package(props: {
  title: string;
  description: string;
  version: string;
  downloads?: number;
}) {
  const formatter = new Intl.NumberFormat("en-US");
  return (
    <div className="relative overflow-visible">
      <div className="relative bg-background border border-accent hover:border-orange-300 z-[2] has-[[data-selected=true]]:!border-green-500 dark:hover:border-orange-600 ">
        <div className="p-4">
          <h3 className="text-lg font-semibold mb-2 flex items-center justify-between">
            <Link href="/registry/tasmota" className="hover:underline">
              {props.title}
            </Link>
          </h3>
          <p className="text-muted-foreground mb-4">{props.description}</p>
          <div className="flex items-center justify-between">
            <div className="text-sm text-muted-foreground">
              {props.version ? (
                <span className="font-medium">v{props.version}</span>
              ) : null}
              {props.downloads !== undefined && props.version ? (
                <span className="mx-2">•</span>
              ) : null}
              {props.downloads !== undefined ? (
                <span>{formatter.format(props.downloads)} downloads</span>
              ) : null}
            </div>
            <AddButton
              name={props.title}
              version={props.version}
              downloads={props.downloads ?? 0}
            />
          </div>
        </div>
      </div>
      <div className="relative w-full h-full z-0 -translate-y-full top-2 left-2 bg-background border border-accent" />
    </div>
  );
}

function PackageIcon(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      {...props}
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <title>Package</title>
      <path d="m7.5 4.27 9 5.15" />
      <path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z" />
      <path d="m3.3 7 8.7 5 8.7-5" />
      <path d="M12 22V12" />
    </svg>
  );
}

function SearchIcon(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      {...props}
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <title>Search</title>
      <circle cx="11" cy="11" r="8" />
      <path d="m21 21-4.3-4.3" />
    </svg>
  );
}
