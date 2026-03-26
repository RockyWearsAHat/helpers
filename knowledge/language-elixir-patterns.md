# Elixir Patterns: OTP, GenServer, Supervision, and Production Architecture

## GenServer: The Foundation of Stateful Actors

GenServer (generic server) is the workhorse of Elixir concurrency. It's a behavior — a callback module that implements a protocol for handling calls and casts.

```elixir
defmodule Counter do
  use GenServer
  require Logger

  # Client API
  def start_link(initial_value) do
    GenServer.start_link(__MODULE__, initial_value, name: __MODULE__)
  end

  def increment do
    GenServer.call(__MODULE__, :inc)
  end

  def decrement do
    GenServer.call(__MODULE__, :dec)
  end

  def value do
    GenServer.call(__MODULE__, :get)
  end

  # Server callbacks (run in GenServer process)

  @impl true
  def init(initial_value) do
    Logger.info("Counter started with value: #{initial_value}")
    {:ok, initial_value}
  end

  @impl true
  def handle_call(:inc, _from, count) do
    {:reply, count + 1, count + 1}
  end

  @impl true
  def handle_call(:dec, _from, count) do
    {:reply, count - 1, count - 1}
  end

  @impl true
  def handle_call(:get, _from, count) do
    {:reply, count, count}
  end

  @impl true
  def handle_cast(:reset, _state) do
    {:noreply, 0}
  end
end

# Usage
Counter.start_link(0)
Counter.increment()     # Synchronous call — waits for reply
GenServer.cast(Counter, :reset)  # Asynchronous cast — fire and forget
Counter.value()         # Returns 1
```

**Key differences:**
- **`call`** — synchronous request/response; caller waits for return value
- **`cast`** — asynchronous message; caller doesn't wait
- **`info`** — out-of-band messages sent via `send/2`; handled in `handle_info/2`

## Supervisor Trees: Hierarchical Fault Tolerance

Supervisors manage child processes and implement restart strategies. They form a tree structure where each node is responsible for its children.

```elixir
defmodule MyApp.Supervisor do
  use Supervisor
  require Logger

  def start_link(opts) do
    Supervisor.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @impl true
  def init(_opts) do
    children = [
      # Simple child (Counter)
      {Counter, 0},

      # Named child with options
      Supervisor.child_spec(
        {Worker, [name: :worker_1]},
        id: :worker_1
      ),

      # Another supervisor (nested)
      {PoolSupervisor, [size: 5]}
    ]

    # Supervision strategy: restart strategy if child dies
    Supervisor.init(children, strategy: :one_for_one)
  end
end

# Supervision strategies
# :one_for_one — restart only the failed child
# :one_for_all — restart the failed child and all siblings
# :rest_for_one — restart failed child and all started after it
# :simple_one_for_one — (deprecated in Elixir 1.15+) dynamic children, all identical

# Dynamic child management
Supervisor.start_child(MyApp.Supervisor, {Counter, 100})
Supervisor.terminate_child(MyApp.Supervisor, child_id)
```

**Restart intensity:** Supervisors track restarts using `:max_restarts` (default 3) in `:max_seconds` (default 5). If too many restarts occur, the supervisor itself dies.

## Phoenix LiveView: Real-Time, Stateful UIs

LiveView combines server-side state management with real-time client updates via WebSocket, reducing client-side complexity.

```elixir
defmodule MyAppWeb.Live.Counter do
  use Phoenix.LiveView

  def render(assigns) do
    ~H"""
    <div>
      <p>Count: <%= @count %></p>
      <button phx-click="increment">+</button>
      <button phx-click="decrement">-</button>
    </div>
    """
  end

  def mount(_params, _session, socket) do
    {:ok, assign(socket, :count, 0)}
  end

  def handle_event("increment", _value, socket) do
    new_count = socket.assigns.count + 1
    {:noreply, assign(socket, :count, new_count)}
  end

  def handle_event("decrement", _value, socket) do
    new_count = socket.assigns.count - 1
    {:noreply, assign(socket, :count, new_count)}
  end
end

# Lifecycle: mount → render → handle_event → render (delta broadcast to client)

# Push events from server
def broadcast_update(socket, data) do
  {:noreply, push_event(socket, "update", data)}
end
```

**Advantages:** State lives on server (no sync problems); diffs computed server-side (less data); `phx-` bindings are declarative. **Disadvantages:** Requires WebSocket; fine-grained interactions can be chatty.

## Ecto: Schemas, Changesets, and Database Integration

Ecto separates concerns: **Repo** (persistence), **Schema** (data structure), **Changeset** (validation/transformation).

