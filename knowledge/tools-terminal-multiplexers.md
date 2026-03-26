# Terminal Multiplexers — Sessions, Panes, Scripting & Persistence

## Conceptual Model: Detach & Reattach

A terminal multiplexer decouples your terminal interaction from the processes running inside. Key insight:

- Running a process in tmux/screen/Zellij lets you disconnect (detach session) while processes keep running
- Reconnect (reattach) later and see exactly where you left off
- Multiple users can attach the same session simultaneously

This enables persistent remote development, pair programming, and process isolation.

```
Your terminal (local)
      ↓
SSH connection to server
      ↓
Multiplexer daemon (tmux server)
      ↓
Multiple clients, sessions, windows, panes
```

The daemon outlives any single TCP connection, so network hiccups don't kill your session.

## Tmux: Sessions, Windows, Panes

The dominant multiplexer. Hierarchical mental model:

```
Server (tmux process)
├── Session 1
│   ├── Window 0: shell (active pane)
│   ├── Window 1: editor
│   │   ├── Pane 0 (left)
│   │   └── Pane 1 (right)
│   └── Window 2
└── Session 2
    └── Window 0
```

### Sessions

A session is a collection of windows. Usually represents a project or context.

```bash
tmux new-session -s dev        # Create session "dev"
tmux new -s work -d            # Detached (no attach)
tmux list-sessions             # ls
tmux attach-session -t dev     # Attach (or tmux a -t dev)
tmux kill-session -t dev       # Destroy
tmux kill-server               # Kill entire tmux server & all sessions
```

**Naming convention:** `session:window:pane` addresses nested targets. `dev:1:0` = session "dev", window 1, pane 0.

### Windows

A window is a single terminal within a session. Switch between windows like tabs.

```bash
tmux new-window -t dev -n editor    # New window in session "dev" named "editor"
tmux list-windows -t dev            # List windows in dev
Ctrl+b c                            # New window (within session)
Ctrl+b n                            # Next window
Ctrl+b p                            # Previous window
Ctrl+b 1                            # Go to window 1
tmux rename-window -t dev:0 rocks   # Rename window 0 in dev to "rocks"
```

### Panes

Split a window into panes. Useful for viewing two things side-by-side.

```bash
Ctrl+b %                # Vertical split
Ctrl+b "                # Horizontal split
Ctrl+b ↑↓←→            # Navigate between panes
Ctrl+b x                # Kill current pane
Ctrl+b {                # Swap current pane with previous
tmux select-pane -t dev:0.1  # Navigate to session dev, window 0, pane 1
```

## Screen: Simpler, Older Sibling

Screen preceded tmux and still sees use in legacy environments. Simpler mental model (no nested hierarchy):

```bash
screen -S dev           # Create session "dev"
screen -ls              # List sessions
screen -r dev           # Reattach
Ctrl+a c                # New window
Ctrl+a n                # Next
Ctrl+a p                # Previous
Ctrl+a d                # Detach
```

**Screen vs tmux:**
- Screen: simpler keybindings, older, less actively developed
- Tmux: more powerful (nested panes), better scripting, modern

Most new projects use tmux.

## Zellij: Modern Rust Rewrite

Newer Rust-based multiplexer with focus on ergonomics and pane layouts.

```bash
zellij                   # Start new session
zellij attach-session work   # Reattach to "work"
zellij list sessions     # List sessions (running in background)
zellij kill-session work # Kill session
```

