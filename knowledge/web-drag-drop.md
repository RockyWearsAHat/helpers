# Drag and Drop API — HTML5 DnD, DataTransfer, Sortable Lists, Accessibility & Libraries

## Overview

The HTML5 Drag and Drop API enables users to drag DOM elements and drop them into designated zones. Native browser support (>95%) makes it an alternative to custom mouse-based drag implementations. Core concepts: `draggable` attribute, drag events (dragstart, dragover, drop), DataTransfer object for payload exchange, drop zones, visual feedback. Production applications use libraries (dnd-kit, react-beautiful-dnd, SortableJS) for complex scenarios. This note covers the native API, common patterns (sortable lists, kanban boards, file drop), accessibility, and trade-offs between vanilla vs library approaches.

## Core Concepts: Draggable & Drop Zone

### Draggable Elements

```html
<div draggable="true" id="item-1">Drag me</div>
<div draggable="true" id="item-2">Drag me too</div>

<div id="drop-zone" style="border: 2px dashed gray; padding: 20px; min-height: 100px;">
  Drop items here
</div>
```

The `draggable="true"` attribute makes an element draggable. By default, images, links, and selected text are draggable (no attribute needed).

### Drag Event Lifecycle

```
┌─ User presses mouse on draggable element
│
└─ dragstart
   ├─ (user moves mouse with button pressed)
   │
   └─ dragenter (hovering over potential drop target)
      ├─ dragover (moving within target; fires repeatedly)
      │
      ├─ (if move out) dragleave
      │
      └─ (if drop) drop
         │
         └─ dragend (always fires, even if drop failed)
```

### Event Methods & Properties

```javascript
// dragstart: user initiates drag
element.addEventListener('dragstart', (e) => {
  e.dataTransfer.effectAllowed = 'move' // or 'copy', 'link'
  e.dataTransfer.setData('text/plain', element.id)
  e.dataTransfer.setData('application/json', JSON.stringify({id: element.id, type: 'item'}))
})

// dragover: moving over drop target (prevent default to allow drop)
dropZone.addEventListener('dragover', (e) => {
  e.preventDefault() // required to allow drop
  e.dataTransfer.dropEffect = 'move'
  dropZone.classList.add('drag-over') // visual feedback
})

// dragleave: left the target
dropZone.addEventListener('dragleave', (e) => {
  dropZone.classList.remove('drag-over')
})

// drop: user releases over target
dropZone.addEventListener('drop', (e) => {
  e.preventDefault()
  const itemId = e.dataTransfer.getData('text/plain')
  const item = document.getElementById(itemId)
  dropZone.appendChild(item)
  dropZone.classList.remove('drag-over')
})

// dragend: drag finished (success or cancelled)
element.addEventListener('dragend', (e) => {
  // cleanup; e.dataTransfer.dropEffect tells if drop was successful
})
```

## DataTransfer Object

The `DataTransfer` object holds the drag payload and controls visual feedback. Available in **dragstart, drag, dragenter, dragover, dragleave, drop, dragend** events (not all events for all data types).

### Setting & Getting Data

```javascript
// In dragstart
e.dataTransfer.setData('text/plain', 'some text')
e.dataTransfer.setData('text/html', '<p>html content</p>')
e.dataTransfer.setData('application/json', JSON.stringify({id: 123}))

// In drop
const plainText = e.dataTransfer.getData('text/plain')
const json = e.dataTransfer.getData('application/json')

// Check available types
console.log(e.dataTransfer.types) // ['text/plain', 'text/html', 'application/json']
```

**Custom MIME types are allowed:**
```javascript
e.dataTransfer.setData('application/x-my-app/item-id', '42')
```

**Files (special handling):**
```javascript
// In drop event
const files = e.dataTransfer.files // FileList object
for (const file of files) {
  console.log(file.name, file.type, file.size)
}
```

### Visual Feedback

```javascript
// Drag image (appears under cursor during drag)
const dragImage = new Image()
dragImage.src = 'item.png'
e.dataTransfer.setDragImage(dragImage, 10, 10) // offset: 10px from cursor

// Drop effect (visual cursor)
e.dataTransfer.dropEffect = 'move' // 'copy', 'link', or 'none'
e.dataTransfer.effectAllowed = 'move' // limits allowed drop effects
```

If `dropEffect` doesn't match `effectAllowed`, the drop is not allowed (browser shows "no-drop" cursor).

## Common Patterns

### Sortable List

```html
<ul id="items">
  <li draggable="true" data-id="1">Item 1</li>
  <li draggable="true" data-id="2">Item 2</li>
  <li draggable="true" data-id="3">Item 3</li>
</ul>
```

