# R Language: Data Science & the Tidyverse

## Philosophy

R is the statistical computing lingua franca. The **tidyverse** ecosystem—dplyr, ggplot2, tidyr, ggplot2—represents a philosophical shift: tidy data (observations as rows, variables as columns) and consistent function APIs. R excels at interactive data exploration, statistical modeling, and publication-quality graphics. Trade-offs: slower than Python on large-scale ML pipelines, weaker for production deployment, but unmatched for statistical rigor and exploratory visualization.

## Data Structures: Vectors, Lists, Data Frames, and Tibbles

### Base R: Vectors and Lists

R is fundamentally a vectorized language. Operations broadcast across entire vectors without explicit loops:

```r
x <- c(1, 2, 3)
x + 1  # vectorized: c(2, 3, 4)
```

**Vectors** are homogeneous; **lists** (the recursive data structure) mix types. Most R code leverages this implicitly.

### Data Frames: The Base Structure

Data frames are lists of equal-length vectors, each representing a column. Atomic structure:

```r
df <- data.frame(
  name = c("Alice", "Bob"),
  age = c(30, 25),
  score = c(85.5, 92.0)
)
```

Row and column subscripting: `df[1, ]`, `df$name`, `df[["name"]]`.

### Tibbles: The Tidyverse Evolution

Tibbles are **stricter, lazier data frames** (from the tibble package). Key differences:

- **Partial matching disabled**: `df$na` won't match `df$name`; throws error
- **Recycling prevented**: won't silently recycle vectors of mismatched lengths
- **Printing**: cleaner console output with type hints (e.g., `<dbl>`, `<chr>`)
- **Lazy evaluation**: columns can reference earlier-defined columns in creation

```r
tibble::tibble(
  x = 1:3,
  y = x * 2  # references x from same call
)
```

Use tibbles by default in tidyverse workflows; convert base data frames with `as_tibble()`.

## The Pipe: %>% and |>

The **pipe** chains operations left-to-right, reducing nesting and intermediate variables.

### magrittr Pipe: %>%

Introduced by magrittr (imported by dplyr):

```r
data %>%
  filter(age > 25) %>%
  select(name, score) %>%
  arrange(desc(score))
```

Pipe inserts left-hand side as first argument (usually). For other positions:

```r
lm(y ~ x, data = .)  # . as placeholder
```

### Native Pipe: |> (R 4.1+)

Native pipe syntax avoids magrittr dependency:

```r
data |>
  filter(age > 25) |>
  select(name, score)
```

Differences from `%>%`:

- No eager evaluation of intermediate steps (slightly faster)
- `_` as placeholder: `x |> f(y, _)` instead of `x %>% f(y, .)`
- Supports `=>` for lambda shorthand (R 4.1+ with pattern matching in development)

Preference: `|>` for new code; `%>%` is stable in established projects.

## Tidyverse Ecosystem

### dplyr: Grammar of Data Manipulation

Five core verbs (+ combinations):

- **`select()`**: pick columns by name, position, or pattern
- **`filter()`**: subset rows by logical condition
- **`mutate()`**: create or modify columns
- **`arrange()`**: sort rows
- **`summarize()`**: collapse rows into summary statistics

Group operations with `group_by()`:

```r
mtcars %>%
  group_by(cyl) %>%
  summarize(avg_hp = mean(hp), n = n())
```

Advanced: `across()` for applying functions to multiple columns, `nest_by()` for nested data frames.

### ggplot2: Grammar of Graphics

Layered approach: map data to aesthetic attributes (x, y, color, size), then specify geometry (points, lines, bars).

```r
ggplot(mtcars, aes(x = wt, y = mpg, color = factor(cyl))) +
  geom_point() +
  geom_smooth(method = "lm", se = FALSE) +
  facet_wrap(~ am) +
  theme_minimal()
```

Layers: **data** → **mapping** (aesthetics) → **geometry** → **statistics** → **coordinates** → **theme**.

Advanced: custom themes, annotation, position adjustments (`position_dodge`, `position_jitter`), coordinate systems.

### tidyr: Reshaping & Tidy Data

Reshape between long and wide formats:

```r
tidyr::pivot_longer(df, cols = -id, names_to = "var", values_to = "val")
tidyr::pivot_wider(df, names_from = var, values_from = val)
```

**Tidy principles**: One observation per row, one variable per column. `nest()` and `unnest()` for hierarchical data.

