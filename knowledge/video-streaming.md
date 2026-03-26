# Video Streaming — Codecs, Adaptive Bitrate, DRM, Latency, CDN

## Video Codecs

### Uncompressed and Intermediate

**Raw pixel data**: **RGB** (typically) or **YUV** (luma + chroma). 1920×1080 @ 30fps RGB:
- 1920 × 1080 × 3 bytes/pixel × 30 fps = **≈3 Gbps**. Impractical for storage/streaming.

**Intermediate/mezzanine codecs** (used in production pipelines):
- **ProRes** (Apple): High quality, lower compression (8–12:1). Used in editing workflows.
- **DNxHD** (Avid): Similar to ProRes.

Not used for distribution.

### H.264 (AVC, MPEG-4 Part 10)

**H.264** is the dominant codec for streaming (2003–present).

**Compression**: ~50–200:1 depending on quality target. 1920×1080 @ 30fps can stream at 2–10 Mbps for good quality.

**Encoding efficiency**: Interprets video as blocks; exploits spatial (intra-frame) and temporal (inter-frame) redundancy. Key concepts:
- **I-frame** (intra): Fully encoded, independent. Large.
- **P-frame** (predictive): Encodes differences from prior I/P-frame. Smaller.
- **B-frame** (bi-predictive): Encodes differences from both prior and future frames. Smallest, improved compression.

**Latency**: Encoding time depends on preset (fast → realtime possible; slow → days for mastering). Typical live encoding: ~1–2 seconds delay.

**Universal support**: Supported everywhere (browsers, devices, older TV). De facto standard for streaming.

**Licensing**: Patent pool (MPEG LA). Royalty-free for end-users streaming video, but encoder/decoder manufacturers may owe royalties.

**Profile/Level**: H.264 defines profiles (baseline, main, high) and levels (30, 31, 40, etc. indicating max resolution/bitrate). Most streaming uses Main or High profile at Level 4.1 (up to 1920×1080 @ 60fps).

### H.265/HEVC (High Efficiency Video Coding, MPEG-H Part 2)

**H.265** succeeds H.264 (~2013+).

**Efficiency**: ~2× compression vs. H.264 at same quality (or half the bitrate). 1080p good quality: ~2–5 Mbps (vs. 4–8 Mbps H.264).

**Trade-offs**:
- Encoding is slower (2–10× CPU vs. H.264 depending on preset).
- Decoding faster than H.264 (lower latency for decoder).
- Browser support: Mixed. Chrome/Edge on Windows requires hardware decoder; Safari (iOS 13+) supports; Firefox via extension. Not universal.

**Licensing**: HEVC Advance, Qualcomm, and others hold patents. Licensing uncertain; manufacturers hesitant. Adoption slower than H.264 due to IP ambiguity.

**Use**: 4K streaming (Netflix, Apple TV 4K), mobile video, bandwidth-constrained scenarios. Not recommended for universal web distribution yet.

### VP9 (Google)

**VP9** is Google's open (royalty-free) codec.

**Efficiency**: Similar to H.265 (~2× H.264).

**Advantages**:
- Open source, no patent licensing issues.
- Royalty-free.
- Good browser support (Chrome, Firefox, Edge). Safari: none.

**Disadvantages**:
- Slower encoding than H.264.
- Hardware acceleration limited (less common on devices).
- Not widely supported for playback on older/non-PC devices.

**Use**: YouTube (prefers VP9 for signed-in users; falls back to H.264), WebRTC, open-source streaming.

### AV1 (MPEG-LA)

**AV1** is the next-gen codec (2018+), backed by an alliance (Google, Mozilla, Cisco, Amazon, etc.).

**Efficiency**: ~1.5× better than HEVC (3× H.264). Bitrates for 1080p good quality: 1–3 Mbps.

**Trade-offs**:
- Extremely slow encoding (10–50× H.264). Impractical for live-streaming; mastering only.
- Decoding power similar to H.265.
- Hardware acceleration emerging but not mainstream yet.
- Browser support: Chrome 90+, Firefox 116+, Safari 16+. Good but not universal for older browsers.

**Patents**: Open, royalty-free (MPEG LA commitment).

**Use**: Future-proofing, ultra-bitrate-constrained scenarios (satellite, low-bandwidth), archival. Not yet practical for realtime live streaming.

### Codec Selection for Streaming

**Universal compatibility**: H.264 (boring but safe).

**Modern web + good mobile support**: H.264 primary, VP9 secondary (YouTube model).

**Bandwidth-optimized** (mobile, cellular): HEVC (if targeting modern devices) + H.264 fallback.

**Open-source/no licensing**: VP9, AV1.

**4K/UHD**: H.265 or AV1 (H.264 struggles to achieve good 4K quality at reasonable bitrates).

## Adaptive Bitrate Streaming (ABR)

### Concept

**Adaptive Bitrate Streaming** detects client bandwidth and adjusts video quality in real-time. Avoids buffering, maximizes quality within network constraints.

