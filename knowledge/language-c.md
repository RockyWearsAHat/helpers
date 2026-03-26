# C Conventions and Idioms

## The Reality of C

C is a sharp tool. It gives you absolute control and absolute responsibility. Every byte of memory, every pointer dereference, every buffer size — your problem.

**C is appropriate when:**

- Writing operating systems, kernels, embedded firmware.
- Maximum performance with minimal overhead.
- Interfacing with hardware directly.
- Building libraries consumed by every other language (FFI).

## Memory Management

### malloc/free Discipline

```c
// Allocate
int *arr = malloc(n * sizeof(*arr));  // sizeof(*arr), not sizeof(int)
if (arr == NULL) {
    // ALWAYS check malloc return value
    perror("malloc failed");
    return -1;
}

// Use the memory
for (size_t i = 0; i < n; i++) {
    arr[i] = (int)i;
}

// Free when done
free(arr);
arr = NULL;  // Prevent use-after-free (dangling pointer)
```

**Rules:**

1. **Every `malloc` must have a corresponding `free`.** No exceptions.
2. **Always check `malloc` return value.** It can return NULL.
3. **Set freed pointers to NULL.** Prevents double-free and use-after-free.
4. **Use `sizeof(*ptr)` not `sizeof(Type)`.** Survives type changes.
5. **Never cast `malloc` return in C.** `void*` converts implicitly: `int *p = malloc(sizeof(*p));`
6. **Use `calloc` for zero-initialized memory.** `calloc(n, sizeof(*arr))` — also checks for overflow.

### Buffer Overflow Prevention

```c
// ❌ NEVER use these (unbounded)
gets(buffer);                     // Removed in C11
strcpy(dest, src);                // No bounds checking
strcat(dest, src);                // No bounds checking
sprintf(buf, "%s", input);       // No bounds checking
scanf("%s", buffer);             // No bounds checking

// ✅ Use bounded alternatives
fgets(buffer, sizeof(buffer), stdin);
strncpy(dest, src, sizeof(dest) - 1);
dest[sizeof(dest) - 1] = '\0';   // strncpy doesn't guarantee null-termination
strncat(dest, src, sizeof(dest) - strlen(dest) - 1);
snprintf(buf, sizeof(buf), "%s", input);

// ✅ Even better: use strlcpy/strlcat where available (BSD, macOS)
strlcpy(dest, src, sizeof(dest));  // Always null-terminates
strlcat(dest, src, sizeof(dest));  // Always null-terminates
```

### Common Memory Bugs

| Bug                      | Description                   | Consequence                     |
| ------------------------ | ----------------------------- | ------------------------------- |
| Buffer overflow          | Writing past allocated bounds | Code execution, crashes         |
| Use-after-free           | Accessing freed memory        | Undefined behavior              |
| Double free              | Freeing same pointer twice    | Heap corruption                 |
| Memory leak              | Not freeing allocated memory  | Resource exhaustion             |
| Null dereference         | Dereferencing NULL pointer    | Segfault/crash                  |
| Uninitialized read       | Reading before writing        | Unpredictable values            |
| Integer overflow in size | `malloc(n * size)` overflows  | Tiny allocation, later overflow |

## Defensive Programming

```c
// Assert preconditions (development builds)
#include <assert.h>

void process_buffer(const char *buf, size_t len) {
    assert(buf != NULL);
    assert(len > 0);
    assert(len <= MAX_BUFFER_SIZE);
    // ... proceed safely
}

// Validate all inputs at function boundaries
int parse_port(const char *str) {
    if (str == NULL) return -1;

    char *end;
    errno = 0;
    long val = strtol(str, &end, 10);

    if (errno != 0 || end == str || *end != '\0') return -1;
    if (val < 0 || val > 65535) return -1;

    return (int)val;
}

// Check return values of EVERY system call
int fd = open(path, O_RDONLY);
if (fd == -1) {
    perror("open");
    return -1;
}

ssize_t n = read(fd, buf, sizeof(buf));
if (n == -1) {
    perror("read");
    close(fd);
    return -1;
}
```

## Integer Safety

