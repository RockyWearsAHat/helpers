# Natural Language Processing — Modern Pipelines, Embeddings, and Downstream Tasks

Natural language processing transforms text into machine-readable representations and extracts meaning. Modern NLP pipelines combine pre-trained embeddings, task-specific fine-tuning, and end-to-end neural models.

## Text Preprocessing & Tokenization

### Tokenization

Split text into discrete units (words, subwords, characters).

```
Text: "ChatGPT's transformers aren't traditional."

Word Tokenization: ["ChatGPT's", "transformers", "aren't", "traditional", "."]

Subword Tokenization (BPE): ["Chat", "GPT", "'s", "trans", "formers", "aren", "'t", "traditional", "."]
```

**Tokenization strategies**:
- **Word-level**: Simple but large vocabulary (100K+), struggles with rare words, inflections
- **Subword (BPE, WordPiece)**: Balance vocab size (~50K) and coverage, handle rare words gracefully
- **Character-level**: Small vocab (100 chars), but long sequences, harder to model

**BPE (Byte-Pair Encoding)**: Iteratively merge most frequent adjacent tokens; compress 256→50K tokens.

### Normalization

Standardize text representation:
- **Lowercasing**: Reduce vocabulary (context-dependent: keep for most tasks, preserve for casing-sensitive like sentiment)
- **Accent removal**: "café" → "cafe"
- **Stop word removal**: Remove common words (optional, modern deep models handle this)
- **Lemmatization/Stemming**: Reduce to root form

```
Lemmatization: walked→walk, running→run, was→be
Stemming: running→run, runner→run (crude suffix stripping)
```

## Embedding Models

Map text (words, sentences, documents) to dense vectors in shared semantic space.

### Word Embeddings (Distributed Representations)

#### Word2Vec (Skip-gram)

Given word, predict context words (or vice versa).

```
Training: "The quick brown fox jumps"

Skip-gram (word → context):
  Input: "brown"
  Output: predict {"quick", "fox"} (within context window of 3)

Loss: Minimize prediction error on context words
```

**Result**: Each word assigned a 300-dim vector where similar words (semantically/syntactically) cluster close.

**Properties**: Arithmetic works (king - man + woman ≈ queen); captures syntactic and semantic similarity.

**Limitation**: Single vector per word (polysemy: "bank" has different meanings).

#### GloVe (Global Vectors)

Matrix factorization approach: Create word-context co-occurrence matrix, factor it.

$$\min \sum_{i,j} f(X_{ij}) (w_i^T w_j + b_i + b_j - \log X_{ij})^2$$

Where $X_{ij}$ = co-occurrence count, $w, b$ = word & bias vectors.

**Advantage**: Leverages global statistics; often better on analogies than Word2Vec.

### Contextual Embeddings (Transformer-based)

#### ELMo (2018)

Extract embeddings from intermediate layers of pre-trained BiLSTM language model.

```
Upstream Task: Language Modeling (predict next word given context)
  ↓
BiLSTM learns bidirectional representations
  ↓
Extract hidden states from multiple layers
  ↓
ELMo embeddings: Learned weighted combination of layer outputs
```

**Key insight**: Different layers encode different levels of information (syntax, semantics). Downstream tasks benefit from weighted combination.

#### BERT (2018)

Pre-train bidirectional Transformers with masked language modeling.

```
Upstream Objective: Given masked tokens, predict them
  Input:  "The quick [MASK] fox jumps [MASK] the lazy dog"
  Predict: "brown", "over"

Full context (left+right) available during prediction → deep bidirectional representations
```

**Architecture**: 12-768 (base) or 24-1024 (large) transformer layers. 110M-340M parameters.

**Fine-tuning**: Adds task-specific head (classification, NER tagging, etc); train end-to-end on target task.

**Variants**: RoBERTa (improved training), ALBERT (parameter sharing), DistilBERT (50% fewer parameters, 40% faster).

#### Sentence Transformers (2019)

Fine-tune BERT for sentence/paragraph embeddings, enabling semantic similarity search.

```
Input: Pair of sentences (s1, s2)
Architecture:
  s1 → BERT → Embedding e1
  s2 → BERT → Embedding e2
  
Loss: Contrastive loss (similar pairs close, dissimilar far)

Output: Fixed-size sentence embeddings
```

**Use case**: Semantic search, clustering, duplicate detection.

