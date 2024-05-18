import { type Plugin, PluginPage } from "@/components/plugin-page";
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

/**
 * Returns versions in descending order
 *
 * @param title The title of the plugin
 * @param version The version of the plugin. If not provided, the latest version is returned
 */
const getPluginData = async (
  title: string,
  version?: string,
): Promise<Plugin> => {
  const versions = [
    { version: "0.1.2", date: new Date() },
    { version: "0.1.1", date: new Date() },
    { version: "0.1.0", date: new Date() },
  ];

  return {
    title,
    version:
      versions.find((v) => v.version === version)?.version ??
      versions[0].version,
    versions,
    configSchema:
      '{"properties": {  "config": {    "properties": {      "ip": {        "description": "The ip address of the device to connect to.",        "items": [          {            "format": "uint8",            "minimum": 0.0,            "type": "integer"          },          {            "format": "uint8",            "minimum": 0.0,            "type": "integer"          },          {            "format": "uint8",            "minimum": 0.0,            "type": "integer"          },          {            "format": "uint8",            "minimum": 0.0,            "type": "integer"          }        ],        "maxItems": 4,        "minItems": 4,        "type": "array"      }    },    "required": [      "ip"    ],    "type": "object"  },  "plugin": {    "const": "tasmota@0.1.1"  }},"required": [  "plugin",  "config"],"type": "object"}',
    author: "Alex Lyon",
    source: "https://github.com/arlyon/litehouse",
    capabilities: ["http-client"],
    homepage: "https://github.com/arlyon/litehouse",
    description: "A real cool plugin!",
    size: 60345,
  };
};

const getPlugins = async (): Promise<
  { title: string; versions: string[] }[]
> => {
  return [{ title: "tasmota", versions: ["0.1.2", "0.1.1", "0.1.0"] }];
};

export default Page;

export async function generateStaticParams() {
  const results = (await getPlugins()).flatMap((page) =>
    [undefined, ...page.versions].map((version) => ({
      slug: [page.title, version].filter((x) => x !== undefined),
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

  console.log(params);

  // if (page == null) notFound();

  return {
    title: `Litehouse - ${pluginData.title}@${pageVersion.version}`,
    description: pluginData.description,
    authors: pluginData.author ? [{ name: pluginData.author }] : undefined,
  } satisfies Metadata;
}
