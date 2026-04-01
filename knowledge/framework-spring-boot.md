# Spring Boot

## Core Concepts

Spring Boot is an opinionated framework on top of the Spring Framework. **Auto-configuration** detects classes on the classpath and configures beans automatically. **Starters** are curated dependency sets (e.g., `spring-boot-starter-web` brings embedded Tomcat, Jackson, Spring MVC).

```java
@SpringBootApplication  // = @Configuration + @EnableAutoConfiguration + @ComponentScan
public class Application {
    public static void main(String[] args) {
        SpringApplication.run(Application.class, args);
    }
}
```

## Dependency Injection

### Stereotype Annotations

| Annotation        | Purpose                                  | Layer       |
| ----------------- | ---------------------------------------- | ----------- |
| `@Component`      | Generic bean                             | Any         |
| `@Service`        | Business logic                           | Service     |
| `@Repository`     | Data access (adds exception translation) | Persistence |
| `@Controller`     | Web controller (returns views)           | Web         |
| `@RestController` | `@Controller` + `@ResponseBody`          | Web/API     |
| `@Configuration`  | Java-based config (replaces XML)         | Config      |

### Injection

```java
@Service
public class OrderService {
    // Constructor injection (preferred — immutable, testable)
    private final OrderRepository orderRepo;
    private final PaymentService paymentService;

    public OrderService(OrderRepository orderRepo, PaymentService paymentService) {
        this.orderRepo = orderRepo;
        this.paymentService = paymentService;
    }

    // @Autowired on constructor is optional when there's only one constructor (Spring 4.3+)
}
```

**Field injection (`@Autowired` on fields)** is discouraged — makes testing harder, hides dependencies.

### Bean Scopes

| Scope                 | Lifecycle                           |
| --------------------- | ----------------------------------- |
| `singleton` (default) | One instance per ApplicationContext |
| `prototype`           | New instance every injection        |
| `request`             | One per HTTP request                |
| `session`             | One per HTTP session                |
| `application`         | One per ServletContext              |

### Profiles

```java
@Configuration
@Profile("production")
public class ProdConfig {
    @Bean
    public DataSource dataSource() { /* production datasource */ }
}
```

```properties
# application.properties
spring.profiles.active=dev
```

## Spring MVC (Web)

### Controllers

```java
@RestController
@RequestMapping("/api/v1/users")
public class UserController {

    private final UserService userService;

    public UserController(UserService userService) {
        this.userService = userService;
    }

    @GetMapping
    public Page<UserDto> list(
            @RequestParam(defaultValue = "0") int page,
            @RequestParam(defaultValue = "20") int size,
            @RequestParam(defaultValue = "name") String sort) {
        return userService.findAll(PageRequest.of(page, size, Sort.by(sort)));
    }

    @GetMapping("/{id}")
    public UserDto get(@PathVariable Long id) {
        return userService.findById(id)
                .orElseThrow(() -> new ResponseStatusException(HttpStatus.NOT_FOUND));
    }

    @PostMapping
    @ResponseStatus(HttpStatus.CREATED)
    public UserDto create(@Valid @RequestBody UserCreateDto dto) {
        return userService.create(dto);
    }

    @PutMapping("/{id}")
    public UserDto update(@PathVariable Long id, @Valid @RequestBody UserUpdateDto dto) {
        return userService.update(id, dto);
    }

    @DeleteMapping("/{id}")
    @ResponseStatus(HttpStatus.NO_CONTENT)
    public void delete(@PathVariable Long id) {
        userService.delete(id);
    }
}
```

### Validation

```java
public record UserCreateDto(
    @NotBlank @Size(max = 100) String name,
    @NotBlank @Email String email,
    @NotNull @Min(0) Integer age
) {}
```

### Exception Handling

