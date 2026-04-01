# Phoenix

## Architecture

Phoenix is an Elixir web framework built on the Erlang VM (BEAM). It leverages Erlang's concurrency model — each request is handled by a lightweight process. Millions of concurrent connections are routine.

```
Request → Endpoint → Router → Pipeline (plugs) → Controller → View → Template
         (or)       → LiveView (WebSocket, real-time)
         (or)       → Channel (WebSocket, bidirectional)
```

## Contexts (Bounded Contexts)

Phoenix organizes business logic into **contexts** — modules that group related functionality:

```elixir
# lib/myapp/blog.ex (context)
defmodule MyApp.Blog do
  alias MyApp.Blog.{Post, Comment}
  alias MyApp.Repo

  def list_posts(opts \\ []) do
    Post
    |> order_by(desc: :inserted_at)
    |> maybe_filter_by_status(opts[:status])
    |> Repo.all()
    |> Repo.preload(:author)
  end

  def get_post!(id), do: Repo.get!(Post, id) |> Repo.preload([:author, :comments])

  def create_post(attrs) do
    %Post{}
    |> Post.changeset(attrs)
    |> Repo.insert()
  end

  def update_post(%Post{} = post, attrs) do
    post
    |> Post.changeset(attrs)
    |> Repo.update()
  end

  def delete_post(%Post{} = post), do: Repo.delete(post)

  defp maybe_filter_by_status(query, nil), do: query
  defp maybe_filter_by_status(query, status), do: where(query, status: ^status)
end
```

Contexts are the public API for a domain. Controllers call contexts, never Repo directly.

## Ecto

### Schemas

```elixir
defmodule MyApp.Blog.Post do
  use Ecto.Schema
  import Ecto.Changeset

  schema "posts" do
    field :title, :string
    field :body, :string
    field :status, Ecto.Enum, values: [:draft, :published, :archived], default: :draft
    field :views, :integer, default: 0
    field :metadata, :map, default: %{}

    belongs_to :author, MyApp.Accounts.User
    has_many :comments, MyApp.Blog.Comment, preload_order: [asc: :inserted_at]
    many_to_many :tags, MyApp.Blog.Tag, join_through: "posts_tags", on_replace: :delete

    timestamps(type: :utc_datetime)
  end

  def changeset(post, attrs) do
    post
    |> cast(attrs, [:title, :body, :status, :author_id])
    |> validate_required([:title, :body, :author_id])
    |> validate_length(:title, min: 1, max: 300)
    |> validate_inclusion(:status, [:draft, :published, :archived])
    |> foreign_key_constraint(:author_id)
    |> unique_constraint(:slug)
  end

  def publish_changeset(post) do
    post
    |> change(status: :published, published_at: DateTime.utc_now())
    |> validate_required([:title, :body])
  end
end
```

### Changesets

Changesets are the core data validation mechanism. They track changes, cast external data, validate, and report errors — all without touching the database.

```elixir
# Changeset is just data — inspect it
changeset = Post.changeset(%Post{}, %{title: "", body: "content"})
changeset.valid?    # false
changeset.errors    # [title: {"can't be blank", [validation: :required]}]
changeset.changes   # %{body: "content"}
```

### Queries

```elixir
import Ecto.Query

# Keyword syntax
from p in Post,
  where: p.status == :published,
  where: p.views > 100,
  order_by: [desc: p.inserted_at],
  limit: 20,
  preload: [:author, :tags],
  select: %{id: p.id, title: p.title, views: p.views}

# Pipe syntax (composable)
Post
|> where([p], p.status == :published)
|> where([p], p.inserted_at > ^one_week_ago)
|> order_by([p], desc: p.inserted_at)
|> limit(20)
|> preload(:author)
|> Repo.all()

# Aggregation
Repo.aggregate(Post, :count)
Repo.aggregate(Post, :avg, :views)

from p in Post,
  group_by: p.status,
  select: {p.status, count(p.id)}

# Subquery
popular_authors = from p in Post,
  group_by: p.author_id,
  having: count(p.id) > 10,
  select: p.author_id

from u in User, where: u.id in subquery(popular_authors)

# Dynamic queries
def filter_posts(params) do
  Post
  |> maybe_filter(:status, params["status"])
  |> maybe_filter(:author_id, params["author_id"])
  |> Repo.all()
end

defp maybe_filter(query, _field, nil), do: query
defp maybe_filter(query, :status, val), do: where(query, status: ^val)
defp maybe_filter(query, :author_id, val), do: where(query, author_id: ^val)
```

### Associations and Preloading

```elixir
# Eager loading
Repo.all(Post) |> Repo.preload([:author, comments: :user])

# Query-based preload (control the query)
comments_query = from c in Comment, order_by: [desc: c.inserted_at], limit: 5
Repo.all(Post) |> Repo.preload(comments: ^comments_query)

# Inline preload
from p in Post, preload: [comments: ^comments_query]
```

