# Signal Processing — Mathematical Fundamentals

## Overview

Signal processing is the transformation and analysis of signals (time-varying or space-varying quantities) to extract information or modify them for specific objectives. The mathematical foundations underpin audio processing, image analysis, communications, radar, seismic data interpretation, and countless domain-specific applications.

## The Fourier Transform

### Continuous and Discrete Fourier Analysis

The **Fourier transform** decomposes a signal into sinusoidal components at different frequencies. For a continuous signal $x(t)$, the Fourier transform is:
$$X(f) = \int_{-\infty}^{\infty} x(t) e^{-j2\pi ft} dt$$

where $f$ is frequency in Hz. The inverse transform recovers $x(t)$ from $X(f)$.

Key insight: **Any periodic or quasi-periodic signal can be expressed as a sum of sine and cosine waves.** The transform reveals which frequencies are present and their magnitudes (via $|X(f)|$) and phases.

For discrete signals $x[n]$ (sampled time series), the **Discrete Fourier Transform** (DFT) is:
$$X[k] = \sum_{n=0}^{N-1} x[n] e^{-j2\pi kn/N}$$

for $k = 0, 1, ..., N-1$.

### Practical Interpretation in the Frequency Domain

When working with real signals, the magnitude spectrum $|X(f)|$ shows the "strength" of each frequency component. The phase spectrum $\angle X(f)$ indicates the time offset of each sinusoid.

Example: A pure 1 kHz sine wave sampled at 44.1 kHz produces a DFT with a single peak at the 1 kHz bin (and its mirror image due to real-signal symmetry).

Advantage of frequency-domain analysis:
- Filtering designs are intuitive (suppress certain frequencies, preserve others)
- Convolution (time-domain multiplication) becomes pointwise multiplication (frequency-domain), enabling efficient computation
- Spectral characteristics reveal signal quality, noise, harmonics, and anomalies

## The Sampling Theorem (Nyquist-Shannon)

### The Fundamental Connection Between Continuous and Discrete

The **Nyquist-Shannon sampling theorem** states: a bandlimited signal (containing no frequency components above frequency $f_{\max}$) is completely determined by samples taken at intervals $T_s = 1/(2f_{\max})$.

In other words, **sampling rate must be at least $2f_{\max}$** to avoid information loss.

The Nyquist frequency is $f_N = 1/(2T_s)$: the highest frequency that can be represented in sampled data.

### Aliasing

If sampling rate is below the Nyquist rate, **aliasing** occurs: high-frequency components fold back into low-frequency components, corrupting the signal.

Example: A 5 kHz sine sampled at 8 kHz (Nyquist rate = 4 kHz) appears as a 3 kHz component (`5 - 8 = -3`, reflected as 3 kHz). Recovery becomes impossible: the original frequency information is lost.

In practice:
- **Anti-aliasing filter**: low-pass filter applied before sampling to remove frequencies above Nyquist
- **Reconstruction filter**: applied after digital-to-analog conversion to smooth the staircase approximation

Failure to apply anti-aliasing (or choosing too-low sampling rate) is a primary source of misleading experimental data in measurement systems.

## Fast Fourier Transform (FFT)

### Computational Efficiency

Computing the DFT naively requires $O(N^2)$ operations (matrix-vector product). The **Fast Fourier Transform** (FFT), introduced by Cooley and Tukey, reduces this to $O(N \log N)$.

For $N = 10^6$ samples:
- Naive DFT: ~$10^{12}$ operations (infeasible in real-time)
- FFT: ~$2 \times 10^7$ operations (practical on modern hardware)

The FFT exploits **divide-and-conquer**: splitting the computation into even and odd-indexed samples, recursively transforming each half, then combining with twiddle factors (phase adjustments).

This speedup is why frequency-domain signal processing became practical; without FFT, audio processing and communications systems would be orders of magnitude slower.

### Implementation Notes

Common butterflies include:
- Radix-2 FFT (most common)
- Radix-4, mixed-radix FFTs (optimized for specific hardware)
- In-place computation (overwrites input with output)
- Bit-reversal permutation (reordering input for in-place computation)

## Convolution and Filtering

### Convolution in Time and Frequency

**Convolution** of signals $x(t)$ and $h(t)$ is:
$$y(t) = (x * h)(t) = \int_{-\infty}^{\infty} x(\tau) h(t - \tau) d\tau$$

For discrete signals, the discrete convolution is:
$$y[n] = \sum_{m=0}^{\infty} x[m] h[n - m]$$

The critical property: **Convolution in time is multiplication in frequency:**
$$Y(f) = X(f) \cdot H(f)$$

Therefore, **filtering (convolving with an impulse response) is implemented efficiently in the frequency domain** as pointwise multiplication, especially for long signals. The FFT enables this "fast convolution."

### Finite Impulse Response (FIR) Filters

An **FIR filter** has impulse response $h[n]$ that is zero for $n < 0$ and $n \geq M$ (finite duration). The output is:
$$y[n] = \sum_{m=0}^{M-1} b_m x[n - m]$$

Properties:
- Always stable (bounded input → bounded output)
- Can be designed with linear phase (all frequencies delayed equally), preserving signal shape
- Implementation: straightforward convolution or FFT-based fast convolution
- Drawback: requires many coefficients for sharp frequency response

Example: Moving average is an FIR filter; averaging 100 consecutive samples provides low-pass filtering.

### Infinite Impulse Response (IIR) Filters

An **IIR filter** has infinite duration impulse response; the output depends on past outputs:
$$y[n] = \sum_{m=0}^{M} b_m x[n - m] - \sum_{m=1}^{N} a_m y[n - m]$$

