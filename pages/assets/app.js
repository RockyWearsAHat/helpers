/* -- State ----------------------------------------------------------------- */
const state = {
  page: "search",
  corpus: null,
  query: "",
  scope: "all",
  selectedId: null,
  lastResults: [],
  documentsById: new Map(),
  readerDoc: null,
  readerCache: new Map(),
  routeDocId: "",
  routePractice: false,
  pageSize: 20,
  currentPage: 0,
  isLoadingMore: false,
  lastTerms: [],
};

var loadMoreObserver = null;

/* -- DOM refs -------------------------------------------------------------- */
const queryInput = document.getElementById("query-input");
const resultsList = document.getElementById("results-list");
const resultsSummary = document.getElementById("results-summary");
const resultsMeta = document.getElementById("results-meta");
const previewCard = document.getElementById("preview-card");
const suggestionStrip = document.getElementById("suggestion-strip");
const emptyState = document.getElementById("empty-state");
const resultsColumn = document.getElementById("results-column");
const previewColumn = document.getElementById("preview-column");
const readerColumn = document.getElementById("reader-column");
const readerBack = document.getElementById("reader-back");
const readerBody = document.getElementById("reader-body");
const themeToggle = document.getElementById("theme-toggle");
const pageSearch = document.getElementById("page-search");
const pageAbout = document.getElementById("page-about");
const practiceToggle = document.getElementById("practice-toggle");

const scopeButtons = Array.from(document.querySelectorAll("[data-scope]"));
const navLinks = Array.from(document.querySelectorAll(".nav-link[data-page]"));
const bentoStatButtons = Array.from(
  document.querySelectorAll(".bento-card[data-scope]"),
);
const scopeChipButtons = Array.from(
  document.querySelectorAll(".scope-chip[data-scope]"),
);

/* -- Page navigation ------------------------------------------------------- */
function showPage(name) {
  state.page = name === "about" ? "about" : "search";
  pageSearch.style.display = name === "search" ? "" : "none";
  pageAbout.classList.toggle("page-hidden", name !== "about");
  navLinks.forEach(function (btn) {
    btn.classList.toggle("active", btn.dataset.page === name);
  });
}

function syncPracticeToggleState() {
  if (!practiceToggle) return;
  if (typeof window.AtlasPractice === "undefined") {
    practiceToggle.classList.remove("active");
    return;
  }
  practiceToggle.classList.toggle("active", window.AtlasPractice.isActive());
}

navLinks.forEach(function (btn) {
  btn.addEventListener("click", function () {
    showPage(btn.dataset.page);
    updateUrl();
  });
});

/* -- Theme toggle ---------------------------------------------------------- */
function getTheme() {
  return document.documentElement.getAttribute("data-theme") || "light";
}

function setTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("theme", theme);
}

themeToggle.addEventListener("click", function () {
  setTheme(getTheme() === "dark" ? "light" : "dark");
});

