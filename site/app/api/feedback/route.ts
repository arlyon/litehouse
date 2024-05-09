import { type AxiomRequest, withAxiom } from "next-axiom";
import { NextResponse } from "next/server";
import { z } from "zod";

const FeedbackSchema = z
	.object({
		name: z.string().optional(),
		email: z.string().email().optional(),
		version: z.string(),
		feedback: z.string(),
	})
	.or(
		z.object({
			name: z.string().optional(),
			email: z.string().email().optional(),
			event: z.string(),
		}),
	);

/**
 * Called when a user presses the button to get access to the app
 */
export const GET = withAxiom(async (req: AxiomRequest) => {
	return NextResponse.json({ status: true });
});

/**
 * Called when a user submits feedback via the CLI
 */
export const POST = withAxiom(async (req: AxiomRequest) => {
	req.log.info("receiving event");

	let data: object;
	try {
		data = await req.json();
	} catch (e) {
		return NextResponse.json({ error: "invalid json" }, { status: 400 });
	}
	const validated = FeedbackSchema.safeParse(data);
	if (!validated.success) {
		return NextResponse.json({ error: validated.error }, { status: 400 });
	}

	if ("version" in validated.data) {
		req.log.info("feedback received", validated.data);
	} else if ("event" in validated.data) {
		req.log.info("event received", validated.data);
	}

	return NextResponse.json({ success: true });
});
