import { auth } from "@/lib/auth";
import { VercelToolbar } from "@vercel/toolbar/next";
import { headers } from "next/headers";

export const Toolbar = async () => {
  const session = await auth.api.getSession({
    headers: await headers(),
  });

  // TODO: Add admin role check to Better Auth session
  // For now, hide the toolbar
  if (!session) {
    return null;
  }

  return <VercelToolbar />;
};
