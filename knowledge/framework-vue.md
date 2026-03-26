# Vue

## Reactivity System

Vue 3 uses **Proxy-based reactivity**. When you access a reactive property, Vue tracks which effect (render function, computed, watcher) is reading it. When the property changes, Vue re-runs those tracked effects.

```js
// Two reactive primitives
const count = ref(0); // wraps value, access via .value
const state = reactive({ x: 1, y: 2 }); // deep reactive Proxy

// ref auto-unwraps in templates: {{ count }} not {{ count.value }}
// ref auto-unwraps inside reactive objects: state.count = ref(0); state.count === 0
```

| API                 | Use case                      | Unwrapping                       | Destructuring                   |
| ------------------- | ----------------------------- | -------------------------------- | ------------------------------- |
| `ref()`             | Primitives, single values     | `.value` in JS, auto in template | Loses reactivity (use `toRefs`) |
| `reactive()`        | Objects, arrays               | Direct access                    | Loses reactivity (use `toRefs`) |
| `shallowRef()`      | Large objects, manual trigger | `.value`                         | N/A                             |
| `shallowReactive()` | Top-level only reactivity     | Direct access                    | Loses reactivity                |
| `readonly()`        | Immutable reactive copy       | Direct access                    | Read-only                       |

### Reactivity Internals

```
reactive(obj) → new Proxy(obj, {
  get(target, key) {
    track(target, key);  // record dependency
    return target[key];
  },
  set(target, key, value) {
    target[key] = value;
    trigger(target, key); // notify watchers
    return true;
  }
})
```

Vue batches DOM updates using microtasks. Multiple synchronous state changes result in a single DOM update. Use `nextTick()` to read the DOM after updates.

**Reactivity caveats:**

