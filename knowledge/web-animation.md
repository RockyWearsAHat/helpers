# Web Animation Techniques & Performance

## Overview

Animation is the art of creating motion in the browser. Techniques range from declarative CSS (transitions, keyframes) to procedural JavaScript (requestAnimationFrame, physics libraries). Each technique has different performance characteristics, flexibility, and browser support. Performance-conscious animation prioritizes GPU compositing, avoiding layout thrashing, and respecting user motion preferences.

## CSS Transitions & Animations

**Transitions** animate from current state to target state: `transition: all 0.3s ease-in-out`. Triggered by pseudo-class (hover, focus) or JavaScript class change.

**Animations** define keyframes and repeat: `@keyframes slide { 0% { transform: translateX(0); } 100% { transform: translateX(100px); } }`, then `animation: slide 1s ease`.

**Advantages**: Declarative, no JavaScript overhead, GPU accelerated (if animating transform/opacity), works everywhere, built into CSS.

**Disadvantages**: Limited to CSS properties, no physics (duration/easing only), hard to interrupt mid-animation cleanly, can't inspect animation state from JavaScript, no callback until animation end.

**Performance**: Only certain properties are GPU accelerated (transform, opacity). Animating width, height, left, top causes layout recalculation every frame — expensive. Animating transform (2D/3D) is cheap.

**Easing**: `linear`, `ease-in`, `ease-out`, `ease-in-out`, `cubic-bezier()` for custom. Pre-defined easings don't approximate physics — they're arbitrary curves.

## Web Animations API (WAAPI)

Browser API for animations (`Element.animate()`). More flexible than CSS, more performant than JavaScript.

```javascript
element.animate(
  [{ transform: 'translateX(0px)' }, { transform: 'translateX(100px)' }],
  { duration: 300, easing: 'ease-in-out' }
);
```

**Advantages**: Programmatic control (pause, play, reverse, seek), state inspection, callbacks, no JavaScript every frame, GPU accelerated.

**Disadvantages**: API is verbose compared to CSS, less browser support (IE11) than CSS transitions, less widely adopted (most teams still reach for CSS or libraries).

**Motion.dev enhancements**: Provides spring physics, scroll-linked animations, improved API ergonomics. Sits on top of WAAPI, not a replacement.

## requestAnimationFrame (RAF)

Callback that fires before each browser repaint (~60fps on 60Hz screens). Core of all JavaScript animations.

```javascript
function animate() {
  element.style.transform = `translateX(${progress}px)`;
  if (progress < 100) requestAnimationFrame(animate);
}
animate();
```

**Pros**: Fine-grained control, can animate any value, responsive to screen refresh rate, can inspect and respond to state.

**Cons**: Requires manual loop management, easy to cause layout thrashing (read then write DOM properties), ties animation to render cycle (frame budgets matter).

**Layout thrashing**: `width = element.offsetWidth + 1; setInterval(() => { element.offsetWidth; element.style.width = ... })` reads, then writes, in loop. Browser must recalculate layout on each iteration. Slow.

**Best practice**: Batch reads, then batch writes. Measure all, then update all.

## FLIP Technique

FLIP = **F**irst, **L**ast, **I**nvert, **P**lay.

Solves a hard problem: animating elements whose final layout is unknown. Classic example: list item that moves when list reorders. Item's current position is known, final position is unknown until re-layout happens.

**Flow**:
1. **First**: Record element's current position and size.
2. Perform (mutate DOM, react re-renders).
3. **Last**: Calculate element's new position/size after layout.
4. **Invert**: Apply `transform` to move element back to its "First" position (inverting the difference).
5. **Play**: Animate `transform` to 0 (identity), element slides from old to new position.

**Code pattern**:
```javascript
const first = element.getBoundingClientRect();
// Mutate DOM
list.appendChild(item); // Or re-order
const last = element.getBoundingClientRect();
const deltaX = first.left - last.left;
const deltaY = first.top - last.top;
element.style.transform = `translate(${deltaX}px, ${deltaY}px)`;
element.style.transition = 'transform 0.3s';
element.offsetHeight; // Trigger reflow
element.style.transform = '';
```

**Advantages**: Animates re-ordering, layout changes, and size changes smoothly. Feels like items flow into new positions.

**Libraries**: Framer Motion's `layoutId` implements FLIP implicitly. React spring can leverage it. Shopify Polaris has FLIP helpers.

**When to use**: List reordering, filter animations, reveal/collapse, any "layout change becoming motion" scenario.

## Spring Physics

Animations governed by physics (mass, stiffness, damping) feel natural — they overshoot, oscillate, and settle rather than tweening linearly.

**Mental model**: Imagine a spring with a mass attached. You change the target (where the spring is attached). The mass accelerates toward target, overshoots, oscillates, damping friction gradually settles it.

**Parameters**: `mass`, `stiffness`, `damping`. Higher stiffness = snappier. Higher damping = less bounce. Common presets: "gentle", "bouncy", "smooth".

**Advantage over tweening**: No duration needed. Spring time is determined by physics—naturally feels responsive. Users can't tell the exact duration, but it *feels right*.

**Libraries**: Framer Motion (`whileHover={{ scale: 1.1 }}`), React Spring (`useSpring()`), GSAP (`quickSetter` with physics), Motion for React (built-in).

**Performance**: If implemented in JavaScript + RAF, springs add math overhead but still performant (simple physics). If backed by WAAPI, very performant.

