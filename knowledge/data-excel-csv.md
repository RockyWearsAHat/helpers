# Programmatic Data Formats — CSV, Excel, Parquet & Arrow

Programmatic data manipulation requires understanding how to parse, generate, and interchange tabular data. The landscape spans text formats (CSV), binary spreadsheet formats (Excel), columnar analytics formats (Parquet, Arrow), and their trade-offs.

## CSV: The Format Without Specification

CSV (RFC 4180, 2005) is the paradox of data interchange: ubiquitous yet formally underspecified until recently. The standard defines:

- **Line breaks**: CRLF (carriage return + line feed) as record delimiters
- **Field delimiters**: commas separate fields within records
- **Headers**: optional first-line column names
- **Quoting**: double-quoted fields can contain commas, line breaks, and escaped quotes (doubled: `""`). Unquoted fields must not contain special characters.
- **Edge case**: spaces around delimiters are part of field values (no implicit trimming)

### CSV Parsing Edge Cases

**The quoting problem**: RFC 4180 allows but doesn't mandate quotes. Some producers (Excel exports, old CSV writers) omit quotes entirely, conflicting with the spec. Parsers must decide: enforce strict RFC 4180, or be liberal?

**Line ending ambiguity**: The spec prescribes CRLF, but operating systems historically used LF (Unix) or CR (old Mac). Real-world CSV files mix them. Robust parsers detect and normalize.

**Encoding transparency**: CSV has no standard encoding marker. UTF-8 is modern practice, but Latin-1, ISO-8859-1, and Windows-1252 appear frequently in legacy data. Encoding detection is heuristic and fallible.

**Numeric type inference**: Pure CSV has no type system — all fields are strings. Parsers guess: "123" → integer/float, "true" → boolean, "" → null/missing. Type inference is fragile and depends on column context (looking across rows to infer uniformity).

**Delimiter detection**: When delimiter is unknown, heuristics examine the first few lines. Tab, semicolon, and pipe are alternatives to comma. Delimiter detection fails on malformed headers or when data contains the candidate delimiter.

### Implementation Strategies

Libraries evolve along different axes:

- **Pandas** (Python): Offers `read_csv()` with pluggable engines (C, Python, PyArrow). C engine is fast. Python engine is feature-rich but slower. PyArrow balances speed and completeness, supports chunked reading, and auto-detects separators.
- **Node.js**: `papaparse` prioritizes correctness and RFC 4180 compliance. `csv-parser` emphasizes streaming and low memory. Both transform CSV streams into object arrays.
- **Rust**: `csv` crate provides zero-alloc parsing; `polars` layers analytics on top with auto-detection and type inference.

**Key parser parameters**:
- `delimiter`: Must be explicitly set if non-comma
- `quotechar` (usually `"`): Escape mechanism
- `encoding`: Crucial for international data
- `parse_dates`: Enable heuristic date parsing post-parse
- `on_bad_lines`: Error policy when field counts mismatch

## Excel: Beyond the Text Format

Excel files (.xlsx, .xlsm, .xltx) use Office Open XML (OOXML), a ZIP archive containing XML documents plus media. This format enables:

- **Styling**: fonts, colors, borders, cell alignment (metadata CSV cannot store)
- **Formulas**: embedded calculations (static after export to CSV)
- **Multiple sheets**: workbook tabs
- **Named ranges**: semantic grouping
- **Comments and annotations**: metadata on cells

Programmatic Excel manipulation requires either:

1. **Native libraries parsing OOXML**:
   - `openpyxl` (Python): reads/writes `.xlsx` and `.xlsm`. Supports formulas (evaluated by Excel, not calculated), styled output. Column/row organization is row-major (iteration optimizes for row access, not column).
   - `ExcelJS` (Node.js): similar scope. Supports streaming reads (memory-efficient for huge files), formula writing.
   - `xlwt/xlrd` legacy stack (Python): older `.xls` format (BIFF), no longer maintained for `xls` writing.

2. **IPC communication with Excel**:
   - VBA (Visual Basic for Applications): Excel's macro language; limited, platform-specific (Windows)
   - COM interop (.NET) / UNO (LibreOffice): spawns Excel/Calc as a subprocess, sends RPC commands. Expensive, fragile, requires Excel installed.

### Operation Patterns

**Read**:
```
Load workbook → Select sheet → Iterate rows/columns → Extract values
```

**Write**:
```
Create workbook → Add sheet → Write cells (addressing: row/col indices or letters: A1, B2)
→ Set styles/formulas → Save to .xlsx
```

**Key limitation**: Formulas are stored as text strings, not re-evaluated. Re-calculating a workbook programmatically requires opening it in Excel or using a headless evaluator (rare, complex).

## Parquet: Columnar Analytics Format

Parquet (Apache) is the standard columnar storage for analytical data lakes. Unlike CSV (row-oriented), Parquet stores data **column-by-column**, enabling:

- **Compression**: Same-typed columns compress 5–10x better than heterogeneous row data
- **Predicate pushdown**: Filters (e.g., `WHERE age > 30`) evaluated at file-read time using column statistics; entire column chunks skipped without decompression
- **Selective reads**: Queries accessing five of 100 columns read only those five, not the whole file
- **Statistical metadata**: Per-column min/max, null counts, distinct value counts tracked in file footer for query optimization

**Internal structure**:
- **Row group**: horizontal partition (e.g., 100K rows per group)
- **Column chunk**: one column's data within a row group
- **Page**: compressed unit within column chunk (e.g., 10K values per page)
- **Footer**: schema, statistics, file metadata

**Trade-offs**: Parquet excels at analytical workloads (wide tables, selective columns, high compression). Insert/update performance is poor — Parquet files are immutable after creation. For transactional data, use row stores (CSV, SQLite, Postgres).

## Arrow: In-Memory Columnar & IPC

Apache Arrow defines two things:

1. **In-memory format**: Columnar, zero-copy layout enabling data sharing between processes without serialization overhead. Buffers follow a spec: fixed-width slots, variable-length data in separate buffers, null bitmaps.

2. **Inter-process communication (IPC)**: Arrow Flight (gRPC-based protocol) and Arrow IPC on disk (Arrow File, Streaming formats).

**Why Arrow matters**: Parsing CSV into Arrow table is faster than parsing into native objects (NumPy arrays, Pandas DataFrames) because Arrow avoids object allocation overhead. Downstream processing (Polars, DuckDB) consumes Arrow buffers directly.

**Pattern**: 
```
CSV → Arrow Table → Parquet (columnar serialization) → Storage/Analysis
```

## Data Interchange Patterns

**Batch pipeline** (most common):
- Extract: read CSV/Parquet from data source
- Transform: filter, aggregate, join in-memory or via SQL
- Load: write Parquet/Arrow to data lake or database

**Streaming pipeline**:
- Read CSV in chunks (e.g., 10K rows at a time) to bound memory
- Apply transformations per chunk
- Emit results incrementally to downstream system

**Schema-driven**:
- Define schema upfront (data types, required fields, nullability)
- Validate incoming CSV/Parquet against schema
- Fail fast on type mismatches or missing required columns
- Enables type safety and downstream code that assumes types

**Data contract**, emerging pattern:
- Teams publish schemas and SLAs (data freshness, accuracy)
- Consumers depend on contracts, not ad-hoc parsing
- Enables decoupled evolution: producer can add optional columns without breaking consumers

See also: [data-serialization-formats](data-serialization-formats.md), [data-engineering-formats](data-engineering-formats.md), [data-engineering-etl](data-engineering-etl.md)