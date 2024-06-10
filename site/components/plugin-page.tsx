import { GithubStars as GithubBanner } from "@/components/github-stars";
import Link from "next/link";
import type { SVGProps } from "react";
import { AddButton } from "./add-button";
import { SchemaEditor } from "./shema-editor";
import { Button } from "./ui/button";

export type Plugin = {
  title: string;
  version: string;
  downloads?: number;
  size?: number;
  versions: { version: string; date: Date }[];
  configSchema?: string;
  author?: string;
  description?: string;
  capabilities?: string[];
  homepage?: string;
  source?: string;
  readme?: string;
};

function formatBytes(bytes: number, decimals = 2) {
  if (!+bytes) return "0 Bytes";

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = [
    "Bytes",
    "KiB",
    "MiB",
    "GiB",
    "TiB",
    "PiB",
    "EiB",
    "ZiB",
    "YiB",
  ];

  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return `${Number.parseFloat((bytes / k ** i).toFixed(dm))} ${sizes[i]}`;
}

export function PluginPage(
  props: Plugin & {
    versions: { version: string; date: Date; current?: boolean }[];
  },
) {
  const format = new Intl.DateTimeFormat("en-US");
  const id = `${props.title}@${props.version}`;
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 grid-flow-col grid-rows-[auto_auto_auto_auto] md:grid-rows-[auto_auto] gap-8 py-8">
      <header className="flex flex-row justify-between items-end">
        <div className="space-y-2">
          <h2 className="text-2xl font-bold">{props.title}</h2>
          {props.downloads ? (
            <div className="flex items-center gap-2 text-sm text-neutral-500 dark:text-neutral-400">
              <DownloadIcon className="h-4 w-4" />
              <span>{props.downloads} downloads</span>
            </div>
          ) : null}
        </div>
        <AddButton
          className="mt-1"
          name={props.title}
          version={props.version}
          downloads={props.downloads}
        />
      </header>
      <div className="space-y-6">
        <GithubBanner url={props.source} />
        <div className="space-y-2">
          <h3 className="text-lg font-medium">Version History</h3>
          <div className="space-y-2 text-sm">
            {props.versions?.map((v) => (
              <Link
                scroll={false}
                key={`${props.title}@${v.version}`}
                href={`/registry/${props.title}/${v.version}`}
                // @ts-expect-error
                data-current={v.current}
                className="flex items-center justify-between hover:underline data-[current=true]:bg-green-100 data-[current=true]:dark:bg-green-900 data-[current=true]:border data-[current=true]:border-green-400 data-[current=true]:dark:border-green-700 -my-1 py-1 -mx-2 px-2 dark:border-neutral-800"
              >
                <span className="font-mono">v{v.version}</span>
                <span className="text-neutral-500 dark:text-neutral-400">
                  {format.format(v.date)}
                </span>
              </Link>
            ))}
          </div>
        </div>
        <div className="space-y-2">
          <h3 className="text-lg font-medium">Details</h3>
          <div className="grid grid-cols-2 gap-4 text-sm">
            {props.size ? (
              <div className="flex items-center gap-2">
                <FileIcon className="h-4 w-4" />
                <span>
                  <span className="font-mono">{formatBytes(props.size)}</span>
                </span>
              </div>
            ) : null}
            <div className="flex items-center gap-2">
              <PackageIcon className="h-4 w-4" />
              <span className="font-mono">{props.version}</span>
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
          <ul className="text-sm text-muted-foreground">
            {props.capabilities?.map((c) => (
              <li
                key={c}
                className="rounded-full text-xs font-mono bg-primary-foreground border w-max px-3 py-1"
              >
                {c}
              </li>
            )) ?? "None"}
          </ul>
        </div>
      </div>
      <h3 className="text-lg font-medium flex items-end justify-between">
        <span>Configuration</span>
        <Button variant="ghost">Share</Button>
      </h3>
      <SchemaEditor id={id} schema={props.configSchema} />
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
