# Ruby Best Practices

## Ruby Philosophy

"Optimized for developer happiness." — Yukihiro Matsumoto (Matz)

- **Principle of Least Surprise**: The language should behave as you'd expect.
- **Everything is an object**: `42.class # => Integer`. Even `nil` is an object.
- **Multiple ways to do things**: Ruby embraces flexibility. The community picks conventions.

## Style Guide (Based on RuboCop / Community Guide)

### Naming

- `snake_case` for methods, variables, files.
- `PascalCase` for classes and modules.
- `SCREAMING_SNAKE_CASE` for constants.
- `predicate_method?` returns boolean.
- `dangerous_method!` modifies in place or may raise.

### Layout

- 2-space indentation (not tabs, not 4).
- `do...end` for multi-line blocks, `{ }` for single-line.
- No parens for methods with no arguments: `user.name` not `user.name()`.
- Trailing commas in multi-line arrays/hashes.

## Blocks, Procs, and Lambdas

Ruby's killer feature — code as a first-class value.

```ruby
# Block (most common — not an object, passed to methods)
[1, 2, 3].map { |n| n * 2 }  # => [2, 4, 6]

[1, 2, 3].each do |n|
  puts n
end

# Block with yield
def with_retry(attempts: 3)
  attempts.times do |i|
    return yield
  rescue StandardError => e
    raise if i == attempts - 1
    sleep(2**i)
  end
end

with_retry { api_call() }

# Proc (stored block — relaxed about arguments)
doubler = Proc.new { |n| n * 2 }
doubler.call(5)   # => 10
doubler.(5)        # => 10 (syntactic sugar)

# Lambda (strict Proc — checks argument count, returns to caller)
validator = ->(n) { n > 0 }
validator.call(5)   # => true
validator.call(-1)  # => false

# Method reference
["hello", "world"].map(&method(:puts))

# Symbol to proc
["hello", "world"].map(&:upcase)  # => ["HELLO", "WORLD"]
```

## Enumerable — The Core Abstraction

Any class that includes `Enumerable` and defines `each` gets 50+ methods for free.

```ruby
numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

numbers.select(&:even?)           # => [2, 4, 6, 8, 10]
numbers.reject(&:odd?)            # => [2, 4, 6, 8, 10]
numbers.map { |n| n ** 2 }       # => [1, 4, 9, 16, ...]
numbers.reduce(:+)                # => 55
numbers.each_slice(3).to_a        # => [[1,2,3], [4,5,6], [7,8,9], [10]]
numbers.flat_map { |n| [n, -n] } # => [1, -1, 2, -2, ...]
numbers.group_by(&:even?)         # => {false: [1,3,5,7,9], true: [2,4,6,8,10]}
numbers.min_by { |n| (n - 6).abs } # => 6 (closest to 6)
numbers.each_with_object({}) { |n, h| h[n] = n * 2 }
numbers.tally                      # => {1=>1, 2=>1, ...} (Ruby 2.7+)
numbers.filter_map { |n| n * 2 if n.even? }  # => [4, 8, 12, 16, 20] (Ruby 2.7+)

# Lazy evaluation for infinite sequences
(1..Float::INFINITY).lazy.select(&:even?).take(5).to_a  # => [2, 4, 6, 8, 10]

# Chaining
users
  .select(&:active?)
  .sort_by(&:name)
  .map { |u| { name: u.name, email: u.email } }
```

## Duck Typing

Ruby doesn't care about types — it cares about behavior.

```ruby
# If it walks like a duck and quacks like a duck...
def print_content(source)
  source.each_line do |line|
    puts line
  end
end

# All of these work:
print_content(File.open("data.txt"))  # File responds to each_line
print_content("line1\nline2\nline3")  # String responds to each_line
print_content(StringIO.new("data"))   # StringIO responds to each_line

# respond_to? for checking capability
def process(input)
  if input.respond_to?(:read)
    input.read
  elsif input.respond_to?(:to_s)
    input.to_s
  end
end
```

## Classes and Modules

