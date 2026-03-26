# Flask

## Core Concepts

Flask is a Python micro-framework — "micro" means it doesn't prescribe database layers, form libraries, or auth. The core provides routing, request handling, templating (Jinja2), and a development server. Everything else comes via extensions.

## Application Factory Pattern

The recommended way to structure Flask apps. Avoids circular imports and enables testing with different configs:

```python
# app/__init__.py
from flask import Flask

def create_app(config_name='default'):
    app = Flask(__name__)
    app.config.from_object(config[config_name])

    # Initialize extensions
    db.init_app(app)
    migrate.init_app(app, db)
    login_manager.init_app(app)

    # Register blueprints
    from app.auth import bp as auth_bp
    app.register_blueprint(auth_bp, url_prefix='/auth')

    from app.api import bp as api_bp
    app.register_blueprint(api_bp, url_prefix='/api/v1')

    return app
```

## Blueprints

Blueprints organize related views, templates, and static files:

```python
# app/auth/__init__.py
from flask import Blueprint
bp = Blueprint('auth', __name__, template_folder='templates')

from app.auth import routes  # import at bottom to avoid circular

# app/auth/routes.py
from app.auth import bp

@bp.route('/login', methods=['GET', 'POST'])
def login():
    ...

@bp.before_request
def require_login():
    ...  # runs before every request in this blueprint
```

### Blueprint Organization

| Feature         | Blueprint                        | Application     |
| --------------- | -------------------------------- | --------------- |
| Scope           | Module-level routes/templates    | Global          |
| Registration    | `app.register_blueprint(bp)`     | N/A             |
| URL prefix      | Per-registration                 | Base URL        |
| Error handlers  | Blueprint-scoped first, then app | Global fallback |
| Template folder | Blueprint-specific               | App-level       |

## Routing

```python
@app.route('/users/<int:user_id>')          # type converter
@app.route('/files/<path:filepath>')         # catches slashes
@app.route('/items/', defaults={'page': 1})  # default values
@app.route('/items/page/<int:page>')
def items(page):
    ...

# Variable converters: string (default), int, float, path, uuid
# Methods
@app.route('/submit', methods=['GET', 'POST'])
```

### URL Building

```python
from flask import url_for
url_for('auth.login')                  # blueprint.endpoint
url_for('static', filename='style.css')
url_for('user_profile', id=42, _external=True)  # absolute URL
```

## Request & Response

### Request Object

```python
from flask import request

request.method          # 'GET', 'POST', etc.
request.args            # query string (ImmutableMultiDict)
request.form            # form data
request.json            # parsed JSON body (or None)
request.get_json(force=True)  # parse even without Content-Type
request.files           # uploaded files (FileStorage objects)
request.headers         # request headers
request.cookies         # cookies dict
request.remote_addr     # client IP
request.url             # full URL
request.is_json         # True if Content-Type is JSON
request.data            # raw bytes
```

### Response Patterns

```python
from flask import jsonify, make_response, redirect, abort

# Tuple shorthand: (body, status, headers)
return 'Created', 201
return jsonify(data), 200, {'X-Custom': 'value'}

# Full control
resp = make_response(render_template('page.html'))
resp.set_cookie('session_id', value, httponly=True, secure=True, samesite='Lax')
return resp

# Errors
abort(404)  # raises HTTPException
abort(403, description='Access denied')
```

## Application Context vs Request Context

This is Flask's most confusing concept:

| Context     | Proxy                | Available When                             | Use Case               |
| ----------- | -------------------- | ------------------------------------------ | ---------------------- |
| Application | `current_app`, `g`   | Inside request OR `with app.app_context()` | Config, DB connections |
| Request     | `request`, `session` | Only during request handling               | Request data, cookies  |

```python
from flask import g, current_app

# g is per-request storage — reset between requests
@app.before_request
def load_user():
    g.user = User.query.get(session.get('user_id'))

# current_app is the application proxy
@bp.route('/info')
def info():
    return current_app.config['SECRET_KEY']  # don't actually do this

# Manual context (for CLI, tests, background tasks)
with app.app_context():
    db.create_all()
```

## Sessions & Flash Messages

```python
from flask import session, flash, get_flashed_messages

app.secret_key = os.environ['SECRET_KEY']  # required for sessions

@app.route('/login', methods=['POST'])
def login():
    session['user_id'] = user.id
    session.permanent = True        # uses PERMANENT_SESSION_LIFETIME
    flash('Login successful', 'success')  # category is optional
    return redirect(url_for('index'))

# In template:
# {% for category, message in get_flashed_messages(with_categories=true) %}
#   <div class="alert alert-{{ category }}">{{ message }}</div>
# {% endfor %}
```

Sessions are signed cookies by default — not encrypted. For server-side sessions use Flask-Session.

## Jinja2 Templates

```html
{# Inheritance #} {% extends "base.html" %} {% block content %}
<h1>{{ title | escape }}</h1>
{% for item in items %}
<div class="{{ loop.cycle('odd', 'even') }}">
  {{ item.name }} — {{ item.price | round(2) }}
</div>
{% else %}
<p>No items</p>
{% endfor %} {% endblock %} {# Macros #} {% macro input(name, type='text',
value='') %}
<input type="{{ type }}" name="{{ name }}" value="{{ value }}" />
{% endmacro %} {{ input('email', type='email') }}
```

### Auto-escaping

Jinja2 auto-escapes HTML in `{{ }}` expressions. Mark trusted content explicitly:

