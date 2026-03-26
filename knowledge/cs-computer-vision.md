# Computer Vision — Convolution, Detection, Segmentation, and Transfer Learning

Computer vision enables machines to extract and reason about visual information from images and video. Core tasks progressed from fixed feature extraction (edges, corners) to learned feature hierarchies (CNNs) to end-to-end semantic understanding.

## Convolution & CNNs (Convolutional Neural Networks)

### Convolution Operation

Applies a small learnable filter (kernel) across an image to extract local features.

```
Input Image (5×5):          Filter (3×3):
1 2 3 4 5                    0.1  0.2  0.1
6 7 8 9 10                   0.2  0.4  0.2
11 12 13 14 15               0.1  0.2  0.1
16 17 18 19 20
21 22 23 24 25

Convolution at position (0,0):
1*0.1 + 2*0.2 + 3*0.1 + 6*0.2 + 7*0.4 + 8*0.2 + 11*0.1 + 12*0.2 + 13*0.1 = 5.2

Output (3×3) after sliding filter across entire image
```

**Hyperparameters**:
- **Kernel size**: Typically 3×3 (local features), 1×1 (mixing channels), 5×5 (broader context)
- **Stride**: Step distance (default 1; stride=2 halves spatial dimensions)
- **Padding**: Add borders (default 0; "same" padding preserves dimensions)
- **Dilation**: Skip pixels in filter (increases receptive field without adding parameters)

**Interpretation**: Each filter learns to detect a feature (edges, textures, shapes). Early layers detect low-level features; deep layers detect high-level semantic features.

### Pooling Layers

Downsample spatial dimensions, reducing computation and introducing shift invariance.

- **Max pooling**: $\max(2×2)$ neighborhood (preserves strong activations)
- **Average pooling**: Mean of neighborhood (smoother)

```
Input (4×4):          Max Pooling (2×2):
5 3  2 1              5  2
7 1  4 8              7  8
2 6  4 1
8 2  1 3
```

### CNN Architectures

#### LeNet (1990s)
Pioneer architecture: Conv → Pool → Conv → Pool → FC → FC (classification).

#### AlexNet (2012)
Deep CNN that won ImageNet competition. Introduced:
- ReLU activation (vs. sigmoid, tanh)
- GPU acceleration (training feasible)
- Dropout (regularization for overfitting)
- Data augmentation

8 layers, 60M parameters. ImageNet top-1 error: 15.3% (vs. 26% prior baseline).

#### VGGNet (2014)
Very deep (16-19 layers), all 3×3 convolutions. Showed: **depth matters for representation quality**.

```
Block: Conv(3×3) → Conv(3×3) → MaxPool(2×2)
(repeated with increasing channels)
```

144M parameters (large memory footprint, slower training).

#### ResNet (Residual Networks, 2015)

Added **skip connections** (residual blocks) enabling training very deep networks (50-152 layers).

```
y = F(x) + x   (instead of y = F(x))
```

Without residuals, gradients vanish in deep networks (backpropagation dies). Skip connections provide gradient highway.

**ResNet-50**: 50 layers, 25M parameters, faster than VGG despite depth. Became standard backbone.

#### EfficientNet (2019)

Systematically scales depth, width, and resolution jointly using **compound scaling**.

```
depth multiplier: d^φ
width multiplier: w^φ
resolution multiplier: r^φ
(φ determined by grid search)
```

For same accuracy as ResNet-50 with 4× fewer parameters.

Trade-off: Minimal latency for inference (critical for mobile/edge).

## Image Classification Pipeline

```
Raw Image
  ↓ [Data Augmentation: Crop, Flip, Rotate, Color Jitter]
  ↓ [Normalization: Subtract mean, divide by std]
Input Tensor (224×224×3 for ResNet)
  ↓ [CNN Backbone: Learned Feature Extraction]
Feature Maps (progressively lower resolution, higher semantics)
  ↓ [Global Average Pooling or Flatten]
Feature Vector (e.g., 2048-dim for ResNet-50)
  ↓ [Fully Connected Layers]
Logits (num_classes dimensions)
  ↓ [Softmax]
Class Probabilities (sum to 1)
```

### Transfer Learning

Train on large dataset (ImageNet: 1.2M images, 1000 classes), then fine-tune on target task.

**Rationale**: ImageNet pre-training learns universal features (edges, textures, objects). Target task (e.g., medical imaging, satellite imagery) benefits from learned hierarchy.

**Two strategies**:
1. **Feature extraction**: Freeze backbone, train only final FC layer (fast, less data needed)
2. **Fine-tuning**: Unfreeze backbone, train end-to-end with lower learning rate (higher accuracy, more data/compute needed)

**Observation**: For similar domains, early layers transfer well (low-level features universal). For distant domains (e.g., satellite→medical), intermediate layers often transfer best.

## Object Detection

Localize and classify multiple objects in an image: bounding boxes + class labels.

### YOLO (You Only Look Once)

Single-stage detector: divide image into grid, predict bboxes + class probabilities per cell.

