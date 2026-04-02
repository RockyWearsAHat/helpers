import { readToken } from "./_shared/auth.mjs";
import { getQuota, useQuota } from "./_shared/quota.mjs";
import { complete } from "./_shared/ai.mjs";
import { options, json, fail } from "./_shared/response.mjs";

export default async (req) => {
  if (req.method === "OPTIONS") return options();
  if (req.method !== "POST") return fail("Method not allowed", 405);

  const user = readToken(req);
  if (!user) return fail("Unauthorized", 401);

  const quota = await getQuota(user.email);
  if (quota.remaining <= 0) return fail("Quota exceeded", 429);

  const body = await req.json().catch(() => null);
  if (!body) return fail("Invalid JSON");

  const { problemDescription, userCode, hintCount } = body;
  if (!problemDescription) return fail("problemDescription required");

  try {
    const hint = await complete([
      {
        role: "system",
        content:
          "You are a helpful CS tutor. Give a concise, progressive hint. " +
          "Do NOT reveal the full solution. Just nudge the student forward.",
      },
      {
        role: "user",
        content:
          `Problem: ${problemDescription}\n\n` +
          `My code:\n\`\`\`\n${userCode || ""}\n\`\`\`\n\n` +
          `I got ${hintCount || 0} hints already. Give me the next one.`,
      },
    ]);

    const updated = await useQuota(user.email);
    return json({ hint, quota: updated });
  } catch (err) {
    return fail(`Hint failed: ${err.message}`, 502);
  }
};
