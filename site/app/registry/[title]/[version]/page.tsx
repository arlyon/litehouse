import { Header } from "@/components/header";
import { Footer } from "@/components/footer";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Box,
  Download,
  Clock,
  User,
  GitBranch,
  ExternalLink,
  Copy,
  Check,
  ChevronRight,
  FileText,
  Shield,
  Settings,
} from "lucide-react";
import Link from "next/link";
import { getPluginData, getPlugins } from "@/lib/registry";
import type { Metadata } from "next";
import { SchemaEditor } from "@/components/shema-editor";

export const experimental_ppr = true;

export default async function Page({
  params,
}: {
  params: Promise<{ title: string; version: string }>;
}) {
  const { title, version: slugVersion } = await params;
  const pluginData = await getPluginData(title, slugVersion);
  const pageVersion =
    pluginData.versions?.find((v) => v.version === slugVersion) ??
    pluginData.versions?.[0];

  const formatDate = (date: Date) => {
    return new Intl.DateTimeFormat("en-US", {
      month: "long",
      day: "numeric",
      year: "numeric",
    }).format(date);
  };

  const id = `${pluginData.title}@${pageVersion?.version}`;

  return (
    <div className="min-h-screen flex flex-col">
      <main className="flex-1">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
          {/* Breadcrumb */}
          <nav className="flex items-center gap-1.5 text-sm text-muted-foreground mb-6">
            <Link href="/registry" className="hover:text-foreground transition-colors">
              Registry
            </Link>
            <ChevronRight className="w-3.5 h-3.5" />
            <span className="text-foreground">{pluginData.title}</span>
          </nav>

          <div className="grid lg:grid-cols-3 gap-8">
            {/* Main Content */}
            <div className="lg:col-span-2 space-y-6">
              {/* Header Card */}
              <div className="border border-border rounded-lg bg-card p-6">
                <div className="flex items-start gap-4 mb-4">
                  <div className="w-14 h-14 rounded-xl bg-secondary flex items-center justify-center border border-border shrink-0">
                    <Box className="w-7 h-7 text-accent" />
                  </div>
                  <div className="min-w-0">
                    <div className="flex items-center gap-3 flex-wrap">
                      <h1 className="text-2xl font-bold text-foreground">{pluginData.title}</h1>
                      <Badge variant="secondary">Plugin</Badge>
                    </div>
                    <p className="text-muted-foreground mt-1">{pluginData.description}</p>
                  </div>
                </div>

                <div className="flex flex-wrap gap-4 text-sm text-muted-foreground mb-6">
                  {pluginData.downloads && (
                    <span className="flex items-center gap-1.5">
                      <Download className="w-4 h-4" />
                      {pluginData.downloads.toLocaleString()} downloads
                    </span>
                  )}
                  {pageVersion && (
                    <span className="flex items-center gap-1.5">
                      <Clock className="w-4 h-4" />
                      {formatDate(pageVersion.date)}
                    </span>
                  )}
                  {pluginData.author && (
                    <span className="flex items-center gap-1.5">
                      <User className="w-4 h-4" />
                      {pluginData.author}
                    </span>
                  )}
                </div>

                {/* Install Command */}
                <div className="bg-secondary rounded-lg p-4 border border-border">
                  <div className="flex items-center justify-between gap-4">
                    <code className="font-mono text-sm text-foreground">
                      litehouse install {pluginData.title}@{pageVersion?.version}
                    </code>
                    <Button variant="ghost" size="icon" className="shrink-0 h-8 w-8">
                      <Copy className="w-4 h-4" />
                    </Button>
                  </div>
                </div>
              </div>

              {/* About */}
              {pluginData.readme && (
                <div className="border border-border rounded-lg bg-card p-6">
                  <h2 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
                    <FileText className="w-5 h-5 text-muted-foreground" />
                    About
                  </h2>
                  <p className="text-muted-foreground leading-relaxed">{pluginData.readme}</p>
                </div>
              )}

              {pluginData.configSchema && (
                <div className="border border-border rounded-lg bg-card p-6 space-y-6">
                  <h2 className="text-lg font-semibold text-foreground flex items-center gap-2">
                    <Settings className="w-5 h-5 text-muted-foreground" />
                    Configuration
                  </h2>
                  <SchemaEditor id={id} schema={pluginData.configSchema} />
                </div>
              )}

              {/* Capabilities */}
              {pluginData.capabilities && pluginData.capabilities.length > 0 && (
                <div className="border border-border rounded-lg bg-card p-6">
                  <h2 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
                    <Shield className="w-5 h-5 text-muted-foreground" />
                    Required Capabilities
                  </h2>
                  <div className="flex flex-wrap gap-2">
                    {pluginData.capabilities.map((cap) => (
                      <Badge key={cap} variant="outline" className="font-mono text-xs">
                        {cap}
                      </Badge>
                    ))}
                  </div>
                </div>
              )}
            </div>

            {/* Sidebar */}
            <div className="space-y-6">
              {/* Install */}
              <div className="border border-border rounded-lg bg-card p-5">
                <Button className="w-full gap-2 mb-4" size="lg">
                  <Download className="w-4 h-4" />
                  Install Plugin
                </Button>
                <div className="text-center text-xs text-muted-foreground">or run the CLI command above</div>
              </div>

              {/* Versions */}
              <div className="border border-border rounded-lg bg-card p-5">
                <h3 className="text-sm font-medium text-foreground mb-3 flex items-center gap-2">
                  <GitBranch className="w-4 h-4 text-muted-foreground" />
                  Versions
                </h3>
                <div className="space-y-1">
                  {pluginData.versions?.map((v) => (
                    <Link
                      key={v.version}
                      href={`/registry/${pluginData.title}/${v.version}`}
                      className={`flex items-center justify-between py-2 px-3 rounded-md text-sm transition-colors ${
                        v.version === pageVersion?.version
                          ? "bg-secondary text-foreground"
                          : "text-muted-foreground hover:text-foreground hover:bg-secondary/50"
                      }`}
                    >
                      <span className="font-mono">v{v.version}</span>
                      {v.version === pageVersion?.version && <Check className="w-4 h-4 text-accent" />}
                    </Link>
                  ))}
                </div>
              </div>

              {/* Links */}
              <div className="border border-border rounded-lg bg-card p-5">
                <h3 className="text-sm font-medium text-foreground mb-3">Links</h3>
                <div className="space-y-2">
                  {pluginData.source && (
                    <a
                      href={pluginData.source}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
                    >
                      <ExternalLink className="w-4 h-4" />
                      Repository
                    </a>
                  )}
                  {pluginData.homepage && (
                    <a
                      href={pluginData.homepage}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
                    >
                      <FileText className="w-4 h-4" />
                      Documentation
                    </a>
                  )}
                </div>
              </div>

              {/* Size and Version Info */}
              {pluginData.size && (
                <div className="border border-border rounded-lg bg-card p-5">
                  <h3 className="text-sm font-medium text-foreground mb-2">Package Size</h3>
                  <p className="text-sm text-muted-foreground font-mono">
                    {(pluginData.size / 1024).toFixed(2)} KiB
                  </p>
                </div>
              )}
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}

export async function generateStaticParams() {
  const results = (await getPlugins()).flatMap((page) =>
    [undefined, ...page.versions].map((version) => ({
      slug: [page.title, version?.version].filter((x) => x !== undefined),
    })),
  );
  return results;
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ title: string; version: string }>;
}) {
  const { title, version: slugVersion } = await params;
  const pluginData = await getPluginData(title, slugVersion);
  const pageVersion =
    pluginData.versions?.find((v) => v.version === slugVersion) ??
    pluginData.versions?.[0];

  return {
    title: `Litehouse - ${pluginData.title}@${pageVersion?.version}`,
    description: pluginData.description,
    authors: pluginData.author ? [{ name: pluginData.author }] : undefined,
  } satisfies Metadata;
}
