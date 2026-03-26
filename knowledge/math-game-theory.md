# Game Theory for Computer Science — Strategic Interaction and Equilibrium

## Overview

Game theory is the mathematical study of strategic interaction between rational agents. Unlike optimization (controlling a single agent) or probability (random events), game theory models competitive or cooperative situations where each agent's outcome depends on others' choices. Applications span auctions, network protocols, resource allocation, incentive design, and economics—anywhere strategic behavior matters.

## Fundamentals: Games and Strategies

### Normal (Strategic) Form

A **normal form game** specifies:
- A finite set of players $N = \{1, 2, ..., n\}$
- For each player $i$, a set of strategies $S_i$ (actions available)
- A payoff function $u_i: S_1 \times S_2 \times ... \times S_n \to \mathbb{R}$ for each player

A strategy profile is a tuple $s = (s_1, s_2, ..., s_n)$ where each $s_i \in S_i$. The payoff to player $i$ under profile $s$ is $u_i(s)$.

Example (Prisoner's Dilemma):
```
           Cooperate    Defect
Cooperate  (-1, -1)     (-3, 0)
Defect     (0, -3)      (-2, -2)
```
Two players; each can Cooperate or Defect. Payoff pairs are (payoff to row player, payoff to column player).

### Pure and Mixed Strategies

A **pure strategy** is a deterministic choice: player $i$ selects $s_i \in S_i$ with certainty.

A **mixed strategy** is a probability distribution $\pi_i$ over $S_i$. If the strategy space is $\{s_i^1, s_i^2, ..., s_i^{m_i}\}$, a mixed strategy is $\pi_i = (p_1, p_2, ..., p_{m_i})$ where $p_j \geq 0$, $\sum_j p_j = 1$.

The expected payoff to player $i$ when others use mixed strategy profile $\pi_{-i} = (\pi_1, ..., \pi_{i-1}, \pi_{i+1}, ..., \pi_n)$ is:
$$U_i(\pi_i, \pi_{-i}) = \sum_{s \in S} \pi(s) u_i(s)$$

where $\pi(s) = \prod_{j=1}^{n} \pi_j(s_j)$.

## Best Response and Nash Equilibrium

### Best Response

Player $i$'s **best response** to opponents' strategy $\pi_{-i}$ is a strategy that maximizes payoff:
$$BR_i(\pi_{-i}) = \arg\max_{\pi_i} U_i(\pi_i, \pi_{-i})$$

The best response correspondence is the set of all best responses (may be multiple strategies yielding the same maximum payoff).

### Nash Equilibrium (Pure Strategy)

A strategy profile $s^* = (s_1^*, ..., s_n^*)$ is a **pure strategy Nash equilibrium** if for every player $i$ and every alternative strategy $s_i' \in S_i$:
$$u_i(s_i^*, s_{-i}^*) \geq u_i(s_i', s_{-i}^*)$$

Each player's strategy is a best response to others' strategies. No player benefits from unilaterally deviating.

Example (Matching Pennies):
```
         Heads   Tails
Heads    (1,-1) (-1,1)
Tails   (-1,1)  (1,-1)
```
No pure strategy Nash equilibrium exists (check: if row plays Heads, column prefers Tails, but then row prefers Tails, circular). Pure Nash equilibria exist when there's agreement on mutual advantage or strong asymmetry.

### Nash Equilibrium (Mixed Strategy)

A mixed strategy profile $\pi^*$ is a **Nash equilibrium** if for all players $i$ and mixed strategies $\pi_i$:
$$U_i(\pi_i^*, \pi_{-i}^*) \geq U_i(\pi_i, \pi_{-i}^*)$$

**Key theorem (Nash, 1950)**: Every finite game has at least one mixed strategy Nash equilibrium.

Computing mixed Nash equilibria for 2-player games reduces to solving systems of polynomial equations. For $n \geq 3$ or larger strategy spaces, computation becomes intractable (PPAD-complete in general).

## Dominant Strategies

A strategy $s_i \in S_i$ is **strictly dominant** for player $i$ if:
$$u_i(s_i, s_{-i}) > u_i(s_i', s_{-i})$$
for all $s_{-i}$ and all $s_i' \neq s_i$.

Player $i$ prefers $s_i$ regardless of others' choices.

A strategy is **weakly dominant** if the inequality is non-strict ($\geq$).

In games where every player has a dominant strategy, the **dominant strategy equilibrium** is when all play their dominant strategies—a strong prediction because no player has incentive to deviate, even knowing opponents' strategies.

Prisoner's Dilemma: "Defect" dominates "Cooperate" for both players, yet both prefer mutual cooperation. This tension highlights why dominant strategies lead to suboptimal group outcomes.

## Mechanism Design and Incentive Compatibility

### The Mechanism Design Problem

**Mechanism design** (or inverse game theory) asks: Design a game (mechanism) to achieve a desired outcome despite strategic agents with private information.

Setup:
- A set of possible outcomes $X$
- Players with valuations (preferences) $v_i$ over outcomes, privately known
- Mechanism specifies a mapping from reported valuations to outcomes and payments $t_i$ (transfers)

Goal: incentivize truthful reporting while achieving social objectives (efficiency, fairness, budget balance).

### Incentive Compatibility

A mechanism is **incentive compatible (IC)** if reporting truthfully is a dominant strategy:
$$u_i(x(v_i, v_{-i}), t_i(v_i, v_{-i}), v_i) \geq u_i(x(v_i', v_{-i}), t_i(v_i', v_{-i}), v_i)$$
for all $v_i, v_i', v_{-i}$.

The mechanism chooses outcomes and payments such that lying (reporting $v_i'$ instead of truths $v_i$) never improves payoff.

## Auction Theory

### Classic Auction Formats

**First-price sealed-bid auction**: bidders submit bids simultaneously; highest bidder wins and pays their bid.
- Dominant strategy: bid less than true valuation (shade bid to save on payment) — bidden amount depends on beliefs about others' valuations
- Not incentive compatible

**Second-price (Vickrey) auction**: highest bidder wins but pays the second-highest bid.
- Dominant strategy: bid truthfully (your valuation)
- Incentive compatible
- Revenue: typically below first-price equilibrium (winner pays less)

**Sealed-bid multi-item**: bidders submit bids for multiple items; allocation and payments determined by mechanism (e.g., VCG procedure).

### Vickrey-Clarke-Grooves (VCG) Mechanism

The VCG mechanism is a general framework for combinatorial auctions:
- Allocate items to maximize total value: $x^* = \arg\max_x \sum_i v_i(x_i)$
- Charge player $i$ the "externality cost": $t_i = \sum_{j \neq i} v_j(x_{-i}^*) - \sum_{j \neq i} v_j(x^*)$

where $x_{-i}^*$ is the optimal allocation excluding player $i$.

Property: VCG is incentive compatible and truthful. Truthful bidding is optimal for every player.

Drawback: VCG often results in payments less than costs (budget deficit) in many applications.

## Algorithmic Game Theory and Price of Anarchy

### Price of Anarchy (PoA)

When selfish agents optimize independently, the outcome is typically inefficient (lower total utility than centrally coordinated solution). The **Price of Anarchy** quantifies this inefficiency:
$$\text{PoA} = \frac{\text{Social Cost at Worst Nash Equilibrium}}{\text{Optimal Social Cost}}$$

For many games, PoA can be large: Prisoner's Dilemma has PoA = 2 (equilibrium yields half the cooperative payoff).

**Braess's Paradox**: adding roads to a traffic network can worsen equilibrium congestion (routes become less desirable as additional cars use them).

### Selfish Routing

In a network where agents route traffic selfishly (each minimizing personal latency), the Nash equilibrium routing can be significantly worse than optimal routing in terms of total latency.

Results from algorithmic game theory show bounds on PoA for specific network topologies and cost functions:
- Linear latency functions: PoA ≤ 4/3
- Polynomial cost functions of degree $d$: PoA grows with $d$

This analysis informs network design: routing protocols must either accept inefficiency or use pricing to incentivize socially optimal behavior.

## Matching Markets

### Stable Marriage Problem (Gale-Shapley Algorithm)

Two groups (men and women) each have strict preferences over the other group. A **matching** pairs each man with a woman (one-to-one).

A matching is **stable** if no man-woman pair would rather be with each other than their current partners (no blocking pair).

**Gale-Shapley algorithm**:
1. Each man proposes to his most-preferred woman
2. Each woman holds her most-preferred proposal, rejects others
3. Rejected men propose to their next-preferred woman
4. Repeat until no man is rejected

Terminates in at most $n^2$ rounds; produces a stable matching (always exists for any preference lists).

Property: solution favors proposers (men get best possible stable matching; women get worst).

### Applications Beyond Marriage

Stable matching models real systems:
- **Medical residency matching** (NRMP): hospitals and medical graduates
- **School choice**: students and schools (Boston mechanism reformed using stable principles)
- **Kidney exchange**: donors and recipients
- **Two-sided labor markets**: firms and workers

## Evolutionary Game Theory

### Evolutionary Stable Strategy (ESS)

In populations where individuals play a game repeatedly against random opponents, an **evolutionarily stable strategy** is one that resists invasion by mutant strategies.

Formally, a strategy $s^*$ is an ESS if:
- $u(s^*, s^*) \geq u(s, s^*)$ for all strategies $s$ (stability)
- If $u(s, s^*) = u(s^*, s^*)$, then $u(s^*, s) > u(s, s)$ (resistance to invasion)

ESS provides a prediction when Nash equilibrium exists but players don't have common knowledge (information asymmetry, limited rationality).

Example (Hawk-Dove): in a population, Hawk (aggressive) and Dove (retreat) coexist at equilibrium frequencies determined by fitness differences. Any pure strategy is invaded; the mixed ESS is stable.

## Game Theory in Networks

### Congestion Games

Agents choose resources (paths, servers); payoff decreases as usage increases.

**Rosenthal's theorem**: every finite congestion game has a pure strategy Nash equilibrium (can reach via iterative best response).

### Network Formation

Agents create links and derive utility from connections (positive) minus link costs (negative). Equilibrium network structures balance these.

Dense networks (more links) improve connectivity but increase costs; sparse networks reduce costs but isolate communities. The tension drives predictions about real social and infrastructure networks.

## Connections and Applications

**Incentive design** in blockchain (mining rewards, staking), crowdsourcing (payment mechanisms), cloud computing (resource allocation), and machine learning (federation, multi-agent systems) all rely on game-theoretic principles to align individual incentives with system objectives.

The challenge is that **dominant strategy equilibria are rare**; most games have multiple Nash equilibria or no pure equilibria. This multiplicity and the computational complexity of finding equilibria in large games remain active research areas.

See also: [algorithms-optimization](algorithms-optimization.md), [math-probability-foundations](math-probability-foundations.md), [networks-distributed-systems](networks-distributed-systems.md)