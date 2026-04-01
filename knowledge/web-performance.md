# Web Performance Principles & Measurement

## Why Performance Matters

Web performance sits at the intersection of engineering discipline, user experience, business outcomes, and accessibility. Its importance extends beyond raw speed.

**User perception**: Research across multiple decades consistently shows that response latency shapes user satisfaction, task completion rates, and willingness to return. Thresholds vary by context — a search result page has different tolerance than a data visualization dashboard — but delays beyond a few hundred milliseconds are perceptible, and delays beyond a few seconds fundamentally change user behavior.

**Business metrics**: Performance correlates with conversion rates, engagement, bounce rates, and revenue across industries. The relationship is not always linear — there may be thresholds below which improvements yield diminishing returns and above which degradation compounds.

**Accessibility**: Performance is an accessibility concern. Users on low-end devices, constrained networks, or limited data plans experience performance problems more severely. A site that loads in 2 seconds on a flagship device over fiber may take 15 seconds on a budget phone over 3G. Performance optimization disproportionately benefits users with the fewest resources.

**Search ranking**: Search engines incorporate performance signals into ranking algorithms, creating a feedback loop where slower pages receive less traffic and slower-loading competitors gain visibility.

## The Two Performance Domains

Web performance splits into two largely independent problem spaces that require different diagnostic approaches and different optimization strategies.

### Loading Performance

Concerns the time from navigation initiation to the page becoming usable. Key factors:

| Factor                 | Nature                                          |
| ---------------------- | ----------------------------------------------- |
| Network latency        | Round-trip time between client and server       |
| Bandwidth              | Rate at which bytes arrive                      |
| Resource size          | Total bytes that must be transferred            |
| Resource count         | Number of independent requests                  |
| Dependency chains      | Sequential requests that cannot overlap         |
| Server response time   | Time to generate and begin sending the response |
| Parse and compile time | CPU cost of processing received resources       |

Loading performance is bounded by physics (speed of light for network latency), infrastructure (server capacity, CDN distribution), and engineering decisions (resource optimization, delivery strategy).

### Runtime Performance

Concerns the responsiveness and visual smoothness of the page after it has loaded. Key factors:

| Factor                | Nature                                          |
| --------------------- | ----------------------------------------------- |
| Main thread occupancy | Long-running JavaScript blocking input handling |
| Layout complexity     | Cost of computing element positions and sizes   |
| Paint complexity      | Cost of rasterizing visual content              |
| Memory pressure       | Garbage collection pauses, memory leaks         |
| Animation smoothness  | Maintaining consistent frame timing             |
| Input latency         | Delay between user action and visible response  |

Runtime performance is bounded by device capability (CPU, GPU, memory), application architecture, and the efficiency of DOM interaction patterns.

Optimizations for one domain may not affect the other. A page can load quickly but be sluggish to interact with, or load slowly but run smoothly once rendered.

## Core Web Vitals as Measurement Concepts

Core Web Vitals represent an attempt to quantify user experience through measurable proxies. The specific metrics evolve over time, but the underlying concepts they measure are more stable.

### Loading Experience

How quickly does meaningful content become visible? This encompasses:

- **First paint**: When the browser renders anything other than the background
- **First contentful paint**: When the browser renders the first piece of DOM content (text, image, SVG)
- **Largest contentful paint**: When the largest visible content element finishes rendering

The progression from first paint to largest contentful paint captures the user's perception of "the page is loading" to "the page has loaded." The largest element is used as a proxy because it typically represents the primary content the user came to see.

### Interactivity

How quickly does the page respond to user input? This encompasses:

- **First input delay**: Latency between the user's first interaction and the browser's ability to process it
- **Interaction to next paint**: The time from any user interaction (click, tap, keypress) to the next visual update

The interactivity metric has evolved from measuring only the first interaction to measuring all interactions, recognizing that responsiveness throughout the session matters.

### Visual Stability

How much does the page layout shift unexpectedly? This encompasses:

- **Cumulative layout shift**: A score aggregating all unexpected layout movements during the page lifecycle

Layout shifts are frustrating — they cause misclicks, disorientation, and a sense that the page is unreliable. Sources include images without dimensions, dynamically injected content, fonts that cause text reflow, and late-loading advertisements.

### Limitations of Metrics

Metrics are proxies, not the experience itself. Notable gaps:

