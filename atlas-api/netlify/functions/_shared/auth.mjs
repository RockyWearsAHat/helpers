import bcrypt from "bcryptjs";
import jwt from "jsonwebtoken";

const JWT_SECRET = process.env.JWT_SECRET;

/** Hash a plaintext password with bcrypt (cost 10). */
export async function hashPassword(pw) {
  return bcrypt.hash(pw, 10);
}

/** Verify a plaintext password against a bcrypt hash; resolves to a boolean. */
export async function checkPassword(pw, hash) {
  return bcrypt.compare(pw, hash);
}

/** Sign a 30-day JWT carrying the user's email. */
export function createToken(email) {
  return jwt.sign({ email }, JWT_SECRET, { expiresIn: "30d" });
}

/** Verify the Bearer token on a request; returns the decoded payload or null. */
export function readToken(request) {
  const h = request.headers.get("authorization");
  if (!h || !h.startsWith("Bearer ")) return null;
  try {
    return jwt.verify(h.slice(7), JWT_SECRET);
  } catch {
    return null;
  }
}
