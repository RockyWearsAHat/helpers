# Web Forms: Structure, Validation & Accessibility

## Overview

Forms are the primary input mechanism for the web. Three dimensions define modern form handling: **structure** (controlled vs uncontrolled inputs), **validation** (client-side, schema-based, server-side), and **accessibility** (ARIA labels, error announcements, keyboard navigation). Poor forms frustrate users; well-designed forms reduce abandonment.

## Controlled vs Uncontrolled Inputs

**Controlled input**: React state owns the value. Every keystroke triggers `onChange` → state update → re-render → input value is read from state.

```jsx
const [name, setName] = useState('');
<input value={name} onChange={(e) => setName(e.target.value)} />
```

Advantages: React state is source of truth, can validate on every keystroke, can disable submit when invalid, can integrate with React devtools. Disadvantages: verbose, forces re-render per keystroke (performance risk on large forms), requires state for every field.

**Uncontrolled input**: DOM owns the value. Read value via ref when needed (on submit).

```jsx
const nameRef = useRef();
const handleSubmit = () => {
  const name = nameRef.current.value;
  // Validate, submit
};
<input ref={nameRef} />
```

Advantages: Less verbose, no re-renders per keystroke, integrates with browser form APIs naturally. Disadvantages: React doesn't "know" the value, harder to validate in real-time, can't easily disable submit button based on form state.

**Pattern in modern forms**: Uncontrolled fields (native browser value management) + validation library that reads all fields on submit or blur. Libraries like React Hook Form and Formik bridge this with performant approaches.

## Form Libraries: React Hook Form vs Formik

**React Hook Form**: Minimal library using hooks, uncontrolled inputs by default, focuses on validation + error handling with minimal re-renders.

```jsx
const { register, handleSubmit, formState: { errors } } = useForm();
<input {...register('email', { required: 'Email required' })} />
{errors.email && <span>{errors.email.message}</span>}
```

**Pros**: Smallest bundle (~9kb), minimal re-renders (only affected fields), excellent DX, TypeScript first-class, flexible validation integration (Zod, Yup, VeeValidate).

**Cons**: Uncontrolled by default (requires ref learning), slightly different mental model, fewer built-in features than Formik.

**Formik**: Older standard. Managed state, opinionated validation flow, tightly integrated with Yup schema library.

```jsx
const formik = useFormik({
  initialValues: { email: '' },
  validationSchema: yupSchema,
  onSubmit: (values) => { /* ... */ }
});
<input {...formik.getFieldProps('email')} />
{formik.touched.email && formik.errors.email && <span>{formik.errors.email}</span>}
```

**Pros**: Explicit, all-in-one solution, tutorial-rich, familiar to large teams.

**Cons**: Larger bundle (~15kb), more re-renders by default, verbose API, couples validation (Yup) slightly tighter.

**Decision**: React Hook Form for new projects (simpler, modern). Formik for established codebases or teams highly invested in the pattern. Difference in UX is minimal.

## Schema Validation: Zod, Yup, Valibot

Schema libraries define what data is valid, then validate values against schemas. Used for form input validation and API response validation.

**Zod**: Modern, TypeScript-first, great error messages. Schemas are chainable.

```javascript
const schema = z.object({
  email: z.string().email(),
  age: z.number().min(18)
});
```

Advantages: Excellent DX, inferred TypeScript types, small bundle. Disadvantages: Newer (less Stack Overflow answers), smaller ecosystem than Yup.

**Yup**: Older, widely adopted, familiar API, schema builder pattern. Used in Formik tutorials.

```javascript
const schema = yup.object().shape({
  email: yup.string().email().required(),
  age: yup.number().min(18).required()
});
```

Advantages: Established, ecosystem, documentation. Disadvantages: Slightly more verbose, older API, TypeScript support added later.

**Valibot**: Modular, tree-shakeable, small bundles even with many validators. Newer entrant.

```javascript
const schema = v.object({
  email: v.pipe(v.string(), v.email()),
  age: v.pipe(v.number(), v.minValue(18))
});
```

Advantages: Modular tree-shaking, small, composable. Disadvantages: Youngest ecosystem, smallest community.

**Pattern**: Use one schema library for both form validation and fetched data validation. Example: React Hook Form + Zod for client-side, send data to server, server parses and validates with same/compatible schema.

## Client-Side Validation

Validate as user types (debounced for performance) or on blur. Provides instant feedback. Examples: email format, password strength, field length.

**Advantages**: Fast feedback loop, better UX, catches obvious errors before submit.

**Disadvantages**: Can't validate against server state (user already registered), can't access server-only data, attackers ignore client validation.

**Never trust client validation alone.** Always validate on server.

**Debouncing** prevents validation from running on every keystroke. Example: user types 10 characters; don't validate 10 times. Validate once after user stops typing for 300ms.

## Server-Side Validation

After form submit, server receives data. Server re-validates (never trust client). If invalid, returns errors; client displays them.

**Required because**: Client-side validation can be bypassed (disabled JS, intercept network request), only server knows full context (is username available? is this user authorized?).

**Pattern**: Form submit → server validation fails → return 400 + error details → client displays errors → user corrects → re-submit.

**Modern approach**: Structured error responses.

```json
{
  "errors": {
    "email": "Email already registered",
    "password": "Password too weak"
  }
}
```

Client maps errors to form fields, displays inline feedback.

**Server frameworks** (Next.js Server Actions, tRPC, GraphQL) blur client/server boundary. Can call server functions directly from form handlers, errors bubble up to client automatically.

## Progressive Enhancement for Forms

Forms work without JavaScript. HTML form submission (POST) has been the default for 30 years.

```html
<form method="POST" action="/submit">
  <input name="email" type="email" required />
  <button type="submit">Submit</button>
</form>
```

