# Media Processing Pipelines — Image, Video, Transcoding & Delivery

## Overview

Media processing pipelines transform user-uploaded images and videos into optimized, viewable assets. A pipeline ingests raw media, performs quality checks, generates multiple sizes/formats, applies effects, extracts metadata, and distributes to CDN. Pipelines must balance **latency** (how fast users see results), **cost** (CPU, storage, bandwidth), and **quality** (visual fidelity, format compatibility).

## Core Operations

### Image Processing

**Input validation:** Check file size, format (JPEG, PNG, WebP, AVIF) and dimensions before processing. Reject oversized uploads (>100 MB per image).

**Resizing & thumbnails:** Generate multiple sizes for responsive delivery:
- Thumbnail: 128×128 (social feeds, galleries)
- Medium: 512×512 (detail pages)
- Large: 1920×1080 (high-res displays)
- Retina: 2x variants for high-DPI screens

Store as separate assets (not srcset in HTML), enabling selective invalidation and CDN-edge optimization.

**Format selection:** Choose format based on target device and network:
- **JPEG** — Lossy, compact, broad compatibility. Good for photographs
- **PNG** — Lossless, larger, supports transparency. Good for graphics, icons
- **WebP** — Lossy/lossless hybrid, ~25% smaller than JPEG, supported on modern browsers since ~2018
- **AVIF** — Newer codec, 20-30% smaller than WebP, but limited legacy support

Serve multiple formats using HTML5 `<picture>` element with fallbacks.

**Cropping & aspect ratios:** Some UIs require specific aspect ratios (3:4 portrait, 16:9 widescreen). Intelligently crop or letterbox to fit; for faces/portraits, use face detection to preserve eyes.

**Optimization / compression:** Reduce file size via quality degradation. A JPEG at 80% quality is often indistinguishable from 100% but 40% smaller. PNGQUANT reduces PNG size via palette quantization.

**Metadata extraction & stripping:** Read EXIF (embedded camera/GPS data). Stripping EXIF reduces file size and protects privacy; preserving it enables photo organization by date/location. Use ImageMagick `identify`, Sharp `.metadata()`, or Pillow `.info`.

**Effects:** Apply filters if needed (blur, grayscale, sepia). Blur is common for preview placeholders while full image loads.

### Video Processing

**Ingestion:** Upload large files to object storage (S3, GCS) directly; server tracks status and triggers processing when file arrives.

**Transcoding:** Convert video to multiple bitrates and codecs for adaptive playback:

| Codec | Bitrate | Use Case |
| --- | --- | --- |
| H.264 | 500-1000 kbps | Mobile, baseline |
| H.265 / HEVC | 300-600 kbps | Modern devices, cloud |
| VP9 | 400-800 kbps | YouTube-compatible |
| AV1 | 200-400 kbps | Emerging, slow encoding |

Each bitrate targets a network condition (2G → 500 kbps, 4G → 2 Mbps, WiFi → 5 Mbps).

**Container format:** Wrap codec in container (MP4 for H.264, WebM for VP9/AV1). Include keyframes every 2-5 seconds for scrubbing (seeking).

**Audio codec:** Same video, multiple audio tracks (stereo, 5.1 surround). AAC or Opus for audio.

**HLS / DASH:** Segment video into 2-10 second chunks. HLS (HTTP Live Streaming) uses .m3u8 playlists; DASH uses .mpd manifests. Player fetches chunks adaptively: if bandwidth drops, switch to lower bitrate.

**Processing throughput:** Transcoding is CPU-intensive (H.264 at 60 FPS can consume 2 cores per stream). Use queues (SQS, Celery) to parallelize; process multiple videos on multiple machines.

**Cost:** If a 1-hour video transcodes to 8 bitrates × 2 audio tracks = 16 outputs at ~$5 per output = $80 per video. Pre-encode popular formats (H.264, H.265) and lazy-encode niche codecs on-demand.

## Thumbnail Generation

Thumbnails serve video previews and require **fast generation** and **high quality**. Strategies:

**Keyframe extraction:** Extract a frame 25% into the video (not black intro frame, not white outro). Use FFmpeg: `ffmpeg -i video.mp4 -ss 00:00:10 thumb.jpg`.

**Multi-frame filmstrip:** Generate 6 keyframes at regular intervals. On hover, show filmstrip to preview video scrubbing.

**AI-based:** Detect scenes with people/faces and prioritize those frames. More accurate but slower (requires ML inference).

**Blur placeholder:** Generate a 10×10 JPEG of the thumbnail, then blur it. Display blurred version while full thumbnail loads (LQIP — Low Quality Image Placeholder). Improves perceived loading speed.

## Content Moderation

Media must comply with platform policy: no nudity, violence, hate speech, spam. Two approaches:

