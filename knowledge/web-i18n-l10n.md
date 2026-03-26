# Web Internationalization (i18n) & Localization (l10n) — MessageFormat, Locale APIs & Translation Workflows

## Internationalization vs. Localization

**Internationalization (i18n)**: Designing software to work across languages, locales, and scripts. This is an engineering process: building infrastructure for translations, date/number formatting, RTL layout, and locale-specific behavior.

**Localization (l10n)**: Adapting software to a specific locale (language + region). Translation is one part; locale-specific behavior (currency formatting, date conventions, collation order) is another.

The ratio "19" in i18n and "10" in l10n come from the number of letters between the first and last letter (internationalization = i + 18 letters + n).

## Message Format and Pluralization

Not all messages translate word-for-word. English plurals are binary: "1 item" vs. "N items". Polish has 4 plural forms. Japanese has none. ICU MessageFormat abstracts this.

### ICU MessageFormat syntax

```
{count, plural,
  zero {no items}
  one {one item}
  other {# items}
}
```

The `#` is replaced with the numeric value. Each locale defines which plural form applies. English uses `one` (count === 1) and `other`; Polish uses singular, few (2-4), many (5+), other.

Libraries like `format-message`, `react-intl`, and `i18next` handle MessageFormat compilation and locale data lookup:

```javascript
const msg = intl.formatMessage({
  id: 'item-count',
  defaultMessage: '{count, plural, one {one item} other {# items}}',
}, { count: 5 });
// "5 items"
```

### Gender and selection

MessageFormat also handles gender agreement and contextual selection:

```
{name} {gender, select,
  male {visited his friend}
  female {visited her friend}
  other {visited their friend}
}
```

And complex nesting:

```
{count, plural,
  one {{name} has one notification}
  other {{name} has # notifications}
}
```

## Unicode CLDR (Common Locale Data Repository)

CLDR is a database of locale-specific data: plural rules, date formats, number formats, currency symbols, collation order, calendar systems, etc. It's maintained by the Unicode Consortium and used by the `Intl` APIs and most i18n frameworks.

Features:
- **Plural rules**: For each language, which numeric threshold determines plural form (CLDR defines 15+ categories)
- **Date/time patterns**: Locale-specific orderings (MM/DD/YYYY vs. DD/MM/YYYY)
- **Number patterns**: Grouping separators, decimal symbols
- **Collation order**: Sort order for characters (important for languages with diacritics or multiple scripts)
- **Calendar systems**: Gregorian, Islamic, Hebrew, Chinese, etc.

When using `Intl.DateTimeFormat` or similar, the browser queries CLDR data for the locale.

## Native Intl APIs

JavaScript's `Intl` object provides locale-aware formatting without external libraries:

### Intl.NumberFormat

```javascript
const eur = new Intl.NumberFormat('de-DE', {
  style: 'currency',
  currency: 'EUR',
});
console.log(eur.format(1234.56));  // "1.234,56 €"

const compact = new Intl.NumberFormat('en-US', {
  notation: 'compact',  // "1.2K" instead of "1234"
});
console.log(compact.format(1234));  // "1.2K"
```

### Intl.DateTimeFormat

```javascript
const date = new Date('2025-03-25');
const options = {
  weekday: 'long',
  year: 'numeric',
  month: 'long',
  day: 'numeric',
  timeZone: 'UTC',
};
const formatter = new Intl.DateTimeFormat('de-DE', options);
console.log(formatter.format(date));  // "Dienstag, 25. März 2025"
```

### Intl.RelativeTimeFormat

```javascript
const rtf = new Intl.RelativeTimeFormat('en-US', { numeric: 'auto' });
console.log(rtf.format(-1, 'day'));    // "yesterday"
console.log(rtf.format(3, 'month'));   // "in 3 months"
```

### Intl.Collator

For sorting strings with locale-aware rules:

```javascript
const collator = new Intl.Collator('sv-SE');  // Swedish
const words = ['älska', 'apples', 'ålder'];
console.log(words.sort(collator.compare));
// In Swedish, 'ä' sorts after 'z', and 'å' after 'ä'
```