- Adding new properties to a `reactive` object IS tracked (Proxy-based, unlike Vue 2's Object.defineProperty)
- Array index mutation IS tracked
- Map, Set, WeakMap, WeakSet are supported
- Destructuring `reactive` loses reactivity — use `toRefs(state)` or `toRef(state, 'key')`

## Composition API

```vue
<script setup>
import { ref, computed, watch, watchEffect, onMounted } from "vue";

const count = ref(0);
const doubled = computed(() => count.value * 2);

// Watch specific source
watch(
  count,
  (newVal, oldVal) => {
    console.log(`changed from ${oldVal} to ${newVal}`);
  },
  { immediate: false, deep: false },
);

// Auto-track dependencies
watchEffect(() => {
  console.log(`count is ${count.value}`); // re-runs when count changes
});

// Lifecycle
onMounted(() => {
  /* DOM ready */
});
</script>
```

### watch vs watchEffect

| Feature   | `watch`                       | `watchEffect`         |
| --------- | ----------------------------- | --------------------- |
| Source    | Explicit (ref, getter, array) | Auto-tracked          |
| Old value | Provided                      | Not available         |
| Lazy      | Yes (default)                 | No (runs immediately) |
| Deep      | Opt-in `{ deep: true }`       | Implicit if accessed  |
| Cleanup   | `onCleanup` param             | `onCleanup` param     |

### provide/inject

```vue
<!-- Parent -->
<script setup>
import { provide, ref } from "vue";
const theme = ref("dark");
provide("theme", theme); // reactive if you provide a ref
</script>

<!-- Deeply nested child -->
<script setup>
import { inject } from "vue";
const theme = inject("theme", "light"); // 'light' is default
</script>
```

## Single File Components

```vue
<script setup lang="ts">
// script setup is syntactic sugar for setup() function
// top-level bindings are exposed to template automatically

import { ref } from "vue";
import ChildComponent from "./Child.vue";

// Props with defaults
const props = withDefaults(
  defineProps<{
    title: string;
    count?: number;
  }>(),
  { count: 0 },
);

// Events with validation
const emit = defineEmits<{
  update: [value: string];
  delete: [id: number];
}>();

// Expose to parent via template ref
defineExpose({
  reset() {
    /* ... */
  },
});

// v-model support
const model = defineModel<string>(); // Vue 3.4+
</script>

<template>
  <div>{{ title }}</div>
</template>

<style scoped>
/* scoped styles use data-v-xxx attribute selector */
/* :deep(.child-class) penetrates child components */
/* :slotted(.slot-class) targets slotted content */
/* :global(.class) escapes scoping */
</style>
```

## Template Features

### Directives

| Directive               | Purpose            | Example                                    |
| ----------------------- | ------------------ | ------------------------------------------ |
| `v-bind` (`:`)          | Dynamic attribute  | `:class="{ active: isActive }"`            |
| `v-on` (`@`)            | Event listener     | `@click.prevent.stop="handler"`            |
| `v-model`               | Two-way binding    | `v-model.trim.number="value"`              |
| `v-if/v-else-if/v-else` | Conditional render | Destroys/creates DOM                       |
| `v-show`                | Toggle visibility  | `display: none` (keeps DOM)                |
| `v-for`                 | List rendering     | `v-for="(item, i) in list" :key="item.id"` |
| `v-slot` (`#`)          | Named slot content | `#header="{ data }"`                       |
| `v-memo`                | Memoize sub-tree   | `v-memo="[item.id === selected]"`          |

### Slots

```vue
<!-- Parent usage -->
<Card>
  <template #header="{ title }">
    <h1>{{ title }}</h1>
  </template>
  <template #default>
    Main content
  </template>
</Card>

<!-- Card.vue -->
<template>
  <div>
    <slot name="header" :title="cardTitle" />
    <slot />
    <!-- default slot -->
  </div>
</template>
```

## Built-in Components

### Teleport

```vue
<Teleport to="#modal-root">
  <Modal v-if="showModal" />
</Teleport>
```

### Transition

```vue
<Transition name="fade" mode="out-in">
  <component :is="currentView" />
</Transition>
<!-- CSS classes: .fade-enter-from, .fade-enter-active, .fade-enter-to -->
<!-- .fade-leave-from, .fade-leave-active, .fade-leave-to -->
```

### KeepAlive

```vue
<KeepAlive :include="['TabA', 'TabB']" :max="10">
  <component :is="activeTab" />
</KeepAlive>
<!-- Cached components get onActivated/onDeactivated hooks -->
```

### Suspense (experimental)

```vue
<Suspense>
  <template #default>
    <AsyncComponent />
  </template>
  <template #fallback>
    <Loading />
  </template>
</Suspense>
```

## Composables Pattern

Composables are functions that encapsulate and reuse stateful logic using Composition API:

```js
// composables/useMouse.js
import { ref, onMounted, onUnmounted } from "vue";

export function useMouse() {
  const x = ref(0);
  const y = ref(0);
  function update(event) {
    x.value = event.pageX;
    y.value = event.pageY;
  }
  onMounted(() => window.addEventListener("mousemove", update));
  onUnmounted(() => window.removeEventListener("mousemove", update));
  return { x, y };
}
```

Conventions: name starts with `use`, returns refs/reactive (not raw values), handles cleanup.

## Pinia State Management

```js
import { defineStore } from "pinia";

export const useCartStore = defineStore("cart", () => {
  // State
  const items = ref([]);

  // Getters (computed)
  const total = computed(() =>
    items.value.reduce((sum, item) => sum + item.price * item.qty, 0),
  );

  // Actions (functions)
  function addItem(product) {
    const existing = items.value.find((i) => i.id === product.id);
    if (existing) existing.qty++;
    else items.value.push({ ...product, qty: 1 });
  }

  return { items, total, addItem };
});
```

Pinia stores are reactive singletons. They support devtools, hot module replacement, plugins, and SSR.

## Vue Router

```js
const routes = [
  { path: "/", component: Home },
  { path: "/user/:id", component: User, props: true }, // route params as props
  {
    path: "/admin",
    component: AdminLayout,
    beforeEnter: [authGuard],
    children: [
      { path: "", component: Dashboard },
      { path: "users", component: UserList, meta: { requiresAdmin: true } },
    ],
  },
  { path: "/:pathMatch(.*)*", component: NotFound }, // catch-all
];
```

Navigation guards execute in order: `beforeEach` (global) → `beforeEnter` (route) → `beforeRouteEnter` (component) → `afterEach` (global).

## Virtual DOM and Patch

Vue's compiler transforms templates into optimized render functions with **patch flags** that tell the runtime exactly what can change:

```
<!-- Template -->
<div>
  <span>Static</span>
  <span>{{ dynamic }}</span>
</div>

<!-- Compiled with patch flags -->
// The static span is hoisted — never diffed
// The dynamic span gets TEXT patch flag — only text content is compared
```

This makes Vue's runtime faster than React's full-tree diffing for template-based code since the compiler provides hints.
