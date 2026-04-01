# Audio Fundamentals — Sampling, Bit Depth, Codecs, Web Audio API

## Sampling and Quantization

### Nyquist Theorem and Sampling Rates

**Sampling** converts continuous analog audio (sound waves) into discrete digital samples.

**Nyquist Theorem**: To accurately represent a signal containing frequencies up to `f_max`, the sampling rate must be at least `2 × f_max`.

- **Human hearing**: ~20 Hz to 20 kHz. Nyquist rate: 40 kHz minimum.
- **CD audio**: 44.1 kHz sampling. Captures up to 22.05 kHz.
- **Professional audio**: 48 kHz or 96 kHz (for higher frequency headroom).
- **DAT**, **DVD-A**: 96 kHz.
- **Ultrasonic frequencies**: 192 kHz sampling captures up to 96 kHz.

**Undersampling**: Below Nyquist rate introduces **aliasing** — high frequencies fold back into audible range, creating artifacts (digital distortion). Prevented via low-pass filtering before sampling.

**Common rates in practice**:
- Web/streaming: 44.1 kHz, 48 kHz.
- Telephony: 8 kHz (voiceband barely adequate).
- Voice conferencing: 16 kHz (bandwidth-constrained but intelligible).

### Bit Depth and Quantization Noise

**Bit depth** is the resolution of each sample, quantizing amplitude to discrete levels.

- **8-bit**: 256 levels. 48 dB dynamic range. Audibly degraded for music; acceptable for telephony.
- **16-bit**: 65,536 levels. ~96 dB dynamic range. CD standard. Imperceptible quantization noise.
- **24-bit**: ~140 dB dynamic range. Professional recording, studio mastering. Overkill for playback (human hearing ~120 dB), useful for editing headroom.
- **32-bit float**: Beyond 16-bit accuracy; used in digital audio workstations for internal processing to avoid rounding errors.

**Quantization noise**: The error from rounding to discrete levels. In 16-bit, worst-case ~96 dB down from peak signal — inaudible at normal listening levels.

**Dither**: Adds low-amplitude noise before quantizing, spreading quantization error across frequencies, reducing perceived distortion. Used when reducing bit depth (e.g., 24-bit → 16-bit mastering). Web audio typically doesn't apply dither; browsers handle natively.

## Audio Codecs

### Uncompressed: PCM

**PCM (Pulse Code Modulation)**: Raw samples, no compression. Sampling rate × bit depth × channels = bitrate.

- **CD**: 44.1 kHz, 16-bit, stereo = 44,100 × 16 × 2 = 1,411 kbps ≈ 176 KB/s.
- **File format**: WAV (container), AIFF (Apple).

**Use**: Studio recording, lossless storage when compression isn't critical. Inefficient for streaming.

### Lossless Compression

**FLAC (Free Lossless Audio Codec)**:
- Compression: ~40–50% of uncompressed size.
- Metadata support: Tags, seekable (can seek to any frame without decoding full file).
- Web support: Minimal (requires JavaScript decoder).
- Use: Archival, high-fidelity distributions (e.g., Bandcamp, Hydroplanet).

**Lossless Codecs** (general):
- **Alac** (Apple Lossless): Proprietary, similar compression to FLAC. Default on Apple devices.
- **WMA Lossless**: Windows Media. Rarely used now.

### Lossy Compression

**MP3 (MPEG-1 Layer III)**:
- Compression: ~10–12:1 (128 kbps typical, from 1,411 kbps PCM).
- Psychoacoustics: Removes frequencies human ear can't hear (especially in presence of loud frequencies — **masking**).
- Patents: No longer encumbered (expired ~2017). De facto standard for backward compatibility.
- Quality: Transparent at 192 kbps+. Audible artifacts at 128 kbps under scrutiny.
- Web support: Universal (all browsers).

**AAC (Advanced Audio Coding, MPEG-4 Part 3)**:
- Compression: ~20% better than MP3 at same bitrate.
- Quality: 128 kbps AAC ≈ 192 kbps MP3.
- Patents: Encumbered (MPEG Licensing Administration), but royalty-free for streaming/devices.
- Web support: Universal.
- Use: Streaming (Apple Music, YouTube, Netflix preset), mobile devices.

