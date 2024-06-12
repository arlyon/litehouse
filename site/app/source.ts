import { map } from "@/.map";
import { loader } from "fumadocs-core/source";
import { createMDXSource, defaultSchemas } from "fumadocs-mdx";
import { icons } from "lucide-react";
import { z } from "zod";
import { create } from "@/components/ui/icon";

export const { getPage, getPages, pageTree } = loader({
  baseUrl: "/docs",
  rootDir: "docs",
  icon(icon) {
    if (icon && icon in icons)
      return create({ icon: icons[icon as keyof typeof icons] });
  },
  source: createMDXSource(map, {
    schema: {
      frontmatter: defaultSchemas.frontmatter.extend({
        toc: z.boolean().default(true),
        // index: z.boolean().default(false),
      }),
    },
  }),
});
