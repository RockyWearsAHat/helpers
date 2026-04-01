# Spring Ecosystem

## Overview

The **Spring Framework** is Java's dominant application framework, and its ecosystem extends across database access (Spring Data), security (Spring Security), cloud deployment (Spring Cloud), reactive programming (Project Reactor + WebFlux), and observability (Micrometer). Spring Boot packages Spring + opinionated defaults into zero-configuration application servers.

Most enterprise Java projects use Spring or derivatives. Understanding the ecosystem means understanding a central technology stack in the Java world.

## Spring Boot — Rapid Configuration

**Spring Boot** is an opinionated wrapper around Spring Framework. It removes XML boilerplate, provides auto-configuration, and bundles production-ready defaults.

### Auto-Configuration

Auto-configuration detects dependencies on the classpath and automatically configures beans:

```java
// Add spring-boot-starter-web: auto-configures DispatcherServlet, embedded Tomcat, Jackson
@SpringBootApplication
public class Application {
    public static void main(String[] args) {
        SpringApplication.run(Application.class, args);
    }
}

// Add spring-boot-starter-data-jpa: auto-configures EntityManager, DataSource, repositories
@Repository
public interface UserRepository extends JpaRepository<User, Long> {}

// Create a controller — routing + DispatcherServlet already wired
@RestController
@RequestMapping("/api/users")
public class UserController {
    @Autowired
    private UserRepository repo;
    
    @GetMapping("/{id}")
    public User getUser(@PathVariable Long id) {
        return repo.findById(id).orElseThrow();
    }
}
```

Starters are curated dependency sets. `spring-boot-starter-web` includes Spring MVC, embedded Tomcat, Jackson, and logging. No manual dependency versioning; Spring Boot manages compatibility.

### Configuration

```yaml
# application.yml — Spring Boot scans for this in classpath root
spring:
  datasource:
    url: jdbc:mysql://localhost/mydb
    username: root
    password: secret
  jpa:
    hibernate:
      ddl-auto: update
  jackson:
    serialization:
      indent-output: true
  mvc:
    throw-exception-if-no-handler-found: true
```

## Spring Data — Data Access Abstraction

**Spring Data** is a family of projects providing repository-based data access.

### Repositories (JPA Example)

```java
@Entity
@Table(name = "users")
public class User {
    @Id
    @GeneratedValue
    private Long id;
    
    @Column(nullable = false)
    private String email;
    
    @ManyToMany
    private Set<Role> roles;
}

@Repository
public interface UserRepository extends JpaRepository<User, Long> {
    User findByEmail(String email);
    List<User> findByRolesContaining(Role role);
    
    @Query("SELECT u FROM User u WHERE u.email LIKE %?1%")
    List<User> searchEmail(String pattern);
}

// Usage
User user = repo.findByEmail("alice@test.com");
List<User> admins = repo.findByRolesContaining(Role.ADMIN);
```

Spring Data generates SQL from method names and `@Query` annotations. Reduces boilerplate significantly.

### JPA / Hibernate

JPA is the Jakarta Persistence API standard. Hibernate is the reference implementation.

- **Entity mapping**: `@Entity`, `@Table`, `@Column` map objects to tables
- **Relationships**: `@OneToMany`, `@ManyToMany`, `@OneToOne` with cascade/fetch strategies
- **Queries**: HQL (Hibernate Query Language) or JPQL (Java Persistence Query Language)
- **Transactions**: `@Transactional` manages begin/commit/rollback
- **Lazy loading**: Relationships can be fetched on-demand; beware N+1 queries

```java
@Service
public class OrderService {
    @Autowired
    private OrderRepository repo;
    
    @Transactional(readOnly = true)
    public OrderDTO getOrderDetails(Long id) {
        Order order = repo.findById(id).orElseThrow();
        // order.getItems() triggers a query if items was lazy-loaded
        return OrderDTO.from(order);
    }
}
```

### Other Spring Data Modules

- **Spring Data MongoDB**: NoSQL repository pattern for MongoDB
- **Spring Data Redis**: Key-value caching layer
- **Spring Data Elasticsearch**: Full-text search indexing
- **Spring Data R2DBC**: Reactive relational database connectivity

## Spring Security — Authentication & Authorization

**Spring Security** provides authentication, authorization, and attack prevention.

### Filter Chain

Spring Security wraps HTTP requests in a filter chain:

```
Request → Authentication Filters → Authorization Filters → Controller
```

```java
@Configuration
@EnableWebSecurity
public class SecurityConfig {
    @Bean
    public SecurityFilterChain securityFilterChain(HttpSecurity http) throws Exception {
        http
            .authorizeHttpRequests(auth -> auth
                .requestMatchers("/public/**").permitAll()
                .requestMatchers("/admin/**").hasRole("ADMIN")
                .anyRequest().authenticated()
            )
            .formLogin(form -> form.loginPage("/login"))
            .logout(logout -> logout.logoutUrl("/logout"))
            .csrf(csrf -> csrf.disable()); // usually keep CSRF enabled
        
        return http.build();
    }
}
```

### Authentication Methods

- **Form login**: Username/password form submission
- **HTTP Basic**: Base64-encoded credentials in Authorization header
- **OAuth 2.0**: Delegated identity (Google, GitHub login)
- **JWT tokens**: Stateless token-based auth
- **LDAP**: Enterprise directory authentication