**Opus (RFC 6716)**:
- Compression: Best-in-class for variable bitrate. 6 kbps (speech) to 510 kbps (hi-fi stereo) with transparent quality.
- Latency: Low latency mode (designed for real-time communication, VoIP).
- Adaptive: Real-time bitrate adaptation to network.
- Patents: Open, royalty-free.
- Web support: ~80% of browsers (not Safari as of 2025).
- Use: WebRTC, real-time communication, modern streaming where supported.

**Vorbis**:
- Similar quality to AAC, slightly worse compression than Opus.
- Open, royalty-free.
- Web support: Limited (Safari unsupported).
- Use: Niche; superseded by Opus.

### Codec Selection

**Streaming music**: AAC (universal) or Opus (if targeting modern browsers).

**Podcasts/voice**: AAC or Opus (lower bitrates acceptable: 64–96 kbps).

**Real-time communication**: Opus (mandatory for WebRTC best practice).

**Archival**: FLAC (lossless, future-proof).

**Legacy/compatibility**: MP3 (still plays everywhere).

## Web Audio API

### Basics

The **Web Audio API** provides JavaScript interfaces for audio synthesis, analysis, and playback.

```javascript
const audioContext = new (window.AudioContext || window.webkitAudioContext)();
const source = audioContext.createMediaElementAudioSource(audioElement);
const analyser = audioContext.createAnalyser();

source.connect(analyser);
analyser.connect(audioContext.destination);

// Play audio through analyser
audioElement.play();
```

**AudioContext**: Singleton (per tab) managing audio processing graph.

**Nodes**: Sources (microphone, file), effects (filters, delays), and destination (speakers).

**Graph**: DAG of connected nodes. Audio flows: source → effects → destination.

### Common Nodes and Effects

**Gain**: Volume control.
```javascript
const gain = audioContext.createGain();
gain.gain.value = 0.5; // 50% volume
source.connect(gain);
```

**BiquadFilter**: Low-pass, high-pass, band-pass, peaking filters.
```javascript
const filter = audioContext.createBiquadFilter();
filter.type = 'lowpass';
filter.frequency.value = 1000; // 1 kHz
source.connect(filter);
```

**Convolver**: Convolution (impulse response). Used for reverb, realistic room acoustics.
```javascript
const convolver = audioContext.createConvolver();
convolver.buffer = impulseResponseBuffer; // Pre-recorded IR
source.connect(convolver);
```

**Delay**: Echo, delay lines.
```javascript
const delay = audioContext.createDelay(5);
delay.delayTime.value = 0.5; // 500ms delay
source.connect(delay);
```

**Analyser**: Frequency or time-domain analysis. Enables visualizations.

### Audio Visualization

**Analyser** provides real-time spectrum or waveform data:

```javascript
const analyser = audioContext.createAnalyser();
analyser.fftSize = 2048; // FFT resolution
const frequencyData = new Uint8Array(analyser.frequencyBinCount);

function draw() {
  requestAnimationFrame(draw);
  analyser.getByteFrequencyData(frequencyData);
  
  // frequencyData: 0–255 per frequency bin
  for (let i = 0; i < frequencyData.length; i++) {
    // Draw bar for bin i with height frequencyData[i]
  }
}
```

**Frequency visualization**: Bar graphs, spectrograms. Most common.

**Waveform visualization**: Time-domain display (`getByteTimeDomainData`). Less CPU-intensive but simpler feedback.

### Real-Time Audio Processing

**ScriptProcessorNode** (deprecated but still works):
```javascript
const processor = audioContext.createScriptProcessor(4096, 1, 1);
processor.onaudioprocess = (event) => {
  const inputData = event.inputBuffer.getChannelData(0);
  const outputData = event.outputBuffer.getChannelData(0);
  
  for (let i = 0; i < inputData.length; i++) {
    outputData[i] = inputData[i] * 0.5; // Volume reduction
  }
};
source.connect(processor);
processor.connect(audioContext.destination);
```

**AudioWorklet** (modern replacement):
```javascript
// Register custom worklet
await audioContext.audioWorklet.addModule('my-worklet.js');

const worklet = new AudioWorkletNode(audioContext, 'my-processor');
source.connect(worklet);
worklet.connect(audioContext.destination);
```