/* -- Utilities ------------------------------------------------------------- */
function escapeHtml(text) {
  return String(text)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function escapeRegex(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function normalizeWhitespace(text) {
  return String(text || "")
    .replace(/\s+/g, " ")
    .trim();
}

function isValidTag(tag) {
  return (
    typeof tag === "string" &&
    tag !== "" &&
    tag.indexOf("/") === -1 &&
    !/^\d{4}-\d{2}-\d{2}/.test(tag) &&
    tag.length <= 40
  );
}

function tokenize(query) {
  return normalizeWhitespace(query)
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(function (term) {
      return term.length >= 2;
    });
}

function countOccurrences(text, term) {
  if (!term) return 0;
  var matches = text.match(new RegExp(escapeRegex(term), "g"));
  return matches ? matches.length : 0;
}

function highlight(text, terms) {
  if (!terms.length) return escapeHtml(text);
  var pattern = new RegExp("\\b(" + terms.map(escapeRegex).join("|") + ")\\b", "gi");
  return escapeHtml(text).replace(pattern, "<mark>$1</mark>");
}

function buildSnippet(doc, terms) {
  var haystack = doc.searchText || doc.snippet || "";
  // Strip Markdown horizontal rules, setext separators, and table separator rows
  haystack = haystack.replace(/(?:^|\n)\s*[-=]{3,}\s*(?=\n|$)/g, " ");
  haystack = haystack.replace(/(?:^|\n)\s*[|\-: ]+\s*(?=\n|$)/g, " ");
  haystack = haystack.replace(/\s{2,}/g, " ").trim();
  var titleLower = (doc.title || "").toLowerCase();
  if (titleLower && haystack.toLowerCase().startsWith(titleLower)) {
    haystack = haystack.slice(titleLower.length).replace(/^[\s,.:;-]+/, "");
  }
  var lowerHaystack = haystack.toLowerCase();
  var start = 0;
  for (var i = 0; i < terms.length; i++) {
    var index = lowerHaystack.indexOf(terms[i]);
    if (index !== -1) {
      start = Math.max(0, index - 90);
      break;
    }
  }
  var end = Math.min(haystack.length, start + 260);
  var prefix = start > 0 ? "\u2026" : "";
  var suffix = end < haystack.length ? "\u2026" : "";
  var snippet = prefix + haystack.slice(start, end).trim() + suffix;

  // Quality filter: word-cloud detection. If the snippet has very little
  // punctuation relative to word count it's probably raw token noise — fall
  // back to previewText or the first real sentence in the haystack.
  var cleanSnippet = snippet.replace(/^\u2026/, "").replace(/\u2026$/, "").trim();
  var snippetWords = cleanSnippet.split(/\s+/).length;
  var snippetPunct = (cleanSnippet.match(/[.,;:!?()]/g) || []).length;
  if (snippetWords >= 10 && snippetPunct < snippetWords / 8) {
    var fallback = doc.previewText || "";
    if (fallback && fallback !== haystack) {
      var fb = fallback.slice(0, 260).trim();
      return fb.length < fallback.length ? fb + "\u2026" : fb;
    }
    var firstSentenceMatch = haystack.match(/[A-Z][^.!?]{15,}[.!?]/);
    if (firstSentenceMatch) {
      return firstSentenceMatch[0].trim();
    }
  }

  return snippet;
}

/* -- Scoring --------------------------------------------------------------- */
function scoreDocument(doc, normalizedQuery, terms) {
  if (state.scope !== "all" && doc.scopeKey !== state.scope) return 0;

  if (!normalizedQuery) {
    if (doc.scopeKey === "copilot") return 16;
    if (doc.scopeKey === "knowledge") return 14;
    return 5;
  }

  var title = doc.title.toLowerCase();
  var path = doc.path.toLowerCase();
  var headings = doc.headings.map(function (h) {
    return h.toLowerCase();
  });
  var keywords = doc.keywords.map(function (k) {
    return k.toLowerCase();
  });
  var topics = (doc.topics || []).map(function (t) {
    return t.toLowerCase();
  });
  var metaPills = (doc.metaPills || []).join(" ").toLowerCase();
  var searchText = doc.searchText.toLowerCase();

  var score =
    doc.scopeKey === "knowledge" ? 8 : doc.scopeKey === "copilot" ? 12 : 0;
  var matchedTerms = 0;

  if (normalizedQuery && title.includes(normalizedQuery)) {
    score += 180;
  } else if (normalizedQuery && searchText.includes(normalizedQuery)) {
    score += 80;
  }

  for (var i = 0; i < terms.length; i++) {
    var term = terms[i];
    var termScore = 0;
    if (title.includes(term)) termScore += 110;
    if (
      headings.some(function (h) {
        return h.includes(term);
      })
    )
      termScore += 50;
    if (keywords.includes(term)) termScore += 36;
    if (
      topics.some(function (t) {
        return t.includes(term);
      })
    )
      termScore += 32;
    if (metaPills.includes(term)) termScore += 18;
    if (path.includes(term)) termScore += 22;
    termScore += Math.min(countOccurrences(searchText, term), 6) * 11;
    if (termScore > 0) matchedTerms += 1;
    score += termScore;
  }

  if (!matchedTerms) return 0;
  score *=
    matchedTerms === terms.length ? 1.22 : 0.58 + matchedTerms / terms.length;
  return score;
}

function sortResults(results) {
  return results.sort(function (a, b) {
    if (b.score !== a.score) return b.score - a.score;
    return a.document.title.localeCompare(b.document.title);
  });
}

function summaryForScope(scopeKey) {
  if (scopeKey === "copilot") return "Top Copilot guides.";
  if (scopeKey === "knowledge") return "Top knowledge notes.";
  if (scopeKey === "archive") return "Top archive entries.";
  return "Top guides across Copilot customization, knowledge atlas, and archive.";
}

function buildChipButton(label, className, options) {
  var attrs = [
    'type="button"',
    'class="' + escapeHtml(className + " chip-button") + '"',
  ];

  if (options && options.query) {
    attrs.push('data-chip-query="' + escapeHtml(options.query) + '"');
  }
  if (options && options.scope) {
    attrs.push('data-chip-scope="' + escapeHtml(options.scope) + '"');
  }
  if (options && options.clearQuery) {
    attrs.push('data-chip-clear="true"');
  }

  return "<button " + attrs.join(" ") + ">" + escapeHtml(label) + "</button>";
}

function activateChip(button) {
  if (!button) return;

  var nextScope = button.dataset.chipScope || state.scope;
  var nextQuery = normalizeWhitespace(button.dataset.chipQuery || "");
  var shouldClear = button.dataset.chipClear === "true";

  showPage("search");
  if (!readerColumn.classList.contains("hidden")) closeReader();

  state.scope = nextScope;
  state.query = shouldClear ? "" : nextQuery;
  state.currentPage = 0;
  queryInput.value = state.query;

  syncScopeButtons();
  updateUrl();
  runSearch();
  queryInput.focus();
}

/* -- Render: suggestions --------------------------------------------------- */
function renderSuggestions() {
  suggestionStrip.innerHTML = "";
  if (!state.corpus) return;
  state.corpus.metadata.featuredCategories.forEach(function (cat) {
    var btn = document.createElement("button");
    btn.type = "button";
    btn.className = "suggestion-button";
    btn.textContent = cat.label + " (" + cat.count + ")";
    btn.addEventListener("click", function () {
      queryInput.value = cat.label;
      state.query = cat.label;
      state.currentPage = 0;
      updateUrl();
      runSearch();
      queryInput.focus();
    });
    suggestionStrip.appendChild(btn);
  });
}

/* -- Render: stats --------------------------------------------------------- */
function renderStats() {
  if (!state.corpus) return;
  var m = state.corpus.metadata;
  document.getElementById("doc-count").textContent = m.totalDocuments;
  document.getElementById("copilot-count").textContent =
    m.scopeCounts.copilot || 0;
  document.getElementById("curated-count").textContent =
    m.scopeCounts.knowledge || 0;
  document.getElementById("archive-count").textContent =
    m.scopeCounts.archive || 0;
  var builtAt = new Date(m.builtAt);
  document.getElementById("built-at").textContent = builtAt.toLocaleDateString(
    undefined,
    { month: "short", day: "numeric", year: "numeric" },
  );
  var bentoGrid = document.querySelector(".bento-grid");
  if (bentoGrid) bentoGrid.classList.remove("bento-loading");
}

/* -- Render: preview card -------------------------------------------------- */
function renderPreview(doc) {
  if (!doc) {
    previewCard.innerHTML =
      '<p class="preview-kicker">Preview</p>' +
      "<h2>Select a result to preview.</h2>" +
      '<p class="preview-body">Hover or click any result card to see a detailed breakdown here.</p>';
    return;
  }

  var topicPills = (doc.topics && doc.topics.length ? doc.topics : doc.keywords)
    .filter(isValidTag)
    .slice(0, 10)
    .map(function (item) {
      return buildChipButton(item, "keyword-pill", { query: item });
    })
    .join("");

  var highlightPills = (
    doc.highlights && doc.highlights.length ? doc.highlights : doc.headings
  )
    .filter(isValidTag)
    .slice(0, 6)
    .map(function (item) {
      return buildChipButton(item, "meta-pill", { query: item });
    })
    .join("");

  var resourceLinks = (doc.resourceLinks || [])
    .map(function (link) {
      return (
        '<a class="preview-link" href="' +
        escapeHtml(link.url) +
        '" target="_blank" rel="noreferrer">' +
        escapeHtml(link.label) +
        "</a>"
      );
    })
    .join("");

  var relatedButtons = (doc.relatedIds || [])
    .map(function (id) {
      return state.documentsById.get(id);
    })
    .filter(Boolean)
    .map(function (rel) {
      return (
        '<button class="related-trigger" type="button" data-related-id="' +
        escapeHtml(rel.id) +
        '">' +
        escapeHtml(rel.title) +
        "</button>"
      );
    })
    .join("");

  var metaPills = (doc.metaPills || [doc.scopeLabel, doc.category])
    .filter(isValidTag)
    .map(function (pill, index) {
      if (index === 0) {
        return buildChipButton(pill, "meta-pill", {
          scope: doc.scopeKey,
          clearQuery: true,
        });
      }
      return buildChipButton(pill, "meta-pill", { query: pill });
    })
    .join("");

  previewCard.innerHTML =
    '<p class="preview-kicker">' +
    escapeHtml(doc.scopeLabel) +
    "</p>" +
    "<h2>" +
    escapeHtml(doc.title) +
    "</h2>" +
    '<div class="preview-meta">' +
    metaPills +
    buildChipButton(doc.path, "meta-pill meta-pill-path", {
      query: doc.path,
    }) +
    "</div>" +
    '<p class="preview-body">' +
    escapeHtml(doc.previewText || doc.snippet) +
    "</p>" +
    (doc.rawUrl
      ? '<button class="read-article-btn" type="button" data-doc-id="' +
        escapeHtml(doc.id) +
        '">Read full article &rarr;</button>'
      : "") +
    (resourceLinks
      ? '<details class="dev-links-details"><summary>Developer links</summary><div class="preview-links">' +
        resourceLinks +
        "</div></details>"
      : "") +
    '<p class="preview-section-title">Topics</p>' +
    '<div class="preview-keywords">' +
    (topicPills || '<span class="meta-pill">No extracted topics</span>') +
    "</div>" +
    (highlightPills
      ? '<p class="preview-section-title">' +
        (doc.documentType === "community" ? "Key guidance" : "Section headings") +
        "</p>" +
        '<div class="preview-headings">' + highlightPills + "</div>"
      : "") +
    (relatedButtons
      ? '<p class="preview-section-title">Related next steps</p><div class="preview-related">' +
        relatedButtons +
        "</div>"
      : "");

  previewCard.querySelectorAll("[data-related-id]").forEach(function (btn) {
    btn.addEventListener("click", function () {
      setSelected(btn.dataset.relatedId);
    });
  });

  var readBtn = previewCard.querySelector(".read-article-btn");
  if (readBtn) {
    readBtn.addEventListener("click", function () {
      openReader(readBtn.dataset.docId);
    });
  }

  previewColumn.scrollTop = 0;
  requestAnimationFrame(updatePreviewScrollState);
}

/* -- Set selected ---------------------------------------------------------- */
function setSelected(documentId) {
  state.selectedId = documentId;
  var selected =
    (
      state.lastResults.find(function (e) {
        return e.document.id === documentId;
      }) || {}
    ).document ||
    state.documentsById.get(documentId) ||
    null;
  renderPreview(selected);
  resultsList.querySelectorAll(".result-card").forEach(function (card) {
    card.classList.toggle("active", card.dataset.id === documentId);
  });
}

function disconnectLoadMoreObserver() {
  if (!loadMoreObserver) return;
  loadMoreObserver.disconnect();
  loadMoreObserver = null;
}

function renderSkeletonCards(count) {
  var container = resultsList.querySelector(".skeleton-group");
  if (container) container.remove();

  var group = document.createElement("li");
  group.className = "skeleton-group";
  group.setAttribute("aria-hidden", "true");

  for (var i = 0; i < count; i++) {
    group.innerHTML +=
      '<div class="skeleton-card" style="animation-delay:' + (i * 60) + 'ms">' +
        '<div class="skeleton-line skeleton-line-short"></div>' +
        '<div class="skeleton-line skeleton-line-title"></div>' +
        '<div class="skeleton-line skeleton-line-body"></div>' +
        '<div class="skeleton-line skeleton-line-body skeleton-line-body-short"></div>' +
        '<div class="skeleton-pills">' +
          '<div class="skeleton-pill"></div>' +
          '<div class="skeleton-pill"></div>' +
          '<div class="skeleton-pill"></div>' +
        '</div>' +
      '</div>';
  }
  resultsList.appendChild(group);
  return group;
}

function loadNextResultsPage() {
  if (state.isLoadingMore) return;
  var nextStart = (state.currentPage + 1) * state.pageSize;
  if (nextStart >= state.lastResults.length) {
    disconnectLoadMoreObserver();
    return;
  }

  state.isLoadingMore = true;
  disconnectLoadMoreObserver();

  var existingSentinel = resultsList.querySelector(".load-more-item");
  if (existingSentinel) existingSentinel.remove();

  var remaining = state.lastResults.length - nextStart;
  var skeletonCount = Math.min(remaining, state.pageSize, 5);
  renderSkeletonCards(skeletonCount);

  requestAnimationFrame(function () {
    requestAnimationFrame(function () {
      var skeletons = resultsList.querySelector(".skeleton-group");
      state.currentPage += 1;
      var start = state.currentPage * state.pageSize;
      var end = start + state.pageSize;
      var pageResults = state.lastResults.slice(start, end);
      var terms = state.lastTerms;
      var frag = document.createDocumentFragment();

      pageResults.forEach(function (entry) {
        var resultDoc = entry.document;
        var listItem = document.createElement("li");
        var snippet = buildSnippet(resultDoc, terms);

        var pillsHtml = (
          resultDoc.resultPills || [resultDoc.scopeLabel, resultDoc.category]
        )
          .filter(isValidTag)
          .slice(0, 4)
          .map(function (k) {
            return buildChipButton(k, "result-pill", { query: k });
          })
          .join("");

        var readBtnHtml = resultDoc.rawUrl
          ? '<button class="read-article-btn" type="button" data-doc-id="' +
            escapeHtml(resultDoc.id) +
            '">Read article &rarr;</button>'
          : "";

        listItem.innerHTML =
          '<article class="result-card" data-id="' +
          escapeHtml(resultDoc.id) +
          '" tabindex="0">' +
          '<div class="result-topline"><div>' +
          '<p class="result-path">' +
          escapeHtml((resultDoc.path.split("/").pop() || resultDoc.path).replace(/\s+[\u00B7\u2014\u2013]\s+.*$/, "").trim()) +
          "</p>" +
          '<h2 class="result-title"><span class="result-link">' +
          highlight(resultDoc.title, terms) +
          "</span></h2>" +
          "</div>" +
          '<button class="result-scope-badge chip-button" type="button" data-chip-scope="' +
          escapeHtml(resultDoc.scopeKey) +
          '" data-chip-clear="true" data-scope="' +
          escapeHtml(resultDoc.scopeKey) +
          '">' +
          escapeHtml(resultDoc.scopeLabel) +
          "</button></div>" +
          '<p class="result-snippet">' +
          highlight(snippet, terms) +
          "</p>" +
          '<div class="result-pills">' +
          pillsHtml +
          "</div>" +
          readBtnHtml +
          "</article>";

        var card = listItem.firstElementChild;
        var activate = function () {
          setSelected(resultDoc.id);
        };
        card.addEventListener("mouseenter", activate);
        card.addEventListener("focus", activate);
        card.addEventListener("click", function (event) {
          if (
            event.target.closest(".read-article-btn") ||
            event.target.closest("[data-chip-query], [data-chip-scope]")
          )
            return;
          activate();
          if (resultDoc.rawUrl) openReader(resultDoc.id);
        });
        card.addEventListener("keydown", function (event) {
          if (event.key !== "Enter") return;
          event.preventDefault();
          activate();
          if (resultDoc.rawUrl) openReader(resultDoc.id);
        });

        var readBtn = card.querySelector(".read-article-btn");
        if (readBtn) {
          readBtn.addEventListener("click", function (event) {
            event.stopPropagation();
            openReader(resultDoc.id);
          });
        }

        frag.appendChild(listItem);
      });

      if (skeletons) skeletons.replaceWith(frag);
      else resultsList.appendChild(frag);

      if (end < state.lastResults.length) {
        var remainingCount = state.lastResults.length - end;
        var loadMoreLi = document.createElement("li");
        loadMoreLi.className = "load-more-item";
        loadMoreLi.innerHTML =
          '<div class="load-more-indicator" aria-hidden="true">' +
            '<span class="load-more-count">' + remainingCount + ' more</span>' +
          '</div>';
        resultsList.appendChild(loadMoreLi);
        attachLoadMoreObserver();
      } else {
        if (state.lastResults.length > state.pageSize) {
          var endMarker = document.createElement("li");
          endMarker.className = "load-more-item";
          endMarker.innerHTML = '<div class="load-end-indicator" aria-hidden="true">All results loaded</div>';
          resultsList.appendChild(endMarker);
        }
      }

      state.isLoadingMore = false;
    });
  });
}

function attachLoadMoreObserver() {
  disconnectLoadMoreObserver();

  var sentinel = resultsList.querySelector(".load-more-item");
  if (!sentinel || !("IntersectionObserver" in window)) return;

  loadMoreObserver = new IntersectionObserver(
    function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) loadNextResultsPage();
      });
    },
    {
      root: null,
      rootMargin: "260px 0px 160px",
      threshold: 0.01,
    },
  );

  loadMoreObserver.observe(sentinel);
}