```
Input: 416×416 image
Grid: 13×13 cells
Per cell: 5 predictions per anchor = (x, y, w, h, confidence) + class_probs
Output: 13×13×(5×5+num_classes)
```

**Speed**: Real-time (~60 FPS). Trade-off: struggles with small objects, multiple objects per cell.

**Loss function**: Combine localization loss (smooth L1 on bbox), objectness loss (IoU-based), classification loss (cross-entropy).

### Faster R-CNN (Region-based CNN)

Two-stage detector: propose candidate regions (RPN) → classify & refine.

```
Image → Backbone CNN → Feature Maps
                    ↓ (two parallel heads)
        Regional Proposal Network → Candidate Boxes
        Classifier & Box Predictor → Refined Boxes + Labels
```

**Region Proposal Network (RPN)**:
- Slide anchor boxes (multiple sizes/aspects) across feature map
- Score each anchor (object/no-object)
- Refine anchor positions via regression

**Strengths**: More accurate than YOLO, handles small objects better (two stages allow focus).

**Weakness**: Slower (not real-time), more complex training.

### mAP (mean Average Precision)

Standard metric for detection evaluation.

1. For each class, compute precision-recall curve
2. Average precision = area under PR curve
3. Mean AP = average over all classes
4. Often report AP@IoU=0.5 and AP@IoU=0.5:0.95 (strict threshold)

## Semantic & Instance Segmentation

### Semantic Segmentation

Classify every pixel (no instance boundaries).

```
Input: Image
Output: Per-pixel class label (road, sky, car, pedestrian, ...)

Example:
  Car → all pixels in car object same label "car"
  Another car → same label (no distinction between instances)
```

**FCN (Fully Convolutional Networks)**: Replace FC layers with 1×1 convolutions, enable variable input sizes, use skip connections to preserve spatial resolution.

**U-Net**: Encoder-decoder with skip connections, used for medical image segmentation.

```
Encoder: Downsample (Conv + Pool)
Decoder: Upsample (Transposed Conv) + Skip merge from encoder
Output: Per-pixel classification
```

### Instance Segmentation

Classify pixels AND distinguish separate instances.

```
Output: Per-pixel label (car_1, car_2, pedestrian_1, ...)
(same class, different instances)
```

**Mask R-CNN**: Extend Faster R-CNN with segmentation branch.

```
Faster R-CNN: Boxes + Classification
Mask R-CNN: + Mask per box (FCN on RoI)

Per bounding box:
  → Classification head (is it a car?)
  → Bounding box regression head (refine box)
  → Mask head (pixel-level segmentation within box) ← NEW
```

## Pose Estimation

Localize body/object keypoints (joints, landmarks).

```
Input: Image of person
Output: Keypoint positions (nose, eyes, shoulders, elbows, wrists, hips, knees, ankles)
```

**Approach**: CNN backbone → feature maps → per-keypoint heatmaps (heat=confidence at location).

```
Backbone: ResNet-50
  ↓
Feature Map (64×64)
  ↓ (per 17 keypoints)
Heatmap 1: nose confidence at each pixel
Heatmap 2: left_eye confidence
... (17 total)
Key-point Matching: Connect confidences above threshold
```

**Post-processing**: Greedy connection (match keypoints across frames for temporal consistency) or graph-based optimization.

## Optical Character Recognition (OCR)

Extract text from images.

**Pipeline**:
1. **Localization**: Detect text regions (bounding boxes)
2. **Recognition**: Classify letter sequences per region

**Modern approach**: End-to-end CNN + RNN:
- CNN extracts features from image strip
- RNN (LSTM) sequences features into character predictions
- CTC loss (Connectionist Temporal Classification) handles variable-length sequences

Used in: document scanning, street sign reading, license plate recognition.

## Data Augmentation & Regularization

Artificially expand training data to improve generalization.

**Techniques**:
- **Geometric**: Crop, flip, rotate, warp, perspective transform
- **Color**: Brightness, contrast, hue, saturation, color jitter
- **Dropout**: Randomly zero activations per layer (prevents co-adaptation)
- **Batch Normalization**: Normalize layer inputs (faster convergence, regularization effect)
- **L2 regularization**: Penalize large weights

**Trade-off**: More augmentation = more robust but slower training; less = faster training but overfitting on small datasets.

## Common Architectures & Libraries

**Architectures**: EfficientNet, Vision Transformer (ViT), ConvNeXt (modern convnets), Swin Transformer (hierarchical vision transformer)

**Frameworks**: PyTorch, TensorFlow/Keras, JAX

**Pre-trained weights**: ImageNet1k, ImageNet21k, self-supervised (DINO, MAE)

## Cross-References

See also: [machine-learning-fundamentals.md](machine-learning-fundamentals.md) (neural networks, backpropagation), [ml-deep-learning.md](ml-deep-learning.md) (RNNs, attention), [genai-multimodal.md](genai-multimodal.md) (vision-language models), [ml-feature-engineering.md](ml-feature-engineering.md) (data augmentation techniques)