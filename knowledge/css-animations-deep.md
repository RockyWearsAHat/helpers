# CSS Animations Deep Dive

## Fundamentals: @keyframes and Animation Properties

CSS animations are defined by two parts: a `@keyframes` rule declaring intermediate states, and `animation-*` properties applying that sequence to elements.

```css
@keyframes slide {
  0% { transform: translateX(0); }
  50% { transform: translateX(100px); opacity: 0.8; }
  100% { transform: translateX(200px); opacity: 1; }
}

.box {
  animation: slide 2s ease-in-out 0.5s infinite alternate;
}
```

The `animation` shorthand combines:
- `animation-name`: references the `@keyframes` rule
- `animation-duration`: total length (ms or s)
- `animation-timing-function`: easing curve
- `animation-delay`: pause before start
- `animation-iteration-count`: number of repetitions (number or `infinite`)
- `animation-direction`: `normal`, `reverse`, `alternate`, `alternate-reverse`
- `animation-fill-mode`: `none`, `forwards`, `backwards`, `both` (how to style outside animation)
- `animation-play-state`: `running` or `paused`

### Keyframe Selectors

Keyframes can use percentages (0%, 50%, 100%) or keyword `from` (= 0%) and `to` (= 100%). If no 0% or 100% keyframe is defined, the browser creates implicit ones using the element's initial/final computed styles.

```css
@keyframes fade {
  from { opacity: 0; }
  to { opacity: 1; }
}
```

You can specify multiple property changes per keyframe. Properties not explicitly set at a keyframe interpolate across the animation timeline using the animation's timing function.

## Timing Functions and Cubic-Bezier Curves

Easing determines **how fast** an animation progresses through time. Timing functions control acceleration and deceleration.

### Predefined Timing Functions

- **`linear`**: Constant speed throughout
- **`ease`** (default): Slow start, fast middle, slow end (~= cubic-bezier(0.25, 0.1, 0.25, 1))
- **`ease-in`**: Slow start, accelerates (cubic-bezier(0.42, 0, 1, 1))
- **`ease-out`**: Fast start, decelerates (cubic-bezier(0, 0, 0.58, 1))
- **`ease-in-out`**: Slow start and end (cubic-bezier(0.42, 0, 0.58, 1))

### Cubic-Bezier

`cubic-bezier(x1, y1, x2, y2)` defines a curve using two control points on a unit square (0 to 1 horizontally, time; 0 to 1 vertically, progress). The control points *steer* how animation progresses.

```css
animation-timing-function: cubic-bezier(0.17, 0.67, 0.83, 0.67);  /* "spring-like" overshoot */
```

- x-axis: time (always 0 to 1)
- y-axis: animation progress (can exceed 0-1, creating overshoot)

