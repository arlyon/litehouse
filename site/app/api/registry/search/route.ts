import { getPages } from "@/app/source";
import { createSearchAPI } from "fumadocs-core/search/server";

type Plugin = {
  id: string;
  url: string;
  title: string;
  metadata: Record<string, string>;
};

function getPlugins(): Plugin[] {
  return [];
}

export const { GET } = createSearchAPI("advanced", {
  indexes: getPages().map((page) => ({
    title: page.data.title,
    structuredData: page.data.exports.structuredData,
    id: page.url,
    url: page.url,
  })),
});
