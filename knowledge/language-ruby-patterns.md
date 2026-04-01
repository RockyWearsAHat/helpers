# Ruby Patterns — Metaprogramming, Blocks, Mixins & Type Safety

## Ruby Philosophy

Ruby is designed for developer happiness and expressiveness. It emphasizes flexibility, convention over configuration, and code that reads like natural language. This flexibility enables powerful metaprogramming patterns absent or difficult in other languages.

- **Dynamic typing with optional gradual typing** (Sorbet)
- **Blocks as first-class idioms** — control flow, iteration, callbacks
- **Open classes** — reopen and extend any class, including built-ins
- **REPL-driven development** — interactive exploration and testing

## Metaprogramming — Code That Writes Code

Ruby's metaprogramming capabilities allow runtime class modification, method generation, and behavior customization that would require macros or code generation in compiled languages.

### Dynamic Method Definition

```ruby
# define_method — generate methods at runtime
class User
  ROLES = [:admin, :user, :viewer]

  ROLES.each do |role|
    define_method("#{role}?") do
      self.role == role
    end
  end
end

user = User.new(role: :admin)
user.admin?   # => true
user.viewer?  # => false
```

### method_missing — Catch Undefined Method Calls

```ruby
# Intercept calls to missing methods for dynamic dispatch
class DynamicHash
  def initialize(data)
    @data = data
  end

  def method_missing(name, *args, **kwargs, &block)
    # Check if method name matches a key pattern
    key = name.to_s
    return @data[key.to_sym] if @data.key?(key.to_sym)

    # Fallback
    super
  end

  def respond_to_missing?(name, include_private = false)
    @data.key?(name.to_sym) || super
  end
end

hash = DynamicHash.new(user_name: 'Alice', user_email: 'alice@example.com')
hash.user_name   # => 'Alice'
hash.user_email  # => 'alice@example.com'
```

**Caution:** `method_missing` is slow (method lookup fails first) and complicates debugging. Use for DSLs/adapters, not hot paths.

### send — Call Methods Dynamically

```ruby
# Call a method by its name or variable
class Calculator
  def add(a, b) = a + b
  def subtract(a, b) = a - b
  def multiply(a, b) = a * b
end

calc = Calculator.new
method = :add
result = calc.send(method, 5, 3)  # Calls calc.add(5, 3)
```

### eval and binding — Execute Code in a Context

```ruby
# eval executes code as strings (dangerous with untrusted input)
x = 10
code = "x + 5"
result = eval(code)  # => 15

# binding captures the execution environment
def create_binding
  x = 42
  return binding
end

context = create_binding
eval("x", context)  # => 42
```

**Caution:** `eval` is a security risk and makes code hard to understand. Prefer `send` or DSL patterns.

### define_singleton_method — Add Methods to Individual Objects

```ruby
# Add a method only to a specific object (singleton/eigenclass)
alice = User.new('Alice')
bob = User.new('Bob')

alice.define_singleton_method(:greeting) do
  "Hello, I'm special!"
end

alice.greeting  # => "Hello, I'm special!"
bob.greeting    # => NoMethodError
```

### Refinements — Scoped Monkey-Patching

```ruby
# Refinements limit changes to specific scopes (safer than global reopening)
module StringRefinements
  refine String do
    def shout
      self.upcase + "!"
    end
  end
end

# Without refine, 'hello'.shout is undefined
using StringRefinements
'hello'.shout  # => 'HELLO!'

# Outside the scope, String is unchanged
```

Refinements prevent polluting the global namespace but add complexity. Use for library extensions.

### Open Classes — Reopen and Extend Built-ins

```ruby
# Add methods to existing classes (including built-ins)
class String
  def words_count
    split.length
  end

  def reverse_each_word
    split.reverse.join(' ')
  end
end

'hello world'.words_count      # => 2
'hello world'.reverse_each_word  # => 'world hello'
```

**Caution:** Modifying built-ins creates implicit dependencies. Use with care; prefer refinements.

## Blocks, Procs, and Lambdas

Ruby treats code blocks as first-class citizens, enabling elegant control flow and callbacks.

### Blocks — Implicit Code

