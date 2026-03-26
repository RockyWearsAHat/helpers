# Data Science Visualization — Grammar of Graphics, Design, and Tools

## Grammar of Graphics Foundation

Data visualization rests on a formal system called the **grammar of graphics** (ggplot2, Vega-Lite). This declarative grammar separates visualization into independent layers:

- **Data**: A dataset with variables (columns)
- **Aesthetic mapping** (`aes`): Binding variables to visual properties (position, color, size, shape)
- **Geometric objects** (`geom`): Marks that appear (points, lines, bars, areas, text)
- **Statistical transformations** (`stat`): Computed values (binning, smoothing, density estimation)
- **Coordinate system** (`coord`): Space where marks are placed (Cartesian, polar, map projections)
- **Facets**: Small multiples partitioned by categorical variables
- **Scales**: Mapping from data values to visual encoding ranges (linear, log, categorical palettes)
- **Themes**: Non-data elements (fonts, backgrounds, gridlines)

The grammar unifies chart types under one system: a bar chart and scatter plot differ only in their geoms and coordinate systems, not fundamental structure. This **composability** enables rapid exploration and consistent syntax across tools.

Implementations differ in scope:
- **ggplot2** (R): Comprehensive, statistical focus, publication-quality defaults
- **Vega-Lite** (JSON/Python/JavaScript): Specification-first, web-native, declarative
- **Plotly/Altair** (Python): Vega-Lite wrappers with interactive defaults
- **D3.js**: Low-level primitives; requires explicit code for each layer

## Chart Selection and Data-Ink Ratio

Chart choice depends on the data structure and question:

| Task | Good Choices | Avoid |
|------|--------------|-------|
| **Trends over time** | Line (trend focus), area (accumulation), ribbon (uncertainty) | Pie, radar |
| **Distributions** | Histogram (counts), density (smooth), box plot (quartiles), violin (full shape) | 3D pie, coin series |
| **Comparisons** | Grouped/faceted bar, dot plot, slope chart (change) | Stacked-percentage unless composition matters |
| **Part-to-whole** | Stacked bar (ordinal), treemap (hierarchy), sunburst (nested categories) | Pie (unless 2-3 slices); angles are hard to judge |
| **Correlation** | Scatter (continuous-continuous), heatmap (matrix), parallel coordinates (multivariate) | Line joining unrelated points |

**Data-ink ratio** (Tufte): Maximize the proportion of ink used to represent data vs. decoration. Remove gridlines, redundant labels, and "chart junk." Every visual element must earn its presence.

Related principle: **gestalt grouping**. Viewers automatically group by proximity, color, and shape. Use these cues intentionally; conflicting cues create cognitive load.

## Color in Visualization

Color serves three roles:

1. **Sequential palettes** (light → dark): Represent continuous, ordered data (temperature, income, concentration). Perceptually uniform palettes (Viridis, Cividis) vary brightness consistently across the spectrum, preventing false midpoint emphasis.

2. **Diverging palettes** (extremes with neutral center): Highlight deviation from a reference (e.g., temperature anomalies around 0). Often blue-red or purple-orange.

3. **Categorical palettes** (distinct hues): Show unordered categories. Limit to 5-7 colors; beyond that, names or legends become necessary.

**Accessibility constraints**:
- ~8% of males are red-green colorblind (protanopia/deuteranopia). Avoid red-green for encoding.
- Colorblind-friendly palettes exist (Cividis, Color Brewer safe palettes). Test designs with color blindness simulators.
- Ensure sufficient **contrast ratio** (WCAG AA: 4.5:1 for text, 3:1 for graphics) for users with low vision.
- Never encode information in color alone; use shape, pattern, or text redundantly.

## Storytelling and Dashboard Design

Effective data visualization communicates a narrative:

1. **Top-down design**: Lead with the key insight, then support with details. Headlines should state the finding, not the metric.
2. **Context matters**: Provide reference points (benchmarks, historical baselines, comparable companies) so viewers assess magnitude.
3. **Visual hierarchy**: Use size, color, and position to guide eye to the most important element first.
4. **Reduce cognitive load**: One idea per chart; complex relationships require multiple linked views or small multiples.

