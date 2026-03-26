# Observer & Pub-Sub Patterns — Event Notification, Decoupling & Scalability

The Observer pattern and pub-sub (publish-subscribe) pattern both solve the problem of notifying multiple objects about state changes or events, but they operate at different scales and with different coupling models.

## Observer Pattern (Gang of Four)

**Intent:** Define a one-to-many dependency between objects so that when one object changes state, all dependents are notified automatically.

### Core Structure

**Subject (Publisher)** — Maintains state and a list of observer references. Notifies observers when state changes. Works with observers through a common interface.

**Observer Interface** — Declares a single update method that all observers must implement. Receives notification from the subject.

**Concrete Observers** — Implement the observer interface. React to notifications by performing some action (logging, UI update, side effect).

### Push vs. Pull Models

**Push:** The subject sends changed data to the observer in the notification call:

```java
subject.notify(new StateChangeEvent(subject.getState(), timestamp));
observer.update(event);  // Observer receives data
```

**Pull:** The observer polls the subject for data after being notified:

```java
subject.notify("stateChanged");
observer.update(subject);  // Observer calls back to get state
```

Push is more efficient when observers need different subsets of data; pull is simpler when all observers need the same data. Pull also decouples the data structure from the notification mechanism.

### Coupling via Interfaces

The pattern's power lies in the subject knowing nothing about concrete observer classes. It only knows the observer interface:

```python
class Subject:
    def __init__(self):
        self._observers = []
    
    def attach(self, observer: Observer):
        self._observers.append(observer)
    
    def notify(self):
        for observer in self._observers:
            observer.update()  # Calls interface, not concrete class
```

This allows new observers to be added without modifying the subject.

### Drawbacks of Classic Observer

- **Unordered notifications:** Observers are notified in an undefined order (usually insertion order). If observer B depends on observer A's side effects, bugs result.
- **Memory leaks:** If observers aren't explicitly unsubscribed, they remain referenced and keep themselves and their dependencies alive. Long-lived subjects can accumulate dead observers.
- **No filtering:** Subject broadcasts to all observers regardless of interest. Observers that don't care about a notification waste cycles checking them.

## Node.js EventEmitter

The Node.js core EventEmitter implements the observer pattern with event types as a key insight:

```javascript
const EventEmitter = require('events');

class DataSource extends EventEmitter {
    load(url) {
        fetch(url).then(data => {
            this.emit('data', data);  // Named event
        }).catch(err => {
            this.emit('error', err);
        });
    }
}

const source = new DataSource();
source.on('data', (data) => console.log('Got:', data));
source.on('error', (err) => console.error('Error:', err));
source.load('https://api.example.com/data');
```

Key improvements over naive Observer:

- **Event types:** Subscribe to specific events, not all state changes. Observers can be selective.
- **Multiple listeners:** `.on()` adds listeners; `.once()` adds one-time listeners. Fluent API.
- **Error handling:** `error` events are conventional; if emitted and no listener exists, EventEmitter throws.
- **Unsubscription:** `.off()` or `.removeListener()` explicitly detach listeners. Prevents memory leaks when used correctly.

### Memory Leaks in EventEmitter

Objects holding references to event listeners prevent garbage collection:

```javascript
class Component {
    constructor(emitter) {
        this.emitter = emitter;
        // Binds this to handler; handler holds ref to this
        this.emitter.on('update', () => this.handleUpdate());
    }
    
    destroy() {
        // Must explicitly remove or the component and emitter keep each other alive
        this.emitter.removeAllListeners('update');
    }
}
```

This is particularly dangerous in long-lived servers where components are dynamically created/destroyed.

## RxJS Observables

RxJS Observables extend the pattern with functional composition, backpressure handling, and time-based operations:

```javascript
import { fromEvent, map, debounceTime } from 'rxjs';

const button = document.querySelector('button');
const clicks$ = fromEvent(button, 'click');

clicks$.pipe(
    debounceTime(300),
    map(event => event.clientX)
).subscribe(x => console.log(`Clicked at ${x}`));
```

Key differences from EventEmitter:

- **Functional pipelines:** Operators (map, filter, debounce) transform the stream declaratively.
- **Backpressure:** Subscriptions can signal they're overwhelmed; producers can slow down. Prevents buffer overflows.
- **Lazy evaluation:** Subscriptions don't execute until someone subscribes.
- **Resource management:** Subscriptions return an unsubscribe function, ensuring cleanup.

## Pub-Sub (Publish-Subscribe)

