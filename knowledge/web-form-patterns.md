# Web Form Patterns: State, Validation, and Accessibility

## Overview

Modern form handling balances **controlled state management** (React), **validation strategies** (client vs. server, progressive), **accessibility** (labels, errors, focus management), and **user experience** (autosave, multi-step flows, error display). Trade-offs: full control (React Hook Form) requires more setup; framework defaults (HTML5 validation) sacrifice flexibility. Success requires coordination across form state, validation, submission, and error handling.

## Controlled vs. Uncontrolled Inputs

### Controlled Inputs

React component state drives input value. **Single source of truth**: React state.

```jsx
function ControlledInput() {
  const [email, setEmail] = useState("");
  
  return (
    <input
      value={email}
      onChange={(e) => setEmail(e.target.value)}
      type="email"
    />
  );
}
```

Advantages:

- Programmatic control: reset, prefill, conditional rendering
- Real-time validation and computed fields
- Can enforce constraints immediately (e.g., no special chars)

Disadvantages:

- More boilerplate (state per field)
- Every keystroke causes re-render (mitigated by `React.memo` or memoization)
- Requires explicit onChange handler

### Uncontrolled Inputs

HTML input retains its own state. React accesses value via **refs** (imperative).

```jsx
function UncontrolledInput() {
  const emailRef = useRef(null);
  
  const handleSubmit = () => {
    console.log(emailRef.current.value);
  };
  
  return (
    <>
      <input ref={emailRef} type="email" />
      <button onClick={handleSubmit}>Submit</button>
    </>
  );
}
```

Advantages:

- Less boilerplate for trivial forms
- No re-renders on input change
- Integrates with non-React libraries (file inputs, rich editors)

Disadvantages:

- No programmatic control without refs
- Validation requires manual ref access
- Harder to coordinate multiple fields
- File inputs essentially uncontrolled (security restriction)

**Modern practice**: controlled inputs for managed applications; uncontrolled for simple inputs or library boundaries (file pickers, rich editors).

## Form State Management Libraries

### React Hook Form (RHF)

Minimal, performance-first library using uncontrolled inputs internally but exposing controlled-like API.

```jsx
import { useForm } from "react-hook-form";

function Form() {
  const { register, handleSubmit, formState: { errors }, watch } = useForm({
    defaultValues: { email: "", password: "" }
  });
  
  const watchedEmail = watch("email");  // re-render only this field on change
  
  const onSubmit = (data) => console.log(data);
  
  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <input {...register("email", { required: "Email required" })} />
      {errors.email && <span>{errors.email.message}</span>}
      
      <button type="submit">Submit</button>
    </form>
  );
}
```

**Strengths**: minimal re-renders (isolation), small bundle, flexible validation (sync/async), good TypeScript support, integrates any validation library.

**Weaknesses**: less opinionated (more decisions for developer); advanced state coordination requires understanding subscription model.

### Formik (Legacy Standard)

More opinionated, with built-in validation and state management.

```jsx
import { Formik, Form, Field, ErrorMessage } from "formik";
import * as Yup from "yup";

const validationSchema = Yup.object().shape({
  email: Yup.string().email().required(),
  password: Yup.string().min(8).required()
});

<Formik
  initialValues={{ email: "", password: "" }}
  validationSchema={validationSchema}
  onSubmit={(values) => console.log(values)}
>
  {({ isSubmitting }) => (
    <Form>
      <Field name="email" type="email" />
      <ErrorMessage name="email" />
      
      <button type="submit" disabled={isSubmitting}>Submit</button>
    </Form>
  )}
</Formik>
```

**Strengths**: complete ecosystem (validation, submission, error display), good for medium-complexity forms, mature API.

**Weaknesses**: heavier (bundle size), more re-renders, tighter coupling between validation and state.

### TanStack Form (Modern Alternative)

Headless form library providing state and validation orchestration without UI opinions.

