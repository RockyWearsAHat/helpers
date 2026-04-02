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
  "testCases",
  "gradingRubric",
  "hints",
];

const DIFFICULTY_GUIDANCE = {
  easy:
    "Difficulty: EASY. One core concept, one function, 5-15 lines of solution logic. " +
    "The problem practically walks the student through it. 2-3 simple test cases. " +
    "Starter code has full signature, docstring, and type hints.",
  medium:
    "Difficulty: MEDIUM. Combine 2+ concepts or add edge cases. 15-40 lines of solution. " +
    "Clear problem but student figures out the strategy. 3-4 test cases with edge cases. " +
    "Starter code has function signatures and brief docstrings.",
  hard:
    "Difficulty: HARD. Non-obvious application or optimization. 30-80 lines of solution. " +
    "Student designs the approach. 4-5 test cases with edge cases and performance bounds. " +
    "Minimal starter code — just function name and parameter types.",
};

const GENERATION_SYSTEM_PROMPT =
  "You are an expert CS instructor who has ALREADY studied the article below. " +
  "The article teaches the concept. You understood it. Now design ONE coding assessment.\n\n" +
  "YOUR PROCESS (follow this order):\n" +
  "1. IDENTIFY the core concept the article teaches.\n" +
  "2. DESIGN a concrete coding task solvable only by understanding that concept. " +
  "The task must have exactly one unambiguous correct behavior for each input.\n" +
  "3. WRITE the reference solution — clean, idiomatic, well-commented.\n" +
  "4. DERIVE test cases FROM the solution. Trace each input through the code.\n" +
  "5. WRITE the grading rubric: pass (correct outputs), good (+ clean code), " +
  "excellent (+ optimal complexity).\n" +
  "6. WRITE 3 progressive hints.\n" +
  "7. WRITE the problem description LAST — include test cases as examples.\n" +
  "8. WRITE starter code with function signature, parameter types, and return type.\n\n" +
  "CRITICAL: The problem MUST be solvable (you proved it). Test cases MUST be " +
  "derived from your solution. The rubric MUST exist before grading.\n\n" +
  "Language: Systems → C/Rust. Web → JS/TS. Algorithms → Python. DB → SQL.\n\n" +
  "Return ONLY valid JSON:\n" +
  '{"title":"...","difficulty":"...","language":"...","allowedLanguages":[...],' +
  '"concept":"specific concept tested",' +
  '"description":"# Title\\n\\n...\\n\\n## Examples\\n\\n...\\n\\n## Constraints\\n\\n...",' +
  '"starterCode":"...","solution":"...",' +
  '"testCases":[{"input":"...","expected":"...","explanation":"..."}],' +
  '"gradingRubric":{"pass":"...","good":"...","excellent":"..."},' +
  '"hints":["...","...","..."]}';

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

  const { articleTitle, articleContent, difficulty, previousExamples } = body;
  if (!articleTitle || !articleContent)
    return fail("articleTitle and articleContent required");

  const diff = difficulty || "medium";
  const diffGuide = DIFFICULTY_GUIDANCE[diff] || DIFFICULTY_GUIDANCE.medium;

  let exCtx = "";
  if (previousExamples?.length) {
    exCtx =
      "\n\nPrevious practice (do NOT repeat):\n" +
      previousExamples
        .slice(-3)
        .map(
          (ex) =>
            `- ${ex.title || ex.type}: ${(ex.description || "").slice(0, 200)}`,
        )
        .join("\n");
  }

  try {
    const raw = await complete(
      [
        { role: "system", content: GENERATION_SYSTEM_PROMPT },
        {
          role: "user",
          content:
            `${diffGuide}\n\nArticle: ${articleTitle}\n\n` +
            `Article content (the textbook):\n${articleContent.slice(0, 6000)}${exCtx}`,
        },
      ],
      { json: true, model: body.model },
    );

    const problem = JSON.parse(raw);
    for (const k of REQUIRED) {
      if (!problem[k]) return fail(`AI response missing "${k}"`, 502);
    }

    /* Validate the assessment is pedagogically complete */
    if (!problem.solution || problem.solution.length < 20) {
      return fail("Generated assessment has no reference solution", 502);
    }
    if (!Array.isArray(problem.testCases) || problem.testCases.length === 0) {
      return fail("Generated assessment has no test cases", 502);
    }

    const updated = await useQuota(user.email);
    return json({ problem, quota: updated });
  } catch (err) {
    return fail(`Generation failed: ${err.message}`, 502);
  }
};