/* -- Render: result list --------------------------------------------------- */
function renderResults(results, durationMs, terms) {
  disconnectLoadMoreObserver();

  if (state.currentPage === 0) {
    resultsList.innerHTML = "";
    emptyState.hidden = results.length > 0;

    if (!state.query) {
      resultsSummary.textContent = summaryForScope(state.scope);
      resultsMeta.textContent =
        results.length.toLocaleString() + " curated picks";
    } else {
      resultsSummary.textContent =
        "About " +
        results.length.toLocaleString() +
        " result" +
        (results.length === 1 ? "" : "s");
      resultsMeta.textContent = durationMs.toFixed(1) + " ms";
    }

    if (!results.length) {
      if (state.query) {
        resultsSummary.textContent = "No results found";
        resultsMeta.textContent = "Try broader terms or switch scope.";
      }
      renderPreview(null);
      return;
    }
  } else {
    var existingLoadMore = resultsList.querySelector(".load-more-item");
    if (existingLoadMore) existingLoadMore.remove();
  }

  var start = state.currentPage * state.pageSize;
  var end = start + state.pageSize;
  var pageResults = results.slice(start, end);

  pageResults.forEach(function (entry) {
    var resultDoc = entry.document;
    var listItem = document.createElement("li");
    var snippet = buildSnippet(resultDoc, terms);

    var pillsHtml = (
      resultDoc.resultPills || [resultDoc.scopeLabel, resultDoc.category]
    )
      .filter(isValidTag)
      .slice(0, 4)
      .map(function (k) {
        return buildChipButton(k, "result-pill", { query: k });
      })
      .join("");

    var readBtnHtml = resultDoc.rawUrl
      ? '<button class="read-article-btn" type="button" data-doc-id="' +
        escapeHtml(resultDoc.id) +
        '">Read article &rarr;</button>'
      : "";

    listItem.innerHTML =
      '<article class="result-card" data-id="' +
      escapeHtml(resultDoc.id) +
      '" tabindex="0">' +
      '<div class="result-topline"><div>' +
      '<p class="result-path">' +
      escapeHtml((resultDoc.path.split("/").pop() || resultDoc.path).replace(/\s+[\u00B7\u2014\u2013]\s+.*$/, "").trim()) +
      "</p>" +
      '<h2 class="result-title"><span class="result-link">' +
      highlight(resultDoc.title, terms) +
      "</span></h2>" +
      "</div>" +
      '<button class="result-scope-badge chip-button" type="button" data-chip-scope="' +
      escapeHtml(resultDoc.scopeKey) +
      '" data-chip-clear="true" data-scope="' +
      escapeHtml(resultDoc.scopeKey) +
      '">' +
      escapeHtml(resultDoc.scopeLabel) +
      "</button></div>" +
      '<p class="result-snippet">' +
      highlight(snippet, terms) +
      "</p>" +
      '<div class="result-pills">' +
      pillsHtml +
      "</div>" +
      readBtnHtml +
      "</article>";

    var card = listItem.firstElementChild;
    var activate = function () {
      setSelected(resultDoc.id);
    };
    card.addEventListener("mouseenter", activate);
    card.addEventListener("focus", activate);
    card.addEventListener("click", function (event) {
      if (
        event.target.closest(".read-article-btn") ||
        event.target.closest("[data-chip-query], [data-chip-scope]")
      )
        return;
      activate();
      if (resultDoc.rawUrl) openReader(resultDoc.id);
    });
    card.addEventListener("keydown", function (event) {
      if (event.key !== "Enter") return;
      event.preventDefault();
      activate();
      if (resultDoc.rawUrl) openReader(resultDoc.id);
    });

    var readBtn = card.querySelector(".read-article-btn");
    if (readBtn) {
      readBtn.addEventListener("click", function (event) {
        event.stopPropagation();
        openReader(resultDoc.id);
      });
    }

    resultsList.appendChild(listItem);
  });

  if (state.currentPage === 0 && results.length > 0) {
    setSelected(results[0].document.id);
  }

  if (end < results.length) {
    var remainingCount = results.length - end;
    var loadMoreLi = document.createElement("li");
    loadMoreLi.className = "load-more-item";
    loadMoreLi.innerHTML =
      '<div class="load-more-indicator" aria-hidden="true">' +
        '<span class="load-more-count">' + remainingCount + ' more</span>' +
      '</div>';
    resultsList.appendChild(loadMoreLi);
    if ("IntersectionObserver" in window) {
      attachLoadMoreObserver();
    } else {
      loadMoreLi.innerHTML =
        '<button class="load-more-btn" type="button">Load ' +
        Math.min(remainingCount, state.pageSize) + ' more results&hellip;</button>';
      loadMoreLi.querySelector(".load-more-btn").addEventListener("click", function () {
        loadNextResultsPage();
      });
    }
  } else {
    disconnectLoadMoreObserver();
    if (results.length > state.pageSize) {
      var endMarker = document.createElement("li");
      endMarker.className = "load-more-item";
      endMarker.innerHTML = '<div class="load-end-indicator" aria-hidden="true">All results loaded</div>';
      resultsList.appendChild(endMarker);
    }
  }
}