### HLS (HTTP Live Streaming)

**Format**: Apple's standard (.m3u8 playlist, MPEG-TS or MP4 segments).

**Manifest** (`.m3u8`):
```
#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:10
#EXTINF:10.0, 0
segment-0-a.ts
segment-1-a.ts
segment-2-a.ts
#EXT-X-ENDLIST
```

**Variant playlists** (for ABR):
```
#EXTM3U
#EXT-X-STREAM-INF:BANDWIDTH=1500000
480p/playlist.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=4000000
720p/playlist.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=8000000
1080p/playlist.m3u8
```

**Client logic**: Monitor network throughput, select appropriate variant, download segments sequentially.

**Latency**: ~6–30 seconds typical (segment duration ~10s × 2–3 buffered segments).

**Advantage**: Widely supported (iOS native, most Android devices, web via HLS.js library).

### DASH (Dynamic Adaptive Streaming over HTTP)

**Format**: MPEG standard (`.mpd` manifest, MP4 segments).

**Manifest** (simplified):
```xml
<MPD>
  <Period>
    <AdaptationSet>
      <Representation bandwidth="2000000" width="1280" height="720">
        <BaseURL>video-720p.mp4</BaseURL>
      </Representation>
      <Representation bandwidth="5000000" width="1920" height="1080">
        <BaseURL>video-1080p.mp4</BaseURL>
      </Representation>
    </AdaptationSet>
  </Period>
</MPD>
```

**Advantages over HLS**:
- Standard, vendor-agnostic.
- Finer control (per-segment quality, not per-playlist).
- Better support for multiple codecs, languages, subtitles.

**Disadvantage**: Requires DASH player (Shaka, dash.js). Less native support than HLS.

### Comparison

| Aspect       | HLS              | DASH                          |
|--------------|------------------|-------------------------------|
| Origin       | Apple            | MPEG (standardized)           |
| Segment type | MPEG-TS, MP4     | MP4                           |
| Latency      | 6–30s            | 3–30s (configurable)          |
| Codec support| H.264, H.265     | H.264, H.265, VP9, AV1        |
| Adoption     | Ubiquitous (iOS) | Growing (Netflix, YouTube)    |
| Web support  | HLS.js library   | dash.js, Shaka, etc.          |

**Current practice**: Distribute both HLS (compatibility) and DASH (flexibility).

## Transcoding and Encoding

### Transcoding Pipeline

**Typical workflow**:
1. **Ingest**: Upload source video (ProRes, DNxHD, or H.264 at high bitrate).
2. **Transcode**: Encode to multiple bitrates and codecs (e.g., H.264 @ 5Mbps, 2Mbps, 500kbps + VP9 @ 3Mbps).
3. **Segment**: Split into chunks (~10s segments).
4. **Manifest**: Generate HLS/DASH manifests and variant lists.
5. **Distribute**: Upload to CDN, replicate globally.
6. **Consume**: Client fetches manifest, adaptively downloads segments.

### Preset Strategies

**Preset** = encoding speed vs. quality trade-off. Typical ladder (Netflix, YouTube):

- **360p**: 500 kbps (mobile, low-bandwidth)
- **480p**: 1–1.5 Mbps
- **720p**: 2–4 Mbps (good quality, standard)
- **1080p**: 5–8 Mbps
- **2160p (4K)**: 15–25 Mbps (or ~8 Mbps with HEVC)

Each bitrate encoded once, distributed globally. Segments stored per-bitrate.

### Encoding Tools

**FFmpeg**: Open-source, industry standard. Example:
```bash
ffmpeg -i input.mov -c:v libx264 -crf 23 -preset slow -b:v 2500k output.mp4
```

**x264/x265**: Fast, efficient H.264/H.265 encoders (libx264, libx265).

**VP9 (libvpx-vp9), AV1 (libaom)**: Open-source, slower.

**Commercial**: AWS MediaConvert, Google Transcoder API, Mux, Brightcove. Abstracts complexity, scales.

**Real-time**: OBS, GStreamer for live encoding (H.264/VP8 typically).

## Digital Rights Management (DRM)

### Concepts

**DRM** protects video content from unauthorized copying/distribution. Browser support via **EME** (Encrypted Media Extensions).

### Width of Protection

**Clear key**: Minimal protection; key transmitted in manifest (development only).

**Commercial DRM**:
- **PlayReady** (Microsoft): Windows, Xbox, Edge.
- **Widevine** (Google): Chrome, Android, smart TVs.
- **FairPlay** (Apple): iOS, macOS, Safari.

**Usage**: Netflix, Disney+, Hulu use one or more (typically multiple for cross-platform).

### How It Works (Encrypted Media Extensions)

