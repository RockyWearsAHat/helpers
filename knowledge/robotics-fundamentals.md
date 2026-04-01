# Robotics Fundamentals — ROS/ROS2, SLAM, Path Planning, Control, Sensor Fusion, Sim-to-Real

## Overview

Robotics software integrates multiple disciplines: real-time sensing, state estimation, decision-making, and actuation. The field's complexity arises from operating in continuous, uncertain physical environments where timing guarantees and sensor noise directly impact safety and performance. Modern robotics relies on middleware (ROS/ROS2), algorithms for spatial reasoning (SLAM, path planning), and system-level skills (sensor fusion, control, sim-to-real transfer). These tools address the gap between theoretical algorithms and physical machines.

See also: [algorithms-graph.md](algorithms-graph.md), [os-concurrency-primitives.md](os-concurrency-primitives.md).

## ROS and ROS2 — Middleware Architecture

### ROS1: The Original Framework

ROS (Robot Operating System) started as a peer-to-peer middleware enabling message passing between **nodes** (processes) running on one or more machines.

**Core abstractions**:

1. **Node**: Executable process (C++, Python) doing specific task (reading sensor, planning path, controlling motor).
2. **Topic**: Named channel for asynchronous publish-subscribe communication. Multiple publishers/subscribers; messages bypassed if no subscribers.
3. **Service**: Request-reply synchronous communication. Client waits for server response; blocking.
4. **Action**: Goal-oriented communication with feedback. Client sends goal; server publishes progress updates and eventually a result.
5. **Parameter Server**: Central key-value store for configuration; accessible globally.

**Message definition**: Plain-text `.msg` files define serializable data types (structs). Code generators create C++ and Python classes.

**Graph-based organization**: Nodes form a computation graph; dependency and data-flow are explicit via connections.

**Ecosystem**: Hundreds of packages (tf2 for coordinate transformations, rviz for visualization, gazebo for simulation). Extensive tutorials and community support.

**Limitations**:

- **Real-time unfriendly**: TCP for topics; no hard timing guarantees.
- **Single point of failure**: ROS Master (central registry) crash breaks entire system.
- **Limited security**: No authentication, encryption, or access control built-in.
- **Python 2 legacy**: ROS1 remained on Python 2 maintenance mode briefly after Python 2 EOL.

### ROS2: Modern Redesign

ROS2, released ~2017 (matured by 2020), addressed ROS1's architectural issues.

**Key improvements**:

1. **DDS (Data Distribution Service)**: Replaces custom middleware. DDS is an OMG standard, distributed, with QoS policies.
2. **No central master**: Decentralized discovery via DDS.
3. **Explicit QoS**: Publishers/subscribers declare Quality of Service (reliability, timeliness, history).
   - **Best-effort vs. Reliable**: Message loss tolerance.
   - **Volatile vs. Transient Local History**: Whether late subscribers get latest message.
   - **Deadline**: Maximum acceptable latency.
4. **Middleware abstraction**: Support multiple DDS vendors (FastDDS, RTI Connext, etc.).
5. **Security**: Built-in DTLS encryption, authentication; access control via policy files.
6. **Real-time support**: DDS enables predictable latency for hard real-time tasks (not guaranteed, but better architecture).

**Trade-off**: DDS complexity and licensing (open-source FastDDS, commercial alternatives). Larger RAM/CPU overhead than ROS1.

### Topics, Services, Actions

**Topics** (async): Best for continuous data streams (sensor readings, status). No expectation of explicit response.

```
Publisher (sensor):    Image, Image, Image, ... (stream)
Subscriber (planner):  consumes stream asynchronously
```

**Services** (sync): Request-reply for one-off queries. Client blocks.

```
Client:   request(start_x, start_y, goal_x, goal_y)
Server:   [compute path]
Server:   respond(path)
Client:   [unblocks]
```

**Actions** (goal + feedback): Long-running tasks with progress updates.

```
Client:   goal(target_position)  -> Server knows goal
Server:   feedback(current_position, eta, status)
Client:   [receives feedback asynchronously]
Server:   result(final_position, success)
```

Choosing correctly impacts architecture: topics for real-time sensing, services for synchronous queries (less common in robotics), actions for manipulation, navigation, long-running task orchestration.

