/* ====================================================================
 * Atlas Practice Module
 * Interactive code editor, local AI (Ollama) integration, problem
 * generation, hint system, solution evaluation, and examples storage.
 * ==================================================================== */
(function () {
  "use strict";

  /* -- Configuration --------------------------------------------------- */
  /* HOSTED_API_BASE comes from env.js (gitignored) via window.ATLAS_API_BASE */
  var HOSTED_API_BASE = window.ATLAS_API_BASE || "";

  var config = {
    ollamaBase: localStorage.getItem("practice-ollama-url") || "http://localhost:11434",
    model: localStorage.getItem("practice-model") || "qwen3.5:30b",
    autoSetup: localStorage.getItem("practice-auto-setup") === "true",
    apiBase: localStorage.getItem("practice-api-base") || HOSTED_API_BASE,
    difficulty: localStorage.getItem("practice-difficulty") || "medium",
    hostedModel: localStorage.getItem("practice-hosted-model") || "gpt-4o-mini",
  };

  /* -- Module state ---------------------------------------------------- */
  var ps = {
    active: false,
    articleId: null,
    articleContent: "",
    articleTitle: "",
    currentProblem: null,
    editor: null,
    hints: [],
    hintIndex: 0,
    db: null,
    aiConnected: false,
    generating: false,
    activeTab: "problem",
  };

  /* ==================================================================
   * IndexedDB Storage
   * ================================================================== */
  var DB_NAME = "atlas-practice";
  var DB_VERSION = 1;

  function openDb() {
    return new Promise(function (resolve, reject) {
      if (ps.db) return resolve(ps.db);
      var request = indexedDB.open(DB_NAME, DB_VERSION);
      request.onupgradeneeded = function (event) {
        var db = event.target.result;
        if (!db.objectStoreNames.contains("problems")) {
          var pStore = db.createObjectStore("problems", {
            keyPath: "id",
            autoIncrement: true,
          });
          pStore.createIndex("articleId", "articleId", { unique: false });
        }
        if (!db.objectStoreNames.contains("solutions")) {
          var sStore = db.createObjectStore("solutions", {
            keyPath: "id",
            autoIncrement: true,
          });
          sStore.createIndex("problemId", "problemId", { unique: false });
          sStore.createIndex("articleId", "articleId", { unique: false });
        }
        if (!db.objectStoreNames.contains("examples")) {
          var eStore = db.createObjectStore("examples", {
            keyPath: "id",
            autoIncrement: true,
          });
          eStore.createIndex("articleId", "articleId", { unique: false });
          eStore.createIndex("type", "type", { unique: false });
        }
      };
      request.onsuccess = function (event) {
        ps.db = event.target.result;
        resolve(ps.db);
      };
      request.onerror = function () {
        reject(request.error);
      };
    });
  }

  function dbPut(storeName, item) {
    return openDb().then(function (db) {
      return new Promise(function (resolve, reject) {
        var tx = db.transaction(storeName, "readwrite");
        var store = tx.objectStore(storeName);
        var req = store.put(item);
        req.onsuccess = function () {
          resolve(req.result);
        };
        req.onerror = function () {
          reject(req.error);
        };
      });
    });
  }

  function dbGetByIndex(storeName, indexName, key) {
    return openDb().then(function (db) {
      return new Promise(function (resolve, reject) {
        var tx = db.transaction(storeName, "readonly");
        var store = tx.objectStore(storeName);
        var index = store.index(indexName);
        var req = index.getAll(key);
        req.onsuccess = function () {
          resolve(req.result);
        };
        req.onerror = function () {
          reject(req.error);
        };
      });
    });
  }

  function dbGetAll(storeName) {
    return openDb().then(function (db) {
      return new Promise(function (resolve, reject) {
        var tx = db.transaction(storeName, "readonly");
        var req = tx.objectStore(storeName).getAll();
        req.onsuccess = function () {
          resolve(req.result);
        };
        req.onerror = function () {
          reject(req.error);
        };
      });
    });
  }

  /* ==================================================================
   * Hosted API Client
   * ================================================================== */
  var hosted = {
    token: localStorage.getItem("practice-hosted-token") || "",
    email: localStorage.getItem("practice-hosted-email") || "",
  };

  function hostedRequest(endpoint, body) {
    body.model = config.hostedModel;
    return fetch(config.apiBase + "/.netlify/functions/" + endpoint, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": "Bearer " + hosted.token,
      },
      body: JSON.stringify(body),
    }).then(function (res) {
      return res.json().then(function (data) {
        if (!res.ok) {
          if (res.status === 401) { hostedLogout(); }
          throw new Error(data.error || "Request failed");
        }
        if (data.quota) updateQuotaDisplay(data.quota);
        return data;
      });
    });
  }

  function hostedAuth(endpoint, email, password) {
    return fetch(config.apiBase + "/.netlify/functions/" + endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email: email, password: password }),
    }).then(function (res) {
      return res.json().then(function (data) {
        if (!res.ok) throw new Error(data.error || "Auth failed");
        hosted.token = data.token;
        hosted.email = data.email;
        localStorage.setItem("practice-hosted-token", data.token);
        localStorage.setItem("practice-hosted-email", data.email);
        return data;
      });
    });
  }

  function hostedLogout() {
    hosted.token = "";
    hosted.email = "";
    localStorage.removeItem("practice-hosted-token");
    localStorage.removeItem("practice-hosted-email");
  }

  function isHostedAuthenticated() {
    return !!(hosted.token && hosted.email && config.apiBase);
  }

  function updateQuotaDisplay(quota) {
    var el = document.getElementById("quota-display");
    if (!el) return;
    el.textContent = quota.remaining + "/" + quota.limit + " left";
    el.classList.toggle("quota-low", quota.remaining < 50);
    el.classList.remove("hidden");
  }

  /* ==================================================================
   * AI Client — Ollama
   * ================================================================== */

  /**
   * Strip thinking-model tags and extract a JSON object from an AI response.
   * Handles <think>...</think> blocks, markdown code fences, and extra text
   * around the JSON payload.
   */
  function parseJsonResponse(text) {
    var cleaned = text
      .replace(/<think>[\s\S]*?<\/think>/gi, "")
      .replace(/^```(?:json)?\s*/, "")
      .replace(/```\s*$/, "")
      .trim();
    try {
      return JSON.parse(cleaned);
    } catch (_e) {
      var match = cleaned.match(/\{[\s\S]*\}/);
      if (match) {
        try { return JSON.parse(match[0]); } catch (_e2) { /* fall through */ }
      }
      return null;
    }
  }

  function checkConnection() {
    return fetch(config.ollamaBase + "/api/tags", {
      signal: AbortSignal.timeout(3000),
      cache: "no-store",
    })
      .then(function (res) {
        if (!res.ok) return { connected: false, models: [] };
        return res.json().then(function (data) {
          var models = (data.models || []).map(function (m) {
            return m.name;
          });
          return { connected: true, models: models };
        });
      })
      .catch(function (err) {
        var errorType = "unreachable";
        if (err && err.name === "AbortError") {
          errorType = "timeout";
        } else if (
          window.location.protocol === "https:" &&
          /^http:\/\//i.test(config.ollamaBase)
        ) {
          errorType = "origin";
        }
        return {
          connected: false,
          models: [],
          errorType: errorType,
          errorMessage: err && err.message ? err.message : "",
        };
      });
  }

  function chatComplete(messages, options) {
    var streaming = options && typeof options.onChunk === "function";
    return fetch(config.ollamaBase + "/api/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: config.model,
        messages: messages,
        stream: streaming,
        think: false,
        options: { temperature: 0.7 },
      }),
    }).then(function (res) {
      if (!res.ok)
        throw new Error("AI request failed (" + res.status + ")");
      if (!streaming) {
        return res.json().then(function (data) {
          return data.message.content;
        });
      }
      var reader = res.body.getReader();
      var decoder = new TextDecoder();
      var fullText = "";
      function pump() {
        return reader.read().then(function (result) {
          if (result.done) return fullText;
          var chunk = decoder.decode(result.value, { stream: true });
          chunk
            .split("\n")
            .filter(Boolean)
            .forEach(function (line) {
              try {
                var parsed = JSON.parse(line);
                if (parsed.message && parsed.message.content) {
                  fullText += parsed.message.content;
                  options.onChunk(parsed.message.content, fullText);
                }
              } catch (_e) {
                /* skip malformed line */
              }
            });
          return pump();
        });
      }
      return pump();
    });
  }

  /* ==================================================================
   * Problem Generation
   * ================================================================== */

  /* ------------------------------------------------------------------ *
   * Difficulty scoping — controls solution complexity, NOT concept.     *
   * The concept comes from the article. Difficulty controls how many    *
   * steps and edge cases the student must handle.                       *
   * ------------------------------------------------------------------ */
  var DIFFICULTY_GUIDANCE = {
    easy:
      "Difficulty: EASY. " +
      "One core concept from the article, one function, 5-15 lines of solution logic. " +
      "The problem statement practically walks the student through it — " +
      "they need to translate clear English into code. " +
      "Include 2-3 simple test cases with expected outputs. " +
      "Starter code has the full function signature, docstring, and type hints.",
    medium:
      "Difficulty: MEDIUM. " +
      "Combine 2+ concepts from the article, or add realistic edge cases. " +
      "15-40 lines of solution logic. " +
      "The problem statement is clear but doesn't hand-hold the approach — " +
      "the student must figure out the strategy. " +
      "Include 3-4 test cases with at least one edge case. " +
      "Starter code has function signature(s) and brief docstrings.",
    hard:
      "Difficulty: HARD. " +
      "Apply the article's concepts to a non-obvious scenario, or require optimization. " +
      "30-80 lines of solution logic. " +
      "The problem gives context and constraints but the student designs the approach. " +
      "Include 4-5 test cases including edge cases and performance bounds. " +
      "Starter code is minimal — just the function name and parameter types.",
  };

  /* ------------------------------------------------------------------ *
   * CORE PEDAGOGICAL PROMPT                                            *
   *                                                                    *
   * This is the most important string in the practice module.          *
   * The article is the textbook. The AI is the professor who already   *
   * read and understood the textbook. It designs the assessment as a   *
   * single coherent unit: problem + solution + test cases + rubric.    *
   * Nothing is generated without its answer key. Nothing is graded     *
   * without pre-established criteria.                                  *
   * ------------------------------------------------------------------ */
  var GENERATION_SYSTEM_PROMPT =
    "You are an expert CS instructor who has ALREADY studied the article below. " +
    "The article teaches the concept. You understood it. Now design ONE coding assessment.\n\n" +
    "YOUR PROCESS (follow this order — it is how good teachers design exams):\n" +
    "1. IDENTIFY the core concept the article teaches.\n" +
    "2. DESIGN a concrete coding task that can ONLY be solved correctly by someone who " +
    "understood that concept. The task must have exactly one unambiguous correct behavior " +
    "for each input.\n" +
    "3. WRITE the reference solution — clean, idiomatic, well-commented code that " +
    "demonstrates best practices. This is the answer key.\n" +
    "4. DERIVE test cases FROM the solution. Run the solution mentally against each input " +
    "and record the exact expected output. Every test case must be verified against your " +
    "solution — never invent an expected output without tracing through the code.\n" +
    "5. WRITE the grading rubric: what makes a submission correct (must produce identical " +
    "outputs for all test cases), and what distinguishes good from great (style, efficiency, " +
    "edge case handling).\n" +
    "6. WRITE 3 progressive hints that guide WITHOUT giving away the solution.\n" +
    "7. WRITE the problem description LAST — now that you know exactly what the student " +
    "must produce, describe the task clearly. Include the test cases as examples so " +
    "the student can self-check.\n" +
    "8. WRITE starter code that gives the student the function signature, parameter types, " +
    "and return type so they know the exact contract to fulfill.\n\n" +
    "CRITICAL RULES:\n" +
    "- The problem MUST be solvable. You proved it is solvable by writing the solution.\n" +
    "- The test cases MUST be derived from YOUR solution, not invented independently.\n" +
    "- The grading rubric MUST exist BEFORE the student sees the problem.\n" +
    "- Every test case has an exact expected output — no ambiguity.\n" +
    "- The problem guides the student toward the right answer. It should make them think, " +
    "but it should always have a clear path from A to B.\n\n" +
    "LANGUAGE SELECTION: Analyze the article topic. Systems/OS → C or Rust. " +
    "Web/DOM/API → JavaScript/TypeScript. Algorithms/data → Python. " +
    "Database → SQL. Choose what fits naturally.\n\n" +
    "Return ONLY valid JSON (no markdown fencing, no explanation) in this EXACT format:\n" +
    "{\n" +
    '  "title": "Concise descriptive title",\n' +
    '  "difficulty": "easy|medium|hard",\n' +
    '  "language": "python",\n' +
    '  "allowedLanguages": ["python", "javascript"],\n' +
    '  "concept": "The specific concept from the article being tested",\n' +
    '  "description": "# Title\\n\\nClear problem statement...\\n\\n## Examples\\n\\n' +
    "**Input:** `example_input`\\n**Output:** `expected_output`\\n\\n" +
    '## Constraints\\n\\n- constraint 1\\n- constraint 2",\n' +
    '  "starterCode": "def solve(...): ...\\n    pass",\n' +
    '  "solution": "def solve(...):\\n    # complete working solution with comments",\n' +
    '  "testCases": [\n' +
    '    {"input": "describe the input", "expected": "exact expected output", "explanation": "why this tests the concept"},\n' +
    '    {"input": "edge case input", "expected": "exact expected output", "explanation": "what edge case this covers"}\n' +
    "  ],\n" +
    '  "gradingRubric": {\n' +
    '    "pass": "Produces correct output for all test cases",\n' +
    '    "good": "Correct + clean code structure and naming",\n' +
    '    "excellent": "Correct + clean + optimal time/space complexity or demonstrates deeper understanding"\n' +
    "  },\n" +
    '  "hints": ["Conceptual nudge", "Approach suggestion", "Near-solution technique hint"]\n' +
    "}";

  function generateProblem() {
    if (ps.generating || !ps.articleId) return Promise.resolve();
    ps.generating = true;
    ps.currentProblem = null;
    ps.hints = [];
    ps.hintIndex = 0;
    updateActionButtons();
    var display = document.getElementById("problem-display");
    display.innerHTML =
      '<div class="practice-loading">' +
      '<div class="typing-dots"><span></span><span></span><span></span></div>' +
      "<p>Designing " + config.difficulty + " assessment\u2026</p></div>";

    return dbGetByIndex("examples", "articleId", ps.articleId)
      .then(function (examples) {
        var recent = examples.slice(-3);

        /* ---- Hosted mode: server does the AI call ---- */
        if (activeProvider === "hosted") {
          return hostedRequest("generate", {
            articleTitle: ps.articleTitle,
            articleContent: ps.articleContent.slice(0, 6000),
            difficulty: config.difficulty,
            previousExamples: recent.map(function (ex) {
              return { title: ex.title, type: ex.type, description: (ex.description || "").slice(0, 200) };
            }),
          }).then(function (data) { return data.problem; });
        }

        /* ---- Local mode: Ollama direct ---- */
        var exCtx = "";
        if (recent.length > 0) {
          exCtx =
            "\n\nPrevious practice problems for this article (do NOT repeat these):\n" +
            recent
              .map(function (ex) {
                return "- " + (ex.title || ex.type) + ": " + (ex.description || "").slice(0, 200);
              })
              .join("\n");
        }

        var diffGuide = DIFFICULTY_GUIDANCE[config.difficulty] || DIFFICULTY_GUIDANCE.medium;

        var userPrompt =
          diffGuide + "\n\n" +
          "Article title: " + ps.articleTitle + "\n\n" +
          "Article content (this is the textbook — the concept is already explained here):\n" +
          ps.articleContent.slice(0, 6000) + exCtx;

        return chatComplete([
          { role: "system", content: GENERATION_SYSTEM_PROMPT },
          { role: "user", content: userPrompt },
        ]).then(function (response) {
          var problem = parseJsonResponse(response);
          if (!problem) throw new Error("AI did not return valid JSON");
          /* Validate the assessment is complete — reject if missing answer key */
          if (!problem.solution || problem.solution.length < 20) {
            throw new Error("Generated assessment has no reference solution — cannot grade without an answer key");
          }
          if (!problem.testCases || !problem.testCases.length) {
            throw new Error("Generated assessment has no test cases — cannot verify correctness");
          }
          return problem;
        });
      })
      .then(function (problem) {
        problem.articleId = ps.articleId;
        problem.createdAt = Date.now();
        ps.currentProblem = problem;
        ps.hints = problem.hints || [];
        ps.hintIndex = 0;

        return dbPut("problems", problem).then(function () {
          renderProblem(problem);
          updateActionButtons();
          updateStatusBar(problem);
          setEditorLanguage(problem.language || "python");
          vfsInitForProblem(problem);
          if (problem.starterCode && ps.editor) {
            ps.editor.setValue(problem.starterCode);
          }
          /* Cache as example to avoid regenerating similar problems */
          return dbPut("examples", {
            articleId: ps.articleId,
            type: "problem",
            title: problem.title,
            description: problem.description,
            code: problem.solution,
            language: problem.language || "python",
            allowedLanguages: problem.allowedLanguages || [problem.language || "python"],
            tags: ["ai-generated", problem.difficulty || "medium"],
            source: "ai-generated",
            createdAt: Date.now(),
          });
        });
      })
      .catch(function (err) {
        display.innerHTML =
          '<div class="practice-error">' +
          "<p>Failed to generate problem: " +
          esc(err.message) + "</p>" +
          '<button class="btn btn-primary btn-sm" id="retry-generate-btn">Try Again</button></div>';
        var retryBtn = document.getElementById("retry-generate-btn");
        if (retryBtn) retryBtn.addEventListener("click", generateProblem);
      })
      .finally(function () {
        ps.generating = false;
        updateActionButtons();
      });
  }

  /* ==================================================================
   * Hints
   * ================================================================== */
  function getHint() {
    if (!ps.currentProblem) return;
    var hintDisplay = document.getElementById("hint-display");
    if (!hintDisplay) return;

    if (ps.hintIndex < ps.hints.length) {
      var hint = ps.hints[ps.hintIndex];
      ps.hintIndex++;
      var hintHtml =
        '<div class="hint-card">' +
        '<span class="hint-badge">Hint ' + ps.hintIndex + "/" + ps.hints.length + "</span>" +
        "<p>" + esc(hint) + "</p></div>";
      hintDisplay.innerHTML += hintHtml;
      hintDisplay.classList.remove("hidden");
      updateActionButtons();
      return;
    }

    hintDisplay.classList.remove("hidden");
    ps.generating = true;
    updateActionButtons();
    hintDisplay.innerHTML +=
      '<div class="hint-card hint-loading">' +
      '<div class="typing-dots"><span></span><span></span><span></span></div></div>';

    var userCode = ps.editor ? ps.editor.getValue() : "";
    var hintPromise;

    if (activeProvider === "hosted") {
      hintPromise = hostedRequest("hint", {
        problemDescription: ps.currentProblem.description,
        userCode: userCode,
        hintCount: ps.hintIndex,
      }).then(function (data) { return data.hint; });
    } else {
      hintPromise = chatComplete(
        [
          {
            role: "system",
            content:
              "You are a helpful CS tutor. Give a concise, progressive hint " +
              "for the problem below. Do NOT reveal the full solution. " +
              "Just nudge the student in the right direction.",
          },
          {
            role: "user",
            content:
              "Problem: " + ps.currentProblem.description +
              "\n\nMy current code:\n```\n" + userCode + "\n```\n\n" +
              "I already got " + ps.hintIndex + " hints. Give me the next one.",
          },
        ],
        {
          onChunk: function (_chunk, full) {
            var cards = hintDisplay.querySelectorAll(".hint-card");
            var last = cards[cards.length - 1];
            if (last) {
              last.classList.remove("hint-loading");
              last.innerHTML =
                '<span class="hint-badge">AI Hint</span><p>' + esc(full) + "</p>";
            }
          },
        },
      );
    }

    hintPromise
      .then(function (full) {
        var cards = hintDisplay.querySelectorAll(".hint-card");
        var last = cards[cards.length - 1];
        if (last) {
          last.classList.remove("hint-loading");
          last.innerHTML =
            '<span class="hint-badge">AI Hint</span><p>' + esc(full) + "</p>";
        }
      })
      .catch(function (err) {
        var cards = hintDisplay.querySelectorAll(".hint-card");
        var last = cards[cards.length - 1];
        if (last) {
          last.classList.remove("hint-loading");
          last.innerHTML = "<p>Hint failed: " + esc(err.message) + "</p>";
        }
      })
      .finally(function () {
        ps.generating = false;
        updateActionButtons();
      });
  }

  /* ==================================================================
   * Solution Submission & Evaluation
   *
   * Grading uses the RUBRIC and TEST CASES that were generated WITH
   * the problem — not invented after the fact. The AI evaluator
   * receives the answer key, the test cases, the rubric, AND the
   * student's code. It grades against pre-established criteria.
   * ================================================================== */
  function submitSolution() {
    if (!ps.currentProblem || !ps.editor) return;
    var userCode = ps.editor.getValue().trim();
    if (!userCode) return;

    ps.generating = true;
    updateActionButtons();
    var feedbackEl = document.getElementById("feedback-display");
    feedbackEl.classList.remove("hidden");
    feedbackEl.innerHTML =
      '<div class="feedback-card feedback-loading">' +
      '<div class="typing-dots"><span></span><span></span><span></span></div>' +
      "<p>Grading against rubric\u2026</p></div>";

    /* Build the grading context from the pre-established assessment */
    var prob = ps.currentProblem;
    var testCasesStr = "";
    if (prob.testCases && prob.testCases.length) {
      testCasesStr = "\n\nTEST CASES (from the answer key):\n" +
        prob.testCases.map(function (tc, i) {
          return (i + 1) + ". Input: " + tc.input +
            "\n   Expected output: " + tc.expected +
            (tc.explanation ? "\n   Purpose: " + tc.explanation : "");
        }).join("\n");
    }

    var rubricStr = "";
    if (prob.gradingRubric) {
      rubricStr = "\n\nGRADING RUBRIC (established when the problem was created):\n" +
        "- PASS: " + (prob.gradingRubric.pass || "Correct output for all test cases") + "\n" +
        "- GOOD: " + (prob.gradingRubric.good || "Correct + clean code") + "\n" +
        "- EXCELLENT: " + (prob.gradingRubric.excellent || "Correct + clean + optimal");
    }

    var evalSysPrompt =
      "You are a CS instructor grading a student submission against a PRE-ESTABLISHED answer key and rubric. " +
      "You are NOT inventing criteria — the criteria were written BEFORE the student saw the problem.\n\n" +
      "YOUR GRADING PROCESS:\n" +
      "1. Trace through the student's code with EACH test case input. Determine the actual output.\n" +
      "2. Compare each actual output to the expected output from the answer key.\n" +
      "3. Apply the rubric: PASS if all test cases produce correct output, GOOD or EXCELLENT based on code quality.\n" +
      "4. If any test case fails, the submission is INCORRECT — explain which test case(s) failed and why.\n\n" +
      "RESPONSE FORMAT (use exactly this structure):\n" +
      "**Grade:** PASS / GOOD / EXCELLENT / INCORRECT\n" +
      "**Test Results:**\n" +
      "- Test 1: PASS/FAIL (brief explanation)\n" +
      "- Test 2: PASS/FAIL (brief explanation)\n" +
      "**Feedback:** What the student did well and what to improve (2-3 sentences)\n" +
      "**Key Insight:** One takeaway about the underlying concept";

    var evalUserPrompt =
      "PROBLEM:\n" + prob.description +
      "\n\nCONCEPT BEING TESTED: " + (prob.concept || prob.title) +
      "\n\nREFERENCE SOLUTION (the answer key):\n```\n" + prob.solution + "\n```" +
      testCasesStr +
      rubricStr +
      "\n\nSTUDENT SUBMISSION:\n```\n" + userCode + "\n```";

    var feedbackPromise;

    if (activeProvider === "hosted") {
      feedbackPromise = hostedRequest("evaluate", {
        problemDescription: prob.description,
        referenceSolution: prob.solution,
        testCases: prob.testCases || [],
        gradingRubric: prob.gradingRubric || {},
        concept: prob.concept || "",
        userCode: userCode,
      }).then(function (data) { return data.feedback; });
    } else {
      feedbackPromise = chatComplete(
        [
          { role: "system", content: evalSysPrompt },
          { role: "user", content: evalUserPrompt },
        ],
        {
          onChunk: function (_chunk, full) {
            feedbackEl.innerHTML =
              '<div class="feedback-card">' + renderMiniMarkdown(full) + "</div>";
          },
        },
      );
    }

    feedbackPromise
      .then(function (fullResponse) {
        feedbackEl.innerHTML =
          '<div class="feedback-card">' + renderMiniMarkdown(fullResponse) + "</div>";

        /* Determine grade from structured response */
        var gradeMatch = fullResponse.match(/\*\*Grade:\*\*\s*(PASS|GOOD|EXCELLENT|INCORRECT)/i);
        var grade = gradeMatch ? gradeMatch[1].toUpperCase() : "UNKNOWN";
        var isCorrect = grade === "PASS" || grade === "GOOD" || grade === "EXCELLENT";

        return dbPut("solutions", {
          articleId: ps.articleId,
          problemId: prob.id,
          code: userCode,
          correct: isCorrect,
          grade: grade,
          feedback: fullResponse,
          createdAt: Date.now(),
        }).then(function () {
          if (isCorrect) {
            dbPut("examples", {
              articleId: ps.articleId,
              type: "solved",
              title: (prob.title || "Submission") + " (" + grade.toLowerCase() + ")",
              description: (fullResponse || "").slice(0, 500),
              code: userCode,
              language: prob.language || "python",
              tags: [grade.toLowerCase(), "user-submitted"],
              source: "user-submitted",
              createdAt: Date.now(),
            }).catch(function () { /* best-effort */ });
            return generateInsight(userCode, fullResponse);
          }
        });
      })
      .catch(function (err) {
        feedbackEl.innerHTML =
          '<div class="feedback-card feedback-error">' +
          "<p>Evaluation failed: " + esc(err.message) + "</p></div>";
      })
      .finally(function () {
        ps.generating = false;
        updateActionButtons();
      });
  }

  /* ==================================================================
   * Insight Generation (feedback loop for KB/CC examples)
   * ================================================================== */
  function generateInsight(userCode, feedback) {
    return chatComplete([
      {
        role: "system",
        content:
          "You are a knowledge curator. Based on a successful student solution " +
          "and instructor feedback, extract a reusable coding example that " +
          "demonstrates the concept. Return ONLY valid JSON:\n" +
          '{"title":"Example title","description":"What this example demonstrates",' +
          '"code":"// clean, commented example code",' +
          '"language":"python","tags":["tag1","tag2"],' +
          '"conceptInsight":"One sentence about the deeper concept demonstrated"}',
      },
      {
        role: "user",
        content:
          "Article: " + ps.articleTitle +
          "\nProblem: " + (ps.currentProblem ? ps.currentProblem.title : "") +
          "\nStudent code:\n```\n" + userCode +
          "\n```\nFeedback: " + feedback,
      },
    ]).then(function (response) {
      var insight = parseJsonResponse(response);
      if (!insight) return;
      return dbPut("examples", {
        articleId: ps.articleId,
        type: "insight",
        title: insight.title,
        description: insight.description,
        code: insight.code,
        language: insight.language || "python",
        tags: insight.tags || [],
        source: "user-solved",
        conceptInsight: insight.conceptInsight,
        createdAt: Date.now(),
      });
    }).catch(function () {
      /* insight generation is best-effort */
    });
  }

  /* ==================================================================
   * Show Solution
   * ================================================================== */
  function showSolution() {
    if (!ps.currentProblem) return;
    var feedbackEl = document.getElementById("feedback-display");
    feedbackEl.classList.remove("hidden");
    feedbackEl.innerHTML =
      '<div class="feedback-card solution-reveal">' +
      '<span class="solution-badge">Reference Solution</span>' +
      '<pre><code class="language-' +
      esc(ps.currentProblem.language || "python") + '">' +
      esc(ps.currentProblem.solution) +
      "</code></pre>" +
      (ps.currentProblem.testDescription
        ? "<p><strong>Verification:</strong> " +
          esc(ps.currentProblem.testDescription) + "</p>"
        : "") +
      "</div>";
    if (typeof hljs !== "undefined") {
      feedbackEl.querySelectorAll("pre code").forEach(function (block) {
        hljs.highlightElement(block);
      });
    }
  }

  /* ==================================================================
   * CodeMirror Editor
   * ================================================================== */
  function initEditor() {
    if (ps.editor) return;
    var textarea = document.getElementById("code-editor");
    if (!textarea || typeof CodeMirror === "undefined") return;

    ps.editor = CodeMirror.fromTextArea(textarea, {
      lineNumbers: true,
      matchBrackets: true,
      autoCloseBrackets: true,
      indentUnit: 4,
      tabSize: 4,
      indentWithTabs: false,
      lineWrapping: false,
      theme: getEditorTheme(),
      mode: "python",
      extraKeys: {
        Tab: function (cm) {
          if (cm.somethingSelected()) {
            cm.indentSelection("add");
          } else {
            cm.replaceSelection(
              Array(cm.getOption("indentUnit") + 1).join(" "),
              "end",
            );
          }
        },
      },
    });
    ps.editor.setSize("100%", "300px");
  }

  function getEditorTheme() {
    var theme = document.documentElement.getAttribute("data-theme");
    return theme === "dark" ? "atlas-dark" : "atlas-light";
  }

  function syncEditorTheme() {
    if (ps.editor) ps.editor.setOption("theme", getEditorTheme());
  }

  function setEditorLanguage(lang) {
    if (!ps.editor) return;
    var modeMap = {
      python: "python",
      javascript: "javascript",
      js: "javascript",
      typescript: "javascript",
      ts: "javascript",
      java: "text/x-java",
      c: "text/x-csrc",
      cpp: "text/x-c++src",
      "c++": "text/x-c++src",
      csharp: "text/x-csharp",
      "c#": "text/x-csharp",
      go: "text/x-go",
      rust: "text/x-rustsrc",
      sql: "text/x-sql",
      bash: "text/x-sh",
      shell: "text/x-sh",
      sh: "text/x-sh",
    };
    var mode = modeMap[(lang || "python").toLowerCase()] || "python";
    ps.editor.setOption("mode", mode);
  }

  /* ==================================================================
   * UI Rendering
   * ================================================================== */
  function esc(text) {
    return String(text || "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  function renderMiniMarkdown(text) {
    return esc(text)
      .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
      .replace(/`([^`]+)`/g, "<code>$1</code>")
      .replace(/\n/g, "<br>");
  }

  function renderProblem(problem) {
    var display = document.getElementById("problem-display");
    var difficultyClass = "difficulty-" + (problem.difficulty || "medium");
    display.innerHTML =
      '<div class="problem-card">' +
      '<div class="problem-header">' +
      '<h3 class="problem-title">' + esc(problem.title) + "</h3>" +
      '<span class="difficulty-badge ' + difficultyClass + '">' +
      esc(problem.difficulty || "medium") + "</span>" +
      '<span class="language-badge">' +
      esc(problem.language || "python") + "</span>" +
      "</div>" +
      '<div class="problem-description">' +
      renderMiniMarkdown(problem.description) +
      "</div></div>";

    document.getElementById("hint-display").innerHTML = "";
    document.getElementById("hint-display").classList.add("hidden");
    document.getElementById("feedback-display").innerHTML = "";
    document.getElementById("feedback-display").classList.add("hidden");
  }

  function updateActionButtons() {
    var hasProblem = !!ps.currentProblem;
    var busy = ps.generating;
    var genBtn = document.getElementById("generate-problem-btn");
    var hintBtn = document.getElementById("hint-btn");
    var submitBtn = document.getElementById("submit-btn");
    var showSolBtn = document.getElementById("show-solution-btn");
    var runBtn = document.getElementById("run-code-btn");
    if (genBtn) {
      genBtn.disabled = busy;
      genBtn.textContent = busy ? "Generating\u2026" : hasProblem ? "New Problem" : "Generate Problem";
    }
    if (hintBtn) hintBtn.disabled = !hasProblem || busy;
    if (submitBtn) submitBtn.disabled = !hasProblem || busy;
    if (showSolBtn) showSolBtn.disabled = !hasProblem || busy;
    if (runBtn) runBtn.disabled = !hasProblem || busy;
  }

  function updateAiStatus(info) {
    var dot = document.getElementById("ai-dot");
    var label = document.getElementById("ai-label");
    if (!dot || !label) return;
    if (info.connected) {
      var hasModel = info.models.some(function (m) {
        return m.indexOf(config.model.split(":")[0]) !== -1;
      });
      dot.className = "ai-dot " + (hasModel ? "ai-connected" : "ai-warning");
      label.textContent = hasModel
        ? "AI Ready"
        : "Model not found";
      ps.aiConnected = true;
      populateModelSelector(info.models);
    } else {
      dot.className = "ai-dot ai-disconnected";
      label.textContent = "AI Offline";
      ps.aiConnected = false;
    }
  }

  /* ==================================================================
   * Virtual Filesystem
   *
   * In-memory file store per problem. Each problem gets its own set of
   * files. The "active file" is the one displayed in the editor.
   * ================================================================== */
  var vfs = {
    files: {},       /* { filename: { content: string, language: string, readonly: boolean } } */
    activeFile: null,
    mainFile: null,  /* the primary solution file */
  };

  function vfsReset() {
    vfs.files = {};
    vfs.activeFile = null;
    vfs.mainFile = null;
  }

  function vfsCreateFile(name, content, language, readonly) {
    vfs.files[name] = {
      content: content || "",
      language: language || guessLanguageFromFilename(name),
      readonly: !!readonly,
    };
  }

  function vfsDeleteFile(name) {
    if (name === vfs.mainFile) return; /* Never delete the main solution file */
    delete vfs.files[name];
    if (vfs.activeFile === name) {
      vfs.activeFile = vfs.mainFile;
      vfsSwitchTo(vfs.mainFile);
    }
    renderExplorerTree();
    renderEditorTabs();
  }

  function vfsGetContent(name) {
    return vfs.files[name] ? vfs.files[name].content : "";
  }

  function vfsSaveCurrentEditor() {
    if (vfs.activeFile && vfs.files[vfs.activeFile] && ps.editor) {
      vfs.files[vfs.activeFile].content = ps.editor.getValue();
    }
  }

  function vfsSwitchTo(name) {
    if (!vfs.files[name]) return;
    vfsSaveCurrentEditor();
    vfs.activeFile = name;
    if (ps.editor) {
      ps.editor.setValue(vfs.files[name].content);
      setEditorLanguage(vfs.files[name].language);
      ps.editor.setOption("readOnly", vfs.files[name].readonly);
      ps.editor.refresh();
    }
    renderEditorTabs();
    renderExplorerTree();
    updateEditorFilename(name);
  }

  /** Build the initial file set for a problem */
  function vfsInitForProblem(problem) {
    vfsReset();
    var lang = (problem.language || "python").toLowerCase();
    var ext = langToExt(lang);
    var mainName = "solution" + ext;

    vfsCreateFile(mainName, problem.starterCode || "", lang, false);
    vfs.mainFile = mainName;
    vfs.activeFile = mainName;

    /* Add test cases file if test cases exist */
    if (problem.testCases && problem.testCases.length) {
      var testContent = buildTestFile(problem, lang, ext);
      if (testContent) {
        vfsCreateFile("tests" + ext, testContent, lang, true);
      }
    }

    /* Add scaffold files based on language */
    addScaffoldFiles(lang, problem);

    renderExplorerTree();
    renderEditorTabs();
  }

  function langToExt(lang) {
    var map = {
      python: ".py", javascript: ".js", js: ".js",
      typescript: ".ts", ts: ".ts",
      java: ".java", c: ".c", cpp: ".cpp", "c++": ".cpp",
      csharp: ".cs", "c#": ".cs", go: ".go", rust: ".rs",
      sql: ".sql", bash: ".sh", shell: ".sh", sh: ".sh",
    };
    return map[lang] || ".txt";
  }

  function guessLanguageFromFilename(name) {
    var ext = name.slice(name.lastIndexOf(".")).toLowerCase();
    var map = {
      ".py": "python", ".js": "javascript", ".ts": "typescript",
      ".java": "java", ".c": "c", ".cpp": "cpp", ".cs": "csharp",
      ".go": "go", ".rs": "rust", ".sql": "sql", ".sh": "bash",
      ".json": "javascript", ".md": "text", ".txt": "text",
    };
    return map[ext] || "text";
  }

  function buildTestFile(problem, lang, ext) {
    var lines = [];
    if (lang === "python") {
      lines.push("# Auto-generated test cases (read-only)");
      lines.push("# These verify your solution against the expected outputs.\n");
      problem.testCases.forEach(function (tc, i) {
        lines.push("# Test " + (i + 1) + ": " + (tc.explanation || ""));
        lines.push("# Input:    " + tc.input);
        lines.push("# Expected: " + tc.expected);
        lines.push("");
      });
    } else if (lang === "javascript" || lang === "js" || lang === "typescript" || lang === "ts") {
      lines.push("// Auto-generated test cases (read-only)");
      lines.push("// These verify your solution against the expected outputs.\n");
      problem.testCases.forEach(function (tc, i) {
        lines.push("// Test " + (i + 1) + ": " + (tc.explanation || ""));
        lines.push("// Input:    " + tc.input);
        lines.push("// Expected: " + tc.expected);
        lines.push("");
      });
    } else {
      lines.push("Test Cases (read-only)\n");
      problem.testCases.forEach(function (tc, i) {
        lines.push("Test " + (i + 1) + ": " + (tc.explanation || ""));
        lines.push("  Input:    " + tc.input);
        lines.push("  Expected: " + tc.expected);
        lines.push("");
      });
    }
    return lines.join("\n");
  }

  function addScaffoldFiles(lang, problem) {
    if (lang === "python") {
      vfsCreateFile("README.md",
        "# " + (problem.title || "Practice Problem") + "\n\n" +
        (problem.description || "").slice(0, 500) + "\n",
        "text", true);
    } else if (lang === "javascript" || lang === "js" || lang === "typescript" || lang === "ts") {
      vfsCreateFile("package.json",
        JSON.stringify({ name: "practice-problem", version: "1.0.0", main: "solution" + langToExt(lang) }, null, 2),
        "javascript", true);
    } else if (lang === "csharp" || lang === "c#") {
      vfsCreateFile("Program.cs",
        "// Entry point — calls your solution\nusing System;\n\nclass Program {\n    static void Main() {\n        // TODO: test your solution here\n    }\n}\n",
        "csharp", true);
    }
  }

  /* ==================================================================
   * File Explorer Rendering
   * ================================================================== */
  function renderExplorerTree() {
    var tree = document.getElementById("explorer-tree");
    if (!tree) return;
    var names = Object.keys(vfs.files).sort(function (a, b) {
      /* Main file first, then alphabetical */
      if (a === vfs.mainFile) return -1;
      if (b === vfs.mainFile) return 1;
      return a.localeCompare(b);
    });

    tree.innerHTML = names.map(function (name) {
      var f = vfs.files[name];
      var isActive = name === vfs.activeFile;
      var cls = "explorer-item" + (isActive ? " active" : "") + (f.readonly ? " readonly" : "");
      var icon = getFileIcon(name);
      var deleteBtn = (!f.readonly && name !== vfs.mainFile)
        ? '<span class="explorer-item-actions">' +
          '<button class="explorer-item-btn" data-delete="' + esc(name) + '" title="Delete file">&times;</button>' +
          '</span>'
        : '';
      return '<div class="' + cls + '" data-file="' + esc(name) + '">' +
        icon + '<span>' + esc(name) + '</span>' + deleteBtn + '</div>';
    }).join("");
  }

  function getFileIcon(name) {
    var ext = name.slice(name.lastIndexOf(".")).toLowerCase();
    /* Simple colored circle icons by file type */
    var colors = {
      ".py": "#3572A5", ".js": "#f1e05a", ".ts": "#3178c6",
      ".java": "#b07219", ".c": "#555555", ".cpp": "#f34b7d",
      ".cs": "#178600", ".go": "#00ADD8", ".rs": "#dea584",
      ".json": "#292929", ".md": "#083fa1", ".sql": "#e38c00",
      ".sh": "#89e051",
    };
    var color = colors[ext] || "var(--ink-muted)";
    return '<svg viewBox="0 0 16 16" width="12" height="12"><circle cx="8" cy="8" r="4" fill="' + color + '" opacity="0.7"/></svg>';
  }

  function renderEditorTabs() {
    var container = document.getElementById("editor-tabs-container");
    if (!container) return;
    var names = Object.keys(vfs.files);
    /* Show tabs only if more than one file */
    container.innerHTML = names.map(function (name) {
      var isActive = name === vfs.activeFile;
      var cls = "editor-file-tab" + (isActive ? " active" : "");
      return '<span class="' + cls + '" data-file="' + esc(name) + '">' +
        getFileIcon(name) +
        '<span>' + esc(name) + '</span>' +
        '</span>';
    }).join("");
  }

  /* ==================================================================
   * Code Execution Engine
   *
   * Python:     Pyodide (WASM, in-browser)
   * JavaScript: sandboxed eval in a Web Worker
   * Other:      Piston public API (https://emkc.org/api/v2/piston/execute)
   * ================================================================== */
  var execution = {
    pyodide: null,
    pyodideLoading: false,
    worker: null,
  };

  function runCode() {
    vfsSaveCurrentEditor();
    var mainContent = vfsGetContent(vfs.mainFile);
    if (!mainContent.trim()) return;

    var lang = vfs.files[vfs.mainFile] ? vfs.files[vfs.mainFile].language : "python";
    var consoleEl = document.getElementById("console-display");
    switchOutputTab("console");

    consoleEl.innerHTML =
      '<div class="console-running">' +
      '<div class="typing-dots"><span></span><span></span><span></span></div>' +
      'Running\u2026</div>';

    var runBtn = document.getElementById("run-code-btn");
    if (runBtn) runBtn.disabled = true;

    var execPromise;
    if (lang === "python") {
      execPromise = runPython(mainContent);
    } else if (lang === "javascript" || lang === "js") {
      execPromise = runJavaScript(mainContent);
    } else {
      execPromise = runPiston(mainContent, lang);
    }

    execPromise
      .then(function (result) {
        renderConsoleOutput(consoleEl, result);
      })
      .catch(function (err) {
        consoleEl.innerHTML =
          '<pre class="console-output"><span class="console-line stderr">' +
          esc(err.message || String(err)) + '</span></pre>';
      })
      .finally(function () {
        if (runBtn) runBtn.disabled = false;
      });
  }

  function renderConsoleOutput(el, result) {
    var lines = [];
    if (result.stdout) {
      result.stdout.split("\n").forEach(function (line) {
        if (line) lines.push('<span class="console-line stdout">' + esc(line) + '</span>');
      });
    }
    if (result.stderr) {
      result.stderr.split("\n").forEach(function (line) {
        if (line) lines.push('<span class="console-line stderr">' + esc(line) + '</span>');
      });
    }
    if (result.error) {
      lines.push('<span class="console-line stderr">' + esc(result.error) + '</span>');
    }
    if (lines.length === 0) {
      lines.push('<span class="console-line info">(no output)</span>');
    }
    if (result.exitCode !== undefined && result.exitCode !== 0) {
      lines.push('<span class="console-line info">Exit code: ' + result.exitCode + '</span>');
    }
    el.innerHTML = '<pre class="console-output">' + lines.join("\n") + '</pre>';
  }

  /** Run Python via Pyodide (in-browser WASM) */
  function runPython(code) {
    if (execution.pyodide) {
      return executePyodide(code);
    }
    if (execution.pyodideLoading) {
      return Promise.reject(new Error("Python runtime is still loading\u2026 please wait."));
    }
    if (typeof loadPyodide === "undefined") {
      return Promise.reject(new Error("Pyodide not available. Reload the page."));
    }
    execution.pyodideLoading = true;
    var consoleEl = document.getElementById("console-display");
    if (consoleEl) {
      consoleEl.innerHTML =
        '<div class="console-running">' +
        '<div class="typing-dots"><span></span><span></span><span></span></div>' +
        'Loading Python runtime (first run only)\u2026</div>';
    }
    return loadPyodide()
      .then(function (py) {
        execution.pyodide = py;
        execution.pyodideLoading = false;
        return executePyodide(code);
      })
      .catch(function (err) {
        execution.pyodideLoading = false;
        throw err;
      });
  }

  function executePyodide(code) {
    var py = execution.pyodide;
    return new Promise(function (resolve) {
      var stdout = [];
      var stderr = [];
      py.setStdout({ batched: function (line) { stdout.push(line); } });
      py.setStderr({ batched: function (line) { stderr.push(line); } });
      try {
        py.runPython(code);
        resolve({ stdout: stdout.join("\n"), stderr: stderr.join("\n"), exitCode: 0 });
      } catch (err) {
        resolve({ stdout: stdout.join("\n"), stderr: stderr.join("\n"), error: err.message, exitCode: 1 });
      }
    });
  }

  /** Run JavaScript in a sandboxed iframe */
  function runJavaScript(code) {
    return new Promise(function (resolve) {
      var stdout = [];
      var stderr = [];
      var iframe = document.createElement("iframe");
      iframe.style.display = "none";
      iframe.sandbox = "allow-scripts";
      document.body.appendChild(iframe);

      var timeout = setTimeout(function () {
        document.body.removeChild(iframe);
        resolve({ stdout: stdout.join("\n"), stderr: stderr.join("\n"), error: "Execution timed out (5s)", exitCode: 1 });
      }, 5000);

      /* Listen for messages from the sandboxed iframe */
      function onMessage(e) {
        if (e.source !== iframe.contentWindow) return;
        var d = e.data;
        if (d && d.type === "console-log") stdout.push(String(d.value));
        if (d && d.type === "console-error") stderr.push(String(d.value));
        if (d && d.type === "done") {
          clearTimeout(timeout);
          window.removeEventListener("message", onMessage);
          document.body.removeChild(iframe);
          resolve({ stdout: stdout.join("\n"), stderr: stderr.join("\n"), exitCode: d.error ? 1 : 0, error: d.error || "" });
        }
      }
      window.addEventListener("message", onMessage);

      /* Build a self-contained script for the iframe */
      var wrappedCode =
        '<script>' +
        'var _log = [];' +
        'var _err = [];' +
        'console.log = function() { var a = Array.prototype.slice.call(arguments).map(String).join(" "); parent.postMessage({type:"console-log",value:a},"*"); };' +
        'console.error = function() { var a = Array.prototype.slice.call(arguments).map(String).join(" "); parent.postMessage({type:"console-error",value:a},"*"); };' +
        'console.warn = console.error;' +
        'try {' + code.replace(/<\/script>/gi, "<\\/script>") + '} catch(e) { parent.postMessage({type:"done",error:e.message},"*"); }' +
        'parent.postMessage({type:"done"},"*");' +
        '<\/script>';
      iframe.srcdoc = wrappedCode;
    });
  }

  /** Run other languages via Piston API */
  function runPiston(code, lang) {
    var pistonLangMap = {
      python: { language: "python", version: "3.10.0" },
      javascript: { language: "javascript", version: "18.15.0" },
      js: { language: "javascript", version: "18.15.0" },
      typescript: { language: "typescript", version: "5.0.3" },
      ts: { language: "typescript", version: "5.0.3" },
      java: { language: "java", version: "15.0.2" },
      c: { language: "c", version: "10.2.0" },
      cpp: { language: "c++", version: "10.2.0" },
      "c++": { language: "c++", version: "10.2.0" },
      csharp: { language: "csharp", version: "6.12.0" },
      "c#": { language: "csharp", version: "6.12.0" },
      go: { language: "go", version: "1.16.2" },
      rust: { language: "rust", version: "1.68.2" },
      bash: { language: "bash", version: "5.2.0" },
      shell: { language: "bash", version: "5.2.0" },
      sh: { language: "bash", version: "5.2.0" },
      sql: { language: "sqlite3", version: "3.36.0" },
    };
    var mapped = pistonLangMap[lang] || { language: lang, version: "*" };

    return fetch("https://emkc.org/api/v2/piston/execute", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        language: mapped.language,
        version: mapped.version,
        files: [{ name: "solution" + langToExt(lang), content: code }],
      }),
    })
      .then(function (res) {
        if (!res.ok) throw new Error("Piston API error (" + res.status + ")");
        return res.json();
      })
      .then(function (data) {
        var run = data.run || {};
        return {
          stdout: run.stdout || "",
          stderr: run.stderr || "",
          exitCode: run.code || 0,
          error: run.signal ? "Killed by signal: " + run.signal : "",
        };
      });
  }

  /* ==================================================================
   * Output Tab Switching
   * ================================================================== */
  function switchOutputTab(tabName) {
    var tabs = document.querySelectorAll(".output-tab");
    tabs.forEach(function (tab) {
      tab.classList.toggle("active", tab.dataset.output === tabName);
    });
    var feedbackEl = document.getElementById("feedback-display");
    var consoleEl = document.getElementById("console-display");
    if (feedbackEl) feedbackEl.classList.toggle("hidden", tabName !== "feedback");
    if (consoleEl) consoleEl.classList.toggle("hidden", tabName !== "console");
  }

  /* ==================================================================
   * Fullscreen Management
   * ================================================================== */
  var isFullscreen = false;

  function toggleFullscreen() {
    var panel = document.getElementById("practice-panel");
    if (!panel) return;
    isFullscreen = !isFullscreen;
    panel.classList.toggle("ide-fullscreen", isFullscreen);

    /* Refresh editor to fit new size */
    if (ps.editor) {
      setTimeout(function () {
        ps.editor.setSize("100%", null);
        ps.editor.refresh();
      }, 100);
    }
  }

  /* ==================================================================
   * IDE Mode Toggle (Simplified ↔ Full)
   * ================================================================== */
  var ideMode = localStorage.getItem("practice-ide-mode") || "simplified";

  function setIdeMode(mode) {
    ideMode = mode;
    localStorage.setItem("practice-ide-mode", mode);
    var panel = document.getElementById("practice-panel");
    if (!panel) return;
    panel.classList.toggle("ide-mode-full", mode === "full");

    var explorerBtn = document.getElementById("ide-toggle-explorer");
    if (explorerBtn) explorerBtn.classList.toggle("active", mode === "full");

    var modeBtn = document.getElementById("ide-toggle-mode");
    if (modeBtn) modeBtn.classList.toggle("active", mode === "full");

    if (ps.editor) {
      setTimeout(function () { ps.editor.refresh(); }, 50);
    }
  }

  function toggleIdeMode() {
    setIdeMode(ideMode === "full" ? "simplified" : "full");
  }

  /* ==================================================================
   * Status Bar Updates
   * ================================================================== */
  function updateStatusBar(problem) {
    var diffEl = document.getElementById("status-difficulty");
    var langEl = document.getElementById("status-language");
    if (diffEl && problem) {
      diffEl.textContent = (problem.difficulty || "medium").charAt(0).toUpperCase() +
        (problem.difficulty || "medium").slice(1);
    }
    if (langEl && problem) {
      langEl.textContent = (problem.language || "python").charAt(0).toUpperCase() +
        (problem.language || "python").slice(1);
    }
  }

  function updateEditorFilename(nameOrLang) {
    var el = document.getElementById("editor-filename");
    if (!el) return;
    /* If it looks like a filename (has a dot), use it directly */
    if (nameOrLang && nameOrLang.indexOf(".") !== -1) {
      el.textContent = nameOrLang;
    } else {
      el.textContent = "solution" + langToExt(nameOrLang || "python");
    }
  }

  function updateCursorPosition() {
    if (!ps.editor) return;
    var cursor = ps.editor.getCursor();
    var el = document.getElementById("status-line-info");
    if (el) el.textContent = "Ln " + (cursor.line + 1) + ", Col " + (cursor.ch + 1);
  }

  /* -- Model selector population ------------------------------------- */
  function populateModelSelector(models) {
    var sel = document.getElementById("model-selector");
    if (!sel) return;
    var prev = sel.value;
    sel.innerHTML = "";
    models.forEach(function (m) {
      var opt = document.createElement("option");
      opt.value = m;
      opt.textContent = m;
      sel.appendChild(opt);
    });
    if (models.indexOf(config.model) !== -1) {
      sel.value = config.model;
    } else if (models.length > 0) {
      sel.value = models[0];
      config.model = models[0];
      localStorage.setItem("practice-model", models[0]);
    }
  }

  /* -- Provider tab switching (local / hosted) ----------------------- */
  var activeProvider = localStorage.getItem("practice-provider") || "local";

  function switchProvider(provider) {
    activeProvider = provider;
    localStorage.setItem("practice-provider", provider);
    var tabs = document.querySelectorAll(".provider-tab");
    tabs.forEach(function (tab) {
      tab.classList.toggle("active", tab.dataset.provider === provider);
    });
    var setupEl = document.getElementById("practice-setup");
    var hostedGate = document.getElementById("hosted-login-gate");
    var problemView = document.getElementById("practice-problem-view");
    var practiceActions = document.querySelector(".practice-actions");
    var editorContainer = document.querySelector(".editor-container");
    var modelSel = document.getElementById("model-selector");
    var hostedModelGroup = document.getElementById("hosted-model-group");
    var statusProviderLabel = document.getElementById("status-provider-label");

    if (provider === "hosted") {
      if (setupEl) setupEl.classList.add("hidden");
      if (modelSel) modelSel.disabled = true;
      if (hostedModelGroup) hostedModelGroup.style.display = "";
      if (statusProviderLabel) statusProviderLabel.textContent = "Hosted";
      stopInstallerPolling();

      if (!config.apiBase) {
        /* API not configured — show message */
        hidePracticeContent();
        if (hostedGate) {
          hostedGate.classList.remove("hidden");
          hostedGate.innerHTML =
            '<div class="setup-card hosted-gate">' +
            '<div class="hosted-gate-icon">' +
            '<svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>' +
            '</div>' +
            '<h3>Hosted Mode Not Configured</h3>' +
            '<p class="hosted-gate-desc">The hosted API endpoint has not been set up yet. Use the <strong>Local</strong> tab with Ollama for free practice.</p>' +
            '<button class="btn btn-primary btn-lg" id="switch-to-local-fallback">Switch to Local</button>' +
            '</div>';
          var fb = document.getElementById("switch-to-local-fallback");
          if (fb) fb.addEventListener("click", function () { switchProvider("local"); });
        }
      } else if (isHostedAuthenticated()) {
        /* Already logged in — show practice content + status bar */
        if (hostedGate) hostedGate.classList.add("hidden");
        showPracticeContent();
        showHostedStatusBar();
        fetchHostedQuota();
      } else {
        /* Need login */
        hidePracticeContent();
        renderHostedLoginGate();
      }
    } else {
      if (hostedGate) hostedGate.classList.add("hidden");
      hideHostedStatusBar();
      if (modelSel) modelSel.disabled = false;
      if (hostedModelGroup) hostedModelGroup.style.display = "none";
      if (statusProviderLabel) statusProviderLabel.textContent = "Local";
      refreshAiStatus();
    }
  }

  /* -- Hosted status bar -------------------------------------------- */
  function showHostedStatusBar() {
    var bar = document.getElementById("hosted-status-bar");
    if (!bar) return;
    bar.classList.remove("hidden");
    var userEl = bar.querySelector(".hosted-user");
    if (userEl) userEl.textContent = hosted.email;
  }

  function hideHostedStatusBar() {
    var bar = document.getElementById("hosted-status-bar");
    if (bar) bar.classList.add("hidden");
  }

  function fetchHostedQuota() {
    if (!isHostedAuthenticated()) return;
    fetch(config.apiBase + "/.netlify/functions/check-quota", {
      headers: { "Authorization": "Bearer " + hosted.token },
    })
      .then(function (res) { return res.json(); })
      .then(function (data) {
        if (data.remaining !== undefined) updateQuotaDisplay(data);
      })
      .catch(function () { /* silent — quota display just stays empty */ });
  }

  function renderHostedLoginGate() {
    var gate = document.getElementById("hosted-login-gate");
    if (!gate) return;
    gate.classList.remove("hidden");
    gate.innerHTML =
      '<div class="setup-card hosted-gate">' +
      '<div class="hosted-gate-icon">' +
      '<svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>' +
      '</div>' +
      '<h3 id="hosted-gate-heading">Sign in to Practice</h3>' +
      '<p class="hosted-gate-desc">Hosted mode lets you practice without installing anything locally. Usage is quota-based (500 / month).</p>' +
      '<div id="hosted-auth-error" class="hosted-auth-error hidden"></div>' +
      '<form class="hosted-login-form" id="hosted-login-form" autocomplete="off">' +
      '<label class="hosted-field"><span>Email</span>' +
      '<input type="email" id="hosted-email" placeholder="you@example.com" required /></label>' +
      '<label class="hosted-field"><span>Password</span>' +
      '<input type="password" id="hosted-password" placeholder="&#8226;&#8226;&#8226;&#8226;&#8226;&#8226;&#8226;&#8226;" required minlength="8" /></label>' +
      '<button class="btn btn-primary btn-lg hosted-login-btn" type="submit" id="hosted-auth-btn">Sign In</button>' +
      '</form>' +
      '<p class="hosted-gate-footer" id="hosted-gate-footer">' +
      'Don\u2019t have an account? <a href="#" id="hosted-signup-link">Create one</a></p>' +
      '</div>';

    /* Wire the dynamic form */
    var hostedAuthMode = "login";
    var form = document.getElementById("hosted-login-form");
    var heading = document.getElementById("hosted-gate-heading");
    var authBtn = document.getElementById("hosted-auth-btn");
    var footer = document.getElementById("hosted-gate-footer");
    var errEl = document.getElementById("hosted-auth-error");

    if (form) {
      form.addEventListener("submit", function (e) {
        e.preventDefault();
        var email = document.getElementById("hosted-email").value.trim();
        var password = document.getElementById("hosted-password").value;
        if (!email || !password) return;
        if (errEl) { errEl.classList.add("hidden"); errEl.textContent = ""; }
        authBtn.disabled = true;
        authBtn.textContent = hostedAuthMode === "login" ? "Signing in\u2026" : "Creating account\u2026";

        var endpoint = hostedAuthMode === "login" ? "auth-login" : "auth-signup";
        hostedAuth(endpoint, email, password)
          .then(function () {
            var hostedGate = document.getElementById("hosted-login-gate");
            if (hostedGate) hostedGate.classList.add("hidden");
            showPracticeContent();
            showHostedStatusBar();
            fetchHostedQuota();
          })
          .catch(function (err) {
            if (errEl) {
              errEl.textContent = err.message;
              errEl.classList.remove("hidden");
            }
          })
          .finally(function () {
            authBtn.disabled = false;
            authBtn.textContent = hostedAuthMode === "login" ? "Sign In" : "Create Account";
          });
      });
    }

    /* Toggle signup / login */
    gate.addEventListener("click", function (e) {
      var link = e.target.closest("#hosted-signup-link");
      if (!link) return;
      e.preventDefault();
      if (hostedAuthMode === "login") {
        hostedAuthMode = "signup";
        if (heading) heading.textContent = "Create an Account";
        if (authBtn) authBtn.textContent = "Create Account";
        if (footer) footer.innerHTML = 'Already have an account? <a href="#" id="hosted-signup-link">Sign in</a>';
      } else {
        hostedAuthMode = "login";
        if (heading) heading.textContent = "Sign in to Practice";
        if (authBtn) authBtn.textContent = "Sign In";
        if (footer) footer.innerHTML = 'Don\u2019t have an account? <a href="#" id="hosted-signup-link">Create one</a>';
      }
      if (errEl) { errEl.classList.add("hidden"); errEl.textContent = ""; }
    });
  }

  /* ==================================================================
   * Ollama Setup Gate & In-Browser Model Pull
   * ================================================================== */
  var installer = {
    pollTimer: null,
    pulling: false,
  };

  function detectOs() {
    var ua = navigator.userAgent || "";
    var platform = navigator.platform || "";
    if (/Win/i.test(platform)) return "windows";
    if (/Mac/i.test(platform) || /Mac/i.test(ua)) return "mac";
    if (/Linux/i.test(platform) || /Linux/i.test(ua)) return "linux";
    return "unknown";
  }

  function getDownloadUrl() {
    var os = detectOs();
    if (os === "mac") return "https://ollama.com/download/mac";
    if (os === "windows") return "https://ollama.com/download/windows";
    if (os === "linux") return "https://ollama.com/download/linux";
    return "https://ollama.com/download";
  }

  function getLaunchHint() {
    var os = detectOs();
    if (os === "mac") {
      return "After installing, open Ollama from Applications and leave it running in your menu bar.";
    }
    if (os === "windows") {
      return "After installing, launch Ollama from the Start menu and keep the local server running.";
    }
    if (os === "linux") {
      return "After installing, start the Ollama server and keep it running so Atlas can connect.";
    }
    return "After installing, launch Ollama so the local server is running before you return here.";
  }

  function getOriginSetupHelp() {
    var origin = window.location.origin;
    var os = detectOs();
    if (os === "mac") {
      return {
        command: 'launchctl setenv OLLAMA_ORIGINS "' + origin + '"',
        restart: "Then fully quit Ollama and reopen it from Applications.",
      };
    }
    if (os === "windows") {
      return {
        command:
          '[Environment]::SetEnvironmentVariable("OLLAMA_ORIGINS", "' +
          origin +
          '", "User")',
        restart: "Then fully quit Ollama from the taskbar and start it again from the Start menu.",
      };
    }
    if (os === "linux") {
      return {
        command: 'OLLAMA_ORIGINS="' + origin + '" ollama serve',
        restart: "If you run Ollama as a service, add OLLAMA_ORIGINS to the service environment and restart it.",
      };
    }
    return {
      command: 'OLLAMA_ORIGINS="' + origin + '" ollama serve',
      restart: "Restart Ollama after allowing this site origin.",
    };
  }

  function runInstallerCheck() {
    return checkConnection().then(function (info) {
      if (info.connected) {
        stopInstallerPolling();
        onOllamaDetected(info);
        return true;
      }
      return false;
    });
  }

  function startInstallerPolling() {
    stopInstallerPolling();
    runInstallerCheck();
    installer.pollTimer = setInterval(runInstallerCheck, 2000);
  }

  function stopInstallerPolling() {
    if (installer.pollTimer) {
      clearInterval(installer.pollTimer);
      installer.pollTimer = null;
    }
  }

  /* -- Pull model via Ollama HTTP API with streaming progress -------- */
  function pullModel() {
    if (installer.pulling) return;
    installer.pulling = true;
    var el = document.getElementById("practice-setup");
    if (!el) return;

    el.innerHTML =
      '<div class="setup-card setup-pulling">' +
      '<h3>Downloading ' + esc(config.model) + '</h3>' +
      '<p class="setup-pull-sub">This happens once. The model stays on your machine.</p>' +
      '<div class="pull-progress-wrap">' +
      '<div class="pull-progress-bar"><div class="pull-progress-fill" id="pull-fill"></div></div>' +
      '<p class="pull-status" id="pull-status">Starting download\u2026</p>' +
      '</div></div>';

    var fillEl = document.getElementById("pull-fill");
    var statusEl = document.getElementById("pull-status");

    fetch(config.ollamaBase + "/api/pull", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: config.model, stream: true }),
    })
      .then(function (res) {
        if (!res.ok) throw new Error("Pull failed (" + res.status + ")");
        var reader = res.body.getReader();
        var decoder = new TextDecoder();

        function pump() {
          return reader.read().then(function (result) {
            if (result.done) return;
            var text = decoder.decode(result.value, { stream: true });
            text.split("\n").filter(Boolean).forEach(function (line) {
              try {
                var data = JSON.parse(line);
                if (data.total && data.completed) {
                  var pct = Math.round((data.completed / data.total) * 100);
                  if (fillEl) fillEl.style.width = pct + "%";
                  var mbDone = (data.completed / 1048576).toFixed(0);
                  var mbTotal = (data.total / 1048576).toFixed(0);
                  if (statusEl) statusEl.textContent = data.status + " \u2014 " + mbDone + " / " + mbTotal + " MB (" + pct + "%)";
                } else if (data.status) {
                  if (statusEl) statusEl.textContent = data.status;
                }
              } catch (_e) { /* skip */ }
            });
            return pump();
          });
        }
        return pump();
      })
      .then(function () {
        installer.pulling = false;
        localStorage.setItem("practice-setup-done", "true");
        checkConnection().then(function (info) {
          updateAiStatus(info);
          showPracticeContent();
        });
      })
      .catch(function (err) {
        installer.pulling = false;
        if (statusEl) {
          statusEl.innerHTML =
            '<span class="pull-error">Download failed: ' + esc(err.message) + "</span>" +
            '<br><button class="btn btn-primary btn-sm" id="retry-pull-btn">Retry</button>';
          var retryBtn = document.getElementById("retry-pull-btn");
          if (retryBtn) retryBtn.addEventListener("click", pullModel);
        }
      });
  }

  /* -- Called when Ollama is first detected (may or may not have model) -- */
  function onOllamaDetected(info) {
    var hasModel = info.models.some(function (m) {
      return m.indexOf(config.model.split(":")[0]) !== -1;
    });
    updateAiStatus(info);
    if (hasModel) {
      localStorage.setItem("practice-setup-done", "true");
      showPracticeContent();
    } else {
      renderModelPullPrompt();
    }
  }

  /* -- Show the main practice UI, hide the gate ---------------------- */
  function showPracticeContent() {
    var setupEl = document.getElementById("practice-setup");
    var problemView = document.getElementById("practice-problem-view");
    var practiceActions = document.querySelector(".practice-actions");
    var editorContainer = document.querySelector(".editor-container");
    if (setupEl) setupEl.classList.add("hidden");
    if (problemView) problemView.classList.remove("hidden");
    if (practiceActions) practiceActions.classList.remove("hidden");
    if (editorContainer) editorContainer.classList.remove("hidden");
    stopInstallerPolling();

    /* auto-generate if autoSetup is on and no problem loaded */
    if (config.autoSetup && !ps.currentProblem && ps.articleId && !ps.generating) {
      generateProblem();
    }
  }

  /* -- Hide practice content behind the gate ------------------------- */
  function hidePracticeContent() {
    var problemView = document.getElementById("practice-problem-view");
    var practiceActions = document.querySelector(".practice-actions");
    var editorContainer = document.querySelector(".editor-container");
    if (problemView) problemView.classList.add("hidden");
    if (practiceActions) practiceActions.classList.add("hidden");
    if (editorContainer) editorContainer.classList.add("hidden");
  }

  /* -- Big install gate ---------------------------------------------- */
  function renderInstallGate(info) {
    var el = document.getElementById("practice-setup");
    if (!el) return;
    el.classList.remove("hidden");
    hidePracticeContent();

    var os = detectOs();
    var osLabels = { mac: "macOS", linux: "Linux", windows: "Windows", unknown: "your platform" };
    var osLabel = osLabels[os] || "your platform";
    var downloadUrl = getDownloadUrl();
    var launchHint = getLaunchHint();
    var originHelp = getOriginSetupHelp();
    var diagnosticHtml = "";

    if (info && info.errorType === "origin") {
      diagnosticHtml =
        '<div class="setup-alert">' +
        '<strong>Ollama may already be running.</strong> Atlas is loaded from <code>' +
        esc(window.location.origin) +
        '</code>, and Ollama blocks that browser origin until <code>OLLAMA_ORIGINS</code> includes it.</div>' +
        '<div class="setup-troubleshoot">' +
        '<p class="setup-troubleshoot-title">Allow this site origin in Ollama:</p>' +
        '<pre class="setup-command">' +
        esc(originHelp.command) +
        '</pre>' +
        '<p class="setup-troubleshoot-note">' +
        esc(originHelp.restart) +
        '</p></div>';
    } else if (info && info.errorType === "timeout") {
      diagnosticHtml =
        '<div class="setup-alert">Atlas reached <code>' +
        esc(config.ollamaBase) +
        '</code> too slowly. If Ollama is starting up, wait a moment and click <strong>Check Again</strong>.</div>';
    }

    el.innerHTML =
      '<div class="setup-card setup-gate">' +
      '<div class="setup-gate-icon">' +
      '<svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">' +
      '<path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>' +
      '<polyline points="3.27 6.96 12 12.01 20.73 6.96"/>' +
      '<line x1="12" y1="22.08" x2="12" y2="12"/></svg></div>' +
      '<h3>Install and Launch Ollama</h3>' +
      '<p class="setup-gate-desc">Practice mode uses a local AI model that runs entirely on your machine. ' +
      'No data is sent anywhere \u2014 everything stays private.</p>' +
      '<p class="setup-gate-desc">Download alone is not enough. <strong>Ollama must be actively running</strong> so Atlas can reach the local server at ' + esc(config.ollamaBase) + '.</p>' +
      diagnosticHtml +
      '<div class="setup-steps">' +
      '<div class="setup-step"><span class="setup-step-num">1</span><span>Install <strong>Ollama</strong> for ' + esc(osLabel) + '.</span></div>' +
      '<div class="setup-step"><span class="setup-step-num">2</span><span>' + esc(launchHint) + '</span></div>' +
      '<div class="setup-step"><span class="setup-step-num">3</span><span>Come back here and click <strong>Check Again</strong>. Atlas will also keep polling automatically.</span></div>' +
      '</div>' +
      '<div class="setup-gate-actions">' +
      '<a class="btn btn-primary btn-lg setup-download-btn" href="' + esc(downloadUrl) + '" target="_blank" rel="noreferrer">' +
      'Download Ollama for ' + esc(osLabel) + '</a>' +
      '<button class="btn btn-secondary btn-lg setup-check-btn" id="check-ollama-btn">Check Again</button>' +
      '</div>' +
      '<div class="setup-gate-polling">' +
      '<div class="typing-dots"><span></span><span></span><span></span></div>' +
      '<span>Waiting for a running Ollama server\u2026</span></div>' +
      '<details class="setup-advanced"><summary>Advanced settings</summary>' +
      '<div class="setup-config">' +
      '<label>Model: <input type="text" id="model-input" value="' + esc(config.model) + '" /></label>' +
      '<label>Ollama URL: <input type="text" id="ollama-url-input" value="' + esc(config.ollamaBase) + '" /></label>' +
      '<label class="setup-checkbox"><input type="checkbox" id="auto-setup-check"' +
      (config.autoSetup ? " checked" : "") + ' /> Auto-generate problem when opening Practice</label>' +
      '<button class="btn btn-sm btn-primary" id="save-config-btn">Save</button>' +
      '</div></details>' +
      '</div>';

    wireSetupEvents(el);
    startInstallerPolling();
  }

  /* -- Model pull prompt (Ollama running, model missing) ------------- */
  function renderModelPullPrompt() {
    var el = document.getElementById("practice-setup");
    if (!el) return;
    el.classList.remove("hidden");
    hidePracticeContent();

    el.innerHTML =
      '<div class="setup-card setup-gate">' +
      '<div class="setup-gate-icon">' +
      '<svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">' +
      '<polyline points="8 17 12 21 16 17"/>' +
      '<line x1="12" y1="12" x2="12" y2="21"/>' +
      '<path d="M20.88 18.09A5 5 0 0 0 18 9h-1.26A8 8 0 1 0 3 16.29"/></svg></div>' +
      '<h3>Download AI Model</h3>' +
      '<p class="setup-gate-desc">Ollama is running! Now we need to download the AI model ' +
      '<strong>' + esc(config.model) + '</strong>. This is a one-time download.</p>' +
      '<p class="setup-gate-desc setup-gate-size">The model is large (~18 GB). Make sure you have enough disk space and a stable connection.</p>' +
      '<button class="btn btn-primary btn-lg" id="start-pull-btn">Download ' + esc(config.model) + '</button>' +
      '<details class="setup-advanced"><summary>Advanced settings</summary>' +
      '<div class="setup-config">' +
      '<label>Model: <input type="text" id="model-input" value="' + esc(config.model) + '" /></label>' +
      '<label>Ollama URL: <input type="text" id="ollama-url-input" value="' + esc(config.ollamaBase) + '" /></label>' +
      '<label class="setup-checkbox"><input type="checkbox" id="auto-setup-check"' +
      (config.autoSetup ? " checked" : "") + ' /> Auto-generate problem when opening Practice</label>' +
      '<button class="btn btn-sm btn-primary" id="save-config-btn">Save</button>' +
      '</div></details>' +
      '</div>';

    wireSetupEvents(el);
    var pullBtn = document.getElementById("start-pull-btn");
    if (pullBtn) pullBtn.addEventListener("click", pullModel);
  }

  /* -- Shared event wiring for setup panels -------------------------- */
  function wireSetupEvents(el) {
    var checkBtn = el.querySelector("#check-ollama-btn");
    if (checkBtn) {
      checkBtn.addEventListener("click", function () {
        refreshAiStatus();
      });
    }

    var saveBtn = el.querySelector("#save-config-btn");
    if (saveBtn) {
      saveBtn.addEventListener("click", function () {
        var modelVal = document.getElementById("model-input").value.trim();
        var urlVal = document.getElementById("ollama-url-input").value.trim();
        var autoCheck = document.getElementById("auto-setup-check");
        if (modelVal) {
          config.model = modelVal;
          localStorage.setItem("practice-model", modelVal);
        }
        if (urlVal) {
          config.ollamaBase = urlVal;
          localStorage.setItem("practice-ollama-url", urlVal);
        }
        if (autoCheck) {
          config.autoSetup = autoCheck.checked;
          localStorage.setItem("practice-auto-setup", autoCheck.checked ? "true" : "false");
        }
        refreshAiStatus();
      });
    }
  }

  function renderSetupInstructions() {
    checkConnection().then(function (info) {
      if (info.connected) {
        onOllamaDetected(info);
      } else {
        renderInstallGate(info);
      }
    });
  }

  function refreshAiStatus() {
    checkConnection().then(function (info) {
      updateAiStatus(info);
      if (info.connected) {
        var hasModel = info.models.some(function (m) {
          return m.indexOf(config.model.split(":")[0]) !== -1;
        });
        if (hasModel) {
          showPracticeContent();
          return;
        }
      }
      renderSetupInstructions();
    });
  }

  /* ==================================================================
   * Examples Tab
   * ================================================================== */
  function renderExamples() {
    if (!ps.articleId) return Promise.resolve();
    return dbGetByIndex("examples", "articleId", ps.articleId).then(function (examples) {
      var container = document.getElementById("examples-list");
      if (!container) return;
      if (!examples.length) {
        container.innerHTML =
          '<p class="examples-empty">No examples yet. Solve practice problems to build your example library.</p>';
        return;
      }

      var sorted = examples.sort(function (a, b) {
        return (b.createdAt || 0) - (a.createdAt || 0);
      });
      container.innerHTML = sorted
        .map(function (ex) {
          var typeClass = "example-type-" + (ex.type || "general");
          var sourceLabel = ex.source === "user-solved" ? "Your Solution" :
            ex.source === "ai-generated" ? "AI Generated" : "Example";
          return (
            '<div class="example-card ' + typeClass + '">' +
            '<div class="example-header">' +
            '<span class="example-source-badge">' + esc(sourceLabel) + "</span>" +
            (ex.tags
              ? ex.tags.map(function (t) {
                  return '<span class="example-tag">' + esc(t) + "</span>";
                }).join("")
              : "") +
            "</div>" +
            "<h4>" + esc(ex.title || "Untitled") + "</h4>" +
            (ex.description
              ? "<p>" + esc(ex.description).slice(0, 300) + "</p>"
              : "") +
            (ex.code
              ? '<pre><code class="language-' +
                esc(ex.language || "python") + '">' +
                esc(ex.code) + "</code></pre>"
              : "") +
            (ex.conceptInsight
              ? '<p class="example-insight"><strong>Insight:</strong> ' +
                esc(ex.conceptInsight) + "</p>"
              : "") +
            "</div>"
          );
        })
        .join("");

      if (typeof hljs !== "undefined") {
        container.querySelectorAll("pre code").forEach(function (block) {
          hljs.highlightElement(block);
        });
      }
    });
  }

  /* ==================================================================
   * Practice Panel Lifecycle
   * ================================================================== */
  function close() {
    var panel = document.getElementById("practice-panel");
    ps.active = false;
    if (panel) panel.classList.add("hidden");
    document.getElementById("reader-column").classList.remove("practice-open");
  }

  function toggle(articleId, articleTitle, articleContent) {
    var panel = document.getElementById("practice-panel");
    if (!panel) return;

    if (ps.active && ps.articleId === articleId) {
      close();
      return;
    }

    ps.active = true;
    ps.articleId = articleId;
    ps.articleTitle = articleTitle || "";
    ps.articleContent = articleContent || "";
    ps.currentProblem = null;
    ps.hints = [];
    ps.hintIndex = 0;

    panel.classList.remove("hidden");
    document.getElementById("reader-column").classList.add("practice-open");

    document.getElementById("problem-display").innerHTML =
      '<p class="practice-intro">Generate a practice problem based on this article.</p>';
    document.getElementById("hint-display").innerHTML = "";
    document.getElementById("hint-display").classList.add("hidden");
    document.getElementById("feedback-display").innerHTML = "";
    document.getElementById("feedback-display").classList.add("hidden");
    updateActionButtons();

    initEditor();
    if (ps.editor) {
      ps.editor.setValue("");
      ps.editor.refresh();
    }

    /* Gate: hide practice content until Ollama is confirmed */
    hidePracticeContent();
    switchTab("problem");
    switchProvider(activeProvider);

    dbGetByIndex("problems", "articleId", articleId).then(function (problems) {
      if (problems.length > 0) {
        var latest = problems[problems.length - 1];
        ps.currentProblem = latest;
        ps.hints = latest.hints || [];
        renderProblem(latest);
        updateActionButtons();
        setEditorLanguage(latest.language || "python");
        vfsInitForProblem(latest);
        if (latest.starterCode && ps.editor && !ps.editor.getValue().trim()) {
          ps.editor.setValue(latest.starterCode);
        }
      }
    });
  }

  function switchTab(tabName) {
    ps.activeTab = tabName;
    var tabs = document.querySelectorAll(".practice-tab");
    tabs.forEach(function (tab) {
      tab.classList.toggle("active", tab.dataset.tab === tabName);
    });
    var problemView = document.getElementById("practice-problem-view");
    var examplesView = document.getElementById("practice-examples-view");
    if (problemView)
      problemView.classList.toggle("hidden", tabName !== "problem");
    if (examplesView)
      examplesView.classList.toggle("hidden", tabName !== "examples");
    if (tabName === "examples") renderExamples();
  }

  /* ==================================================================
   * Event Wiring
   * ================================================================== */
  function wireEvents() {
    document.addEventListener("click", function (event) {
      var genBtn = event.target.closest("#generate-problem-btn");
      if (genBtn) {
        generateProblem();
        return;
      }
      var genEmptyBtn = event.target.closest("#generate-problem-btn-empty");
      if (genEmptyBtn) {
        generateProblem();
        return;
      }
      var hintBtn = event.target.closest("#hint-btn");
      if (hintBtn && !hintBtn.disabled) {
        getHint();
        return;
      }
      var submitBtn = event.target.closest("#submit-btn");
      if (submitBtn && !submitBtn.disabled) {
        submitSolution();
        return;
      }
      var solBtn = event.target.closest("#show-solution-btn");
      if (solBtn && !solBtn.disabled) {
        showSolution();
        return;
      }
      var runBtn = event.target.closest("#run-code-btn");
      if (runBtn && !runBtn.disabled) {
        runCode();
        return;
      }
      var tab = event.target.closest(".practice-tab");
      if (tab && tab.dataset.tab) {
        switchTab(tab.dataset.tab);
        return;
      }
      var provTab = event.target.closest(".provider-tab");
      if (provTab && provTab.dataset.provider) {
        switchProvider(provTab.dataset.provider);
        return;
      }
      /* Output tab switching */
      var outTab = event.target.closest(".output-tab");
      if (outTab && outTab.dataset.output) {
        switchOutputTab(outTab.dataset.output);
        return;
      }
      /* File explorer item click */
      var explorerItem = event.target.closest(".explorer-item");
      if (explorerItem && explorerItem.dataset.file) {
        vfsSwitchTo(explorerItem.dataset.file);
        return;
      }
      /* File explorer delete button */
      var deleteBtn = event.target.closest("[data-delete]");
      if (deleteBtn) {
        vfsDeleteFile(deleteBtn.dataset.delete);
        return;
      }
      /* Editor file tab click */
      var fileTab = event.target.closest(".editor-file-tab");
      if (fileTab && fileTab.dataset.file) {
        vfsSwitchTo(fileTab.dataset.file);
        return;
      }
      /* Fullscreen button */
      var fsBtn = event.target.closest("#ide-fullscreen-btn");
      if (fsBtn) {
        toggleFullscreen();
        return;
      }
      /* Mode toggle button */
      var modeBtn = event.target.closest("#ide-toggle-mode");
      if (modeBtn) {
        toggleIdeMode();
        return;
      }
      /* Explorer toggle button */
      var explorerToggle = event.target.closest("#ide-toggle-explorer");
      if (explorerToggle) {
        toggleIdeMode();
        return;
      }
      /* Show full mode link in explorer notice */
      var fullModeLink = event.target.closest("#show-full-mode-btn");
      if (fullModeLink) {
        setIdeMode("full");
        return;
      }
      /* Reset code button */
      var resetBtn = event.target.closest("#reset-code-btn");
      if (resetBtn && ps.currentProblem && ps.editor) {
        ps.editor.setValue(ps.currentProblem.starterCode || "");
        if (vfs.mainFile && vfs.files[vfs.mainFile]) {
          vfs.files[vfs.mainFile].content = ps.currentProblem.starterCode || "";
        }
        return;
      }
    });

    /* Difficulty selector */
    document.addEventListener("click", function (e) {
      var diffBtn = e.target.closest(".diff-btn");
      if (!diffBtn) return;
      var diff = diffBtn.dataset.difficulty;
      if (!diff) return;
      config.difficulty = diff;
      localStorage.setItem("practice-difficulty", diff);
      document.querySelectorAll(".diff-btn").forEach(function (b) {
        b.classList.toggle("active", b.dataset.difficulty === diff);
      });
      var statusDiff = document.getElementById("status-difficulty");
      if (statusDiff) statusDiff.textContent = diff.charAt(0).toUpperCase() + diff.slice(1);
    });

    /* Hosted model selector */
    var hostedModelSel = document.getElementById("hosted-model-selector");
    if (hostedModelSel) {
      hostedModelSel.addEventListener("change", function () {
        config.hostedModel = hostedModelSel.value;
        localStorage.setItem("practice-hosted-model", config.hostedModel);
      });
    }

    /* Model selector */
    var modelSel = document.getElementById("model-selector");
    if (modelSel) {
      modelSel.addEventListener("change", function () {
        config.model = modelSel.value;
        localStorage.setItem("practice-model", config.model);
        var dot = document.getElementById("ai-dot");
        var label = document.getElementById("ai-label");
        if (dot) dot.className = "ai-dot ai-connected";
        if (label) label.textContent = "AI Ready";
      });
    }

    /* Hosted logout button (in status bar) */
    document.addEventListener("click", function (e) {
      if (e.target.closest(".hosted-logout-btn")) {
        hostedLogout();
        hideHostedStatusBar();
        switchProvider("hosted");
      }
    });

    /* New file button in explorer */
    var newFileBtn = document.getElementById("new-file-btn");
    if (newFileBtn) {
      newFileBtn.addEventListener("click", function () {
        var lang = ps.currentProblem ? (ps.currentProblem.language || "python") : "python";
        var ext = langToExt(lang);
        var name = prompt("File name:", "helper" + ext);
        if (!name || !name.trim()) return;
        name = name.trim();
        if (vfs.files[name]) { alert("File already exists."); return; }
        vfsCreateFile(name, "", guessLanguageFromFilename(name), false);
        renderExplorerTree();
        renderEditorTabs();
        vfsSwitchTo(name);
      });
    }

    /* Keyboard shortcuts */
    document.addEventListener("keydown", function (e) {
      if (!ps.active) return;
      /* Ctrl/Cmd + Enter: Run code */
      if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
        e.preventDefault();
        var runBtn = document.getElementById("run-code-btn");
        if (runBtn && !runBtn.disabled) runCode();
        return;
      }
      /* Ctrl/Cmd + B: Toggle explorer */
      if ((e.ctrlKey || e.metaKey) && e.key === "b") {
        e.preventDefault();
        toggleIdeMode();
        return;
      }
      /* F11: Fullscreen */
      if (e.key === "F11") {
        e.preventDefault();
        toggleFullscreen();
        return;
      }
      /* Escape: Exit fullscreen */
      if (e.key === "Escape" && isFullscreen) {
        toggleFullscreen();
        return;
      }
    });

    /* Theme sync */
    var observer = new MutationObserver(function (mutations) {
      mutations.forEach(function (m) {
        if (m.attributeName === "data-theme") syncEditorTheme();
      });
    });
    observer.observe(document.documentElement, { attributes: true });
  }

  /* ==================================================================
   * Public API
   * ================================================================== */
  function init() {
    wireEvents();

    /* Apply saved IDE mode */
    setIdeMode(ideMode);

    /* Difficulty initial highlight */
    var activeDiffBtn = document.querySelector('.diff-btn[data-difficulty="' + config.difficulty + '"]');
    if (activeDiffBtn) {
      document.querySelectorAll(".diff-btn").forEach(function (b) { b.classList.remove("active"); });
      activeDiffBtn.classList.add("active");
    }

    /* Hosted model initial value */
    var hostedModelSel = document.getElementById("hosted-model-selector");
    if (hostedModelSel) hostedModelSel.value = config.hostedModel;

    /* Cursor position tracking */
    var cursorInterval = setInterval(function () {
      if (ps.editor) {
        clearInterval(cursorInterval);
        ps.editor.on("cursorActivity", updateCursorPosition);
      }
    }, 500);

    /* Enable Run button when a problem exists */
    var runBtn = document.getElementById("run-code-btn");
    if (runBtn) runBtn.disabled = !ps.currentProblem;

    openDb().catch(function (err) {
      console.warn("Practice DB init failed:", err);
    });
  }

  window.AtlasPractice = {
    close: close,
    init: init,
    toggle: toggle,
    isActive: function () {
      return ps.active;
    },
    refreshTheme: syncEditorTheme,
  };
})();
