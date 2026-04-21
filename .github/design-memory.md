# Design Memory — Visual Decisions & Patterns

**Updated:** April 17, 2026  
**Maintained By:** UI Design Engineer Agent

This file tracks significant visual decisions, new patterns, and design evolution across sessions.

---

## Current Design System Status

### Pages/Atlas System (v1.0)
- **Status:** Established, cohesive
- **Last Audit:** April 17, 2026
- **Primary Files:** `/pages/assets/styles.css`, `/pages/index.html`
- **Key Characteristics:** Dark-first, cool blues, neural elements, paper-cut shapes
- **Primary Components:** 
  - Top navigation bar (sticky)
  - Hero section with asymmetrical paper-cut SVGs
  - Filter/category chips
  - Search interface
  - Knowledge result cards
  - Stat displays
  - Neural canvas background

### VS Code Extension System (v1.0)
- **Status:** Established, theme-adaptive
- **Last Audit:** April 17, 2026
- **Primary Files:** `vscode-extension/media/community-cache.css`
- **Key Characteristics:** Native VS Code, theme-respecting, minimal custom styling
- **Primary Components:**
  - Tool toggle items (checkbox-based)
  - Settings sections
  - Authentication gate
  - Model picker dropdown
  - Badge count displays
  - Activity feed

---

## Component Patterns Established

### Card/Panel Layouts
- Used in: Knowledge results (Pages/Atlas)
- Pattern: Soft shadow, panel background color, rounded corners
- Reusable: YES — extract if used 3+ times

### Toggle/Checkbox Components
- Used in: Extension settings (VS Code Extension)
- Pattern: 14px × 14px, with tick mark, hover background
- Reusable: YES — already established

### Filter Chips
- Used in: Pages/Atlas category filters
- Pattern: Pill-shaped, border only, darker on active
- Reusable: YES — consider extracting if expanded

### Typography Hierarchy
- **Largest:** Saira 2.5-3rem (display headlines)
- **Large:** Saira 1.5-2rem (section titles)
- **Normal:** DM Sans 1rem (body text)
- **Small:** DM Sans 0.875rem (secondary text)
- **Tiny:** DM Sans 0.75rem (labels, metadata)
- **Code:** JetBrains Mono 0.875rem
- **Status:** Consistent across both systems ✓

---

## Color Usage Patterns

### Pages/Atlas Dark Theme
| Role | Token | Value | Used For |
|------|-------|-------|----------|
| Background | `--bg-0` | #060a14 | Page background, deepest surface |
| Surface | `--bg-1` | #0a1122 | Primary panels, cards |
| Light Surface | `--bg-2` | #0e1730 | Hover states, alternate panels |
| Primary Text | `--ink-1` | #c8def3 | Body text, primary content |
| Secondary Text | `--ink-2` | #8fb2cf | Secondary labels, disabled text |
| Primary Accent | `--accent-0` | #6fc6ff | Links, highlights |
| Strong Accent | `--accent-1` | #60b2ff | Active states, emphasis |
| Secondary Accent | `--accent-2` | #4de0be | Complementary elements |

### VS Code Extension
- **Uses:** Native `--vscode-*` variables
- **Custom Override:** Only when necessary (minimal)
- **Light/Dark:** Automatic theme detection
- **High Contrast:** Supported via native variables

---

## Decision Log

### Apr 17, 2026 — Design Authority Established
- **Decision:** Create UI Design Engineer Agent with full authority
- **Rationale:** Prevent design debt, enforce consistency, ensure production quality
- **Impact:** All future UI work subject to design audit before finalization
- **Status:** ACTIVATED

### Apr 17, 2026 — Design System Dual Model Clarified
- **Decision:** Document two separate design systems (Pages/Atlas + VS Code Extension)
- **Rationale:** Different contexts require different aesthetics (web vs. native extension)
- **Decision:** Pages/Atlas uses distinctive custom colors; Extension respects VS Code themes
- **Impact:** New components must choose which system they belong to
- **Status:** DOCUMENTED

---

## Accessibility Audit Status

### Pages/Atlas
- [ ] Color contrast ratios (WCAG AA minimum 4.5:1) — PENDING
- [ ] Keyboard navigation on all interactive elements — PENDING
- [ ] Focus indicators visible and intentional — PENDING
- [ ] Screen reader compatibility (heading structure, labels) — PENDING
- [ ] Light theme variant suitable for light backgrounds — PENDING

### VS Code Extension
- [ ] Native keyboard navigation works — ASSUMED ✓ (uses native components)
- [ ] Focus indicators present — ASSUMED ✓ (native styling)
- [ ] Light/Dark/High Contrast themes work — ASSUMED ✓ (native variables)
- [ ] ARIA labels where needed — REVIEW NEEDED

---

## Upcoming Refinements

### High Priority
1. Accessibility audit for Pages/Atlas (contrast, keyboard, focus)
2. Light theme contrast review
3. Component state documentation (hover, active, disabled, loading)

### Medium Priority
1. Neural canvas performance optimization
2. Extended component pattern library
3. Responsive design testing (mobile breakpoints)

### Low Priority
1. Visual captures/screenshot library
2. Component showcase page
3. Micro-animation documentation

---

## New Patterns to Extract (When 3+ Uses Detected)

- Filter chip components
- Card/panel layouts
- Loading states
- Error/warning displays
- Toast notifications
- Modal dialogs
- Form inputs (text, select, multi-select)

---

## Design System Files Reference

| File | Purpose | System |
|------|---------|--------|
| `~/.copilot/instructions/ui-design-authority.instructions.md` | Agent activation & authority framework | Both |
| `.github/design-audit-2026-04-17.md` | Comprehensive audit report | Both |
| `.github/DESIGN_QUICK_REFERENCE.md` | One-page token reference | Both |
| `pages/assets/styles.css` | Pages/Atlas stylesheet | Pages/Atlas |
| `vscode-extension/media/community-cache.css` | Extension stylesheet | VS Code Extension |

---

## Notes for Future Agent Sessions

- Design authority is **non-negotiable** — you have veto power over mediocre designs
- Always reference the audit report when building new components
- Check the quick reference before making token decisions
- Maintain this memory file as patterns emerge
- Document new component patterns when they appear 3+ times
- Escalate accessibility concerns immediately (don't defer)
- Update design system version when system-wide changes occur

---

*This file evolves as the codebase grows. Update it whenever significant design patterns are established, refined, or discovered.*
