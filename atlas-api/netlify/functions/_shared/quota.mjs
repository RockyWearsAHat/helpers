import { getStore } from "@netlify/blobs";

const LIMIT = 500;

function monthKey(email) {
  const d = new Date();
  return `${email}:${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
}

export async function getQuota(email) {
  const raw = await getStore("quota").get(monthKey(email));
  const count = raw ? JSON.parse(raw).count : 0;
  return { used: count, remaining: Math.max(0, LIMIT - count), limit: LIMIT };
}

export async function useQuota(email) {
  const store = getStore("quota");
  const key = monthKey(email);
  const raw = await store.get(key);
  const data = raw ? JSON.parse(raw) : { count: 0 };
  data.count += 1;
  data.lastUsed = Date.now();
  await store.set(key, JSON.stringify(data));
  return getQuota(email);
}