## SLAM — Simultaneous Localization and Mapping

### The Problem

A robot navigates an unknown environment without GPS, starting from an unknown location. It needs:

1. **Localization**: Where am I relative to the environment?
2. **Mapping**: What does the environment look like?

These are coupled: a good map helps localization (recognizing landmarks), and accurate localization enables consistent mapping. SLAM solves them simultaneously.

### EKF-SLAM (Extended Kalman Filter)

Early SLAM approach; intuitive but limited by computational cost.

**State representation**: Robot pose $(x, y, \theta)$ + positions of $N$ landmarks.

**Sensor model**: Robot observes landmarks within range, receiving noisy bearing and distance.

**EKF update**:

1. **Predict**: Dead reckoning — advance robot pose using odometry (wheel encoder velocities, gyro).
2. **Observe**: Measure landmarks (laser scan, camera features). Match observations to map.
3. **Update**: EKF correction step adjusts estimate based on observation residual.

**Computational complexity**: $O(n^2)$ where $n$ is number of landmarks. Limits scalability to moderate-sized maps (~1000 landmarks).

**Assumption**: Gaussian noise; linear motion/observation models (EKF linearizes). Weak in highly nonlinear scenarios.

### GraphSLAM and Pose Graphs

Modern approach; scales to large environments by modeling SLAM as **optimization over a graph**.

**Representation**: Nodes are robot poses at time steps $t_0, t_1, \ldots$. Edges are **constraints**:

- **Odometry edge**: Relative motion between consecutive poses (from wheel encoders, IMU).
- **Loop-closure edge**: Observation that two distant poses see the same landmark (detected via image feature matching, place recognition).

**Problem formulation**: Find poses minimizing constraint violation (residuals):

$$\min_{\mathbf{poses}} \sum_{\text{edges}} \|\mathbf{constraint}\|_{\Sigma}^2$$

Weighted least-squares over all constraints.

**Advantages**:

- **Scalable**: $O(n)$ computation per optimization iteration.
- **Loop closure natural**: Solving globally integrates new information (loop closure) into entire map retroactively.
- **Nonlinear optimization available**: Gauss-Newton, Levenberg-Marquardt handle nonlinear models.

**Tools**: g2o (General Graph Optimization), GTSAM, Ceres Solver.

### Sensor-Specific SLAM

**Lidar/Laser SLAM**:

- **ICP (Iterative Closest Point)**: Align consecutive laser scan pairs by minimizing distance between point clouds.
- **High precision**: Lidar points directly represent environment geometry.
- **Cost**: Lidar sensors expensive; large data volume.

**Visual SLAM (camera)**:

- **Feature detection/matching**: Extract keypoints (SIFT, ORB), match across frames.
- **Structure-from-motion**: Triangulate 3D positions of matched features from camera geometry.
- **Scales up**: Cheaper cameras, open source (ORB-SLAM, OpenVSLAM).
- **Challenges**: Featureless environments, dynamic lighting, computational load of keyframe optimization.

**RGB-D (depth camera)**:

- Combines color and depth; easier feature matching than monocular, cheaper than lidar.
- **Drift**: Depth camera noise accumulates; loop closure essential.

## Path Planning and Navigation

### Global Path Planning

**Goal**: Compute a sequence of waypoints from start to goal, considering obstacles.

#### A* Search

Heuristic search over a discretized environment (grid or graph).

**Algorithm**:

1. Maintain open set (frontier) of nodes to explore, closed set (visited).
2. Always expand node with lowest $f(n) = g(n) + h(n)$ where $g(n)$ is cost from start, $h(n)$ is heuristic estimate to goal.
3. If goal reached, reconstruct path.

**Heuristic choice**:

- **Admissible heuristic**: $h(n) \leq$ true cost to goal. Guarantees optimal path.
- **Euclidean distance**: Common for 2D grids; admissible.
- **Manhattan distance**: Admissible for grids; often faster.

**Complexity**: $O(b^d)$ where $b$ is branching factor, $d$ is depth. Real-time performance depends on grid resolution and environment clutter.

**Assumption**: Static obstacles; no dynamic constraints (timing, vehicle dynamics).

