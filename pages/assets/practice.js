/* ====================================================================
 * Atlas Practice Module
 * Interactive code editor, local AI (Ollama) integration, problem
 * generation, hint system, solution evaluation, and examples storage.
 * ==================================================================== */
(function () {
  "use strict";

  /* -- Configuration --------------------------------------------------- */
  var config = {
    ollamaBase: localStorage.getItem("practice-ollama-url") || "http://localhost:11434",
    model: localStorage.getItem("practice-model") || "qwen3.5:30b",
    autoSetup: localStorage.getItem("practice-auto-setup") === "true",
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
   * AI Client — Ollama
   * ================================================================== */
  function checkConnection() {
    return fetch(config.ollamaBase + "/api/tags", {
      signal: AbortSignal.timeout(3000),
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
      .catch(function () {
        return { connected: false, models: [] };
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
      "<p>Generating a practice problem\u2026</p></div>";

    return dbGetByIndex("examples", "articleId", ps.articleId)
      .then(function (examples) {
        var exCtx = "";
        if (examples.length > 0) {
          var recent = examples.slice(-3);
          exCtx =
            "\n\nPrevious practice activity for this article (avoid repeating):\n" +
            recent
              .map(function (ex) {
                return "- " + (ex.title || ex.type) + ": " + (ex.description || "").slice(0, 200);
              })
              .join("\n");
        }

        var sysPrompt =
          "You are a CS instructor creating coding practice problems. " +
          "Based on the knowledge article provided, generate ONE coding problem " +
          "that tests understanding of the core concepts. Choose an appropriate " +
          "difficulty level. Analyze the article content and choose the most " +
          "appropriate programming language(s) for the topic — for example, " +
          "systems topics suit C/Rust, web topics suit JavaScript/TypeScript, " +
          "data/algorithm topics suit Python, etc. Include an allowedLanguages " +
          "array listing ALL languages that are appropriate for this problem. " +
          "Return your response as ONLY valid JSON in this exact format:\n" +
          '{"title":"Short problem title",' +
          '"difficulty":"easy|medium|hard",' +
          '"language":"python",' +
          '"allowedLanguages":["python","javascript"],' +
          '"description":"Full problem description in markdown with examples",' +
          '"starterCode":"// starter code template with function signature",' +
          '"solution":"// complete working solution",' +
          '"testDescription":"How to verify the solution is correct",' +
          '"hints":["Progressive hint 1","More specific hint 2","Nearly gives it away hint 3"]}' +
          "\n\nReturn ONLY the JSON object. No markdown fencing, no explanation.";

        var userPrompt =
          "Article title: " + ps.articleTitle + "\n\n" +
          ps.articleContent.slice(0, 6000) + exCtx;

        return chatComplete([
          { role: "system", content: sysPrompt },
          { role: "user", content: userPrompt },
        ]);
      })
      .then(function (response) {
        var jsonStr = response
          .replace(/^```(?:json)?\s*/, "")
          .replace(/```\s*$/, "")
          .trim();
        var problem;
        try {
          problem = JSON.parse(jsonStr);
        } catch (_e) {
          var match = jsonStr.match(/\{[\s\S]*\}/);
          if (match) problem = JSON.parse(match[0]);
          else throw new Error("AI did not return valid JSON");
        }

        problem.articleId = ps.articleId;
        problem.createdAt = Date.now();
        ps.currentProblem = problem;
        ps.hints = problem.hints || [];
        ps.hintIndex = 0;

        return dbPut("problems", problem).then(function () {
          renderProblem(problem);
          updateActionButtons();
          setEditorLanguage(problem.language || "python");
          if (problem.starterCode && ps.editor) {
            ps.editor.setValue(problem.starterCode);
          }
          /* auto-cache problem as example */
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
    chatComplete(
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
    )
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
      "<p>Evaluating your solution\u2026</p></div>";

    chatComplete(
      [
        {
          role: "system",
          content:
            "You are a CS instructor evaluating a student's code solution. " +
            "Compare it against the reference solution. Be encouraging but honest. " +
            "Structure your response as:\n" +
            "**Correctness:** (correct/partially correct/incorrect)\n" +
            "**Feedback:** (what they did well, what could improve)\n" +
            "**Key Insight:** (one takeaway about the underlying concept)\n" +
            "Keep it concise — 4-6 sentences total.",
        },
        {
          role: "user",
          content:
            "Problem: " + ps.currentProblem.description +
            "\n\nReference solution:\n```\n" + ps.currentProblem.solution +
            "\n```\n\nStudent solution:\n```\n" + userCode + "\n```",
        },
      ],
      {
        onChunk: function (_chunk, full) {
          feedbackEl.innerHTML =
            '<div class="feedback-card">' + renderMiniMarkdown(full) + "</div>";
        },
      },
    )
      .then(function (fullResponse) {
        var isCorrect =
          /correct/i.test(fullResponse) && !/incorrect/i.test(fullResponse);

        return dbPut("solutions", {
          articleId: ps.articleId,
          problemId: ps.currentProblem.id,
          code: userCode,
          correct: isCorrect,
          feedback: fullResponse,
          createdAt: Date.now(),
        }).then(function () {
          /* always save submission as example for local cache */
          dbPut("examples", {
            articleId: ps.articleId,
            type: isCorrect ? "solved" : "attempt",
            title: (ps.currentProblem ? ps.currentProblem.title : "Submission") +
              (isCorrect ? " (solved)" : " (attempt)"),
            description: (fullResponse || "").slice(0, 500),
            code: userCode,
            language: ps.currentProblem ? (ps.currentProblem.language || "python") : "python",
            tags: [isCorrect ? "correct" : "attempt", "user-submitted"],
            source: "user-submitted",
            createdAt: Date.now(),
          }).catch(function () { /* best-effort */ });
          if (isCorrect) return generateInsight(userCode, fullResponse);
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
      var jsonStr = response
        .replace(/^```(?:json)?\s*/, "")
        .replace(/```\s*$/, "")
        .trim();
      var insight;
      try {
        insight = JSON.parse(jsonStr);
      } catch (_e) {
        var match = jsonStr.match(/\{[\s\S]*\}/);
        if (match) insight = JSON.parse(match[0]);
        else return;
      }
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
    if (genBtn) {
      genBtn.disabled = busy;
      genBtn.textContent = busy ? "Generating\u2026" : hasProblem ? "New Problem" : "Generate Problem";
    }
    if (hintBtn) hintBtn.disabled = !hasProblem || busy;
    if (submitBtn) submitBtn.disabled = !hasProblem || busy;
    if (showSolBtn) showSolBtn.disabled = !hasProblem || busy;
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
        ? "AI Ready (" + config.model + ")"
        : "Ollama connected \u2014 model not found";
      ps.aiConnected = true;
    } else {
      dot.className = "ai-dot ai-disconnected";
      label.textContent = "AI Offline";
      ps.aiConnected = false;
    }
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

  function startInstallerPolling() {
    stopInstallerPolling();
    installer.pollTimer = setInterval(function () {
      checkConnection().then(function (info) {
        if (info.connected) {
          stopInstallerPolling();
          onOllamaDetected(info);
        }
      });
    }, 2000);
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
  function renderInstallGate() {
    var el = document.getElementById("practice-setup");
    if (!el) return;
    el.classList.remove("hidden");
    hidePracticeContent();

    var os = detectOs();
    var osLabels = { mac: "macOS", linux: "Linux", windows: "Windows", unknown: "your platform" };
    var osLabel = osLabels[os] || "your platform";
    var downloadUrl = getDownloadUrl();

    el.innerHTML =
      '<div class="setup-card setup-gate">' +
      '<div class="setup-gate-icon">' +
      '<svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">' +
      '<path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>' +
      '<polyline points="3.27 6.96 12 12.01 20.73 6.96"/>' +
      '<line x1="12" y1="22.08" x2="12" y2="12"/></svg></div>' +
      '<h3>Install Ollama to Practice</h3>' +
      '<p class="setup-gate-desc">Practice mode uses a local AI model that runs entirely on your machine. ' +
      'No data is sent anywhere \u2014 everything stays private.</p>' +
      '<p class="setup-gate-desc">Install <strong>Ollama</strong> for ' + esc(osLabel) + ', then come back here. ' +
      'We\u2019ll detect it automatically.</p>' +
      '<a class="btn btn-primary btn-lg setup-download-btn" href="' + esc(downloadUrl) + '" target="_blank" rel="noreferrer">' +
      'Download Ollama for ' + esc(osLabel) + '</a>' +
      '<div class="setup-gate-polling">' +
      '<div class="typing-dots"><span></span><span></span><span></span></div>' +
      '<span>Waiting for Ollama\u2026</span></div>' +
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
        renderInstallGate();
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
  function toggle(articleId, articleTitle, articleContent) {
    var panel = document.getElementById("practice-panel");
    if (!panel) return;

    if (ps.active && ps.articleId === articleId) {
      ps.active = false;
      panel.classList.add("hidden");
      document.getElementById("reader-column").classList.remove("practice-open");
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
    refreshAiStatus();

    dbGetByIndex("problems", "articleId", articleId).then(function (problems) {
      if (problems.length > 0) {
        var latest = problems[problems.length - 1];
        ps.currentProblem = latest;
        ps.hints = latest.hints || [];
        renderProblem(latest);
        updateActionButtons();
        setEditorLanguage(latest.language || "python");
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
      var tab = event.target.closest(".practice-tab");
      if (tab && tab.dataset.tab) {
        switchTab(tab.dataset.tab);
        return;
      }
    });

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
    openDb().catch(function (err) {
      console.warn("Practice DB init failed:", err);
    });
  }

  window.AtlasPractice = {
    init: init,
    toggle: toggle,
    isActive: function () {
      return ps.active;
    },
    refreshTheme: syncEditorTheme,
  };
})();