- Metrics cannot capture **perceived smoothness** of animations or scroll
- A page with good metric scores can still feel subjectively slow if critical content is delayed while non-critical content renders quickly
- Metrics are **aggregated differently** across real-user measurement populations, and the chosen aggregation (median, p75, p95) changes the story
- Lab metrics and field metrics frequently disagree because lab conditions cannot reproduce the full diversity of real-world conditions

## The Critical Rendering Path

The critical rendering path describes the sequence of steps between receiving an HTML document and rendering pixels on screen.

```
HTML bytes → Parse HTML → DOM tree
                              ↓
CSS bytes → Parse CSS → CSSOM tree → Render tree → Layout → Paint → Composite
                              ↑
                        (blocking)
                              ↑
JS bytes → Parse → Compile → Execute (may modify DOM/CSSOM)
```

### Key Dependencies

1. **CSS blocks rendering**: The browser will not paint until it has processed all render-blocking CSS, because painting without styles would produce a flash of unstyled content
2. **JavaScript blocks parsing** (by default): A `<script>` tag pauses HTML parsing until the script downloads, compiles, and executes, because the script might modify the document
3. **JavaScript depends on CSSOM**: Scripts may query computed styles, so JavaScript execution waits for preceding stylesheets to load

These dependencies create chains where one resource blocks another:

```
HTML parsing → discovers CSS link → blocks rendering until CSS loads
            → discovers script tag → blocks parsing until script loads and executes
                                     → script waits for CSSOM before running
```

### Reducing Critical Path Length

Strategies for shortening the critical path revolve around:

- **Minimizing render-blocking resources**: Inlining critical CSS, deferring non-critical CSS
- **Minimizing parser-blocking scripts**: Using `async` or `defer` attributes, moving scripts to document end
- **Reducing round trips**: Server push, preloading critical resources, reducing redirect chains
- **Minimizing bytes on the critical path**: Compression, minification, removing unused code

The critical rendering path is primarily a loading-performance concept, but understanding it also explains certain runtime behaviors (style recalculation after DOM modifications follows a similar pipeline).

## Resource Loading Strategies

How and when resources load significantly affects both loading performance and runtime behavior.

### Eager Loading

Load everything as soon as possible. The page requests all resources during initial load.

- **Advantages**: Resources are available immediately when needed; simpler mental model
- **Disadvantages**: Increases initial load time; wastes bandwidth for resources the user may never need; higher memory pressure

### Lazy Loading

Defer loading resources until they are needed or about to be needed.

- **Advantages**: Faster initial load; reduced bandwidth consumption; lower initial memory use
- **Disadvantages**: Resources unavailable if user navigates to them before loading completes; can cause visible loading states; adds implementation complexity

Lazy loading works well for below-the-fold images, secondary routes in single-page applications, and features accessed by small percentages of users.

### Prefetching

Speculatively load resources that will likely be needed soon, based on heuristics or user behavior patterns.

- **Advantages**: Resources may be available by the time they are needed, providing an instant experience
- **Disadvantages**: Wastes bandwidth if predictions are wrong; may compete with current-page resources for bandwidth; cache eviction may remove prefetched resources before use

### Preloading

Explicitly prioritize specific resources that the browser would otherwise discover late in the loading waterfall (fonts referenced in CSS, scripts loaded by other scripts, images referenced in CSS).

- **Advantages**: Moves critical resources earlier in the loading timeline
- **Disadvantages**: Excessive preloading dilutes the benefit; preloading non-critical resources can delay critical ones

### Priority Signals

Browsers assign fetch priorities to resources based on type and location. Mechanisms exist to hint that a resource is more or less important than the default priority for its type, allowing developers to fine-tune the loading order without changing the markup structure.

### Trade-off Summary

| Strategy | Initial Load         | Subsequent Navigation   | Bandwidth         | Complexity |
| -------- | -------------------- | ----------------------- | ----------------- | ---------- |
| Eager    | Slower               | Fast (cached)           | Higher            | Lower      |
| Lazy     | Faster               | May show loading states | Lower             | Moderate   |
| Prefetch | Neutral              | Potentially instant     | Speculative waste | Higher     |
| Preload  | Targeted improvement | N/A                     | Minimal waste     | Moderate   |

