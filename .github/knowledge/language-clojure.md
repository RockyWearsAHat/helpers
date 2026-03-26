# Clojure Best Practices

## Clojure Philosophy

Clojure is a modern Lisp on the JVM (and JavaScript via ClojureScript). It emphasizes immutability, functional programming, and simplicity. Rich Hickey's "Simple Made Easy" is the guiding design principle.

- **Immutable by default**: All core data structures are persistent and immutable.
- **Data-oriented**: Prefer plain data (maps, vectors) over custom types.
- **REPL-driven development**: Build programs interactively, one expression at a time.

## Core Data Structures (Persistent & Immutable)

```clojure
;; Vectors (indexed)
[1 2 3 4 5]
(conj [1 2 3] 4)       ;=> [1 2 3 4]
(nth [10 20 30] 1)      ;=> 20
(get [10 20 30] 1)      ;=> 20

;; Maps (key-value)
{:name "Alice" :age 30 :email "alice@test.com"}
(assoc m :age 31)       ;=> new map with updated age
(dissoc m :email)       ;=> new map without email
(get m :name)           ;=> "Alice"
(:name m)               ;=> "Alice" (keywords are functions)

;; Sets
#{1 2 3 4 5}
(conj #{1 2 3} 4)      ;=> #{1 2 3 4}
(disj #{1 2 3} 2)      ;=> #{1 3}
(contains? #{1 2 3} 2)  ;=> true

;; Lists (linked, prepend)
'(1 2 3)
(cons 0 '(1 2 3))      ;=> (0 1 2 3)

;; All operations return new collections; originals unchanged
```

## Sequence Operations

```clojure
;; Threading macros for readability
(->> data
     (filter :active?)
     (map :score)
     (remove nil?)
     (sort >)
     (take 10))

;; Thread-first for nested access
(-> response
    :body
    (json/parse-string true)
    :results
    first
    :name)

;; Transducers (composable, no intermediate collections)
(def xf (comp (filter odd?)
              (map #(* % %))
              (take 5)))

(into [] xf (range 100))
;=> [1 9 25 49 81]

;; Reduce
(reduce + 0 [1 2 3 4 5])  ;=> 15

(reduce (fn [acc item]
          (update acc (:category item) (fnil conj []) item))
        {}
        items)
```

## Functions

```clojure
;; Named function
(defn greet
  "Greets a person by name."
  ([name] (greet name "Hello"))
  ([name greeting]
   (str greeting ", " name "!")))

;; Anonymous / lambda
(fn [x] (* x x))
#(* % %)             ; shorthand

;; Destructuring
(defn process-user [{:keys [name age email] :as user}]
  (println name age email))

(defn first-two [[a b & rest]]
  [a b])

;; Higher-order
(defn apply-twice [f x]
  (f (f x)))

(apply-twice inc 5)  ;=> 7
```

## State Management

```clojure
;; Atoms (uncoordinated, synchronous)
(def counter (atom 0))
(swap! counter inc)          ;=> 1
(swap! counter + 10)         ;=> 11
(reset! counter 0)           ;=> 0
@counter                     ;=> 0 (deref)

;; Refs (coordinated, transactional — STM)
(def account-a (ref 100))
(def account-b (ref 200))

(dosync
  (alter account-a - 50)
  (alter account-b + 50))   ;; atomic transfer

;; Agents (uncoordinated, asynchronous)
(def logger (agent []))
(send logger conj "log entry")
```

## Namespaces

```clojure
(ns myapp.core
  (:require [clojure.string :as str]
            [clojure.set :as set]
            [myapp.db :as db]
            [myapp.util :refer [parse-config]]))

;; Use full namespace qualification for clarity
(str/join ", " ["a" "b" "c"])
(db/find-user 42)
```

## Protocols and Records

```clojure
;; Protocols (like interfaces/type classes)
(defprotocol Serializable
  (to-bytes [this])
  (from-bytes [this data]))

;; Records (typed maps with protocol implementation)
(defrecord User [name age email]
  Serializable
  (to-bytes [this]
    (.getBytes (pr-str this)))
  (from-bytes [this data]
    (read-string (String. data))))

;; Records are also maps
(def alice (->User "Alice" 30 "alice@test.com"))
(:name alice)           ;=> "Alice"
(assoc alice :age 31)   ;=> updated User record

;; Prefer plain maps unless polymorphism is needed
```

## Concurrency

```clojure
;; Futures
(let [result (future (expensive-computation))]
  ;; do other work...
  @result)  ;; deref blocks until done

;; core.async (CSP channels)
(require '[clojure.core.async :as async :refer [go chan <! >! <!! >!!]])

(let [ch (chan 10)]
  (go (>! ch "hello"))  ;; put (non-blocking in go block)
  (go (println (<! ch))));; take (non-blocking in go block)

;; Pipeline
(let [in (chan 10)
      out (chan 10)]
  (async/pipeline 4 out (map #(* % %)) in)
  (async/onto-chan! in (range 10))
  (<!! (async/into [] out)))
```

## Key Rules

1. **Prefer data over objects.** Use maps and vectors. Only reach for records/protocols when you need polymorphism.
2. **Keep functions pure.** Side effects at the edges. Pure transformation in the middle.
3. **Use the REPL constantly.** Evaluate forms as you write them. REPL-driven dev is Clojure's superpower.
4. **Destructure aggressively.** Use `:keys`, `:as`, and sequential destructuring for clean function signatures.
5. **Avoid macros unless necessary.** Functions compose; macros don't. Write a macro only when a function can't do the job.
6. **Spec your data boundaries.** Use `clojure.spec` at system edges (API inputs, config) — not internal functions.

---

_Sources: Clojure.org, Programming Clojure (Halloway), Clojure Applied (Vandgrift & Miller), ClojureDocs.org_
