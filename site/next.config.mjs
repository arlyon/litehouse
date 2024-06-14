import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import createMDX from "fumadocs-mdx/config";
import { withAxiom } from "next-axiom";
import { bundledLanguages } from "shiki";

const cwd = process.cwd();
const rootMapPath = ".map.ts";

const wit = JSON.parse(fs.readFileSync("wit.tmLanguage.json", "utf8"));

const rootMapFile = path.resolve(cwd, rootMapPath);

const withMDX = createMDX({
  mdxOptions: {
    providerImportSource: "@/mdx-components",
    remarkPlugins: [
      ["remark-code-import", { allowImportingFromOutside: true }],
    ],
    rehypeCodeOptions: {
      langs: [...Object.keys(bundledLanguages), wit],
    },
  },
});

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
  images: {
    remotePatterns: [
      {
        protocol: "https",
        hostname: "opengraph.githubassets.com",
      },
    ],
  },
  pageExtensions: ["ts", "tsx"],
  experimental: {
    // useLightningcss: true,
    reactCompiler: true,
    // typedRoutes: true,
    // serverMinification: true,
    // serverSourceMaps: true,
    ppr: true,
    mdxRs: true,
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