```java
@RestControllerAdvice
public class GlobalExceptionHandler {

    @ExceptionHandler(MethodArgumentNotValidException.class)
    @ResponseStatus(HttpStatus.BAD_REQUEST)
    public Map<String, String> handleValidation(MethodArgumentNotValidException ex) {
        return ex.getBindingResult().getFieldErrors().stream()
                .collect(Collectors.toMap(FieldError::getField, FieldError::getDefaultMessage));
    }

    @ExceptionHandler(EntityNotFoundException.class)
    @ResponseStatus(HttpStatus.NOT_FOUND)
    public Map<String, String> handleNotFound(EntityNotFoundException ex) {
        return Map.of("error", ex.getMessage());
    }
}
```

## Spring Data JPA

### Repository Interface

```java
public interface UserRepository extends JpaRepository<User, Long> {
    // Derived query methods
    Optional<User> findByEmail(String email);
    List<User> findByNameContainingIgnoreCase(String name);
    boolean existsByEmail(String email);
    long countByActiveTrue();

    // JPQL
    @Query("SELECT u FROM User u WHERE u.department.name = :dept AND u.active = true")
    List<User> findActiveByDepartment(@Param("dept") String departmentName);

    // Native SQL
    @Query(value = "SELECT * FROM users WHERE created_at > :date", nativeQuery = true)
    List<User> findRecentUsers(@Param("date") LocalDateTime date);

    // Modifying
    @Modifying
    @Query("UPDATE User u SET u.active = false WHERE u.lastLogin < :cutoff")
    int deactivateInactiveUsers(@Param("cutoff") LocalDateTime cutoff);

    // Projections
    <T> List<T> findByActive(boolean active, Class<T> type);
}
```

### Entity Mapping

```java
@Entity
@Table(name = "users", indexes = @Index(columnList = "email"))
public class User {
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    private Long id;

    @Column(nullable = false, length = 100)
    private String name;

    @Column(unique = true, nullable = false)
    private String email;

    @ManyToOne(fetch = FetchType.LAZY)
    @JoinColumn(name = "department_id")
    private Department department;

    @OneToMany(mappedBy = "user", cascade = CascadeType.ALL, orphanRemoval = true)
    private List<Order> orders = new ArrayList<>();

    @ManyToMany
    @JoinTable(name = "user_roles",
        joinColumns = @JoinColumn(name = "user_id"),
        inverseJoinColumns = @JoinColumn(name = "role_id"))
    private Set<Role> roles = new HashSet<>();

    @CreatedDate
    private LocalDateTime createdAt;

    @LastModifiedDate
    private LocalDateTime updatedAt;
}
```

### Specifications (dynamic queries)

```java
public class UserSpecs {
    public static Specification<User> hasName(String name) {
        return (root, query, cb) -> cb.like(cb.lower(root.get("name")), "%" + name.toLowerCase() + "%");
    }

    public static Specification<User> isActive() {
        return (root, query, cb) -> cb.isTrue(root.get("active"));
    }
}

// Usage
userRepo.findAll(UserSpecs.hasName("alice").and(UserSpecs.isActive()), pageable);
```

## Spring Security

```java
@Configuration
@EnableWebSecurity
public class SecurityConfig {

    @Bean
    public SecurityFilterChain filterChain(HttpSecurity http) throws Exception {
        return http
            .csrf(csrf -> csrf.disable())  // disable for stateless API
            .sessionManagement(session -> session.sessionCreationPolicy(SessionCreationPolicy.STATELESS))
            .authorizeHttpRequests(auth -> auth
                .requestMatchers("/api/auth/**").permitAll()
                .requestMatchers("/api/admin/**").hasRole("ADMIN")
                .anyRequest().authenticated()
            )
            .oauth2ResourceServer(oauth2 -> oauth2.jwt(Customizer.withDefaults()))
            .build();
    }

    @Bean
    public JwtDecoder jwtDecoder() {
        return NimbusJwtDecoder.withPublicKey(rsaPublicKey).build();
    }

    @Bean
    public PasswordEncoder passwordEncoder() {
        return new BCryptPasswordEncoder();
    }
}
```

### Method Security