```elixir
# Schema
defmodule MyApp.User do
  use Ecto.Schema
  import Ecto.Changeset

  schema "users" do
    field :email, :string
    field :password_hash, :string
    field :active, :boolean, default: true
    has_many :posts, MyApp.Post
    
    timestamps()  # inserted_at, updated_at
  end

  # Cast and validate
  def changeset(user, attrs) do
    user
    |> cast(attrs, [:email])
    |> validate_required([:email])
    |> validate_format(:email, ~r/@/)
    |> unique_constraint(:email)
  end

  # Companion for password changes
  def password_changeset(user, attrs) do
    user
    |> cast(attrs, [:password])
    |> validate_required(:password)
    |> validate_length(:password, min: 8)
    |> put_password_hash()
  end

  defp put_password_hash(changeset) do
    if password = get_change(changeset, :password) do
      put_change(changeset, :password_hash, hash(password))
    else
      changeset
    end
  end
end

# Persisting
user_changeset = User.changeset(%User{}, %{"email" => "alice@example.com"})

case Repo.insert(user_changeset) do
  {:ok, user} -> IO.puts("User created: #{user.id}")
  {:error, changeset} -> IO.inspect(changeset.errors)
end

# Queries
import Ecto.Query

Repo.all(from u in User, where: u.active == true)
Repo.get(User, user_id)
Repo.get_by(User, email: "alice@example.com")
```

**Changeset pattern:** Never trust input. Separate validation/transformation (Changeset) from database writes (Repo).

## Broadway: Event Processing Pipelines

Broadway models data pipelines as DAGs: producers (sources) → processors (transforms) → consumers (sinks).

```elixir
defmodule MyApp.Pipeline do
  use Broadway

  def start_link(_opts) do
    Broadway.start_link(__MODULE__,
      name: __MODULE__,
      producer: [
        module: {BroadwayKafka.Producer, topic: "events"}
      ],
      processors: [
        default: [concurrency: 10]
      ],
      batchers: [
        default: [batch_size: 100, batch_timeout: 500]
      ]
    )
  end

  def handle_message(_processor, message, _context) do
    # Transform each message
    data = message.data |> parse() |> enrich()
    Message.update_data(message, data)
  end

  def handle_batch(_batcher, messages, _batch_info, _context) do
    # Batch processing: insert to database, send to API, etc.
    results = Enum.map(messages, &process_message/1)
    
    # Return succeeded/failed messages
    {succeeded, failed} = Enum.split_with(results, &elem(&1, 0))
    succeeded
  end
end
```

## Nerves: Embedded IoT with Elixir

Nerves uses Buildroot to cross-compile Erlang/Elixir to embedded platforms (Raspberry Pi, etc.).

```elixir
# Typical Nerves supervision tree
defmodule MyDevice.Supervisor do
  use Supervisor

  def start_link(opts) do
    Supervisor.start_link(__MODULE__, opts, name: __MODULE__)
  end

  def init(_opts) do
    children = [
      # GPIO control
      {Circuits.GPIO, [pin: 17, direction: :out, name: :led]},
      
      # I2C sensor reading
      {Circuits.I2C, [bus: "i2c-1", address: 0x48]},
      
      # Your application logic
      MyDevice.SensorReader,
      
      # Network/connectivity
      Nerves.Firmware.HTTP.Server
    ]

    Supervisor.init(children, strategy: :one_for_one)
  end
end

# Typical workflow
defmodule MyDevice.SensorReader do
  use GenServer

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, nil, name: __MODULE__)
  end

  def init(_) do
    schedule_read()
    {:ok, nil}
  end

  def handle_info(:read, state) do
    temp = Circuits.I2C.read!(bus(), address(), 2)
    IO.puts("Temperature: #{temp}")
    schedule_read()
    {:noreply, state}
  end

  defp schedule_read do
    Process.send_after(self(), :read, 5000)
  end
end
```

## ExUnit: Testing Framework

ExUnit provides structure for unit and integration tests with fixtures, assertions, and async test execution.

```elixir
defmodule MyApp.UserTest do
  use ExUnit.Case
  doctest MyApp.User

  setup do
    # Run before each test
    user = insert(:user, email: "test@example.com")
    {:ok, user: user}
  end

  test "user can be created", %{user: user} do
    assert user.id
    assert user.email == "test@example.com"
  end

  test "changeset rejects invalid email" do
    changeset = User.changeset(%User{}, %{email: "invalid"})
    refute changeset.valid?
  end

  test "works with async", do: assert 1 + 1 == 2
end
```

ExUnit runs tests concurrently by default (tagged `:async`). Integration tests typically run serially.

## OTP Application Design

OTP applications define a supervision structure and startup behavior via `application.ex`.

```elixir
defmodule MyApp.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      {MyApp.Repo, []},      # Database
      {MyApp.Cache, []},     # In-memory cache
      {MyApp.Supervisor, []},  # Main supervision tree
      {Phoenix.PubSub, [name: MyApp.PubSub]},  # Pub/sub system
    ]

    opts = [strategy: :one_for_one, name: MyApp.Supervisor]
    Supervisor.start_link(children, opts)
  end
end

# mix.exs
def application do
  [
    extra_applications: [:logger],
    mod: {MyApp.Application, []}  # Starts via start/2
  ]
end
```

---

## See Also

- [Language: Elixir Conventions](language-elixir.md)
- [Concurrency Patterns](concurrency-patterns.md)
- [Framework: Phoenix](framework-phoenix.md)