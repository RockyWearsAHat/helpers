import { getStore } from "@netlify/blobs";
import { checkPassword, createToken } from "./_shared/auth.mjs";
import { options, json, fail } from "./_shared/response.mjs";

export default async (req) => {
  if (req.method === "OPTIONS") return options();
  if (req.method !== "POST") return fail("Method not allowed", 405);

  const body = await req.json().catch(() => null);
  if (!body) return fail("Invalid JSON");

  const { email, password } = body;
  if (!email || !password) return fail("Email and password required");

  const e = String(email).toLowerCase().trim();
  const users = getStore("users");
  const raw = await users.get(e);
  if (!raw) return fail("Invalid credentials", 401);

  const user = JSON.parse(raw);
  if (!(await checkPassword(password, user.passwordHash))) {
    return fail("Invalid credentials", 401);
  }

  return json({ token: createToken(e), email: e });
};
