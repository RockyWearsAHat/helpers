# Multimodal AI — Vision, Language, Audio & Generation

## Vision-Language Models: Connecting Images and Text

Multimodal models process both images and text, enabling tasks like visual question answering, image captioning, and reasoning over visual content. The dominant architecture chains a **vision encoder** to a **language model** via a **projection layer**.

### Architecture: CLIP → Projector → LLM

**CLIP (OpenAI)** is the standard pretraining base: a vision transformer trained on 400M internet image-text pairs with contrastive learning (same image and caption pull together; different pairs push apart). CLIP learns aligned image-text embeddings but outputs only fixed-size vectors, not detailed visual features.

**LLaVA** (Large Language-and-Vision Assistant) exemplifies the pattern:

1. Freeze a pretrained CLIP ViT vision encoder (e.g., ViT-L/14) to extract image features
2. Pass image patches through a simple linear or MLP projection to match LLM embedding dimension
3. Insert projected image features into the token stream before text input
4. Fine-tune the LLM and projection layer on instruction-following data (e.g., "What is in this image?")

The projection layer is critical: it admits where vision and language spaces differ. A learnable projection allows retraining for specific domains (medical images, satellite imagery) by replacing just the projection and LLM, keeping the frozen vision encoder.

**Trade-offs**:
- Frozen CLIP encoders ensure generalization (transfer-friendly) but may miss domain-specific visual patterns
- Fine-tuning the vision encoder (unfrozen) improves accuracy ~5-10% but costs more compute and risks overfitting
- Resolution: CLIP typically processes images at 224×224; higher resolutions require architectural modifications

### GPT-4V and Proprietary Approaches

Commercial models (GPT-4 Vision, Claude with Vision, Gemini Vision) use proprietary vision encoders and techniques. Key known differences:
- **Variable resolution**: Handle images from 320×240 to 2048×2048, adapting encoding strategy
- **Document/chart understanding**: Specialized training on OCR and structured data
- **Length limitation**: Gated by token costs, typically 4-8 images per request

Fine-tuning these models is often unavailable; adaptation usually goes through prompting or RAG (retrieving relevant visual examples).

## Diffusion Models for Image Generation

Diffusion models reverse the process of noising an image. Start with pure noise; iteratively denoise over hundreds of steps, guided by a text prompt.

### Forward & Reverse Diffusion

The **forward process** gradually adds Gaussian noise: $x_t = \sqrt{ᾱ_t} x_0 + \sqrt{1 - ᾱ_t} ε$ where $ᾱ_t$ is a schedule decreasing from 1 to ~0. After many steps, $x_T$ is nearly pure noise and loses all image information.

The **reverse process** learns to predict the noise at each step: a neural network trained on pairs $(x_t, t, \text{prompt})$ predicts the noise added at step $t$. Fine-tuned versions adjust for prompt guidance, and at inference, iteratively applying the denoiser produces a coherent image.

### Text Conditioning and Guidance

Early diffusion (DDPM) ignored text; DALL-E and Stable Diffusion add guidance:

- **CLIP guidance** (DALL-E 2): Use a CLIP encoder during denoising to increase image-prompt alignment
- **Token embeddings** (Stable Diffusion): Embed the text prompt and condition the denoiser on embeddings. Simpler and faster than CLIP guidance.
- **Classifier-free guidance**: Train the model both with and without prompt conditioning. At inference, use a weighted combination of conditional and unconditional predictions to control adherence to the prompt (higher weight = stricter adherence).

| Approach | Speed | Quality | Controllability |
|----------|-------|---------|-----------------|
| Unconditional generation | Fast | Varied | None |
| Text embedding guidance | Fast | Good | Moderate |
| CLIP guidance | Slow (extra CLIP forward) | Excellent | High |
| Classifier-free guidance | Medium | Excellent | High |

### ControlNet and Structural Control

Recent extensions (ControlNet, Adapter) inject spatial control: edge maps, poses, depth maps. These allow users to specify *where* content should appear, not just *what*. The architecture learns to condition the diffusion process on structural inputs, enabling precise layouts.

## Audio Models: Whisper & Text-to-Speech

**Whisper** (OpenAI) is an automatic speech recognition (ASR) model trained on 680K hours of multilingual audio from the web. It predicts both transcriptions and translations, learning from diverse audio conditions (background noise, accents, music, technical jargon).

Architecture: Encoder-decoder transformer. The encoder is a 24-layer transformer processing mel-frequency cepstral coefficient (MFCC) features; the decoder generates tokens autoregressively. Trained with a contrastive loss encouraging consistency across languages.

Strengths: Robust to accent and noise; handles 99 languages; fast (real-time on consumer CPU). Limitations: Hallucination on silent audio; struggles with rare languages; limited to transcription (no speaker diarization).

**Text-to-Speech (TTS)** models (Vall-E, Natural Language Generation-based systems) convert text to audio waveforms. Modern approaches:

- Encode text into high-level features (prosody, phonetics)
- Generate acoustic features (mel-spectrograms)
- Convert spectrograms to raw audio via a vocoder (e.g., HiFi-GAN)

Key challenge: naturalness and speaker consistency—TTS must match speaker identity and emotional tone. Zero-shot TTS (generate novel voices) is an open problem.

## Video Generation: Temporal Consistency

Video generation extends image diffusion to sequences of frames. The challenge is **temporal consistency**: frames must cohere spatially and align with physics (objects don't teleport).

**Sora** (OpenAI) generates videos up to 1 minute long. Key technique: apply diffusion over both spatial (image patches) and temporal (frame indices) dimensions. Attention spans both space and time, enabling the model to learn object persistence.

| Challenge | Approach |
|-----------|----------|
| Temporal flicker | Multi-frame diffusion; attention across frames |
| Long-range motion | Autoregressive frame generation (predict next 16 frames, condition on previous 16) |
| Consistency with text | CLIP guidance or prompt embedding conditioning |
| Computational cost | Latent-space diffusion (like Stable Diffusion); work in compressed space |

Video generation is computationally expensive—single-frame generation is ~1 second on a GPU; full video generation requires optimization via latent diffusion and selective refinement.

## Cross-modal Attention & Contrastive Learning

**Cross-modal attention** lets models apply attention from one modality to another. In vision-language models, the text decoder's attention mechanism can attend to image frature s, enabling fine-grained visual grounding.

**Contrastive learning** (used in CLIP, multimodal embeddings) trains via pairs of matching and non-matching items. For each image-caption pair: increase similarity of image and its caption; decrease similarity of image and random captions. This self-supervised approach enables learning without explicit labels and creates aligned embedding spaces where similar content clusters together.

Advanced contrastive methods incorporate hard negatives (challenging but relevant pairs) to avoid trivial solutions. The quality of negative sampling significantly impacts downstream performance.

## Practical Deployment Patterns

**Vision-language in production**: Typically use smaller frozen encoders (MobileViT, CLIP-base) with quantization (4-8 bit). Batch multiple images per inference pass to amortize projection costs.

**Diffusion-based generation**: Substantial latency (10-30s for high-quality images). Production systems often use queuing + asynchronous generation; client polls for results. Smaller models (latent diffusion, distilled versions) reduce latency at some quality cost.

**Audio multimodal**: Combine speech recognition with text understanding. Transcribe audio, then apply NLP; faster and more accurate than direct audio-to-meaning without intermediate text.

**Video**: Still mostly research; commercial deployment rare. When needed, prefer shorter durations (<10s) and assume 1-2 minute generation time.

See also: **Large language model architecture**, **Machine learning operations**, **Neural network concepts**