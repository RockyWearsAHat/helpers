# Technical Interviews — System Design, Coding Assessments & Evaluation

## The Interview Landscape

Technical hiring uses multiple evaluation methods, each measuring different skills:

| Method                    | Measures                        | Time                  | Bandwidth | Bias Risk              |
| ------------------------- | ------------------------------- | --------------------- | --------- | ---------------------- |
| System design interview    | Architectural thinking, trade-offs | 45-60 min            | High      | Favors experienced hires |
| Live coding               | Problem-solving, syntax, testing | 45-60 min            | High      | Pressure-sensitive; high noise |
| Coding assessment (take-home) | Code quality, self-direction | 1-3 hours (async)    | Low       | Varies; can be noisy if not scoped well |
| Behavioral interview      | Communication, teamwork, growth | 30-45 min            | Medium    | Narrative bias; favors extroverts |
| Pair programming interview | Collaboration, real-world thinking | 45-60 min            | High      | Assesses collaboration, not solo skill |
| Portfolio / GitHub review  | Actual work quality, consistency | Async review (15-30 min) | Low       | Past experience may not transfer |
| Trivia / algorithms quiz   | Memorization, pattern recognition | 30-45 min            | Low       | Weak correlation to job performance |

No single method is sufficient. The best evaluation uses multiple methods to triangulate skill. But multiple interviews create friction (time cost for both candidates and team) and noise (inconsistent interviewer quality).

## System Design Interviews

A system design interview presents an ambiguously-scoped problem ("Design a URL shortening service", "Design a rate limiter") and asks the candidate to propose an architecture.

### What's Being Measured

- **Structured thinking:** Can they break a large problem into smaller components?
- **Trade-off reasoning:** Do they understand performance vs. consistency vs. availability trade-offs?
- **Communication:** Can they explain reasoning clearly, draw diagrams, get feedback?
- **Practical knowledge:** Do they know databases, caching, load balancing, etc.?
- **Adaptability:** When you add a constraint ("now 1 million requests per second"), can they pivot?

### How It Usually Works

1. Interviewer presents problem with ambiguous scope ("You have 45 minutes")
2. Candidate asks clarifying questions
3. Candidate proposes initial design (usually at whiteboard or Google Docs)
4. Interviewer adds constraints or changes requirements
5. Candidate revises

The interview is not a test with a right answer. It's a conversation revealing the candidate's thinking process.

### Common Anti-Patterns

**No clarifications:** Candidate launches into a design without asking what "right" means (latency? consistency? cost?). They guess wrong and waste the interview.

**Premature optimization:** Candidate proposes sharding and caching before establishing scale requirements. Over-engineered toys; they miss the forest for the trees.

**No communication:** Candidate is silent, then reveals their design all at once. Interviewer has no window into their thinking.

**Recitation:** Candidate regurgitates a memorized design ("Like Instagram does it...") without adapting to requirements. Shows memorization, not understanding.

**Evaluation difficulty:** It's hard to objectively score system design. Interviewers disagree on whether a candidate's choice was "good" or "lucky." Standard rubrics help but aren't a panacea.

### Rubric (Simple)

- **Requirements gathering:** Did they ask clarifying questions? (0-2 points)
- **Component breakdown:** Clear separation of concerns? (0-2 points)
- **Trade-off reasoning:** Did they discuss performance, scalability, consistency? (0-3 points)
- **Technical depth:** Database choice, caching, queuing—do they know this domain? (0-2 points)
- **Adaptability:** When constraints changed, could they pivot? (0-1 points)

Interviewers score each independently; recruiters average or discuss mismatches.

## Live Coding Interviews

A live coding interview asks the candidate to solve a problem (often an algorithmic puzzle: "reverse a linked list", "find the kth largest element") while writing actual code, usually in an online editor, speaking aloud about their thinking.

### What's Being Measured

- **Problem-solving process:** Do they break the problem down or jump to code?
- **Language fluency:** Can they write syntactically correct code under pressure?
- **Testing mindset:** Do they consider edge cases or test their solution?
- **Communication:** Can they explain their approach, ask for hints?
- **Velocity:** How quickly do they move?

### Common Issues

**Pressure effects:** Many strong engineers perform terribly in live settings. The social pressure of being watched, time limits, and whiteboard anxiety trigger performance anxiety. Live coding measures a narrow slice: how well someone codes while being observed and timed.

**Artificial problem selection:** Leetcode-style problems (balanced trees, dynamic programming) don't correlate to day-to-day engineering work for most roles. Senior engineers solving these problems may perform worse than junior engineers trained on Leetcode.

**No junior mistakes:** In real work, engineers look things up, Google syntax, iterate. In a live coding interview, looking anything up signals weakness. This teaches bad habits and penalizes appropriate tool use.

**Language and environment:**
- Some candidates are fast in their primary language; slow in less familiar ones
- Unfamiliar online editor (no IDE features) adds friction
- Some engineers think in pseudo-code first (good); the interview expects working code immediately

### Alternative: Reverse Interview

Some companies have the candidate interview the engineer: "Tell me about a hard problem you solved recently; I'll ask questions." This assesses understanding of real-world thinking without artificial constraints.

## Take-Home Coding Assessments

A take-home assessment is a small project sent to the candidate with a 1-3 hour time limit and a written description ("Build a TODO app that...", "Implement a rate limiter that..."). Candidate submits code; engineers review.

### Strengths

- **Real-world conditions:** Candidates have IDE, documentation, Google access. Measures actual working ability, not performance-anxiety ability.
- **Async:** Fits into candidate's schedule better than live interviews.
- **Code review:** You see actual code quality, testing, error handling, not just pseudocode.
- **Depth:** 1-3 hours allows more thoughtful work than 45-minute live coding.

### Weaknesses

