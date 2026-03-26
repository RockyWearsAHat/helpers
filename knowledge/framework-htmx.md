# htmx

## Philosophy

htmx extends HTML with attributes that enable AJAX, CSS transitions, WebSockets, and SSE directly in markup. The core idea: the server returns HTML fragments, not JSON. No build step, no virtual DOM, no client-side routing. The hypermedia (HTML) is the API.

### When to Use htmx vs SPA Frameworks

**htmx fits when**: content-heavy sites, admin panels, CRUD apps, progressive enhancement, teams with backend strength, projects where SEO matters, reduced client complexity.

**SPA fits better when**: highly interactive UIs (Google Docs, Figma), offline-first, complex client-side state machines, rich real-time collaboration.

## Core Attributes

### AJAX Requests

```html
<!-- HTTP verbs -->
<button hx-get="/api/users">Load Users</button>
<form hx-post="/api/users">...</form>
<button hx-put="/api/users/1">Update</button>
<button hx-delete="/api/users/1">Delete</button>
<button hx-patch="/api/users/1">Partial Update</button>

<!-- All return HTML fragments from the server -->
```

### Targets (Where to Put the Response)

```html
<!-- Default: replace innerHTML of the element itself -->
<button hx-get="/users" hx-target="#user-list">Load</button>
<div id="user-list"></div>

<!-- Target selectors -->
<button hx-target="this">Replace self</button>
<button hx-target="closest tr">Replace table row</button>
<button hx-target="next .output">Next sibling with .output</button>
<button hx-target="previous div">Previous div</button>
<button hx-target="find .child">First child match</button>
```

### Swap Strategies (How to Insert)

```html
<!-- hx-swap controls insertion method -->
<div hx-get="/content" hx-swap="innerHTML">
  <!-- default: replace children -->
  <div hx-get="/content" hx-swap="outerHTML">
    <!-- replace entire element -->
    <div hx-get="/content" hx-swap="beforebegin">
      <!-- insert before element -->
      <div hx-get="/content" hx-swap="afterbegin">
        <!-- insert as first child -->
        <div hx-get="/content" hx-swap="beforeend">
          <!-- insert as last child (append) -->
          <div hx-get="/content" hx-swap="afterend">
            <!-- insert after element -->
            <div hx-get="/content" hx-swap="delete">
              <!-- delete target -->
              <div hx-get="/content" hx-swap="none">
                <!-- don't swap (for side effects) -->

                <!-- Swap with transition timing -->
                <div
                  hx-get="/content"
                  hx-swap="innerHTML swap:300ms settle:500ms"
                >
                  <!-- swap: delay before old content removed -->
                  <!-- settle: delay before new content settled (for CSS transitions) -->

                  <!-- Scroll behavior -->
                  <div hx-get="/content" hx-swap="innerHTML scroll:top">
                    <div
                      hx-get="/content"
                      hx-swap="innerHTML show:#element:top"
                    ></div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
```

## Triggers

```html
<!-- Default triggers: click for most elements, change for inputs/selects, submit for forms -->

<!-- Custom triggers -->
<input
  hx-get="/search"
  hx-trigger="keyup changed delay:300ms"
  hx-target="#results"
/>
<!-- keyup: on keyup event -->
<!-- changed: only if value actually changed -->
<!-- delay:300ms: debounce -->

<div hx-get="/news" hx-trigger="every 5s">Live feed</div>

<div hx-get="/more" hx-trigger="revealed">Loads when scrolled into view</div>

<div hx-get="/data" hx-trigger="intersect threshold:0.5">
  Loads when 50% visible
</div>

<!-- Event modifiers -->
<button hx-post="/action" hx-trigger="click once">One-time click</button>
<form hx-post="/save" hx-trigger="submit throttle:2s">Throttled submit</form>
<div hx-get="/data" hx-trigger="click from:body">Click anywhere</div>
<div hx-get="/data" hx-trigger="myCustomEvent from:body">Custom event</div>

<!-- Multiple triggers -->
<input hx-get="/validate" hx-trigger="change, keyup delay:500ms changed" />

<!-- Consume (stop propagation) -->
<button hx-get="/data" hx-trigger="click consume">Won't bubble</button>
```

