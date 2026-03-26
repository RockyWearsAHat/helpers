# Data Anonymization — k-anonymity, l-diversity, Differential Privacy & GDPR/CCPA

## Overview

Data anonymization is the process of removing, masking, or transforming personally identifiable information (PII) so that individuals cannot be re-identified. Anonymization is used when sharing data for research, publishing analytics, or regulatory compliance (GDPR, CCPA). Unlike encryption (which hides data but allows recovery with the key), anonymization aims for permanent irreversibility. Tension: strong anonymization reduces data utility (usefulness for analysis).

---

## De-identification Approaches

### Masking/Redaction

Direct removal or replacement of obvious PII: names, SSNs, email addresses, phone numbers. Naive approach; insufficient alone (quasi-identifiers like birthdate + zip code can re-identify individuals).

### Generalization

Reduces specificity of attributes:
- Zip code 94043 → region "Mountain View, CA" → state "California" → super-state "West Coast"
- Age 35 → age group [30-40]
- Exact location → city, or census tract

Tradeoff: higher generalization = less re-identification risk but less data utility.

### Perturbation/Synthetic Data

Adds noise or generates synthetic records:
- Differential privacy (adds calibrated noise)
- Synthetic data generation: train generative model on real data, sample new records from model (not real individuals, but statistically similar)
- Swapping: exchange values between records (e.g., shuffle ages across individuals)

---

## Formal Anonymization Models

### k-anonymity (Samarati & Sweeney, 1998)

**Definition:** A dataset achieves k-anonymity if every combination of quasi-identifiers (attributes that could identify individuals) appears at least k times in the dataset.

**Example:** Dataset with name, age, zip, disease:
- Remove name, generalize: age [30-40], zip [94000-94999], disease
- If 100 people have age [30-40] AND zip [94000-94999], dataset is 100-anonymous on these quasi-identifiers

**Parameters:**
- k = 2: each quasi-identifier combination appears ≥2 times
- k = 5: safer, typically used minimum
- k ≥ 100: strong anonymity for large populations

**Advantages:** Formal guarantee; computationally straightforward.

**Weaknesses:**
- **Homogeneity attack:** If all k individuals with age [30-40], zip [94000-94999] have the *same* disease, an attacker learns the disease with certainty.
- **Background knowledge attack:** Attacker with external knowledge (e.g., "John Doe works in Mountain View, CA, is age 35") can still re-identify despite k-anonymity.

### l-diversity (Machanavajjhala et al., 2006)

Extends k-anonymity to address homogeneity attacks. **Definition:** Within each quasi-identifier group, sensitive attributes (disease, salary) must have l distinct values.

**Example:** 100 people with age [30-40], zip [94000-94999] must have ≥5 different diseases (5-diverse).

**Variants:**
- **Entropy l-diversity:** Distribution of sensitive values has high entropy (≥log(l))
- **Recursive (r, l)-diversity:** Avoids "almost-homogeneous" groups (one value dominates)

**Tradeoff:** l-diversity requires more generalization (less data utility) but prevents homogeneity attacks.

**Weaknesses:**
- Attacker might know frequency distribution from external sources (e.g., "disease X occurs in 90% of population"); l-diversity doesn't prevent inference based on prior knowledge.

### t-closeness (Li, Li & Venkatasubramanian, 2007)

Addresses statistical inference attacks. **Definition:** Within each quasi-identifier group, distribution of sensitive attribute values should be "close" to the overall distribution in the dataset.

**Measure:** Earth Mover's Distance (EMD) between group distribution and global distribution ≤ t.

**Example:** Dataset has 10% disease X, 20% disease Y, 70% disease Z globally. Every quasi-identifier group must have distribution within t-closeness (e.g., ±5%).

**Advantages:** Prevents inference using prior probability knowledge.

**Limitations:** Requires large groups and high generalization. Even small t can severely reduce data utility.

---

## Differential Privacy (ϵ-δ Framework)

Mathematical framework guaranteeing privacy loss bounds regardless of attacker knowledge. **Core insight:** presence/absence of any individual in dataset should have negligible impact on query results.

### Formal Definition

**ϵ-differential privacy:** Algorithm A satisfies ε-differential privacy if for any two adjacent datasets D and D' (differing by one record):

```
P(A(D) = output) ≤ e^ε × P(A(D') = output)
```

For all possible outputs. Implies: knowing the output of A with ε-DP, an attacker cannot confidently determine whether a specific individual is in the dataset.

**Smaller ε = stronger privacy, larger ε = weaker privacy.**

**Common ε values:**
- ε = 0.1: very strong (query output similar whether one person in or out)
- ε = 1.0: moderate (accepted in some deployments)
- ε = 10: weak (but better than no DP)

### Mechanisms

**Laplace mechanism (for real-valued queries):** Add noise drawn from Laplace distribution, scaled by sensitivity (max change in query result if one record added/removed).

```
Noisy_count = true_count + Laplace(0, Δf/ε)
Δf = sensitivity = max difference one record can make
```

