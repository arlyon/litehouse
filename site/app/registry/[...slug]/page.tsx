import { PluginPage } from "@/components/plugin-page";
import { NextPage } from "next";
import type { Metadata } from "next";
import { useRouter } from "next/navigation";
import { notFound } from "next/navigation";
import { PropsWithChildren } from "react";

const Page = async ({ params }: { params: { slug: string[] } }) => {
  const plugin = params.slug[0];
  const allVersions = await getVersions(plugin);
  const pageVersion = params.slug[1] ?? allVersions[0];

  console.log(pageVersion, allVersions, params);

  const versions = [...allVersions.entries()].map(([index, version]) => ({
    version,
    date: new Date(),
    current: version === pageVersion,
  }));

  return (
    <PluginPage
      title="tasmota"
      version={pageVersion}
      versions={versions}
      capabilities={["http-client"]}
    />
  );
};

/**
 * Returns versions in descending order
 */
const getVersions = async (version: string) => {
  return ["0.1.2", "0.1.1", "0.1.0"];
};

const getPlugins = async (): Promise<
  { title: string; versions: string[] }[]
> => {
  return [{ title: "tasmota", versions: ["0.1.2", "0.1.1", "0.1.0"] }];
};

export default Page;

export async function generateStaticParams() {
  const results = (await getPlugins()).flatMap((page) =>
    [undefined, ...page.versions].map((version) =>
      ({slug: [page.title, version].filter(x => x !== undefined)})
    ),
  );
  console.log(results)
  return results
}

export function generateMetadata({ params }: { params: { slug?: string[] } }) {
  const page = {
    title: "Title",
    description: "Description",
  };

  // if (page == null) notFound();

  return {
    title: page.title,
    description: page.description,
  } satisfies Metadata;
}
