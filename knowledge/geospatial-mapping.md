# Web Mapping — Tiles, Libraries, and Rendering

## Tile-Based Architecture

Web maps (Google Maps, OpenStreetMap) serve pre-rendered **tiles**: square PNG or JPEG images, typically 256×256 pixels, at multiple zoom levels. A tile server stores tiles for zoom 0 (whole Earth = 1 tile) through zoom 20+ (each tile covers a few city blocks). The browser fetches only the tiles visible in the viewport, enabling fast panning.

Map tile URLs follow a pattern: `/z/x/y.png` where `z` is zoom level (0 = world, 20 = high detail), `x` and `y` are grid coordinates. At zoom 2, the world is 4×4 tiles (16 total); at zoom 12, it's 4096×4096 tiles (16 million). A tile server typically caches tiles on disk or in CDN.

**Raster tiles** (PNG, JPEG) are pre-rendered images — fast to serve but inflexible. If you need a different style, you re-render all tiles. **Vector tiles** (MVT format — Mapbox Vector Tile) store geometry primitives (points, lines, polygons) and styling at the tile level. The client renders them (WebGL or Canvas) with custom styles. Vector tiles are smaller on the wire, enable client-side filtering and styling, but require client-side rendering compute.

## Vector Tiles and MVT Format

**MVT** (Mapbox Vector Tile) is a binary format that encodes geometries and attributes hierarchically in compressed protobuf. A single MVT tile can contain multiple layers (roads, buildings, water, etc.). At low zoom levels, tiles contain simplified geometries; at high zoom, full detail. Client-side rendering applies style rules ("color roads red, buildings gray") after fetching.

Tradeoff: Vector tiles use less bandwidth (smaller files due to protobuf compression and geometry simplification) and enable dynamic styling, but require capable client rendering (WebGL context, decent CPU). Raster tiles are simpler (just images) but inflexible and larger.

**GeoTIFF** is a tagged image format for raster imagery (satellite photos, elevation data). A GeoTIFF includes geospatial metadata (coordinate system, geotransform) embedded in the file. Tools like GDAL can read GeoTIFF files and convert them to tile pyramids for web serving.

## Web Mapping Libraries

**Leaflet** is lightweight (~42 KB) and simple. It abstracts tile layer management, markers, popups, and basic interactions (pan, zoom, dragging). Leaflet works with raster tiles, GeoJSON, and WMS layers. Strength: easy to learn, works on mobile, large ecosystem of plugins. Weakness: doesn't handle 3D, vector tiles, or complex styling well.

**Mapbox GL JS** is built for vector tiles and WebGL rendering. It stores vector tiles and applies declarative style specifications (Mapbox Style) using shaders. Supports dynamic styling, 3D extrusion (building heights), terrain, data-driven colors ("color buildings by height"). Strength: modern, performant, feature-rich. Weakness: proprietary (Mapbox tiles require API key and billing), more complex API.

**Google Maps API** provides pre-rendered raster tiles, satellite imagery, and routing. It's ubiquitous and well-documented but requires API key, has rate limits, and bills by requests. No vector tiles. Good for simple use cases; custom maps require Mapbox or Leaflet.

## Style Specifications and Data-Driven Styling

**Mapbox Style Specification** (declarative JSON) defines how to render vector tiles:
```json
{
  "layers": [{
    "id": "buildings",
    "source": "vector-tiles",
    "source-layer": "building",
    "type": "fill",
    "paint": {
      "fill-color": ["interpolate", ["linear"], ["get", "height"],
        50, "#f0f0f0",
        200, "#808080"
      ]
    }
  }]
}
```

This colors buildings by height: heights near 50 → light gray; heights near 200 → dark gray. **Data-driven styling** binds geometry appearance to properties in the data, enabling rich visualization without code duplication.

## Marker Clustering

When zooming out from a map with thousands of markers, rendering all markers separately creates clutter and poor performance. **Marker clustering** aggregates nearby markers into clusters. At low zoom, a single circle showing "237 markers" is rendered; zooming in reveals subclusters, then individual markers.

Libraries like Leaflet.markercluster or Mapbox Cluster Layers handle this. Clustering algorithms (e.g., k-means on marker coordinates) or spatial indexing (R-tree) determine which markers belong to clusters at each zoom level.

## 3D Terrain and Indoor Mapping

**3D terrain** renders elevation data as textured geometry. Mapbox GL JS supports terrain using elevation raster tiles (mapbox-raster-dem format) and shaders. A camera elevation angle and observer height create perspective-rendered mountains and valleys. This is computationally expensive (WebGL) but visually striking.

**Indoor mapping** renders building interiors (floor plans, room layouts). Google Maps, Apple Maps, and some Mapbox integrations support indoor maps for shopping malls, airports, offices. Data is typically stored as GeoJSON or MVT with source-layer separation by floor. Rendering toggles between levels; routing considers indoor connectivity. Complexity: building interior geometry, floor linking, semantic room data (names, functions).

## Tile Servers and Serving Strategies

Ready-made tile servers: OpenStreetMap CDN (free raster), Mapbox (vector + raster, paid), Stamen Design (free artful raster). Self-hosted: Tileserver GL (serves pre-built vector tiles), TileProxy (caches and combines sources).

Serving raster or vector tiles at scale requires:
1. **Tile generation**: Render (raster) or extract (vector) at multiple zoom levels — compute-intensive, typically batch offline.
2. **Caching**: On CDN near end-users for fast delivery.
3. **Updates**: Scheduled re-rendering when source data (OSM, satellite imagery) changes.

For small regions or custom data, pre-generating rarely pays off; on-demand tile rendering with caching is simpler. For global baseline maps, pre-gen is standard.

## Related Concepts

See also: [geospatial-fundamentals.md](geospatial-fundamentals.md) for coordinate systems and GeoJSON, [web-image-optimization.md](web-image-optimization.md) for image compression strategies, [graphics-rendering.md](graphics-rendering.md) for WebGL and rendering architectures.