```ruby
# Blocks are passed implicitly; yield executes them
def with_timing
  start = Time.now
  yield  # Execute the block
  puts "Took #{Time.now - start}s"
end

with_timing { sleep 0.1 }  # => Took 0.1s

# Blocks with parameters
def transform(collection)
  collection.map { |item| yield(item) }
end

transform([1, 2, 3]) { |n| n * 2 }  # => [2, 4, 6]
```

### Procs — Explicit Objects

```ruby
# Proc captures a block as an object; can be stored and called later
greeting = Proc.new { |name| "Hello, #{name}!" }
greeting.call('Alice')  # => "Hello, Alice!"

# Alternative syntax
greet = proc { |x| "Hi, #{x}" }

# Procs are lenient with arguments
add = Proc.new { |a, b| a + b }
add.call(1, 2)      # => 3
add.call(1, 2, 3)   # => nil for b, results in error
add.call(1)         # => nil for b, error
```

### Lambdas — Strict Procs

```ruby
# Lambdas enforce arity (argument count) and early return behavior
multiply = lambda { |a, b| a * b }
multiply.call(2, 3)  # => 6
multiply.call(2)     # => ArgumentError: wrong number of args

# Alternative syntax (stabby lambda)
square = ->(x) { x * x }
square.call(5)  # => 25

# Return in lambda only returns from lambda, not enclosing method
def outer
  l = lambda { return 'from lambda' }
  l.call
  'from method'
end
outer  # => 'from method'

# Return in proc returns from enclosing method
def outer_with_proc
  p = proc { return 'from proc' }
  p.call
  'from method'
end
outer_with_proc  # => 'from proc'
```

### &:symbol — Method to Block Conversion

```ruby
# &:method_name converts a symbol to a block calling that method
[1, 2, 3].map(&:to_s)     # => ['1', '2', '3']
['a', 'b'].map(&:upcase)  # => ['A', 'B']

# Equivalent to:
[1, 2, 3].map { |n| n.to_s }
```

## Mixins — Modules and Inheritance

Ruby uses modules for code reuse and composable behavior without multiple inheritance.

### Modules — Bundles of Methods

```ruby
# Define a module with methods
module Timestamped
  def created_at
    @created_at ||= Time.now
  end

  def updated_at
    @updated_at ||= Time.now
  end
end

# Include — mix into instances
class Document
  include Timestamped
end

doc = Document.new
doc.created_at  # => Time instance
```

### include vs extend vs prepend

```ruby
module Greetings
  def hello
    "Hello from module"
  end
end

class Person
  # include — add to instance methods
  include Greetings
end

# extend — add to class methods
class Admin
  extend Greetings
end

Person.new.hello  # => "Hello from module"
Admin.hello       # => "Hello from module" (class method)

# prepend — call before the original method (for wrapping)
module Logging
  def hello
    puts "Calling hello"
    super  # Call the original method
  end
end

class Verbose
  prepend Logging
  def hello
    "response"
  end
end

Verbose.new.hello  # => Prints "Calling hello", then returns "response"
```

### Module Methods (ClassMethods Pattern)

```ruby
# Modules can add both instance and class methods via self.included hook
module Trackable
  def self.included(klass)
    # Called when module is included; extend with class methods
    klass.extend(ClassMethods)
  end

  module ClassMethods
    def find_by_id(id)
      # Class method logic
    end
  end

  def track_access
    # Instance method logic
  end
end

class User
  include Trackable
end

User.find_by_id(1)      # Class method
User.new.track_access   # Instance method
```

## Enumerable — Composable Iteration

`Enumerable` is a cornerstone mixin providing functional methods for collections. It requires implementing only `each`.

### Core Methods

```ruby
# Any class with each can mix in Enumerable
class Range
  include Enumerable
  # Only define each; Enumerable provides the rest
end

# Selection and transformation
[1, 2, 3, 4, 5]
  .select { |n| n.even? }    # => [2, 4]
  .map { |n| n * 2 }         # => [4, 8]

# Reduction
[1, 2, 3, 4].reduce(0) { |sum, n| sum + n }  # => 10
[1, 2, 3].reduce(:+)  # => 6 (using symbol)

# Boolean checks
[1, 2, 3].any? { |n| n > 2 }  # => true
[1, 2, 3].all? { |n| n > 0 }  # => true
[1, 2, 3].none? { |n| n > 5 } # => true

# Lazy evaluation (for large datasets)
(1..Float::INFINITY).lazy.map { |n| n * 2 }.first(5)  # => [2, 4, 6, 8, 10]
```

