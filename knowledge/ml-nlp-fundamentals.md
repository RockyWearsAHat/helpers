# Natural Language Processing Fundamentals — Tokenization, Embeddings, Sequence Models & Transformers

Natural Language Processing transforms text—humanity's primary communication medium—into a form algorithms can process. Despite language's ambiguity and context-dependence, NLP has evolved from rule-based systems to learned representations that capture semantic meaning remarkably well.

## Tokenization

**Tokenization** breaks text into smaller units (tokens): typically words, but can be subwords, characters, or sentences depending on the task. Simple word tokenization splits on whitespace but breaks with contractions ("don't" → "do", "n't"), punctuation, and language-specific rules.

**Byte-pair encoding (BPE)** is a subword tokenization used by GPT and other modern models. Given raw text, iteratively merge the most frequent adjacent token pair, learning a vocabulary of subword units. This handles out-of-vocabulary words: unknown words decompose into learned subwords.

**Sentence-piece** and similar algorithms learn tokenization and vocabulary jointly from raw text, optimal for language-specific patterns and multilingual settings.

**WordPiece** (used in BERT) applies BPE with a probabilistic twist: prioritize merges that maximize likelihood under a language model. Tokens balance vocabulary size and reconstruction efficiency.

Tokenization choices affect downstream models: character-level tokenization requires longer sequences; word tokenization risks OOV problems; subword tokenization balances both.

## Word Embeddings

A word embedding is a dense vector (typically 50-300 dimensions) representing a word. Words with similar meanings cluster nearby in vector space. Embeddings replace one-hot encoding (sparse, high-dimensional) with dense, learned representations.

**Word2Vec** learns embeddings by predicting context from a word (skip-gram) or vice versa (CBOW). Skip-gram: given "the dog barks", predict context words {barks, the} from "dog". Training with stochastic gradient descent on a corpus teaches embeddings where semantically related words cluster. Word2Vec captures analogy reasoning: the vector arithmetic dog + female ≈ bitch (in some embeddings).

**GloVe (Global Vectors)** combines matrix factorization of word co-occurrence statistics with local context windows. Combines global corpus statistics (which words co-occur) with local context (like skip-gram), often producing smoother embeddings.

**FastText** extends Word2Vec by learning embeddings for subword n-grams, then composing word embeddings from subword vectors. This handles morphological variants and out-of-vocabulary words gracefully. A word like "playing" decomposes into character n-grams; the embedding combines these learned subword representations.

**Key limitation**: all three produce one embedding per word, ignoring context. "Bank" (financial institute) and "bank" (river shore) get the same embedding, conflating distinct meanings.

## Contextualized Embeddings

**Contextualized embeddings** (ELMo, BERT) generate term-level representations conditioned on surrounding context. "Bank" in "savings bank" and "river bank" produce different embeddings, enabling word sense disambiguation.

**ELMo** stacks bidirectional LSTM layers, learning multiple layers of context. Downstream tasks use learned weighted combinations of these layers (different layers capture different linguistic phenomena).

**BERT (Bidirectional Encoder Representations from Transformers)** uses transformer layers (discussed below) to encode bidirectional context. BERT is pretrained on two tasks: masked language modeling (predict masked tokens) and next-sentence prediction. The learned representations transfer well to downstream tasks with minimal fine-tuning.

**GPT** uses unidirectional transformers optimized for next-token prediction. Unlike BERT's bidirectional encoding, GPT predicts autoregressively: encode past tokens, predict the next one. This design enables generation naturally.

## Sequence Models

**Recurrent Neural Networks (RNNs)** process sequences token-by-token, maintaining a hidden state that accumulates information from past tokens. At each step: h_t = f(x_t, h_{t-1}). RNNs are universal approximators for sequence functions but suffer from **vanishing gradient** problem: gradients shrink exponentially over many steps, preventing learning of long-range dependencies (>10-15 tokens).

**LSTMs (Long Short-Term Memory)** add memory cells and gates to RNNs, mitigating vanishing gradients. Gates (input, output, forget) control information flow, allowing the network to keep important information over many steps. LSTM hidden states can model 100+ token dependencies effectively.

**GRUs (Gated Recurrent Units)** simplify LSTMs with fewer parameters (reset and update gates) while maintaining similar performance. Lighter computational cost, similar expressiveness.

**Bidirectional RNNs** process sequences left-to-right and right-to-left, concatenating representations. This requires the entire sequence in advance (no online/streaming processing), but captures full context. Bidirectionality is standard in NLP encoders for tasks like named entity recognition and sentiment analysis.

## Attention Mechanisms