**Dashboard design** principles:
- **Grid-based layout**: Organize charts on a consistent grid to reduce visual noise.
- **Interactivity cost**: Each filter/selector adds friction. Provide only essential controls.
- **Responsive design**: Charts must adapt to container width without information loss. Stack on mobile; responsive fonts.
- **Updated frequency signals intent**: Real-time dashboards imply operational decisions; daily dashboards suit strategic review.

## D3.js Architecture and Observable

**D3.js** (Data-Driven Documents) is the dominant low-level visualization library, giving fine control over every pixel:

- **Data binding** (`data()`, `enter()`, `exit()`): Maps array elements to DOM elements (SVG or Canvas), enabling efficient updates when data changes.
- **Selections** (`select()`, `selectAll()`): CSS-like queries to target elements for manipulation.
- **Scales** (`scaleLinear()`, `scaleOrdinal()`): Functions mapping data domain to visual range.
- **Transitions** (`transition()`): Smooth animations between states.
- **Generators** (`line()`, `arc()`, `pie()`): Functions producing SVG path strings.

D3 excels at **bespoke, animated, interactive visualizations** (explorable explanations, custom network graphs). Cost: steep learning curve and verbose code compared to declarative systems.

**Observable** (notebooks by D3 creators) simplifies D3 development through:
- **Reactive cells**: Automatic recomputation when dependencies change
- **Inline visualization**: Charts render directly without explicit DOM manipulation
- **Import ecosystem**: Cells can depend on other published notebooks
- **Deployment**: Notebooks are live URLs, shareable without download

Observable popularized data journalism and scientific visualization; it bridges declarative and imperative approaches.

## Vega-Lite, Plotly, and Altair

**Vega-Lite** is a high-level visualization grammar compiled to the lower-level Vega specification:

- **JSON specification**: Plots defined declaratively (no custom code)
- **Interaction**: Built-in brushing, selection, cross-filtering without JavaScript
- **Composition**: Multiple views composed via `concat()`, `facet()`, `layer()`
- **Data transformations**: Filter, aggregate, calculate within the spec

**Altair** (Python) and **Plotly** (Python/JavaScript) wrap Vega-Lite or similar engines:
- Altair uses Vega-Lite's grammar with Python syntax
- Plotly adds 3D charts, Mapbox maps, and business defaults (emphasis on readability over ink efficiency)

Trade-off: Declarative systems prevent arbitrary customization; D3 offers freedom at the cost of complexity. Vega-Lite suits exploratory analysis and dashboards; D3 suits bespoke, publication-quality graphics.

## Accessibility and Inclusive Design

Web accessibility (WCAG 2.1) requires:

1. **Alternative text**: `<img alt="...">` or ARIA labels describe chart content for screen readers.
2. **Color contrast**: Text on background must meet minimum ratios; use color blindness simulators to test.
3. **Keyboard navigation**: Interactive elements (buttons, tooltips) must be reachable via Tab and Enter.
4. **Data table fallback**: Provide raw data table alongside visualization for users who cannot perceive visual encoding.
5. **Large click targets**: Hover areas in interactive charts should be ≥44×44px (mobile minimum).

Testing tools: WebAIM contrast checker, Coblis color blindness simulator, keyboard-only navigation, screen reader (NVDA, JAWS).

## Modern Visualization Ecosystem

Current landscape (2026):

- **Static & exploratory**: ggplot2 (R), Plotly (Python)
- **Web-declarative**: Vega-Lite, Altair, Observable
- **Custom web**: D3.js, Canvas

Emerging trends:
- **Grammar-of-graphics for JavaScript**: Nivo, Recharts bring R/Python declarative patterns to React
- **Linked views**: Brushing one chart to filter others (Observable, Plotly's `plotly.restyle`)
- **Geospatial**: Deck.gl (WebGL), Folium (Python) handle large geographic datasets
- **Temporal**: Specialized libraries (Recharts, Apache ECharts) optimize time-series rendering for dashboards

## See Also
- [api-rest-maturity.md](api-rest-maturity.md) — API design principles (relevant for data service specifications)
- [design-color-typography.md](design-color-typography.md) — Typography and color design foundations
- [web-accessibility.md](web-accessibility.md) — Web standards for accessible design