**Exponential mechanism (for discrete/non-numeric outputs):** Weight candidate outputs by `e^(ε × score(output) / 2Δf)`, sample probabilistically. Higher-scoring outputs more likely.

**Gaussian mechanism:** Add Gaussian noise (privacy loss measured in √ε instead of ε, allows better accuracy).

### (ϵ, δ) Variants

**Approximate (ϵ, δ)-DP:** Relaxation allowing δ probability of violating ε-DP. Enables stronger accuracy with tunable privacy failure probability.

```
P(A(D)) ≤ e^ε × P(A(D')) + δ
```

δ typically tiny (10^-6 to 10^-8).

### Composition

**Query composition:** Answering multiple DP queries on same dataset causes privacy loss to accumulate.

- **Sequential composition:** k queries each with ε-DP → total ε' ≈ k×ε (worst case)
- **Parallel composition:** Queries on disjoint subsets → total ε same as single query
- **Advanced composition (Renyi DP):** Better bounds for many queries

**Practical impact:** Privacy budget is finite. Must allocate total ε across all queries planning to answer.

---

## Synthetic Data Generation

Generate records statistically similar to real data but not actual individuals (privacy by construction).

### Methods

**Generative Adversarial Networks (GANs):** Generator creates synthetic records; discriminator tries to distinguish from real data. Iterative training.

**Diffusion models:** Transform noise into synthetic data via reverse diffusion process; privacy depends on model training set size.

**Variational Autoencoders (VAEs):** Compress data to latent distribution, sample new records from latent space.

### Privacy-Utility Tradeoff

Synthetic data inherently trades privacy for utility:
- **Privacy:** Attacker cannot be certain any synthetic record corresponds to a real individual; but if generation process is poor, synthetic distribution differs too much from real data.
- **Utility:** Downstream models trained on synthetic data should perform similarly to real-data models.

Theoretical guarantee: differential privacy can be applied to synthetic data generation (DP-VAE, DP-GAN) to bound privacy loss formally.

---

## Tokenization and Pseudonymization

### Tokenization

Replace PII with opaque tokens via one-way hash (deterministic) or token vault (non-deterministic):

- **Deterministic:** same PII → same token (enables joins, analytics)
- **Non-deterministic:** same PII → different tokens (stronger isolation, prevents cross-dataset linking)

Example: SSN 123-45-6789 → token "tok_a3k8j2_x" (immutable, cannot recover SSN without vault key).

**Use cases:** Payment processing (PCI-DSS), healthcare (HIPAA), log systems (sensitive event IDs).

### Pseudonymization

Replace identifying attributes with pseudonyms (e.g., person Bob → ID "P1234"). Pseudonyms allow:
- Linking records for same individual across datasets (if key is retained)
- Reversal with pseudonym key (linkage key maintained separately)

**GDPR distinction:** Pseudonymized data is still personal data if reversal is possible. True anonymization requires irreversibility.

---

## Re-identification Risks

### Types of Attack

1. **Record linkage:** Combine de-identified dataset with external sources (voter registration, public records) to match individuals
2. **Attribute inference:** Statistical inference using correlated attributes (e.g., "people in zip X with university degree Y typically have salary Z")
3. **Membership inference:** Determine if a specific record is in the dataset (especially for rare combinations of attributes)

### Mitigation

- Publish aggregate statistics instead of individual records (surveys, counts, averages)
- Use geographic/temporal suppression (don't publish data for small regions where individuals are identifiable)
- Apply multiple anonymization methods (k-anonymity + generalization + DP noise)
- Restrict access (data use agreements, IRB review for research)

---

## Regulatory Context

### GDPR (EU, 2018)

Definition: Personal data can be "anonymized" if the individual is not identifiable "in isolation or in combination with other information." Process must be **irreversible.**

**Guidance:** Bare k-anonymity insufficient; must combine with l-diversity, t-closeness, or differential privacy.

**Reconciliation:** GDPR allows sharing anonymized data without consent. If reversibility exists (key retained), data remains personal and subject to consent/purpose limitation.

### CCPA (California, 2020)

Definition: Consumer "personal information" is broadly defined. Pseudonymized data is protected unless truly non-identifiable.

**Key difference from GDPR:** CCPA focuses on **consumer rights** (access, deletion, opt-out) rather than anonymization techniques.

---

## Practical Deployment

**Typical pipeline:**
1. Identify quasi-identifiers and sensitive attributes
2. Apply generalization (age groups, geographic regions)
3. Check k-anonymity (≥5 minimum)
4. If high-sensitivity data (health, financial): add l-diversity + t-closeness OR differential privacy
5. Audit for re-identification risks (linkage tests)
6. Use data governance: track lineage, access control, retention limits
7. Document anonymization process in data dictionary

**Tools:** ARX (open-source k-anonymity), OpenDP (differential privacy), Synthetic Data Vault (synthetic generation).

---

## See Also

- privacy-engineering.md (privacy principles, privacy by design)
- privacy-differential.md (detailed DP theory, mechanisms)
- security-compliance-frameworks.md (GDPR, CCPA, HIPAA technical requirements)
- data-engineering-governance.md (data lineage, access control)