- **Scope creep:** Candidates who don't understand the 1-3 hour time estimate may spend 8 hours or nothing, wasting time or producing incomplete work.
- **Collaboration invisible:** Did they pair? Ask a friend? Only show what they could do alone. You don't know.
- **Reviewer subjectivity:** It's hard to objectively score code. One reviewer finds a solution elegant; another finds it over-engineered. Lack of consistency.
- **Dropout risk:** Some candidates drop out of the process because they don't want to "do homework" after work. Selection bias toward candidates with more time.

### Making It Work

- **Clear scope:** "This should take 2-3 hours. You have 3 days." Give a clear time window and expected effort.
- **Reasonable problem:** Not a mini real-world system; small enough to complete, complex enough to reveal thinking
- **Rubric:** Share the rubric upfront so candidates know what you're evaluating
- **Multiple reviewers:** Two engineers independently review; discuss differences. Reduces individual bias
- **Follow-up interview:** Use the submitted code as basis for a code review conversation. Learn their reasoning.

## Behavioral Interviews

A behavioral interview focuses on soft skills, communication, and how the candidate responds to challenging situations. Format: "Tell me about a time you..."

### What It Measures

- **Communication:** Can they articulate experiences clearly?
- **Teamwork:** How do they collaborate? Do they blame others or own mistakes?
- **Growth mindset:** Can they reflect on failures and learn?
- **Problem-solving under ambiguity:** How do they handle unclear requirements?

### Challenges

- **Narrative bias:** Candidates who interview well may be better storytellers than engineers. The STAR method (Situation, Task, Action, Result) is taught in every interview prep guide; everyone gives STAR answers. Hard to distinguish.
- **Consistency:** Interviewers have different follow-up questions, different standards for "good" answers. One interviewer is strict, another lenient.
- **Selection effects:** Behavioral interviews sometimes screen for conformity and communication style, not competence. Introverts, neurodiverse people, non-native speakers may struggle to present experiences in the expected format.

### Rubric (Simple)

- **Clarity:** Was the story understandable? (0-2)
- **Specificity:** Concrete details or vague generalizations? (0-2)
- **Reflection:** Did they learn from the experience? (0-1)
- **Teamwork signal:** Did they own actions or blame circumstances? (0-1)

Pair with other interviews to avoid over-weighting communication skills.

## Pair Programming Interviews

A pair programming interview asks the candidate to code alongside an engineer, solving a real problem collaboratively. The focus is on how they communicate, collaborate, and handle feedback in real-time.

### Strengths

- **Reveals collaboration style:** Are they defensive or open to suggestions? Do they explain reasoning?
- **Lower pressure:** It's a conversation, not a performance. Candidates often feel more comfortable.
- **Real work simulation:** Most engineering is collaborative; this tests that directly.

### Weaknesses

- **Hides solo skill:** You don't see how they'd work independently or debug alone
- **Interviewer effect:** Passive interviewers who don't guide; active interviewers who "lead the witness." Inconsistency.
- **Time-consuming:** Requires both candidate and engineer to collaborate for 45-60 minutes

## Structured Interviewing and Consistency

Unstructured interviews (different questions, different rubrics, interviewer impressions) show high variability and bias. Teams that standardize produce better hiring:

- **Standard questions:** All candidates for the same role get the same system design prompt or coding problem
- **Shared rubric:** Consistent evaluation criteria used across all interviews
- **Trained interviewers:** Explicit training on what to evaluate, how to score, avoiding common biases
- **Calibration sessions:** Interviewers review old interviews and calibrate scoring
- **Blind scores first:** Interviewers score independently before discussing; prevents one strong voice from anchoring the group

## Bias Mitigation

Technical interviews inherently contain bias traps:

- **Availability bias:** Recent performance in the interview dominates over careful scoring
- **Confirmation bias:** Finding evidence that supports your first impression
- **In-group bias:** Favoring candidates who are similar to current team (similar background, communication style, experience level)
- **Pressure under observation:** Candidates with test-taking experience, those from privileged backgrounds with interview coaching, perform better even with equal ability

**Mitigations:**
- Multiple interviewers, independent scoring, then discussion
- Structure and rubrics reduce individual judgment
- Diverse interview panels (different perspectives, backgrounds)
- Calibration sessions to check for systematic bias (e.g., "Are we scoring women lower on communication?")
- Offer support or accommodations (extra time for anxious candidates, familiar IDEs, etc.)

## Candidate Experience

Good companies treat interview processes as outreach, not obstacle courses. Even rejected candidates should come away thinking "That company is professional and respectful."

- **Clear communication:** Describe the process upfront so candidates know what to expect
- **Timely feedback:** Don't leave candidates hanging for weeks
- **Respectful timing:** No interviews outside working hours unless the candidate volunteers
- **Reasonable scope:** Take-home should be 2-3 hours, not 8+
- **Feedback for rejected candidates:** "You were strong in system design but we saw gaps in async programming" beats silence

## Red Flags in Interview Performance

These don't necessarily disqualify, but they're worth investigating:

- **Inflexibility:** Proposed one design; couldn't adapt when requirements changed
- **No testing mindset:** Submitted code with obvious edge cases not handled
- **Blame-shifting:** When something went wrong, attributed it to tools/teammates rather than thinking about their own role
- **No questions:** If they ask no clarifying questions (system design) or no follow-up questions (behavioral), that's unusual
- **Communication breakdown:** Can't explain reasoning; Interviewer has to guess what they meant

## See Also

- [System Design & Distributed Systems — From Single Box to Planet Scale](system-design-distributed.md) — Technical depth for system design thinking
- [Process Code Review](process-code-review.md) — Post-hire: evaluating real work
- [Process Pair Programming](process-pair-programming.md) — Pairing as collaboration, not interview