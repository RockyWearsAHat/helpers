---
applyTo: "**/*.{js,mjs,ts,tsx,py,sh,bash,zsh,go,rs,java,cs,c,cpp,h,hpp}"
description: "Lean universal design principles: clarity, modularity, validation, and maintainability."
---

# Software Design (Lean)

1. Prefer clear, traceable logic over clever compactness.
2. Keep modules and functions focused; extract cohesive helpers when needed.
3. Validate boundaries and handle errors explicitly.
4. Preserve behavior unless change is intentional.
5. Validate changes with diagnostics/tests before completion.

Use an explicit named lock object: `private static readonly object _lock = new();`. Never `lock(this)` or `lock(typeof(...))`.

### File header

```csharp
// <copyright file="FileName.cs" company="CompanyOrProject">
// Copyright (c) YEAR Author. All rights reserved.
// </copyright>
// Author: Alex Waldmann
// Date: YYYY-MM-DD
```

For course projects: use the course-provided copyright block verbatim and add `// Name:` / `// Date:` directly below it.

### Razor / Blazor

Directive order: `@page`, `@rendermode`, `@using`, `@inject`. `id` attribute on every interactive element. All C# logic in `@code { }` at bottom of file. Conditional CSS computed in `@{ }` block before markup, never inline ternary in `class=`.