**Attention** lets the model focus on relevant tokens regardless of distance. Rather than compressing all history into a fixed-size hidden state, attention computes relevance scores between the query token and all sequence tokens, then computes a weighted average.

**Self-attention** applies attention within a single sequence: each token attends to all other tokens. The matrix of queries Q, keys K, and values V from a sequence allows parallel computation of all attention weights: softmax(QK^T / √d)V.

Self-attention is the core mechanism enabling **Transformers**, introduced in "Attention Is All You Need" (Vaswani et al., 2017). Unlike RNNs that process sequentially, transformers process entire sequences in parallel, then compute attention across all pairs. This parallelism drastically reduced training time.

**Multi-head attention** computes attention multiple times with different learned projections (heads), then concatenates. Different heads learn different relevance patterns: some attend to adjacent words (syntax), others to distant related words (semantics). This diversity improves representation power.

## BERT and GPT Lineage

**BERT** (2018) demonstrated that massive bidirectional pretraining on unlabeled text learns representations useful for downstream tasks. Masked language modeling (masking 15% of tokens, predicting them) and next-sentence prediction are self-supervised tasks requiring no labels. Fine-tuning BERT on specific tasks (classification, NER, QA) achieved SOTA (state-of-the-art) results with minimal task-specific data. BERT's bidirectionality is a strength for encoding tasks but limits generation.

**GPT** (2018, refined in GPT-2, GPT-3) uses left-to-right (causal) transformer language modeling: predict the next token autoregressively. GPT models optimize generative tasks: text generation, summarization, machine translation. GPT-3 (175B parameters) showed few-shot learning: given task examples in the prompt, it performs the task without fine-tuning ("in-context learning").

**BERT variants** (RoBERTa, DistilBERT, ALBERT) optimize training data, corpus size, architecture, and regularization. RoBERTa improved BERT with longer training and larger batches; DistilBERT compresses BERT for efficiency; ALBERT shares parameters across layers, reducing memory.

**Decoder-only models** (GPT lineage) also enable classification and NER by constructing prompts. Encoder-only models (BERT) are specialized for encoding. **Encoder-decoder models** (BART, T5) combine both: encode the input, then decode a target sequence. BART masks and deletes tokens (corrupts input), then decodes the original—a flexible self-supervised task enabling diverse downstream applications.

## NLP Tasks

**Named Entity Recognition (NER)** identifies and classifies named entities: person, organization, location, product. Sequence labeling assigns a tag to each token (BIO scheme: Begin, Inside, Outside). RNNs and transformers fine-tuned on annotated corpora perform NER; recent models achieve >95% F1 on standard benchmarks.

**Sentiment Analysis** classifies text polarity (positive, negative, neutral). Can be sentence-level (single prediction per text) or aspect-based (sentiment toward specific aspects). LSTMs and transformers trained on opinionated text learn sentiment patterns; transfer learning from pretrained models accelerates training.

**Text Classification** assigns documents to categories: spam/not spam, topic (sports, politics, tech), intent (support request, complaint, question). Can be binary or multi-class. Transformers fine-tuned for classification add a linear layer on top of the pooled representation (e.g., BERT's [CLS] token).

**Summarization** condenses text while preserving key information. Extractive summarization selects important sentences; abstractive summarization generates new text. Seq2seq models (encoder-decoder, transformers) enable abstractive summarization, though hallucination (generating plausible but false information) remains a challenge.

**Machine Translation** converts text from one language to another. Seq2seq models with attention enabled SOTA translation: encode source text, decode target language token-by-token. Multilingual models (mBERT, mT5) handle many language pairs in one model through shared representations.

**Question Answering** retrieves or generates answers to questions. Span-based QA (SQuAD) extracts a span from the document; generation-based QA produces free-form answers. Transformers excel here: encode context and question, predict start and end positions (span-based) or generate tokens (generation-based).

## Linguistic Phenomena

NLP models learn to handle **syntactic** structures (grammar, part-of-speech tags) and **semantic** relationships (meaning, synonymy, entailment). **Pragmatic** understanding (context, intent, figurative language) remains challenging; sarcasm, irony, and cultural references often confuse even large models.

**Word senses** (homonymy, polysemy) and **referential ambiguity** (pronoun resolution) require context. Contextualized embeddings address this better than static embeddings.

**Bias and fairness** in NLP models reflect training data: embeddings may encode gender, racial, or other stereotypes. Debiasing via data or postprocessing is an active area.

## See Also

- genai-llm-architecture.md — transformer architecture in depth
- genai-embeddings-vectors.md — embeddings and vector spaces
- machine-learning-fundamentals.md — supervised learning foundations