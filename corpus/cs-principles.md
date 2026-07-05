# CS principles — the enforceable canon (machine-global corpus rules)

## no_empty_catch [high]
Never swallow an exception silently. A swallowed error is a hidden bug. Handle the error or rethrow it.

```python:bad
try:
    risky()
except Exception:
    pass
```

```python:good
try:
    risky()
except Exception as e:
    log.error("risky failed: %s", e)
    raise
```

## no_type_equality_compare [medium]
Do not compare types with ==. That comparison is broken for subclasses and hides real bugs. Use isinstance in the check instead.

```python:bad
if type(value) == type(other):
    combine(value, other)
```

```python:good
if isinstance(value, type(other)):
    combine(value, other)
```

## no_mutable_default_argument [high]
Never use a mutable default argument. The one shared default instance leaks state across calls — a classic hidden bug.

```python:bad
def append_item(item, items=[]):
    items.append(item)
    return items
```

```python:good
def append_item(item, items=None):
    if items is None:
        items = list()
    items.append(item)
    return items
```

## no_var_declaration [medium]
Never declare variables with var. Its function-wide hoisting leaks bindings and hides scoping bugs. Declare with const, or let when reassignment is needed.

```javascript:bad
var count = 1;
```

```javascript:good
let count = 1;
```
