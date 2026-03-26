# Web Internationalization (i18n): Locale APIs, Message Format & Text Layout

## Overview

Internationalization (i18n) is designing software to work correctly across languages, locales, and scripts. The web platform provides native APIs (the `Intl` object) for formatting numbers, dates, and text, plus layout primitives for RTL and text segmentation.

Frameworks like react-intl, vue-i18n, and format-style tools wrap these primitives into higher-level abstractions. But understanding the platform fundamentals is essential for avoiding subtle bugs with plurals, collation, and script boundaries.

## The Intl API: Native Locale Support

### Intl.NumberFormat

Formats numbers according to locale conventions. In the US, `1000` is "1,000". In France, "1 000". In Germany, "1.000". Currency symbols, decimal places, and grouping all vary:

```javascript
const usFormat = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
});
console.log(usFormat.format(1234.56));  // "$1,234.56"

const deFormat = new Intl.NumberFormat('de-DE', {
  style: 'currency',
  currency: 'EUR',
});
console.log(deFormat.format(1234.56));  // "1.234,56 €"

const percentFormat = new Intl.NumberFormat('en-US', {
  style: 'percent',
  minimumFractionDigits: 2,
});
console.log(percentFormat.format(0.275));  // "27.50%"
```

Key options:
- `style`: "decimal", "currency", "percent", "unit"
- `currency`: ISO 4217 code ("USD", "EUR", "JPY")
- `minimumFractionDigits`, `maximumFractionDigits`: Decimal places
- `useGrouping`: true/false for thousands separators

Locale negotiation: passing `'en'` finds the best available (e.g., `'en-US'` if exact not found).

### Intl.DateTimeFormat

Formats dates and times:

```javascript
const date = new Date('2025-03-25');

const usFormat = new Intl.DateTimeFormat('en-US', {
  weekday: 'long',  // "Tuesday"
  year: 'numeric',
  month: 'long',
  day: 'numeric',
  timeZone: 'UTC',
});
console.log(usFormat.format(date));  // "Tuesday, March 25, 2025"

const jpFormat = new Intl.DateTimeFormat('ja-JP', {
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
});
console.log(jpFormat.format(date));  // "2025/03/25"
```

Locale-aware calendars: pass `calendar` option ("buddhist", "hebrew", "islamicc", "iso8601").

Critical: always pass `timeZone` explicitly. Browsers default to local timezone; not specifying it leads to bugs when the server is in a different zone than the user.

### Intl.Collator

Compares and sorts strings according to locale rules. German ö sorts after o. Spanish ch was historically sorted as a separate letter. Swedish ä, ö, å have special sorting rules:

```javascript
const deCollator = new Intl.Collator('de-DE');
const words = ['Zürcher', 'Zebra', 'Zagen'];
console.log(words.sort(deCollator.compare));  // Proper German sort order

const fecollator = new Intl.Collator('en-US', { numeric: true });
const versions = ['v2', 'v10', 'v1'];
console.log(versions.sort(fecollator.compare));  // ["v1", "v2", "v10"]
```

Options:
- `numeric`: Sort "2" before "10" (numeric order, not lexicographic)
- `sensitivity`: "base" (ignore accents), "accent", "case", "variant" (all differences)
- `caseFirst`: "upper", "lower", or undefined (default)

### Intl.PluralRules

English has simple plurals: "1 cat", "2 cats". Polish has five plural forms. Mandarin has one. Relying on English rules elsewhere breaks translations:

```javascript
const enRules = new Intl.PluralRules('en-US');
console.log(enRules.select(1));      // "one"
console.log(enRules.select(0));      // "other"
console.log(enRules.select(2));      // "other"

const plRules = new Intl.PluralRules('pl-PL');
console.log(plRules.select(1));      // "one"
console.log(plRules.select(5));      // "many"
console.log(plRules.select(22));     // "few"
```

Return values (singular categories): "zero", "one", "two", "few", "many", "other" (not all present in every locale).

### Intl.Segmenter

Text segmentation splits strings at word, sentence, or grapheme boundaries. Critical for text processing in languages with complex scripts. Japanese doesn't use spaces; Khmer combines characters:

```javascript
const segmenter = new Intl.Segmenter('en-US', { granularity: 'word' });
const segments = [...segmenter.segment('Hello, World!')];
// [{segment: 'Hello', index: 0}, {segment: ',', index: 5}, ...]

const graphemeSegmenter = new Intl.Segmenter(undefined, { granularity: 'grapheme' });
const emoji = [...graphemeSegmenter.segment('👨‍👩‍👧‍👦')];
// Family emoji is 7 UTF-16 units, 1 grapheme cluster
console.log(emoji.length);  // 1, not 7
```

