"use strict";
// src/activity-tracker.js — Tool call tracking and activity rendering

module.exports = function createActivityTracker(deps) {
  const { getWebviewProvider, getChatSessions } = deps;

  let _activitySeq = 0;
  let _sessionStartedAt = 0;
  const activeToolCalls = new Map(); // id → { id, tool, label, startedAt, args }

  function beginToolCall(tool, label, args) {
    if (!_sessionStartedAt) {
      _sessionStartedAt = Date.now();
    }
    const id = `tc-${++_activitySeq}`;
    activeToolCalls.set(id, {
      id,
      tool,
      label,
      startedAt: Date.now(),
      args: args || {},
    });
    getWebviewProvider()?.pushUpdate({
      type: "activityUpdate",
      items: getActivityItems(),
    });
    return id;
  }

  function endToolCall(id) {
    activeToolCalls.delete(id);
    getWebviewProvider()?.pushUpdate({
      type: "activityUpdate",
      items: getActivityItems(),
    });
  }

  function getActivityItems() {
    const now = Date.now();
    const items = [];
    for (const c of activeToolCalls.values()) {
      items.push({
        id: c.id,
        type: "tool",
        label: c.label,
        elapsed: Math.floor((now - c.startedAt) / 1000),
        startedAt: c.startedAt,
        args: JSON.stringify(c.args, null, 2),
      });
    }
    const allSessions = [];
    const chatSessions = getChatSessions();
    for (const [sessionId, sess] of chatSessions) {
      const recency = sess.active
        ? sess.startedAt
        : sess.completedAt || sess.startedAt;
      allSessions.push({ sessionId, recency, ...sess });
    }
    allSessions.sort((a, b) => b.recency - a.recency);
    const top3 = allSessions.slice(0, 3);
    for (const sess of top3) {
      if (sess.active) {
        items.push({
          id: `chat-${sess.sessionId}`,
          type: "session-active",
          label: sess.title,
          elapsed: Math.floor((now - (sess.activeAt || sess.startedAt)) / 1000),
          startedAt: sess.activeAt || sess.startedAt,
          preview: sess.preview || "Working\u2026",
          sessionId: sess.sessionId,
        });
      } else {
        items.push({
          id: `chat-${sess.sessionId}`,
          type: "session-done",
          label: sess.title,
          preview: sess.preview || "",
          sessionId: sess.sessionId,
        });
      }
    }
    return items;
  }

  function _formatDuration(ms) {
    const s = Math.round(ms / 1000);
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60);
    const rem = s % 60;
    return rem > 0 ? `${m} min ${rem}s` : `${m} min`;
  }

  function _formatAgo(ms) {
    const s = Math.round(ms / 1000);
    if (s < 60) return "just now";
    const m = Math.floor(s / 60);
    if (m < 60) return `${m} min ago`;
    const h = Math.floor(m / 60);
    return `${h} hr ago`;
  }

  function _activityCountLabel(items) {
    const active = items.filter(
      (i) => i.type === "session-active" || i.type === "tool",
    );
    if (items.length === 0) return "idle";
    if (active.length === 0) return `${items.length} recent`;
    return `${active.length} running`;
  }

  function _renderActivityItem(item, esc) {
    if (item.type === "session-active") {
      return `
      <div class="activity-item activity-item--session" data-sessionid="${item.sessionId}">
        <div class="activity-row">
          <span class="activity-spinner"></span>
          <span class="activity-label">${esc(item.label)}</span>
          <span class="activity-elapsed" data-started="${item.startedAt}">${item.elapsed}s</span>
        </div>
        ${item.preview ? `<div class="activity-sub">${esc(item.preview)}</div>` : ""}
      </div>`;
    }
    if (item.type === "session-done") {
      return `
      <div class="activity-item activity-item--done" data-sessionid="${item.sessionId}">
        <div class="activity-row">
          <span class="activity-dot-done"></span>
          <span class="activity-label">${esc(item.label)}</span>
          <span class="activity-meta">completed</span>
        </div>
        ${item.preview ? `<div class="activity-sub">${esc(item.preview)}</div>` : ""}
      </div>`;
    }
    return `
    <details class="activity-item">
      <summary class="activity-summary">
        <span class="activity-pulse"></span>
        <span class="activity-label">${esc(item.label)}</span>
        <span class="activity-elapsed" data-started="${item.startedAt}">${item.elapsed}s</span>
        <svg class="activity-chevron" viewBox="0 0 16 16" fill="currentColor"><path fill-rule="evenodd" d="M6.22 4.22a.75.75 0 0 1 1.06 0l3.25 3.25a.75.75 0 0 1 0 1.06l-3.25 3.25a.75.75 0 0 1-1.06-1.06L8.94 8 6.22 5.28a.75.75 0 0 1 0-1.06z"/></svg>
      </summary>
      <div class="activity-detail"><pre>${esc(item.args)}</pre></div>
    </details>`;
  }

  function pushActivityUpdate() {
    getWebviewProvider()?.pushUpdate({
      type: "activityUpdate",
      items: getActivityItems(),
    });
  }

  function getSessionStartedAt() {
    return _sessionStartedAt;
  }

  function ensureSessionStarted() {
    if (!_sessionStartedAt) _sessionStartedAt = Date.now();
  }

  return {
    activeToolCalls,
    beginToolCall,
    endToolCall,
    getActivityItems,
    _formatDuration,
    _formatAgo,
    _activityCountLabel,
    _renderActivityItem,
    pushActivityUpdate,
    getSessionStartedAt,
    ensureSessionStarted,
  };
};
