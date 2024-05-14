/**
 * This code was generated by v0 by Vercel.
 * @see https://v0.dev/t/IeZF9j1Xy5j
 * Documentation: https://v0.dev/docs#integrating-generated-code-into-your-nextjs-app
 */

/** Add fonts into your Next.js project:

import { Archivo } from 'next/font/google'
import { Rethink_Sans } from 'next/font/google'

archivo({
  subsets: ['latin'],
  display: 'swap',
})

rethink_sans({
  subsets: ['latin'],
  display: 'swap',
})

To read more about using these font, please visit the Next.js documentation:
- App Directory: https://nextjs.org/docs/app/building-your-application/optimizing/fonts
- Pages Directory: https://nextjs.org/docs/pages/building-your-application/optimizing/fonts
**/

/** Add border radius CSS variable to your global CSS:

:root {
  --radius: 0rem;
}
**/
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import Link from "next/link";
import type { SVGProps } from "react";
import { AddButton } from "./add-button";
import { CopyBox } from "./copy-box";

export function PluginPage(props: {
  title: string;
  version: string;
  downloads?: number;
  versions?: { version: string; date: Date; current?: boolean }[];
  capabilities: string[];
  description?: string;
}) {
  const format = new Intl.DateTimeFormat("en-US");
  const addCommand = "litehouse::bGl0ZWhvdXNl";

  const id = `${props.title}@${props.version}`;
  return (
    <div key="1" className="grid grid-cols-2 gap-8 py-8">
      <div className="space-y-6">
        <div className="flex flex-row justify-between">
          <div className="space-y-2">
            <h2 className="text-2xl font-bold">{props.title}</h2>
            <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
              <DownloadIcon className="h-4 w-4" />
              <span>
                <span className="font-mono">{props.downloads}</span>
                downloads{"\n"}
              </span>
            </div>
          </div>
          <AddButton
            className="mt-1"
            name={props.title}
            version={props.version}
            downloads={props.downloads}
          />
        </div>
        <div className="space-y-2">
          <h3 className="text-lg font-medium">Version History</h3>
          <div className="space-y-2 text-sm">
            {props.versions?.map((v) => (
              <Link
                key={`${props.title}@${v.version}`}
                href={`/registry/${props.title}/${v.version}`}
                data-current={v.current}
                className="flex items-center justify-between hover:underline data-[current=true]:bg-green-100 data-[current=true]:dark:bg-green-900 data-[current=true]:border data-[current=true]:border-green-400 data-[current=true]:dark:border-green-700 -my-1 py-1 -mx-2 px-2 dark:border-gray-800"
              >
                <span className="font-mono">v{v.version}</span>
                <span className="text-gray-500 dark:text-gray-400">
                  {format.format(v.date)}
                </span>
              </Link>
            ))}
          </div>
        </div>
        <div className="space-y-2">
          <h3 className="text-lg font-medium">Details</h3>
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div className="flex items-center gap-2">
              <FileIcon className="h-4 w-4" />
              <span>
                <span className="font-mono">2.3 MB</span>
              </span>
            </div>
            <div className="flex items-center gap-2">
              <PackageIcon className="h-4 w-4" />
              <span className="font-mono">v2.1.0</span>
            </div>
          </div>
        </div>
        <div className="space-y-2">
          <h3 className="text-lg font-medium">Description</h3>
          <p className="text-sm text-muted-foreground">
            {props.description ?? "No description"}
          </p>
        </div>
        <div className="space-y-2">
          <h3 className="text-lg font-medium">Required Capabilities</h3>
          <p className="text-sm text-muted-foreground">
            {props.capabilities?.map((c) => (
              <div
                key={c}
                className="rounded-full text-xs font-mono bg-primary-foreground border w-max px-3 py-1"
              >
                {c}
              </div>
            )) ?? "None"}
          </p>
        </div>
      </div>
      <div className="space-y-6">
        <div className="space-y-2">
          <h3 className="text-lg font-medium">
            <span>Configuration</span>
          </h3>
          <div className="border p-4 border-accent bg-secondary">
            <pre className="font-mono text-sm">
              {JSON.stringify(
                {
                  $schema: "./schema.json",
                  plugins: {
                    instance: {
                      lat: 24.0,
                      lon: 25.0,
                    },
                  },
                  imports: [id],
                },
                null,
                2,
              )}
            </pre>
          </div>
        </div>
        <div className="border border-accent bg-secondary p-4">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Key</TableHead>
                <TableHead>Value</TableHead>
                <TableHead>Type</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow>
                <TableCell className="font-mono">apiKey</TableCell>
                <TableCell>abc123</TableCell>
                <TableCell className="font-mono">string</TableCell>
              </TableRow>
              <TableRow>
                <TableCell className="font-mono">endpoint</TableCell>
                <TableCell>https://api.example.com</TableCell>
                <TableCell className="font-mono">string</TableCell>
              </TableRow>
              <TableRow>
                <TableCell className="font-mono">debug</TableCell>
                <TableCell>true</TableCell>
                <TableCell className="font-mono">boolean</TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </div>
        <div className="flex flex-col gap-2">
          <h3 className="text-lg font-medium">Add To Manifest</h3>
          <p className="text-sm">
            Run the following command in your project directory to automatically
            insert this plugin and config into your manifest.
          </p>
          <CopyBox
            className="text-sm"
            command={`litehouse add ${addCommand}`}
          />
        </div>
      </div>
    </div>
  );
}

function DownloadIcon(props: SVGProps<SVGSVGElement>) {
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
      <title>Download</title>
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="7 10 12 15 17 10" />
      <line x1="12" x2="12" y1="15" y2="3" />
    </svg>
  );
}

function FileIcon(props: SVGProps<SVGSVGElement>) {
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
      <title>File</title>
      <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
      <path d="M14 2v4a2 2 0 0 0 2 2h4" />
    </svg>
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
