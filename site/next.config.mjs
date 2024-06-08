import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { MapWebpackPlugin } from "fumadocs-mdx/config";
import createMDX from "fumadocs-mdx/config";
import { withAxiom } from "next-axiom";
import codeImport from "remark-code-import";
import { bundledLanguages } from "shiki";

const cwd = process.cwd();
const rootMapPath = ".map.ts";

const wit = JSON.parse(fs.readFileSync("wit.tmLanguage.json", "utf8"));

const rootMapFile = path.resolve(cwd, rootMapPath);

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

// create the map file
new MapWebpackPlugin({ rootMapFile }).create();

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
    turbo: {
      rules: {
        "*.{mx,mdx}": [
          {
            loader: "fumadocs-mdx/loader-mdx",
            options: {
              rootContentDir: "./content",
              providerImportSource: "@/mdx-components",
              rootMapFile,
              rehypeCodeOptions: {
                langs: [...Object.keys(bundledLanguages), wit],
              },
              remarkPlugins: [
                ["remark-code-import", { allowImportingFromOutside: true }],
              ],
            },
          },
        ],
        ".map.ts": [
          {
            loader: "fumadocs-mdx/loader",
            options: {
              rootContentDir: "./content",
              rootMapFile,
            },
          },
        ],
      },
    },
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
