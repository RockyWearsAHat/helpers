# Elixir Conventions and Idioms

## Elixir Philosophy

Elixir runs on the BEAM (Erlang VM) — built for massive concurrency, fault tolerance, and distributed systems. If it needs to stay up 24/7 and handle millions of connections, Elixir excels.

## Core Concepts

### Pattern Matching (Everywhere)

```elixir
# Variable binding IS pattern matching
{:ok, user} = fetch_user(id)
{:error, reason} = {:error, :not_found}

# Function heads match patterns
def greet(%{name: name}), do: "Hello, #{name}!"
def greet(_), do: "Hello, stranger!"

# Case statements
case HTTP.get(url) do
  {:ok, %{status: 200, body: body}} -> parse(body)
  {:ok, %{status: 404}} -> {:error, :not_found}
  {:ok, %{status: status}} -> {:error, {:http_error, status}}
  {:error, reason} -> {:error, reason}
end

# With statement (chain pattern matches)
with {:ok, user} <- fetch_user(id),
     {:ok, posts} <- fetch_posts(user.id),
     {:ok, rendered} <- render(posts) do
  {:ok, rendered}
else
  {:error, :not_found} -> {:error, "User not found"}
  {:error, reason} -> {:error, "Failed: #{inspect(reason)}"}
end

# Pin operator — match against existing value
x = 1
^x = 1  # Matches (asserts x == 1)
^x = 2  # ** (MatchError) no match of right hand side value: 2
```

### Pipe Operator

```elixir
# Transform data through a pipeline (data flows left to right)
"  Hello, World!  "
|> String.trim()
|> String.downcase()
|> String.split(", ")
|> Enum.map(&String.capitalize/1)
|> Enum.join(" - ")
# => "Hello - World!"

# Real-world example
conn
|> authenticate()
|> authorize(:admin)
|> fetch_data(params)
|> render_response()
```

### Processes (Lightweight Concurrency)

```elixir
# Spawn a process (extremely cheap — millions possible)
pid = spawn(fn ->
  receive do
    {:greet, name} -> IO.puts("Hello, #{name}!")
  end
end)

send(pid, {:greet, "Alice"})

# Task — structured async
task = Task.async(fn -> expensive_computation() end)
result = Task.await(task)

# Parallel map
results = tasks
|> Task.async_stream(fn task -> process(task) end, max_concurrency: 10)
|> Enum.to_list()
```

### GenServer (Generic Server)

```elixir
defmodule Counter do
  use GenServer

  # Client API
  def start_link(initial \\ 0) do
    GenServer.start_link(__MODULE__, initial, name: __MODULE__)
  end

  def increment, do: GenServer.cast(__MODULE__, :increment)
  def get_count, do: GenServer.call(__MODULE__, :get)

  # Server callbacks
  @impl true
  def init(initial), do: {:ok, initial}

  @impl true
  def handle_cast(:increment, count), do: {:noreply, count + 1}

  @impl true
  def handle_call(:get, _from, count), do: {:reply, count, count}
end
```

### Supervisors (Fault Tolerance)

```elixir
# "Let it crash" — supervisors restart failed processes
defmodule MyApp.Application do
  use Application

  def start(_type, _args) do
    children = [
      {MyApp.Cache, []},
      {MyApp.Worker, []},
      {MyApp.WebServer, port: 4000},
    ]

    # one_for_one: if a child crashes, only restart that child
    opts = [strategy: :one_for_one, name: MyApp.Supervisor]
    Supervisor.start_link(children, opts)
  end
end

# Supervision strategies:
# :one_for_one   — restart only the crashed child
# :one_for_all   — restart all children if one crashes
# :rest_for_one  — restart the crashed child and all children started after it
```

## Structs & Protocols

```elixir
# Structs
defmodule User do
  @enforce_keys [:name, :email]
  defstruct [:name, :email, age: 0, role: :user]
end

user = %User{name: "Alice", email: "alice@test.com"}
admin = %{user | role: :admin}

# Protocols (polymorphism)
defprotocol Renderable do
  def render(data)
end

defimpl Renderable, for: User do
  def render(user), do: "#{user.name} <#{user.email}>"
end

defimpl Renderable, for: Map do
  def render(map), do: inspect(map)
end
```

## Functional Patterns

```elixir
# Enum module (eager)
users
|> Enum.filter(&(&1.active))
|> Enum.sort_by(& &1.name)
|> Enum.map(& &1.email)
|> Enum.uniq()

# Stream module (lazy)
File.stream!("huge.csv")
|> Stream.map(&String.trim/1)
|> Stream.reject(&(&1 == ""))
|> Stream.map(&parse_line/1)
|> Enum.take(100)

# Comprehensions
for user <- users, user.active, into: %{} do
  {user.id, user.name}
end
```

## Phoenix Framework

```elixir
# LiveView — real-time server-rendered UI
defmodule MyAppWeb.CounterLive do
  use MyAppWeb, :live_view

  def mount(_params, _session, socket) do
    {:ok, assign(socket, count: 0)}
  end

  def handle_event("increment", _, socket) do
    {:noreply, update(socket, :count, &(&1 + 1))}
  end

  def render(assigns) do
    ~H"""
    <div>
      <h1>Count: <%= @count %></h1>
      <button phx-click="increment">+1</button>
    </div>
    """
  end
end
```

## Tooling

| Tool         | Purpose                          |
| ------------ | -------------------------------- |
| **mix**      | Build tool + package manager     |
| **ExUnit**   | Testing (built-in)               |
| **Credo**    | Static analysis / linting        |
| **Dialyxir** | Type checking (Dialyzer wrapper) |
| **Ecto**     | Database wrapper + query builder |
| **Phoenix**  | Web framework                    |
| **Livebook** | Interactive notebooks            |
| **Observer** | Runtime system monitoring        |

---

_Sources: Elixir documentation, Programming Elixir (Dave Thomas), Elixir in Action (Saša Jurić), Phoenix documentation_