The appropriate strategy depends on usage patterns, network conditions, and the relative importance of initial vs. subsequent interactions.

## Code Splitting and Tree Shaking

### Code Splitting

The principle of dividing application code into chunks that can be loaded independently. Instead of delivering one large bundle containing all application logic, the build process produces multiple smaller bundles loaded on demand.

**Entry-point splitting**: Each page or route gets its own bundle containing only the code it needs.

**Dynamic splitting**: Code is split at points where the application conditionally loads functionality (feature flags, user roles, lazy-loaded routes).

**Vendor splitting**: Third-party dependencies are separated from application code, leveraging the fact that vendor code changes less frequently and can be cached longer.

The granularity tension: too few chunks means downloading unused code; too many chunks means excessive HTTP requests and lost compression efficiency (compression works better on larger files). The optimal split point depends on the HTTP protocol version, caching strategy, and application structure.

### Tree Shaking

The principle of eliminating dead code — modules or exports that are imported but never used. Build tools analyze the dependency graph statically and remove unreachable code paths.

Tree shaking depends on:

- **Static module structure**: ES modules with static `import`/`export` enable analysis; CommonJS `require()` with dynamic paths defeats it
- **Side-effect declarations**: Modules must declare whether they have side effects (code that runs at import time), because the build tool cannot safely remove a module that might have side effects
- **Export granularity**: Barrel files that re-export everything from a directory can prevent tree shaking if the tool cannot determine which re-exports are unused

In practice, tree shaking eliminates varying amounts of code depending on library design. Libraries that export many small functions with no side effects are highly shakeable. Libraries that rely on class hierarchies, global registrations, or init-time side effects are resistant to tree shaking.

## Runtime Performance

### Layout Thrashing

Layout thrashing occurs when JavaScript repeatedly reads layout properties and writes DOM changes in an interleaved pattern, forcing the browser to recalculate layout multiple times within a single frame.

```
// Conceptual thrashing pattern
for each element:
    read element.offsetHeight    // forces layout recalculation
    write element.style.height   // invalidates layout
    // next read forces ANOTHER recalculation
```

The fix in principle: batch all reads together, then batch all writes together. This allows the browser to calculate layout once rather than once per element.

### Long Tasks

Any JavaScript execution that occupies the main thread for more than 50ms is considered a "long task." During a long task, the browser cannot:

- Process user input events
- Run requestAnimationFrame callbacks
- Perform time-critical rendering updates

Sources of long tasks include: large DOM manipulations, expensive computations, synchronous layout calculations, large script parsing and compilation, and deep call stacks in framework lifecycle methods.

Strategies for breaking up long tasks:

- Yielding to the browser between chunks of work (via scheduling APIs, setTimeout, or message channels)
- Moving computation to web workers
- Reducing the scope of work (virtualization, pagination, incremental processing)
- Deferring non-essential work until the browser is idle

### Memory Leaks

In garbage-collected environments, memory leaks manifest as objects that remain reachable but are no longer needed. Common patterns:

| Leak Pattern                | Mechanism                                                                                            |
| --------------------------- | ---------------------------------------------------------------------------------------------------- |
| Detached DOM trees          | JavaScript holds a reference to a DOM node that has been removed from the document                   |
| Forgotten event listeners   | Listeners registered on long-lived objects that reference short-lived closures                       |
| Accumulated closures        | Closures that capture large scopes and are stored in growing collections                             |
| Interval/timeout references | Timers that hold references to objects that should be garbage collected                              |
| Orphaned observers          | IntersectionObserver, MutationObserver, ResizeObserver not disconnected when their target is removed |

Memory leaks degrade runtime performance progressively — the application starts fast but slows as garbage collection must sweep increasingly large heaps.

### Animation and Frame Timing

Smooth visual updates require consistent frame delivery. At 60fps, each frame has approximately 16.7ms. The frame budget must accommodate:

```
Frame budget (16.7ms) = JavaScript + Style + Layout + Paint + Composite
```

If any frame exceeds its budget, the user perceives a "dropped frame" or stutter. Strategies for maintaining frame consistency:

- **Compositor-only properties**: Transforms and opacity can be animated on the compositor thread, bypassing layout and paint entirely
- **Will-change hints**: Inform the browser that an element will be animated, allowing it to promote the element to its own compositor layer
- **requestAnimationFrame**: Synchronize visual updates with the browser's rendering cycle rather than using arbitrary timers

