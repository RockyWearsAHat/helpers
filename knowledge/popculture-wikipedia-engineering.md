# Wikipedia & MediaWiki: Remarkable Scale with a Tiny Engineering Team

## Overview

Wikipedia runs on **MediaWiki**, an open-source wiki engine written in PHP. As of 2026, Wikipedia serves ~500 million monthly users, handles ~5 billion page views per month, and stores 60+ million articles across 300+ languages. Yet the **Wikimedia Foundation** engineering team is shockingly small: ~150 total staff, of which only ~30–40 work on core platform engineering.

For comparison, a single Netflix team (100+ engineers) manages content delivery for 200 million users. Wikipedia does the same with <10% the staff. This is not accidental; it's the result of deliberate architectural choices and a unique human-powered content curation model that precedes the ML boom by decades.

## Architecture: Shared-Nothing, Cache-Heavy

### Data Model: Relational Simplicity

MediaWiki stores pages in a straightforward relational schema:

- **`page` table**: Title, namespace, page_id, restrictions
- **`revision` table**: Every edit ever; page_id, timestamp, editor, comment, text_id
- **`text` table**: Page content (stored once, deduped by text_id)
- **`user` table**: Registered editors

This is **intentionally minimal**. No JSON objects, no document store, no graph database. The schema is ~20 tables. An edit records one row in `revision` and one in `text` (if new content).

### Horizontality and Caching

The entire read workload is satisfied by **two strategies**:

**1. Parser output cache**

The expensive operation: rendering MediaWiki markup (double brackets for citations, templates, etc.) into HTML.

MediaWiki caches:
- `job_queue`: Places rendering jobs (from edits) into a queue, processes them asynchronously
- `bluesky_parser_cache`: Stores rendered HTML (mw-cache-table)
- A write-through cache layer: On edit, the rendered version is pre-computed and stored

Result: **Page requests are nearly O(1) if cached.** Most traffic hits the cache before it touches the database.

**2. Database replica cluster**

The **primary database** is on one master (writes only). Reads go to a **replica cluster**:

- Writes go to master (synchronous replication lag: <1 second)
- Reads go to replica pool (asynchronous, eventual consistency acceptable)

Replicas can be lagged without breaking the experience (you might see slightly old talk page comments, but articles are stable).

This is **classic CQRS** (Command Query Responsibility Segregation), implemented 15+ years ago before the term was fashionable.

### Memcached Tier

**Memcached** sits in front of the database, hitting it at:
- Rendered page cache
- User permissions (can this user edit?)
- Latest revisions

Cache invalidation policy: **Time-based eviction + explicit invalidation on edit.**

Because Wikipedians edit pages relatively infrequently (a page might get one edit per week on average), the cache hit rate is enormous: **>95%** for content retrieval.

## Vandalism Detection: Humans + ML

Wikipedia's content quality is defended by:

### 1. Human-Powered Rollback (Reactive)

- **Registered editors**: Watch pages they care about; see edits in real-time (RC feed)
- **Admin tools**: One-click rollback of an edit (revert page to previous version)
- **Bots**: Automated revert-on-vandalism patterns (blanking page = revert in <30 seconds)

This is both ML (automated pattern detection) and human (final judgment).

### 2. Edit Filters (Predictive)

MediaWiki Edit Filters are a **rule engine** (not ML, but pattern logic):

```
# Example: Detect massive deletion
if length_change < -500 && user_editcount < 10 then 
  action = warn | require_approval
```

Problematic edits are flagged or held in queue for review.

### 3. Abuse Filter Scoring (Statistical)

Wikimedia deployed statistical scoring:

- **Feature extraction**: Editor reputation, edit timing, content semantics, pattern matching
- **Scoring**: Predict abuse likelihood
- **Action**: Throttle or hold repeat offenders

This is **lightweight pre-ML abuse detection** (though modern Wikimedia uses some neural networks for ORES, Objective Revision Evaluation Service).

### Why This Scales Without a Huge Team

1. **Humans review edge cases** (10–20 admins on enwiki at any time, rotating)
2. **Simple ML flags suspicious edits** (not trying to catch all vandalism, just raise to humans faster)
3. **Community polices itself** (reputational incentive: take away edit rights if you spam)

Compare to Facebook content moderation: 15,000+ humans + ML still can't catch everything. Wikipedia outsources to 7,000 volunteer admins + simple automation. **Same quality, 1/1000th the staff.**

## The Wikidata Breakthrough

**Wikidata** (launched 2012) is a structured knowledge base underlying Wikipedia. It's semantically powerful but rarely mentioned outside Wikipedia circles.

### Structure

Wikidata models facts as triples:

```
[Human subject]
  | birthPlace | [place]
  | occupation | [job]
  | dateOfBirth | [date]
```

These are **decoupled from Wikipedia** articles. An article in enwiki, frwiki, dewiki can all *reference* the same Wikidata item.

### Why This Matters: Localization at Scale

Before Wikidata:
- Each language Wikipedia had its own article on "Albert Einstein"
- Facts were duplicated (birthed 1879, in-info box)
- Edits to one didn't sync to others

**After Wikidata**:
- One source of truth for structured data
- Infoboxes across 300+ language editions can pull from Wikidata
- A user edits birthplace once, 300 language articles update

This means **30 engineers can maintain content across 300 languages** because they're not duplicating facts.