**Models**: sentence-transformers/all-MiniLM-L6-v2 (small, fast), all-mpnet-base-v2 (larger, better quality).

### Multilingual & Domain-Specific Models

- **mBERT**: Single BERT for 100+ languages (shared vocab, but larger vocabulary)
- **SciBERT**: Pre-trained on scientific papers (PubMed, ArXiv); better for biomedical NER
- **FinBERT**: Financial domain, captures domain terminology
- **DomainBERT**: Domain-adaptive pre-training boosts downstream performance

## Named Entity Recognition (NER)

Identify and classify entities (Person, Organization, Location, Date, etc.).

```
Text: "Steve Jobs founded Apple in Cupertino on April 1, 1976."

Entities:
  Steve Jobs: Person
  Apple: Organization
  Cupertino: Location
  April 1, 1976: Date
```

### Sequence Labeling Approach

Tag each token with BIO scheme (Begin, Inside, Outside):

```
Word:  ["Steve", "Jobs", "founded", "Apple", ...]
Tag:   ["B-PER", "I-PER", "O", "B-ORG", ...]

B-PER: Begin Person
I-PER: Inside Person (continuation)
O: Outside entity
```

### Model Architectures

#### BiLSTM-CRF

Bidirectional LSTM (captures context) + Conditional Random Field (models tag dependencies).

```
Input Embeddings
  ↓ [BiLSTM]
Hidden States (2d vectors per token)
  ↓ [CRF Layer]
Constrain tag sequences (e.g., I-PER never follows O without B-PER)
  ↓
Viterbi Decoding: Most likely tag sequence
```

**Why CRF?** LSTM independently predicts tags; CRF adds global constraints (improves consistency).

#### Transformer-Based (BERT-NER)

Pre-trained BERT + fine-tuning on NER task.

```
Input: [CLS] Steve Jobs founded ... [SEP]
  ↓ [BERT]
Token Representations
  ↓ [Classification Head per Token]
Tags: B-PER, I-PER, O, B-ORG, ...
```

Much higher accuracy than BiLSTM-CRF; benefits from pre-trained knowledge.

## Relation Extraction

Identify relationships between entities.

```
Text: "Bill Gates founded Microsoft in 1975."

Relations:
  (Bill Gates, founded, Microsoft)
  (Microsoft, foundationDate, 1975)
```

### Approaches

#### Feature-Based (Linguistics)

Extract hand-crafted features:
- POS tags: NNP (proper noun), VB (verb), etc.
- Dependency tree: Is subject the founder, object the company?
- Entity types: Person-Verb-Organization pattern
- Word distance, bag-of-words between entities

Train SVM/CRF classifier.

#### Neural (Sequence Tagging)

Extend NER to tag relation types on entity pairs.

Drawback: Scales poorly with entities per sentence.

#### Span-Based (End-to-End)

Enumerate entity pairs, classify relation independently.

```
For each (e1, e2) span pair:
  Compute span representations (context + entity features)
  Classifier: Relation Type or None
```

#### Transformer Fine-tuning (Modern)

BERT fine-tuned on relation extraction datasets (TACL, SemEval).

```
Input: [CLS] Bill Gates [E1] founded [/E1] Microsoft [E2] [SEP]
Label: founded(founder, company)
```

**Embedding-based approach**: Represent entity pairs as (e1_emb + e2_emb); relation classifier operates on this representation.

## Document Classification

Assign documents to predefined categories.

```
Text: "The Fed raised interest rates by 50 basis points today..."
Category: Finance (vs. Sports, Politics, Technology, etc.)
```

### Simple Baselines

**TF-IDF + SVM**: Vectorize text (term frequency), train SVM classifier. Interpretable, fast, works for simple categories.

**Naive Bayes**: Probabilistic model P(category|words). Fast, works well when word independence assumption approximately holds.

### Neural Models

#### CNN for Text

Treat text as sequence, apply 1D convolutions.

```
Input: Word embeddings [500 words × 300-dim]
  ↓ [Conv filters: 1×300, 2×300, 3×300]
Feature maps
  ↓ [Max pooling per filter type]
Combined features
  ↓ [FC Classification]
Category probabilities
```

Captures n-gram patterns (filters slide over adjacent words).

#### RNN/LSTM

Process sequence sequentially, capture long-range dependencies.