```ruby
# Class with idiomatic Ruby
class User
  attr_reader :name, :email        # Getter only
  attr_accessor :role              # Getter + setter

  def initialize(name, email, role: :user)
    @name = name.freeze
    @email = email.freeze
    @role = role
  end

  def admin?
    role == :admin
  end

  def to_s
    "#{name} <#{email}>"
  end

  private

  def secret_method
    # Only accessible within the class
  end
end

# Modules for shared behavior (mixins)
module Auditable
  def audit_log
    @audit_log ||= []
  end

  def record_change(field, old_value, new_value)
    audit_log << { field:, old_value:, new_value:, at: Time.now }
  end
end

class Order
  include Auditable  # Instance methods
  # extend Auditable # Would add as class methods
end

# Struct — quick data classes
Point = Struct.new(:x, :y) do
  def distance_to(other)
    Math.sqrt((x - other.x)**2 + (y - other.y)**2)
  end
end

# Data (Ruby 3.2+) — immutable value objects
Point = Data.define(:x, :y)
p = Point.new(x: 1, y: 2)
p.x  # => 1
```

## Error Handling

```ruby
# Rescue specific exceptions
begin
  result = JSON.parse(input)
rescue JSON::ParserError => e
  logger.warn("Invalid JSON: #{e.message}")
  result = {}
rescue StandardError => e
  logger.error("Unexpected error: #{e.message}")
  raise  # Re-raise
ensure
  # Always runs (like finally)
  cleanup()
end

# Inline rescue (use sparingly — only for simple defaults)
value = Integer(input) rescue nil

# Custom exceptions
class AppError < StandardError; end
class NotFoundError < AppError; end
class ValidationError < AppError
  attr_reader :errors

  def initialize(errors)
    @errors = errors
    super(errors.join(", "))
  end
end

# retry
def fetch_with_retry(url, attempts: 3)
  response = Net::HTTP.get_response(URI(url))
  raise "HTTP #{response.code}" unless response.is_a?(Net::HTTPSuccess)
  response.body
rescue StandardError => e
  attempts -= 1
  retry if attempts > 0
  raise
end
```

## Modern Ruby (3.0+)

```ruby
# Pattern matching (Ruby 3.0+)
case response
in { status: 200, body: String => body }
  process(body)
in { status: 404 }
  not_found
in { status: (500..) }
  server_error
end

# Find pattern
case users
in [*, { name: "Alice", role: } => alice, *]
  puts "Found Alice with role: #{role}"
end

# Shorthand hash syntax (Ruby 3.1+)
name = "Alice"
age = 30
{ name:, age: }  # => { name: "Alice", age: 30 }

# Ractor (Ruby 3.0+) — true parallelism
ractor = Ractor.new do
  Ractor.receive * 2
end
ractor.send(21)
ractor.take  # => 42

# Endless method definition (Ruby 3.0+)
def double(n) = n * 2
def greet(name) = "Hello, #{name}!"
```

## Testing with RSpec / Minitest

```ruby
# RSpec (BDD style)
RSpec.describe User do
  describe "#admin?" do
    context "when role is admin" do
      let(:user) { User.new("Alice", "a@test.com", role: :admin) }

      it "returns true" do
        expect(user).to be_admin
      end
    end

    context "when role is user" do
      let(:user) { User.new("Bob", "b@test.com") }

      it "returns false" do
        expect(user).not_to be_admin
      end
    end
  end
end

# Minitest (simpler, stdlib)
class UserTest < Minitest::Test
  def test_admin
    user = User.new("Alice", "a@test.com", role: :admin)
    assert user.admin?
  end
end
```

## Tooling

| Tool                     | Purpose                         |
| ------------------------ | ------------------------------- |
| **RuboCop**              | Linting + style enforcement     |
| **Bundler**              | Dependency management           |
| **RSpec** / **Minitest** | Testing                         |
| **Sorbet**               | Static type checking            |
| **Pry** / **IRB**        | REPL / debugger                 |
| **Steep** + **RBS**      | Type checking / type signatures |

---

_Sources: Ruby Style Guide (rubocop), Practical Object-Oriented Design (Sandi Metz), Eloquent Ruby (Russ Olsen), The Well-Grounded Rubyist (David Black), Ruby documentation_
