import { CodeBlock, Pre } from "@/components/code-block";
import defaultComponents from "fumadocs-ui/mdx";
import type { MDXComponents } from "mdx/types";

export function useMDXComponents(components: MDXComponents): MDXComponents {
	return {
		...defaultComponents,
		...components,
		pre: ({ ref: _ref, icon, title, ...props }: any) => (
			<CodeBlock icon={icon} title={title} allowDL={title?.includes(".")}>
				<Pre {...props} />
			</CodeBlock>
		),
	};
}