**Automatic scanning:**
- **Computer vision:** AWS Rekognition, Google Vision, Clarifai detect objects, text, explicit content. Returns confidence scores; flag borderline cases for human review.
- **OCR + NLP:** Extract text from images (license plates, signs), then apply text moderation (hate speech filters).
- **Hashing:** Store hashes of known illegal content (NCMEC database). Compare new uploads to hash repo; instant rejection if match found.

**Human review:** Show flagged images to moderation team; they approve or remove. Expensive (~$2 per image) but accurate. Use automatic scoring to triage: highest confidence rejections go to simple "remove" queue, low-confidence ambiguous cases go to experienced moderators.

**User reporting:** Allow users to report inappropriate content. Track complaint volume; automatic action after 5+ reports from distinct users.

## CDN Delivery

Raw media files stored in origin (S3, GCS). CDN caches at edge locations worldwide.

**Cache busting:** If image changes, old CDN cache must refresh. Strategies:

- **Versioned URLs:** Use content hash in URL: `/images/hash_abc123.png`. Changes hash → new URL → no cache collision
- **Cache-Control headers:** Set `max-age=31536000` for immutable assets (versioned URLs), `max-age=3600` for mutable (profile picture)
- **Purge API:** CDN provider API (Cloudflare, Fastly) allows on-demand cache invalidation

**Compression:** CDN applies gzip/brotli compression in-transit (reduces JPEG 50%, PNG 40%). Real-time, transparent to origin.

**Geographic routing:** Route user to nearest edge (latency-based). Use Geo-IP or BGP Anycast.

**Bandwidth costs:** Egress from cloud storage is expensive (~$0.02 per GB). CDN caching reduces origin egress 10-100×, saving $100K+ annually for large platforms.

## Metadata Extraction

Extract structured data from media for indexing and discovery:

**Image metadata:**
- EXIF: Camera model, GPS, timestamp, lens, ISO, aperture
- IPTC: Keywords, copyright, description
- XMP: Color profile, dimensions

**Video metadata:**
- Duration, resolution, codec, bitrate, framerate, color profile
- Audio: Sample rate, channel count, codec
- Text tracks: Subtitles, captions, language

**Optical Character Recognition (OCR):** Extract visible text from images; enables search within images ("show images with text 'hello'").

**What / who:** Computer vision models detect objects (cat, car, mountain) and people. Enables image tagging ("this photo has a sunset") and search ("find photos with sunsets").

Use `exiftool`, ImageMagick `identify`, FFprobe (for video), or cloud APIs (Google Vision, AWS Rekognition).

## Typical Architecture

**Async pipeline:**
1. User uploads image/video to pre-signed S3 URL
2. S3 event triggers Lambda / Cloud Function
3. Process spawns workers: resize images, transcode video, extract metadata, run moderation
4. Store outputs to S3, update database with asset metadata
5. CDN caches outputs
6. User notified (webhook, polling) when ready

**Tools:**

| Operation | Tools | Notes |
| --- | --- | --- |
| Image resize/crop | Sharp (Node.js), Pillow (Python), ImageMagick | Sharp is fastest for Node.js |
| Image optimization | PNGQUANT, guetzli, jpegoptim | Post-processing |
| Video transcode | FFmpeg, HandBrake, AWS MediaConvert | FFmpeg most flexible |
| Metadata extraction | exiftool, FFprobe, Sharp `.metadata()` | Always strip EXIF before CDN for privacy |
| Moderation | AWS Rekognition, Google Vision, Clarifai | Hybrid: auto + human |
| CDN | Cloudflare, Fastly, AWS CloudFront | Cache invalidation strategy critical |

## Performance & Cost Trade-offs

**Synchronous encoding:** Wait for transcode to complete before showing upload success. Simple, but laggy (transcode = 5-60 seconds).

**Asynchronous encoding:** Upload succeeds immediately; transcode happens in background. User sees placeholder; progress updates via polling/WebSocket.

**On-demand encoding:** Don't pre-encode all formats. Encode video to H.264 by default; if user requests VP9, encode on-first-request and cache. Saves 70% of transcode costs but first request of new format is slow.

**Cost per video:** Basic transcode (H.264 + H.265) = ~$10. Full range (8 bitrates) = ~$80. Selective encoding (H.264 only for slow networks, H.265 for fast) = ~$15.

**Storage:** 1TB of media costs ~$25/month. Archive old videos (>1 year unused) to Glacier ($5/month, 12-hour retrieval).

## See Also

- [web-image-optimization.md](web-image-optimization.md) — Responsive image delivery, formats, lazy loading
- [algorithms-compression.md](algorithms-compression.md) — Compression algorithms behind JPEG, WebP, AVIF
- [performance-web-vitals.md](performance-web-vitals.md) — Impact of media optimization on page speed
- [scaling-load-balancing.md](scaling-load-balancing.md) — Load balancing video transcoding workers
- [architecture-data-pipeline.md](architecture-data-pipeline.md) — General pipeline architecture patterns