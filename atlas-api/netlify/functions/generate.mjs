import { readToken } from "./_shared/auth.mjs";
import { getQuota, useQuota } from "./_shared/quota.mjs";
import { complete } from "./_shared/ai.mjs";
import { options, json, fail } from "./_shared/response.mjs";

const REQUIRED = [
  "title",
  "difficulty",
  "language",
  "description",
  "starterCode",
  "solution",
  "hints",
];

export default async (req) => {
  if (req.method === "OPTIONS") return options();
  if (req.method !== "POST") return fail("Method not allowed", 405);

  const user = readToken(req);
  if (!user) return fail("Unauthorized", 401);

  const quota = await getQuota(user.email);
  if (quota.remaining <= 0)
    return fail(
      `Quota exceeded (${quota.used}/${quota.limit}). Resets next month.`,
      429,
    );

  const body = await req.json().catch(() => null);
  if (!body) return fail("Invalid JSON");

  const { articleTitle, articleContent, previousExamples } = body;
  if (!articleTitle || !articleContent)
    return fail("articleTitle and articleContent required");

  let exCtx = "";
  if (previousExamples?.length) {
    exCtx =
      "\n\nPrevious practice (avoid repeating):\n" +
      previousExamples
        .slice(-3)
        .map(
          (ex) =>
            `- ${ex.title || ex.type}: ${(ex.description || "").slice(0, 200)}`,
        )
        .join("\n");
  }

  const sys =
    "You are a CS instructor creating coding practice problems. " +
    "Based on the knowledge article, generate ONE coding problem testing core concepts. " +
    "Choose appropriate difficulty and the most suitable language(s). " +
    'Return JSON: {"title":"...","difficulty":"easy|medium|hard",' +
    '"language":"python","allowedLanguages":["python"],' +
    '"description":"Full problem description with examples",' +
    '"starterCode":"// starter code template",' +
    '"solution":"// complete working solution",' +
    '"testDescription":"How to verify correctness",' +
    '"hints":["Hint 1","Hint 2","Hint 3"]}';

  try {
    const raw = await complete(
      [
        { role: "system", content: sys },
        {
          role: "user",
          content: `Article: ${articleTitle}\n\n${articleContent.slice(0, 6000)}${exCtx}`,
        },
      ],
      { json: true },
    );

    const problem = JSON.parse(raw);
    for (const k of REQUIRED) {
      if (!problem[k]) return fail(`AI response missing "${k}"`, 502);
    }

    const updated = await useQuota(user.email);
    return json({ problem, quota: updated });
  } catch (err) {
    return fail(`Generation failed: ${err.message}`, 502);
  }
};
