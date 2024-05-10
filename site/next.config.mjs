import { rehypeCodeDefaultOptions } from "fumadocs-core/mdx-plugins";
import createMDX from "fumadocs-mdx/config";
import { withAxiom } from "next-axiom";
import { bundledLanguages } from "shiki";

const withMDX = createMDX({
  mdxOptions: {
    rehypeCodeOptions: {
      langs: [
        ...Object.keys(bundledLanguages),
        {
          name: "wit",
          displayName: "Wit",
          fileTypes: ["wit"],
          scopeName: "source.wit",
          patterns: [],
        },
      ],
    },
  },
});

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
};

export default withAxiom(withMDX(config));
