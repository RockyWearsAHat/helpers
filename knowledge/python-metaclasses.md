# Python Metaclasses: Type as Metaclass, Descriptor Protocol, and Class Customization

## What Is a Metaclass?

A **metaclass** is the class of a class. Just as an instance is created by calling a class, a class is created by calling its metaclass. The default metaclass is `type`.

```python
class MyClass:
    pass

print(type(MyClass))  # <class 'type'>
print(isinstance(MyClass, type))  # True
print(MyClass.__class__)  # <class 'type'>
```

Everything in Python is an object, including classes. Classes are instances of their metaclass. To customize how a class is created or behaves as a callable, you write a metaclass.

## The Metaclass Protocol: `__new__` and `__init__`

When you define a class, Python calls the metaclass to create it. The metaclass constructor follows this protocol:

```python
class Meta(type):
    def __new__(mcs, name, bases, namespace, **kwargs):
        """Called when the class body is finished, before the class object exists."""
        print(f"Creating class {name}")
        return super().__new__(mcs, name, bases, namespace)

    def __init__(cls, name, bases, namespace, **kwargs):
        """Called after the class object is created."""
        print(f"Initializing class {name}")
        super().__init__(name, bases, namespace)

class MyClass(metaclass=Meta):
    x = 42
```

**Signatures:**
- `__new__(mcs, name, bases, namespace, **kwargs)` — `mcs` is the metaclass itself, `name` is the class name (string), `bases` is the tuple of base classes, `namespace` is the dict of class attributes.
- `__init__(cls, name, bases, namespace, **kwargs)` — `cls` is the newly created class object.

**Key distinction:**
- `__new__` can modify the class creation process and return a different class object.
- `__init__` performs setup after the class exists.

## `__init_subclass__`: Simpler Alternative

For most use cases, you don't need a metaclass. Python 3.6+ provides `__init_subclass__`, which is called whenever a class is subclassed:

```python
class Plugin:
    def __init_subclass__(cls, **kwargs):
        """Called when a subclass is created."""
        super().__init_subclass__(**kwargs)
        print(f"Registered plugin: {cls.__name__}")
        Registry.register(cls)

class MyPlugin(Plugin):
    pass  # __init_subclass__ is called here, even with no metaclass

class AnotherPlugin(Plugin, category="util"):
    pass  # kwargs captured from the class definition
```

**When to use `__init_subclass__`:** Plugin systems, registries, validators, inheritance hierarchies.

**When to use a metaclass:** Global class behavior, class-level descriptors, or intercepting `__call__` to customize instance creation.

## The Descriptor Protocol: `__get__`, `__set__`, `__delete__`

Descriptors are objects that define how attributes are accessed. They power properties, methods, and classmethods.

```python
class Descriptor:
    def __get__(self, obj, objtype=None):
        """Called on attribute access: instance.attr"""
        if obj is None:
            return self  # Class access
        print(f"Getting {self.name} from {obj}")
        return obj.__dict__.get(self.name)

    def __set__(self, obj, value):
        """Called on attribute assignment: instance.attr = value"""
        print(f"Setting {self.name} to {value}")
        obj.__dict__[self.name] = value

    def __delete__(self, obj):
        """Called on attribute deletion: del instance.attr"""
        print(f"Deleting {self.name}")
        del obj.__dict__[self.name]

class User:
    name = Descriptor()
    name.name = "name"  # Store the attribute name

    def __init__(self, name):
        self.name = name

user = User("Alice")
print(user.name)  # Calls __get__
```

**Descriptor binding:**
- **Data descriptors** define both `__get__` and `__set__` (or `__delete__`). They take precedence over instance `__dict__`.
- **Non-data descriptors** define only `__get__`. Instance `__dict__` takes precedence.
- Methods are non-data descriptors — `__func__` is looked up in the class, bound to the instance in `__get__`.

**Common descriptors:**
- `property` — read/write/delete with custom logic
- `classmethod` — binds the class, not the instance
- `staticmethod` — disables binding entirely

## `__slots__`: Memory Optimization

Instance attributes normally live in `obj.__dict__`. `__slots__` preallocates fixed slots, eliminating the dict overhead:

```python
class Point:
    __slots__ = ('x', 'y')

    def __init__(self, x, y):
        self.x = x
        self.y = y

p = Point(1, 2)
p.z = 3  # AttributeError: 'Point' object has no attribute 'z'
```

**Benefits:** ~40% memory savings for instances with few attributes.

