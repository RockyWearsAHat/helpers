# Django

## ORM

### Models

```python
from django.db import models

class Author(models.Model):
    name = models.CharField(max_length=200)
    email = models.EmailField(unique=True)
    bio = models.TextField(blank=True)
    created = models.DateTimeField(auto_now_add=True)

    class Meta:
        ordering = ['name']
        indexes = [models.Index(fields=['email'])]

    def __str__(self):
        return self.name

class Book(models.Model):
    class Genre(models.TextChoices):
        FICTION = 'FI', 'Fiction'
        NONFICTION = 'NF', 'Non-Fiction'
        TECHNICAL = 'TE', 'Technical'

    title = models.CharField(max_length=300)
    author = models.ForeignKey(Author, on_delete=models.CASCADE, related_name='books')
    genre = models.CharField(max_length=2, choices=Genre.choices)
    published = models.DateField()
    price = models.DecimalField(max_digits=6, decimal_places=2)
    tags = models.ManyToManyField('Tag', blank=True)

class Tag(models.Model):
    name = models.CharField(max_length=50, unique=True)
    slug = models.SlugField(unique=True)
```

### QuerySet API

```python
# Basic queries
Book.objects.all()
Book.objects.filter(genre='FI')
Book.objects.exclude(price__gt=50)
Book.objects.get(pk=1)                         # raises DoesNotExist or MultipleObjectsReturned

# Field lookups (double underscore)
Book.objects.filter(title__icontains='python')  # case-insensitive LIKE
Book.objects.filter(price__range=(10, 50))
Book.objects.filter(published__year__gte=2020)
Book.objects.filter(author__name='Alice')       # follow FK
Book.objects.filter(tags__name__in=['python', 'django'])  # M2M traversal

# Chaining (lazy — no DB hit until evaluated)
qs = Book.objects.filter(genre='TE').exclude(price__lt=20).order_by('-published')[:10]

# Q objects (complex lookups)
from django.db.models import Q
Book.objects.filter(Q(genre='FI') | Q(price__lt=10))
Book.objects.filter(~Q(genre='NF'))             # NOT

# F expressions (reference other fields)
from django.db.models import F
Book.objects.filter(price__gt=F('author__avg_price'))
Book.objects.update(price=F('price') * 1.1)     # 10% increase

# Aggregation
from django.db.models import Avg, Count, Sum, Max, Min
Book.objects.aggregate(avg_price=Avg('price'), total=Count('id'))

# Annotation (per-row computed values)
authors = Author.objects.annotate(
    book_count=Count('books'),
    avg_price=Avg('books__price')
).filter(book_count__gt=5)

# Subquery
from django.db.models import Subquery, OuterRef
latest_book = Book.objects.filter(author=OuterRef('pk')).order_by('-published')
Author.objects.annotate(latest_title=Subquery(latest_book.values('title')[:1]))
```

### N+1 Prevention

```python
# select_related — JOIN for ForeignKey/OneToOne (single query)
books = Book.objects.select_related('author').all()

# prefetch_related — separate query + Python join for M2M/reverse FK
authors = Author.objects.prefetch_related('books').all()

# Custom prefetch
from django.db.models import Prefetch
Author.objects.prefetch_related(
    Prefetch('books', queryset=Book.objects.filter(genre='FI'), to_attr='fiction_books')
)
```

### Managers

```python
class PublishedManager(models.Manager):
    def get_queryset(self):
        return super().get_queryset().filter(status='published')

class Article(models.Model):
    objects = models.Manager()          # default
    published = PublishedManager()      # Article.published.all()
```

### Migrations

```bash
python manage.py makemigrations          # generate migration from model changes
python manage.py migrate                  # apply migrations
python manage.py showmigrations          # status
python manage.py sqlmigrate app 0001     # show SQL
python manage.py migrate app 0003       # migrate to specific migration
python manage.py migrate app zero       # rollback all
```

Migrations support: `RunPython` for data migrations, `RunSQL` for raw SQL, `SeparateDatabaseAndState` for complex refactors.

## Views

### Function-Based Views

```python
from django.http import JsonResponse
from django.shortcuts import get_object_or_404, render, redirect
from django.views.decorators.http import require_http_methods

@require_http_methods(["GET"])
def book_list(request):
    query = request.GET.get('q', '')
    books = Book.objects.filter(title__icontains=query) if query else Book.objects.all()
    return render(request, 'books/list.html', {'books': books, 'query': query})

@require_http_methods(["GET", "POST"])
def book_create(request):
    if request.method == 'POST':
        form = BookForm(request.POST)
        if form.is_valid():
            book = form.save(commit=False)
            book.created_by = request.user
            book.save()
            return redirect('book-detail', pk=book.pk)
    else:
        form = BookForm()
    return render(request, 'books/create.html', {'form': form})
```

### Class-Based Views (Generic)

```python
from django.views.generic import ListView, DetailView, CreateView, UpdateView, DeleteView
from django.contrib.auth.mixins import LoginRequiredMixin

class BookListView(ListView):
    model = Book
    paginate_by = 25
    context_object_name = 'books'
    template_name = 'books/list.html'

    def get_queryset(self):
        qs = super().get_queryset().select_related('author')
        q = self.request.GET.get('q')
        if q:
            qs = qs.filter(title__icontains=q)
        return qs

class BookCreateView(LoginRequiredMixin, CreateView):
    model = Book
    fields = ['title', 'author', 'genre', 'published', 'price']
    success_url = reverse_lazy('book-list')

    def form_valid(self, form):
        form.instance.created_by = self.request.user
        return super().form_valid(form)
```

## URL Routing

