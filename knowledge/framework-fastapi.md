# FastAPI

## Core Concepts

FastAPI is a modern Python web framework built on Starlette (ASGI) and Pydantic (validation). It generates OpenAPI/Swagger docs automatically from type hints.

```python
from fastapi import FastAPI

app = FastAPI(title="My API", version="1.0.0")

@app.get("/")
async def root():
    return {"message": "Hello"}
```

## Pydantic Models (v2)

```python
from pydantic import BaseModel, Field, field_validator, model_validator
from datetime import datetime
from typing import Annotated

class UserCreate(BaseModel):
    name: str = Field(min_length=1, max_length=100)
    email: str = Field(pattern=r'^[\w.-]+@[\w.-]+\.\w+$')
    age: int = Field(ge=0, le=150)
    tags: list[str] = Field(default_factory=list, max_length=10)

    @field_validator('name')
    @classmethod
    def name_must_not_be_empty(cls, v: str) -> str:
        if not v.strip():
            raise ValueError('Name cannot be blank')
        return v.strip()

    @model_validator(mode='after')
    def check_consistency(self) -> 'UserCreate':
        # Cross-field validation
        return self

    model_config = ConfigDict(
        str_strip_whitespace=True,
        json_schema_extra={"example": {"name": "Alice", "email": "alice@example.com", "age": 30}},
    )

class UserResponse(BaseModel):
    id: int
    name: str
    email: str
    created_at: datetime

    model_config = ConfigDict(from_attributes=True)  # Works with ORM objects
```

## Parameters

### Path, Query, Body

```python
from fastapi import Path, Query, Body

@app.get("/items/{item_id}")
async def get_item(
    item_id: int = Path(gt=0, description="Item ID"),
    q: str | None = Query(None, min_length=1, max_length=50),
    skip: int = Query(0, ge=0),
    limit: int = Query(20, ge=1, le=100),
):
    ...

@app.post("/items")
async def create_item(
    item: ItemCreate,                                  # body (auto-detected from Pydantic model)
    importance: int = Body(gt=0),                      # additional body field
):
    ...

@app.put("/items/{item_id}")
async def update_item(
    item_id: int,                                      # path param
    item: ItemUpdate,                                  # request body
    q: str | None = None,                              # query param
):
    ...
```

### Headers, Cookies, Form Data

```python
from fastapi import Header, Cookie, Form, UploadFile, File

@app.get("/items")
async def get_items(
    user_agent: str = Header(),
    token: str | None = Cookie(None),
):
    ...

@app.post("/login")
async def login(
    username: str = Form(),
    password: str = Form(),
):
    ...

@app.post("/upload")
async def upload(
    file: UploadFile = File(description="File to upload"),
    description: str = Form(""),
):
    contents = await file.read()
    return {"filename": file.filename, "size": len(contents)}

@app.post("/upload-multiple")
async def upload_multiple(files: list[UploadFile]):
    return {"count": len(files)}
```

## Dependency Injection

FastAPI's DI system is one of its strongest features. Dependencies can be functions, classes, or generators:

```python
from fastapi import Depends

# Simple dependency
async def get_db():
    db = SessionLocal()
    try:
        yield db           # generator dependency — cleanup after response
    finally:
        db.close()

# Dependency with sub-dependencies
async def get_current_user(
    token: str = Depends(oauth2_scheme),
    db: Session = Depends(get_db),
) -> User:
    user = await authenticate(token, db)
    if not user:
        raise HTTPException(status_code=401, detail="Invalid credentials")
    return user

# Class dependency
class Pagination:
    def __init__(self, skip: int = Query(0, ge=0), limit: int = Query(20, ge=1, le=100)):
        self.skip = skip
        self.limit = limit

@app.get("/items")
async def list_items(
    db: Session = Depends(get_db),
    user: User = Depends(get_current_user),
    pagination: Pagination = Depends(),
):
    return db.query(Item).filter(Item.owner_id == user.id).offset(pagination.skip).limit(pagination.limit).all()

# Global dependency
app = FastAPI(dependencies=[Depends(verify_api_key)])
```

## Response Models and Status Codes

```python
from fastapi import status
from fastapi.responses import JSONResponse, StreamingResponse, FileResponse

@app.post("/items", response_model=ItemResponse, status_code=status.HTTP_201_CREATED)
async def create_item(item: ItemCreate, db: Session = Depends(get_db)):
    db_item = Item(**item.model_dump())
    db.add(db_item)
    db.commit()
    return db_item

@app.get("/items", response_model=list[ItemResponse])
async def list_items(db: Session = Depends(get_db)):
    return db.query(Item).all()

# Exclude fields from response
@app.get("/users/me", response_model=UserResponse, response_model_exclude={"email"})
async def get_me(user: User = Depends(get_current_user)):
    return user
```

## Exception Handling

```python
from fastapi import HTTPException
from fastapi.exceptions import RequestValidationError
from fastapi.responses import JSONResponse

# Standard HTTP exceptions
@app.get("/items/{item_id}")
async def get_item(item_id: int, db: Session = Depends(get_db)):
    item = db.query(Item).get(item_id)
    if not item:
        raise HTTPException(status_code=404, detail="Item not found")
    return item

# Custom exception handler
class BusinessError(Exception):
    def __init__(self, message: str, code: str):
        self.message = message
        self.code = code

@app.exception_handler(BusinessError)
async def business_error_handler(request, exc: BusinessError):
    return JSONResponse(status_code=422, content={"error": exc.code, "message": exc.message})

@app.exception_handler(RequestValidationError)
async def validation_handler(request, exc: RequestValidationError):
    return JSONResponse(status_code=422, content={"detail": exc.errors()})
```

