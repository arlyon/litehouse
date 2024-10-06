import { type Plugin, PluginPage } from "@/components/plugin-page";
import { getPluginData, getPlugins } from "@/lib/registry";
import type { Metadata } from "next";

export const experimental_ppr = true;

const Page = async ({ params }: { params: { title: string } }) => {
  const plugin = params.title;
  const pluginData = await getPluginData(plugin);
  const pageVersion = pluginData.versions[0];

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
  params: { title: string };
}) {
  const plugin = params.title;
  const pluginData = await getPluginData(plugin);
  const pageVersion = pluginData.versions[0];

  // if (page == null) notFound();

  return {
    title: `Litehouse - ${pluginData.title}@${pageVersion.version}`,
    description: pluginData.description,
    authors: pluginData.author ? [{ name: pluginData.author }] : undefined,
  } satisfies Metadata;
}
