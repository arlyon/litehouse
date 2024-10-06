import { defineConfig, defineDocs, frontmatterSchema } from 'fumadocs-mdx/config';
import { z } from 'zod';
import remarkCodeImport from 'remark-code-import';
import { rehypeCode } from 'fumadocs-core/mdx-plugins';
import { bundledLanguages } from 'shiki/langs';
import fs from 'node:fs';

export const { docs, meta } = defineDocs({

  docs: {
    dir: './content',
    schema: frontmatterSchema.extend({}),
  }
});

const wit = JSON.parse(fs.readFileSync("wit.tmLanguage.json", "utf8"));

export default defineConfig({
  lastModifiedTime: "git",
  mdxOptions: {
    remarkPlugins: [
      [remarkCodeImport, { allowImportingFromOutside: true }],
    ],
    rehypeCodeOptions: {
      langs: [...Object.keys(bundledLanguages), wit],
      themes: {
        light: 'github-light',
        dark: 'github-dark'
      }
    },
  }
})
