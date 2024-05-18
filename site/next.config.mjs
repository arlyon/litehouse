import fs from "node:fs";
import createMDX from "fumadocs-mdx/config";
import { withAxiom } from "next-axiom";
import codeImport from "remark-code-import";
import { bundledLanguages } from "shiki";

const wit = JSON.parse(fs.readFileSync("wit.tmLanguage.json", "utf8"));

const withMDX = createMDX({
  mdxOptions: {
    remarkPlugins: [
      () =>
        codeImport({
          allowImportingFromOutside: true,
        }),
    ],
    rehypeCodeOptions: {
      langs: [...Object.keys(bundledLanguages), wit],
    },
  },
});

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
  experimental: {
    reactCompiler: true,
    ppr: true,
  },
  typescript: {
    // !! WARN !!
    // Dangerously allow production builds to successfully complete even if
    // your project has type errors.
    // !! WARN !!
    ignoreBuildErrors: true,
  },
};

export default withAxiom(withMDX(config));
