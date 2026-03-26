# Ruby on Rails

## Convention Over Configuration

Rails makes decisions for you: file placement, naming, database schema. Follow conventions and you write minimal boilerplate.

| Convention          | Example                                                      |
| ------------------- | ------------------------------------------------------------ |
| Model → Table       | `User` model → `users` table                                 |
| Controller → Routes | `UsersController` → `/users/*`                               |
| FK naming           | `user_id` in `orders` table                                  |
| Join table          | `roles_users` (alphabetical, plural)                         |
| Primary key         | `id` (auto-increment)                                        |
| Timestamps          | `created_at`, `updated_at` (auto-managed)                    |
| Template            | `app/views/users/index.html.erb` for `UsersController#index` |

## Active Record

### Associations

```ruby
class User < ApplicationRecord
  has_many :posts, dependent: :destroy
  has_many :comments
  has_many :commented_posts, through: :comments, source: :post
  has_one :profile, dependent: :destroy
  has_and_belongs_to_many :roles

  # Polymorphic
  has_many :images, as: :imageable
end

class Post < ApplicationRecord
  belongs_to :user
  has_many :comments, dependent: :destroy
  has_many :tags, through: :taggings
end

class Image < ApplicationRecord
  belongs_to :imageable, polymorphic: true  # imageable_type, imageable_id
end
```

### Validations

```ruby
class User < ApplicationRecord
  validates :name, presence: true, length: { minimum: 2, maximum: 100 }
  validates :email, presence: true, uniqueness: { case_sensitive: false },
                    format: { with: URI::MailTo::EMAIL_REGEXP }
  validates :age, numericality: { greater_than_or_equal_to: 0 }, allow_nil: true
  validates :username, exclusion: { in: %w[admin root superuser] }
  validate :custom_validation

  private

  def custom_validation
    errors.add(:base, "Account suspended") if suspended? && active_changed?(to: true)
  end
end
```

### Callbacks

```ruby
class Order < ApplicationRecord
  before_validation :normalize_data
  after_create :send_confirmation
  before_destroy :check_cancellable
  after_commit :sync_to_external_system, on: [:create, :update]
  # after_commit is safer than after_save for external side effects
  # (only fires after DB transaction commits)
end
```

### Scopes and Query Interface

```ruby
class Post < ApplicationRecord
  scope :published, -> { where(status: :published) }
  scope :recent, -> { order(created_at: :desc) }
  scope :by_author, ->(user) { where(user: user) }
  scope :popular, -> { where("views_count > ?", 100) }

  # Chainable
  # Post.published.recent.by_author(user).limit(10)
end
```

```ruby
# Query interface
User.where(active: true)
User.where("age > ? AND role = ?", 18, "member")  # parameterized (safe)
User.where(created_at: 1.week.ago..)               # range
User.where.not(role: "admin")
User.order(name: :asc, created_at: :desc)
User.limit(20).offset(40)
User.includes(:posts).where(posts: { status: "published" })  # eager load + filter
User.joins(:posts).group(:id).having("COUNT(posts.id) > 5") # aggregate
User.pluck(:name, :email)                           # returns arrays, not AR objects
User.find_each(batch_size: 1000) { |u| process(u) } # memory-efficient iteration
User.exists?(email: "alice@test.com")

# Calculations
Order.average(:total)
Order.sum(:total)
Order.group(:status).count
```

### Migrations

```ruby
class CreatePosts < ActiveRecord::Migration[7.1]
  def change
    create_table :posts do |t|
      t.references :user, null: false, foreign_key: true
      t.string :title, null: false, limit: 300
      t.text :body
      t.integer :status, default: 0, null: false
      t.jsonb :metadata, default: {}
      t.timestamps
    end

    add_index :posts, [:user_id, :created_at]
    add_index :posts, :status
  end
end
```

```bash
rails generate migration AddSlugToPosts slug:string:uniq
rails db:migrate
rails db:rollback STEP=2
rails db:migrate:status
rails db:seed
```

## Controllers

```ruby
class PostsController < ApplicationController
  before_action :authenticate_user!, except: [:index, :show]
  before_action :set_post, only: [:show, :edit, :update, :destroy]
  before_action :authorize_post!, only: [:edit, :update, :destroy]

  def index
    @posts = Post.published.recent.includes(:user).page(params[:page]).per(20)
  end

  def show
  end

  def create
    @post = current_user.posts.build(post_params)
    if @post.save
      redirect_to @post, notice: "Post created."
    else
      render :new, status: :unprocessable_entity
    end
  end

  def update
    if @post.update(post_params)
      redirect_to @post, notice: "Post updated."
    else
      render :edit, status: :unprocessable_entity
    end
  end

  def destroy
    @post.destroy
    redirect_to posts_path, notice: "Post deleted.", status: :see_other
  end

  private

  def set_post
    @post = Post.find(params[:id])
  end

  def post_params
    params.require(:post).permit(:title, :body, :status, tag_ids: [])
  end
end
```

## Routing

