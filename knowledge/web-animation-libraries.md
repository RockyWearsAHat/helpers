# Web Animation Libraries: GSAP, Framer Motion, and Alternatives

## Overview

Web animation libraries span a spectrum: **timeline-based** (GSAP, anime.js) orchestrate complex sequences; **physics-based** (react-spring, Framer Motion with Reanimated) simulate natural motion; **design-to-code** (Lottie) bridge After Effects and web. Framework integration, performance characteristics, and ergonomics vary significantly. Trade-offs: GSAP excels at control and performance; Framer Motion dominates React ecosystems; Lottie is lightweight for pre-rendered graphics.

## GSAP (GreenSock Animation Platform)

### Core Strengths

**GSAP** is framework-agnostic, production-proven JavaScript animation engine. It prioritizes performance (requestAnimationFrame optimization, efficient DOM updates) and developer experience.

- **Timeline orchestration**: sequence multiple tweens with precise timing
- **Selector API**: shorthand tweenings (`gsap.to(".class", ...)`)
- **Easing library**: 30+ built-in easing functions
- **Plugin ecosystem**: ScrollTrigger (scroll-linked animations), Draggable, MotionPath, morphSVG, Text plugin

### Architecture

```javascript
// Basic tween (single animation)
gsap.to(".box", { 
  duration: 1, 
  x: 100, 
  opacity: 0.5,
  ease: "power2.out"
});

// Timeline: sequence tweens
const tl = gsap.timeline();
tl.to(".box1", { duration: 1, x: 100 })
  .to(".box2", { duration: 1, x: 100 }, 0.5)  // offset 0.5s (overlap)
  .addLabel("midpoint")
  .to(".box1", { duration: 0.5, rotation: 360 }, "midpoint");

// Reverse, seek, play with full timeline control
tl.reverse();
tl.seek(0.5);
```

### Plugin: ScrollTrigger

Links animations to scroll position:

```javascript
gsap.registerPlugin(ScrollTrigger);

gsap.to(".element", {
  scrollTrigger: {
    trigger: ".section",
    start: "top center",
    end: "bottom center",
    scrub: 1,  // tie animation to scroll (1 = 1s smoothing)
    markers: true,  // debug
    onEnter: () => console.log("entered")
  },
  x: 500,
  duration: 1
});
```

### Performance

GSAP uses high-precision internal calculations but batches DOM updates for efficiency. Animate `transform` (translate, scale, rotate) and `opacity` for GPU acceleration; animating width/height triggers layout recalculations. Typical performance: 60 FPS on 100+ simultaneous tweens (depends on browser, target properties).

### Ecosystem & Trade-offs

**Strengths**: precise control, timeline orchestration, ecosystem (ScrollTrigger especially dominates scroll-linked animation), excellent docs, paid support available.

**Weaknesses**: proprietary (free for most use; licensing for some features); steep learning curve for complex sequences; not React-native (requires DOM selectors or refs).

## Framer Motion

### Philosophy

Framer Motion prioritizes **declarative React-first animation**. Animations are component props, not imperative sequences. Targets React developers building interactive UIs with natural motion.

```jsx
import { motion } from "framer-motion";

<motion.div
  animate={{ x: 100, opacity: 1 }}
  transition={{ duration: 0.5, ease: "easeInOut" }}
  initial={{ x: 0, opacity: 0 }}
/>
```

### Key Features

**Variants**: abstract animation definitions, applied via `animate` prop:

```jsx
const variants = {
  hidden: { opacity: 0, y: -10 },
  visible: { opacity: 1, y: 0, transition: { duration: 0.5 } }
};

<motion.ul initial="hidden" animate="visible" variants={variants}>
  {items.map(item => (
    <motion.li key={item.id} variants={variants}>{item}</motion.li>
  ))}
</motion.ul>
```

**Layout animations**: re-layout components smoothly when DOM changes:

```jsx
<motion.div layout>
  {expanded ? <ExpandedContent /> : <CollapsedContent />}
</motion.div>
```

**AnimatePresence**: animate components exiting/entering:

```jsx
<AnimatePresence>
  {isOpen && (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
    />
  )}
</AnimatePresence>
```

**Gesture animations**: automatic response to user interaction:

```jsx
<motion.button
  whileHover={{ scale: 1.1 }}
  whileTap={{ scale: 0.95 }}
  onClick={() => setIsOpen(!isOpen)}
/>
```

### Physics & Spring Animations