Over-promoting elements to compositor layers consumes GPU memory and can itself cause performance problems — the trade-off is between CPU cost (layout/paint) and GPU cost (layer composition).

## Performance Budgets

A performance budget is an engineering constraint — a defined threshold for performance metrics that the team commits to not exceeding.

### Types of Budgets

| Budget Type   | Example                           | Measured At  |
| ------------- | --------------------------------- | ------------ |
| Size budget   | JS bundle < 200 KB compressed     | Build time   |
| Timing budget | Largest contentful paint < 2.5s   | Lab or field |
| Count budget  | < 50 requests on initial load     | Lab          |
| Score budget  | Lighthouse performance score > 90 | Lab          |
| Custom budget | Main thread blocked < 300ms total | Lab or field |

### Budget Enforcement Points

- **Build pipeline**: Fail the build if bundle sizes exceed thresholds
- **CI/CD**: Run synthetic performance tests and gate deployment on results
- **Monitoring**: Alert when field metrics drift beyond budget
- **Code review**: Review changes that add significant dependencies or assets

### Trade-offs of Strict Budgets

- **Benefits**: Prevents gradual performance degradation; makes performance a first-class engineering concern; creates clear decision criteria for adding dependencies
- **Costs**: May slow development velocity; budgets chosen arbitrarily can frustrate engineers; single metrics can be gamed without improving actual user experience
- **Calibration difficulty**: Setting budgets too tight creates friction; setting them too loose provides no meaningful constraint

## Synthetic vs. Real-User Measurement

### Synthetic (Lab) Measurement

Runs performance tests in controlled environments — fixed hardware, network conditions, and configurations.

**Strengths**:

- Reproducible — same test produces similar results
- Available before deployment (CI/CD integration)
- Captures detailed traces for debugging
- Can simulate specific conditions (slow 3G, low-end CPU)

**Limitations**:

- Does not represent the diversity of real user conditions
- Cannot capture interaction-based metrics (no real user interactions)
- Environment may not match actual user demographics
- Results may not correlate with field data

### Real-User Measurement (RUM)

Collects performance data from actual users via browser APIs (Performance Observer, Navigation Timing, Resource Timing).

**Strengths**:

- Reflects actual user experience across all device/network combinations
- Captures interaction metrics from real usage patterns
- Reveals geographic and demographic performance variation
- Identifies long-tail performance problems that lab tests miss

**Limitations**:

- Noisy data requiring statistical analysis
- Not available before deployment
- Privacy considerations for data collection
- Limited diagnostic detail compared to lab traces
- Requires sufficient traffic volume for statistical significance

### Complementary Use

| Question                                           | Best Source                       |
| -------------------------------------------------- | --------------------------------- |
| "Did this change improve performance?"             | Synthetic (controlled comparison) |
| "How fast is the site for real users?"             | RUM (actual conditions)           |
| "Why is this page slow?"                           | Synthetic (detailed traces)       |
| "Which user segments experience poor performance?" | RUM (demographic segmentation)    |
| "Will this pass our performance budget?"           | Synthetic (CI/CD gating)          |
| "What is our p95 latency?"                         | RUM (tail distribution)           |

Neither measurement type alone provides a complete picture. Disagreement between lab and field metrics is common — investigating the gap often reveals important insights about real-world conditions the lab did not model.

## The Perception Gap

Perceived performance and measured performance frequently diverge. Users do not experience milliseconds — they experience progress, responsiveness, and predictability.

### Factors That Improve Perceived Performance

- **Progress indicators**: Showing that something is happening reduces perceived wait time, even if actual duration is unchanged
- **Skeleton screens**: Displaying page structure before content arrives creates an impression of immediate loading
- **Optimistic UI**: Reflecting user actions instantly (before server confirmation) eliminates perceived latency for the common success case
- **Prioritized content rendering**: Showing above-the-fold content first, even if the page is not fully loaded, aligns technical completion with user perception
- **Consistent frame timing**: Smooth 30fps animation can feel better than stuttery 60fps because consistency matters more than peak framerate

### Factors That Worsen Perceived Performance Despite Good Metrics