```jsx
import { useForm } from "@tanstack/react-form";

const form = useForm({
  defaultValues: { email: "", password: "" },
  onSubmit: async (values) => console.log(values)
});

const emailField = form.getFieldInfo("email");

<input
  value={emailField.state.value}
  onChange={(e) => emailField.handleChange(e.target.value)}
  onBlur={() => emailField.fns.validateSync()}
/>
```

**Strengths**: framework-agnostic (works with Vue, Svelte, vanilla JS), highly composable, framework stability (used in industry).

**Weaknesses**: newer (less Stack Overflow help); requires explicit control of all interactions; opinionated around fields-as-objects.

## Validation Strategies

### Client-Side Validation

Provide immediate feedback; improve UX but not security.

**HTML5 native** (no library):

```html
<input type="email" required />
<input type="password" minlength="8" />
<input type="number" min="0" max="100" />
```

Limited control; browser-dependent UI (ugly error messages).

**Library-based** (Yup, Zod, Valibot):

```javascript
const schema = Yup.object().shape({
  email: Yup.string().email("Invalid email").required(),
  age: Yup.number().min(18, "Must be 18+").max(120)
});

schema.validate({ email: "test", age: 17 })
  .catch(err => console.log(err.message));
```

Declarative, composable, reusable schemas.

### Server-Side Validation

**Always validate on server** (client can be bypassed). Server is source of truth.

```javascript
// Server: Express + Zod
import { z } from "zod";

const schema = z.object({
  email: z.string().email(),
  password: z.string().min(8)
});

app.post("/register", (req, res) => {
  const result = schema.safeParse(req.body);
  if (!result.success) {
    return res.status(400).json({ errors: result.error.flatten() });
  }
  // proceed
});
```

### Progressive Validation

**Optimistic validation**: validate as user types (client), then confirm on submit (server).

```javascript
// 1. Client validates on blur (debounced)
<input
  onBlur={async (e) => {
    const { error } = await schema.validate(e.target.value);
    setFieldError(error ? error.message : null);
  }}
/>

// 2. Server validates on submit
const onSubmit = async (data) => {
  const response = await fetch("/api/submit", { 
    method: "POST", 
    body: JSON.stringify(data) 
  });
  const serverErrors = await response.json();
  if (serverErrors) setFormErrors(serverErrors);
};
```

UX benefit: fast feedback without waiting for network; security maintained via server.

## Multi-Step Forms

Break complex forms into digestible steps. State persists across steps; validation per step.

```jsx
function MultiStepForm() {
  const [step, setStep] = useState(1);
  const [formData, setFormData] = useState({
    email: "",
    password: "",
    profile: ""
  });
  
  const handleNext = async () => {
    if (step === 1) {
      // Validate email, password
      const valid = await validateStep1(formData);
      if (valid) setStep(2);
    } else if (step === 2) {
      // Validate profile
      const valid = await validateStep2(formData);
      if (valid) handleSubmit();
    }
  };
  
  return (
    <>
      {step === 1 && <StepAccount data={formData} onChange={setFormData} />}
      {step === 2 && <StepProfile data={formData} onChange={setFormData} />}
      
      <button onClick={() => setStep(Math.max(1, step - 1))}>Back</button>
      <button onClick={handleNext}>
        {step === 2 ? "Submit" : "Next"}
      </button>
    </>
  );
}
```

Track state in parent; children dispatch updates. Validation per step prevents forward navigation until valid.

## Autosave and Draft Management

Save partial form state periodically to server.

```jsx
function Form() {
  const [formData, setFormData] = useState(initialData);
  const [saving, setSaving] = useState(false);
  
  useEffect(() => {
    const timer = setTimeout(async () => {
      setSaving(true);
      await fetch("/api/draft", {
        method: "POST",
        body: JSON.stringify(formData)
      });
      setSaving(false);
    }, 1000);  // 1s debounce after last change
    
    return () => clearTimeout(timer);
  }, [formData]);
  
  return (
    <form>
      <input value={formData.email} onChange={e => setFormData({...formData, email: e.target.value})} />
      {saving && <span>Saving...</span>}
    </form>
  );
}
```