**Worklet is class-based**, lower latency, runs in separate thread. Use for serious real-time audio.

## Spatial Audio

### Binaural Rendering

**Spatial audio** places sound in 3D space. Binaural rendering uses **HRTF** (Head-Related Transfer Functions) — phase/amplitude filters unique to head shape — to create illusion of direction.

**PannerNode**:
```javascript
const panner = audioContext.createPanner();
panner.setPosition(1, 0, 0); // 1 meter to the right
panner.setOrientation(0, 0, -1); // Listener facing forward
source.connect(panner);
panner.connect(audioContext.destination);
```

**Parameters**:
- **Position**: (x, y, z) in 3D space relative to listener.
- **Orientation**: Forward direction vector for listener.
- **Rolloff factor**: How quickly sound attenuates with distance.

### Ambisonics and Spatial Formats

**Ambisonics**: Format encoding directional audio. First-order (4 channels), higher orders (16+) for finer resolution. Advantages:
- Format-agnostic: Same ambisonics file renderable on any speaker setup.
- Scalable: More channels = better directional resolution.

**Use cases**: VR video, 360° content, immersive experiences. Web Audio supports via custom nodes or libraries (resonance-audio, etc.).

## Audio Streaming and Protocols

### HTTP Streaming

**Progressive download**: Standard `<audio src="...">`. Buffers as downloads. Simple but no bitrate adaptation on bandwidth changes.

**DASH (Dynamic Adaptive Streaming over HTTP)**:
- Server offers multiple bitrate variants (480p video + variants at 800k, 2.5M, 5M audio).
- Manifest (MPD file) lists segments.
- Client measures bandwidth, selects appropriate bitrate in real-time.
- Minimizes buffering, adapts to connection quality.

**HLS (HTTP Live Streaming)**: Apple's variant. `.m3u8` playlist, similar adaptive logic. Standard on iOS.

### Real-Time Protocols

**WebRTC**: Peer-to-peer audio/video. Uses Opus codec by default. Low latency (100–300ms typical). Endpoints: negotiate ICE candidates, establish DTLS connection, exchange audio via RTP.

**RTMP** (Real-Time Messaging Protocol): Older streaming protocol. Latency 2–10 seconds. Less common now; HLS/DASH preferred.

### Low-Latency Streaming

**LL-HLS** (Low-Latency HLS): Partial segments, chunk delivery. Reduces latency from ~10 seconds to ~3–5 seconds.

**WebRTC**: Sub-second latency, but requires direct peer connectivity. Impractical for broadcast (1000+ viewers).

**SRT** (Secure Reliable Transport): Newer protocol for low-latency ingest (camera → cloud). Not directly supported by Web Audio but used in streaming pipelines.

## Audio Processing Considerations

### Latency

**Algorithmic latency**: Built into audio effects (convolver adds 50–2000ms depending on IR length).

**Hardware latency**: Device and driver dependent. Low-latency audio requires:
- Small buffer sizes (128–512 samples).
- Dedicated audio driver (ASIO on Windows, CoreAudio on macOS, ALSA on Linux).
- Web Audio latency: ~100–200ms typical (browser buffer overhead).

### Artifacts and Quality

**Clipping**: Output exceeds ±1.0 (normalized) or audio range. Produces digital distortion.

**Aliasing**: High-frequency content folds back with undersampling.

**Quantization noise**: Inherent to bit-depth (inaudible in practice at 16-bit+).

**Phase issues**: Poorly designed filters or mixing introduce phase distortions, audible as comb filtering or phase cancellation.

## Audio Subsystems in Web Development

**Contexts**:
1. **Background music/ambience**: HTTP streaming (DASH/HLS), low latency requirements.
2. **Real-time communication**: WebRTC, Opus codec, VoiceActivityDetection (VAD).
3. **Synthesis/effects**: Web Audio API, oscillators, filters, custom worklets.
4. **Analysis**: FFT (frequency analysis), spectrum visualization.

**Browser compatibility**: Web Audio API widely supported. Opus codec less so (no Safari). AAC universal.

## See Also

- **math-signal-processing.md** — FFT, digital filtering theory.
- **web-event-loop.md** — AudioContext blocking considerations.
- **web-workers.md** — AudioWorklet threading.
- **web-service-workers.md** — Audio streaming caching.