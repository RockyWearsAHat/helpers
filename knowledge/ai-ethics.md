# AI Ethics for Developers

## Overview

AI ethics is the study of fairness, transparency, accountability, and safety in machine learning systems. For developers, it means understanding where bias enters models, how to measure fairness, making models interpretable, implementing responsible practices, and navigating regulation (EU AI Act). Unlike abstract philosophy, applied AI ethics is about concrete technical choices: feature selection, training data curation, evaluation metrics, and deployment safeguards.

The challenge: there's no single "fair" definition. Different fairness metrics conflict. What's fair to one stakeholder harms another. Developers must document assumptions, measure trade-offs, and be prepared to explain decisions to users, auditors, and regulators.

---

## Bias: Sources and Detection

**Where bias enters:**

1. **Historical bias** — Training data reflects past discrimination.
   - Example: resume screening model trained on historical hires biased against women (historical underrepresentation in tech).
   - *Detection:* Compare model performance across demographic groups; if women have lower acceptance rates than men with identical resumes, the model is perpetuating historical bias.

2. **Aggregation bias** — Treating diverse populations as homogeneous.
   - Example: single default interest rate model for all users; in reality, fairness may require different treatment for different credit profiles.
   - *Mitigation:* Stratified analysis; test model performance on subgroups.

3. **Representation bias** — Underrepresented groups in training data.
   - Example: face recognition trained 80% on Western faces; poor performance on darker skin tones (documented: Buolamwini & Gebru, 2018).
   - *Mitigation:* Stratified data collection; oversample minority groups; weight loss function to penalize errors on underrepresented populations.

4. **Measurement bias** — Proxy variables correlate with protected attributes.
   - Example: zip code as a feature correlates with race; not explicitly using race, but implicitly proxying it.
   - *Detection:* Correlation analysis; SHAP feature importance; test for proxy variables.
   - *Mitigation:* Remove the proxy; if necessary, use adversarial debiasing to remove predictive power for protected attributes.

5. **Evaluation bias** — Metrics don't capture real-world harm.
   - Example: accuracy alone hides disparities. A model could be 90% accurate overall but 70% accurate for a minority group.
   - *Mitigation:* Use fairness-aware metrics (below).

6. **Deployment bias** — Model performs differently in production than in test.
   - Example: model trained on users of one platform; deployed to users of another platform with different behavior.
   - *Mitigation:* Continuous monitoring; stratify performance by user segment; retrain regularly.

---

## Fairness Metrics

**Key constraint: no metric satisfies all fairness definitions simultaneously.**

Choose the metric that aligns with your use case:

### 1. Demographic Parity
**Definition:** Model predictions independent of protected attribute.

Formula: $P(\hat{Y}=1|A=0) = P(\hat{Y}=1|A=1)$ (acceptance rate equal across groups A)

**Use case:** Hiring, admissions where you want proportional representation.

**Limitation:** Ignores actual qualifications; can enforce "equal incompetence."

### 2. Equalized Odds
**Definition:** False positive rate AND false negative rate equal across groups.

Formula: $P(\hat{Y}=1|Y=1, A=0) = P(\hat{Y}=1|Y=1, A=1)$ (true positive rate equal)
AND $P(\hat{Y}=1|Y=0, A=0) = P(\hat{Y}=1|Y=0, A=1)$ (false positive rate equal)