#### Rapidly-Exploring Random Tree (RRT)

Sampling-based planning; addresses continuous spaces and differential constraints.

**Intuition**: Randomly sample configuration space; grow tree toward samples; check collision-free paths.

```
tree = [start]
for i = 1 to N:
    random_config = random_point_in_configuration_space()
    nearest = nearest_node_to(random_config, tree)
    new_node = steer(nearest, random_config, step_size)
    if collision_free(nearest, new_node):
        tree.add(new_node)
        if goal_reached(new_node):
            return path
```

**Characteristics**:

- **Probabilistically complete**: As $N \to \infty$, probability of finding a path approaches 1 (if one exists).
- **Not optimal**: Path is often longer than necessary.
- **Handles dynamics**: Steering function can incorporate vehicle model (e.g., Dubins curves for cars).

**Variants**:

- **RRT***: Rewiring neighbors to optimize path cost; asymptotically optimal.
- **Bidirectional RRT**: Grow trees from start and goal; meet in middle.

### Local Planning and Obstacle Avoidance

**Problem**: Real-time obstacle avoidance as robot executes path; recovery when obstacles not in static map.

#### Dynamic Window Approach (DWA)

Locally searches for safe velocities by simulating short trajectories forward.

**Algorithm**:

1. Sample velocity space $(v_x, v_y, \omega)$ within robot's acceleration limits.
2. Simulate each velocity for 0.5–1s; predict future robot position.
3. Evaluate trajectory using cost function: distance to obstacles, progress toward goal, smoothness.
4. Execute best trajectory; recompute every 50–100ms.

**Advantage**: Reactive; works with local sensor information (lidar, sonar). Scales to real-time.

**Limitation**: Greedy; can get stuck in local minima (narrow corridors, U-shaped obstacles).

## Control Systems

### PID Control

**Problem**: Robot actuators (motors) have dynamics; direct command doesn't guarantee desired behavior.

**Feedback loop**: Measure current state, compare to desired state, apply corrective action.

**PID (Proportional-Integral-Derivative)** is the workhorse:

$$u(t) = K_p e(t) + K_i \int e(t) dt + K_d \frac{de}{dt}$$

where $e(t) = \text{desired} - \text{measured}$.

- **Proportional ($K_p$)**: Reacts to current error; too high causes oscillation.
- **Integral ($K_i$)**: Accumulates steady-state error; eliminates bias; too high causes slow response.
- **Derivative ($K_d$)**: Anticipates error trend; damps oscillation.

**Tuning**: Often manual or via Ziegler-Nichols method. Requires understanding system dynamics (inertia, friction, actuator limits).

**Application**: Motor speed control, position tracking, orientation (yaw) stabilization.

### Model Predictive Control (MPC)

Advanced approach: predict system behavior over future $N$ steps using a model; optimize sequence of control inputs to minimize deviation from trajectory.

**Optimization problem**:

$$\min \sum_{t=1}^{N} \left[ \|x_t - x_{\text{ref}}\|^2 + \|u_t\|^2 \right]$$

subject to dynamics $x_{t+1} = f(x_t, u_t)$ and constraints (actuator limits, collision avoidance).

**Advantages**:

- **Horizon**: Considers future trajectory, not just current error.
- **Constraints**: Naturally incorporates actuator limits and safety bounds.
- **Nonlinear models**: Handles complex vehicle dynamics (quadrotors, legged robots).

**Disadvantages**:

- **Computational cost**: Solving optimization at 10–50Hz demands substantial CPU; offloaded to dedicated hardware.
- **Model fidelity**: Relies on accurate system model; mismatch causes poor performance.

**Use cases**: Quadrotor trajectory tracking, autonomous driving lane-keeping, manipulation.

## Sensor Fusion and State Estimation

### Kalman Filter (Linear Case)

**Goal**: Estimate true state from noisy measurements and imperfect model.

**Setup**:

- **State dynamics**: $x_{t+1} = F x_t + w_t$ where $w_t \sim N(0, Q)$ is process noise.
- **Measurements**: $z_t = H x_t + v_t$ where $v_t \sim N(0, R)$ is measurement noise.

**Kalman Filter cycle**:

