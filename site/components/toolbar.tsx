import { auth } from "@clerk/nextjs/server";
import { VercelToolbar } from "@vercel/toolbar/next";

export const Toolbar = () => {
  const user = auth();

  if (user.sessionClaims?.meta?.admin !== true) {
    return null;
  }

  return <VercelToolbar />;
};
