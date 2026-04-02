const state = {
  corpus: null,
  query: "",
  scope: "all",
  selectedId: null,
  lastResults: [],
  documentsById: new Map(),
};

const queryInput = document.getElementById("query-input");
const resultsList = document.getElementById("results-list");
const resultsSummary = document.getElementById("results-summary");
const resultsMeta = document.getElementById("results-meta");
const previewCard = document.getElementById("preview-card");
const suggestionStrip = document.getElementById("suggestion-strip");
const emptyState = document.getElementById("empty-state");
const luckyButton = document.getElementById("lucky-button");

const scopeButtons = Array.from(document.querySelectorAll("[data-scope]"));

function escapeHtml(text) {
  return String(text)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/\"/g, "&quot;")
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
    .filter((term) => term.length >= 2);
}

function countOccurrences(text, term) {
  if (!term) return 0;
  const matches = text.match(new RegExp(escapeRegex(term), "g"));
  return matches ? matches.length : 0;
}

function highlight(text, terms) {
  if (!terms.length) return escapeHtml(text);
  const pattern = new RegExp(`(${terms.map(escapeRegex).join("|")})`, "gi");
  return escapeHtml(text).replace(pattern, "<mark>$1</mark>");
}

function buildSnippet(document, terms) {
  const haystack = document.searchText || document.snippet || "";
  const lowerHaystack = haystack.toLowerCase();
  let start = 0;

  for (const term of terms) {
    const index = lowerHaystack.indexOf(term);
    if (index !== -1) {
      start = Math.max(0, index - 90);
      break;
    }
  }

  const end = Math.min(haystack.length, start + 260);
  const prefix = start > 0 ? "..." : "";
  const suffix = end < haystack.length ? "..." : "";
  return `${prefix}${haystack.slice(start, end).trim()}${suffix}`;
}

function scoreDocument(document, normalizedQuery, terms) {
  if (state.scope !== "all" && document.scopeKey !== state.scope) {
    return 0;
  }

  if (!normalizedQuery) {
    if (document.scopeKey === "copilot") return 16;
    if (document.scopeKey === "knowledge") return 14;
    return 5;
  }

  const title = document.title.toLowerCase();
  const path = document.path.toLowerCase();
  const headings = document.headings.map((heading) => heading.toLowerCase());
  const keywords = document.keywords.map((keyword) => keyword.toLowerCase());
  const topics = (document.topics || []).map((topic) => topic.toLowerCase());
  const metaPills = (document.metaPills || []).join(" ").toLowerCase();
  const searchText = document.searchText.toLowerCase();

  let score =
    document.scopeKey === "knowledge"
      ? 8
      : document.scopeKey === "copilot"
        ? 12
        : 0;
  let matchedTerms = 0;

  if (normalizedQuery && title.includes(normalizedQuery)) {
    score += 180;
  } else if (normalizedQuery && searchText.includes(normalizedQuery)) {
    score += 80;
  }

  for (const term of terms) {
    let termScore = 0;
    if (title.includes(term)) termScore += 110;
    if (headings.some((heading) => heading.includes(term))) termScore += 50;
    if (keywords.includes(term)) termScore += 36;
    if (topics.some((topic) => topic.includes(term))) termScore += 32;
    if (metaPills.includes(term)) termScore += 18;
    if (path.includes(term)) termScore += 22;

    const occurrenceCount = countOccurrences(searchText, term);
    termScore += Math.min(occurrenceCount, 6) * 11;

    if (termScore > 0) matchedTerms += 1;
    score += termScore;
  }

  if (!matchedTerms) return 0;
  if (matchedTerms === terms.length) {
    score *= 1.22;
  } else {
    score *= 0.58 + matchedTerms / terms.length;
  }

  return score;
}

function sortResults(results) {
  return results.sort((left, right) => {
    if (right.score !== left.score) return right.score - left.score;
    return left.document.title.localeCompare(right.document.title);
  });
}

function renderSuggestions() {
  suggestionStrip.innerHTML = "";
  if (!state.corpus) return;

  for (const category of state.corpus.metadata.featuredCategories) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "suggestion-button";
    button.textContent = `${category.label} (${category.count})`;
    button.addEventListener("click", () => {
      queryInput.value = category.label;
      state.query = category.label;
      updateUrl();
      runSearch();
      queryInput.focus();
    });
    suggestionStrip.appendChild(button);
  }
}

