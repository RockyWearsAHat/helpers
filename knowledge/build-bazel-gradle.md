# Enterprise Build Systems — Bazel, Gradle, Maven

## Why Enterprise Scale Matters

A startup's 50-package build system might take 2 minutes: acceptable feedback loop for most developers. A multinational's 100,000-package monorepo needs 10–30 seconds, or engineering productivity collapses. Enterprise build systems optimize for:

1. **Reproducibility**: same code + config → identical artifacts everywhere (critical for compliance, distributed CI)
2. **Scalability**: thousands of targets, hundreds of developers, CI farms with thousands of machines
3. **Hermeticity**: explicit dependency declaration, no implicit environment leaks (security, supply chain)
4. **Remote execution**: offload build work to data centers with spare capacity

Make and Maven don't meet these needs at scale; Gradle and Bazel do.

---

## Bazel: Hermetic, Remote-Execution-Native

Bazel originated inside Google to build its 50+ billion–line monorepo. It prioritizes reproducibility and distributed execution.

### Core Principles

**Hermeticity**: build actions run in sandboxes. Inputs are explicit; no access to environment unless declared:

```starlark
cc_library(
  name = "engine",
  srcs = ["engine.cc", "engine.h"],
  deps = ["//third_party/openssl"],  # Explicit dep; implicit deps rejected
  # No access to /usr/include unless declared in toolchain
)
```

Sandbox prevents:
- Implicit dependencies on system libraries
- Leakage of environment variables
- Accidental creation of non-deterministic artifacts

### Starlark: Build Language

Bazel rules are written in Starlark (Python-like DSL):

```starlark
def my_custom_rule_impl(ctx):
    inputs = ctx.files.src
    output = ctx.actions.declare_file("output.txt")
    ctx.actions.run(
        executable = ctx.executable.tool,
        arguments = [inputs[0].path, output.path],
        inputs = inputs,
        outputs = [output],
    )
    return [DefaultInfo(files = depset([output]))]

my_custom_rule = rule(
    implementation = my_custom_rule_impl,
    attrs = {
        "src": attr.label_list(allow_files = True),
        "tool": attr.label(executable = True, cfg = "exec"),
    },
)
```

This defines a custom build action. Bazel's actions are:
- **Deterministic**: same inputs always produce identical outputs
- **Sandboxed**: isolated from system
- **Parallelizable**: can run on any machine without setup
- **Cacheable**: output stored with key = hash(inputs)

### Remote Execution

```bash
bazel build //app:binary --remote_executor=grpc://build-farm.company.com
```

Bazel sends build actions to remote workers (via gRPC). Each worker:
1. Fetches inputs from remote content-addressable storage (CAS)
2. Executes action in sandbox
3. Uploads outputs back to CAS

Result: parallelism scales horizontally; 1000-machine farms reduce wall-clock time by 1000x (minus networking and critical-path bottlenecks).

### Trade-offs

**Advantages:**
- Reproducible builds enforce correctness
- Distributed execution scales to unlimited parallelism
- Fine-grained caching enables cross-project artifact reuse
- Monorepo support with efficient incremental builds

**Disadvantages:**
- **High onboarding cost**: Starlark learning curve, rewriting all build rules
- **Complexity**: declaring all dependencies is tedious; implicit linkage tempting
- **Debugging difficulty**: sandboxed execution hides environment problems until deployment
- **Slow feedback for small tests**: remote execution latency (network round-trip) outweighs work
- **Not ubiquitous**: Bazel shines for monorepos and C++; less powerful for JavaScript/Python workflows

### Typical Adoption

- **Large tech companies** (Google, Stripe, Databricks): entire monorepos in Bazel
- **Open-source projects**: used for C++ (AOSP, TensorFlow, Envoy) 
- **Mid-market**: selective adoption (build infrastructure, release pipeline) while staying on Gradle/Maven for development

---

## Gradle: Task-Based with Incremental Caching

Gradle (2008-present) is the Build Tool of choice for JVM and Android. It balances power with pragmatism: less rigid than Bazel, more scalable than Maven.

### Task Graph and Incremental Builds

A Gradle build is a directed acyclic graph of tasks. Each task declares inputs and outputs:

```groovy
task compileJava(type: JavaCompile) {
  source = fileTree('src/main/java')
  outputs.dir 'build/classes'
}

task packageJar(type: Jar) {
  from compileJava.outputs
  archiveName = 'app.jar'
}
```

Gradle executes only tasks affected by changes:

```bash
gradle build
# Modifies one line of Java
gradle build
# Output: compileJava (recompile), packageJar (rebuild), test (rerun)
#         All other tasks marked "UP-TO-DATE"
```

### Build Cache

Gradle caches task outputs with a content-addressable key:

```
Key = hash(all task inputs + classpath + compiler flags)
```

Cache hit means task is reused without re-execution (even across CI machines if remote cache configured).

### Groovy and Kotlin DSLs

Build logic in Groovy:

```groovy
def isRelease = hasProperty('release')
dependencies {
  if (isRelease) {
    implementation configurations.pinned
  } else {
    implementation configurations.unpinned
  }
}
```

Or Kotlin (more type-safe):

```kotlin
val isRelease = hasProperty("release")
dependencies {
  if (isRelease) {
    implementation(configurations.pinned)
  } else {
    implementation(configurations.unpinned)
  }
}
```

Full language expressiveness (control flow, functions, plugins) vs Bazel's Starlark (restricted, deterministic).

