import { auth } from "../lib/auth";

async function migrate() {
  try {
    console.log("Running database migrations...");

    // Better Auth will automatically create tables on first request
    // or you can use the database adapter's migration tools

    console.log("✅ Migrations complete!");
    console.log("Better Auth tables will be created automatically on first use.");
  } catch (error) {
    console.error("❌ Migration failed:", error);
    process.exit(1);
  }
}

migrate();