Properties:
- Can achieve sharp cutoff with fewer coefficients than FIR
- **Feedback loop**: can become unstable if coefficients are improperly chosen
- **Phase distortion**: typically non-linear phase (not all frequencies delayed equally)
- Implementation: difference equation directly (poles and zeros define behavior)

Trade-off: IIR filters are efficient but require stability analysis; FIR filters are stable but less efficient.

## Windowing

### The Spectral Leakage Problem

When the signal duration doesn't align with the DFT window length, **spectral leakage** occurs: a single frequency "leaks" energy across multiple bins, contaminating adjacent frequency estimates.

Applying a **window function** (tapering the signal to zero at edges) reduces leakage but at the cost of wider main lobe and reduced frequency resolution.

Common windows:
- **Rectangular** (no tapering): narrowest main lobe, worst sidelobe suppression (~13 dB)
- **Hamming**: good balance (~43 dB sidelobe suppression)
- **Hann (Hanning)**: similar to Hamming, slightly different coefficients
- **Blackman**: stronger sidelobe suppression (~58 dB) but wider main lobe
- **Kaiser**: parameterizable; adjusts trade-off between main lobe and sidelobe

Window selection depends on whether the goal is **frequency resolution** (narrow main lobe) or **spurious component suppression** (strong sidelobe attenuation).

## Spectrogram and Time-Frequency Analysis

### The Spectrogram

A **spectrogram** is a time-frequency representation: applying windowed FFTs to successive segments of the signal and stacking the magnitude spectra.

Process:
1. Divide signal into overlapping windows (e.g., 50% overlap)
2. Apply window function (e.g., Hamming)
3. Compute FFT of each windowed segment
4. Display magnitude (often log scale) vs. time and frequency

Interpretation: bright regions indicate strong time-localized frequency content. Example: a chirp (frequency sweeping over time) forms a visible diagonal path in the spectrogram.

### Time-Frequency Resolution Trade-off

Short windows → **good time resolution, poor frequency resolution** (uncertain which frequency is present, but know exactly when)

Long windows → **poor time resolution, good frequency resolution** (know the frequency precisely, but not when it occurred)

This is fundamentally a Heisenberg uncertainty principle: $\Delta t \cdot \Delta f \geq 1/(4\pi)$ (with specific window functions closer to this bound).

**Constant-Q analysis** (e.g., wavelets) provides a fixed ratio $Q = f_c / \Delta f$, giving better relative resolution at high frequencies and better time resolution at low frequencies.

## Wavelets

### Wavelet Transform vs. Fourier

The **continuous wavelet transform** (CWT) is:
$$W(a, b) = \int_{-\infty}^{\infty} x(t) \psi^*\left(\frac{t - b}{a}\right) \frac{dt}{a}$$

where $\psi(t)$ is a wavelet (localized oscillation), $a$ is scale (inverse frequency), and $b$ is time translation.

Key difference from STFT (windowed Fourier):
- STFT uses fixed window width at all frequencies
- CWT uses windows that scale with frequency: narrower for high frequencies (good time resolution), wider for low frequencies (good frequency resolution)

This **constant-Q property** makes wavelets superior for signals with events at different frequency scales (e.g., music, geophysical data).

### Discrete Wavelet Transform (DWT)

The **DWT** uses dyadic scales ($a = 2^j$) and dyadic translations, enabling hierarchical decomposition:
$$x[n] = \sum_k c_J[k] \phi(2^{-J}n - k) + \sum_{j=0}^{J-1} \sum_k d_j[k] \psi(2^{-j}n - k)$$

where $c_J$ are coarse approximation coefficients and $d_j$ are detail coefficients at each scale.

Implementation is fast via filter banks: downsampling and upsampling with low-pass ($H$) and high-pass ($G$) filters. No FFT needed.

Applications:
- **JPEG2000**: compression via wavelet decomposition
- **Denoising**: soft-thresholding wavelet coefficients (noise is spread across all scales, signals concentrate)
- **Feature detection**: local maxima in wavelet coefficients mark sharp transitions

## Digital Signal Processing Applications

### Audio Signal Processing

Typical pipeline:
1. **Recording**: microphone (analog) → ADC (sample at 44.1 kHz or higher)
2. **Filtering**: remove DC offsets, suppress 50/60 Hz hum
3. **Feature extraction**: FFT for spectrum, MFCC (Mel-frequency cepstral coefficients) for speech recognition
4. **Effects**: reverb (convolution with impulse response), compression (dynamic range reduction)
5. **Playback**: DAC (digital-to-analog) → speaker

### Image Processing

Images are 2D signals; operations extend naturally:
- **2D convolution**: blurs, edge detection (Sobel, Canny operators)
- **2D FFT**: frequency domain filtering, compression
- **Wavelets**: multiresolution decomposition

## Key Principles

**Causality and Realizability**: A filter must not access future samples. In real-time systems, FIR filters are inherently causal. IIR filters can be non-causal if not properly designed.

**Numerical precision**: Repeated floating-point operations accumulate rounding errors. Fixed-point arithmetic (common in embedded systems) requires careful scaling to avoid overflow and maintain sensitivity.

**Latency**: Real-time systems (audio, communications) cannot afford large delays. FFT-based fast convolution introduces algorithmic latency (buffer accumulation); FIR filters offer lower latency at higher complexity.

See also: [math-fourier-analysis](math-fourier-analysis.md), [machine-learning-signal-features](machine-learning-signal-features.md), [audio-processing-basics](audio-processing-basics.md)