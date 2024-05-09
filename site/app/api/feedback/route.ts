import { withAxiom, AxiomRequest } from "next-axiom";
import { NextResponse } from "next/server";
import { z } from "zod";

const FeedbackSchema = z.object({
  name: z.string().optional(),
  email: z.string().email().optional(),
  version: z.string(),
  feedback: z.string(),
});

export const POST = withAxiom(async (req: AxiomRequest) => {
  req.log.info("receiving a piece of feedback");

  let data;
  try {
    data = await req.json();
  } catch (e) {
    return NextResponse.json({ error: "invalid json" }, { status: 400 });
  }
  let validated = FeedbackSchema.safeParse(data);
  if (!validated.success) {
    return NextResponse.json({ error: validated.error }, { status: 400 });
  }

  req.log.info("feedback received", validated.data);

  return NextResponse.json({ success: true });
});
