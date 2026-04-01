# Geospatial Fundamentals — Coordinate Systems, Formats, and Spatial Indexing

## Coordinate Systems: WGS84, UTM, and EPSG Codes

**WGS84** (World Geodetic System 1984) is the global standard coordinate system. It's geocentric (origin at Earth's center of mass) and defines latitude/longitude as decimal degrees. WGS84 provides ~1 meter global consistency and is the reference system behind GPS. Most web maps and geospatial APIs default to WGS84.

The tradeoff: WGS84 is convenient for planetary-scale data and interoperability, but distances are not uniform. One degree of latitude is always ~111 km, but one degree of longitude varies by latitude: at the equator it's ~111 km, but at 60° north it's only ~55.5 km. For local measurements and distance calculations, projected coordinate systems are more useful.

**UTM** (Universal Transverse Mercator) divides the Earth into 60 zones, each 6 degrees of longitude wide, running from 80°S to 84°N. Within each zone, distances are approximately preserved (low distortion). UTM uses easting/northing in meters, making calculations straightforward. The tradeoff: UTM requires zone selection—a point near a zone boundary requires careful handling. Different zones have different coordinate scales, complicating multi-zone workflows.

**EPSG codes** are numeric identifiers for coordinate systems and transformations. EPSG:4326 is WGS84 (latitude, longitude). EPSG:3857 is Web Mercator, used by most web tile providers. EPSG:2154 is a French Lambert projection. These standardize reference across tools and datasets; without them, "37.7749, -122.4194" is ambiguous—degrees in which datum?

## GeoJSON: Format for Geometry and Features

**GeoJSON** (RFC 7946) is a JSON-based format for encoding geographic data structures. It supports Point, LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon, and GeometryCollection. Each geometry includes optional properties (name, color, etc.) and a coordinate array in [longitude, latitude] order (note: longitude first, breaking common lat/lon convention).

Example:
```json
{
  "type": "Feature",
  "geometry": {
    "type": "Point",
    "coordinates": [-122.4194, 37.7749]
  },
  "properties": {"city": "San Francisco"}
}
```

GeoJSON strength: human-readable, web-native, works directly in JavaScript. Weakness: no built-in indexing. A GeoJSON file of 100k points is text, not indexed—querying requires scanning all records. For indexed spatial queries, databases (PostGIS) or specialized spatial stores are needed.

## Spatial Indexing: R-tree, S2, H3, Geohash

**R-tree** (1984, Antonin Guttman) hierarchically groups nearby spatial objects using minimum bounding rectangles. Leaf nodes store individual geometries; internal nodes store the bounding boxes of children. Queries ("find all restaurants within 2 km") recursively prune irrelevant subtrees. R-trees are standard in relational databases (PostgreSQL's GiST/BRIN), spatial databases (Oracle Spatial), and PostGIS.

Trade: R-trees have good worst-case complexity for range queries but variable performance dependent on tree balance and data distribution. They're mutable (support insertion/deletion) and suitable for disk-based storage.

**Geohash** interleaves latitude and longitude bits into a single bit string, then encodes as base-32 characters. Example: San Francisco is `9q8yy`. Nearby points have similar prefixes, enabling prefix-based spatial queries. Geohashing trades accuracy for simplicity; a 6-character geohash covers ~1 km² but loses fine-grained precision.

**S2 Geometry** (Google) tessellates the Earth into a hierarchical grid using Hilbert curves on cube faces. It preserves spatial locality better than geohashing and supports efficient nearest-neighbor and region-covering queries. S2 is used in Google Maps and Spanner database. Tradeoff: more complex to understand and implement than geohashing, but faster for complex queries.

**H3** (Uber) uses hexagonal indexing on a spherical geodesic discrete global grid. It provides property that similar H3 indices are geographically close, supports variable-resolution hierarchies, and avoids latitude/longitude distortion issues. H3 is popular for ride-hailing and regional analysis. Tradeoff: hexagonal approximation introduces small errors compared to exact polygons.

## Spatial Queries: Geometry vs. Geography

**Geometry** types (used by PostGIS, MySQL Spatial) treat coordinates as Euclidean points in a flat plane. `ST_Contains(poly, point)` checks point-in-polygon using planar math, `ST_Distance(a, b)` uses Pythagorean distance. Fast but only accurate over small regions; at continental scale, Earth curvature introduces error.

**Geography** types (PostGIS only) compute on the sphere using proper geodetic calculations. `ST_DWithin(point_a, point_b, 1000 * 1000 /* 1000 km */)` returns true if two points are within 1000 km on Earth's surface (great-circle distance). Slower than geometry queries but correct for continental/global data.

Choice: Use geometry for local applications (city-scale), where curvature is negligible and speed matters. Use geography for global data (flight paths, shipping routes, worldwide monitoring). A distance of 10 km at the equator is 10 km; at 60° latitude, the same degree-difference is ~5.5 km, making geography correctness critical at scale.

## PostGIS: Spatial Extension for PostgreSQL

PostGIS adds geometry and geography types, spatial indexing (GiST/BRIN), and functions (`ST_Intersects`, `ST_Length`, `ST_Buffer`, `ST_Centroid`). It stores spatial data in tables, supports indexing for queries over millions of geometries, and integrates with SQL.

Common pattern:
```sql
SELECT id, name FROM restaurants 
WHERE ST_DWithin(location, ST_Point(-122.4194, 37.7749), 1000)
ORDER BY ST_Distance(location, ST_Point(-122.4194, 37.7749));
```

This finds restaurants within 1 km and sorts by distance. PostGIS internally uses the spatial index to avoid scanning every row. For large datasets (millions of locations), this is how geo-queries scale.

## Related Concepts

See also: [architecture-search-platform.md](architecture-search-platform.md) for geo-indexing in search systems, [database-indexing-strategies.md](database-indexing-strategies.md) for B-tree and GiST concepts, [cs-information-retrieval.md](cs-information-retrieval.md) for querying.