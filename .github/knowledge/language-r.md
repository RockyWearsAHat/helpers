# R Best Practices

## R Philosophy

R is the lingua franca of statistical computing and data science. The tidyverse ecosystem provides a consistent, expressive grammar for data manipulation and visualization.

- **Vectorize everything**: R operates on entire vectors/columns, not scalar loops.
- **Tidyverse for data wrangling**: dplyr, tidyr, ggplot2, purrr — consistent, pipe-friendly APIs.
- **Functional style**: Map/reduce over for loops. Functions are first-class.

## Tidyverse Data Manipulation

```r
library(tidyverse)

# Pipe-based data wrangling
results <- data |>
  filter(age >= 18, status == "active") |>
  mutate(
    bmi = weight / (height / 100)^2,
    age_group = case_when(
      age < 30  ~ "young",
      age < 60  ~ "middle",
      TRUE      ~ "senior"
    )
  ) |>
  group_by(age_group) |>
  summarise(
    n = n(),
    mean_bmi = mean(bmi, na.rm = TRUE),
    sd_bmi = sd(bmi, na.rm = TRUE),
    .groups = "drop"
  ) |>
  arrange(desc(mean_bmi))
```

## Vectorization

```r
# BAD: scalar loop
result <- numeric(length(x))
for (i in seq_along(x)) {
  result[i] <- x[i]^2 + 1
}

# GOOD: vectorized
result <- x^2 + 1

# Conditional vectorization
ifelse(x > 0, sqrt(x), NA_real_)

# Apply family (when vectorization isn't straightforward)
sapply(list_of_dfs, nrow)
purrr::map_dbl(list_of_values, ~ .x^2 + 1)
```

## ggplot2 Visualization

```r
ggplot(data, aes(x = age, y = income, color = education)) +
  geom_point(alpha = 0.6, size = 2) +
  geom_smooth(method = "lm", se = TRUE) +
  facet_wrap(~region, scales = "free_y") +
  scale_color_viridis_d() +
  labs(
    title = "Income by Age and Education",
    x = "Age (years)",
    y = "Annual Income ($)",
    color = "Education Level"
  ) +
  theme_minimal(base_size = 14)

ggsave("plot.png", width = 10, height = 6, dpi = 300)
```

## Data Types and Structures

```r
# Vectors (atomic — single type)
nums <- c(1, 2, 3, 4, 5)
chars <- c("a", "b", "c")
logicals <- c(TRUE, FALSE, TRUE)

# Factors (categorical data)
status <- factor(c("low", "med", "high"), levels = c("low", "med", "high"), ordered = TRUE)

# Data frames / tibbles
df <- tibble(
  id = 1:100,
  name = paste0("person_", 1:100),
  score = rnorm(100, mean = 50, sd = 10)
)

# Lists (heterogeneous)
model_result <- list(
  coefficients = c(2.5, 0.3),
  r_squared = 0.87,
  residuals = rnorm(100)
)
model_result$r_squared  # 0.87
```

## Functions

```r
# Functions with defaults and type discipline
calculate_ci <- function(x, confidence = 0.95, na.rm = TRUE) {
  if (!is.numeric(x)) stop("x must be numeric")

  x <- if (na.rm) x[!is.na(x)] else x
  n <- length(x)
  se <- sd(x) / sqrt(n)
  alpha <- 1 - confidence
  z <- qnorm(1 - alpha / 2)

  list(
    mean = mean(x),
    lower = mean(x) - z * se,
    upper = mean(x) + z * se,
    n = n
  )
}

# Purrr for functional iteration
map(list_of_dfs, ~ .x |> filter(score > 50) |> nrow())
map2(xs, ys, ~ .x + .y)
pmap(list(a = 1:3, b = 4:6, c = 7:9), sum)
```

## Statistical Modeling

```r
# Linear model
model <- lm(mpg ~ wt + hp + factor(cyl), data = mtcars)
summary(model)
confint(model)

# Tidy model output (broom)
library(broom)
tidy(model)     # coefficient table as tibble
glance(model)   # model-level statistics
augment(model)  # observation-level fitted values / residuals

# Multiple models (nest-map-unnest pattern)
models <- data |>
  group_by(category) |>
  nest() |>
  mutate(
    model = map(data, ~ lm(y ~ x, data = .x)),
    tidied = map(model, tidy)
  ) |>
  unnest(tidied)
```

## Reading/Writing Data

```r
# CSV (readr — fast, consistent)
data <- read_csv("data.csv", col_types = cols(
  id = col_integer(),
  date = col_date(),
  value = col_double()
))
write_csv(data, "output.csv")

# Excel
library(readxl)
data <- read_excel("data.xlsx", sheet = "Sheet1")

# Parquet (for large datasets)
library(arrow)
data <- read_parquet("data.parquet")
write_parquet(data, "output.parquet")

# Database
library(DBI)
con <- dbConnect(RSQLite::SQLite(), "database.db")
results <- dbGetQuery(con, "SELECT * FROM users WHERE age > 18")
dbDisconnect(con)
```

## Key Rules

1. **Vectorize, don't loop.** If you have a for loop, there's probably a vectorized alternative.
2. **Use `NA` correctly.** `na.rm = TRUE` in aggregations. Check with `is.na()`, never `== NA`.
3. **Use tibbles over data.frames.** Better printing, no partial matching, no `stringsAsFactors`.
4. **Use `|>` (base pipe, 4.1+) or `%>%` (magrittr).** Pipes make data transformations readable.
5. **Avoid `attach()`, `<<-`, and modifying the global environment.**
6. **Use `here::here()` for file paths.** Reproducible across systems.

---

_Sources: R for Data Science (Wickham & Grolemund), Advanced R (Wickham), tidyverse.org, CRAN documentation_