```
Input: w1 → w2 → w3 → ... → wn
  ↓ [LSTM cell per step]
Hidden states: h1, h2, ..., hn
  ↓ [Attention or last hidden state]
Document representation
  ↓ [FC Classification]
```

#### Transformers (BERT-Classification)

Pre-trained BERT + classification head.

```
[CLS] token representation
  ↓ [Fine-tuning on labeled data]
Classification logits
```

State-of-art; often 88-95% accuracy on benchmark datasets (AGNews, DBpedia).

## Topic Modeling

Discover latent topics (themes) in document collection unsupervised.

```
Documents: {d1, d2, d3, ...}
↓ Topic Modeling
Topics: {T1: "politics", T2: "sports", T3: "finance"}
Document Distribution:
  d1: {T1: 0.6, T2: 0.3, T3: 0.1} (mostly politics)
  d2: {T1: 0.1, T2: 0.7, T3: 0.2} (mostly sports)
```

### LDA (Latent Dirichlet Allocation)

Bayesian model: documents = mixtures of topics; topics = word distributions.

```
Generative Process:
  1. For each document: Sample topic distribution θ ~ Dirichlet(α)
  2. For each word position in document:
     a. Sample topic z ~ θ
     b. Sample word w from topic's word distribution β[z]
```

Use Gibbs sampling to infer topics and document-topic assignments.

**Interpretation**: Word "university" has high probability in "education" topic, low in "sports" topic.

## Text Generation & RAG

### Generation

Sequence-to-sequence: Produce text autoregressively (predict next token given history).

```
Input: "Summarize: The stock market..."
  ↓ [Encoder: BERT]
Context representation
  ↓ [Decoder: GPT-like]
Generate: "The stock... [/STOP]"
```

Loss: Cross-entropy on target token sequence.

**Decoding strategies**:
- **Greedy**: Always pick highest-probability token (fast, myopic)
- **Beam search**: Track K most likely sequences, refine (better quality)
- **Sampling**: Sample from distribution (diverse but sometimes incoherent)
- **Top-k / nucleus sampling**: Sample from top-k or cumulative probability threshold (balanced)

### Retrieval-Augmented Generation (RAG)

Retrieve relevant documents, condition generation on them.

```
Query: "What is the capital of France?"
  ↓ [Retriever: Dense embedding search]
Top docs: Wikipedia article on France, etc.
  ↓ [Generator: Seq2seq with context]
Generate: "The capital of France is Paris."
```

**Advantage**: Factuality improved (grounded in retrieved data), handles out-of-distribution queries better.

## Summarization & Evaluation

### Abstractive Summarization

Generate new text capturing key information.

```
Input Document (1000 words)
  ↓ [Seq2seq or Encoder-Decoder]
Summary (100 words, human-written-like)
```

### Extractive Summarization

Select and reorder existing sentences.

```
Input Document
  ↓ [Score sentences by relevance]
Top sentences: [2, 5, 8]
  ↓ [Reorder and conjoin]
Summary
```

Often simpler to evaluate (ground truth is subset of original).

### Evaluation Metrics

**ROUGE (Recall-Oriented Understudy for Gisting Evaluation)**:
- ROUGE-N: Overlap of N-grams between system summary and reference
- ROUGE-L: Longest common subsequence
- Drawback: Lexical overlap, penalizes paraphrases

**BLEU (Bilingual Evaluation Understudy)**:
- Precision of N-grams
- Used for machine translation
- Known issue: Doesn't always correlate with human judgment

**BERTScore**:
- Compare embeddings of summary vs. reference (semantic similarity)
- Better correlation with human judgment than ROUGE/BLEU

## Common Frameworks & Libraries

- **Transformers (Hugging Face)**: Pre-trained models, easy fine-tuning
- **spaCy**: Industrial NLP (tokenization, POS, NER, dependency parsing)
- **NLTK**: Educational NLP toolkit
- **PyTorch, TensorFlow**: Deep learning backends

## Cross-References

See also: [ml-nlp-fundamentals.md](ml-nlp-fundamentals.md) (tokenization, seq2seq, attention), [genai-embeddings-vectors.md](genai-embeddings-vectors.md) (embeddings detail), [genai-rag-patterns.md](genai-rag-patterns.md) (RAG systems), [cs-information-retrieval.md](cs-information-retrieval.md) (retrieval component), [cs-knowledge-graphs.md](cs-knowledge-graphs.md) (NER, relation extraction for KG construction)