**Trade-offs:** No `__dict__`, can't add attributes dynamically. Inheritance from `__slots__` classes requires care — slots don't stack; you must redeclare in subclasses.

**When to use:** Millions of instances (e.g., NumPy arrays, game entities). Not typical for application code.

## `ABCMeta`: Abstract Base Classes

`ABCMeta` is a metaclass that enforces abstract methods:

```python
from abc import ABC, abstractmethod

class Animal(ABC):
    @abstractmethod
    def speak(self):
        pass

class Dog(Animal):
    def speak(self):
        return "Woof"

# Animal()  # TypeError: Can't instantiate abstract class
dog = Dog()
dog.speak()  # "Woof"
```

**ABCMeta is also a class decorator:**

```python
from abc import ABC, abstractmethod

class Animal(ABC):
    @abstractmethod
    def speak(self): ...
```

This is syntactic sugar for `class Animal(metaclass=ABCMeta)`.

**Protocol classes** (Python 3.8+) offer structural subtyping without explicit inheritance:

```python
from typing import Protocol

class Drawable(Protocol):
    def draw(self) -> None: ...

class Circle:
    def draw(self) -> None: print("●")

def render(obj: Drawable):
    obj.draw()

render(Circle())  # Works — Circle implements the protocol
```

## Dataclasses Internals

The `@dataclass` decorator uses the descriptor protocol and class introspection to generate `__init__`, `__repr__`, `__eq__`, etc.:

```python
from dataclasses import dataclass, field

@dataclass
class User:
    name: str
    age: int = 0
    tags: list[str] = field(default_factory=list)
```

This generates code equivalent to:

```python
class User:
    def __init__(self, name: str, age: int = 0, tags: list[str] = None):
        self.name = name
        self.age = age
        self.tags = tags if tags is not None else []

    def __repr__(self): ...
    def __eq__(self, other): ...
```

**Introspection:** Dataclasses store field metadata in `__dataclass_fields__`, enabling PostInit hooks and validation:

```python
@dataclass
class Temperature:
    celsius: float

    def __post_init__(self):
        if self.celsius < -273.15:
            raise ValueError("Below absolute zero")
```

**Advanced:** Use `replace()` to create modified copies, `asdict()` to export, and `astuple()` for tuple views.

## `attrs` Library: Dataclass Alternative

`attrs` predates dataclasses and offers more features via class decoration or class variables:

```python
import attrs

@attrs.define
class User:
    name: str
    age: int = 0
    tags: list[str] = attrs.field(factory=list)

    @property
    def is_adult(self):
        return self.age >= 18

@attrs.frozen  # Immutable
class Point:
    x: float
    y: float

# attrs generates __init__, __repr__, __eq__, __hash__
user = User("Alice", 30)
print(attrs.asdict(user))
```

**attrs advantages over dataclasses:**
- Converters and validators built-in
- Better inheritance and slots support
- Structural equality (not just value equality)
- Instance methods and properties coexist cleanly
- Frozen (immutable) by default option

**Under the hood:** `attrs` uses `__attrs_attrs__` to store field metadata, similar to dataclasses' `__dataclass_fields__`.

## Metaclass Pitfalls and When NOT to Use

**Metaclass conflicts:**
```python
class Meta1(type): pass
class Meta2(type): pass

class A(metaclass=Meta1): pass
class B(metaclass=Meta2): pass

# class C(A, B): pass  # TypeError: metaclass conflict
```

**Solution:** Use `__init_subclass__` or abstract metaclass inheritance instead.

**Readability:** Metaclasses are powerful but obscure. Other developers may not understand your code. Prefer:
- `__init_subclass__` for subclass hooks
- Descriptors for attribute access control
- Decorators for class transformation
- Base classes for shared behavior

**Performance:** Metaclass overhead is negligible unless `__new__` or `__init__` do expensive work (e.g., scanning the class tree, modifying all methods).

## Typical Use Cases

- **ORMs** (SQLAlchemy, Django) — intercept class definition to build schema mappings
- **Plugin systems** — register subclasses automatically
- **API frameworks** — validate endpoints, extract route metadata
- **Validation libraries** — declare schemas as classes, enforce at class creation time
- **Serialization** — intercept field definitions to build serializers

**Rule of thumb:** If you find yourself writing a metaclass, first ask: "Can `__init_subclass__`, a decorator, or a descriptor solve this?" Usually the answer is yes.

## See Also

- Descriptor protocol: `object.__getattribute__`, `object.__setattr__`
- Type system: Python type hints, gradual typing
- Class design: inheritance, composition, mixins