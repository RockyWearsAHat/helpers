# IoT Edge AI — TinyML, Quantization, NPU, Federated Learning

## Overview

Edge AI runs inference locally on IoT devices, eliminating cloud latency, bandwidth, privacy concerns, and connectivity dependencies. Trade-off: devices have kilobytes of RAM and single-digit MIPS; cloud servers have gigabytes and GPUs. Edge AI compresses models through quantization, pruning, distillation, and custom hardware. The field convergence is around TensorFlow Lite Micro (TFLMicro), ONNX Runtime, and specialized accelerators (NPU, VPU).

## TinyML — Embedded ML on Microcontrollers

TinyML runs ML inference on devices with 32 MB RAM or less, typically ARM Cortex-M4/M7. Applications: wake-word detection, fall detection, anomaly detection on sensor streams, gesture recognition.

### TensorFlow Lite for Microcontrollers (TFLMicro)

**Design**: Lightweight interpreter for quantized TensorFlow Lite models. No operating system, no OS heap allocator required; uses static memory pools. Footprint: ~200 KB with core operations. Supports integer-only operators (no floats on inference).

**Workflow**:
1. Train neural net (Python + full TensorFlow).
2. Convert to TensorFlow Lite FlatBuffer format (quantized, 8-bit INT8 or even 1-bit binary/ternary).
3. Embed as C array in firmware via `xxd -i model.tflite`.
4. Call interpreter on device; feed input tensors, invoke, read output tensors.

**Operator Support**: Limited to ops implementable efficiently on embedded CPUs. Conv2D, DepthwiseConv2D, FullyConnected, Softmax, Pooling. No dynamic shapes; all tensors pre-allocated at convert time.

**No-malloc Requirement**: Critical. TFLMicro pre-allocates all intermediate buffers at initialization. Inference is deterministic; no allocation failures, no allocation latency.

**Example**: Wake-word detection on Cortex-M4 runs in ~100 ms using ~4 MB RAM (including model + activations). Detection threshold: 90 dB input, 100 ms process, <100 mA.

### Alternative: microTVM (Apache TVM)

microTVM compiles natively to machine code (not interpreter overhead). Generates C code from computational graph; compiler optimizes for target hardware (ARM, RISC-V, x86). Performance: ~2–5x faster than interpreted bytecode but compile time is higher; model size overhead to include compiled code.

**Tradeoffs**: Interpreted (TFLMicro) = smaller binary, less RAM, slower. Compiled (microTVM) = larger binary, faster, more memory for code.

## ONNX Runtime — Open Standard Format

ONNX (Open Neural Network Exchange) is a portable format for models. ONNX Runtime is a standardized inference engine supporting multiple hardware targets (CPU, GPU, NPU, VPU, FPGA).

**Portability**: Train in PyTorch, export to ONNX, run on ONNX Runtime on any device. Decouples training framework from inference platform.

**Edge Variant — ONNX Runtime Lite**: Subset for embedded; removes features needed for cloud (multi-GPU, distributed), keeps core inference. Smaller, faster than full Runtime.

**Quantization in ONNX**: Tools convert float32 models to INT8 offline. Runtime loads quantized model; inference is integer-only. Dequantization (INT8 → float) happens only at output if needed for application logic.

## Model Quantization — Trading Precision for Size and Speed

Quantization reduces model size and inference latency by lowering numeric precision. Trade-off: accuracy loss (typically 0.5–2% for well-designed models).

### INT8 Quantization (8-bit integers)

**Approach**: Map float32 weights and activations to INT8 range [-128, 127]. Scaling factor $s$ converts: `INT8 = round(float32 / s)`. Inference uses only integer arithmetic (no floating-point unit needed).

**Impact**:
- Model size: ~75% reduction (4 bytes → 1 byte per parameter).
- Inference speed: 3–4x faster on CPU (native int8 support).
- RAM: Activations reduced proportionally, but temporary buffers still required.
- Accuracy: Usually <0.5% loss after retraining (post-training quantization) or fine-tuning (quantization-aware training).

**Calibration**: Quantization schemes require calibration data (representative inputs) to choose scaling factors. Poor calibration → large accuracy loss.

### INT4 Quantization (4-bit)

Further compression: 16x smaller than float32. Used in edge AI when model size is critical. Accuracy loss increases (~2–5% for some models). Emerging standard for LLM edges (e.g., GGUF format, llama.cpp on phones).

### Binary / Ternary Quantization

Extreme case: 1-bit (binary) or 2-bit (ternary) weights. Up to 32x compression. Accuracy drops significantly (~5–10%); suitable for low-precision tasks (binary face detection, coarse classification). Rarely used outside specialized research.

### Quantization-Aware Training (QAT)

Fine-tune model during training to mimic quantization. Network learns to be robust to precision loss. Better accuracy than post-training quantization alone. Standard practice in production edge AI.

## Neural Processing Units (NPU) and Vision Processing Units (VPU)

Specialized silicon accelerates edge inference beyond CPU.

### NPU (Neural Processing Unit)

Dedicated fixed-function hardware for matrix operations (key in convolutions, matrix multiply). Examples: Qualcomm Hexagon NPU (Snapdragon), Apple Neural Engine, MediaTek Helio X/G series. Offloads computation from CPU; can achieve 10–100x speedup on convolutions at lower power than CPU.

**Integration**: Accessed via vendor frameworks (Qualcomm SNPE, TensorFlow Lite NNAPI delegate, Core ML on Apple). Model quantization often required for hardware acceleration.