## Boosting

Turn regular links and forms into AJAX calls automatically:

```html
<!-- Boost a navigation section — all links become AJAX -->
<nav hx-boost="true">
  <a href="/about">About</a>
  <!-- now AJAX, swaps body -->
  <a href="/contact">Contact</a>
</nav>

<!-- Boost a form -->
<form hx-boost="true" action="/login" method="post">
  <input name="email" type="email" />
  <button type="submit">Login</button>
</form>
<!-- Submits via AJAX, swaps body with response -->
```

## Indicators

Show loading state during requests:

```html
<button hx-get="/slow-data" hx-indicator="#spinner">Load Data</button>
<span id="spinner" class="htmx-indicator">Loading...</span>

<!-- CSS: htmx adds .htmx-request class during requests -->
<style>
  .htmx-indicator {
    display: none;
  }
  .htmx-request .htmx-indicator {
    display: inline;
  }
  .htmx-request.htmx-indicator {
    display: inline;
  }
</style>
```

## Request & Response Headers

### Request Headers (sent by htmx)

| Header            | Value                     |
| ----------------- | ------------------------- |
| `HX-Request`      | `true` (always sent)      |
| `HX-Target`       | ID of target element      |
| `HX-Trigger`      | ID of triggered element   |
| `HX-Trigger-Name` | Name of triggered element |
| `HX-Current-URL`  | Current browser URL       |
| `HX-Prompt`       | Value from `hx-prompt`    |
| `HX-Boosted`      | `true` if boosted request |

### Response Headers (server can set)

| Header                    | Effect                                 |
| ------------------------- | -------------------------------------- |
| `HX-Redirect`             | Client-side redirect                   |
| `HX-Location`             | Client-side navigation (like hx-boost) |
| `HX-Refresh`              | Full page refresh if `true`            |
| `HX-Trigger`              | Trigger client-side event(s)           |
| `HX-Trigger-After-Settle` | Trigger after settling                 |
| `HX-Trigger-After-Swap`   | Trigger after swap                     |
| `HX-Reswap`               | Override `hx-swap`                     |
| `HX-Retarget`             | Override `hx-target`                   |
| `HX-Replace-Url`          | Update URL bar                         |
| `HX-Push-Url`             | Push to history                        |

```python
# Server triggers client event with data
response.headers['HX-Trigger'] = json.dumps({
    'showToast': {'message': 'User created', 'type': 'success'}
})
```

```html
<!-- Listen for custom event -->
<div hx-on:showToast="showToast(event.detail)"></div>
```

## CSRF Handling

```html
<!-- Include CSRF token in all requests -->
<meta name="csrf-token" content="{{ csrf_token }}" />

<script>
  document.body.addEventListener("htmx:configRequest", (event) => {
    event.detail.headers["X-CSRFToken"] = document.querySelector(
      'meta[name="csrf-token"]',
    ).content;
  });
</script>

<!-- Or use hx-headers -->
<body hx-headers='{"X-CSRFToken": "{{ csrf_token }}"}'></body>
```

## Common Patterns

### Infinite Scroll

```html
<table>
  <tbody id="rows">
    {% for item in items %}
    <tr>
      <td>{{ item.name }}</td>
    </tr>
    {% endfor %}
    <tr
      hx-get="/items?page={{ next_page }}"
      hx-trigger="revealed"
      hx-swap="afterend"
      hx-target="this"
    >
      <td>Loading...</td>
    </tr>
  </tbody>
</table>
```

### Active Search

```html
<input
  type="search"
  name="q"
  hx-get="/search"
  hx-trigger="input changed delay:300ms, search"
  hx-target="#results"
  hx-indicator="#search-spinner"
  placeholder="Search..."
/>
<span id="search-spinner" class="htmx-indicator">🔍</span>
<div id="results"></div>
```

### Inline Editing

```html
<!-- View mode -->
<div hx-get="/users/1/edit" hx-trigger="click" hx-swap="outerHTML">
  <span>John Doe</span>
  <span>john@example.com</span>
</div>

<!-- Server returns edit form -->
<form hx-put="/users/1" hx-swap="outerHTML">
  <input name="name" value="John Doe" />
  <input name="email" value="john@example.com" />
  <button>Save</button>
  <button hx-get="/users/1" hx-swap="outerHTML">Cancel</button>
</form>
```