Debounce saves to avoid server overload; show "Saving" indicator for transparency.

## File Uploads

HTML file input is inherently uncontrolled (security restriction).

```jsx
function FileUpload() {
  const fileInputRef = useRef(null);
  const [files, setFiles] = useState([]);
  
  const handleChange = (e) => {
    const newFiles = Array.from(e.target.files);
    setFiles(prev => [...prev, ...newFiles]);
  };
  
  return (
    <>
      <input ref={fileInputRef} type="file" multiple onChange={handleChange} />
      {files.map(file => (
        <div key={file.name}>{file.name} ({file.size} bytes)</div>
      ))}
    </>
  );
}
```

**Validation**: check file type, size on client; always re-validate on server:

```javascript
const MAX_SIZE = 5 * 1024 * 1024;  // 5MB
const ALLOWED_TYPES = ["image/png", "image/jpeg"];

function validateFile(file) {
  if (file.size > MAX_SIZE) throw new Error("File too large");
  if (!ALLOWED_TYPES.includes(file.type)) throw new Error("Invalid type");
}
```

## Accessibility Patterns

### Labels and Associations

Always associate labels with inputs:

```jsx
<label htmlFor="email">Email Address</label>
<input id="email" type="email" aria-required="true" />
```

Clicking label focuses input; screen readers announce label text.

### Error Messaging with aria-describedby

Link error message to input for assistive technology:

```jsx
<input
  id="email"
  aria-describedby="email-error"
  aria-invalid={hasError}
/>
{hasError && (
  <span id="email-error" role="alert">{errorMessage}</span>
)}
```

Screen reader announces error when input receives focus.

### Focus Management

Trap focus in modal forms; manage focus on errors:

```jsx
useEffect(() => {
  if (hasError) {
    errorRef.current?.focus();
    errorRef.current?.scrollIntoView({ behavior: "smooth" });
  }
}, [hasError]);
```

### Required Indicators

**Visual + semantic**:

```jsx
<label htmlFor="email">
  Email Address
  <span aria-label="required"> *</span>
</label>
<input id="email" required />
```

Don't rely solely on `*`; use `required` attribute and aria-required.

## Error Display Strategies

### Inline Errors (Recommended)

Show errors next to field:

```jsx
<div>
  <input {...input} />
  {errors.email && (
    <p style={{ color: "red" }} role="alert">{errors.email.message}</p>
  )}
</div>
```

**Pros**: clear which field failed; accessible via aria-describedby.

### Summary Errors (Complementary)

List all errors at form top, linking to fields:

```jsx
{Object.keys(errors).length > 0 && (
  <div role="alert">
    <h2>Please fix these errors:</h2>
    <ul>
      {Object.entries(errors).map(([field, error]) => (
        <li key={field}>
          <a href={`#${field}`}>{field}: {error.message}</a>
        </li>
      ))}
    </ul>
  </div>
)}
```

**Combination**: summary (overview) + inline (context), best for long forms.

### Toast/Modal Errors

Avoid for form errors (unclear which field); reserve for general app errors.

## Modern Best Practices

1. **Validate progressively**: client-side on blur (UX); server-side on submit (security)
2. **Show loading state**: disable submit button, show spinner during submission
3. **Preserve data on errors**: don't clear form on validation failure
4. **Autosave drafts**: reduce user loss on page close
5. **Accessible labels**: always use `<label>` + `htmlFor`
6. **Combine inline + summary errors**: especially for multi-step forms
7. **Debounce async validation**: avoid overwhelming server (e.g., email uniqueness checks)
8. **Server revalidates**: never trust client validation

Form UX is often a product differentiator; investment in thoughtful patterns pays dividends in user retention and satisfaction.