```c
// Check for overflow BEFORE arithmetic
#include <stdint.h>
#include <limits.h>

// Safe addition check
if (a > 0 && b > INT_MAX - a) {
    // Would overflow
    return -1;
}
int result = a + b;

// Safe multiplication for allocation sizes
if (n > 0 && count > SIZE_MAX / n) {
    // Would overflow
    return NULL;
}
void *p = malloc(count * n);

// Use fixed-width types for known sizes
uint8_t byte_val;
int32_t signed_val;
uint64_t large_val;
size_t array_index;    // For sizes and indices
ptrdiff_t pointer_diff; // For pointer arithmetic
```

## String Handling

```c
// String length — O(n), cache the result
size_t len = strlen(str);

// Safe string building
int ret = snprintf(buf, sizeof(buf), "name=%s&age=%d", name, age);
if (ret < 0 || (size_t)ret >= sizeof(buf)) {
    // Truncated or error
    handle_error();
}

// Dynamic string building
char *result = NULL;
int ret = asprintf(&result, "Hello, %s!", name);  // POSIX extension
if (ret == -1) {
    // allocation failed
}
// ... use result ...
free(result);
```

## Struct Design

```c
// Opaque types (information hiding)
// In header (public interface):
typedef struct database database_t;
database_t *db_open(const char *path);
void db_close(database_t *db);
int db_query(database_t *db, const char *sql);

// In implementation (private):
struct database {
    int fd;
    char *path;
    // ... internal details hidden
};

// Designated initializers (C99+)
struct config cfg = {
    .port = 8080,
    .host = "localhost",
    .max_connections = 100,
};

// Flexible array member (variable-length struct)
struct packet {
    uint32_t length;
    uint8_t data[];  // Must be last member
};

struct packet *pkt = malloc(sizeof(*pkt) + data_len);
pkt->length = data_len;
memcpy(pkt->data, source, data_len);
```

## Error Handling Patterns

```c
// Pattern 1: Return error code, output via pointer
int parse_config(const char *path, config_t *out) {
    if (path == NULL || out == NULL) return -EINVAL;

    FILE *f = fopen(path, "r");
    if (f == NULL) return -errno;

    // ... parse ...

    fclose(f);
    return 0;  // Success
}

// Pattern 2: Goto cleanup (standard for complex functions)
int process_file(const char *path) {
    int ret = -1;
    FILE *f = NULL;
    char *buf = NULL;

    f = fopen(path, "r");
    if (f == NULL) goto cleanup;

    buf = malloc(BUF_SIZE);
    if (buf == NULL) goto cleanup;

    // ... do work ...

    ret = 0;  // Success

cleanup:
    free(buf);
    if (f) fclose(f);
    return ret;
}

// Pattern 3: errno for system-level errors
if (write(fd, data, len) == -1) {
    fprintf(stderr, "write failed: %s\n", strerror(errno));
    return -1;
}
```

## Build & Tooling

```makefile
# Compiler flags for safety
CFLAGS = -Wall -Wextra -Werror -pedantic -std=c17
CFLAGS += -Wshadow -Wconversion -Wstrict-prototypes
CFLAGS += -fstack-protector-strong -D_FORTIFY_SOURCE=2
CFLAGS += -fsanitize=address,undefined  # Development builds
```

| Tool                                   | Purpose                                 |
| -------------------------------------- | --------------------------------------- |
| **AddressSanitizer (ASan)**            | Buffer overflows, use-after-free, leaks |
| **UndefinedBehaviorSanitizer (UBSan)** | Integer overflow, null deref, alignment |
| **Valgrind**                           | Memory leaks, uninitialized reads       |
| **cppcheck**                           | Static analysis                         |
| **clang-tidy**                         | Static analysis + modernization         |
| **Coverity**                           | Deep static analysis (commercial)       |
| **AFL / libFuzzer**                    | Fuzz testing                            |
| **clang-format**                       | Code formatting                         |

## CERT C Coding Standard (Key Rules)

- **ARR38-C**: Guarantee array indices are within valid range.
- **STR31-C**: Guarantee string storage has sufficient space for data and null terminator.
- **MEM30-C**: Do not access freed memory.
- **MEM35-C**: Allocate sufficient memory for an object.
- **INT30-C**: Ensure unsigned integer operations do not wrap.
- **INT32-C**: Ensure signed integer operations do not overflow.
- **ERR33-C**: Detect and handle standard library errors.
- **FIO30-C**: Exclude user input from format strings.

---

_Sources: CERT C Coding Standard (SEI), C11/C17 Standard, Linux Kernel Coding Style, Expert C Programming (Peter van der Linden), Secure Coding in C and C++ (Seacord)_
