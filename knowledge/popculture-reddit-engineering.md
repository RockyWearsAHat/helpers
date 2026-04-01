# Reddit Engineering Lore: Scaling a Community Platform

## The Hug of Death

The "Reddit hug of death" (sometimes called "the Reddit effect") is what happens when a popular Reddit thread links to a small external website. The result is a sudden traffic spike that often overwhelms the target site's infrastructure. It's a variant of the "Slashdot effect"—named after similar overload events from the older tech news site Slashdot. The difference is comedic: instead of killing you with traffic, Reddit is "hugging you to death."

Technically, a hug of death is just **unplanned load testing**. When a thread with tens of thousands of upvotes sends traffic to a small site, request rates can grow 10–100x in minutes. CodinGame's 2016 case study is instructive: the platform went from baseline traffic to 10x higher within seconds of hitting Reddit's front page, then to overwhelm within hours. The database CPU capped; application servers crashed; forums and chat servers failed under load. What broke:

- **Centralized database**: every request hit a single RDS instance. Scaling the machine helped temporarily, but the bottleneck didn't go away.
- **Single-threaded services** (like their chat server using Prosody/Lua): peaked at one CPU core and ejected new connections rather than queueing them.
- **Shared infrastructure**: the forum and blog were on the same machine, so when the forum's SSO authentication queries saturated the CPU, the blog went down too.
- **File descriptor limits**: their WebSocket push server had a 10K limit on open connections—arbitrary, not thought through for scale.

The fix required horizontal scaling (load-balanced pools instead of single instances), distributed caching (memcached/Redis), better connection pooling, and separating concerns across machines. Notably, Reddit's own infrastructure had already solved these problems by the time CodinGame experienced theirs.

## Reddit's Own Scaling Journey

Reddit was built in Python, initially on a single box. As traffic grew, the platform evolved:

- **Caching layer**: Added memcached to cache hot posts, comment threads, and user data. Web servers could fetch from memory in milliseconds instead of hitting the database repeatedly.
- **Database replication and sharding**: moved from a single MySQL instance to replicated read replicas and started sharding by user ID and subreddit.
- **Front-end optimization**: moved from monolithic Python app to lighter request handling; separated static file serving to CDNs.

By the 2010s, Reddit's architecture was sophisticated enough to handle millions of concurrent visitors during major events (elections, celebrity AMAs, Game of Thrones finales). The hug of death became not a threat to Reddit itself, but a threat to *sites linked from Reddit*—a sign that Reddit had achieved robust distributed architecture.

## The Digg Migration (2010) and Subreddit Architecture

In August 2010, Digg launched "Digg v4"—a major redesign meant to increase publisher control and ad inventory. The changes were catastrophic:

- Removed features users loved (bury button, favorites, friends submissions, history search)
- Changed the algorithm so publishers' content promoted over user-curated content
- The site was unstable and unreachable for weeks
- Digg v4 also migrated from MySQL to Cassandra, trying to solve scaling problems at the same time it was redesigning the product—a compounding disaster

The reaction was swift: power users and communities fled to Reddit, which had a simpler design, user-friendly moderation, and an openness to community governance. This was the "Great Digg Migration" of August–September 2010. In weeks, Reddit's traffic exploded as Digg users, accustomed to deep hierarchies of topical subreddits similar to Digg's digging communities, found Reddit's subreddit model actually offered *better* community control.

**Subreddit architecture** proved to be Reddit's killer feature: each subreddit is a self-contained community with its own moderation team, rules, and culture. This is governance through locality—communities police themselves. Compare to Digg's centralized moderation and top-down algorithm. Reddit scaled governance as much as code: moderators are volunteer unpaid labor that keep hundreds of thousands of communities running. The lesson: **platform architecture reflects power distribution**. Digg consolidated power; Reddit distributed it.

Subreddits also enabled content sharding: traffic to r/AskReddit doesn't cache-block traffic to r/programming. Moderation rules, sticky posts, and theme customization live per-subreddit. This meant Reddit could grow communities organically without re-architecting the core platform.

## Moderation as Governance

Reddit's early growth was chaos—spam, harassment, illegal content. The platform's response was novel: **empower volunteer moderators**. Each subreddit has a moderation team that can ban users, remove posts, set rules. This is a form of distributed governance that scales better than centralized content policy.

This design choice had massive consequences:
- Scaling moderation cost: Reddit didn't need to hire thousands of moderators, it crowdsourced the work.
- Community autonomy: subreddits could vary wildly in culture and rules—r/science is heavily moderated; r/TrueOffMyChest is minimal.
- Decentralized power: if you disliked a subreddit's rules, you could create a competing one (e.g., r/drama vs. r/subredditdrama, r/atheism vs. r/TrueAtheism).

This created a marketplace of communities. But it also meant Reddit's centralized platform had to support hostile subreddits, harassment campaigns, and illegal content—problems that plague it to this day.

## Distributed Coordination at Scale

Behind the scenes, Reddit had to solve several distributed systems problems:

- **Cache invalidation**: when a post is upvoted, the ranking changes, so all cached versions are stale. Reddit uses a write-through cache pattern: update the database, invalidate the cache entry.
- **Vote counting**: a single post can get millions of votes. Vote counts are updated asynchronously; stale counts are acceptable because users tolerate eventually-consistent vote tallies.
- **Real-time updates**: as of the 2010s, Reddit used a mix of polling (JavaScript refreshing vote counts every few seconds) and push updates (WebSockets for live streams like the place experiment).

The place experiment (2017) was a stress test: a 1000×1000 pixel canvas where any logged-in user could place one colored pixel every 5 minutes. It ran for 72 hours and drew millions of participants. The peak was tens of thousands of concurrent users updating pixels in real-time. Reddit's infrastructure held because they'd built redundant systems that could shed load: if the pixel placement service fell behind, the front-end just showed stale pixels until they caught up.

## Key Insights

- **Hug of death is infrastructure debt collection**: it exposes assumptions (single-threaded services, file descriptor limits, shared resource pools) that worked at previous scale but fail at new scale.
- **Governance architecture (subreddits + volunteer moderators) is as important as technical architecture** for platform growth.
- **Distributed systems are about accepting eventual consistency**. Reddit's vote counts lag, comment scores have stale reads, but users tolerate this because the alternative (locking for consistency) makes the system slow or unavailable.
- **Scaling is not just vertical**: Reddit didn't just buy bigger machines; it sharded data, moved computation to caches and edge layers, and offloaded governance to volunteers.