**Trade-off**: Unpredictable timing (harder to sync with other animations), new concept for teams used to duration-based animations, but results are often more delightful.

## GSAP

GreenSock Animation Platform. Mature, feature-rich animation library. Targets any DOM property, SVG, Canvas, Web Audio.

```javascript
gsap.to(element, { duration: 1, x: 100, rotation: 360, ease: 'back.out' });
```

**Pros**: Extremely flexible, excellent performance, mature (used in industry since early 2000s), extensive plugin ecosystem (morph, split text, inertia, draw SVG paths, etc.), timeline for complex sequences, great documentation.

**Cons**: Not React-idiomatic (imperative), large library (~150kb), license model (free for most uses, commercial for plugins). Can feel like overkill for simple transitions.

**Niche**: Complex animations, SVG animation, timeline-based sequencing, games, creative studios. Not common in modern React apps but extremely capable.

## Framer Motion

React animation library. Declarative, motion components, gesture-aware, spring physics built-in.

```jsx
<motion.div animate={{ x: 100 }} transition={{ type: 'spring' }} />
```

**Features**: Layout animations (FLIP-backed), gesture animations (drag, hover, tap), shared layout animations (e.g., images morph between pages), Variants API for coordinated animations.

**Pros**: React-native API, excellent DX, springs by default, handles layout changes automatically. Widely used in modern React apps.

**Cons**: Large bundle (~40kb), somewhat opinionated (Framer's affordances), can feel slow on large component trees if overused.

**When to use**: React apps prioritizing animation experience, teams with animation-heavy requirements, design systems that animate.

## Lottie

Plays animations exported from Adobe After Effects as JSON files (via Bodymovin plugin). Browser-agnostic format.

**Workflow**: Animator creates animation in After Effects → Bodymovin exports JSON → Lottie player renders JSON → integrates into web/iOS/Android.

**Use cases**: Loaders, illustrations, complex motion design assets, brand animations.

**Pros**: Animators design in familiar tool, no frame-by-frame code needed, vector-based (scales), light files.

**Cons**: Not interactive (plays linearly), not responsive to user input without extra work, heavy for what amounts to a video (JSON + player overhead). Can't use Lottie to animate app interactions.

**Niche**: Design asset animation, not interaction animation.

## GPU Compositing & will-change

Modern browsers render pages in layers. Some layers are composited (combined) into the final image. Layers that move/scale without re-rendering content are cheap (GPU operation).

**Transform and opacity** are GPU-friendly. Changing `transform` or `opacity` doesn't trigger layout recalculation. Changing anything else (width, height, left, top, color) requires browser to recalculate layout/paint and re-composite.

**will-change CSS property** hints to browser: "This property will animate, optimize it." 

```css
.animated { will-change: transform, opacity; }
```

**Caution**: `will-change` has overhead (creates new stacking context, allocates memory). Use sparingly. Remove via JavaScript after animation ends.

**Performance audit**: Open DevTools → Performance tab → record → play animation → check "Rendering" track. If you see layout/paint events every frame, you're animating wrong properties. If only composite, you're optimized.

## Reduced Motion Accessibility

`prefers-reduced-motion` media query respects user OS preference to reduce motion (for accessibility or battery life).

```css
@media (prefers-reduced-motion: prefer-reduced) {
  * { animation: none !important; transition: none !important; }
}
```

**User perspective**: Busy animations cause dizziness or migraines for some. Disable by default in macOS Accessibility → Display → Reduce motion (or Windows equivalent).

**Team responsibility**: Test with `prefers-reduced-motion: reduce` in DevTools. Disable animations for these users. Offer instant transitions instead.

**Implementation**: Wrap animations in media query or check via JavaScript: `window.matchMedia("(prefers-reduced-motion: reduce)").matches`.

## Animation Performance Budgets

Animation should run at 60fps (16.67ms per frame) on target devices. Budget per frame: 

- 1-2ms: Animation math + state updates
- 10ms: Style calculations, layout, paint
- 3-4ms: Composite + rasterize
- 1-2ms: Browser overhead

If you spend 12ms on animations + layout, you have only 4ms before frame drops.

**Optimization strategies**:
1. Use `transform` and `opacity` (no layout cost)
2. Batch DOM reads/writes (prevent layout thrashing)
3. Test on low-end devices (older phones)
4. Use DevTools profiler to identify bottlenecks
5. Consider reducing animation scope (animate fewer elements, shorter duration)

## When to Use Which

**Simple hover feedback**: CSS transitions (`transition: all 0.2s`).

**Complex sequences, timelines**: GSAP. Framer Motion if React.

**Physics-based feel**: Spring libraries (Framer Motion, React Spring).

**Layout changes**: FLIP (implicit in Framer Motion).

**Asset/brand animation**: Lottie.

**Fine-grained control, unusual properties**: RAF + requestAnimationFrame.

**Interactive animations (drag, gesture-responsive)**: Framer Motion's gesture system.

## References

- Web Animations API: https://developer.mozilla.org/en-US/docs/Web/API/Web_Animations_API
- prefers-reduced-motion: https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion
- FLIP technique: https://aerotwist.com/blog/flip-your-animations/
- Framer Motion: https://www.framer.com/motion/
- GSAP: https://gsap.com/
- Lottie: https://airbnb.io/lottie/
- React Spring: https://www.react-spring.dev/
- Motion for React: https://motion.dev/

See also: [web-performance.md](web-performance.md), [framework-react.md](framework-react.md)