```java
@EnableMethodSecurity
@Configuration
public class MethodSecurityConfig {}

@Service
public class DocumentService {
    @PreAuthorize("hasRole('ADMIN') or #userId == authentication.principal.id")
    public Document getDocument(Long userId, Long docId) { ... }

    @PostAuthorize("returnObject.owner == authentication.name")
    public Document findById(Long id) { ... }
}
```

## Spring WebFlux (Reactive)

```java
@RestController
@RequestMapping("/api/items")
public class ItemController {
    private final ItemRepository itemRepo;

    @GetMapping
    public Flux<Item> list() {
        return itemRepo.findAll();
    }

    @GetMapping("/{id}")
    public Mono<Item> get(@PathVariable String id) {
        return itemRepo.findById(id)
                .switchIfEmpty(Mono.error(new ResponseStatusException(HttpStatus.NOT_FOUND)));
    }

    @PostMapping
    @ResponseStatus(HttpStatus.CREATED)
    public Mono<Item> create(@RequestBody Item item) {
        return itemRepo.save(item);
    }
}
```

WebFlux uses Project Reactor (`Mono<T>` for 0-1 items, `Flux<T>` for 0-N items). Use for high-concurrency I/O-bound workloads. Not faster for CPU-bound work.

## Actuator

```properties
management.endpoints.web.exposure.include=health,info,metrics,prometheus
management.endpoint.health.show-details=when-authorized
```

Key endpoints: `/actuator/health`, `/actuator/metrics`, `/actuator/env`, `/actuator/loggers`, `/actuator/prometheus`.

## Configuration Properties

```java
@ConfigurationProperties(prefix = "app.mail")
public record MailProperties(
    String host,
    int port,
    String from,
    boolean enabled
) {}

// Enable in main class
@EnableConfigurationProperties(MailProperties.class)
```

```yaml
# application.yml
app:
  mail:
    host: smtp.example.com
    port: 587
    from: noreply@example.com
    enabled: true
```

## Testing

```java
// Full integration test
@SpringBootTest(webEnvironment = SpringBootTest.WebEnvironment.RANDOM_PORT)
class UserControllerIT {
    @Autowired
    TestRestTemplate restTemplate;

    @Test
    void shouldCreateUser() {
        var dto = new UserCreateDto("Alice", "alice@test.com", 30);
        var response = restTemplate.postForEntity("/api/v1/users", dto, UserDto.class);
        assertThat(response.getStatusCode()).isEqualTo(HttpStatus.CREATED);
        assertThat(response.getBody().name()).isEqualTo("Alice");
    }
}

// Slice tests (faster — only load relevant context)
@DataJpaTest
class UserRepositoryTest {
    @Autowired UserRepository repo;
    @Autowired TestEntityManager em;

    @Test
    void shouldFindByEmail() {
        em.persist(new User("Alice", "alice@test.com"));
        Optional<User> found = repo.findByEmail("alice@test.com");
        assertThat(found).isPresent();
    }
}

@WebMvcTest(UserController.class)
class UserControllerTest {
    @Autowired MockMvc mvc;
    @MockBean UserService userService;

    @Test
    void shouldReturn404WhenNotFound() throws Exception {
        when(userService.findById(1L)).thenReturn(Optional.empty());
        mvc.perform(get("/api/v1/users/1"))
            .andExpect(status().isNotFound());
    }
}
```

## Native Images (GraalVM)

```xml
<plugin>
    <groupId>org.graalvm.buildtools</groupId>
    <artifactId>native-maven-plugin</artifactId>
</plugin>
```

```bash
mvn -Pnative native:compile
```

Startup in ~50ms vs ~2s for JVM. Trade-off: longer build time, no runtime reflection (requires hints), no dynamic class loading.

## Virtual Threads (Java 21+)

```properties
spring.threads.virtual.enabled=true
```

Enables virtual threads for request handling — each request gets a lightweight virtual thread instead of a platform thread. Dramatically improves throughput for I/O-bound workloads without reactive code complexity. Use instead of WebFlux when you don't need backpressure.