```javascript
let draggedItem = null

document.querySelectorAll('#items li').forEach((item) => {
  item.addEventListener('dragstart', (e) => {
    draggedItem = item
    item.style.opacity = '0.5'
    e.dataTransfer.effectAllowed = 'move'
  })
  
  item.addEventListener('dragend', (e) => {
    item.style.opacity = '1'
    draggedItem = null
  })
  
  item.addEventListener('dragover', (e) => {
    e.preventDefault() // required
    e.dataTransfer.dropEffect = 'move'
    
    // Visual indicator: swap with hovered item
    if (item !== draggedItem) {
      item.parentNode.insertBefore(draggedItem, item)
    }
  })
})
```

### File Drop Zone

```html
<div id="dropzone" style="border: 2px dashed; padding: 40px; text-align: center;">
  Drag files here
</div>
```

```javascript
const dropzone = document.getElementById('dropzone')

// Prevent default browser behavior (opening file)
dropzone.addEventListener('dragover', (e) => {
  e.preventDefault()
  dropzone.style.backgroundColor = '#f0f0f0'
})

dropzone.addEventListener('dragleave', () => {
  dropzone.style.backgroundColor = 'white'
})

dropzone.addEventListener('drop', async (e) => {
  e.preventDefault()
  dropzone.style.backgroundColor = 'white'
  
  const files = e.dataTransfer.files
  for (const file of files) {
    if (file.type.startsWith('image/')) {
      // Process image
      const reader = new FileReader()
      reader.onload = (event) => {
        const img = document.createElement('img')
        img.src = event.target.result
        dropzone.appendChild(img)
      }
      reader.readAsDataURL(file)
    }
  }
})
```

### Kanban Board (Multi-Zone Drag-Drop)

```html
<div class="kanban">
  <div class="column" data-status="todo">
    <h3>To Do</h3>
    <div class="card" draggable="true" data-id="card-1">Task 1</div>
    <div class="card" draggable="true" data-id="card-2">Task 2</div>
  </div>
  
  <div class="column" data-status="in-progress">
    <h3>In Progress</h3>
  </div>
  
  <div class="column" data-status="done">
    <h3>Done</h3>
  </div>
</div>
```

```javascript
const cards = document.querySelectorAll('.card')
const columns = document.querySelectorAll('.column')

cards.forEach((card) => {
  card.addEventListener('dragstart', (e) => {
    e.dataTransfer.effectAllowed = 'move'
    e.dataTransfer.setData('text/plain', card.dataset.id)
    card.style.opacity = '0.5'
  })
  
  card.addEventListener('dragend', () => {
    card.style.opacity = '1'
  })
})

columns.forEach((column) => {
  column.addEventListener('dragover', (e) => {
    e.preventDefault() // required to allow drop
    e.dataTransfer.dropEffect = 'move'
    column.style.backgroundColor = '#ffffcc'
  })
  
  column.addEventListener('dragleave', () => {
    column.style.backgroundColor = 'white'
  })
  
  column.addEventListener('drop', (e) => {
    e.preventDefault()
    column.style.backgroundColor = 'white'
    const cardId = e.dataTransfer.getData('text/plain')
    const card = document.getElementById(cardId)
    column.appendChild(card)
    
    // Optionally sync to server
    fetch('/api/cards', {
      method: 'PATCH',
      body: JSON.stringify({cardId, status: column.dataset.status})
    })
  })
})
```

## Accessibility & Keyboard Interaction

Native drag-drop is **not keyboard accessible** by default. Draggable elements don't get focus or keyboard support; most screen reader users cannot initiate drag.

### Recommendations

1. **Provide alternative keyboard UI:** arrow keys to reorder, tab to focus, Enter to move.
2. **Announce drag state:** Use `aria-live` regions to inform screen reader users of drops.
3. **Use semantic HTML:** Ensure draggable items are semantically meaningful (not just `<div>`) or add `role` attributes.

```html
<!-- Before -->
<div draggable="true" data-id="item-1">Item</div>

<!-- Better for accessibility -->
<li draggable="true" role="button" tabindex="0" data-id="item-1" aria-label="Item 1">Item</li>
```

```javascript
// Add keyboard support
item.addEventListener('keydown', (e) => {
  if (e.key === 'ArrowDown' && item.nextElementSibling) {
    item.parentNode.insertBefore(item.nextElementSibling, item)
  } else if (e.key === 'ArrowUp' && item.previousElementSibling) {
    item.parentNode.insertBefore(item, item.previousElementSibling)
  }
})

// ARIA live region for announcements
const liveRegion = document.createElement('div')
liveRegion.setAttribute('aria-live', 'polite')
liveRegion.setAttribute('aria-atomic', 'true')
document.body.appendChild(liveRegion)

// On drop, announce
liveRegion.textContent = `Item moved to position 3`
```

Libraries like dnd-kit provide built-in keyboard support; prefer them for accessible drag-drop.

## Drag-and-Drop Libraries

### dnd-kit (React)

Modern, accessible, responsive React library with keyboard support.

