import { clerkMiddleware } from "@clerk/nextjs/server";

export default clerkMiddleware();

export const config = {
  matcher: [
    "/(sign-in|sign-up)/(.*)?",
    "/registry/(.*)?",
    "/cockpit/(.*)?",
    "/(api|trpc)(.*)",
  ],
};
