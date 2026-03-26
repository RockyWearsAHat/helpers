# Reinforcement Learning — MDPs, Value Methods, Policy Methods & Applications

Reinforcement learning (RL) trains agents to maximize cumulative reward by interacting with an environment. Unlike supervised learning (learn from labeled examples) or unsupervised learning (learn patterns), RL is defined by the **feedback signal**: the agent receives reward for actions and must discover which actions lead to high cumulative reward. This makes RL suitable for sequential decision-making in interactive environments.

## Markov Decision Processes (MDPs)

An MDP formalizes the RL setting: a tuple $(\mathcal{S}, \mathcal{A}, \mathcal{P}, \mathcal{R}, \gamma)$, where:
- **$\mathcal{S}$**: Set of states (agent's observation of the environment)
- **$\mathcal{A}$**: Set of actions (agent's choices)
- **$\mathcal{P}(s' | s, a)$**: Transition probability (probability of reaching state $s'$ after taking action $a$ in state $s$)
- **$\mathcal{R}(s, a, s')$**: Reward function (immediate reward for transitioning from $s$ to $s'$ via action $a$)
- **$\gamma \in [0, 1]$**: Discount factor (how much future rewards are valued relative to immediate rewards)

The agent's goal is to **maximize expected cumulative discounted reward**: $\mathbb{E}[\sum_{t=0}^{\infty} \gamma^t r_t]$, where $r_t$ is reward at timestep $t$.

### Markov Property
The "Markov" property means the future depends only on the current state, not on history: $\mathbb{P}(s_{t+1} | s_t, a_t) = \mathbb{P}(s_{t+1} | s_0, a_0, \ldots, s_t, a_t)$. This is a strong assumption; many real-world environments are **partially observable** (the agent doesn't see the full state), violating the Markov property. Extensions like **POMDPs** (Partially Observable MDPs) model this but add complexity.

### Policies and Value Functions
A **policy** $\pi(a | s)$ is a mapping from states to actions (stochastic policy) or a deterministic choice $a = \pi(s)$. The **value function** $V^\pi(s)$ measures the expected cumulative reward starting from state $s$ and following policy $\pi$:

$$V^\pi(s) = \mathbb{E}[\sum_{t=0}^{\infty} \gamma^t r_t | s_0=s, \pi]$$

The **optimal value function** $V^*(s) = \max_\pi V^\pi(s)$ is the highest cumulative reward achievable from $s$.

The **action-value function** or **Q-function** $Q^\pi(s, a)$ measures expected cumulative reward for taking action $a$ in state $s$, then following policy $\pi$:

$$Q^\pi(s, a) = \mathbb{E}[\sum_{t=0}^{\infty} \gamma^t r_t | s_0=s, a_0=a, \pi]$$

The **optimal Q-function** $Q^*(s, a) = \max_\pi Q^\pi(s, a)$ defines an optimal policy: $\pi^*(s) = \arg\max_a Q^*(s, a)$.

## Q-Learning: Off-Policy Temporal Difference Learning

Q-learning learns the optimal Q-function via **temporal difference (TD) updates**. In state $s$, take action $a$, receive reward $r$ and next state $s'$, then update:

$$Q(s, a) \leftarrow Q(s, a) + \alpha [r + \gamma \max_{a'} Q(s', a') - Q(s, a)]$$

where $\alpha$ is the learning rate and $[r + \gamma \max_{a'} Q(s', a') - Q(s, a)]$ is the **TD error**.

**Off-policy**: Q-learning learns the optimal policy (greedy over max Q-values) while following a different policy (often $\epsilon$-greedy: mostly greedy, sometimes explore randomly). This allows learning from exploration data without committing to a specific behavior policy.

**Value iteration in disguise**: Q-learning is incremental value iteration. Given enough samples, it converges to $Q^*$ (under conditions: tabular representation, exploration ensures all state-action pairs are visited, learning rate schedule decreases appropriately).

### Limitations
Q-learning is a **tabular method**: it maintains a table of Q-values for each state-action pair. This is infeasible for large or continuous state/action spaces. It also requires explicit exploration (the agent must try actions to learn their value), which can be inefficient.

## Deep Q-Networks (DQN)

DQN extends Q-learning to high-dimensional state spaces (e.g., images) by approximating Q-values with a neural network $Q(s, a; \theta)$, where $\theta$ are network parameters. This enables RL on complex domains like Atari games.

### Key Innovations

**Experience Replay**: Store past experiences $(s, a, r, s')$ in a replay buffer. During training, sample mini-batches from the buffer instead of learning from the current experience stream. This (1) breaks temporal correlation (mini-batch contains diverse timesteps), (2) enables reuse of data, and (3) stabilizes learning (TD targets don't depend only on recent data).

**Target Network**: Use a separate network $Q(s, a; \theta^-)$ to compute TD targets, updated infrequently (every $N$ steps) to the main network's parameters. This decouples the TD target from the current network, reducing instability. Without it, the TD target $r + \gamma \max_{a'} Q(s', a'; \theta)$ changes every update, causing the network to chase a moving target.

**Reward Clipping**: Clip rewards to a fixed range (e.g., $[-1, 1]$) to normalize scale across environments and stabilize training.

### Training Dynamics
Train by minimizing the **Huber loss** (robust to outliers unlike squared error):
$$L(\theta) = \mathbb{E}[(r + \gamma \max_{a'} Q(s', a'; \theta^-) - Q(s, a; \theta))^2]$$

Beyond vanilla DQN, improvements include:
- **Double DQN**: Use the main network to select $a' = \arg\max_a Q(s', a'; \theta)$ but the target network to evaluate $Q(s', a'; \theta^-)$. This reduces overestimation bias in TD targets.
- **Dueling DQN**: Decompose $Q(s, a) = V(s) + A(s, a) - \bar{A}(s)$, where $V(s)$ is the value of state $s$ (independent of action) and $A(s, a)$ is the advantage of action $a$. This decomposition improves learning efficiency.

### Limitations
DQN is unstable in some domains. Learning can diverge. It also requires **discrete action spaces** (argmax over actions). Extensions like **continuous control algorithms** (below) address continuous actions.

## Policy Gradient Methods

Instead of learning values, policy gradients directly optimize the policy parameters $\theta$. The goal is to maximize expected return:

$$J(\theta) = \mathbb{E}_{s \sim \rho^\pi, a \sim \pi(s)} [r(s, a)]$$

where $\rho^\pi(s)$ is the stationary distribution of states visited under $\pi$.

### REINFORCE
REINFORCE uses the **policy gradient theorem**: $\nabla J(\theta) = \mathbb{E}[Q(s, a) \nabla \log \pi(a | s; \theta)]$. Given a trajectory, update:

$$\theta \leftarrow \theta + \alpha \nabla \log \pi(a_t | s_t; \theta) G_t$$

where $G_t = \sum_{k=t}^{\infty} \gamma^{k-t} r_k$ is the return from timestep $t$ onward. Intuitively: increase the probability of actions that led to high returns.

**Advantages**: Uses only the observed trajectory (no model needed), applies to stochastic policies and continuous actions.

**Disadvantages**: High variance (return $G_t$ varies significantly across episodes). Small stochastic rewards in early timesteps lead to large gradient noise. Convergence is slow; many samples are needed.

### Actor-Critic Methods
Actor-Critic uses two networks: (1) **actor**: policy $\pi(a | s; \theta)$, and (2) **critic**: value function $V(s; \phi)$. The critic provides a **baseline** for the actor, reducing variance.

Update the actor:
$$\theta \leftarrow \theta + \alpha \nabla \log \pi(a | s; \theta) [r + \gamma V(s'; \phi) - V(s; \phi)]$$

The bracket is the **TD error** (advantage), which is lower-variance than the full return. Update the critic to minimize $(r + \gamma V(s'; \phi) - V(s; \phi))^2$.

**Advantages**: Lower variance than REINFORCE, sample-efficient.

**Disadvantages**: Requires tuning two networks (actor and critic). The critic must be accurate for the advantage estimate to be reliable.

### A3C (Asynchronous Advantage Actor-Critic)
A3C runs multiple actors in parallel, each interacting with a separate copy of the environment and accumulating gradients. Periodically, all gradients are aggregated, and the central network is updated. This parallelism reduces temporal correlation (each worker sees different trajectories) and speeds up training on multi-core systems.

### PPO (Proximal Policy Optimization)
PPO improves upon A3C by using a simpler, more stable algorithm. Instead of actor-critic, PPO optimizes a **clipped objective**:

$$L(\theta) = \mathbb{E}[r_t(\theta) \hat{A}_t - \beta \text{KL}[\pi_{\text{old}}, \pi_\theta]$$

where $r_t(\theta) = \frac{\pi(a_t | s_t; \theta)}{\pi(a_t | s_t; \theta_{\text{old}})}$ is the importance weight, $\hat{A}_t$ is a bootstrapped advantage (from the value function), and the KL term prevents the new policy from diverging too much from the old policy. This is simpler than A3C's async coordination and more stable than vanilla policy gradient.

PPO is currently one of the most popular algorithms: it's sample-efficient, stable, and scales to complex environments (language models, robotic control).

## Exploration vs. Exploitation

Agents face a **trade-off**: exploit known good actions (actions with high estimated value) or explore unknown actions (to discover potentially better ones). This is the **exploration-exploitation dilemma**.

### Strategies
- **$\epsilon$-greedy**: With probability $\epsilon$, pick a random action; otherwise, pick the greedy action. Simple and common.
- **Decay-based**: Start with high $\epsilon$ (exploration), decay over time ($\epsilon \propto 1/t$) to converge to greedy policy.
- **UCB (Upper Confidence Bound)**: Favor actions with high estimated value or high uncertainty. Uncertainty decays as actions are sampled, naturally balancing exploration and exploitation.
- **Thompson Sampling**: Sample an action from a posterior distribution over value estimates. Explores with probability proportional to the action's probability of being optimal (intelligent exploration).

The **regret** of an exploration strategy is the total reward given up by not following the optimal policy. Theoretical analysis (multi-armed bandit literature) shows that logarithmic regret is optimal, achievable by decay-based and UCB strategies.

## Multi-Armed Bandits

A **k-armed bandit** is a simplified MDP with a single state and $k$ actions (arms). Each arm has an unknown reward distribution; the agent pulls arms sequentially and observes rewards, trying to maximize cumulative reward. This is the canonical exploration-exploitation problem.

**Regret bounds**: For many algorithms (UCB, Thompson sampling), the best achievable regret is $O(\log T)$, where $T$ is the number of interactions. This means on average, the agent's loss per timestep decreases as $O(\log T / T) \to 0$ with more interactions.

Contextual bandits extend this: each timestep starts with a context (feature vector), and the reward distribution depends on both the context and chosen action. This is intermediate between bandits (single state) and full MDPs (rich state/reward structure). Example: recommending articles given user context.

## Reward Shaping

In practice, specifying the reward function is hard. A hand-crafted reward can be uninformative (agent learns slowly) or can mislead the agent (agent learns to game the reward rather than achieve the intended goal).

**Reward shaping** adds auxiliary rewards to guide learning without changing the optimal policy. The shaped reward is $r_{\text{shaped}}(s, a, s') = r(s, a, s') + F(s, s')$, where $F(s, s')$ is an auxiliary reward depending on states but not directly on actions.

**Potential-based reward shaping**: If $F(s, s') = \gamma \phi(s') - \phi(s)$ (potential function $\phi$), the optimal policy is unchanged. This is because the auxiliary rewards telescope: the agent's cumulative auxiliary reward over a trajectory is path-independent (just $-\phi(s_0) + \gamma^T \phi(s_T)$), so it doesn't distort the relative rankings of trajectories.

**Demonstration-based shaping**: Include reward for matching demonstrations or imitating expert behavior, accelerating learning. This is a form of transfer learning (knowledge from experts transferred to the RL agent).

## Applications & Domain Patterns

**Game playing**: Deep Q-Networks beat human experts at Atari. Policy gradient methods (A3C, PPO) master complex games (Dota 2, StarCraft II). Success factors: dense reward signal (game score is immediate feedback), large compute budgets (millions of simulations), ability to reset and replay.

**Robotics**: RL trains robotic arms to grasp objects, walk, or manipulate tools. Challenges: real-world experiments are slow and dangerous. Solutions: sim-to-real transfer (train in simulation, adapt to real world), curriculum learning (start with easy tasks, progress to hard ones), reward shaping to reflect physical constraints.

**Trading**: RL optimizes trading strategies given market data. Challenges: non-stationary environments (market regimes shift), enormous action spaces (continuous position sizing, asset selection), and high-dimensional state spaces (market microstructure, news). Research focus: transfer learning from simulation, ensemble methods to reduce overfitting.

**Recommender systems**: RL balances **immediate reward** (recommending items the user clicks) and **long-term value** (recommending items that keep the user engaged long-term). Contextual bandits are common; full MDPs add complexity (long-horizon planning for engagement).

**Control**: Continuous action robotics, HVAC tuning, autonomous driving. PPO and SAC (Soft Actor-Critic, combining actor-critic with entropy regularization) are popular due to stability and sample efficiency.

## Challenges & Open Problems

**Sample efficiency**: RL is sample-hungry. A human learns to drive a car in ~50 hours; RL agents need millions of simulated interactions. Improvements: offline RL (learn from logged data), meta-learning (learn how to learn), imitation learning (bootstrap from demonstrations).

**Exploration in sparse reward settings**: If reward is only given at the end of an episode (or very rarely), the agent struggles to discover good trajectories. Hierarchical RL (break tasks into subtasks with intermediate rewards) and curiosity-driven exploration (reward exploration itself) are active research areas.

**Non-stationarity and concept drift**: Real environments change over time. Agents must detect distribution shifts and adapt. Continual RL (learn on a stream of tasks without forgetting previous knowledge) is in early stages.

**Sim-to-real transfer**: Simulation enables cheap learning, but learned policies often fail in the real world due to differences in physics, visual perception, and dynamics. Domain randomization (train on diverse simulated environments) and domain adaptation (fine-tune on real data) are practical approaches.