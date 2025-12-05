import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { db } from "./db";
import { magicLink } from "better-auth/plugins";
import { createTransport } from "nodemailer";

const transporter = createTransport({
  host: "smtp.fastmail.com",
  port: 465,
  secure: true,
  auth: {
    user: "alex@arlyon.dev",
    pass: process.env.FASTMAIL_APP_PASSWORD ?? "3v7c4d835w983g6g",
  },
});

export const auth = betterAuth({
  secret: process.env.BETTER_AUTH_SECRET,
  database: drizzleAdapter(db, {
    provider: "sqlite",
  }),
  emailAndPassword: {
    enabled: true,
  },
  plugins: [
    magicLink({
      sendMagicLink: async ({ email, url }) => {
        await transporter.sendMail({
          from: process.env.EMAIL_FROM || "alex@arlyon.dev",
          to: email,
          subject: "Sign in to Litehouse",
          html: `<p>Click <a href="${url}">here</a> to sign in to Litehouse.</p>`,
        });
      },
    }),
  ],
  trustedOrigins: [
    process.env.NEXT_PUBLIC_APP_URL || "http://localhost:3000",
  ],
});

export type Session = typeof auth.$Infer.Session;