function renderStats() {
  if (!state.corpus) return;
  document.getElementById("doc-count").textContent =
    state.corpus.metadata.totalDocuments;
  document.getElementById("copilot-count").textContent =
    state.corpus.metadata.scopeCounts.copilot || 0;
  document.getElementById("curated-count").textContent =
    state.corpus.metadata.scopeCounts.knowledge || 0;
  document.getElementById("archive-count").textContent =
    state.corpus.metadata.scopeCounts.archive || 0;

  const builtAt = new Date(state.corpus.metadata.builtAt);
  document.getElementById("built-at").textContent = builtAt.toLocaleDateString(
    undefined,
    {
      month: "short",
      day: "numeric",
      year: "numeric",
    },
  );
}

function renderPreview(document) {
  if (!document) {
    previewCard.innerHTML = `
      <p class="preview-kicker">Result preview</p>
      <h2>Pick a result to inspect the guide before leaving the page.</h2>
      <p class="preview-body">
        The search index blends Copilot-specific guidance with broader CS notes,
        then ranks titles, topics, headings, and intro text together.
      </p>
    `;
    return;
  }

  const topicPills = (
    document.topics?.length ? document.topics : document.keywords
  )
    .slice(0, 10)
    .map((item) => `<span class="keyword-pill">${escapeHtml(item)}</span>`)
    .join("");
  const highlightPills = (
    document.highlights?.length ? document.highlights : document.headings
  )
    .slice(0, 6)
    .map((item) => `<span class="meta-pill">${escapeHtml(item)}</span>`)
    .join("");
  const resourceLinks = (document.resourceLinks || [])
    .map(
      (link) =>
        `<a class="preview-link" href="${link.url}" target="_blank" rel="noreferrer">${escapeHtml(link.label)}</a>`,
    )
    .join("");
  const relatedDocuments = (document.relatedIds || [])
    .map((id) => state.documentsById.get(id))
    .filter(Boolean);
  const relatedButtons = relatedDocuments
    .map(
      (related) =>
        `<button class="related-trigger" type="button" data-related-id="${escapeHtml(related.id)}">${escapeHtml(related.title)}</button>`,
    )
    .join("");
  const metaPills = (
    document.metaPills || [document.scopeLabel, document.category]
  )
    .map((pill) => `<span class="meta-pill">${escapeHtml(pill)}</span>`)
    .join("");

  previewCard.innerHTML = `
    <p class="preview-kicker">${escapeHtml(document.scopeLabel)}</p>
    <h2>${escapeHtml(document.title)}</h2>
    <div class="preview-meta">
      ${metaPills}
      <span class="meta-pill">${escapeHtml(document.path)}</span>
    </div>
    <p class="preview-body">${escapeHtml(document.previewText || document.snippet)}</p>
    ${resourceLinks ? `<p class="preview-section-title">Open this resource</p><div class="preview-links">${resourceLinks}</div>` : ""}
    <p class="preview-section-title">Topics</p>
    <div class="preview-keywords">${topicPills || '<span class="meta-pill">No extracted topics</span>'}</div>
    <p class="preview-section-title">${document.documentType === "community" ? "Key guidance" : "Section headings"}</p>
    <div class="preview-headings">${highlightPills || '<span class="meta-pill">No extracted highlights</span>'}</div>
    ${relatedButtons ? `<p class="preview-section-title">Related next steps</p><div class="preview-related">${relatedButtons}</div>` : ""}
  `;

  for (const button of previewCard.querySelectorAll("[data-related-id]")) {
    button.addEventListener("click", () => {
      setSelected(button.dataset.relatedId);
    });
  }
}

function setSelected(documentId) {
  state.selectedId = documentId;
  const selected =
    state.lastResults.find((entry) => entry.document.id === documentId)
      ?.document ||
    state.documentsById.get(documentId) ||
    null;
  renderPreview(selected);

  for (const card of resultsList.querySelectorAll(".result-card")) {
    card.classList.toggle("active", card.dataset.id === documentId);
  }
}