### Chaining and Composition

```ruby
users = [
  { name: 'Alice', age: 30, active: true },
  { name: 'Bob', age: 25, active: false },
  { name: 'Charlie', age: 35, active: true }
]

users
  .select { |u| u[:active] }
  .map { |u| u[:name] }
  .sort  # => ['Alice', 'Charlie']
```

## Rails Conventions — ActiveRecord & Patterns

Rails embeds Ruby patterns into a full-stack framework.

### Active Record Callbacks & Hooks

```ruby
class User < ApplicationRecord
  before_save :normalize_email
  after_create :send_welcome_email
  before_destroy :archive_user_data

  private

  def normalize_email
    self.email = email.downcase.strip
  end

  def send_welcome_email
    UserMailer.welcome(self).deliver_later
  end
end
```

### Scopes — Reusable Query Patterns

```ruby
class Post < ApplicationRecord
  scope :published, -> { where(published: true) }
  scope :recent, -> { order(created_at: :desc) }
  scope :by_author, ->(author) { where(author_id: author) }
end

Post.published.recent.by_author(alice)
```

### Associations Metaprogramming

Rails uses metaprogramming extensively:

```ruby
class User < ApplicationRecord
  has_many :posts
  has_one :profile
end

# Rails automatically creates:
# user.posts, user.posts<<, user.posts.build, user.profile=, etc.
```

## Testing — RSpec Patterns

RSpec is the dominant Ruby testing framework, emphasizing readable test syntax.

### Basic Spec Structure

```ruby
RSpec.describe User do
  describe '#full_name' do
    it 'returns first and last name' do
      user = User.new(first_name: 'Alice', last_name: 'Smith')
      expect(user.full_name).to eq('Alice Smith')
    end

    context 'when last name is missing' do
      it 'returns only first name' do
        user = User.new(first_name: 'Alice')
        expect(user.full_name).to eq('Alice')
      end
    end
  end
end
```

### Mocking and Stubbing

```ruby
describe '#send_email' do
  it 'calls the mailer' do
    mailer = double('Mailer')
    allow(mailer).to receive(:send).and_return(true)

    service = EmailService.new(mailer: mailer)
    service.send_to('test@example.com')

    expect(mailer).to have_received(:send)
  end
end
```

## Type Safety — Sorbet

Sorbet adds optional static type checking to Ruby, preserving runtime dynamism while catching type errors early.

### Levels of Strictness

```ruby
# Levels: untyped, true, strict, strong
typed: true

class User
  extend T::Sig

  sig { params(name: String, email: String).returns(User) }
  def initialize(name:, email:)
    @name = name
    @email = email
  end

  sig { returns(String) }
  attr_reader :name

  sig { returns(T::Array[Post]) }
  def posts
    @posts ||= []
  end

  # Duck typing with structural types
  sig do
    params(logger: T.any(Logger, CustomLogger)).void
  end
  def set_logger(logger)
    @logger = logger
  end
end

# Union types
sig { params(id: T.any(Integer, String)).returns(User) }
def find_user(id)
  # ...
end

# Nilable
sig { returns(T.nilable(String)) }
def nickname
  @nickname
end
```

### Gradual Adoption

Sorbet is designed for gradual adoption; you can type-check parts of a codebase incrementally. Use higher strictness levels for sensitive code; lower elsewhere.

## Patterns to Avoid

- **Excessive metaprogramming** — clarity over cleverness; favor explicit code
- **Global state in modules** — use dependency injection
- **Modifying built-ins** — prefer refinements or wrapper classes
- **method_missing for all dynamic dispatch** — use case-specific solutions
- **Untyped large codebases** — adopt Sorbet incrementally for new code

## See Also

- [language-ruby.md](language-ruby.md) — Ruby conventions and idioms
- [framework-rails.md](framework-rails.md) — Rails framework patterns
- [paradigm-metaprogramming.md](paradigm-metaprogramming.md) — Metaprogramming across languages
- [functional-programming.md](functional-programming.md) — Functional patterns in Ruby and beyond