```python
from markupsafe import Markup
Markup('<strong>safe</strong>')  # won't be escaped

# In template: {{ content | safe }}  — use sparingly, XSS risk
```

## Extensions

### SQLAlchemy (Flask-SQLAlchemy)

```python
from flask_sqlalchemy import SQLAlchemy
db = SQLAlchemy()

class User(db.Model):
    id = db.Column(db.Integer, primary_key=True)
    email = db.Column(db.String(120), unique=True, nullable=False)
    posts = db.relationship('Post', backref='author', lazy='dynamic')

# Query
users = User.query.filter_by(active=True).order_by(User.email).all()
user = db.session.get(User, 1)  # SQLAlchemy 2.0 style
```

### Flask-Migrate (Alembic)

```bash
flask db init          # one-time setup
flask db migrate -m "add users table"
flask db upgrade       # apply migrations
flask db downgrade     # rollback
```

### Flask-Login

```python
from flask_login import LoginManager, login_user, logout_user, login_required, current_user

login_manager = LoginManager()
login_manager.login_view = 'auth.login'

@login_manager.user_loader
def load_user(user_id):
    return db.session.get(User, int(user_id))

@app.route('/dashboard')
@login_required
def dashboard():
    return render_template('dashboard.html', user=current_user)
```

### Flask-WTF (Forms)

```python
from flask_wtf import FlaskForm
from wtforms import StringField, PasswordField
from wtforms.validators import DataRequired, Email, Length

class LoginForm(FlaskForm):
    email = StringField('Email', validators=[DataRequired(), Email()])
    password = PasswordField('Password', validators=[DataRequired(), Length(min=8)])

@app.route('/login', methods=['GET', 'POST'])
def login():
    form = LoginForm()
    if form.validate_on_submit():   # checks CSRF + validators
        ...
    return render_template('login.html', form=form)
```

### Flask-CORS

```python
from flask_cors import CORS
CORS(app, resources={r"/api/*": {"origins": ["https://example.com"]}})
```

### Flask-RESTful / Marshmallow

```python
from flask_marshmallow import Marshmallow
ma = Marshmallow(app)

class UserSchema(ma.SQLAlchemyAutoSchema):
    class Meta:
        model = User
        load_instance = True
        exclude = ('password_hash',)
```

## Error Handlers

```python
@app.errorhandler(404)
def not_found(error):
    return render_template('404.html'), 404

@app.errorhandler(ValidationError)  # handle exception classes
def validation_error(error):
    return jsonify(error=str(error)), 422
```

## CLI Commands

```python
import click

@app.cli.command('seed-db')
@click.option('--count', default=10)
def seed_db(count):
    """Seed database with test data."""
    for i in range(count):
        db.session.add(User(email=f'user{i}@test.com'))
    db.session.commit()
    click.echo(f'Seeded {count} users')
```

```bash
flask seed-db --count 50
```

## Configuration Patterns

```python
class Config:
    SECRET_KEY = os.environ.get('SECRET_KEY', 'dev-key-change-me')
    SQLALCHEMY_DATABASE_URI = os.environ.get('DATABASE_URL', 'sqlite:///app.db')
    SQLALCHEMY_TRACK_MODIFICATIONS = False

class ProductionConfig(Config):
    DEBUG = False
    SQLALCHEMY_DATABASE_URI = os.environ['DATABASE_URL']

class TestingConfig(Config):
    TESTING = True
    SQLALCHEMY_DATABASE_URI = 'sqlite://'  # in-memory
    WTF_CSRF_ENABLED = False

config = {
    'development': Config,
    'production': ProductionConfig,
    'testing': TestingConfig,
    'default': Config,
}
```

## Testing

```python
import pytest
from app import create_app, db

@pytest.fixture
def app():
    app = create_app('testing')
    with app.app_context():
        db.create_all()
        yield app
        db.drop_all()

@pytest.fixture
def client(app):
    return app.test_client()

def test_login(client):
    resp = client.post('/auth/login', json={
        'email': 'test@example.com',
        'password': 'password123',
    })
    assert resp.status_code == 200
    assert resp.json['token']
```

## Async Views (Flask 2.0+)

```python
@app.route('/data')
async def get_data():
    result = await async_db_query()
    return jsonify(result)
```

Flask 2.0+ supports `async def` views but runs them in a thread pool under the hood. For truly async I/O, consider Quart (Flask-compatible, ASGI-native).

## Deployment

### Gunicorn (recommended)

```bash
gunicorn -w 4 -b 0.0.0.0:8000 "app:create_app()"
# Workers = (2 × CPU cores) + 1
# Add --preload for shared memory (careful with DB connections)
```

### uWSGI

```ini
[uwsgi]
module = app:create_app()
master = true
processes = 4
socket = /tmp/app.sock
chmod-socket = 660
vacuum = true
die-on-term = true
```

### Signals

```python
from blinker import Namespace
signals = Namespace()
user_registered = signals.signal('user-registered')

# Send
user_registered.send(app, user=new_user)

# Receive
@user_registered.connect_via(app)
def on_user_registered(sender, user, **kwargs):
    send_welcome_email(user)
```

## Common Gotchas

- Circular imports: always import inside functions or at module bottom
- `g` doesn't persist between requests — use session or database
- Debug mode in production exposes interactive debugger (RCE vulnerability)
- `SQLALCHEMY_TRACK_MODIFICATIONS = False` saves significant memory
- File uploads need `enctype="multipart/form-data"` on the form
- `url_for` requires active application context