**Use case:** Criminal justice, medical diagnosis (errors harm both ways; don't accept false positives for one group but not another).

**Limitation:** Requires labeled data for both groups; assumes true labels are ground truth (which may itself be biased).

### 3. Calibration
**Definition:** When model predicts 50% probability for group A and 50% for group B, both should have ~50% true positive rate.

**Use case:** Medical risk scores, loan default predictions; decision-makers need reliable probability estimates.

**Limitation:** Doesn't guarantee equal treatment; two groups can have same calibration but different baseline rates.

### 4. Predictive Parity (Precision Parity)
**Definition:** Given a positive prediction, precision (% actually positive) equal across groups.

Formula: $P(Y=1|\hat{Y}=1, A=0) = P(Y=1|\hat{Y}=1, A=1)$

**Use case:** Content moderation, fraud detection (positive predictions carry consequences; want same confidence across groups).

**Limitation:** Can conflict with equalized odds.

### 5. Individual Fairness
**Definition:** Similar individuals get similar predictions.

**Use case:** Interpretability; "explain why you got a different rate than your neighbor" demands similar decisions for similar inputs.

**Limitation:** Requires defining similarity; in high dimensions, most points are dissimilar.

### Trade-off Example
- **Hiring model:** Demographic parity (hire same % from each group) vs. equalized odds (false rejection rates equal) will conflict if true candidate quality differs. Choose based on organizational values.
- **Loan approval:** Calibration (your 50% prediction is right 50% of time for both groups) doesn't guarantee someone isn't systematically rejected; equalized odds ensures rejection rates are fair.

**Developer responsibility:** Document which metric you're optimizing for, why, and what you're not measuring.

---

## Model Interpretability

Users, auditors, and regulators demand explanations: "Why did you deny my loan?" Black-box accuracy isn't enough.

### LIME (Local Interpretable Model-Agnostic Explanations)

**How it works:**
1. Pick a prediction you want to explain (e.g., "your loan was denied").
2. Perturb inputs slightly (change age, income, credit score, one at a time).
3. Collect new predictions for perturbed inputs.
4. Fit a simple linear model on perturbations → shows which features drove the prediction locally.

**Output:** "Your income (−0.4) and credit utilization (+0.2) were the largest factors."

**Strength:** Works on any model; intuitive.

**Limitation:** Explains one prediction, not the model globally; depends on perturbation strategy.

**Developer responsibility:** Implement LIME for high-stakes decisions. Show users.

### SHAP (SHapley Additive exPlanations)

**How it works:**
1. Compute the contribution of each feature to the prediction using Shapley values from game theory.
2. Fair allocation: feature i's contribution = prediction - (prediction without feature i), averaged over all permutations.

**Output:** Feature importance scores; "feature X added 0.3 to your risk score" (global) or "feature X added 0.15 to your loan denial" (local).

**Strength:** Theoretically sound; global + local explanations; visualizations (dependency plots, feature interaction).

**Limitation:** Computationally expensive for large models; assumes feature independence (not always true).

**Developer responsibility:** Use SHAP for model debugging and user-facing explanations.

### Other Approaches

- **Attention weights (neural networks):** Which input tokens the model focused on.
- **Surrogate model:** Train a simple (interpretable) model to approximate the complex one; use the simple model to explain.
- **Counterfactual:** "Your loan was denied; if your income was $50k higher, you'd have been approved."
- **Feature importance (tree-based models):** Gini/entropy decrease when splits on a feature.

---

## Responsible AI Frameworks

### Microsoft / AI Ethics Board Approach

**Stages:**
1. **Define fairness**: Stakeholder workshop; whose fairness matters (applicants, hiring managers, company)? What trade-offs are acceptable?
2. **Measure bias**: Stratify evaluation by demographic groups; compute multiple fairness metrics.
3. **Mitigate:** Rebalance training data, adjust decision thresholds per group, or retrain with fairness-aware losses.
4. **Monitor:** In production, track performance drift by group; set alerts.
5. **Document:** Model card (intended use, model performance by race/gender/age, failure modes, recommendations).

### Responsible AI Toolkit (ML Commons)

**Pillars:**
- **Fairness:** Stakeholder input, bias testing, metric selection
- **Safety:** Adversarial robustness, out-of-distribution detection, failure modes
- **Transparency:** Model cards, documentation, explainability
- **Accountability:** Access controls, audit trails, incident response
- **Privacy:** Data minimization, differential privacy, federated learning

---

## EU AI Act: Risk Categories & Compliance

The EU AI Act (effective 2025-2026) classifies AI systems by risk:

### Prohibited Risk (Not Allowed)
- Social scoring (ranking citizens by trustworthiness)
- Manipulation (e.g., exploiting vulnerabilities to change behavior)
- Biometric identification in public spaces (real-time, without compelling need)

**Developer responsibility:** Don't build prohibited systems.

### High-Risk (Requires Compliance)
- Recruitment/promotion filtering
- Resume screening for job applicants
- Credit/insurance decisions
- Biometric identification in sensitive contexts (identity verification only)
- Law enforcement (predictive policing)
- Border/migration decisions
- Child safety (content moderation, age estimation)

**Compliance obligations:**
- Risk assessment (document harms, mitigations)
- Data governance (bias monitoring, data quality)
- Model card or algorithmic impact assessment (AIA)
- Transparency: inform users they're being evaluated by AI
- Override capability: human can override AI decision
- Audit trail: log decisions, can be reviewed
- Regular testing & monitoring; document incidents

**Developer responsibility:**
- Generate model cards with performance by demographic groups
- Implement decision logging (who, what, when, outcome)
- Provide override UI for customer service / compliance teams
- Test for robustness (adversarial inputs, concept drift)

### Limited-Risk (Transparency)
- Deepfake/synthetic media: disclose when content is AI-generated
- Chatbots: disclose interaction is with AI (no impersonation)

**Developer responsibility:** Label outputs clearly.

### Minimal/No Risk
- Spam detection
- Grammar checkers
- Recommendation systems
- Most internal tools

---

## Data Privacy in ML

**PII in training data:**

ML models memorize training data. If training set contains personally identifiable information (addresses, emails, financial records), a language model can be prompted to regurgitate it.

**Mitigation:**
- Redact PII before training (remove names, emails, credit cards, SSNs)
- Use differential privacy: add noise to training data, guarantee individual records can't be inferred from model
- Federated learning: train model locally on each user's device; aggregate gradients, never transfer raw data
- Regular privacy audits: prompt model with known PII; measure if it leaks

---

## Content Moderation & Deepfake Detection

### Content Moderation

**Challenge:** Scale human judgment to billions of posts.

**Approaches:**
1. **Rule-based:** Keyword matching, regular expressions (fast, brittle).
2. **Supervised ML:** Train classifier on human-labeled examples (scaling issue: new slurs, context-dependent).
3. **Hybrid:** ML flags content; humans review (human-in-the-loop).

**Bias risk:** Moderation models trained on labeled data reflect labeler preferences. If labelers are homogeneous, decisions will be skewed.

**Mitigation:**
- Diverse labeling team (geographic, cultural diversity)
- Stratified accuracy (measure performance on different content types; don't accept overall accuracy if minority languages/dialects are misclassified)
- Regular retraining (language evolves)
- Appeals process (users can request human review)

### Deepfake Detection

**Challenge:** Detect manipulated media (face swaps, voice clones, synthetic video).

**Signal:**
- Biometric inconsistencies (eye blink patterns, facial micro-expressions)
- Compression artifacts (deepfakes sometimes reveal compression mismatches)
- Audio-visual sync (lip movement vs. speech)
- Spectral analysis (frequency domain anomalies)

**Limitation:** Adversaries are also improving; detection is an arms race.

**Mitigation:**
- Media provenance: cryptographic signing of original (hard to deploy at scale)
- Watermarking: embed hidden signal in authentic media
- Disclosure: "This content was AI-generated" metadata
- Hybrid: ML detection + human review for high-stakes (elections, breaking news)

---

## Algorithmic Transparency & Accountability

**Model Card (Model Audit Worksheet, Google):**

Standard template for documenting a model:
- **Model details**: developers, date, version, license
- **Intended use**: primary use case; out-of-scope uses
- **Data**: source, size, preprocessing, demographic breakdown
- **Performance**: accuracy, precision, recall by demographic group; failure modes
- **Limitations**: when performance degrades; special populations not tested
- **Ethical considerations**: bias discovered, recommendations for mitigation

**Algorithmic Impact Assessment (AIA):**
- Stakeholders: Who is affected? (applicants, employees, customers, public)
- Decision: What is being decided? (hired, approved, ranked, recommended)
- Data: What information is used? (intentional, proxies)
- Trade-offs: What fairness definition are you optimizing for? What are you sacrificing?
- Monitoring: How will you detect failures in production?

---

## Decision Tree: When to Care About Ethics

```
Is this model used for decisions about people? (eligibility, ranking, classification)
├─ YES → Fairness audit required
│   ├─ High-risk (hiring, credit, criminal justice): Full compliance
│   │   ├─ Measure equalized odds or demographic parity
│   │   ├─ Generate model card
│   │   ├─ Make decision explainable (LIME/SHAP)
│   │   ├─ Implement decision logging
│   │   └─ Set up monitoring for performance drift
│   │
│   └─ Standard risk (product recommendations, content ranking): At minimum
│       ├─ Test for disparate impact (stratified evaluation)
│       └─ Document known limitations
│
└─ NO → Less urgent, but still consider
    └─ Privacy (data usage), robustness (adversarial inputs), disclosure (is model-generated content labeled?)
```

---

## Developer Checklist

- [ ] Identify protected attributes (race, gender, age, disability, etc.)
- [ ] Collect stratified evaluation metrics; not just overall accuracy
- [ ] Audit training data: remove/redact PII; check for historical bias
- [ ] Test model on subgroups; flag disparate impact
- [ ] Choose fairness metric (demographic parity, equalized odds, etc.); document trade-offs
- [ ] Implement explainability (LIME/SHAP for high-stakes)
- [ ] Generate model card: performance by demographic group, failure modes
- [ ] If EU AI Act high-risk: impact assessment, decision logging, human override
- [ ] Monitor in production: stratified performance metrics, detect drift
- [ ] Document assumptions: fairness definition, trade-offs accepted, limitations
- [ ] Incident response: if bias discovered, have process to retrain/mitigate
- [ ] Communicate with stakeholders: be transparent about limitations