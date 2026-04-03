(() => {
  const vscode = acquireVsCodeApi();

  // Restore persisted state (details open/close, scroll position)
  const savedState = vscode.getState() || {};

  let bootstrap = {};
  const bootstrapEl = document.getElementById("gsh-bootstrap");
  if (bootstrapEl?.textContent) {
    try {
      bootstrap = JSON.parse(bootstrapEl.textContent);
    } catch {
      bootstrap = {};
    }
  }

  const initialActivityItems = Array.isArray(bootstrap.initialActivityItems)
    ? bootstrap.initialActivityItems
    : [];
  const initialActivityCount =
    typeof bootstrap.initialActivityCount === "string"
      ? bootstrap.initialActivityCount
      : "idle";

  // --- Persist <details> open/close state across full re-renders ---
  function saveDetailsState() {
    const state = {};
    document.querySelectorAll("details.sect").forEach((el, i) => {
      const title = el.querySelector(".sect-title");
      const key = title ? title.textContent.trim() : "sect-" + i;
      state[key] = el.open;
    });
    vscode.setState({ ...savedState, detailsState: state });
  }

  function restoreDetailsState() {
    const ds = savedState.detailsState;
    if (!ds) return;
    document.querySelectorAll("details.sect").forEach((el) => {
      const title = el.querySelector(".sect-title");
      const key = title ? title.textContent.trim() : null;
      if (key && key in ds) {
        el.open = ds[key];
      }
    });
  }

  // Restore on page load (covers full re-renders from _update)
  restoreDetailsState();

  // Track every toggle so state survives the next _update
  document.querySelectorAll("details.sect").forEach((el) => {
    el.addEventListener("toggle", () => saveDetailsState());
  });

  function activityEscape(value) {
    return String(value || "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  }

  function humanizeToolName(name) {
    return String(name || "tool")
      .replace(/[_-]+/g, " ")
      .replace(/\b\w/g, (letter) => letter.toUpperCase());
  }

  function formatActivityDuration(seconds) {
    const totalSeconds = Number.isFinite(seconds) ? Math.max(0, seconds) : 0;
    if (totalSeconds < 60) return totalSeconds + "s";
    const minutes = Math.floor(totalSeconds / 60);
    const remainder = totalSeconds % 60;
    if (minutes < 60) {
      return remainder > 0 ? minutes + "m " + remainder + "s" : minutes + "m";
    }
    const hours = Math.floor(minutes / 60);
    const minuteRemainder = minutes % 60;
    return minuteRemainder > 0
      ? hours + "h " + minuteRemainder + "m"
      : hours + "h";
  }

  function formatActivityAgo(timestamp) {
    if (!timestamp) return "recent";
    const elapsedSeconds = Math.max(
      0,
      Math.floor((Date.now() - timestamp) / 1000),
    );
    if (elapsedSeconds < 60) return "just now";
    if (elapsedSeconds < 3600) return Math.floor(elapsedSeconds / 60) + "m ago";
    return Math.floor(elapsedSeconds / 3600) + "h ago";
  }

  function pluralizeActivity(count, singular) {
    return count + " " + singular + (count === 1 ? "" : "s");
  }

  function groupActivityItems(items) {
    const liveSessions = items
      .filter((item) => item.type === "session-active")
      .sort(
        (left, right) =>
          (right.lastChangedAt || right.startedAt || 0) -
          (left.lastChangedAt || left.startedAt || 0),
      );
    const tools = items
      .filter((item) => item.type === "tool")
      .sort((left, right) => (right.startedAt || 0) - (left.startedAt || 0));
    const recentSessions = items
      .filter((item) => item.type === "session-done")
      .sort(
        (left, right) =>
          (right.completedAt || right.startedAt || 0) -
          (left.completedAt || left.startedAt || 0),
      );
    return { liveSessions, tools, recentSessions };
  }

  function renderActivityGroup(title, items, renderer) {
    if (!items.length) return "";
    return `
      <section class="activity-group">
        <div class="activity-group-head">
          <div class="activity-group-title">${activityEscape(title)}</div>
          <div class="activity-group-count">${items.length}</div>
        </div>
        ${items.map(renderer).join("")}
      </section>`;
  }

  function renderSessionCard(item, state) {
    const isLive = state === "live";
    const requestLabel =
      item.requestCount > 0 ? pluralizeActivity(item.requestCount, "request") : "";
    const metaLabel = requestLabel
      ? `${isLive ? "Live" : "Recent"} · ${requestLabel}`
      : isLive
        ? "Live"
        : "Recent";
    const timingMarkup = isLive
      ? `<span class="activity-card-timestamp activity-elapsed" data-started="${item.startedAt}">${formatActivityDuration(item.elapsed || 0)}</span>`
      : `<span class="activity-card-timestamp">${activityEscape(formatActivityAgo(item.completedAt))}</span>`;
    const stateClass = isLive ? "activity-card--live" : "activity-card--recent";
    const dotClass = isLive
      ? "activity-state-dot--live"
      : "activity-state-dot--recent";
    const preview = item.preview
      ? `<div class="activity-preview">${activityEscape(item.preview)}</div>`
      : "";
    return `
      <button class="activity-card ${stateClass}" data-sessionid="${activityEscape(item.sessionId)}" type="button">
        <div class="activity-card-main">
          <div class="activity-card-leading">
            <span class="activity-state-dot ${dotClass}"></span>
          </div>
          <div class="activity-card-body">
            <div class="activity-card-header">
              <span class="activity-card-title">${activityEscape(item.label)}</span>
              ${timingMarkup}
            </div>
            ${preview}
            <div class="activity-card-footer">
              <span class="activity-card-meta">${activityEscape(metaLabel)}</span>
              <span class="activity-link-hint">${isLive ? "Continue" : "Open"}</span>
            </div>
          </div>
          <svg class="activity-card-chevron" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path fill-rule="evenodd" d="M6.22 4.22a.75.75 0 0 1 1.06 0l3.25 3.25a.75.75 0 0 1 0 1.06l-3.25 3.25a.75.75 0 0 1-1.06-1.06L8.94 8 6.22 5.28a.75.75 0 0 1 0-1.06z"/></svg>
        </div>
      </button>`;
  }

  function renderToolCard(item) {
    const argsText = String(item.args || "").trim();
    const hasArgs = argsText && argsText !== "{}" && argsText !== "[]";
    const header = `
      <div class="activity-headline">
        <div class="activity-title-row">
          <span class="activity-state-dot activity-state-dot--tool"></span>
          <span class="activity-card-title">${activityEscape(item.label)}</span>
        </div>
        <div class="activity-pill-row">
          <span class="activity-pill">${activityEscape(humanizeToolName(item.tool))}</span>
          <span class="activity-pill activity-pill--time activity-elapsed" data-started="${item.startedAt}">${formatActivityDuration(item.elapsed || 0)}</span>
          ${hasArgs ? '<svg class="activity-tool-chevron" viewBox="0 0 16 16" fill="currentColor"><path fill-rule="evenodd" d="M6.22 4.22a.75.75 0 0 1 1.06 0l3.25 3.25a.75.75 0 0 1 0 1.06l-3.25 3.25a.75.75 0 0 1-1.06-1.06L8.94 8 6.22 5.28a.75.75 0 0 1 0-1.06z"/></svg>' : ""}
        </div>
      </div>
      <div class="activity-meta-row">
        <span>Tool call running in the current chat.</span>
        <span>${activityEscape(item.tool || "tool")}</span>
      </div>`;
    if (!hasArgs) {
      return `
        <div class="activity-tool">
          <div class="activity-card-main">
            ${header}
          </div>
        </div>`;
    }
    return `
      <details class="activity-tool">
        <summary class="activity-tool-summary">
          ${header}
        </summary>
        <div class="activity-tool-detail"><pre>${activityEscape(argsText)}</pre></div>
      </details>`;
  }

  function renderActivityList(items, countLabel) {
    const list = document.getElementById("activityList");
    const count = document.getElementById("activityCount");
    const sect = list?.closest(".sect--activity");
    if (!list) return;
    if (count) count.textContent = countLabel || "idle";
    if (!items.length) {
      if (sect) sect.classList.add("sect--idle");
      list.classList.add("activity-list-hidden");
      list.innerHTML = "";
      return;
    }

    const groups = groupActivityItems(items);
    const markup = [
      renderActivityGroup("Live", groups.liveSessions, (item) =>
        renderSessionCard(item, "live"),
      ),
      renderActivityGroup("Tools", groups.tools, renderToolCard),
      renderActivityGroup("Recent", groups.recentSessions, (item) =>
        renderSessionCard(item, "recent"),
      ),
    ]
      .filter(Boolean)
      .join("");

    if (sect) sect.classList.remove("sect--idle");
    list.classList.remove("activity-list-hidden");
    list.innerHTML = markup;
    list.querySelectorAll(".activity-card[data-sessionid]").forEach((card) => {
      card.addEventListener("click", () => {
        vscode.postMessage({
          type: "openChatSession",
          sessionId: card.dataset.sessionid,
        });
      });
    });
  }

  renderActivityList(initialActivityItems, initialActivityCount);

  // --- Optimistic toggle helpers ---
  // Update checkbox visuals instantly without waiting for a full re-render.

  function toggleCheckbox(el) {
    const nowActive = !el.classList.contains("active");
    el.classList.toggle("active", nowActive);
    const cb = el.querySelector(".cb");
    if (cb) cb.classList.toggle("on", nowActive);
    return nowActive;
  }

  function updateSectionCount(el, totalSelector) {
    const sect = el.closest(".sect");
    if (!sect) return;
    const badge = sect.querySelector(".sect-count");
    if (!badge) return;
    const items = sect.querySelectorAll(totalSelector);
    const active = sect.querySelectorAll(totalSelector + ".active");
    badge.textContent = active.length + "/" + items.length;
  }

  // --- MCP Tool group toggles ---
  document.getElementById("loginBtn")?.addEventListener("click", () => {
    vscode.postMessage({ type: "login" });
  });

  document.querySelectorAll(".tool-item").forEach((el) => {
    if (
      el.dataset.strictLinting ||
      el.dataset.branchSessions ||
      el.dataset.sessionMemory ||
      el.dataset.formatBypass ||
      el.dataset.cpkey
    ) {
      return;
    }
    el.addEventListener("click", () => {
      const nowActive = toggleCheckbox(el);
      updateSectionCount(el, ".tool-item");
      vscode.postMessage({ type: "toggleGroup", key: el.dataset.key, enabled: nowActive });
    });
  });

  // --- Chat Tools toggles ---
  document.querySelectorAll("[data-strict-linting]").forEach((el) => {
    el.addEventListener("click", () => {
      const nowActive = toggleCheckbox(el);
      updateSectionCount(el, ".tool-item");
      vscode.postMessage({ type: "toggleStrictLinting", enabled: nowActive });
    });
  });

  document.querySelectorAll("[data-branch-sessions]").forEach((el) => {
    el.addEventListener("click", () => {
      const nowActive = toggleCheckbox(el);
      updateSectionCount(el, ".tool-item");
      vscode.postMessage({ type: "toggleBranchSessions", enabled: nowActive });
    });
  });

  document.querySelectorAll("[data-session-memory]").forEach((el) => {
    el.addEventListener("click", () => {
      const nowActive = toggleCheckbox(el);
      updateSectionCount(el, ".tool-item");
      vscode.postMessage({ type: "toggleSessionMemory", enabled: nowActive });
    });
  });

  document.querySelectorAll("[data-format-bypass]").forEach((el) => {
    el.addEventListener("click", () => {
      const nowActive = toggleCheckbox(el);
      updateSectionCount(el, ".tool-item");
      vscode.postMessage({ type: "toggleFormatBypass", enabled: nowActive });
    });
  });

  // --- Checkpoint toggles ---
  document.querySelectorAll("[data-cpkey]").forEach((el) => {
    el.addEventListener("click", () => {
      toggleCheckbox(el);
      vscode.postMessage({ type: "setCheckpoint", key: el.dataset.cpkey });
    });
  });

  document.querySelectorAll("[data-cpmodel]").forEach((el) => {
    const openPicker = () => {
      if (el.getAttribute("aria-disabled") === "true") return;
      vscode.postMessage({ type: "openCheckpointModelPicker" });
    };
    el.addEventListener("click", openPicker);
    el.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openPicker();
      }
    });
  });

  document
    .getElementById("uploadGpgBtn")
    ?.addEventListener("click", (event) => {
      event.preventDefault();
      vscode.postMessage({ type: "uploadGpgKey" });
    });

  document
    .getElementById("reloginGpgBtn")
    ?.addEventListener("click", (event) => {
      event.preventDefault();
      vscode.postMessage({ type: "reloginGpg" });
    });

  document.getElementById("manageMcpBtn")?.addEventListener("click", () => {
    const tone = document.getElementById("manageMcpBtn").dataset.tone;
    if (tone === "bad") {
      vscode.postMessage({ type: "mcpChipAction", tone: "bad" });
    } else if (tone === "warn") {
      vscode.postMessage({ type: "mcpChipAction", tone: "warn" });
    } else {
      vscode.postMessage({ type: "mcpChipAction", tone: "good" });
    }
  });

  document.querySelectorAll(".agent-item").forEach((item) => {
    item.addEventListener("click", () => {
      const name = item.dataset.agent;
      if (name) {
        vscode.postMessage({ type: "openAgent", name });
      }
    });
  });

  document.querySelectorAll(".agent-start-btn").forEach((btn) => {
    btn.addEventListener("click", (event) => {
      event.stopPropagation();
      vscode.postMessage({ type: "openAgent", name: btn.dataset.agentname });
    });
  });

  document
    .getElementById("viewMoreAgentsBtn")
    ?.addEventListener("click", () => {
      document.querySelectorAll(".agent-overflow").forEach((el) => {
        el.style.display = "flex";
      });
      document.getElementById("viewMoreAgentsBtn").style.display = "none";
    });

  document
    .getElementById("ollamaRefreshChip")
    ?.addEventListener("click", () =>
      vscode.postMessage({ type: "refreshOllama" }),
    );

  document.querySelectorAll(".ollama-tag[data-ollamatoggle]").forEach((btn) => {
    btn.addEventListener("click", () =>
      vscode.postMessage({
        type: "ollamaToggle",
        model: btn.dataset.ollamatoggle,
      }),
    );
  });

  document
    .querySelectorAll(".provider-model-run[data-ollamarun]")
    .forEach((btn) => {
      btn.addEventListener("click", (event) => {
        event.stopPropagation();
        vscode.postMessage({ type: "ollamaRun", model: btn.dataset.ollamarun });
      });
    });

  document
    .querySelectorAll(".provider-model-remove[data-ollamatoggle]")
    .forEach((btn) => {
      btn.addEventListener("click", (event) => {
        event.stopPropagation();
        vscode.postMessage({
          type: "ollamaToggle",
          model: btn.dataset.ollamatoggle,
        });
      });
    });

  document.querySelectorAll(".key-save-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      const provider = btn.dataset.savekey;
      const input = document.getElementById(provider + "KeyInput");
      const value = input ? input.value.trim() : "";
      if (!value) return;
      vscode.postMessage({ type: "saveApiKey", provider, value });
      input.value = "";
    });
  });

  document.querySelectorAll(".key-clear-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      vscode.postMessage({
        type: "saveApiKey",
        provider: btn.dataset.clearkey,
        value: "",
      });
    });
  });

  document.querySelectorAll(".key-input").forEach((input) => {
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        const provider = input.dataset.provider;
        const value = input.value.trim();
        if (!value) return;
        vscode.postMessage({ type: "saveApiKey", provider, value });
        input.value = "";
      }
    });
  });

  window.addEventListener("message", (event) => {
    const msg = event.data;
    if (msg?.type === "activityUpdate") {
      renderActivityList(msg.items || [], msg.countLabel || "idle");
    }
  });

  setInterval(() => {
    document
      .querySelectorAll(".activity-elapsed[data-started]")
      .forEach((el) => {
        const started = parseInt(el.dataset.started, 10);
        if (!Number.isNaN(started)) {
          el.textContent = formatActivityDuration(
            Math.floor((Date.now() - started) / 1000),
          );
        }
      });
  }, 1000);

  document.querySelectorAll(".provider-chip-clickable").forEach((btn) => {
    btn.addEventListener("click", () => {
      const acc = btn.dataset.acc;
      if (!acc) return;
      const panel = document.getElementById(acc + "AccPanel");
      if (!panel) return;
      const isOpen = panel.classList.toggle("open");
      btn.classList.toggle("active", isOpen);
      if (isOpen && acc !== "ollama") {
        const input = document.getElementById(acc + "KeyInput");
        setTimeout(() => input?.focus(), 60);
      }
    });
  });

  document
    .getElementById("ollamaAddModelsBtn")
    ?.addEventListener("click", () => {
      const panel = document.getElementById("ollamaAccPanel");
      const btn = document.getElementById("ollamaAddModelsBtn");
      if (!panel) return;
      const isOpen = panel.classList.toggle("open");
      if (btn) {
        btn.textContent = isOpen ? "- Close" : "+ Add model";
      }
    });

  const gearBtn = document.getElementById("gearBtn");
  const acctPanel = document.getElementById("acctPanel");
  gearBtn?.addEventListener("click", (event) => {
    event.stopPropagation();
    const open = acctPanel.classList.toggle("open");
    gearBtn.classList.toggle("active", open);
  });

  document.addEventListener("click", () => {
    acctPanel?.classList.remove("open");
    gearBtn?.classList.remove("active");
  });

  acctPanel?.addEventListener("click", (event) => event.stopPropagation());

  document.getElementById("signOutBtn")?.addEventListener("click", () => {
    vscode.postMessage({ type: "logout" });
  });

  document.getElementById("selectReposBtn")?.addEventListener("click", () => {
    vscode.postMessage({ type: "selectRepos" });
  });

  document.getElementById("modeSelect")?.addEventListener("change", (event) => {
    vscode.postMessage({ type: "setMode", value: event.target.value });
  });

  let qaContextTarget = null;
  const qaCtxMenu = document.getElementById("qaContextMenu");
  document.querySelectorAll(".qa-run-btn").forEach((btn) => {
    btn.addEventListener("click", (event) => {
      event.stopPropagation();
      vscode.postMessage({ type: "runQuickAction", action: btn.dataset.qa });
    });
  });

  document.querySelectorAll(".qa-item").forEach((item) => {
    item.addEventListener("click", () => {
      vscode.postMessage({
        type: "runQuickAction",
        action: item.dataset.qaaction,
      });
    });
    item.addEventListener("contextmenu", (event) => {
      event.preventDefault();
      qaContextTarget = item.dataset.qaaction;
      if (!qaCtxMenu) return;
      qaCtxMenu.style.display = "block";
      const menuWidth = 210;
      const menuHeight = 60;
      qaCtxMenu.style.left =
        Math.min(event.clientX, window.innerWidth - menuWidth) + "px";
      qaCtxMenu.style.top =
        Math.min(event.clientY, window.innerHeight - menuHeight) + "px";
    });
  });

  document
    .getElementById("ctxOpenWithoutSend")
    ?.addEventListener("click", (event) => {
      event.stopPropagation();
      if (qaContextTarget) {
        vscode.postMessage({
          type: "openQuickActionWithoutSend",
          action: qaContextTarget,
        });
        qaContextTarget = null;
      }
      if (qaCtxMenu) {
        qaCtxMenu.style.display = "none";
      }
    });

  document.addEventListener("click", () => {
    if (qaCtxMenu) {
      qaCtxMenu.style.display = "none";
    }
    qaContextTarget = null;
  });

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && qaCtxMenu) {
      qaCtxMenu.style.display = "none";
    }
  });
})();