**Power**: NPU draws less power per operation than CPU; total power depends on data movement to/from main RAM.

### VPU (Vision Processing Unit)

Similar to NPU but optimized for image processing (filters, transforms, down-sampling). Often combined with NPU on SoC. Intel Movidius VPU is a standalone accelerator for computer vision on edge.

**Contrast with GPU**: GPUs excel at high-throughput; most edge devices (phones, embedded) have weak or no GPU. NPU/VPU are more efficient for low-power, real-time inference.

## Pruning — Removing Redundancy

Neural networks are often over-parameterized. Pruning removes weights (or neurons) with minimal impact on accuracy.

### Structured Pruning

Remove entire channels or filters. Reduces both model size and latency (no holes in compute). Unstructured pruning (random weights) reduces size but doesn't help latency without custom operators.

**Example**: ResNet50 pruned 60% (remove 60% of filters) loses <1% accuracy. Model 2.5x smaller, 2–3x faster on edge hardware accelerators.

### Magnitude-Based Pruning

Remove weights with near-zero magnitude. Simple but crude; better methods use importance scores or gradients.

**Lottery Ticket Hypothesis**: Networks contain sparse subnetworks that match full-network accuracy. Pruning during training finds these subnetworks. Results in smaller, faster models without accuracy loss.

## Knowledge Distillation — Teacher → Student

Train a small student network to mimic a large teacher network. Student learns teacher's intermediate representations, not just final outputs. Enables models 10–100x smaller with minimal accuracy loss.

**Process**:
1. Train large teacher model (e.g., ResNet50).
2. Use teacher to label data with soft targets (probability distributions, not just class indices).
3. Train small student (e.g., MobileNet) on soft targets + hard targets.
4. Deploy small student on edge.

**Benefit**: Student generalizes better than naive training from scratch; inherits teacher's learned feature hierarchy.

**Example**: Student MobileNetV2 (4 MB quantized) with distillation achieves 92% accuracy; same architecture without distillation: 87%.

## Federated Learning on Devices

Train models collaboratively across devices without sending raw data to cloud. Each device trains locally, sends only model updates (gradients); server aggregates to improve global model.

### Use Case

Predict keyboard next-word on phone keystrokes. Training data is sensitive; sending it to cloud violates privacy. Federated learning: phone trains on *its* keystrokes, sends only weight updates; server aggregates updates from thousands of phones.

### Process

1. Server initializes model, sends to 1000 phones.
2. Each phone trains locally for N steps on user data.
3. Phone sends weight deltas (delta W) to server.
4. Server averages deltas from all phones, updates global model.
5. Repeat.

### Challenges

**Non-IID Data**: Each device has different data distribution (user typing style differs). Non-IID hurts convergence; more communication rounds needed.

**Communication Cost**: Sending gradients (even sparse) is expensive on cellular. Compression techniques (quantize gradients, prune) reduce bandwidth.

**Device Dropout**: Phone loses connectivity or shuts off during training. Aggregation must be robust to stragglers (use async aggregation or timeouts).

### Privacy

Gradients alone leak information (differential privacy analysis). Adding noise to gradients before sending improves privacy but degrades learning.

## Power-Aware Inference

Edge inference is constrained by battery or thermal budgets.

### Techniques

**Early Exit**: At inference, if confidence high enough (in first layers), skip remaining layers. Trade-off: latency vs accuracy. Useful for real-time tasks (e.g., face detection).

**Dynamic Quantization**: Automatically lower precision if power threshold hit. Degrade gracefully.

**Scheduling**: Run inference during low-power periods (e.g., when charging). Batch multiple requests.

**Hardware Sleep**: Accelerators (NPU, GPU) consume idle power; turn off when not in use.

**Approximate Computing**: Accept ~1% accuracy loss in exchange for 30% power reduction. Useful in non-critical inference (ad ranking, content recommendations).

## Hardware Examples

| Device | RAM | CPU | NPU | Use Case |
|--------|-----|-----|-----|----------|
| STM32 Cortex-M4 | 192 KB | 160 MHz | No | Wake-word, small CNNs |
| Nordic nRF52840 | 256 KB | 64 MHz | No | BLE + simple inference |
| Qualcomm Snapdragon (mobile) | 6–12 GB | Kryo cores | Hexagon | Phone AI (face, speech) |
| Raspberry Pi 4 | 4–8 GB | ARM Cortex-A72 | No (external USB) | Local object detection |
| Google Coral EdgeTPU | — | — | Yes (TPU) | 4 TOPS sustained, 15W |

## Emerging Trends

**ONNX Growth**: ONNX Runtime adoption accelerating; becoming default format for edge models. Better portability than TensorFlow Lite.

**Quantization Ubiquity**: INT4 and binary weights becoming standard as model compression improves.

**On-Device Fine-Tuning**: Not just inference; fine-tuning on device (federated learning). Requires gradient computation in TFLMicro.

**Neuromorphic Hardware**: Event-based processing (SpikeNNs) consumes less power than synchronous inference for sparse events.

## Key Insight

Edge AI is a systems problem, not just an ML problem. Model architecture, quantization scheme, hardware accelerator, power budget, thermal limits, and real-time deadlines all interlock. A floating-point model that works in cloud may be 20x too slow on edge; distilled + quantized + compiled with NPU support becomes feasible. Design from device constraints backward, not from ideal models.