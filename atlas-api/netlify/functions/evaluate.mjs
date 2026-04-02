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

  const {
    problemDescription,
    referenceSolution,
    testCases,
    gradingRubric,
    concept,
    userCode,
  } = body;
  if (!problemDescription || !referenceSolution || !userCode)
    return fail("problemDescription, referenceSolution, and userCode required");

  /* Build grading context from the pre-established assessment */
  let testCasesStr = "";
  if (testCases?.length) {
    testCasesStr =
      "\n\nTEST CASES (from the answer key):\n" +
      testCases
        .map(
          (tc, i) =>
            `${i + 1}. Input: ${tc.input}\n   Expected: ${tc.expected}` +
            (tc.explanation ? `\n   Purpose: ${tc.explanation}` : ""),
        )
        .join("\n");
  }

  let rubricStr = "";
  if (gradingRubric) {
    rubricStr =
      "\n\nGRADING RUBRIC (established when the problem was created):\n" +
      `- PASS: ${gradingRubric.pass || "Correct output for all test cases"}\n` +
      `- GOOD: ${gradingRubric.good || "Correct + clean code"}\n` +
      `- EXCELLENT: ${gradingRubric.excellent || "Correct + clean + optimal"}`;
  }

  const sys =
    "You are a CS instructor grading a student submission against a PRE-ESTABLISHED " +
    "answer key and rubric. You are NOT inventing criteria.\n\n" +
    "GRADING PROCESS:\n" +
    "1. Trace the student's code with EACH test case input.\n" +
    "2. Compare actual output to expected output from the answer key.\n" +
    "3. Apply the rubric: PASS if all tests pass, GOOD/EXCELLENT based on code quality.\n" +
    "4. If any test fails, it is INCORRECT — explain which and why.\n\n" +
    "RESPONSE FORMAT:\n" +
    "**Grade:** PASS / GOOD / EXCELLENT / INCORRECT\n" +
    "**Test Results:**\n- Test 1: PASS/FAIL (brief)\n" +
    "**Feedback:** What's good and what to improve (2-3 sentences)\n" +
    "**Key Insight:** One takeaway about the concept";

  try {
    const feedback = await complete(
      [
        { role: "system", content: sys },
        {
          role: "user",
          content:
            `PROBLEM:\n${problemDescription}` +
            (concept ? `\n\nCONCEPT: ${concept}` : "") +
            `\n\nREFERENCE SOLUTION:\n\`\`\`\n${referenceSolution}\n\`\`\`` +
            testCasesStr +
            rubricStr +
            `\n\nSTUDENT SUBMISSION:\n\`\`\`\n${userCode}\n\`\`\``,
        },
      ],
      { model: body.model },
    );

    const updated = await useQuota(user.email);
    return json({ feedback, quota: updated });
  } catch (err) {
    return fail(`Evaluation failed: ${err.message}`, 502);
  }
};
