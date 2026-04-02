import { readToken } from "./_shared/auth.mjs";
import { getQuota } from "./_shared/quota.mjs";
import { options, json, fail } from "./_shared/response.mjs";

export default async (req) => {
  if (req.method === "OPTIONS") return options();

  const user = readToken(req);
  if (!user) return fail("Unauthorized", 401);

  return json(await getQuota(user.email));
};