### Delete with Confirmation

```html
<button
  hx-delete="/users/1"
  hx-confirm="Are you sure?"
  hx-target="closest tr"
  hx-swap="outerHTML swap:500ms"
  class="delete-btn"
>
  Delete
</button>
```

### Bulk Operations

```html
<form hx-post="/users/bulk-delete" hx-target="#user-table" hx-swap="innerHTML">
  <table id="user-table">
    <tr>
      <td><input type="checkbox" name="ids" value="1" /></td>
      <td>Alice</td>
    </tr>
    <tr>
      <td><input type="checkbox" name="ids" value="2" /></td>
      <td>Bob</td>
    </tr>
  </table>
  <button>Delete Selected</button>
</form>
```

## Out-of-Band Swaps

Update multiple parts of the page from one response:

```html
<!-- Server response can include OOB elements -->
<!-- Main response swapped into target normally -->
<div id="main-content">Updated content</div>

<!-- OOB element swapped by ID regardless of target -->
<div id="notification-count" hx-swap-oob="true">5</div>
<div id="sidebar" hx-swap-oob="innerHTML">New sidebar content</div>
```

## WebSocket & SSE Extensions

```html
<!-- WebSocket -->
<div hx-ext="ws" ws-connect="/ws/chat">
  <div id="messages"></div>
  <form ws-send>
    <input name="message" />
    <button>Send</button>
  </form>
</div>

<!-- Server-Sent Events -->
<div hx-ext="sse" sse-connect="/events">
  <div sse-swap="message">Waiting for messages...</div>
  <div sse-swap="notification">No notifications</div>
</div>
```

## Backend Integration Patterns

### Django

```python
def search_users(request):
    query = request.GET.get('q', '')
    users = User.objects.filter(name__icontains=query)[:20]

    if request.headers.get('HX-Request'):
        return render(request, 'partials/user_list.html', {'users': users})
    return render(request, 'users/search.html', {'users': users, 'query': query})
```

### Go (Gin)

```go
func searchHandler(c *gin.Context) {
    query := c.Query("q")
    users := searchUsers(query)

    if c.GetHeader("HX-Request") == "true" {
        c.HTML(200, "partials/results.html", gin.H{"users": users})
        return
    }
    c.HTML(200, "search.html", gin.H{"users": users, "query": query})
}
```

### Flask

```python
@app.route('/search')
def search():
    users = User.query.filter(User.name.ilike(f'%{request.args["q"]}%')).all()
    if request.headers.get('HX-Request'):
        return render_template('partials/user_rows.html', users=users)
    return render_template('search.html', users=users)
```

## History & URL Management

```html
<!-- Push URL to browser history -->
<a hx-get="/page2" hx-push-url="true">Page 2</a>

<!-- Replace current URL (no history entry) -->
<div hx-get="/filtered" hx-replace-url="/items?filter=active">Filter</div>

<!-- Disable history -->
<div hx-get="/modal" hx-push-url="false">Open Modal</div>
```

## Validation

```html
<!-- Client-side: combine with HTML5 validation -->
<form hx-post="/users">
  <input name="email" type="email" required />
  <input name="age" type="number" min="0" max="150" required />
  <button>Submit</button>
</form>

<!-- Server-side: return form with errors -->
<!-- Server returns 422 with error markup → htmx swaps it in -->
```

## Events

```html
<!-- Listen to htmx events -->
<div
  hx-on::before-request="console.log('sending...')"
  hx-on::after-request="console.log('done')"
  hx-on::response-error="alert('Server error')"
></div>

<!-- JavaScript event listeners -->
<script>
  htmx.on("htmx:beforeSwap", (event) => {
    if (event.detail.xhr.status === 422) {
      event.detail.shouldSwap = true; // swap even on 4xx
      event.detail.isError = false;
    }
  });
</script>
```

## Size & Performance

htmx is ~14KB minified+gzipped. Zero dependencies. Include via CDN or self-host:

```html
<script src="https://unpkg.com/htmx.org@2.0.0"></script>
```
