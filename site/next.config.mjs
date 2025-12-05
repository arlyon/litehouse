import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { createMDX } from "fumadocs-mdx/next";
import { withAxiom } from "next-axiom";
import { bundledLanguages } from "shiki";

const cwd = process.cwd();
const rootMapPath = ".map.ts";

const wit = JSON.parse(fs.readFileSync("wit.tmLanguage.json", "utf8"));

const withMDX = createMDX();

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
  experimental: {
    // turbopackFileSystemCacheForDev: true
  },
  output: "standalone",
  pageExtensions: ["ts", "tsx"],
  typescript: {
    // !! WARN !!
    // Dangerously allow production builds to successfully complete even if
    // your project has type errors.
    // !! WARN !!
    ignoreBuildErrors: true,
  },
};

export default withAxiom(withMDX(config));
