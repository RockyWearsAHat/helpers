# Common Lisp Best Practices

## Common Lisp Philosophy

Common Lisp is the most full-featured Lisp dialect — a multi-paradigm, dynamically typed language with macros, CLOS (object system), conditions/restarts, and an interactive development model. It's the "programmable programming language."

- **Code is data**: Homoiconicity enables macros that transform code at compile time.
- **Interactive development**: REPL + SLIME/Sly. Build programs incrementally, redefine functions live.
- **Multi-paradigm**: Functional, imperative, OOP (CLOS), and logic programming — all in one language.

## Core Forms

```lisp
;; Variables
(defvar *global-var* 42)           ; dynamic/special variable (earmuffs convention)
(defparameter *config* "default")  ; like defvar but always re-assigns
(defconstant +pi+ 3.14159)        ; constant (plus signs convention)

;; Let bindings (lexical scope)
(let ((x 10)
      (y 20))
  (+ x y))

;; Sequential binding
(let* ((x 10)
       (y (* x 2)))
  y)  ;=> 20

;; Functions
(defun factorial (n)
  "Compute factorial of N."
  (if (<= n 1)
      1
      (* n (factorial (1- n)))))

;; Lambda
(mapcar (lambda (x) (* x x)) '(1 2 3 4 5))
;=> (1 4 9 16 25)

;; Multiple return values
(defun divide (a b)
  (values (floor a b) (mod a b)))

(multiple-value-bind (quotient remainder) (divide 17 5)
  (format t "~A remainder ~A~%" quotient remainder))
```

## Lists and Sequences

```lisp
;; Cons cells — the building block
(cons 1 (cons 2 (cons 3 nil)))  ;=> (1 2 3)

;; List operations
(car '(1 2 3))    ;=> 1
(cdr '(1 2 3))    ;=> (2 3)
(first '(1 2 3))  ;=> 1  (same as car, more readable)
(rest '(1 2 3))   ;=> (2 3)
(nth 2 '(a b c))  ;=> C

;; Higher-order functions
(mapcar #'1+ '(1 2 3))                    ;=> (2 3 4)
(remove-if-not #'evenp '(1 2 3 4 5 6))   ;=> (2 4 6)
(reduce #'+ '(1 2 3 4 5))                ;=> 15

;; Loop macro (powerful iteration)
(loop for i from 1 to 10
      when (evenp i)
      collect (* i i))
;=> (4 16 36 64 100)

(loop for line = (read-line stream nil nil)
      while line
      count (search "ERROR" line) into errors
      finally (return errors))
```

## Macros

```lisp
;; Macros transform code at compile time
(defmacro when-let ((var expr) &body body)
  "Bind VAR to EXPR, execute BODY only if non-nil."
  `(let ((,var ,expr))
     (when ,var
       ,@body)))

(when-let (user (find-user id))
  (format t "Found: ~A~%" (user-name user)))

;; Macro with gensym (avoid variable capture)
(defmacro with-timing (&body body)
  (let ((start (gensym "START")))
    `(let ((,start (get-internal-real-time)))
       (prog1 (progn ,@body)
         (format t "Elapsed: ~,3f seconds~%"
                 (/ (- (get-internal-real-time) ,start)
                    internal-time-units-per-second))))))

;; Usage
(with-timing
  (heavy-computation))
```

## CLOS (Common Lisp Object System)

```lisp
;; Classes
(defclass user ()
  ((name   :initarg :name   :accessor user-name   :type string)
   (age    :initarg :age    :accessor user-age     :type integer)
   (email  :initarg :email  :reader   user-email   :type string))
  (:documentation "A user account."))

(defmethod print-object ((u user) stream)
  (print-unreadable-object (u stream :type t)
    (format stream "~A (~D)" (user-name u) (user-age u))))

;; Generic functions and methods (multiple dispatch)
(defgeneric area (shape)
  (:documentation "Compute the area of a shape."))

(defclass circle ()
  ((radius :initarg :radius :accessor circle-radius)))

(defclass rectangle ()
  ((width  :initarg :width  :accessor rect-width)
   (height :initarg :height :accessor rect-height)))

(defmethod area ((c circle))
  (* pi (expt (circle-radius c) 2)))

(defmethod area ((r rectangle))
  (* (rect-width r) (rect-height r)))

;; Method combinations
(defmethod area :before ((shape t))
  (format t "Computing area...~%"))
```

## Conditions and Restarts

```lisp
;; Conditions (like exceptions, but more powerful)
(define-condition file-not-found (error)
  ((path :initarg :path :reader file-not-found-path))
  (:report (lambda (c stream)
             (format stream "File not found: ~A" (file-not-found-path c)))))

;; Signal with restarts
(defun read-config (path)
  (restart-case
      (if (probe-file path)
          (with-open-file (s path) (read s))
          (error 'file-not-found :path path))
    (use-default ()
      :report "Use default configuration"
      *default-config*)
    (try-another-file (new-path)
      :report "Try a different file"
      :interactive (lambda () (list (read-line *query-io*)))
      (read-config new-path))))

;; Handler can invoke restart without unwinding the stack
(handler-bind
    ((file-not-found
       (lambda (c)
         (declare (ignore c))
         (invoke-restart 'use-default))))
  (read-config "/missing/config.lisp"))
```

## Packages

```lisp
(defpackage :myapp
  (:use :cl)
  (:export :start
           :stop
           :*config*))

(in-package :myapp)

(defvar *config* nil)

(defun start ()
  (format t "Starting application~%"))

;; ASDF system definition (build system)
;; myapp.asd
(asdf:defsystem :myapp
  :version "1.0.0"
  :depends-on (:alexandria :cl-json :dexador)
  :components ((:file "package")
               (:file "config" :depends-on ("package"))
               (:file "main" :depends-on ("config"))))
```

## Key Rules

1. **Use `let` for lexical bindings, `defvar`/`defparameter` for globals.** Earmuffs (`*var*`) for dynamic variables.
2. **Macros are a last resort.** Use functions first. Only write a macro when a function can't express the abstraction (new syntax, control flow).
3. **Use `gensym` in macros** to prevent variable capture. Always.
4. **Conditions and restarts** separate error policy from error recovery. Use them instead of raw handler-case for recoverable situations.
5. **Prefer LOOP or ITERATE** for complex iteration. `mapcar`/`reduce` for simple transforms.
6. **Use Quicklisp** for package management. It's the de facto standard.

---

*Sources: Practical Common Lisp (Peter Seibel), Common Lisp HyperSpec, On Lisp (Paul Graham), ANSI Common Lisp Standard*
