import { type Plugin, PluginPage } from "@/components/plugin-page";
import { getPluginData, getPlugins } from "@/lib/registry";
import type { Metadata } from "next";
import { NextPage } from "next";
import { notFound, useRouter } from "next/navigation";
import { PropsWithChildren } from "react";

const Page = async ({ params }: { params: { slug: string[] } }) => {
  const plugin = params.slug[0];
  const slugVersion = params.slug[1];
  const pluginData = await getPluginData(plugin, slugVersion);
  const pageVersion =
    pluginData.versions.find((v) => v.version === params.slug[1]) ??
    pluginData.versions[0];

  const versions = pluginData.versions.map((version) => ({
    ...version,
    current: version === pageVersion,
  }));

  return <PluginPage {...pluginData} versions={versions} />;
};

export default Page;

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
  params: { slug: string[] };
}) {
  const plugin = params.slug[0];
  const slugVersion = params.slug[1];
  const pluginData = await getPluginData(plugin, slugVersion);
  const pageVersion =
    pluginData.versions.find((v) => v.version === params.slug[1]) ??
    pluginData.versions[0];

  // if (page == null) notFound();

  return {
    title: `Litehouse - ${pluginData.title}@${pageVersion.version}`,
    description: pluginData.description,
    authors: pluginData.author ? [{ name: pluginData.author }] : undefined,
  } satisfies Metadata;
}