### Migrations

```elixir
defmodule MyApp.Repo.Migrations.CreatePosts do
  use Ecto.Migration

  def change do
    create table(:posts) do
      add :title, :string, null: false, size: 300
      add :body, :text
      add :status, :string, null: false, default: "draft"
      add :views, :integer, default: 0
      add :metadata, :map, default: %{}
      add :author_id, references(:users, on_delete: :delete_all), null: false

      timestamps(type: :utc_datetime)
    end

    create index(:posts, [:author_id])
    create index(:posts, [:status, :inserted_at])
    create unique_index(:posts, [:slug])
  end
end
```

### Multi (transactions)

```elixir
Ecto.Multi.new()
|> Ecto.Multi.insert(:post, Post.changeset(%Post{}, post_attrs))
|> Ecto.Multi.insert(:notification, fn %{post: post} ->
  Notification.changeset(%Notification{}, %{post_id: post.id, type: :new_post})
end)
|> Ecto.Multi.update(:author_stats, fn %{post: post} ->
  User.increment_posts_changeset(post.author)
end)
|> Repo.transaction()
|> case do
  {:ok, %{post: post}} -> {:ok, post}
  {:error, _step, changeset, _changes} -> {:error, changeset}
end
```

## LiveView

LiveView enables rich, real-time UIs rendered on the server over WebSocket — no JavaScript required for interactivity.

### Lifecycle

1. Initial HTTP request renders static HTML (SEO-friendly)
2. Client connects via WebSocket
3. `mount/3` runs again in the connected state
4. Events from client trigger `handle_event/3`
5. Server pushes minimal diffs to update the DOM

```elixir
defmodule MyAppWeb.PostLive.Index do
  use MyAppWeb, :live_view

  @impl true
  def mount(_params, _session, socket) do
    if connected?(socket), do: Blog.subscribe()

    {:ok, stream(socket, :posts, Blog.list_posts())}
  end

  @impl true
  def handle_params(params, _url, socket) do
    {:noreply, apply_action(socket, socket.assigns.live_action, params)}
  end

  defp apply_action(socket, :index, _params) do
    assign(socket, :page_title, "Posts")
  end

  defp apply_action(socket, :new, _params) do
    assign(socket, :page_title, "New Post")
    |> assign(:post, %Post{})
  end

  @impl true
  def handle_event("delete", %{"id" => id}, socket) do
    post = Blog.get_post!(id)
    {:ok, _} = Blog.delete_post(post)
    {:noreply, stream_delete(socket, :posts, post)}
  end

  # PubSub handler
  @impl true
  def handle_info({:post_created, post}, socket) do
    {:noreply, stream_insert(socket, :posts, post, at: 0)}
  end
end
```

### HEEx Templates

```heex
<.header>
  Posts
  <:actions>
    <.link patch={~p"/posts/new"}>
      <.button>New Post</.button>
    </.link>
  </:actions>
</.header>

<.table id="posts" rows={@streams.posts} row_click={fn {_id, post} -> JS.navigate(~p"/posts/#{post}") end}>
  <:col :let={{_id, post}} label="Title"><%= post.title %></:col>
  <:col :let={{_id, post}} label="Status"><%= post.status %></:col>
  <:action :let={{_id, post}}>
    <.link phx-click={JS.push("delete", value: %{id: post.id})} data-confirm="Sure?">
      Delete
    </.link>
  </:action>
</.table>
```

### Streams (efficient list rendering)

```elixir
# In mount
{:ok, stream(socket, :posts, Blog.list_posts())}

# Insert at top
stream_insert(socket, :posts, new_post, at: 0)

# Delete
stream_delete(socket, :posts, post)

# Reset
stream(socket, :posts, Blog.list_posts(), reset: true)
```

Streams don't keep data on the server — they send insert/delete commands to the client. Ideal for large lists.

### Async Assigns

```elixir
def mount(_params, _session, socket) do
  {:ok,
   socket
   |> assign(:page_title, "Dashboard")
   |> assign_async(:stats, fn -> {:ok, %{stats: compute_stats()}} end)
   |> assign_async(:recent, fn -> {:ok, %{recent: Blog.recent_posts()}} end)}
end
```

```heex
<.async_result :let={stats} assign={@stats}>
  <:loading>Computing stats...</:loading>
  <:failed :let={_reason}>Failed to load stats</:failed>
  <p>Total views: <%= stats.total_views %></p>
</.async_result>
```

## Phoenix.Component