### Wikidata as Open Data Infrastructure

Wikidata is also a **publicly queryable knowledge graph**:

```sparql
SELECT ?person ?personLabel WHERE {
  ?person wdt:P31 wd:Q5 .        # instance of human
  ?person wdt:P19 wd:Q1758 .     # born in Berlin
}
```

This single dataset has become:
- Input to academic research on knowledge graphs
- Foundation for Linked Open Data initiatives
- Source for AI/ML training (though increasingly controversial)

This is remarkable: **a volunteer project built open-source knowledge infrastructure** that academia and AI labs rely on.

## Database Scaling Without Sharding

By 2015, enwiki (English Wikipedia) was too large for a single database:

- 60 million pages
- 600 million revisions
- >1 TB of data

Rather than shard, Wikimedia adopted **logical separation**:

1. **Primary database**: Main content (articles, user pages)
2. **Replica datacenter**: Read-only copy (for reads, geographic distribution)
3. **Per-language servers**: Smaller languages run on shared hosts
4. **Archive cluster**: Old revisions moved to compressed archive storage

**Key insight**: Most reads are recent content (few people query 10-year-old revisions). Old data can be stored separately (compressed, slow).

This is a **tiered storage architecture** (hot/warm/cold data), not implemented via expensive sharding.

## Why MediaWiki Succeeded Where Others Failed

### 1. Simple, Understandable Code Path

MediaWiki's architecture is PHP + MySQL, readable end-to-end:

```php
// Article page handler
$article = Article::newFromTitle($title, $db);
$html = $article->getRenderCache();
if (!$html) {
  $html = $parser->parse($article->getText(), $title);
  $cache->set($article->getId(), $html);
}
echo $html;
```

Anyone could trace an issue: "Why is page slow?" → "Check cache hit rate" → "Check database query" → "Check parser output."

No black boxes. No "just deploy more containers and let the orchestrator sort it out."

### 2. Horizontal Scalability Without Coordination

Most scaling came from:
- Adding read replicas (no coordination needed, just replicate)
- Adding cache nodes (stateless, no consistency problem)
- Adding web servers (stateless)

**No distributed consensus, no ACID transactions needed at scale.** Wikipedia's workload is mostly reads; writes (edits) are rare and conflict is rare (most edits don't overlap).

This avoided the **complexity explosion** that toppled other platforms (e.g., Twitter's early scaling problems were largely due to trying to shard mutable state).

### 3. Community as Force Multiplier

Volunteer admins:
- Detect problems (spam, vandalism, harassment)
- Document solutions (help pages, policies)
- Train new editors

This is **non-scaling labor**, but it's distributed. No single point of failure or bottleneck.

Modern AI companies are trying to build recommendation systems with centralised ML. Wikipedia built a curation system distributed across 5,000+ volunteer reviewers. **The same outcome with a fundamentally different model.**

## Technical Lessons

### 1. Simple Architecture Scales Longer Than Complex

Wikipedia's 20-table schema, PHP, MySQL, Memcached stack served 500M users. By contrast,:
- Uber needed microservices, distributed tracing, and service mesh just to manage 100M users
- Twitter needed to completely rewrite multiple systems as they scaled

The difference? Wikipedia's workload was **read-heavy and non-critical-latency** (no real-time consistency needed). Uber's workload requires immediate consistency (if I order a ride, the driver sees it now).

Choose architecture for your workload, not for "scale."

### 2. Open Source Core Enables Experimentation

MediaWiki being open source meant:
- Anyone could run it (universities, nonprofits, private wikis) and report issues
- Thousands of public sites running MediaWiki caught edge cases before Wikimedia hit them
- A decade of community patches preceded Wikimedia's own scaling work

Proprietary systems (WordPress.com, Confluence, etc.) only saw bugs at the scale they experienced internally.

### 3. Content Curation Beats AI Moderation (For Now)

Vandalism detection and edit filtering at Wikipedia is:
- ~70% human judgment
- ~20% statistical rules
- ~10% neural network scoring (ORES)

It works better than platforms that flip the ratio (trusting algorithms, humans as appeal layer). Because:
- Humans with domain knowledge (regular editors) know local norms
- Context is important (a blank page from a new user is vandalism; from a long-trusted editor might be intentional cleanup)
- Mistakes are cheaper than incompleteness (better to miss vandalism than to block honest edits)

This philosophy contrasts with Facebook/Twitter's strategy of scaling ML first and asking humans to clean up mistakes later.

## Organizational Structure

### Why It's So Small

Wikimedia's lean engineering philosophy:

- **Core platform team**: ~30 people (maintain MediaWiki engine, database, infrastructure)
- **Product teams**: ~20 people (features for editors: UX, mobile, API)
- **Other**: Support, community managers, operations

**No:** dedicated UI designer per language, localization team (community does it), content moderation staff (admins volunteer).

The bet: **Build the platform. The community maintains it.**

This only works if:
1. The platform is simple enough for volunteers to contribute
2. You have enough volunteer labor (300+ languages means 300 translation communities)
3. You don't need real-time support (wiki culture accepts "someone will fix it eventually")

## See Also

- architecture-caching-strategies.md (tiered caching, cache invalidation)
- database-scaling.md (replication, sharding alternatives)
- process-open-source-governance.md (how communities replace central infrastructure teams)
- ml-moderation-systems.md (how content moderation scaled)