function renderResults(results, durationMs, terms) {
  resultsList.innerHTML = "";
  emptyState.hidden = results.length > 0 || Boolean(state.query);

  if (!state.query) {
    resultsSummary.textContent =
      "Search titles, topics, headings, and guide intros across the public corpus.";
    resultsMeta.textContent = "";
    renderPreview(null);
    return;
  }

  resultsSummary.textContent = `About ${results.length.toLocaleString()} result${results.length === 1 ? "" : "s"}`;
  resultsMeta.textContent = `${durationMs.toFixed(1)} ms`;

  if (!results.length) {
    renderPreview(null);
    return;
  }

  for (const entry of results) {
    const { document, score } = entry;
    const listItem = document.createElement("li");
    const snippet = buildSnippet(document, terms);
    listItem.innerHTML = `
      <article class="result-card" data-id="${escapeHtml(document.id)}" tabindex="0">
        <div class="result-topline">
          <div>
            <p class="result-path">${escapeHtml(document.path)}</p>
            <h2 class="result-title">
              <a class="result-link" href="${document.githubUrl}" target="_blank" rel="noreferrer">${highlight(document.title, terms)}</a>
            </h2>
          </div>
          <span class="result-pill">Score ${Math.round(score)}</span>
        </div>
        <p class="result-snippet">${highlight(snippet, terms)}</p>
        <div class="result-pills">
          ${(document.resultPills || [document.scopeLabel, document.category])
            .slice(0, 4)
            .map(
              (keyword) =>
                `<span class="result-pill">${escapeHtml(keyword)}</span>`,
            )
            .join("")}
        </div>
      </article>
    `;

    const card = listItem.firstElementChild;
    const activate = () => setSelected(document.id);
    card.addEventListener("mouseenter", activate);
    card.addEventListener("focus", activate);
    card.addEventListener("click", (event) => {
      if (event.target.closest("a")) return;
      activate();
    });
    resultsList.appendChild(listItem);
  }

  setSelected(results[0].document.id);
}

function updateUrl() {
  const params = new URLSearchParams(window.location.search);
  if (state.query) params.set("q", state.query);
  else params.delete("q");

  if (state.scope !== "all") params.set("scope", state.scope);
  else params.delete("scope");

  const nextUrl = `${window.location.pathname}${params.toString() ? `?${params}` : ""}`;
  window.history.replaceState({}, "", nextUrl);
}

function syncScopeButtons() {
  for (const button of scopeButtons) {
    button.classList.toggle("active", button.dataset.scope === state.scope);
  }
}

function runSearch() {
  if (!state.corpus) return;

  const normalizedQuery = normalizeWhitespace(state.query).toLowerCase();
  const terms = tokenize(normalizedQuery);
  const startedAt = performance.now();

  const results = sortResults(
    state.corpus.documents
      .map((document) => ({
        document,
        score: scoreDocument(document, normalizedQuery, terms),
      }))
      .filter((entry) => entry.score > 0),
  ).slice(0, 60);

  state.lastResults = results;
  renderResults(results, performance.now() - startedAt, terms);
}

async function loadCorpus() {
  const response = await fetch("./data/notes-search.json", {
    cache: "no-store",
  });
  if (!response.ok) {
    throw new Error(`Failed to load notes-search.json (${response.status})`);
  }
  state.corpus = await response.json();
  state.documentsById = new Map(
    state.corpus.documents.map((document) => [document.id, document]),
  );
  renderStats();
  renderSuggestions();
  runSearch();
}

document.getElementById("search-form").addEventListener("submit", (event) => {
  event.preventDefault();
  state.query = queryInput.value;
  updateUrl();
  runSearch();
});

let debounceTimer = null;
queryInput.addEventListener("input", () => {
  clearTimeout(debounceTimer);
  debounceTimer = window.setTimeout(() => {
    state.query = queryInput.value;
    updateUrl();
    runSearch();
  }, 90);
});

luckyButton.addEventListener("click", () => {
  const topResult = state.lastResults[0];
  if (!topResult) return;
  window.open(topResult.document.githubUrl, "_blank", "noopener,noreferrer");
});

for (const button of scopeButtons) {
  button.addEventListener("click", () => {
    state.scope = button.dataset.scope;
    syncScopeButtons();
    updateUrl();
    runSearch();
  });
}

window.addEventListener("keydown", (event) => {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    queryInput.focus();
    queryInput.select();
  }
});

(function bootstrapFromUrl() {
  const params = new URLSearchParams(window.location.search);
  state.query = params.get("q") || "";
  state.scope = params.get("scope") || "all";
  queryInput.value = state.query;
  syncScopeButtons();
})();

loadCorpus().catch((error) => {
  resultsSummary.textContent = "Search corpus failed to load.";
  resultsMeta.textContent = error.message;
  renderPreview(null);
});