```python
# urls.py
from django.urls import path, include

urlpatterns = [
    path('', views.home, name='home'),
    path('books/', views.book_list, name='book-list'),
    path('books/<int:pk>/', views.book_detail, name='book-detail'),
    path('books/<slug:slug>/', views.book_by_slug),
    path('api/', include('api.urls')),          # include app URLs
    path('admin/', admin.site.urls),
]
```

Converters: `int`, `str`, `slug`, `uuid`, `path`. Custom converters implement `regex`, `to_python`, `to_url`.

## Middleware

```python
# Custom middleware
class TimingMiddleware:
    def __init__(self, get_response):
        self.get_response = get_response

    def __call__(self, request):
        import time
        start = time.perf_counter()
        response = self.get_response(request)
        duration = time.perf_counter() - start
        response['X-Request-Duration'] = f'{duration:.4f}s'
        return response

    def process_exception(self, request, exception):
        # Called if view raises an exception
        pass
```

## Authentication and Permissions

```python
from django.contrib.auth.decorators import login_required, permission_required
from django.contrib.auth import authenticate, login, logout

@login_required
def profile(request):
    return render(request, 'profile.html')

@permission_required('books.change_book', raise_exception=True)
def edit_book(request, pk):
    ...

# Custom permissions
class Book(models.Model):
    class Meta:
        permissions = [('can_publish', 'Can publish books')]
```

## Signals

```python
from django.db.models.signals import post_save, pre_delete
from django.dispatch import receiver

@receiver(post_save, sender=User)
def create_profile(sender, instance, created, **kwargs):
    if created:
        Profile.objects.create(user=instance)
```

Signals: `pre_save`, `post_save`, `pre_delete`, `post_delete`, `m2m_changed`, `request_started`, `request_finished`.

## Caching

```python
# settings.py
CACHES = {
    'default': {
        'BACKEND': 'django.core.cache.backends.redis.RedisCache',
        'LOCATION': 'redis://127.0.0.1:6379',
    }
}

# View caching
from django.views.decorators.cache import cache_page

@cache_page(60 * 15)  # 15 minutes
def book_list(request):
    ...

# Low-level cache
from django.core.cache import cache
cache.set('book_count', Book.objects.count(), timeout=300)
count = cache.get('book_count')

# Template fragment caching
{% load cache %}
{% cache 600 sidebar request.user.id %}
  ... expensive template ...
{% endcache %}
```

## Django Admin

```python
@admin.register(Book)
class BookAdmin(admin.ModelAdmin):
    list_display = ['title', 'author', 'genre', 'price', 'published']
    list_filter = ['genre', 'published']
    search_fields = ['title', 'author__name']
    prepopulated_fields = {'slug': ('title',)}
    raw_id_fields = ['author']              # avoid loading all authors in dropdown
    readonly_fields = ['created']
    actions = ['mark_as_featured']

    def mark_as_featured(self, request, queryset):
        queryset.update(featured=True)
```

## Django REST Framework

```python
from rest_framework import serializers, viewsets, permissions
from rest_framework.decorators import action
from rest_framework.response import Response

class BookSerializer(serializers.ModelSerializer):
    author_name = serializers.CharField(source='author.name', read_only=True)

    class Meta:
        model = Book
        fields = ['id', 'title', 'author', 'author_name', 'genre', 'price']
        read_only_fields = ['id']

    def validate_price(self, value):
        if value <= 0:
            raise serializers.ValidationError("Price must be positive")
        return value

class BookViewSet(viewsets.ModelViewSet):
    queryset = Book.objects.select_related('author')
    serializer_class = BookSerializer
    permission_classes = [permissions.IsAuthenticatedOrReadOnly]
    filterset_fields = ['genre', 'author']
    search_fields = ['title']
    ordering_fields = ['price', 'published']

    @action(detail=True, methods=['post'])
    def publish(self, request, pk=None):
        book = self.get_object()
        book.status = 'published'
        book.save()
        return Response({'status': 'published'})

# urls.py
from rest_framework.routers import DefaultRouter
router = DefaultRouter()
router.register('books', BookViewSet)
urlpatterns += router.urls
```

## Testing

```python
from django.test import TestCase, Client

class BookTests(TestCase):
    fixtures = ['test_data.json']     # or use factory_boy/model_bakery

    @classmethod
    def setUpTestData(cls):
        cls.author = Author.objects.create(name='Alice', email='a@test.com')
        cls.book = Book.objects.create(title='Test', author=cls.author, genre='FI',
                                        published='2024-01-01', price=29.99)

    def test_book_list(self):
        response = self.client.get('/books/')
        self.assertEqual(response.status_code, 200)
        self.assertContains(response, 'Test')

    def test_book_create_requires_login(self):
        response = self.client.post('/books/create/', {'title': 'New'})
        self.assertEqual(response.status_code, 302)  # redirect to login

    def test_api_list(self):
        response = self.client.get('/api/books/', content_type='application/json')
        data = response.json()
        self.assertEqual(len(data['results']), 1)
```

## Management Commands

```python
# myapp/management/commands/import_books.py
from django.core.management.base import BaseCommand

class Command(BaseCommand):
    help = 'Import books from CSV file'

    def add_arguments(self, parser):
        parser.add_argument('csv_file', type=str)
        parser.add_argument('--dry-run', action='store_true')

    def handle(self, *args, **options):
        # ... implementation
        self.stdout.write(self.style.SUCCESS(f'Imported {count} books'))
```

```bash
python manage.py import_books data.csv --dry-run
python manage.py shell                    # interactive Django shell
python manage.py dbshell                  # database shell
python manage.py inspectdb                # generate models from existing DB
python manage.py collectstatic            # gather static files for deployment
```