/* -- URL sync -------------------------------------------------------------- */
function updateUrl() {
  var params = new URLSearchParams(window.location.search);
  if (state.page !== "search") params.set("page", state.page);
  else params.delete("page");
  if (state.query) params.set("q", state.query);
  else params.delete("q");
  if (state.scope !== "all") params.set("scope", state.scope);
  else params.delete("scope");
  if (state.readerDoc && state.readerDoc.id) params.set("doc", state.readerDoc.id);
  else params.delete("doc");
  if (
    state.readerDoc &&
    typeof window.AtlasPractice !== "undefined" &&
    window.AtlasPractice.isActive()
  )
    params.set("practice", "1");
  else params.delete("practice");
  var nextUrl =
    window.location.pathname + (params.toString() ? "?" + params : "");
  window.history.replaceState({}, "", nextUrl);
}

function syncScopeButtons() {
  scopeButtons.forEach(function (btn) {
    btn.classList.toggle("active", btn.dataset.scope === state.scope);
  });
}

/* -- Search ---------------------------------------------------------------- */
function runSearch() {
  if (!state.corpus) return;
  var normalizedQuery = normalizeWhitespace(state.query).toLowerCase();
  var terms = tokenize(normalizedQuery);
  var startedAt = performance.now();

  state.isLoadingMore = false;
  state.lastTerms = terms;

  var results = sortResults(
    state.corpus.documents
      .map(function (doc) {
        return {
          document: doc,
          score: scoreDocument(doc, normalizedQuery, terms),
        };
      })
      .filter(function (e) {
        return e.score > 0;
      }),
  );

  state.lastResults = results;
  renderResults(results, performance.now() - startedAt, terms);
}