1. **Predict**: $\hat{x}_{t}^- = F \hat{x}_{t-1}$; $P_t^- = F P_{t-1} F^T + Q$.
2. **Update**: Kalman gain $K_t = P_t^- H^T (H P_t^- H^T + R)^{-1}$; $\hat{x}_t = \hat{x}_t^- + K_t (z_t - H \hat{x}_t^-)$.

**Optimality**: Minimum mean-square-error estimate under linear Gaussian assumptions.

**Application**: Fusing wheel odometry, IMU, and GPS into smooth, consistent robot pose.

### Extended Kalman Filter (EKF) and Nonlinear Case

Most robot systems are nonlinear (e.g., unicycle kinematics $\dot{x} = v \cos(\theta)$). EKF linearizes around the current estimate:

$$F \approx \nabla f|_{\hat{x}}, \quad H \approx \nabla h|_{\hat{x}}$$

Uses linearized matrices in Kalman filter. **Not optimal** but practical and widely used.

**Failure modes**: If estimate far from truth, linearization breaks; divergence possible. **Particle filters** or **unscented Kalman filters (UKF)** handle nonlinearity better but at higher computational cost.

## Computer Vision for Robotics

### Feature Detection and Matching

**Goal**: Identify visually distinctive points (corners, blobs) in images; match them across frames or cameras.

**Algorithms**:

- **Harris Corner Detection**: Eigenvalue analysis of image gradients; finds high curvature regions.
- **SIFT/SURF**: Scale-invariant feature transform; matches features across zoom levels.
- **ORB**: Oriented FAST features and rotated BRIEF descriptors; fast, open-source, license-negotiation-free.

**Application**: Visual odometry (track feature motion between frames to estimate camera motion), visual SLAM.

### Depth Estimation

**Stereo vision**: Two calibrated cameras separated by baseline $b$. A feature at disparity $d$ (pixel difference between left and right images) has 3D depth $Z = fb/d$ where $f$ is focal length.

**Challenges**: Feature matching across frames under occlusion, specular surfaces, textureless regions. Often fails outdoors in bright sunlight.

**RGB-D cameras** (Kinect, RealSense): Hardware depth sensors; no matching needed, but limited range and outdoor performance.

## Sim-to-Real Transfer

### The Reality Gap

Learning or testing policies in simulation is fast and safe. But sim-to-real transfer often fails: simulated physics are simplified, actuators behave differently, sensors have different noise models. Policy trained in simulation acts unpredictably on physical robot.

### Bridging Strategies

**Domain randomization**: Train in simulation with randomized physics parameters (mass, friction, sensor noise, visual appearance). Theory: diverse training improves generalization. Works for some tasks (grasping, locomotion); inconsistent results.

**System identification**: Estimate actual robot parameters (mass, inertia, friction) from real-world experiments. Update simulator to match. Enables more faithful training.

**Imitation learning with real data**: Train policy on data from real robot demonstrations, or collect small amounts of real experience and fine-tune simulator-trained policy.

**Hardware-in-the-loop**: Run control code on real hardware in loop with simulation; closest match to reality without full deployment.

**Progressive deployment**: Test on physical robot incrementally (slow motion, constrained environment) before full autonomy.

### Practical Expectations

- **Simple control tasks** (joint tracking, force control) often work with simulator + careful tuning.
- **Complex learning-based policies** (vision-based grasping, navigation) typically require real-world adaptation.
- **Rough sim-to-real rule**: Highly controlled tasks (pick-and-place in structured bin) transfer better than open-world tasks (household cleaning).

## Real-Time and Timing Constraints

Robot control runs on limited hardware; timing matters:

- **Sensor processing** (camera, lidar): 10–30 Hz typical.
- **Motion control** (PID, MPC): 50–200 Hz.
- **High-frequency stabilization** (quadrotor attitude control): 200–1000 Hz.

Missing deadlines (executing control update too late) destabilizes the system. **Real-time OS** (VxWorks, QNX, PREEMPT-RT Linux patched kernel) provides predictable latency; middleware (ROS2 with real-time DDS) helps but is not a substitute for OS-level support.

Trade-off: Determinism vs. feature richness. Pure real-time kernels offer predictability; general-purpose OSes (standard Linux) offer flexibility but weak timing guarantees.