**Zellij differences from tmux:**
- **Built-in layouts**: Define pane arrangements declaratively (YAML/KDL)
- **Better UX**: First-run tutorial, UI-driven pane resizing (no cryptic key combos)
- **Floating panels**: Pop windows over the main layout
- **Scripting via KDL**: Configure plugins and behavior declaratively (vs tmux's config file)
- **No session multiplexing**: One main session per terminal (simpler mental model, still experimental)

```yaml
# Zellij layout.kdl
layout {
  pane split_direction="vertical" {
    pane
    pane split_direction="horizontal" {
      pane
      pane
    }
  }
}
```

## Configuration & Scripting

### Tmux Config (`.tmux.conf`)

```bash
# Key remapping (Ctrl+a instead of Ctrl+b)
set -g prefix C-a

# Mouse support
set -g mouse on

# Colors & theme
set -g default-terminal "screen-256color"

# Window base index (start at 1 instead of 0)
set -g base-index 1

# Status line
set -g status-left "#[fg=green]#S"  # Show session name
set -g status-right "%H:%M"

# Pane colors on split
set -g default-shell /bin/zsh

# Custom keybindings
bind-key -n C-s send-keys -X search-forward
```

### Tmux Scripting

```bash
# Create a complete environment programmatically
tmux new-session -d -s dev -x 220 -y 50
tmux send-keys -t dev:0 "cd ~/project && npm start" Enter
tmux new-window -t dev -n editor  
tmux send-keys -t dev:1 "vim src/index.js" Enter

# Complex: conditional logic
tmux if-shell "[[ -f .env ]]" "send-keys -t dev:0 'source .env' Enter"
```

**Use cases:**
- Development environment setup scripts
- CI/CD agent setup (persistent background processes)
- Automated testing across multiple shells

## SSH & Pair Programming

### Persistent Remote Shell

SSH into a server and run tmux:

```bash
ssh user@server
tmux new-session -s work
# ... do work ...
# (network drops)
ssh user@server
tmux attach -t work    # Reconnect to same session
```

Process continues running server-side; reconnection is immediate.

### Pair Programming: Shared Session

Two developers attach the same tmux session. Both see the same screen and can type (careful!).

```bash
# Developer A (on server)
tmux new-session -s pair

# Developer B (SSH to same server)
tmux attach-session -t pair
```

**Caveats:**
- Both see the same cursor position (can be disorienting)
- Only one can be typing at a time (no true simultaneous editing)
- Better alternatives: collaborative editors (VS Code Live Share, Figma) for true simultaneous work

**Alternative: Read-only join for screen sharing**

```bash
tmux new-session -s pair -d
tmux send-keys -t pair "cd project; npm start" Enter

# Other user observes (read-only)
tmux attach-session -t pair -r   # Read-only attach
```

## Integration Patterns

### Session Per Project

```bash
tmux new-session -s api -d -c ~/projects/api
tmux send-keys -t api "npm start" Enter

tmux new-session -s frontend -d -c ~/projects/frontend
tmux send-keys -t frontend "npm start" Enter
```

Switch projects without worrying about window state.

### Persistent Build Environment

Run long-lived processes (builds, tests, watchers) in background sessions:

```bash
tmux new-session -d -s tests
tmux send-keys -t tests "npm test -- --watch" Enter

# Detach, work on code, reattach to see test results
tmux attach -t tests
```

### SSH ProxyJump with Tmux

Complex multi-hop SSH setup combined with tmux for nested terminal environments:

```bash
# Local → Bastion → Internal Server
ssh -J bastion.example.com internal.local
tmux attach -t remote-session
```

Each hop maintains tmux persistence independently.

## Configuration Drift & Portability

### Problem: Cross-Machine Consistency

Different servers may have different tmux versions, shells, or configurations. Common issues:

- Keybindings differ
- Color schemes don't load
- Plugins incompatible

### Solutions

1. **Dotfiles repo**: Sync `.config/tmux/` and `.zshrc` across machines (popular with GitHub dotfiles)
2. **Version guards in config**:
   ```bash
   if-shell "tmux -V | grep -q 3.3" {
       set -g default-terminal "tmux-256color"
   }
   ```
3. **Minimal baseline**: Keep `.tmux.conf` simple; extend in environment-specific files
4. **Plugin manager**: `tpm` (Tmux Plugin Manager) handles version-aware plugin installation

## Performance & Limits

### Terminal Rendering

Tmux copies screen content into its internal buffer and re-renders on client reattach. With many panes or fast-scrolling output, this can bottleneck:

- **Issue**: Slow SSH links + rapid output → input lag
- **Mitigation**: Reduce pane count, cap output with `less`, use asynchronous background execution

### Session Limits

- Tmux scales to hundreds of sessions and panes (limited mainly by RAM)
- Screen is older; fewer panes recommended before performance degrades
- Zellij is single-session-per-terminal (simpler resource model)

### Shell Integration

Tmux doesn't understand shell syntax; it can't auto-complete or parse commands. Use shell plugins for better integration:

```bash
# Zsh + tmux: auto-start tmux on login
[[ -z "$TMUX" ]] && tmux attach -t main || tmux new-session -s main
```