1. **Client** requests content (encrypted video + manifest with key URI).
2. **Client** fetches license from license server (authorization: user subscription, geographic, device fingerprint).
3. **License server** returns decryption key (ephemeral, time-limited).
4. **Browser EME** decrypts video in hardware or software, decodes, renders to protected output (HDCP on HDMI, etc.).
5. **Content never leaves protected buffer**; can't be screen-captured or exported.

### Trade-offs

**Pros**:
- Prevents casual copying.
- Enables subscription/rental models.

**Cons**:
- Complex development (per-platform licensing servers).
- Licensing cost: ~$0.10–$1 per user (varies by provider).
- Performance overhead: encryption/decryption.
- Browser fragmentation: different DRM per platform.

### Web Implementation

```javascript
const config = [{
  initDataTypes: ['cenc'],
  videoCapabilities: [{
    contentType: 'video/mp4;codecs="avc1.4d4015"',
    robustness: 'SW_SECURE_CRYPTO'
  }]
}];

navigator.requestMediaKeySystemAccess('com.widevine.alpha', config)
  .then(access => access.createMediaKeys())
  .then(keys => video.setMediaKeys(keys));
  
// Fetch encrypted segments, license key, play
```

**Practical**: Use a streaming platform (Mux, AWS Elemental, Brighcove) that abstracts DRM complexity.

## WebRTC for Live Streaming

### Protocol

**WebRTC** (Real-Time Communication) provides peer-to-peer or B2M (browser-to-media-server) live streaming via **SFU** (Selective Forwarding Unit) or **MCU** (Multipoint Control Unit).

**Codec**: Opus (audio), VP8/VP9/H.264 (video, varies by implementation).

**Latency**: 100–500ms (sub-second typical for local networks). Significantly lower than HLS/DASH (~10s+).

### Challenges for Mass Broadcast

- **Per-connection overhead**: Each viewer needs a connection to server (or peers in mesh). 1000 viewers = 1000 TCP/UDP flows.
- **Ingress bandwidth**: Server receives one stream, sends N copies (scales linearly with viewer count). CDN ingestion difficult.
- **Scalability**: Traditional CDN doesn't apply; requires SFU farm with load balancing.

**Use cases**: 1:1 calls, small group meetings, low-viewer-count live events. Not suitable for millions of viewers (YouTube Live modelbetter).

**Solutions**: Mux, whereby.com, Google Cloud Media CDN provide broadcast SFUs.

## Latency and LL-HLS

### Latency Sources

1. **Encoding**: 0.5–5 seconds (depends on GOP size; H.264 typically 2–3 seconds).
2. **Packaging**: 1–2 seconds (segment finalization, manifest update).
3. **Network**: 0.1–2 seconds (CDN, client download).
4. **Client buffering**: 3–10 seconds (safety buffer to prevent stalls).

**Total**: HLS ~10–30 seconds. DASH can be configured lower (4–8 seconds).

### Low-Latency HLS (LL-HLS)

**LL-HLS** reduces latency to 2–4 seconds via:

1. **Partial segments**: Deliver segment fragments before completion. Client starts playback mid-segment.
2. **Preloading hints**: Manifest signals next segment location, allowing client to request before its availability.
3. **Rate-based delivery**: Server throttles segment delivery to match playback rate (no buffering ahead).

**Trade-off**: Lower latency but higher packet loss sensitivity; requires tuned client buffer and redundancy.

**Adoption**: Industry moving toward LL-HLS for sports, live events. Widevine broadcast protocol (WebRTC gateway) competes.

## CDN Delivery

### Architecture

**Origin server**: Stores master copy (segments, manifests).

**Cache nodes**: Distributed globally. Client requests routed to nearest node via:
- **GeoDNS**: DNS nameserver returns IP closest to client (by geolocation).
- **Anycast**: All cache nodes share same IP; network routing directs client to nearest.

**Cache hierarchy**: Cache nodes may cache from other cache nodes (never fetching origin directly).

### Edge Computing

**Serverless CDN**: Execute custom logic at cache node (transformation, authorization, origin fallback). Examples: Cloudflare Workers, AWS Lambda@Edge.

**Topology optimization**: Segment optimization, codec selection, bitrate tuning at the edge based on real-time network conditions.

## Video Streaming Architecture

**Complete pipeline**:
1. **Acquisition**: Camera/screen capture.
2. **Encoding**: Produce H.264 + backup codec, multiple bitrates.
3. **Packaging**: Segment, create manifests (HLS/DASH).
4. **DRM**: License server, key distribution (if subscription).
5. **CDN**: Cache globally.
6. **Client**: Player fetches manifest, adaptively downloads, decrypts (if applicable), renders.

**Player** (open-source libraries): HLS.js, Shaka Player (DASH), Video.js (abstraction layer).

## See Also

- **audio-fundamentals.md** — Audio codecs, Web Audio.
- **networking-protocols.md** — HTTP/2, TCP, UDP.
- **performance-web-vitals.md** — Video impact on LCP, CLS.
- **security-tls.md** — HTTPS for secure delivery.