## RTL (Right-to-Left) Layout

Languages like Arabic, Hebrew, Persian, and Urdu are written RTL. CSS logical properties make RTL adaptation simpler:

Instead of:
```css
.sidebar { margin-left: 20px; }  /* Won't flip in RTL */
```

Use:
```css
.sidebar { margin-inline-start: 20px; }  /* Flips based on dir attribute */
```

Logical properties: `margin-inline-start`, `padding-block-end`, `border-inline-end`, `text-align: start`, etc.

Set the `dir` attribute on the root element:
```html
<html dir="ar">  <!-- Arabic -->
```

Be aware: Flexbox and Grid respect `dir` automatically, but floats and absolute positioning do not unless you use logical properties.

## Locale Detection

Three sources determine the user's locale:

1. **User preference**: Browser language settings, user agent header
2. **URL parameter**: `?lang=fr` or `/fr/` path prefix
3. **Geolocation**: Infer from IP (imperfect; VPNs, distributed users, etc.)

Pattern: Use URL-based routing (path prefix or query) so language choice is shareable and bookmarkable. Fall back to browser language on first visit:

```javascript
const userLang = navigator.language;  // e.g., "en-US"
const supportedLangs = ['en', 'fr', 'de'];
const lang = supportedLangs.includes(userLang.split('-')[0]) ? userLang.split('-')[0] : 'en';
```

## Translation Workflow

### Format agnostic storage

Store translatable strings in a canonical format (often JSON-like or PO files):

```json
{
  "greeting": "Hello, {name}!",
  "item_count": "{count, plural, one {one item} other {# items}}"
}
```

Each locale gets its own file:
```
locales/en.json
locales/fr.json
locales/de.json
```

### Extraction and management

**String extraction**: Tools scan source code for `i18n.t('key')` calls and build a master catalog:

```bash
i18n extract --output locales/messages.po
```

**Translation management**: The catalog is sent to translators (via CAT tools like Crowdin, OneSky, or manually). They return translated files.

**Fallback chain**: If a translation is missing, fall back to the default locale, then to a built-in English fallback.

## i18n Libraries

### react-intl

```javascript
import { IntlProvider, FormattedMessage } from 'react-intl';

<IntlProvider locale="fr" messages={frMessages}>
  <FormattedMessage
    id="greeting"
    defaultMessage="Hello, {name}!"
    values={{ name: 'Alice' }}
  />
</IntlProvider>
```

Provides `useIntl()` hook for accessing formatters and message IDs.

### i18next

Framework-agnostic:

```javascript
import i18n from 'i18next';

i18n.init({
  lng: 'en',
  resources: { en: { translation: enMessages } },
});

i18n.t('greeting', { name: 'Bob' });
```

Supports lazy loading namespaces, backend plugins (for server-side translation), pluralization, and interpolation.

### vue-i18n

```javascript
const i18n = createI18n({
  locale: 'en',
  messages: { en: {...}, fr: {...} },
});

app.use(i18n);
```

In templates: `{{ $t('greeting') }}` or `<i18n path="greeting" />`

## Text Expansion and Layout

Translated text often expands or contracts. German sentences can be 30% longer than English; Chinese is typically shorter. This affects fixed-width components.

Patterns:
- Avoid fixed widths on text containers; use flexbox/grid
- Design comfortably for the longest language (German, Finnish)
- Test layout with placeholder text ("Lorem ipsum" doesn't work; use realistic translations)
- Use `word-break: break-word` or `overflow-wrap: break-word` on long strings

## Pseudo-localization (Testing)

Pseudo-localization replaces strings with accented substitutes to identify hard-coded (non-translatable) strings:

```
"Hello" → "Ĥëłłõ"
"Dashboard" → "Ðåšĥβõåŕđ"
```

Any string that doesn't transform is hard-coded and won't translate. Run pseudo-localization against your app to catch missed strings early.

## See also

- [web-internationalization.md](web-internationalization.md) — Intl API details, locale negotiation, formatting
- [unicode-text-processing.md](unicode-text-processing.md) — Unicode normalization, grapheme clusters, text segmentation