Pub-Sub is a distributed pattern where publishers and subscribers are decoupled by a message broker:

```
Publisher -> Message Broker -> Subscriber 1
                            -> Subscriber 2
                            -> Subscriber 3
```

Key differences from Observer:

- **Topics/Channels:** Messages are published to named topics; subscribers filter by interest.
- **Broker mediation:** Publishers don't know about subscribers; subscribers don't know about publishers. Complete decoupling.
- **Network-friendly:** Works across process/machine boundaries. Messages can be routed through infrastructure (message queues, event buses).
- **Message guarantees:** Different implementations offer at-least-once, exactly-once, or best-effort delivery.

### Examples

**Event Bus (In-Process)**

```python
class EventBus:
    def __init__(self):
        self.subscribers = {}  # topic -> [handlers]
    
    def subscribe(self, topic, handler):
        if topic not in self.subscribers:
            self.subscribers[topic] = []
        self.subscribers[topic].append(handler)
    
    def publish(self, topic, message):
        if topic in self.subscribers:
            for handler in self.subscribers[topic]:
                handler(message)

bus = EventBus()
bus.subscribe("user.created", lambda user: print(f"Email {user}"))
bus.subscribe("user.created", lambda user: print(f"Log {user}"))
bus.publish("user.created", "alice@example.com")  # Triggers both
```

**Message Broker (RabbitMQ, Kafka)**

Publishers post messages to an exchange/topic. Brokers route to queues. Subscribers consume from queues. Decoupling allows producers and consumers to start/stop independently.

## Backpressure and Flow Control

In high-throughput systems, subscribers can be slow. If the publisher produces faster than subscribers consume, buffers fill. Backpressure mechanisms signal the publisher to slow down:

- **Pull-based backpressure:** Subscriber requests N events; producer sends up to N.
- **Watermark backpressure:** Producer pauses when queue depth exceeds a threshold.
- **Circuit breaker backpressure:** If a subscriber fails, the broker stops delivering to it (or retries with exponential backoff).

RxJS handles this elegantly. Raw Observer/pub-sub requires explicit implementation.

## Weak References and Listener Cleanup

In garbage-collected languages, observer patterns risk memory leaks. Some frameworks use weak references for listeners:

```java
// Pseudocode: weak reference to listener
class WeakObserver<T> implements Observer<T> {
    private final WeakReference<Observer<T>> ref;
    
    public void update() {
        Observer<T> observer = ref.get();
        if (observer != null) {
            observer.update();
        } else {
            // Observer was garbage collected; remove this from subscribers
            subject.remove(this);
        }
    }
}
```

This prevents listeners from being retained if the original subscriber is garbage collected. Trade-off: subscription becomes implicit (listeners vanish if you lose the reference to them elsewhere).

## Observer vs. Pub-Sub

| Aspect | Observer | Pub-Sub |
|--------|----------|---------|
| Coupling | Subject knows observers | Completely decoupled via broker |
| Scope | Single process | Local or distributed |
| Scalability | Subject is bottleneck | Broker handles many publishers/subscribers |
| Message delivery | Immediate synchronous | Can be async or queued |
| Filtering | All observers notified | Subscribers filter by topic |
| Use case | Object state changes | Loosely coupled microservices, event buses |

## Anti-patterns and Pitfalls

**Observer explosion:** Too many observers on one subject. Notifications become slow and hard to reason about.

**Circular observer chains:** A updates B, B updates A. Infinite loops if not carefully guarded.

**Silent failures:** Observer update throws; subject silently catches. Difficult to debug when notifications disappear.

**No unsubscription discipline:** Long-running systems accumulate dead listeners. Servers slowly fill memory.

**Order dependency:** Code relies on observer registration order. Fragile; adding a new observer can break existing ones.

## Related Patterns

- **Strategy:** Defers *behavior selection*. Observer notifies about *state changes*.
- **Mediator:** Centralizes communication between peers. Similar to pub-sub but more centralized control.
- **Command:** Encapsulates operations. Observer is about reaction to state.
- **State:** Object changes behavior when state changes. Observer reacts to changes from outside.

## Modern Usage

Observer remains fundamental to UI frameworks (Vue reactivity, React hooks). Pub-sub is endemic to event-driven architectures and microservices. RxJS Observables have become the de facto reactive programming model in JavaScript. Node.js EventEmitter is ubiquitous in streaming and async workflows.

The pattern's core insight—*decoupling sources of events from handlers*—has never been more relevant.