Granularities: "grapheme" (user-perceived character), "word", "sentence".

## Locale Negotiation and Resolution

When requesting a locale, the browser resolves the best available:

```javascript
const formatter = new Intl.DateTimeFormat(['de-AT', 'de', 'en-US']);
// If de-AT not available, tries de, then en-US
```

Server sends `Accept-Language` header; use it to set initial locale:

```javascript
const acceptLanguage = request.headers['accept-language'];
// Parse: "en-US,en;q=0.9,de;q=0.8"
```

Never hard-code. Let users override via settings; store preference in localStorage or a DB.

## ICU MessageFormat and Plural Messaging

Standard Intl methods don't handle complex message templates. ICU MessageFormat (used in many frameworks) provides syntax for conditionals and plural selection:

```
{gender, select,
  male {He}
  female {She}
  other {They}
} {name} will receive {count, plural,
  =0 {no notifications}
  one {# notification}
  other {# notifications}
}.
```

Frameworks like react-intl parse and evaluate this format. The `#` symbol is replaced with the selected number.

## Right-to-Left (RTL) Layout

RTL scripts (Arabic, Hebrew, Urdu) require layout direction flips:

```html
<html dir="rtl" lang="ar">
  <body>مرحبا</body>
</html>
```

CSS should use logical properties instead of left/right:

```css
/* Old way (breaks in RTL) */
.container {
  margin-left: 20px;
  text-align: left;
}

/* Logical way (auto-flips) */
.container {
  margin-inline-start: 20px;
  text-align: start;
}
```

Logical properties: `inline-start`/`inline-end` (left/right in LTR, right/left in RTL), `block-start`/`block-end` (top/bottom).

Flexbox and Grid handle RTL automatically with `flex-direction: row` flipping logically.

Pitfall: BiDi text mixing English and Arabic in one line requires `<bdi>` (bidirectional isolation) or explicit `unicode-bidi: isolate` in CSS to prevent reordering bugs.

## Transaction and Workflow Patterns

### Extractable Patterns

i18n frameworks extract translatable strings:

1. **Static strings**: `_("Hello, World")`
2. **Variables**: `_("Hello, {name}")`
3. **Plurals**: `_n("1 cat", "{count} cats", count)`
4. **Context**: `_x("Save", "button")`  // vs. `_x("Save", "verb")`

Tools scan source, generate `.po`/`.json` catalogs, ship to translators.

### Translation Workflow

1. **Extraction**: Tool scans code, builds catalog of strings
2. **Translation**: Humans or services translate to each language
3. **Compilation**: Translations bundled and shipped
4. **Runtime lookup**: Code requests translations by key, falls back to source language

Frameworks automate steps 1 and 4. Translators handle step 2. Developers orchestrate step 3.

## Common Frameworks and Tools

### react-intl

React-specific, declarative API:

```jsx
<FormattedMessage
  id="greeting"
  defaultMessage="Hello, {name}!"
  values={{ name: 'Alice' }}
/>

<FormattedNumber value={1234.56} style="currency" currency="USD" />

<FormattedDate value={new Date()} year="numeric" month="long" day="numeric" />
```

Extracts to JSON/XLIFF for translation.

### vue-i18n

Vue equivalents:

```vue
<p>{{ $t('greeting', { name: 'Alice' }) }}</p>
<p>{{ $n(1234.56, 'currency') }}</p>
```

### i18next

Framework-agnostic, widely used:

```javascript
i18n.t('greeting', { name: 'Alice' });
i18n.language = 'de';
```

## Common Pitfalls

- **Hardcoded strings**: Mixing translatable and untranslatable text; making translations difficult
- **String concatenation**: `"There are " + count + " items"` doesn't handle plurals; use message formats
- **Assuming English plurals**: Naively checking `count === 1` to pluralize fails for other languages
- **Forgetting grapheme clusters**: `string.length` counts UTF-16 units, not user-perceived characters; use Segmenter
- **Not handling RTL**: Testing only LTR; bidirectional text bugs appear later
- **Locale negotiation**: Assuming user's browser language is their preferred language; provide override
- **Hardcoded date formats**: Never use `toLocaleDateString()` without options; it varies by browser implementation
- **Ignoring collation**: Assuming ASCII sort order; use Collator for user-visible sorting

i18n is complex because language and culture vary deeply. Use native APIs where possible; rely on frameworks for higher-level abstractions. Test with diverse locales early.