```ruby
Rails.application.routes.draw do
  root "pages#home"

  resources :posts do
    resources :comments, only: [:create, :destroy]
    member do
      post :publish
    end
    collection do
      get :drafts
    end
  end

  namespace :api do
    namespace :v1 do
      resources :users, only: [:index, :show, :create]
    end
  end

  # Concerns (reusable route groups)
  concern :commentable do
    resources :comments, only: [:create, :destroy]
  end
  resources :articles, concerns: :commentable
  resources :photos, concerns: :commentable

  # Direct routes
  get "up", to: "health#show"
  get "login", to: "sessions#new"
  post "login", to: "sessions#create"
  delete "logout", to: "sessions#destroy"
end
```

## Views (ERB)

```erb
<%# app/views/posts/index.html.erb %>
<h1>Posts</h1>

<%= render @posts %>  <%# renders _post.html.erb partial for each %>

<%= paginate @posts %>

<%# _post.html.erb partial %>
<article id="<%= dom_id(post) %>">
  <h2><%= link_to post.title, post %></h2>
  <p>by <%= post.user.name %> · <%= time_ago_in_words(post.created_at) %> ago</p>
  <%= simple_format(post.body) %>
</article>
```

### Form Builders

```erb
<%= form_with(model: @post) do |f| %>
  <%= f.label :title %>
  <%= f.text_field :title, class: "input" %>

  <%= f.label :body %>
  <%= f.text_area :body, rows: 10 %>

  <%= f.collection_select :category_id, Category.all, :id, :name, prompt: "Select..." %>

  <%= f.submit class: "btn btn-primary" %>
<% end %>
```

## Hotwire (Turbo + Stimulus)

### Turbo Frames

```erb
<%= turbo_frame_tag "post_#{@post.id}" do %>
  <h2><%= @post.title %></h2>
  <%= link_to "Edit", edit_post_path(@post) %>
<% end %>
```

Turbo Frames replace only the matching frame on navigation — no full page reload.

### Turbo Streams

```ruby
# Controller
def create
  @comment = @post.comments.build(comment_params)
  if @comment.save
    respond_to do |format|
      format.turbo_stream  # renders create.turbo_stream.erb
      format.html { redirect_to @post }
    end
  end
end
```

```erb
<%# create.turbo_stream.erb %>
<%= turbo_stream.append "comments" do %>
  <%= render @comment %>
<% end %>
```

Actions: `append`, `prepend`, `replace`, `update`, `remove`, `before`, `after`.

### Stimulus

```html
<div data-controller="toggle">
  <button data-action="click->toggle#switch">Toggle</button>
  <div data-toggle-target="content" class="hidden">Content</div>
</div>
```

```js
// app/javascript/controllers/toggle_controller.js
import { Controller } from "@hotwired/stimulus";

export default class extends Controller {
  static targets = ["content"];

  switch() {
    this.contentTarget.classList.toggle("hidden");
  }
}
```

## Action Cable (WebSockets)

```ruby
# app/channels/chat_channel.rb
class ChatChannel < ApplicationCable::Channel
  def subscribed
    stream_from "chat_#{params[:room]}"
  end

  def speak(data)
    Message.create!(body: data["body"], user: current_user, room: params[:room])
    ActionCable.server.broadcast("chat_#{params[:room]}", {
      body: data["body"], user: current_user.name
    })
  end
end
```

## Active Job

```ruby
class ProcessOrderJob < ApplicationJob
  queue_as :default
  retry_on ActiveRecord::Deadlocked, wait: 5.seconds, attempts: 3
  discard_on ActiveJob::DeserializationError

  def perform(order)
    order.process!
    OrderMailer.confirmation(order).deliver_later
  end
end

# Enqueue
ProcessOrderJob.perform_later(order)
ProcessOrderJob.set(wait: 1.hour).perform_later(order)
```

Backends: Sidekiq (Redis, recommended), Solid Queue (DB-backed, Rails 8 default), Good Job, Delayed Job.

## Testing

```ruby
# Model test
class UserTest < ActiveSupport::TestCase
  test "validates email uniqueness" do
    User.create!(name: "Alice", email: "a@test.com")
    duplicate = User.new(name: "Bob", email: "a@test.com")
    assert_not duplicate.valid?
    assert_includes duplicate.errors[:email], "has already been taken"
  end
end

# Controller test
class PostsControllerTest < ActionDispatch::IntegrationTest
  test "should create post" do
    sign_in users(:alice)
    assert_difference("Post.count") do
      post posts_url, params: { post: { title: "New", body: "Content" } }
    end
    assert_redirected_to post_url(Post.last)
  end
end

# System test (browser)
class PostFlowTest < ApplicationSystemTestCase
  test "creating a post" do
    sign_in users(:alice)
    visit new_post_path
    fill_in "Title", with: "My Post"
    fill_in "Body", with: "Some content"
    click_on "Create Post"
    assert_text "Post created"
  end
end
```

## Credentials

```bash
rails credentials:edit              # edit encrypted credentials
rails credentials:edit --environment production
```

```ruby
# Access in code
Rails.application.credentials.secret_key_base
Rails.application.credentials.dig(:aws, :access_key_id)
```

## Rails Console

```bash
rails console               # IRB with Rails loaded
rails console --sandbox     # changes rolled back on exit
```

```ruby
User.count
User.where(active: true).to_sql   # see generated SQL
User.find(1).update!(name: "New") # modify in place
```