### Method-Level Authorization

```java
@Service
public class OrderService {
    @PreAuthorize("hasRole('USER')")
    public Order getOrder(Long id) {
        return repo.findById(id).orElseThrow();
    }
    
    @PreAuthorize("hasRole('ADMIN')")
    public void deleteOrder(Long id) {
        repo.deleteById(id);
    }
}
```

## Spring Cloud — Distributed Systems

**Spring Cloud** adds distributed system patterns: service discovery, configuration management, load balancing, API gateway, circuit breakers.

```java
// Client-side load balancing with Eureka (service registry)
@Configuration
public class CloudConfig {
    
    @Bean
    public RestTemplate restTemplate(RestTemplateBuilder builder) {
        return builder.build();
    }
}

@RestController
public class OrderController {
    @Autowired
    private RestTemplate restTemplate;
    
    @GetMapping("/order/{id}")
    public Order getOrder(@PathVariable Long id) {
        // Eureka resolves "payment-service" to actual IP:port
        String result = restTemplate.getForObject(
            "http://payment-service/api/payments/" + id, 
            String.class
        );
        return parseOrder(result);
    }
}
```

### Spring Cloud Components

| Component      | Purpose                                              |
| -------------- | ---------------------------------------------------- |
| **Eureka**     | Service discovery (registry + client)                |
| **Config**     | Centralized configuration management                 |
| **Gateway**    | API gateway (routing, rate limiting, auth)           |
| **Circuit Breaker** | Hystrix/Resilience4j (fail-fast on service failure) |
| **Load Balancer** | Client-side & server-side load balancing             |
| **Sleuth/Tracer** | Distributed tracing (works with Micrometer)          |

## WebFlux — Reactive Programming

**Spring WebFlux** implements reactive, non-blocking I/O using Project Reactor.

```java
@RestController
@RequestMapping("/api")
public class ReactiveController {
    @Autowired
    private UserRepository repo;
    
    @GetMapping("/users")
    public Flux<User> getAllUsers() {
        return repo.findAll(); // returns Flux (stream of items)
    }
    
    @GetMapping("/users/{id}")
    public Mono<User> getUser(@PathVariable Long id) {
        return repo.findById(id); // returns Mono (0 or 1 item)
    }
}

// Non-blocking database calls
@Service
public class OrderService {
    @Autowired
    private ReactiveOrderRepository repo;
    
    public Mono<OrderDTO> processOrder(Order order) {
        return repo.save(order)
            .flatMap(saved -> callPaymentService(saved.getId()))
            .map(OrderDTO::from);
    }
}
```

**Mono** and **Flux** are reactive types that emit items asynchronously. Operations chain reactively without blocking threads—ideal for high-concurrency systems or microservices calling downstream services.

Tradeoff: Debugging is harder; mental model differs from synchronous code.

## Micrometer — Observability

**Micrometer** is a vendor-neutral metrics and tracing facade. Spring Boot auto-configures it.

```java
@Service
public class OrderService {
    private final MeterRegistry meterRegistry;
    
    public OrderService(MeterRegistry meterRegistry) {
        this.meterRegistry = meterRegistry;
    }
    
    public void processOrder(Order order) {
        meterRegistry.counter("orders.created").increment();
        
        Timer.Sample sample = Timer.start(meterRegistry);
        try {
            // process order
        } finally {
            sample.stop(Timer.builder("order.processing.time")
                .publishPercentiles(0.95, 0.99)
                .register(meterRegistry));
        }
    }
}
```

Exports to Prometheus, Grafana, Datadog, New Relic, etc. Built-in support for JVM metrics, HTTP metrics, and custom business metrics.

## GraalVM Native Image

**GraalVM Native Image** compiles Spring Boot apps to standalone native executables (not JVM bytecode).

```bash
# Add native build plugin
# mvn -Pnative native:compile
# Creates binary: target/application
```

Benefits:
- **Startup**: Cold start in milliseconds (vs. seconds)
- **Memory**: Lower heap (ideal for Lambda, containers)
- **Portability**: Single executable, no JVM needed

Tradeoffs:
- **Build time**: 1-5 minutes per compilation (vs. instant with JVM)
- **Reflection**: Requires ahead-of-time configuration; reflection in frameworks must be AOT-aware
- **Debugging**: Limited debugging capabilities in native mode
- **Compatibility**: Not all libraries work with native image (reflection, dynamic proxies, serialization)

Spring Boot 3.0+ added first-class Native Image support via native-image-maven-plugin and native profiles.

## Ecosystem Position & Tradeoffs

Spring is enterprise-grade, production-hardened, and nearly ubiquitous in the Java world:

- **Ecosystem size**: Massive. Every major library integrates with Spring.
- **Learning curve**: Significant. Annotations, conventions, and configuration patterns take time.
- **Startup time**: JVM startup + Spring initialization is slow (native image helping).
- **Memory overhead**: Spring context loading uses memory; suitable for servers, not small scripts.
- **Legacy presence**: Much of the ecosystem is older code; modernization (records, virtual threads, Loom) still rolling out.

Alternatives (Quarkus, Micronaut) offer faster startup + lower memory but smaller ecosystems.

**See also**: language-java, runtime-jvm, database-sql-fundamentals, observability-distributed-tracing, cloud-microservices-patterns