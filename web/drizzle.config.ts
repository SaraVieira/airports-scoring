import { config } from "dotenv";
config({ path: "../.env" });
import { defineConfig } from "drizzle-kit";

export default defineConfig({
  schema: ["./app/db/schema.ts", "./app/db/relations.ts"],
  dialect: "postgresql",
  dbCredentials: {
    url: process.env.DATABASE_URL!,
  },
  extensionsFilters: ["postgis"],
});