Framer Motion supports spring physics (alternatives to duration-based easing):

```jsx
<motion.div
  animate={{ x: 100 }}
  transition={{ type: "spring", stiffness: 100, damping: 10 }}
/>
```

Spring parameters: stiffer = faster, damping = oscillation decay. Feels more organic than easing functions.

### Performance & Limitations

Framer Motion uses **Reanimated 2** (on React Native) or **Waapi** (browser) under the hood for non-DOM-blocking animation. GPU-accelerated for transforms/opacity. Trade-off: strongly tied to React lifecycle; harder to use with vanilla JS or outside React.

## anime.js

Lightweight alternative to GSAP; focuses on simplicity and animation primitives.

```javascript
anime({
  targets: '.box',
  translateX: 250,
  duration: 800,
  easing: 'easeInOutQuad'
});

// Timeline
const timeline = anime.timeline();
timeline
  .add({ targets: '.box1', translateX: 100, duration: 500 })
  .add({ targets: '.box2', translateX: 100, duration: 500 }, '-=250');
```

**Strengths**: small footprint (~10KB), clean API, good for vanilla JS, strong TypeScript types (recent).

**Weaknesses**: no scroll-linked animation plugin (must build custom), smaller ecosystem, less mature plugin system.

## Motion (Successor to Framer Motion)

Framer Motion's modern successor, still in development, aims for framework-agnostic animation:

```javascript
// motion (not Framer Motion)
const MotionComponent = motion(HTMLElement);
```

Still evolving; Framer Motion remains the React standard for now.

## Lottie: After Effects to Web

Lottie bridges design and code. Designers export vector animations from After Effects as JSON (via lottie-web or Bodymovin plugin), developers render them on web with a single component:

```javascript
import Lottie from 'lottie-react';
import animationData from './animation.json';

<Lottie animationData={animationData} />;
```

**Strengths**: pixel-perfect animations designed in familiar tools, file sizes can be smaller than video for complex sequences, scalable (vector).

**Weaknesses**: static once exported (limited interactivity); JSON files can be large for complex animations; After Effects timeline doesn't always translate smoothly to web semantics.

Common use: loaders, illustrated narratives, complex character animations.

## react-spring

Physics-based animation for React, using spring metaphors and Reanimated:

```jsx
import { useSpring, animated } from 'react-spring';

function AnimatedBox() {
  const spring = useSpring({
    from: { opacity: 0, x: -20 },
    to: { opacity: 1, x: 0},
    config: { tension: 120, friction: 14 }
  });
  
  return <animated.div style={spring}>Spring Box</animated.div>;
}
```

**Philosophy**: springs feel natural; no easing functions, just physics parameters. Strong for interactive motion.

**Trade-offs**: smaller community than Framer Motion; learning curve steeper (must grok spring physics); less out-of-box gesture support.

## Comparative Performance and Ergonomics

| Library | Framework | GPU Accel | Timeline | Spring Physics | Scroll Integration | Size | Learning |
|---------|-----------|-----------|----------|----------------|-------------------|------|----------|
| GSAP | Agnostic | Yes* | Excellent | No | ScrollTrigger (best) | 38KB | Moderate |
| Framer Motion | React | Yes (Reanimated) | Good (variants) | Yes | Manual | 60KB | Easy |
| anime.js | Agnostic | Yes* | Good | No | None | 10KB | Easy |
| Lottie | Agnostic | N/A (SVG/Canvas) | N/A | N/A | N/A | Varies | Easy |
| react-spring | React | Yes (Reanimated) | Limited | Excellent | Manual | 30KB | Hard |

*GSAP and anime.js benefit from hardware acceleration when animating transform/opacity, but not automatic like Reanimated.

## When to Choose Each

- **GSAP**: Complex timelines, scroll-linked animations, marketing sites, frameworks-agnostic projects, maximum control
- **Framer Motion**: React-first UI animation, gesture-driven apps, layout shifts, prototyping, design-to-code collaboration
- **anime.js**: Lightweight scripts, learning animation fundamentals, modern browsers without React dependency
- **Lottie**: Designer-made animations, loaders, illustrations, consistency across platforms (web + mobile)
- **react-spring**: Interactive, physics-based motion, apps requiring natural feel, when overkill of Framer Motion not justified

Modern choice: **Framer Motion** for React SPA UI (strong momentum, broad ecosystem); **GSAP** for scroll-driven marketing and complex timelines; **anime.js** for lightweight alternatives.