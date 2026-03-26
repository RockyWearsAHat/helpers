# Scheme & Racket Conventions and Idioms

## Scheme/Racket Philosophy

Scheme is a minimalist Lisp dialect — a small core of powerful primitives. Racket extends Scheme into a "language-oriented programming language" with a rich ecosystem, typed dialects, and the ability to create custom languages.

- **Minimalism (Scheme)**: A few special forms + first-class functions + proper tail calls = everything.
- **Language-oriented (Racket)**: Create domain-specific languages with `#lang`. The platform for PL research.
- **Hygiene**: Macros are hygienic by default — no accidental variable capture.

## Scheme Core

```scheme
;; Functions
(define (square x) (* x x))
(define (factorial n)
  (if (<= n 1) 1 (* n (factorial (- n 1)))))

;; Lambda
(define double (lambda (x) (* x 2)))
(map double '(1 2 3 4 5))  ;=> (2 4 6 8 10)

;; Let bindings
(let ((x 10)
      (y 20))
  (+ x y))

;; Named let (loop pattern)
(let loop ((i 0) (sum 0))
  (if (> i 100)
      sum
      (loop (+ i 1) (+ sum i))))

;; Tail recursion (guaranteed optimization in Scheme)
(define (factorial-tail n)
  (define (aux n acc)
    (if (<= n 1)
        acc
        (aux (- n 1) (* acc n))))
  (aux n 1))

;; Higher-order functions
(define (compose f g)
  (lambda (x) (f (g x))))

(define inc-and-double (compose double add1))
(inc-and-double 5)  ;=> 12
```

## Racket Specifics

```racket
#lang racket

;; Pattern matching
(match expr
  [(list 'add a b) (+ a b)]
  [(list 'mul a b) (* a b)]
  [(? number? n)   n]
  [_ (error "unknown expression")])

;; Structs (immutable by default)
(struct point (x y) #:transparent)
(struct circle point (radius) #:transparent)

(define p (point 3 4))
(point-x p)  ;=> 3

;; match with structs
(define (area shape)
  (match shape
    [(circle _ _ r)  (* pi r r)]
    [(rect _ _ w h)  (* w h)]))

;; Hash tables
(define config
  (hash 'host "localhost"
        'port 8080
        'debug #t))
(hash-ref config 'port)  ;=> 8080

;; For loops (comprehensions)
(for/list ([i (in-range 1 11)]
           #:when (even? i))
  (* i i))
;=> (4 16 36 64 100)

(for/hash ([k '(a b c)]
           [v '(1 2 3)])
  (values k v))
;=> #hash((a . 1) (b . 2) (c . 3))

(for/fold ([sum 0])
          ([n (in-range 1 101)])
  (+ sum n))
;=> 5050
```

## Macros (Hygienic)

```racket
;; syntax-rules (pattern-based, fully hygienic)
(define-syntax when
  (syntax-rules ()
    [(_ test body ...)
     (if test (begin body ...) (void))]))

(define-syntax while
  (syntax-rules ()
    [(_ condition body ...)
     (let loop ()
       (when condition
         body ...
         (loop)))]))

;; syntax-parse (Racket's more powerful macro system)
(require (for-syntax syntax/parse))

(define-syntax (define-struct/validate stx)
  (syntax-parse stx
    [(_ name:id (field:id ...) #:validate validator:expr)
     #'(begin
         (struct name (field ...) #:transparent)
         (define (make-name field ...)
           (unless (validator field ...)
             (error 'name "validation failed"))
           (name field ...)))]))
```

## Continuations

```scheme
;; call/cc — first-class continuations (Scheme's unique power)
;; Escape continuation (like throw)
(define (find-first pred lst)
  (call/cc
    (lambda (return)
      (for-each (lambda (x)
                  (when (pred x) (return x)))
                lst)
      #f)))

(find-first even? '(1 3 5 4 7))  ;=> 4

;; Racket: limited continuations (more practical)
(define (safe-divide a b)
  (with-handlers ([exn:fail:contract:divide-by-zero?
                   (lambda (e) +inf.0)])
    (/ a b)))
```

## Contracts (Racket)

```racket
;; Runtime contracts for function boundaries
(provide
  (contract-out
    [deposit  (-> account? positive? account?)]
    [withdraw (-> account? positive? (or/c account? #f))]
    [balance  (-> account? (>=/c 0))]))

;; Custom contracts
(define positive? (and/c number? positive?))
(define non-empty-string? (and/c string? (not/c string=? "")))

;; Function contracts inline
(define/contract (factorial n)
  (-> exact-nonnegative-integer? exact-positive-integer?)
  (if (zero? n) 1 (* n (factorial (sub1 n)))))
```

## Typed Racket

```racket
#lang typed/racket

(: factorial (-> Integer Integer))
(define (factorial n)
  (if (<= n 1) 1 (* n (factorial (- n 1)))))

;; Union types
(: safe-sqrt (-> Real (U Real Complex)))
(define (safe-sqrt x)
  (if (>= x 0) (sqrt x) (sqrt (- x))))

;; Polymorphic functions
(: my-map (All (A B) (-> (-> A B) (Listof A) (Listof B))))
(define (my-map f lst)
  (if (null? lst)
      '()
      (cons (f (car lst)) (my-map f (cdr lst)))))
```

## Modules and Testing

```racket
;; Module system
(module+ main
  ;; Runs when file is executed directly
  (displayln (factorial 10)))

(module+ test
  ;; Runs with `raco test`
  (require rackunit)
  (check-equal? (factorial 0) 1)
  (check-equal? (factorial 5) 120)
  (check-equal? (factorial 10) 3628800))
```

## Conventions

1. **Tail recursion is mandatory for loops.** Scheme guarantees TCO. Use named `let` or accumulator patterns.
2. **Use `match` (Racket) for destructuring.** Pattern matching is cleaner than nested `car`/`cdr`.
3. **Hygienic macros by default.** Use `syntax-rules` or `syntax-parse`. Avoid `define-syntax-rule` for complex macros.
4. **Contracts at module boundaries (Racket).** Use `contract-out` in `provide` forms for public APIs.
5. **Use `#lang typed/racket`** for projects where type safety matters. Gradual typing works well.
6. **Prefer immutable data.** Use `struct` (immutable by default), immutable hash tables, and functional updates.

---

_Sources: The Scheme Programming Language (R. Kent Dybvig), SICP (Abelson & Sussman), Racket Documentation (docs.racket-lang.org), How to Design Programs (Felleisen et al.)_
