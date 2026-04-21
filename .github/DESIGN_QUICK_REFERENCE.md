# Design System Quick Reference

## Which System Am I Working In?

| Context | System | File Location | Accent Colors |
|---------|--------|-------------------|-----------|
| Knowledge search, web pages, public interfaces | **Pages/Atlas** | `/pages/index.html`, `/pages/assets/` | Cool blues (#6fc6ff, #60b2ff, #4de0be) |
| VS Code settings, extension panels, sidebar | **VS Code Extension** | `vscode-extension/`, `vscode-extension/media/` | Native VS Code theme colors |

---

## Pages/Atlas Quick Token Reference

### Colors (Dark Theme)
```
Backgrounds:     #060a14 (deepest)   →   #0a1122   →   #0e1730 (lightest)
Text:            #eef7ff (brightest) →   #c8def3   →   #8fb2cf (tertiary)
Accents:         #6fc6ff (light blue), #60b2ff (blue), #4de0be (cyan)
Borders:         rgba(123, 188, 255, 0.22) 
```

### Fonts
```
Display: "Saira"
Body:    "DM Sans"  
Mono:    "JetBrains Mono"
```

### Layout
```
Border Radius:   18px (xl)  →  12px (lg)  →  9px (md)
Shadows:         0 18px 44px (heavy), 0 10px 24px (medium)
Easing:          cubic-bezier(0.16, 1, 0.3, 1) [ease-out], cubic-bezier(0.4, 0, 0.2, 1) [smooth]
```

---

## VS Code Extension Quick Reference

### Core Rule
**Use VS Code's native color variables. Do NOT custom-style unless essential.**

```css
--vscode-foreground              /* Text */
--vscode-button-background       /* Primary button */
--vscode-button-hoverBackground  /* Button hover */
--vscode-list-hoverBackground    /* List item hover */
--vscode-panel-border            /* Dividers */
--vscode-checkbox-background     /* Checkbox fill */
--vscode-badge-background        /* Badge background */
--vscode-descriptionForeground   /* Secondary text */
```

### Component Patterns
- **Toggles:** Use checkbox component (14px × 14px, with tick mark)
- **Sections:** Title (11px uppercase, 0.65 opacity) + items (grouped below)
- **Items:** Flex row, 5px vertical padding, 9px gap, hover background
- **Spacing:** 10px (section padding), 9px (item gap), 14px (borders)

---

## Self-Critique Checklist (Before Finalizing)

- [ ] Does it avoid generic Tailwind defaults? (Pages/Atlas only)
- [ ] Is every color token used (not hardcoded)? 
- [ ] Does text hierarchy feel intentional (not all equally weighted)?
- [ ] Is spacing consistent with the system's rhythm?
- [ ] Do all interactive states exist (hover, active, disabled, focus)?
- [ ] Does it feel refined, not rough?
- [ ] Does it fit the aesthetic (Paper-cut for Pages, Native for Extension)?
- [ ] Is it production-ready (no TODO, no hardcoded values)?

If ANY checkbox fails → **refine before shipping.**

---

## Common Mistakes to Avoid

| ❌ Don't | ✓ Do Instead |
|---------|-------------|
| Hardcode colors | Use `var(--token-name)` |
| Random padding values | Use the spacing rhythm (9, 12, 14, 18px) |
| Skip hover/active states | Define all interactive states |
| Copy Tailwind defaults | Create distinctive designs within the system |
| Use generic fonts (Inter, Roboto) | Use Saira (display), DM Sans (body), JetBrains (mono) |
| Harsh shadows | Use soft shadows with refined opacity |
| Mix design systems | Pick Pages/Atlas OR VS Code Extension, not both |
| Leave accessibility out | Keyboard nav, focus states, contrast ratios (4.5:1+) |

---

## How to Request Design Changes

1. **Identify the System:** Pages/Atlas or VS Code Extension?
2. **Show the Problem:** "This button looks generic" / "Text is hard to read"
3. **Reference Context:** "Link this to existing component X" or "Create new pattern"
4. **Accept the Critique:** Designer authority makes final call

---

## Design System Version

**Pages/Atlas:** v1.0 (Cool blues, paper-cut aesthetic, dark-first)  
**VS Code Extension:** v1.0 (Theme-adaptive, native VS Code)  
**Design Authority Active:** April 17, 2026

---

*Print this. Post it. Reference it before every design decision.*
