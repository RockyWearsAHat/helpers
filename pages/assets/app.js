/* -- State ----------------------------------------------------------------- */
const state = {
  corpus: null,
  query: "",
  scope: "all",
  selectedId: null,
  lastResults: [],
  documentsById: new Map(),
  readerDoc: null,
  readerCache: new Map(),
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
  pageSearch.style.display = name === "search" ? "" : "none";
  pageAbout.classList.toggle("page-hidden", name !== "about");
  navLinks.forEach(function (btn) {
    btn.classList.toggle("active", btn.dataset.page === name);
  });
}

navLinks.forEach(function (btn) {
  btn.addEventListener("click", function () {
    showPage(btn.dataset.page);
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
  var pattern = new RegExp("(" + terms.map(escapeRegex).join("|") + ")", "gi");
  return escapeHtml(text).replace(pattern, "<mark>$1</mark>");
}

function buildSnippet(doc, terms) {
  var haystack = doc.searchText || doc.snippet || "";
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
  return prefix + haystack.slice(start, end).trim() + suffix;
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
    .slice(0, 10)
    .map(function (item) {
      return buildChipButton(item, "keyword-pill", { query: item });
    })
    .join("");

  var highlightPills = (
    doc.highlights && doc.highlights.length ? doc.highlights : doc.headings
  )
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
      ? '<p class="preview-section-title">Open this resource</p><div class="preview-links">' +
        resourceLinks +
        "</div>"
      : "") +
    '<p class="preview-section-title">Topics</p>' +
    '<div class="preview-keywords">' +
    (topicPills || '<span class="meta-pill">No extracted topics</span>') +
    "</div>" +
    '<p class="preview-section-title">' +
    (doc.documentType === "community" ? "Key guidance" : "Section headings") +
    "</p>" +
    '<div class="preview-headings">' +
    (highlightPills ||
      '<span class="meta-pill">No extracted highlights</span>') +
    "</div>" +
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
  var remaining = state.lastResults.length - nextStart;
  var skeletonCount = Math.min(remaining, state.pageSize, 5);
  renderSkeletonCards(skeletonCount);

  requestAnimationFrame(function () {
    requestAnimationFrame(function () {
      var skeletons = resultsList.querySelector(".skeleton-group");
      if (skeletons) skeletons.remove();
      state.currentPage += 1;
      state.isLoadingMore = false;
      renderResults(state.lastResults, 0, state.lastTerms);
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
      escapeHtml(resultDoc.path) +
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
  if (state.query) params.set("q", state.query);
  else params.delete("q");
  if (state.scope !== "all") params.set("scope", state.scope);
  else params.delete("scope");
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
    function (_match, _lang, code) {
      return "<pre><code>" + escapeHtml(code.trimEnd()) + "</code></pre>";
    },
  );

  html = html.replace(/`([^`]+)`/g, "<code>$1</code>");
  html = html.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  html = html.replace(/__([^_]+)__/g, "<strong>$1</strong>");
  html = html.replace(/(?<!\*)\*([^*]+)\*(?!\*)/g, "<em>$1</em>");
  html = html.replace(/(?<!_)_([^_]+)_(?!_)/g, "<em>$1</em>");

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
  return out.join("\n");
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
      metaBadges.map(function (b) {
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

  var topics = (item.topics || doc.topics || []).filter(Boolean);
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

function openReader(docId) {
  var doc = state.documentsById.get(docId);
  if (!doc) return;

  state.readerDoc = doc;
  resultsColumn.classList.add("hidden");
  previewColumn.classList.add("hidden");
  readerColumn.classList.remove("hidden");
  readerBody.innerHTML = '<p class="reader-loading">Loading article\u2026</p>';

  window.scrollTo({ top: 0, behavior: "smooth" });

  if (doc.communityContent) {
    var rendered = renderCommunityContent(doc);
    if (rendered) {
      state.readerCache.set(docId, rendered);
      readerBody.innerHTML = rendered;
      return;
    }
  }

  if (!doc.rawUrl) {
    readerBody.innerHTML = "<p>No article content available for this item.</p>";
    return;
  }

  if (state.readerCache.has(docId)) {
    readerBody.innerHTML = state.readerCache.get(docId);
    return;
  }

  fetch(doc.rawUrl)
    .then(function (res) {
      if (!res.ok) throw new Error("HTTP " + res.status);
      return res.text();
    })
    .then(function (md) {
      var stripped = md.replace(/^---[\s\S]*?---\s*/, "");
      var rendered = markdownToHtml(stripped);
      state.readerCache.set(docId, rendered);
      if (state.readerDoc && state.readerDoc.id === docId) {
        readerBody.innerHTML = rendered;
      }
    })
    .catch(function (err) {
      readerBody.innerHTML =
        "<p>Failed to load article: " + escapeHtml(err.message) + "</p>";
    });
}

function closeReader() {
  state.readerDoc = null;
  readerColumn.classList.add("hidden");
  resultsColumn.classList.remove("hidden");
  previewColumn.classList.remove("hidden");
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

/* -- Bootstrap ------------------------------------------------------------- */
(function bootstrapFromUrl() {
  var params = new URLSearchParams(window.location.search);
  state.query = params.get("q") || "";
  state.scope = params.get("scope") || "all";
  queryInput.value = state.query;
  syncScopeButtons();
})();

loadCorpus().catch(function (error) {
  resultsSummary.textContent = "Search corpus failed to load.";
  resultsMeta.textContent = error.message;
  renderPreview(null);
});
