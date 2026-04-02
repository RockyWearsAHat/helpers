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

  const { problemDescription, referenceSolution, userCode } = body;
  if (!problemDescription || !referenceSolution || !userCode)
    return fail("problemDescription, referenceSolution, and userCode required");

  try {
    const feedback = await complete([
      {
        role: "system",
        content:
          "You are a CS instructor evaluating a student's solution. Be encouraging but honest. " +
          "Structure: **Correctness:** (correct/partially correct/incorrect) " +
          "**Feedback:** (what they did well, what could improve) " +
          "**Key Insight:** (one takeaway). Keep it concise (4-6 sentences).",
      },
      {
        role: "user",
        content:
          `Problem: ${problemDescription}\n\n` +
          `Reference:\n\`\`\`\n${referenceSolution}\n\`\`\`\n\n` +
          `Student:\n\`\`\`\n${userCode}\n\`\`\``,
      },
    ]);

    const updated = await useQuota(user.email);
    return json({ feedback, quota: updated });
  } catch (err) {
    return fail(`Evaluation failed: ${err.message}`, 502);
  }
};