```elixir
defmodule MyAppWeb.CoreComponents do
  use Phoenix.Component

  attr :type, :string, default: "button"
  attr :class, :string, default: nil
  attr :rest, :global
  slot :inner_block, required: true

  def button(assigns) do
    ~H"""
    <button type={@type} class={["btn", @class]} {@rest}>
      <%= render_slot(@inner_block) %>
    </button>
    """
  end

  attr :flash, :map, required: true
  attr :kind, :atom, values: [:info, :error]

  def flash_message(assigns) do
    ~H"""
    <div :if={msg = Phoenix.Flash.get(@flash, @kind)} class={"alert alert-#{@kind}"}>
      <%= msg %>
    </div>
    """
  end
end
```

## Channels and PubSub

```elixir
# Channel
defmodule MyAppWeb.RoomChannel do
  use MyAppWeb, :channel

  def join("room:" <> room_id, _payload, socket) do
    {:ok, assign(socket, :room_id, room_id)}
  end

  def handle_in("new_msg", %{"body" => body}, socket) do
    broadcast!(socket, "new_msg", %{body: body, user: socket.assigns.user_id})
    {:noreply, socket}
  end
end

# PubSub (decoupled messaging)
Phoenix.PubSub.subscribe(MyApp.PubSub, "posts")
Phoenix.PubSub.broadcast(MyApp.PubSub, "posts", {:post_created, post})
```

## Presence

```elixir
defmodule MyAppWeb.Presence do
  use Phoenix.Presence, otp_app: :myapp, pubsub_server: MyApp.PubSub
end

# In channel
def handle_info(:after_join, socket) do
  Presence.track(socket, socket.assigns.user_id, %{online_at: System.system_time(:second)})
  push(socket, "presence_state", Presence.list(socket))
  {:noreply, socket}
end
```

Presence uses CRDTs — works across distributed nodes without a central store.

## Plugs (Middleware)

```elixir
# Function plug
defmodule MyAppWeb.Plugs.RequireAdmin do
  import Plug.Conn

  def init(opts), do: opts

  def call(conn, _opts) do
    if conn.assigns[:current_user]&.admin? do
      conn
    else
      conn
      |> put_status(:forbidden)
      |> Phoenix.Controller.put_view(MyAppWeb.ErrorHTML)
      |> Phoenix.Controller.render("403.html")
      |> halt()
    end
  end
end

# Pipeline in router
pipeline :admin do
  plug :require_authenticated_user
  plug MyAppWeb.Plugs.RequireAdmin
end

scope "/admin", MyAppWeb.Admin do
  pipe_through [:browser, :admin]
  live "/dashboard", DashboardLive
end
```

## Telemetry

```elixir
# Attach handler
:telemetry.attach("log-query", [:myapp, :repo, :query], fn _name, measurements, metadata, _config ->
  Logger.debug("Query: #{metadata.query} (#{measurements.total_time / 1_000_000}ms)")
end, nil)

# Emit custom event
:telemetry.execute([:myapp, :orders, :created], %{count: 1}, %{user_id: user.id})
```

Phoenix and Ecto emit telemetry events for every request, query, and channel event. Use `phoenix_live_dashboard` to visualize.

## Testing

```elixir
defmodule MyAppWeb.PostLiveTest do
  use MyAppWeb.ConnCase
  import Phoenix.LiveViewTest

  test "lists all posts", %{conn: conn} do
    post = insert(:post, title: "Hello World")
    {:ok, view, html} = live(conn, ~p"/posts")
    assert html =~ "Hello World"
  end

  test "creates a new post", %{conn: conn} do
    {:ok, view, _} = live(conn, ~p"/posts/new")

    assert view
           |> form("#post-form", post: %{title: "", body: "content"})
           |> render_change() =~ "can&#39;t be blank"

    assert view
           |> form("#post-form", post: %{title: "New Post", body: "Content"})
           |> render_submit()

    assert_patch(view, ~p"/posts")
    assert render(view) =~ "New Post"
  end
end

defmodule MyApp.BlogTest do
  use MyApp.DataCase
  alias MyApp.Blog

  test "create_post/1 with valid data" do
    user = insert(:user)
    assert {:ok, post} = Blog.create_post(%{title: "Test", body: "Content", author_id: user.id})
    assert post.title == "Test"
  end

  test "create_post/1 with invalid data returns error changeset" do
    assert {:error, %Ecto.Changeset{}} = Blog.create_post(%{title: ""})
  end
end
```

## Deployment (Releases)

```elixir
# mix.exs
def project do
  [app: :myapp, releases: [myapp: [steps: [:assemble, :tar]]]]
end
```

```bash
MIX_ENV=prod mix release
_build/prod/rel/myapp/bin/myapp start
```

Releases are self-contained — include the Erlang runtime, compiled BEAM files, and config. Deploy to any Linux server without Elixir/Erlang installed.

**Runtime configuration** via `config/runtime.exs` reads environment variables at startup, not compile time.
