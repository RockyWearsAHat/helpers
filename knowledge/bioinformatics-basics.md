# Bioinformatics Computing — Sequence Alignment, BLAST, Genome Assembly, Phylogenetics, AlphaFold, Single-Cell Analysis, Pipelines

## Overview

Bioinformatics applies computation to biology: aligning sequences to find similarity and homology, assembling genome fragments into chromosomes, predicting protein 3D structure, and analyzing cellular populations. The field combines algorithms, statistics, and domain biology. Scale ranges from individual genes (kilobases) to whole genomes (gigabases) to cellular populations (millions of cells, terabytes of data). Success requires both algorithmic sophistication and biological intuition about meaningfulness of results.

See also: [algorithms-string.md](algorithms-string.md), [machine-learning-fundamentals.md](machine-learning-fundamentals.md).

## Biological Sequence Data

### Nucleotide and Protein Sequences

**DNA**: Nucleotides A, T, G, C encoding genetic information. Approximately 3 billion base pairs human genome.

**RNA**: Similar to DNA; T → U. Messenger RNA (mRNA), regulatory RNA (miRNA). Can be degraded or modified.

**Proteins**: 20+ amino acids linked by peptide bonds. Function determined by 3D structure; sequence alone doesn't fully specify structure.

**Central dogma**: DNA → (transcription) → RNA → (translation) → Protein.

### Standard File Formats

**FASTA** (text):

```
>sequence_id description
ATCGATCGATCG
```

Ubiquitous; simple; human-readable; no metadata embedded.

**FASTQ** (sequencing reads):

```
@read_id description
ATCGATCGATCG
+
IIIIIIIIIIII
```

Includes quality scores (confidence in each base call). Standard output from DNA sequencers.

**GenBank/EMBL** (annotated):

```
LOCUS  gene_name            1000 bp    DNA     linear   UNK
...
ORIGIN
  1 atcgatcgatc gatcgatcgat cgatcgatcg
```

Rich metadata: genes, regulatory regions, features.

**VCF (Variant Call Format)**: Differences (Single Nucleotide Polymorphisms—SNPs, insertions, deletions) from reference genome.

```
##fileformat=VCFv4.2
#CHROM  POS  ID  REF ALT
chr1    1000 .   A   G
```

**BAM/SAM (Sequence Alignment/Map)**: Mapped reads (sequence reads aligned to reference genome). Binary (BAM) or text (SAM). Used in next-generation sequencing analysis.

## Sequence Alignment

### Global vs. Local Alignment

**Global alignment**: Align entire sequences end-to-end. Assumes sequences are homologous across their full length. Algorithm: **Needleman-Wunsch** (dynamic programming).

**Local alignment**: Find best local match between sequence regions. Useful when similarity is confined to domains (conserved functional regions). Algorithm: **Smith-Waterman** (dynamic programming).

### Smith-Waterman Algorithm (Local Alignment)

**Goal**: Align two sequences to maximize match score.

**Scoring scheme**:

- Match: $+s$ (e.g., $+2$)
- Mismatch: $-p$ (e.g., $-1$)
- Gap: $-g$ (e.g., $-1$ per gap, or affine: $-d - e \times \text{length}$)

**Dynamic programming table** $H[i][j]$ = maximum alignment score for first $i$ characters of sequence 1, first $j$ of sequence 2.

Recurrence:

$$H[i][j] = \max\begin{cases}
H[i-1][j-1] + \text{match/mismatch score} \\
H[i-1][j] - \text{gap} \\
H[i][j-1] - \text{gap} \\
0
\end{cases}$$

