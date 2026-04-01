# Accessibility Testing

## Concept

Accessibility testing validates that applications are usable by all people, including those with disabilities. Testing covers visual, auditory, motor, and cognitive disabilities across multiple accessibility dimensions.

WCAG (Web Content Accessibility Guidelines) defines three conformance levels (A, AA, AAA), with AA being the common standard for production applications.

## Automated vs. Manual Testing

### Automated Accessibility Testing
Tools scan for violations of accessibility rules that have deterministic checks. ~30-40% of accessibility issues can be automated.

**What automated tools catch:**
- Missing alt text on images
- Color contrast ratios below thresholds
- Heading hierarchy violations
- Form inputs without labels
- Missing ARIA attributes
- Keyboard trap detection
- Missing language declarations

**Tools:** axe-core, Lighthouse, Pa11y, WAVE, Accessibility Insights.

**Limitation:** Cannot understand semantic meaning. A decorative heading tagged as H1 is technically "correct" but semantically wrong. Humans must verify intent.

### Manual Testing
Humans validate semantic correctness, user workflows, and accessibility that requires judgment.

**What manual testing catches:**
- Semantic accuracy of heading structure
- Meaningfulness of alt text (is it descriptive, not just "image123.jpg"?)
- Focus management in complex interactions
- Screen reader announcements matching expectations
- Usable form workflows
- Adequate target sizes for touch
- Meaningful error messages

**Limitation:** Slow, expensive, error-prone if done ad-hoc. Must be systematic and comprehensive.

## Automated Accessibility Tools

### axe-core (Deque Systems)
Most widely adopted automated tool. Integrated into browsers, CI frameworks, and E2E testing libraries.

**Capabilities:**
- Core WCAG rule set with ~100 rules
- Browser integrations (Chrome, Firefox extensions)
- API access for programmatic scanning
- Integrates with Playwright, Cypress, etc.
- AI-powered review (filtering false positives)

**Approach:** Rules-based scanning against WCAG success criteria.

### Lighthouse (Google)
Primarily a performance auditing tool; includes accessibility checks (~20 rules). Built into Chrome DevTools.

**Advantages:** No setup, immediately available in browser. Good for quick audits.

**Limitations:** Smaller rule set than axe. Less comprehensive for accessibility-focused testing.

### Pa11y
Open-source command-line tool and CI-friendly scanner. npm-based, runs headless.

**Advantage:** Fits CI/CD pipelines naturally. Good for automated gates (build fails if accessibility violations found).

### WAVE (WebAIM)
Browser extension and online tool. Simpler rule set, good for accessibility beginners.

### Accessibility Insights (Microsoft)
Similar to axe-core, designed for manual + automated combination. Guides manual testing workflows.

## WCAG Conformance Levels

**Level A:** Basic accessibility. Lowest bar (e.g., text alternatives for images).

**Level AA:** Increased accessibility. Common standard for web applications (e.g., color contrast 4.5:1 for normal text).

**Level AAA:** Enhanced accessibility. Rarely required except for specialized applications (e.g., color contrast 7:1).

**Complexity:** Higher levels are harder to achieve without specialized expertise. Most organizations target AA.

## Screen Reader Testing

Screen readers convey visual information through audio and/or braille. Different screen readers handle content differently.

### Common Screen Readers
- **NVDA (Windows):** Free, open-source, most common for Windows users
- **JAWS (Windows):** Premium, most feature-rich, enterprise standard
- **VoiceOver (macOS/iOS):** System built-in, excellent for Apple ecosystem
- **TalkBack (Android):** System built-in for Android

**Usage:** Testing workflows require reading and navigating apps through audio, which is slow and requires practice. Testers must understand how screen reader users navigate (keyboard shortcuts, reading order, announcements).

### Manual Screen Reader Testing
1. Enable screen reader
2. Close eyes or look away from screen (can't rely on visual layout)
3. Navigate via keyboard only
4. Listen to announcements and verify:
   - Content is announced in logical order
   - Page structure is clear (headings, landmarks)
   - Interactive elements are discoverable and label
   - Form fields are clearly associated with labels
   - Errors are announced with correction guidance
   - Focus management is predictable

**Challenge:** Slow feedback loop. Testing a single workflow takes minutes instead of seconds.

## Keyboard Navigation Testing

Many users cannot use mice (motor disabilities, screen reader users). Verify:

- **Tab order:** Does tabbing through interactive elements follow logical order?
- **Focus visibility:** Is focus always visible (not invisible outline)?
- **Keyboard access:** Can all interactive elements (buttons, links, form inputs) be accessed via keyboard?
- **Keyboard traps:** Can users tab away from elements, or does focus get stuck?
- **Shortcuts:** Do keyboard shortcuts avoid conflicts with assistive tech shortcuts?

**Testing:** Use only keyboard (disable mouse). Tab through the page. Verify all functionality is reachable.

## Color Contrast Testing

WCAG defines contrast ratio requirements to ensure text is readable for low-vision users and in high-ambient-light environments.

**Standards:**
- **Normal text:** 4.5:1 (AA) or 7:1 (AAA)
- **Large text:** 3:1 (AA) or 4.5:1 (AAA)

**Testing:** Automated tools calculate contrast using the WCAG algorithm. Some tools provide visual simulators (simulating color blindness, low vision).

**Note:** Color alone cannot convey meaning. Use color + text labels or icons.

## ARIA (Accessible Rich Internet Applications)

ARIA attributes enhance semantics when native HTML isn't sufficient:
- `role` — semantic role if element isn't a standard HTML element
- `aria-label` — visible label if text label unavailable
- `aria-expanded` — whether a region is expanded/collapsed
- `aria-live` — announces dynamic content updates

**Accessibility Paradox:** Misused ARIA makes things worse. "An inaccessible element with ARIA is still inaccessible." Prefer semantic HTML (native `<button>`, `<nav>`, `<form>`) to custom elements with ARIA patches.

## CI/CD Integration

Accessibility testing in CI:
- **Automated checks:** Run on every build, fail if violations exceed threshold
- **Baseline tracking:** Track violations over time, alert on regressions
- **Manual check gates:** Flag pages for manual accessibility review before release
- **Performance reports:** Dashboard showing accessibility metrics

**Tradeoff:** Automated checks catch low-hanging fruit quickly. Manual testing is required for comprehensive coverage but doesn't scale to every page.

## Testing Strategy

Comprehensive accessibility testing layers:
1. **Automated scanning (CI):** Catch obvious violations fast
2. **Keyboard navigation (manual):** Verify all interactions are keyboard-accessible
3. **Screen reader testing (manual):** Verify content is understandable via audio
4. **Color contrast (automated):** Verify readability standards
5. **User testing (manual):** Involve people with disabilities in real testing

**Limitation:** No testing approach catches all issues. Real-world usage by people with disabilities reveals problems automation and expert testing miss.

## Mental Model

Accessibility testing isn't a binary pass/fail. It's a spectrum: some interfaces are more accessible than others. The goal is continuous improvement toward usability for all people, knowing that perfection is impossible and trade-offs exist.

Automated testing is your safety net. Manual testing with assistive technology is your validation. User testing with actual people with disabilities is your truth.

See also: web-accessibility, testing-philosophy, architecture-resilience (accessibility as reliability)