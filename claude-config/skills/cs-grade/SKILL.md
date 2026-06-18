---
name: cs-grade
description: Objectively grade a CS2420 (data structures/algorithms) or CS3500 (object-oriented design) Java project and restructure it to an A+. Use when the user asks to grade, improve, or restructure a CS2420/CS3500 project, mentions getting an A/A+, or references a course rubric. Produces GRADE.md and drives a fix loop.
---

# CS2420 / CS3500 → A+

Drive a Java course project to an A+ on an objective structural rubric.

## The loop

1. **Grade.** From the project root, run:
   ```sh
   gsh grade . --course cs3500      # or --course cs2420, or omit for auto-detect
   ```
   This writes `GRADE.md`: a numeric+letter grade, a per-category scorecard with the
   exact evidence behind each score, and a prioritized **Path to A+** checklist.

2. **Read `GRADE.md`.** Work the checklist top-down — items are ordered by the points
   they recover. Each item is concrete (e.g. "Add Javadoc to every public method",
   "Split god classes", "Program to interfaces").

3. **Fix.** Make the changes for real:
   - **Design (CS3500):** clean MVC separation; program to interfaces; apply patterns
     (Strategy/Command/Factory/Builder/Observer) where they cut coupling; private fields,
     behavior through methods.
   - **Data structures (CS2420):** correct structure per access pattern; document Big-O of
     key operations; include a timing/analysis writeup.
   - **Tests:** a JUnit test class per non-trivial class; assert edge cases and failures.
   - **Docs:** Javadoc on every public class/interface/method (`@param`/`@return`/`@throws`);
     a real README (build/run + design overview); a design/analysis doc.
   - **Style & cleanliness:** no god classes, no long methods, no debug prints, no TODO/
     FIXME, no commented-out code, standard `src/` layout and packages, a build file.

4. **Re-grade.** Run `gsh grade` again. Repeat until the grade is **A+ (≥ 97)**.

## Notes

- The rubric is **structural** — it scores what you can restructure (design, tests, docs,
  style). It does *not* run the course autograder's correctness suite, so always also run
  the official tests for correctness. Don't game the rubric; make the real improvements
  it points to.
- `git-cs-grade --json` emits machine-readable scores for an automated loop.
- For large jobs, delegate to the `cs-grade-improver` subagent (one category per pass).
