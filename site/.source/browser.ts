// @ts-nocheck
import { browser } from 'fumadocs-mdx/runtime/browser';
import type * as Config from '../source.config';

const create = browser<typeof Config, import("fumadocs-mdx/runtime/types").InternalTypeConfig & {
  DocData: {
  }
}>();
const browserCollections = {
  docs: create.doc("docs", {"index.mdx": () => import("../content/index.mdx?collection=docs"), "settings.mdx": () => import("../content/settings.mdx?collection=docs"), "terminology.mdx": () => import("../content/terminology.mdx?collection=docs"), "commands/auth.mdx": () => import("../content/commands/auth.mdx?collection=docs"), "commands/build.mdx": () => import("../content/commands/build.mdx?collection=docs"), "commands/index.mdx": () => import("../content/commands/index.mdx?collection=docs"), "commands/registry.mdx": () => import("../content/commands/registry.mdx?collection=docs"), "cockpit/features.mdx": () => import("../content/cockpit/features.mdx?collection=docs"), "cockpit/index.mdx": () => import("../content/cockpit/index.mdx?collection=docs"), "plugins/authoring.mdx": () => import("../content/plugins/authoring.mdx?collection=docs"), "plugins/hosting.mdx": () => import("../content/plugins/hosting.mdx?collection=docs"), "plugins/index.mdx": () => import("../content/plugins/index.mdx?collection=docs"), }),
};
export default browserCollections;