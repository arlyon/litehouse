// source.config.ts
import {
  defineConfig,
  defineDocs
} from "fumadocs-mdx/config";
import remarkCodeImport from "remark-code-import";
import { bundledLanguages } from "shiki/langs";
import fs from "node:fs";
var docs = defineDocs({
  dir: "./content"
});
var wit = JSON.parse(fs.readFileSync("wit.tmLanguage.json", "utf8"));
var source_config_default = defineConfig({
  mdxOptions: {
    remarkPlugins: [[remarkCodeImport, { allowImportingFromOutside: true }]],
    rehypeCodeOptions: {
      langs: [...Object.keys(bundledLanguages), wit],
      themes: {
        light: "github-light",
        dark: "github-dark"
      }
    }
  }
});
export {
  source_config_default as default,
  docs
};
