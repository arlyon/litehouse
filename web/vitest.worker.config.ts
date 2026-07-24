import { cloudflareTest } from "@cloudflare/vitest-pool-workers";
import { defineProject } from "vitest/config";

export default defineProject({
  plugins: [
    cloudflareTest({
      wrangler: {
        configPath: "./dist/server/wrangler.json",
      },
    }),
  ],
  test: {
    include: ["tests/worker.test.ts"],
  },
});
