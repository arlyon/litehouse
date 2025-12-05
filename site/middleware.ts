import { betterFetch } from "@better-fetch/fetch";
import type { Session } from "@/lib/auth";
import { NextResponse, type NextRequest } from "next/server";

export default async function authMiddleware(request: NextRequest) {
  const { data: session } = await betterFetch<Session>(
    "/api/auth/get-session",
    {
      baseURL: request.nextUrl.origin,
      headers: {
        cookie: request.headers.get("cookie") || "",
      },
    }
  );

  // Protect cockpit routes
  if (request.nextUrl.pathname.startsWith("/cockpit")) {
    if (!session) {
      return NextResponse.redirect(new URL("/sign-in", request.url));
    }
  }

  // Protect profile routes
  if (request.nextUrl.pathname.startsWith("/profile")) {
    if (!session) {
      return NextResponse.redirect(new URL("/sign-in", request.url));
    }
  }

  // Redirect authenticated users away from auth pages
  if (
    (request.nextUrl.pathname.startsWith("/sign-in") ||
      request.nextUrl.pathname.startsWith("/sign-up")) &&
    session
  ) {
    return NextResponse.redirect(new URL("/registry", request.url));
  }

  return NextResponse.next();
}

export const config = {
  matcher: [
    "/sign-in/:path*",
    "/sign-up/:path*",
    "/profile/:path*",
    "/cockpit/:path*",
  ],
};
