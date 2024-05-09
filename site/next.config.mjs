import createMDX from "fumadocs-mdx/config";
import { withAxiom } from "next-axiom";

const withMDX = createMDX();

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
};

export default withAxiom(withMDX(config));