```javascript
import { DndContext, closestCenter, KeyboardSensor, PointerSensor, useSensor, useSensors } from '@dnd-kit/core'
import { arrayMove, SortableContext, sortableKeyboardCoordinates, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { useSortable } from '@dnd-kit/sortable'

function SortableItem(props) {
  const { attributes, listeners, setNodeRef, transform } = useSortable({ id: props.id })
  return (
    <div ref={setNodeRef} {...attributes} {...listeners} style={{ transform: `translate3d(${transform?.x}px, ${transform?.y}px, 0)` }}>
      {props.children}
    </div>
  )
}

export function SortableList() {
  const [items, setItems] = useState(['Item 1', 'Item 2', 'Item 3'])
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates })
  )
  
  return (
    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
      <SortableContext items={items} strategy={verticalListSortingStrategy}>
        {items.map((item) => (
          <SortableItem key={item} id={item}>{item}</SortableItem>
        ))}
      </SortableContext>
    </DndContext>
  )
  
  function handleDragEnd(event) {
    const { active, over } = event
    if (over && active.id !== over.id) {
      setItems(arrayMove(items, items.indexOf(active.id), items.indexOf(over.id)))
    }
  }
}
```

**Pros:** Keyboard access, mobile-friendly, tree support, accessible, well-maintained.

### react-beautiful-dnd (React)

Older, batteries-included library with opinionated styling and smooth animations.

```javascript
import { DragDropContext, Droppable, Draggable } from 'react-beautiful-dnd'

export function DragDropList() {
  const [items, setItems] = useState([...])
  
  return (
    <DragDropContext onDragEnd={handleDragEnd}>
      <Droppable droppableId="items">
        {(provided, snapshot) => (
          <div ref={provided.innerRef} {...provided.droppableProps}>
            {items.map((item, index) => (
              <Draggable key={item.id} draggableId={item.id} index={index}>
                {(provided, snapshot) => (
                  <div ref={provided.innerRef} {...provided.dragHandleProps}>
                    {item.name}
                  </div>
                )}
              </Draggable>
            ))}
            {provided.placeholder}
          </div>
        )}
      </Droppable>
    </DragDropContext>
  )
  
  function handleDragEnd(result) {
    const { source, destination, draggableId } = result
    if (destination) {
      // reorder items
    }
  }
}
```

**Pros:** Smooth animations, great UX, mature. **Cons:** No keyboard support (legacy), less active maintenance.

### SortableJS (Vanilla JavaScript)

Pure JavaScript library for sortable lists, no framework required.

```html
<script src="https://cdn.jsdelivr.net/npm/sortablejs@1.15.0/Sortable.min.js"></script>

<ul id="sortable">
  <li>Item 1</li>
  <li>Item 2</li>
  <li>Item 3</li>
</ul>

<script>
  Sortable.create(document.getElementById('sortable'), {
    animation: 150,
    ghostClass: 'blue-background-class',
    onEnd: (evt) => {
      console.log('Item reordered:', evt.oldIndex, '→', evt.newIndex)
    }
  })
</script>
```

**Pros:** Lightweight, framework-agnostic, drag handles, multi-list. **Cons:** Less accessible out-of-box.

## When to Use Native vs Library

| Scenario | Approach |
|----------|----------|
| Simple reorderable list, no keyboard | Native API (50 lines code) |
| Production app with accessibility | dnd-kit (React) or Sortable (vanilla) |
| Mobile-first, smooth animations | react-beautiful-dnd or dnd-kit |
| Cross-framework, self-hosted | SortableJS |
| Complex nested trees, constraints | dnd-kit (best) |

## Browser Support & Limitations

- **Support:** All modern browsers (>95%); IE 10+
- **Mobile:** Limited native support; libraries (dnd-kit, react-beautiful-dnd) add touch/pointer events
- **Cross-browser dragging:** Some features (drag images, certain effects) behave inconsistently; test thoroughly
- **Sandbox restrictions:** Sandboxed iframes cannot access certain drag data for security

## Performance Considerations

- **Drag events fire frequently** (dragover fires ~60 Hz during drag). Avoid heavy computations.
- **Debounce or throttle updates:** Use `requestAnimationFrame` to batch visual updates.
- **Virtual lists:** If list is large (1000+ items), use virtual scrolling (windowing) to only render visible items.

```javascript
// Inefficient (fires per dragover)
dropZone.addEventListener('dragover', (e) => {
  e.preventDefault()
  recalculateLayout() // slow
})

// Better
let frameScheduled = false
dropZone.addEventListener('dragover', (e) => {
  e.preventDefault()
  if (!frameScheduled) {
    frameScheduled = true
    requestAnimationFrame(() => {
      recalculateLayout()
      frameScheduled = false
    })
  }
})
```

## See Also

- `web-webrtc.md` — real-time collaboration (WebRTC + drag-drop in shared apps)
- `accessibility-engineering.md` — accessibility principles and WCAG
- `browser-api-file.md` — File API for dropped files