- **Layout shifts**: Even fast-loading pages feel broken when content jumps around
- **Delayed interactivity**: A page that looks loaded but does not respond to clicks creates frustration disproportionate to the actual delay
- **Unexpected loading states**: Content that appears and then disappears or changes creates a sense of unreliability
- **Audio/video stalls**: Media buffering is perceived as more severe than equivalent static-content loading delays

### Implications for Optimization

Purely metric-driven optimization can miss opportunities to improve user experience through perception management. Conversely, perception tricks without underlying performance work create fragile experiences — skeleton screens that stay visible for 10 seconds do not fool anyone.

The most effective performance work addresses both: reduce actual latency AND manage perception for remaining latency.

## Caching as a Performance Lever

Caching appears at every layer of the web stack and is among the highest-impact performance mechanisms.

| Cache Layer          | Location          | Typical Content                      | Invalidation                          |
| -------------------- | ----------------- | ------------------------------------ | ------------------------------------- |
| Browser memory cache | In-process        | Recently fetched subresources        | Page navigation                       |
| Browser disk cache   | Local filesystem  | Resources with appropriate headers   | Expiration headers, cache size limits |
| Service worker cache | Local (Cache API) | Explicitly cached resources          | Application-controlled                |
| CDN cache            | Edge servers      | Static and sometimes dynamic content | TTL, purge APIs                       |
| Application cache    | Server memory     | Computed results, database queries   | Application-controlled                |
| DNS cache            | OS / router / ISP | Domain-to-IP mappings                | TTL                                   |

Cache effectiveness depends on hit rates, which depend on:

- **Content stability**: Static assets with content-hashed filenames have effectively infinite cache lifetimes
- **Cache key design**: Overly specific keys reduce hit rates; overly broad keys serve stale content
- **Eviction policy**: Caches with limited capacity evict entries — hot content stays, long-tail content churns
- **Invalidation correctness**: Serving stale content is a functional bug, not just a performance concern

## Compression

Reducing the byte size of transferred resources directly improves loading performance. The three primary dimensions:

**Transport compression**: Compressing resources during HTTP transfer. Different algorithms offer different compression-ratio-to-speed trade-offs. Higher compression reduces bytes transferred but increases CPU time on both server and client.

**Asset optimization**: Reducing the intrinsic size of resources — minifying code, optimizing images, subsetting fonts, removing metadata. These optimizations are typically applied at build time with no runtime cost.

**Responsive sizing**: Serving different resource variants based on client context — different image resolutions for different screen sizes, different code bundles for different browser capabilities. This avoids transferring bytes the client cannot use.

## Network Protocol Considerations

The HTTP protocol version affects performance characteristics significantly:

| Characteristic         | HTTP/1.1               | HTTP/2             | HTTP/3                |
| ---------------------- | ---------------------- | ------------------ | --------------------- |
| Connections per origin | Multiple (6-8 typical) | Single multiplexed | Single multiplexed    |
| Head-of-line blocking  | Per connection         | At TCP layer       | Eliminated (QUIC)     |
| Header compression     | None (or minimal)      | HPACK              | QPACK                 |
| Server push            | Not available          | Available          | Available (less used) |
| Connection setup       | TCP + TLS (2-3 RTTs)   | Same as HTTP/1.1   | 0-1 RTT (QUIC)        |

These differences change optimization strategies:

- **HTTP/1.1**: Bundling, domain sharding, and sprite sheets reduce request count to work within connection limits
- **HTTP/2+**: Many small files may perform as well as or better than bundled files, because multiplexing eliminates the per-request connection cost
- **HTTP/3**: Eliminates TCP head-of-line blocking, benefiting lossy network conditions (mobile, Wi-Fi)

Optimization strategies appropriate for one protocol version may be counterproductive for another.

## Performance as an Ongoing Practice

Performance is not a feature that is implemented once. It is a property of the system that degrades naturally as features are added, dependencies grow, and content expands.

Sustaining performance requires:

- **Measurement infrastructure** that surfaces regressions quickly
- **Budget enforcement** that prevents gradual degradation
- **Performance-aware culture** where engineers consider the performance cost of changes
- **Regular auditing** to identify and address accumulated slow-downs
- **Realistic testing conditions** that approximate the experience of performance-sensitive user segments

The most common failure mode is not a single catastrophic change but "death by a thousand cuts" — each individual addition is small, but their cumulative effect is significant. Performance budgets and automated measurement exist specifically to counter this pattern.