Without JavaScript, submit posts data to server, page refreshes. With JavaScript, `preventDefault()` and handle via AJAX (no page refresh).

**Progressive enhancement**:
1. HTML form works (browser default behavior)
2. JavaScript enhances to AJAX (no page reload) + inline error display
3. CSS enhances to smooth feedback animations

**Modern relevance**: Increasingly important with slower networks, async JavaScript failures. SPAs that *only* work with JavaScript are brittle. Forms that work as HTML-first are resilient.

## Accessibility: ARIA & Error Announcements

Accessible forms ensure screen reader users can navigate, understand labels, and know what went wrong.

**Labels**:
```html
<label for="email">Email</label>
<input id="email" type="email" />
```

Associating `<label>` to `<input>` via `for`/`id` makes click-target larger (UX win), announced to screen readers, semantic.

**ARIA attributes**:
- `aria-label`: "Close button" for icon-only buttons
- `aria-describedby`: Link input to help text or error message
- `aria-required`: Mark field as required
- `aria-invalid`: Mark field as invalid
- `aria-live`: Announce error messages dynamically

**Error announcement pattern**:
```jsx
<div role="alert" aria-live="polite" aria-atomic="true">
  {errors.email && <p>{errors.email.message}</p>}
</div>
```

`role="alert"` + `aria-live="polite"` tells screen reader: "This region updates dynamically, announce changes." Screen reader will announce error to user immediately after validation.

**Keyboard navigation**:
- `Tab` and `Shift+Tab` navigate between fields (default browser behavior)
- `Enter` submits form (default in `<button type="submit">`)
- `Space` toggles checkboxes, radios
- Arrow keys navigate radio groups

**Validation visual feedback**:
- Red border alone is not enough (color-blind users won't see it)
- Use icon + text: ✗ "Email required"
- High contrast error text

## Multi-Step Forms

Long forms are abandoned. Break into steps (checkout → shipping → billing → review).

**Approaches**:
1. **Single page, step UI**: All fields rendered, show/hide groups. Simple, but loads all fields up-front.
2. **Separate routes per step**: `/checkout/shipping` → `/checkout/billing`. Each route is a fresh page or SPAs with client-side routing.
3. **Tabs w/ validation**: Steps in tabs, can navigate if previous steps are valid.

**State management**: Store form state in React state, URL params, or server. URL params allow back-button navigation, bookmark-ability. Server (session) state survives page refresh.

**Validation strategy**:
- Per-step validation (validate step before allowing next)
- Final validation on submit (lint all fields, then submit)

**UX considerations**:
- Show progress bar (Step 1 of 3)
- Allow "previous" even after validation errors
- Save form progress (so user doesn't lose data on browser crash)
- Confirm before abandoning (half-filled form)

**Library support**: React Hook Form works multi-step naturally (useForm over entire form, validate fields conditionally). Formik also supports multi-step with special handling for field-level validation.

## Conditional & Dynamic Fields

Forms where fields appear/disappear based on previous answers.

**Example**: "Do you have a business?" → if yes, show "Business name" field → if no, skip it.

**Implementation**: Render/hide fields conditionally based on form state.

```jsx
const { watch, register } = useForm();
const hasAccount = watch('hasAccount');

<input {...register('hasAccount')} type="checkbox" />
{hasAccount && (
  <input {...register('accountNumber')} />
)}
```

**Validation**: Schema validators (Zod, Yup) support conditional validation with `.refine()` or `.when()`. Ensure validation schema matches rendering logic.

## File Uploads

Forms can accept file uploads via `<input type="file">`.

**Client-side**:
- Accept attribute: `<input accept=".pdf,.doc">`
- File size check: `file.size > maxBytes`
- File type check: `file.type === 'application/pdf'`

**Server-side**:
- Re-validate file type and size (never trust client)
- Scan for viruses (optional)
- Store securely (outside web root, with random name)

**UX**:
- Show progress bar during upload
- Provide feedback on upload status
- Allow retry on failure
- Preview uploaded files

**Libraries**: Form libraries abstract file handling. React Hook Form integrates with `<Controller>` for file inputs.

## State of the Art: React Server Actions + Zod

Modern pattern: form handler lives on server (Next.js Server Action). Client calls server function directly, server validates with Zod, returns typed result.

```jsx
'use server';
const schema = z.object({ email: z.string().email() });

export async function submitForm(data) {
  const parsed = schema.safeParse(data);
  if (!parsed.success) return { error: parsed.error.flatten() };
  // Save to DB
  return { success: true };
}

// In client component:
'use client';
import { submitForm } from './actions';
export default function Form() {
  const [result, action] = useFormState(submitForm, null);
  return (
    <form action={action}>
      <input name="email" />
      <button>Submit</button>
      {result?.error && <span>{result.error.fieldErrors.email}</span>}
    </form>
  );
}
```

**Advantages**: No API boilerplate, type-safe bridge, validation defined once, works without JavaScript (form submits to server action default).

**Status**: Next.js 13+ with `useFormState`. Emerging pattern, not yet standard across frameworks.

## References

- React Hook Form: https://react-hook-form.com/
- Formik: https://formik.org/
- Zod: https://zod.dev/
- Yup: https://github.com/jquense/yup
- Valibot: https://valibot.dev/
- MDN HTML Forms: https://developer.mozilla.org/en-US/docs/Learn/Forms
- Web Accessibility Initiative forms: https://www.w3.org/WAI/tutorials/forms/
- ARIA authoring practices: https://www.w3.org/WAI/ARIA/apg/patterns/

See also: [web-accessibility.md](web-accessibility.md), [framework-nextjs.md](framework-nextjs.md)