### Build Scans

Gradle Enterprise provides build scans: web dashboard showing:
- Task execution timeline
- Cache hit/miss reasons
- Performance bottlenecks
- Flaky test detection

Essential for large teams to collaborate on build optimization.

### Trade-offs

**Advantages:**
- Pragmatic: incremental by default, no sandboxing required
- Rich plugin ecosystem (100+ plugins for Java, Android, Kotlin)
- Flexible: imperative build logic when needed
- Adoption: nearly universal in JVM ecosystem

**Disadvantages:**
- Less principled than Bazel: implicit dependencies possible (leads to flaky builds)
- Incremental compilation can have bugs (rare, but requires full rebuilds to fix)
- Slower than Bazel at extreme scale (no remote execution primitive, though can integrate with external tools)
- Configuration complexity: many knobs to tune

---

## Maven: Convention Over Configuration

Maven (2004–present) emphasizes standardized project structure and lifecycle. Less flexible than Gradle; more portable.

### Lifecycle Phases

Every Maven project follows a standard lifecycle:

```
validate → compile → test → package → integration-test → verify → install → deploy
```

Common Plugins:
- `maven-compiler-plugin`: compile Java
- `maven-surefire-plugin`: run unit tests
- `maven-jar-plugin`: package JAR
- `maven-shade-plugin`: repackage with dependencies (fat JAR)

### Structure Convention

```
pom.xml                          # Project config (XML)
src/main/java/...               # Source code
src/main/resources/...          # Config files
src/test/java/...               # Test code
target/                         # Build output
```

Any Maven project follows this; no custom configuration needed for simple projects.

### Dependency Management

```xml
<dependencies>
  <dependency>
    <groupId>org.junit</groupId>
    <artifactId>junit-bom</artifactId>
    <version>5.9.2</version>
    <type>pom</type>
    <scope>import</scope>
  </dependency>
</dependencies>
```

Maven resolves transitive dependencies and can detect conflicts (though imperfectly).

### Trade-offs

**Advantages:**
- **Standardization**: Maven projects are immediately understandable
- **Simplicity**: convention reduces boilerplate
- **Stability**: XML schema is rigid; accidental misconfiguration rarer than in Gradle

**Disadvantages:**
- **Inflexibility**: deviating from the standard structure is painful
- **Verbosity**: XML requires more lines than Groovy/Kotlin for equivalent logic
- **Incremental builds**: Maven doesn't support incremental compilation well (rebuilds on every run unless caching external tool)
- **Performance**: slower than Gradle on large projects

---

## Comparison Matrix

| Criterion | Bazel | Gradle | Maven |
|-----------|-------|--------|-------|
| **Reproducibility** | Hermetic; guaranteed | Possible; requires discipline | Partial (not reproducible by design) |
| **Remote Execution** | Native primitive | Via plugins only | Not supported |
| **Ease of Adoption** | High; requires rewriting all builds | Low; adds incrementally | Lowest; mostly automatic for standard layouts |
| **Ecosystem** | Smaller; C++/Go/Python strong | Huge; Java/Android dominant | Large; Java/Maven plugins everywhere |
| **Feedback Loop** (small change) | 1–5 seconds (remote overhead) | 500ms–2s (local) | 2–10 seconds (full build) |
| **Configuration Language** | Starlark (restricted) | Groovy/Kotlin (full language) | XML (schema-driven) |
| **Team Scale** | 50+ developers | 10–100 developers | <50 developers |
| **Monorepo Support** | Excellent; scales to 100k+ targets | Good; scales to 1000+ packages | Poor; federation required |

---

## Migration Strategies

### Maven → Gradle

1. **Gradle wrapper plugin** for Maven (runs Gradle from Maven POM)
2. **Per-module migration**: convert one Maven module to Gradle at a time
3. **Multi-build setup**: Gradle and Maven coexist temporarily, then Maven modules gradually removed

Time: 2–6 weeks for mid-size project (50–100 modules).

### Gradle → Bazel

Much harder. Requires:
1. Mapping Gradle tasks to Bazel rules
2. Rewriting build logic in Starlark
3. Declaring all implicit dependencies explicitly
4. Testing on local machines before deploying to remote execution

Time: 6–12 months for a large monorepo, with dedicated team. ROI only if:
- Reproducibility is critical (regulated industries, security supply chain)
- Remote execution scales to 100+ machines
- Monorepo has 5000+ targets

### Maven → Bazel

Least common. Requires converting Maven's plugin-based logic to Bazel rules (significant effort) + all Gradle → Bazel complexity.

---

## When to Use Each

**Bazel**: Scientific computing, monorepos at Google-scale, security-critical systems requiring reproducible builds. Accept 3–6 month migration cost and ongoing infrastructure overhead.

**Gradle**: JVM/Android projects, teams <100 developers, projects needing incremental builds and build caching without extreme reproducibility requirements. Fast feedback loops matter more than perfect isolation.

**Maven**: Teams with strong XML/POM experience, projects with minimal customization, teams shipping multiple JAR artifacts. Rarely the best choice for new projects; often unavoidable for legacy maintenance.

---

## See Also

- **build-systems-deep.md** — DAG concepts, reproducibility, remote caching apply across all three
- **architecture-monorepo.md** — How build systems enable monorepo scaling
- **devops-supply-chain-security.md** — Hermetic builds and reproducibility as security controls