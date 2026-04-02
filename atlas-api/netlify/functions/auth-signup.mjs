import { getStore } from "@netlify/blobs";
import { hashPassword, createToken } from "./_shared/auth.mjs";
import { options, json, fail } from "./_shared/response.mjs";

export default async (req) => {
  if (req.method === "OPTIONS") return options();
  if (req.method !== "POST") return fail("Method not allowed", 405);

  const body = await req.json().catch(() => null);
  if (!body) return fail("Invalid JSON");

  const { email, password } = body;
  if (!email || !password) return fail("Email and password required");
  if (typeof password !== "string" || password.length < 8)
    return fail("Password must be at least 8 characters");
  if (typeof password !== "string" || password.length > 128)
    return fail("Password too long");

  const e = String(email).toLowerCase().trim().slice(0, 254);
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(e)) return fail("Invalid email");

  const users = getStore("users");
  if (await users.get(e)) return fail("Account already exists", 409);

  const hash = await hashPassword(password);
  await users.set(
    e,
    JSON.stringify({ email: e, passwordHash: hash, createdAt: Date.now() }),
  );

  return json({ token: createToken(e), email: e });
};
