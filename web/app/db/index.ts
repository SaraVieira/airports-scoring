import { config } from "dotenv";
config({ path: "../.env" });
import { drizzle, NodePgDatabase } from "drizzle-orm/node-postgres";
import { Pool } from "pg";
import * as schema from "./schema";
import * as relations from "./relations";

type Schema = typeof schema & typeof relations;

let instance: NodePgDatabase<Schema> | null = null;

export const db = new Proxy({} as NodePgDatabase<Schema>, {
  get(_target, prop) {
    if (!instance) {
      const pool = new Pool({
        connectionString: process.env.DATABASE_URL,
      });
      instance = drizzle(pool, {
        schema: { ...schema, ...relations },
      });
    }
    return (instance as any)[prop];
  },
});
