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
    getWebviewProvider()?.pushUpdate(_buildActivityUpdate());
    return id;
  }

  function endToolCall(id) {
    activeToolCalls.delete(id);
    getWebviewProvider()?.pushUpdate(_buildActivityUpdate());
  }

  function getActivityItems() {
    const now = Date.now();
    const toolItems = [];
    for (const c of activeToolCalls.values()) {
      toolItems.push({
        id: c.id,
        type: "tool",
        tool: c.tool,
        label: c.label,
        elapsed: Math.floor((now - c.startedAt) / 1000),
        startedAt: c.startedAt,
        args: JSON.stringify(c.args, null, 2),
      });
    }
    toolItems.sort((left, right) => right.startedAt - left.startedAt);

    const activeSessions = [];
    const recentSessions = [];
    const chatSessions = getChatSessions();
    for (const [sessionId, sess] of chatSessions) {
      if (sess.active) {
        activeSessions.push({
          id: `chat-${sess.sessionId}`,
          type: "session-active",
          label: sess.title,
          elapsed: Math.floor((now - (sess.activeAt || sess.startedAt)) / 1000),
          startedAt: sess.activeAt || sess.startedAt,
          preview: sess.preview || "Working\u2026",
          sessionId: sess.sessionId,
          requestCount: sess.requestCount || 0,
          lastChangedAt: sess._lastChangedAt || sess.activeAt || sess.startedAt,
        });
      } else {
        recentSessions.push({
          id: `chat-${sess.sessionId}`,
          type: "session-done",
          label: sess.title,
          preview: sess.preview || "",
          sessionId: sess.sessionId,
          requestCount: sess.requestCount || 0,
          completedAt: sess.completedAt || sess._lastChangedAt || sess.startedAt,
        });
      }
    }
    activeSessions.sort(
      (left, right) =>
        (right.lastChangedAt || right.startedAt) -
        (left.lastChangedAt || left.startedAt),
    );
    recentSessions.sort(
      (left, right) =>
        (right.completedAt || right.startedAt) -
        (left.completedAt || left.startedAt),
    );
    return [
      ...activeSessions,
      ...toolItems,
      ...recentSessions.slice(0, 3),
    ];
  }

  function _buildActivityUpdate() {
    const items = getActivityItems();
    return {
      type: "activityUpdate",
      items,
      countLabel: _activityCountLabel(items),
    };
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
    if (active.length > 0) {
      return `${active.length} live`;
    }
    return `${items.length} recent`;
  }

  function pushActivityUpdate() {
    getWebviewProvider()?.pushUpdate(_buildActivityUpdate());
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
    pushActivityUpdate,
    getSessionStartedAt,
    ensureSessionStarted,
  };
};