## Middleware

```python
from fastapi.middleware.cors import CORSMiddleware
from starlette.middleware.base import BaseHTTPMiddleware
import time

# CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["https://myapp.com"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Custom middleware
class TimingMiddleware(BaseHTTPMiddleware):
    async def dispatch(self, request, call_next):
        start = time.perf_counter()
        response = await call_next(request)
        response.headers["X-Process-Time"] = f"{time.perf_counter() - start:.4f}"
        return response

app.add_middleware(TimingMiddleware)

# Or decorator style
@app.middleware("http")
async def add_custom_header(request, call_next):
    response = await call_next(request)
    response.headers["X-Custom"] = "value"
    return response
```

## Security

### OAuth2 + JWT

```python
from fastapi.security import OAuth2PasswordBearer, OAuth2PasswordRequestForm
import jwt

oauth2_scheme = OAuth2PasswordBearer(tokenUrl="auth/token")

def create_token(user_id: int) -> str:
    payload = {"sub": str(user_id), "exp": datetime.utcnow() + timedelta(hours=24)}
    return jwt.encode(payload, SECRET_KEY, algorithm="HS256")

async def get_current_user(token: str = Depends(oauth2_scheme)) -> User:
    try:
        payload = jwt.decode(token, SECRET_KEY, algorithms=["HS256"])
        user_id = int(payload["sub"])
    except jwt.InvalidTokenError:
        raise HTTPException(status_code=401, detail="Invalid token")
    return await get_user(user_id)

@app.post("/auth/token")
async def login(form: OAuth2PasswordRequestForm = Depends()):
    user = await authenticate_user(form.username, form.password)
    if not user:
        raise HTTPException(status_code=401, detail="Incorrect credentials")
    return {"access_token": create_token(user.id), "token_type": "bearer"}
```

## WebSockets

```python
from fastapi import WebSocket, WebSocketDisconnect

class ConnectionManager:
    def __init__(self):
        self.active: list[WebSocket] = []

    async def connect(self, ws: WebSocket):
        await ws.accept()
        self.active.append(ws)

    def disconnect(self, ws: WebSocket):
        self.active.remove(ws)

    async def broadcast(self, message: str):
        for conn in self.active:
            await conn.send_text(message)

manager = ConnectionManager()

@app.websocket("/ws/{room}")
async def websocket_endpoint(ws: WebSocket, room: str):
    await manager.connect(ws)
    try:
        while True:
            data = await ws.receive_text()
            await manager.broadcast(f"{room}: {data}")
    except WebSocketDisconnect:
        manager.disconnect(ws)
```

## Background Tasks

```python
from fastapi import BackgroundTasks

def send_notification(email: str, message: str):
    # Runs after response is sent
    mailer.send(to=email, body=message)

@app.post("/orders")
async def create_order(order: Order, bg: BackgroundTasks):
    result = await process_order(order)
    bg.add_task(send_notification, order.email, f"Order {result.id} confirmed")
    return result
```

For heavy background work, use Celery, ARQ, or Dramatiq instead.

## APIRouter

```python
# routers/items.py
from fastapi import APIRouter

router = APIRouter(prefix="/items", tags=["items"], dependencies=[Depends(verify_token)])

@router.get("/")
async def list_items(): ...

@router.get("/{item_id}")
async def get_item(item_id: int): ...

# main.py
from routers import items, users
app.include_router(items.router)
app.include_router(users.router, prefix="/api/v1")
```

## Lifespan Events

```python
from contextlib import asynccontextmanager

@asynccontextmanager
async def lifespan(app: FastAPI):
    # Startup
    await database.connect()
    redis_pool = await create_redis_pool()
    app.state.redis = redis_pool
    yield
    # Shutdown
    await database.disconnect()
    await redis_pool.close()

app = FastAPI(lifespan=lifespan)
```

## Testing

```python
from fastapi.testclient import TestClient
import pytest

@pytest.fixture
def client():
    # Override dependencies for testing
    app.dependency_overrides[get_db] = lambda: test_db
    app.dependency_overrides[get_current_user] = lambda: test_user
    with TestClient(app) as c:
        yield c
    app.dependency_overrides.clear()

def test_create_item(client):
    response = client.post("/items", json={"name": "Test", "price": 9.99})
    assert response.status_code == 201
    assert response.json()["name"] == "Test"

def test_get_item_not_found(client):
    response = client.get("/items/999")
    assert response.status_code == 404

# Async tests
@pytest.mark.anyio
async def test_async():
    async with AsyncClient(app=app, base_url="http://test") as ac:
        response = await ac.get("/items")
        assert response.status_code == 200
```

## Database Integration (SQLAlchemy Async)

```python
from sqlalchemy.ext.asyncio import create_async_engine, AsyncSession
from sqlalchemy.orm import sessionmaker

engine = create_async_engine("postgresql+asyncpg://user:pass@localhost/db")
async_session = sessionmaker(engine, class_=AsyncSession, expire_on_commit=False)

async def get_db():
    async with async_session() as session:
        yield session

@app.get("/items")
async def list_items(db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(Item).options(selectinload(Item.tags)))
    return result.scalars().all()
```

## Streaming Responses

```python
from fastapi.responses import StreamingResponse

@app.get("/stream")
async def stream_data():
    async def generate():
        for i in range(100):
            yield f"data: {i}\n\n"
            await asyncio.sleep(0.1)
    return StreamingResponse(generate(), media_type="text/event-stream")
```