The $0$ minimum (versus Needleman-Wunsch's $-\infty$) allows local alignment to begin anywhere.

**Traceback**: Start from highest $H$ value in table; follow path back to where score is 0; reconstruct alignment.

**Complexity**: $O(nm)$ time and space for sequences of length $n, m$.

### Gotchas

**Gap penalties heavily influence results**. Linear gaps ($-g$ per gap) favor fewer, longer gaps; affine gaps ($-d - e \times \text{length}$) favor opening a gap differently than extending it. Biological intuition: insertions/deletions are rare, but once started, extend at lower cost.

**Alignment is not identity**: High-scoring alignment reflects evolutionary relationship, but score-to-probability mapping depends on background distribution (what aligns by chance?).

## BLAST — Basic Local Alignment Search Tool

### The Problem

Smith-Waterman finds best local alignment between two sequences in $O(nm)$ time. Querying a 1,000-base sequence against a 3-billion-base genome is prohibitive ($3 \times 10^{12}$ operations).

### BLAST Solution: Heuristic Acceleration

**Idea**: Identify high-scoring **words** (short exact or near-exact matches); build alignments around words.

**Algorithm**:

1. Break query into $w$-mers (short subsequences, typically $w=8-12$).
2. Find all matches of each $w$-mer in database (via hash table).
3. Extend matches bidirectionally using dynamic programming.
4. Report high-scoring local alignments.

**Consequence**: Trades exhaustiveness for speed. May miss weak homology; standard approximation accepted in practice.

### BLAST Variants

- **BLASTN**: Nucleotide query vs. nucleotide database.
- **BLASTP**: Protein query vs. protein database. More sensitive (homology easier to detect at protein level due to 20 amino acids vs. 4 nucleotides).
- **BLASTX**: Translated nucleotide query (all 6 reading frames) vs. protein database. Detects remote homology.
- **TBLASTN**: Protein query vs. translated nucleotide database.

### E-value and Significance

**E-value** (expect value): Number of matches expected by **chance** when searching database with random sequences.

$E = m \times n \times P(\text{score} \geq S)$

where $m, n$ are database and query sizes, $P$ is probability of achieving score $S$ by chance.

**Interpretation**: E-value $10^{-10}$ means you'd expect 1 such score per $10^{10}$ random database searches. Low E-value suggests true homology, not noise. Threshold: E $< 10^{-5}$ typical for significance.

**Caveats**:

- E-value assumes independence; proteins have domains, enabling multiple weak matches.
- Multiple testing correction often needed (searched many sequences so some hits by chance).

## Genome Assembly

### The Problem

DNA sequencing produces millions to billions of short reads (100–10,000 bases, depending on technology). Assembly reconstructs the full genome by overlapping reads.

**Challenge**: Reads are short, repetitive sequences appear in genome, and reads contain errors (especially later in sequencing run).

### Overlap-Layout-Consensus Approach

**Overlap**: Find all pair-wise overlaps between reads (read A's tail matches read B's head by ~50+ bases).

**Layout**: Build graph where nodes are reads, edges connect overlapping reads. Find path visiting each read once (Hamiltonian path-like problem; NP-hard, heuristically solved).

**Consensus**: For each read, align all reads aligned to that region, take majority vote (error correction).

**Complexity**: All-pairs search is $O(n^2)$ for $n$ reads; acceleration via $k$-mer indexing.

### De Bruijn Graph Approach (Modern)

**$k$-mer**: Break all reads into overlapping $k$-mers (e.g., $k=21$).

**Graph construction**: Each $k$-mer is a node. Two $k$-mers are connected if they overlap by $k-1$ bases (i.e., read of $(k+1)$-mers is a path in the graph).

**Genome = Eulerian path** through graph: visit each edge exactly once.

**Advantage**: $O(n)$ in number of $k$-mers, not reads. Handles errors and repeats better.

**Limitation**: Choice of $k$ is critical. Too small: $k$-mers appear many times (ambiguous). Too large: rare $k$-mers disappear (false breaks). Sophisticated assemblers use multiple $k$ values.

### Assembly Challenges

**Repeats**: Long repeated sequences (like transposable elements) create ambiguity; identical sequences generate identical $k$-mers, but in different genomic locations.

**Heterozygosity**: In diploid organism, two chromosome copies may differ. Assembly must resolve which allele, or phase them.

**Errors**: Sequencing errors accumulate; low-coverage regions are underassembled.

### Output: Scaffolds, Contigs

**Contig** (contiguous sequence): Bases for which coverage is high and layout is unambiguous.

**Scaffold**: Multiple contigs connected by mate-pair information (sequencing capture pair-end fragments providing long-range constraints).

**Gap filling**: Closing unknown sequences between scaffolds via targeted resequencing or computational inference.

## Phylogenetic Analysis

### Evolutionary Relationships

**Phylogenetics** reconstructs evolutionary history from sequence data. Assumption: sequences from related organisms diverged from common ancestor; sequence similarity indicates recent common ancestry.

### Distance-Based Methods

**Calculate pairwise distances**: Hamming distance, Jukes-Cantor, or other evolutionary models accounting for multiple substitutions.

**Build tree**: Neighbor-joining (fast, heuristic) or UPGMA (simple but assumes equal rates). Greedy algorithm connecting nearest sequences/clusters.

**Output**: Tree topology showing branching order and (optionally) divergence times.

### Phylogenetic Maximum Likelihood

**Goal**: Find tree topology and branch lengths maximizing probability of observed sequences given evolutionary model.

**Model**: Specify substitution rates (transition/transversion bias, etc.). Tree has topology and branch lengths (time since divergence).

**Inference**: Search over possible topologies (computationally expensive for $>15$ sequences); compute likelihood; find maximum. SPR (subtree pruning-regrafting) or NNI (nearest-neighbor interchange) moves explore tree space.

**Advantage**: Statistically grounded; confidence intervals for divergence times. **Disadvantage**: Slow for large trees.

### Bayesian Phylogenetics

**Prior** on tree topology and parameters. **Likelihood** of data. **Posterior** combines both via MCMC sampling.

**Advantages**: Uncertainty quantification (credible intervals); incorporate fossil data via calibration. **Disadvantages**: Computationally demanding; results sensitive to priors.

## Protein Structure Prediction

### From Sequence to Structure

Proteins fold into 3D structures stabilized by hydrogen bonds, hydrophobic packing, disulfides. For decades, structure was determined experimentally (X-ray crystallography, NMR, cryo-EM) — slow and expensive.

### Traditional Approaches: Homology Modeling

**Idea**: If sequence is similar to a protein of known structure, fold is probably similar.

**Algorithm**:

1. Find homolog in Protein Data Bank (PDB) with known structure via BLAST/PSI-BLAST (position-specific scoring matrix).
2. Align query to template.
3. Model loops and variable regions; optimize side chains.
4. Refine with energy minimization.

**Accuracy**: Limited to $\sim 2-3\,\mathrm{Å}$ RMSD (root mean square deviation), and only if template identity $>30\%$.

### AlphaFold — Learning-Based Structure Prediction

**Transformative breakthrough** (2020, DeepMind). Neural network trained on known protein structures predicts 3D structure from sequence alone, achieving experimental-grade accuracy ($\sim1-2\,\mathrm{Å}$ globally).

**Key innovations**:

1. **Multiple sequence alignment (MSA)**: Query sequence against related sequences (homologous proteins); alignment patterns contain evolutionary constraints; fed to model.
2. **Transformer architecture**: Attention mechanism scales to large proteins ($>1000$ residues).
3. **Geometric learning**: Output intermediate representations (pairwise distances); final layer decodes into atomic coordinates.

**Advantages**:

- High accuracy across fold space (not just similar-to-template).
- Fast (seconds to minutes per protein).
- Confidence scores per residue (useful for distinguishing high-confidence structured regions from uncertain loops).

**Limitations**:

- No explicit energetics; doesn't reason about thermodynamics or kinetics.
- Trained on single structures; can't predict conformational ensembles or dynamics.
- Ligand binding, cofactors not modeled.

**Impact**: Dramatically accelerated structural biology. Complete proteomes (human, model organisms) now predicted. Enabled drug design without crystallography.

## Single-Cell Analysis

### Data and Motivation

Modern RNA-seq and protein assays measure expression in individual cells, not cell populations. Reveals cell-type diversity, developmental trajectories, and disease heterogeneity obscured by bulk averaging.

**Typical workflow**:

1. Tissue → single-cell suspension.
2. Capture, barcode, lyse cells (droplet or plate-based).
3. Sequence cDNA (reverse transcribed RNA).
4. Quantify: gene expression (counts per cell × gene).

### Data Characteristics

**Matrix**: Cells (columns) × Genes (rows) × Expression counts (values). Often $10^4$ cells, $20,000$ genes = $2 \times 10^8$ sparse entries (most cells don't express most genes).

**Sparsity**: Single-cell RNA-seq is "dropout-heavy"; many 0 values due to stochastic capture or degradation.

**Batch effect**: Cells processed in different batches (time, reagent lot, technician) show systematic variation confunding biology-of-interest.

### Analysis Steps

**Quality control**: Filter low-quality cells (few detected genes, many mitochondrial genes suggesting cell lysis).

**Normalization**: Account for cell-to-cell differences in capture efficiency (library size normalization, log transformation).

**Batch correction**: Harmonize across batches (ComBat, Harmony, mutual nearest neighbors).

**Dimensionality reduction**: PCA, UMAP, t-SNE project high-dimensional counts to 2D for visualization; clusters visible by eye.

**Clustering**: Identify groups of similar cells (k-means, hierarchical, Louvain modularity). Interpret as cell types or states.

**Differential expression**: Compare gene expression between clusters (t-test with Bonferroni correction, or robust methods like DESeq2, limma-voom).

**Annotation**: Assign clusters to known cell types using marker genes, reference datasets.

### Challenges

**Sparsity**: Dropout undermines many analyses; missing data imputation (kNN, deep generative models) helps but introduces artifacts.

**Heterogeneity within cell type**: Cell-type boundary is often soft (continua rather than discrete types); best practices still evolving.

**Contamination**: Batch effects, doublets (two cells captured together), ambient RNA.

**Scalability**: Millions of cells × millions of genes; memory and speed challenges. Specialized tools (Scanpy, Seurat) manage this.

## Bioinformatics Pipelines

### Workflow Orchestration

**Pipeline**: Sequence of computational steps (read QC, alignment, variant calling) connected by data flow. Each step consumes previous output; produces next input.

**Challenges**: Many tools, diverse languages (Python, R, C++, Perl), parameter interdependencies. Manual execution error-prone; automation essential.

### Nextflow

**DSL (domain-specific language)** for reproducible workflows. Inspired by Unix pipes; applies them to data processing.

**Concepts**:

- **Process**: Computational task (shell script, Python source, external tool).
- **Channel**: Named data streams connecting processes.
- **Workflow**: DAG (directed acyclic graph) of processes.

**Advantages**: Parallelization automatic (Nextflow handles job scheduling); containerization (Docker/Singularity) for reproducibility; caching for incremental re-runs.

**Example** (pseudocode):

```
process fastqc {
    input: file(reads)
    output: file("*_fastqc.html")
    script: "fastqc $reads"
}

process trim {
    input: file(reads)
    output: file("trimmed.fq")
    script: "cutadapt -o trimmed.fq $reads"
}

workflow {
    reads = Channel.fromPath("*.fq")
    fastqc(reads)
    trim(reads)
}
```

### Snakemake

Similar to Nextflow; **rule**-based (resembles Make). Define rules with inputs, outputs, shell commands. Snakemake infers DAG and executes.

**Advantages**: Pythonic; integrates with conda for dependency management; strong in bioinformatics community.

**Trade-off vs. Nextflow**: Nextflow more modern and cloud-native; Snakemake more embedded in computational biology.

### Common Pipelines

**RNA-seq**: FastQC (quality) → Trimming → Alignment (STAR, Salmon) → Count aggregation → Downstream analysis (differential expression).

**Whole-genome sequencing**: FASTQ quality → Alignment (BWA) → BAM sorting → Variant calling (GATK, bcftools) → Annotation (VEP, SnpEff) → Downstream (gwas, clinical reporting).

**ChIP-seq** (protein-DNA binding): Quality → Alignment → Peak calling (MACS2, SICER) → Annotation → Motif discovery.

## Practical Considerations

### Reference Genomes and Databases

**NCBI GenBank**: Central repository; contains sequences from all organisms; growing rapidly.

**Ensembl**: Curated annotations (genes, regulatory elements) for vertebrates and key model organisms.

**UniProt**: Protein sequences and function; curated cross-references.

**PubMed**: Biomedical literature; computationally mines context for validation of predictions.

### Reproducibility and Data Sharing

**Challenges**: Dependencies (library versions, R packages) vary; random seeds impact stochastic algorithms; undocumented parameters buried in code.

**Best practices**:

- Pin versions (requirements.txt, environment.yml).
- Use containers (Docker, Singularity).
- Document computational environment and parameters.
- Share raw and processed data (genome browsers, supplementary tables).
- Register studies in repositories (GEO, SRA for sequencing, PDBe for structures).

### Validation and Curation

Computational predictions require experimental validation (wet-lab confirmation). Conversely, computational analysis must fit biological reality (cell-type assignments must agree with flow cytometry, gene function predictions consistent with literature).

**Publication expectation**: Validation experiments or orthogonal data (public datasets confirming results) substantially raise confidence.