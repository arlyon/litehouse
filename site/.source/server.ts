// @ts-nocheck
import * as __fd_glob_11 from "../content/plugins/index.mdx?collection=docs"
import * as __fd_glob_10 from "../content/plugins/hosting.mdx?collection=docs"
import * as __fd_glob_9 from "../content/plugins/authoring.mdx?collection=docs"
import * as __fd_glob_8 from "../content/commands/registry.mdx?collection=docs"
import * as __fd_glob_7 from "../content/commands/index.mdx?collection=docs"
import * as __fd_glob_6 from "../content/commands/build.mdx?collection=docs"
import * as __fd_glob_5 from "../content/commands/auth.mdx?collection=docs"
import * as __fd_glob_4 from "../content/cockpit/index.mdx?collection=docs"
import * as __fd_glob_3 from "../content/cockpit/features.mdx?collection=docs"
import * as __fd_glob_2 from "../content/terminology.mdx?collection=docs"
import * as __fd_glob_1 from "../content/settings.mdx?collection=docs"
import * as __fd_glob_0 from "../content/index.mdx?collection=docs"
import { server } from 'fumadocs-mdx/runtime/server';
import type * as Config from '../source.config';

const create = server<typeof Config, import("fumadocs-mdx/runtime/types").InternalTypeConfig & {
  DocData: {
  }
}>({"doc":{"passthroughs":["extractedReferences"]}});

export const docs = await create.docs("docs", "content", {}, {"index.mdx": __fd_glob_0, "settings.mdx": __fd_glob_1, "terminology.mdx": __fd_glob_2, "cockpit/features.mdx": __fd_glob_3, "cockpit/index.mdx": __fd_glob_4, "commands/auth.mdx": __fd_glob_5, "commands/build.mdx": __fd_glob_6, "commands/index.mdx": __fd_glob_7, "commands/registry.mdx": __fd_glob_8, "plugins/authoring.mdx": __fd_glob_9, "plugins/hosting.mdx": __fd_glob_10, "plugins/index.mdx": __fd_glob_11, });