Overshooting y-values (< 0 or > 1) create "bounce" or "spring" effects. Tools like [cubic-bezier.com](https://cubic-bezier.com) visualize curves.

### Steps Timing

`steps(n, <jump-term>)` divides animation into `n` discrete frames, useful for sprite sheets or looping behaviors:

```css
animation-timing-function: steps(12, end);  /* 12 discrete frames; jump happens at end of interval */
```

Jump terms: `start`, `end`, `jump-start`, `jump-end`, `jump-none`, `jump-both`. Legacy: `step-start` (= steps(1, start)), `step-end` (= steps(1, end)).

## Performance: GPU Compositing and Compositor vs Main Thread

Browser rendering pipeline for animations:
1. **Style recalculation** → **Layout** → **Paint** → **Composite**

Only certain properties can be accelerated to the **compositor** (GPU), avoiding main thread work:

### Compositor-Safe Properties
- `transform` (translate, rotate, scale, skew, perspective)
- `opacity`
- `filter`

These properties bypass layout/paint, animating on the GPU. Pure-compositor animations stay smooth even under JavaScript pressure.

### Main Thread Properties
- `top`, `left`, `width`, `height`, `margin`, `padding` — trigger layout recalculation
- `background-color`, `color`, `border-color` — trigger paint
- `display`, `position`, `overflow` — cause layout thrashing

Animating these forces the browser to recalculate layout for every frame, creating jank at 60fps and below. Do NOT animate these properties.

## Will-Change and Performance Hints

```css
.animated {
  will-change: transform;
  animation: spin 1s linear infinite;
}
```

`will-change: <property>` signals the browser to optimize rendering for that property. Overuse creates performance overhead (browsers allocate resources for each will-change). Use sparingly:
- Add before expensive animations
- Remove after animation completes

## Web Animations API (WAAPI)

WAAPI provides JavaScript control over animations with granular timing and event handling. More powerful than CSS, but heavier than declarative CSS.

### Basic Usage

```javascript
element.animate([
  { transform: 'translateX(0)', opacity: 1 },
  { transform: 'translateX(100px)', opacity: 0.5 },
  { transform: 'translateX(200px)', opacity: 1 }
], {
  duration: 2000,
  easing: 'ease-in-out',
  iterations: Infinity,
  direction: 'alternate'
});
```

Returns an `Animation` object:
```javascript
const animation = element.animate(keyframes, options);
animation.play();
animation.pause();
animation.cancel();
animation.playbackRate = 2;  // 2x speed
console.log(animation.currentTime);  // ms elapsed
```

### WAAPI vs CSS Animations

| Feature | CSS | WAAPI |
|---------|-----|-------|
| Syntax | Declarative, simple | Imperative, verbose |
| Performance | Best (browser optimizes) | Good (same rendering pipeline) |
| Control | Limited (play-state, delay) | Full (pause, seek, adjust rate) |
| Events | animationstart, -end, -iteration | play, pause, finish events |
| Composability | Awkward (multiple rules) | Seamless (chain via `.then()`) |
| Browser support | Universal | ~95% modern browsers |

Use CSS for simple, repeated animations (hover effects, loading spinners). Use WAAPI when you need runtime control or orchestration (pause/resume on user action, conditional animations).

## Scroll-Driven Animations (@scroll-timeline)

`@scroll-timeline` and `animation-timeline` link animation progress to scroll position, enabling parallax, reveal-on-scroll, and progress indicators without JavaScript.

```css
@scroll-timeline scroll-on-y {
  source: auto;  /* auto = nearest scroll container */
  orientation: vertical;
  scroll-offsets: 0%, 100%;  /* when scroll reaches start/end of container */
}

.parallax {
  animation: moveX linear;
  animation-timeline: scroll-on-y;
}

@keyframes moveX {
  0% { transform: translateX(0); }
  100% { transform: translateX(200px); }
}
```

Animation progress (0% → 100%) is tied to scroll progress along the timeline's source. Scroll to 50% = animation at 50%.

**Scroll offsets** define the trigger zone:
- `0%` = when scroll container scrolls into view (scroll-position: 0%)
- `100%` = when element fully exits container (scroll-position: 100%)
- Can use `px` units too: `scroll-offsets: 200px 600px`

Browser support is emerging (Chrome 115+, Firefox behind flag). Polyfills exist but are heavy.

## View Transitions API

Provides seamless animated transitions between DOM state changes (SPA navigation, theme switches, modals):

```javascript
// Single-document view transitions (same-origin)
document.startViewTransition(() => {
  updateDOM();  // Change DOM state
});
```

The browser:
1. Captures a screenshot of the current view
2. Executes your DOM update
3. Captures the new view
4. Animates between old → new screenshot with cross-fade + subtle zoom

Customize the transition with CSS:

```css
::view-transition-old(fade-out) {
  animation: fade-out 0.5s ease-out forwards;
}

::view-transition-new(fade-in) {
  animation: fade-in 0.5s ease-in forwards;
}

@keyframes fade-out {
  to { opacity: 0; }
}

@keyframes fade-in {
  from { opacity: 0; }
}
```

Browser support: Chrome 111+, limited in others. Gracefully degrades (DOM updates immediately, no animation).

## Motion Path and Offset

`offset-path` animates elements along arbitrary paths (lines, curves, polygons):

```css
.element {
  offset-path: path('M 0 0 L 100 50 Q 150 100 200 50');
  animation: followPath 2s ease-in-out;
}

@keyframes followPath {
  0% { offset-distance: 0%; }
  100% { offset-distance: 100%; }
}
```

Properties:
- `offset-path`: SVG path, `ray()`, or `url(#path-id)`
- `offset-distance`: How far along the path (0% → 100%)
- `offset-anchor`: Where on the element attaches to the path (default: center)
- `offset-rotate`: auto | reverse | angle

SVG `<path>` syntax:
- `M x y`: Move to
- `L x y`: Line to
- `C x1 y1 x2 y2 x y`: Cubic Bezier curve
- `Q x1 y1 x y`: Quadratic curve
- `A rx ry ...`: Arc

## Reduced Motion: Accessibility and Performance

Users with vestibular disorders, motion sensitivity, or on low-power devices need reduced animation. Detect with `prefers-reduced-motion` media query:

```css
@media (prefers-reduced-motion: reduce) {
  * {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}

.button {
  transition: background 0.3s ease;
}

@media (prefers-reduced-motion: reduce) {
  .button {
    transition: background 0.01ms linear;
  }
}
```

Or remove animations entirely:

```css
@media (prefers-reduced-motion: reduce) {
  .animated { animation: none; }
}
```

JavaScript detection:
```javascript
const prefersReduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
if (!prefersReduced) {
  startExpensiveAnimation();
}
```

This is not optional—accessibility standards (WCAG 2.1 Animation from Interactions guideline) and user OS settings demand respecting this preference.

## Common Patterns and Gotchas

### Restart on Repeated Trigger

Triggering an animation via class change won't replay if animation-iteration-count is finite:

```javascript
// This doesn't replay the animation
el.classList.add('animate');
el.classList.remove('animate');
el.classList.add('animate');  // Second add: animation already ran
```

Solution: Insert a reflow to reset animation-play-state or use WAAPI's `cancel()` → `play()`.

### Chaining Animations

CSS doesn't compose animations. WAAPI does:

```javascript
element.animate(...).finished.then(() => {
  element.animate(...).play();
});
```

Or use `animationend` event in CSS:

```javascript
el.addEventListener('animationend', () => {
  el.classList.add('next-animation');
});
```

### Animation-Delay and Performance

`animation-delay` delays the start but occupies timeline resource. Large delays waste memory. For one-time delays, use `setTimeout`:

```javascript
setTimeout(() => {
  el.classList.add('animate');
}, 500);
// Better than animation-delay: 500ms on :load
```

### Transform-Origin with Animations

Rotating or scaling respects `transform-origin`. Default is center (50%, 50%):

```css
.spin {
  transform-origin: top-left;
  animation: rotate 1s linear infinite;
}

@keyframes rotate {
  100% { transform: rotate(360deg); }
}
```

## Summary

CSS animations provide declarative, GPU-accelerated motion by connecting `@keyframes` to timing curves and animation properties. Advanced features (scroll-timeline, view-transitions, motion-path) enable complex interactions without heavy JavaScript. Always animate compositor-safe properties (transform, opacity), respect `prefers-reduced-motion`, and debug with DevTools to ensure 60fps smoothness. For orchestration and runtime control, use WAAPI.