## Shiny: Reactive Web Applications

Shiny enables interactive R applications without JavaScript knowledge. Reactive framework: **inputs** (UI) trigger **reactions**, which update **outputs**.

### Structure

```r
library(shiny)

ui <- fluidPage(
  titlePanel("My App"),
  sidebarLayout(
    sidebarPanel(
      sliderInput("n", "N:", min = 1, max = 100, value = 50)
    ),
    mainPanel(
      plotOutput("plot")
    )
  )
)

server <- function(input, output) {
  output$plot <- renderPlot({
    hist(rnorm(input$n))
  })
}

shinyApp(ui, server)
```

### Reactivity

- **`reactive()`**: creates reactive value; triggers dependents on change
- **`eventReactive()`**: reactive only when specific event fires
- **`observe()`**: side effects (e.g., logging, UI updates)
- **`observeEvent()`**: run code when specific input changes

Lazy evaluation: outputs update only when their inputs change.

### Deployment

Shiny apps run on R, deployed via shinyapps.io, RStudio Connect, or containerized (Docker). Scaling: connection limits, session management, caching with `memoise`.

## RMarkdown and Quarto

### RMarkdown

Literate programming: **Markdown text** + **R code chunks** + **output** (HTML, PDF, Word, slides).

```markdown
# Analysis

```{r, message=FALSE}
library(dplyr)
mtcars %>% head()
```

Results knit to single document.

### Quarto (Modern Successor)

Language-agnostic evolution: embed R, Python, Julia, Observable JS in one document. YAML front-matter controls execution, output format, rendering engine.

```yaml
---
title: "Analysis"
format: 
  html:
    toc: true
    code-fold: true
---
```

Benefits: shared format across languages, better performance, advanced features (cross-references, citations, custom rendering).

## Package Development

### Structure

```
mypackage/
  R/             # .R files (functions)
  data/          # .Rdata, .csv
  man/           # .Rd documentation
  tests/
  DESCRIPTION    # metadata
  NAMESPACE      # exports
```

### devtools and roxygen2

**roxygen2**: generate man pages and NAMESPACE from comments:

```r
#' Add Two Numbers
#'
#' @param x First number
#' @param y Second number
#' @return Sum
#'
#' @export
add <- function(x, y) x + y
```

Run `roxygen2::roxygenise()` to generate Rd files and NAMESPACE.

**devtools**: workflows for testing (`test()`), loading (`load_all()`), documenting, checking.

### CRAN Standards

Submission requires: working code, passing `R CMD check`, clear documentation, proper DESCRIPTION metadata. CRAN policies: no external system dependencies, proper citations.

## R vs Python

| Aspect | R | Python |
|--------|---|--------|
| **Strength** | Statistical modeling, exploratory analysis, visualization | General-purpose, production ML, web |
| **Data Manipulation** | dplyr is elegant; tidyverse unified | pandas is powerful but inconsistent API |
| **Visualization** | ggplot2 unmatched for publication quality | matplotlib/seaborn functional; plotly interactive |
| **Performance** | Slower on large-scale ML; C/C++ extensions (Rcpp) for speed | NumPy/PyTorch leverage BLAS/GPUs effectively |
| **Deployment** | Harder (Shiny, plumber for APIs); Docker helps | Mature (FastAPI, Django, containerization) |
| **Ecosystem** | CRAN (vetted); strong domain packages | PyPI (vast, uneven quality) |
| **Learning Curve** | Vector-centric; functional paradigm can surprise | Imperative; more intuitive for programmers |

### When to Choose Each

- **R**: Research, statistics-heavy workflows, ggplot2-level visualization control, scientific papers
- **Python**: Production systems, large-scale ML pipelines, full-stack web applications, cross-domain scripting

Increasingly: **both**. R calls Python via `reticulate`; Python calls R via `rpy2`.

## Ecosystem & Community

- **CRAN**: ~20k packages, strict vetting, long-term stability
- **Bioconductor**: domain-specific (genomics, bioinformatics), high quality
- **GitHub packages**: cutting-edge, installed via `devtools::install_github()`
- **Community**: RStudio (now Posit) workshops; conferences (useR!, rstudio::conf); #rstats on Twitter/GitHub heavy presence

Modern R is not legacy R; tidyverse + Shiny represent substantial innovation in data science tooling, with trade-offs balanced toward interactive work over production scale.