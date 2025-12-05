import { loader } from "fumadocs-core/source";
import { docs } from 'fumadocs-mdx:collections/server';

import { lucideIconsPlugin } from 'fumadocs-core/source/lucide-icons';

export const { getPage, getPages, pageTree } = loader({
  baseUrl: "/docs",
  source: docs.toFumadocsSource(),
  plugins: [lucideIconsPlugin()],
});
