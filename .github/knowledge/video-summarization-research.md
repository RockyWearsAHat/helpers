# Video Summarization Research — State of the Art (2024-2025)

Research compiled for designing the gsh-vision-tool efficient video analysis pipeline.

## Core Problem

Video produces vastly more tokens than LLMs can consume. A 1-hour video at 30fps = 108,000 frames. MLLMs can only process 32-64 frames. The field is about choosing which 0.05%-1.8% of frames to keep.

## Key Systems

### AKS — Adaptive Keyframe Sampling (CVPR 2025, Tang et al.)
- Balance **relevance** (frame↔query CLIP/BLIP similarity) with **coverage** (recursive bin partitioning)
- Recursive judge-and-split: compare `s_top - s_all` against threshold → take top-K or split bin
- +3.8% accuracy over uniform for LLaVA-Video-7B. 7B + AKS beats 72B without AKS
- Even 0.25fps pre-filtering still beats uniform sampling
- GitHub: https://github.com/ncTimTang/AKS

### Focus — Frame-Optimistic Confidence Upper-bound Selection (NUS/TikTok, 2025)
- Multi-armed bandit formulation: clips as arms, Bernstein confidence bounds
- Two-stage: coarse exploration → fine exploitation using UCB
- +11.9% on long videos (>20 min), processing only 1.6% of frames (vs AKS 3.7%)
- Key: temporal autocorrelation ~5s half-life — nearby frames nearly identical in relevance
- GitHub: https://github.com/NUS-HPC-AI-Lab/FOCUS

### PRISM — Label-Guided Summarization (arxiv, Jan 2026)
- Stage 1: ResNet-18 embeddings + PELT change-point detection
- Stage 2: VLM labels → LLM filters hallucinated ones → CLIP semantic anchoring (0.9 threshold)
- Stage 3: 4-frame temporal windows → per-window summaries → recursive tree-merge
- <5% frames, 84% BERTScore, +33% METEOR. Label-guided > motion-based filtering alone

### Adaptive Keyframe Sampling — Production (dev.to, Romitelli)
- Score (frame-diff-energy) → Segment (hysteresis hot/cold) → Allocate (floor+cap budget) → Pick (evenly spaced)
- 60 budget-allocated frames beat 300 uniform frames
- Stride 2-5 for screen recordings. Hysteresis prevents segment flicker

### Multimodal Video RAG (KX Systems, 2025)
- ~30s scenes (frames + transcript) → single multimodal embedding (Voyage AI) → vector DB search
- Image sprites (6 frames merged horizontally) dramatically reduce VLM token costs
- "A frame showing a graph means nothing without the speaker's explanation"

## Universal Principles

1. **Uniform sampling always loses** — every system beats it, especially on long videos
2. **Temporal locality** — 5s autocorrelation half-life, exhaustive scoring is wasteful
3. **Transcript + vision together >> either alone**
4. **Budget allocation with floors/caps** — prevents blind spots and over-concentration
5. **Two-tier scoring** — cheap pass (CLIP/frame-diff) filters, expensive pass (VLM) analyzes
6. **Query-awareness** — knowing what you're looking for improves selection dramatically
7. **Scene detection is a cheap win** — ffmpeg scene detection is ~free vs VLM calls

## Architecture for gsh-vision-tool Pipeline

### Phase 0: Free Intelligence (existing)
- Local Whisper transcription (working, ~24s for 12-min video)
- ffmpeg scene detection threshold 0.3 (working, zero cost)

### Phase 1: Transcript-Guided Classification (new, ~free)
- Parse transcript into timed segments
- Classify: talking-head, code/slides, demonstration, transition
- Cross-reference scene-change density with transcript content

### Phase 2: Smart Budget Allocation
- Proportional to info density, with floors (min 1/segment) and caps (max 40%)
- Talking-head: 0-1 frames. Code/slides: high budget, high resolution
- Hysteresis segmentation to prevent over-fragmentation

### Phase 3: Two-Tier Vision Analysis
- Tier 1 (cheap): Classify frames via sprite merging (4-6 frames → 1 image)
- Tier 2 (expensive): Deep analyze only information-rich frames
- Follow PRISM's label validation approach

### Phase 4: Synthesis
- 4-frame temporal windows → per-window summaries
- Recursive tree-merge for final report
- Interleave transcript with visual observations

Expected: 60-80% token reduction with potentially increased accuracy.