/* -- Markdown -> HTML (lightweight client-side) ---------------------------- */
function markdownToHtml(md) {
  var html = md;

  html = html.replace(/^#### (.+)$/gm, "<h4>$1</h4>");
  html = html.replace(/^### (.+)$/gm, "<h3>$1</h3>");
  html = html.replace(/^## (.+)$/gm, "<h2>$1</h2>");
  html = html.replace(/^# (.+)$/gm, "<h1>$1</h1>");
  html = html.replace(/^---$/gm, "<hr>");
  html = html.replace(/^> (.+)$/gm, "<blockquote><p>$1</p></blockquote>");

  html = html.replace(
    /```(\w*)\n([\s\S]*?)```/g,
    function (_match, lang, code) {
      var cls = lang ? ' class="language-' + escapeHtml(lang) + '"' : '';
      return "<pre><code" + cls + ">" + escapeHtml(code.trimEnd()) + "</code></pre>";
    },
  );

  html = html.replace(/`([^`]+)`/g, "<code>$1</code>");
  html = html.replace(/\*\*([^*\n]+)\*\*/g, "<strong>$1</strong>");
  html = html.replace(/__([^_\n]+)__/g, "<strong>$1</strong>");
  html = html.replace(/(?<!\*)\*([^*\n]+)\*(?!\*)/g, "<em>$1</em>");
  html = html.replace(/(?<!_)_([^_\n]+)_(?!_)/g, "<em>$1</em>");

  html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, function (_m, text, href) {
    var safeHref = escapeHtml(href);
    return (
      '<a href="' +
      safeHref +
      '" target="_blank" rel="noreferrer">' +
      escapeHtml(text) +
      "</a>"
    );
  });

  html = html.replace(/^\|[-:| ]+\|$/gm, "");
  html = html.replace(/^\| (.+) \|$/gm, function (_m, row) {
    if (/^[\s|:-]+$/.test(row)) return "";
    var cells = row.split("|").map(function (c) {
      return c.trim();
    });
    return (
      "<tr>" +
      cells
        .map(function (c) {
          return "<td>" + c + "</td>";
        })
        .join("") +
      "</tr>"
    );
  });
  html = html.replace(/((?:<tr>.*<\/tr>\s*)+)/g, "<table>$1</table>");
  html = html.replace(/^[-*] (.+)$/gm, "<li>$1</li>");
  html = html.replace(/((?:<li>.*<\/li>\s*)+)/g, "<ul>$1</ul>");
  html = html.replace(/^\d+\. (.+)$/gm, "<li>$1</li>");

  var lines = html.split("\n");
  var out = [];
  var inParagraph = false;
  for (var i = 0; i < lines.length; i++) {
    var line = lines[i];
    var isBlock =
      /^<(h[1-6]|pre|ul|ol|li|table|tr|td|th|blockquote|hr|div)/.test(line);
    var isEmpty = line.trim() === "";
    if (isEmpty) {
      if (inParagraph) {
        out.push("</p>");
        inParagraph = false;
      }
    } else if (isBlock) {
      if (inParagraph) {
        out.push("</p>");
        inParagraph = false;
      }
      out.push(line);
    } else {
      if (!inParagraph) {
        out.push("<p>");
        inParagraph = true;
      }
      out.push(line);
    }
  }
  if (inParagraph) out.push("</p>");
  var result = out.join("\n");
  result = result.replace(/<(h[1-6])>\s*<em>([\s\S]*?)<\/em>\s*<\/(h[1-6])>/g, function (_m, tag, inner) {
    return "<" + tag + ">" + inner + "</" + tag + ">";
  });
  return result;
}

/* -- Slug link resolver ---------------------------------------------------- */
function resolveSlugLinks(html) {
  // Convert <a href="slug">...</a> patterns to internal see-also links
  html = html.replace(
    /<a href="([^"]+)" target="_blank" rel="noreferrer">([^<]+)<\/a>/g,
    function (match, href, text) {
      if (!(/^[a-z][a-z0-9-]*$/.test(href))) return match;
      var docEntry = state.documentsById.get(href);
      var label = docEntry
        ? escapeHtml(docEntry.title)
        : (text !== href
          ? escapeHtml(text)
          : escapeHtml(href.replace(/-/g, " ").replace(/\b[a-z]/g, function (c) { return c.toUpperCase(); })));
      return '<a href="javascript:void(0)" class="see-also-link" data-slug="' + escapeHtml(href) + '">' + label + '<\/a>';
    }
  );
  // Convert bare hyphenated slug text in list items to internal links
  html = html.replace(/<li>([a-z][a-z0-9-]*(?:-[a-z0-9]+)+)<\/li>/g, function (match, slug) {
    var docEntry = state.documentsById.get(slug);
    var label = docEntry
      ? escapeHtml(docEntry.title)
      : escapeHtml(slug.replace(/-/g, " ").replace(/\b\w/g, function (c) { return c.toUpperCase(); }));
    return '<li><a href="javascript:void(0)" class="see-also-link" data-slug="' + escapeHtml(slug) + '">' + label + '<\/a><\/li>';
  });
  return html;
}

/* -- Article reader -------------------------------------------------------- */
function renderCommunityContent(doc) {
  var cc = doc.communityContent;
  if (!cc || !cc.item) return null;
  var item = cc.item;
  var parts = [];

  parts.push('<h1>' + escapeHtml(doc.title) + '</h1>');

  var metaBadges = [cc.category, cc.kind].filter(Boolean);
  if (item.applicability) metaBadges.push(item.applicability);
  if (item.recommendationStrength) metaBadges.push(item.recommendationStrength);
  if (item.authoritativeSupport) metaBadges.push(item.authoritativeSupport + ' support');
  if (item.severity) metaBadges.push('Severity: ' + item.severity);
  if (item.confidence) metaBadges.push(item.confidence);

  if (metaBadges.length) {
    parts.push('<div class="community-badges">' +
      metaBadges.filter(isValidTag).map(function (b) {
        return buildChipButton(b, 'community-badge', { query: b });
      }).join('') +
    '</div>');
  }

  var description = item.statement || item.description || item.what || item.covers || item.whyItMatters || '';
  if (description) {
    parts.push('<div class="community-description"><p>' + escapeHtml(description) + '</p></div>');
  }

  if (item.replacedBy) {
    parts.push('<div class="community-callout community-callout-warning">' +
      '<strong>Replacement:</strong> ' + escapeHtml(item.replacedBy) +
    '</div>');
  }
  if (item.impact) {
    parts.push('<div class="community-callout community-callout-info">' +
      '<strong>Impact:</strong> ' + escapeHtml(item.impact) +
    '</div>');
  }
  if (item.detectedIn) {
    parts.push('<p class="community-detail"><strong>Detected in:</strong> ' + escapeHtml(item.detectedIn) + '</p>');
  }
  if (item.freshness) {
    parts.push('<p class="community-detail"><strong>Freshness:</strong> ' + escapeHtml(item.freshness) + '</p>');
  }

  if (item.exemplars && item.exemplars.length) {
    parts.push('<h2>Examples</h2><ul>' +
      item.exemplars.map(function (e) { return '<li>' + escapeHtml(e) + '</li>'; }).join('') +
    '</ul>');
  }

  if (item.steps && item.steps.length) {
    parts.push('<h2>Steps</h2><ol>' +
      item.steps.map(function (s) { return '<li>' + escapeHtml(s) + '</li>'; }).join('') +
    '</ol>');
  }

  if (item.whatToUse && item.whatToUse.length) {
    parts.push('<h2>What to use</h2><ul>' +
      item.whatToUse.map(function (w) { return '<li>' + escapeHtml(w) + '</li>'; }).join('') +
    '</ul>');
  }

  if (item.whatToAvoid && item.whatToAvoid.length) {
    parts.push('<h2>What to avoid</h2><ul>' +
      item.whatToAvoid.map(function (w) { return '<li class="community-avoid">' + escapeHtml(w) + '</li>'; }).join('') +
    '</ul>');
  }

  if (item.maintainer) {
    parts.push('<p class="community-detail"><strong>Maintainer:</strong> ' + escapeHtml(item.maintainer) + '</p>');
  }

  var topics = (item.topics || doc.topics || []).filter(isValidTag);
  if (topics.length) {
    parts.push('<h2>Topics</h2><div class="community-topics">' +
      topics.map(function (t) {
        return buildChipButton(t, 'community-topic', { query: t });
      }).join('') +
    '</div>');
  }

  var refs = item.evidenceRefs || [];
  if (refs.length) {
    parts.push('<h2>Evidence &amp; Sources</h2><div class="community-evidence">');
    refs.forEach(function (ref) {
      var sourceLabel = ref.sourceType ? ' <span class="community-source-type">' + escapeHtml(ref.sourceType) + '</span>' : '';
      if (ref.url) {
        parts.push('<a class="community-evidence-link" href="' + escapeHtml(ref.url) +
          '" target="_blank" rel="noreferrer">' + escapeHtml(ref.title || ref.url) + sourceLabel + '</a>');
      } else if (ref.title) {
        parts.push('<span class="community-evidence-item">' + escapeHtml(ref.title) + sourceLabel + '</span>');
      }
    });
    parts.push('</div>');
  }

  var links = (doc.resourceLinks || []).filter(function (l) { return l && l.url; });
  if (links.length) {
    parts.push('<h2>Resources</h2><div class="community-resources">' +
      links.map(function (link) {
        return '<a class="community-resource-link" href="' + escapeHtml(link.url) +
          '" target="_blank" rel="noreferrer">' + escapeHtml(link.label || link.url) + '</a>';
      }).join('') +
    '</div>');
  }

  return parts.join('\n');
}

function highlightReaderCode() {
  if (typeof hljs !== "undefined") {
    readerBody.querySelectorAll("pre code").forEach(function (block) {
      hljs.highlightElement(block);
    });
  }
}

function openReader(docId, options) {
  options = options || {};
  var doc = state.documentsById.get(docId);
  if (!doc) return false;

  state.readerDoc = doc;
  if (options.showPage !== false) showPage("search");
  resultsColumn.classList.add("hidden");
  previewColumn.classList.add("hidden");
  readerColumn.classList.remove("hidden");
  readerColumn.classList.remove("practice-open");
  readerBody.innerHTML = '<p class="reader-loading">Loading article\u2026</p>';
  var practicePanel = document.getElementById("practice-panel");
  if (practicePanel) practicePanel.classList.add("hidden");
  if (
    typeof window.AtlasPractice !== "undefined" &&
    typeof window.AtlasPractice.close === "function"
  ) {
    window.AtlasPractice.close();
  }
  syncPracticeToggleState();
  if (options.syncUrl !== false) updateUrl();

  window.scrollTo({ top: 0, behavior: "smooth" });

  if (doc.communityContent) {
    var rendered = renderCommunityContent(doc);
    if (rendered) {
      rendered = resolveSlugLinks(rendered);
      state.readerCache.set(docId, rendered);
      readerBody.innerHTML = rendered;
      highlightReaderCode();
      return true;
    }
  }

  if (!doc.rawUrl) {
    readerBody.innerHTML = "<p>No article content available for this item.</p>";
    return true;
  }

  if (state.readerCache.has(docId)) {
    readerBody.innerHTML = state.readerCache.get(docId);
    highlightReaderCode();
    return true;
  }

  fetch(doc.rawUrl)
    .then(function (res) {
      if (!res.ok) throw new Error("HTTP " + res.status);
      return res.text();
    })
    .then(function (md) {
      var stripped = md.replace(/^---[\s\S]*?---\s*/, "");
      var rendered = markdownToHtml(stripped);
      rendered = resolveSlugLinks(rendered);
      state.readerCache.set(docId, rendered);
      if (state.readerDoc && state.readerDoc.id === docId) {
        readerBody.innerHTML = rendered;
        highlightReaderCode();
      }
    })
    .catch(function (err) {
      readerBody.innerHTML =
        "<p>Failed to load article: " + escapeHtml(err.message) + "</p>";
    });

  return true;
}

function closeReader(options) {
  options = options || {};
  state.readerDoc = null;
  readerColumn.classList.add("hidden");
  resultsColumn.classList.remove("hidden");
  previewColumn.classList.remove("hidden");
  if (
    typeof window.AtlasPractice !== "undefined" &&
    typeof window.AtlasPractice.close === "function"
  ) {
    window.AtlasPractice.close();
  }
  syncPracticeToggleState();
  if (options.syncUrl !== false) updateUrl();
}

readerBack.addEventListener("click", closeReader);

/* -- Preview scroll indicator ---------------------------------------------- */
function updatePreviewScrollState() {
  var el = previewColumn;
  var isScrollable = el.scrollHeight > el.clientHeight + 8;
  var isAtEnd = el.scrollTop + el.clientHeight >= el.scrollHeight - 8;
  el.classList.toggle("is-scrollable", isScrollable);
  el.classList.toggle("is-scrolled-end", isAtEnd);
}

previewColumn.addEventListener("scroll", updatePreviewScrollState, { passive: true });

/* -- Scroll-based card preview sync ---------------------------------------- */
var scrollSelectionTimer = null;
resultsColumn.addEventListener("scroll", function () {
  clearTimeout(scrollSelectionTimer);
  scrollSelectionTimer = window.setTimeout(function () {
    var cards = resultsList.querySelectorAll(".result-card");
    if (!cards.length) return;
    var colRect = resultsColumn.getBoundingClientRect();
    var topCard = null;
    for (var i = 0; i < cards.length; i++) {
      var r = cards[i].getBoundingClientRect();
      if (r.bottom > colRect.top && r.top < colRect.bottom) {
        topCard = cards[i];
        break;
      }
    }
    if (topCard && topCard.dataset.id !== state.selectedId) {
      setSelected(topCard.dataset.id);
    }
  }, 120);
}, { passive: true });

/* -- Corpus load ----------------------------------------------------------- */
async function loadCorpus() {
  var response = await fetch("./data/notes-search.json", { cache: "no-store" });
  if (!response.ok)
    throw new Error(
      "Failed to load notes-search.json (" + response.status + ")",
    );
  state.corpus = await response.json();
  state.documentsById = new Map(
    state.corpus.documents.map(function (doc) {
      return [doc.id, doc];
    }),
  );
  renderStats();
  renderSuggestions();
  runSearch();

  if (state.routeDocId) {
    var opened = openReader(state.routeDocId, {
      showPage: state.page !== "about",
      syncUrl: false,
    });
    if (!opened) {
      state.routeDocId = "";
      state.routePractice = false;
    }
  }

  if (
    state.routePractice &&
    state.readerDoc &&
    typeof window.AtlasPractice !== "undefined" &&
    !window.AtlasPractice.isActive()
  ) {
    var rawContent = state.readerCache.get(state.readerDoc.id) || "";
    window.AtlasPractice.toggle(
      state.readerDoc.id,
      state.readerDoc.title,
      state.readerDoc.searchText || rawContent,
    );
  }

  syncPracticeToggleState();
  updateUrl();
}

/* -- Event wiring ---------------------------------------------------------- */
document
  .getElementById("search-form")
  .addEventListener("submit", function (event) {
    event.preventDefault();
    if (!readerColumn.classList.contains("hidden")) closeReader();
    state.query = queryInput.value;
    state.currentPage = 0;
    updateUrl();
    runSearch();
  });

var debounceTimer = null;
queryInput.addEventListener("input", function () {
  clearTimeout(debounceTimer);
  debounceTimer = window.setTimeout(function () {
    state.query = queryInput.value;
    state.currentPage = 0;
    updateUrl();
    runSearch();
  }, 90);
});

function handleChipClick(event) {
  var chip = event.target.closest("[data-chip-query], [data-chip-scope]");
  if (!chip) return;
  event.preventDefault();
  event.stopPropagation();
  activateChip(chip);
}

resultsList.addEventListener("click", handleChipClick);
previewCard.addEventListener("click", handleChipClick);
readerBody.addEventListener("click", handleChipClick);

readerBody.addEventListener("click", function (event) {
  var link = event.target.closest(".see-also-link");
  if (!link) return;
  event.preventDefault();
  var slug = link.dataset.slug;
  if (slug) {
    var doc = state.documentsById.get(slug);
    if (doc) openReader(doc.id);
  }
});

scopeButtons.forEach(function (btn) {
  btn.addEventListener("click", function () {
    showPage("search");
    if (!readerColumn.classList.contains("hidden")) closeReader();
    state.scope = btn.dataset.scope;
    state.query = "";
    state.currentPage = 0;
    queryInput.value = "";
    syncScopeButtons();
    updateUrl();
    runSearch();
  });
});

window.addEventListener("keydown", function (event) {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    queryInput.focus();
    queryInput.select();
  }
  if (event.key === "Escape" && !readerColumn.classList.contains("hidden")) {
    closeReader();
  }
});

/* -- Practice panel toggle -------------------------------------------------- */
if (practiceToggle) {
  practiceToggle.setAttribute("title", "Open practice mode");
  practiceToggle.addEventListener("click", function () {
    if (!state.readerDoc || typeof window.AtlasPractice === "undefined") return;
    var rawContent = state.readerCache.get(state.readerDoc.id) || "";
    window.AtlasPractice.toggle(
      state.readerDoc.id,
      state.readerDoc.title,
      state.readerDoc.searchText || rawContent,
    );
    syncPracticeToggleState();
    updateUrl();
  });
}

/* -- Bootstrap ------------------------------------------------------------- */
(function bootstrapFromUrl() {
  var params = new URLSearchParams(window.location.search);
  state.page = params.get("page") === "about" ? "about" : "search";
  state.query = params.get("q") || "";
  state.scope = params.get("scope") || "all";
  state.routeDocId = params.get("doc") || "";
  state.routePractice = params.get("practice") === "1";
  queryInput.value = state.query;
  syncScopeButtons();
  showPage(state.page);
})();

if (typeof window.AtlasPractice !== "undefined") {
  window.AtlasPractice.init();
}

/* -- Bento shimmer loading ------------------------------------------------- */
(function () {
  var bentoGrid = document.querySelector(".bento-grid");
  if (bentoGrid) bentoGrid.classList.add("bento-loading");
})();

/* -- Footer navigation ----------------------------------------------------- */
document.querySelectorAll(".footer-link[data-page]").forEach(function (btn) {
  btn.addEventListener("click", function () {
    showPage(btn.dataset.page);
    updateUrl();
    window.scrollTo({ top: 0, behavior: "smooth" });
  });
});

loadCorpus().catch(function (error) {
  resultsSummary.textContent = "Search corpus failed to load.";
  resultsMeta.textContent = error.message;
  renderPreview(null);
});
