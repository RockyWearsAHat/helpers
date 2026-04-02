import bcrypt from "bcryptjs";
import jwt from "jsonwebtoken";

const JWT_SECRET = process.env.JWT_SECRET;

export async function hashPassword(pw) {
  return bcrypt.hash(pw, 10);
}

export async function checkPassword(pw, hash) {
  return bcrypt.compare(pw, hash);
}

export function createToken(email) {
  return jwt.sign({ email }, JWT_SECRET, { expiresIn: "30d" });
}

export function readToken(request) {
  const h = request.headers.get("authorization");
  if (!h || !h.startsWith("Bearer ")) return null;
  try {
    return jwt.verify(h.slice(7), JWT_SECRET);
  } catch {
    return null;
  }
}
