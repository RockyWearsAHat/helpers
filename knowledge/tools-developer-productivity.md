# Tools: Developer Productivity

Developer productivity focuses on reducing friction: speed up common tasks, reduce context switching, eliminate repetitive CLI typing.

## Shell Customization

### oh-my-zsh

Popular zsh framework; ships with 200+ plugins + themes.

**Installation**:
```bash
sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"
```

**Plugins** (in `~/.zshrc`):
```bash
plugins=(
  git                 # git aliases (gst, gap, gcm, gp)
  fzf                 # fuzzy finder integration
  zsh-autosuggestions # history-based autocomplete
  zsh-syntax-highlighting  # highlight before executing
  dirhistory          # backward/forward through visited dirs (Alt+← / Alt+→)
  copyfile            # copyfile filename (copy to clipboard)
  ripgrep             # rg alias integration
  osx                 # macOS: quick open commands (o, ql, etc.)
)

source $ZSH/oh-my-zsh.sh
```

**Themes**: `robbyrussell` (default), `powerlevel10k` (advanced), `agnoster`.

**Tradeoffs**:
- **Pros**: Batteries-included; large ecosystem of plugins
- **Cons**: Adds ~100ms to shell startup; overkill for minimal setup

**Best for**: Shell-heavy users, teams sharing a standard config.

### Starship

Minimal, fast shell prompt (~5ms, vs oh-my-zsh ~100ms). Replaces PS1/PROMPT with Rust-based prompt.

**Installation**:
```bash
curl -sS https://starship.rs/install.sh | sh
```

**In `~/.zshrc`**:
```bash
eval "$(starship init zsh)"
```

**Configuration** (`~/.config/starship.toml`):
```toml
[character]
success_symbol = "[➜](bold green)"
error_symbol = "[➜](bold red)"

[git_branch]
symbol = "🌱 "
truncation_length = 20

[directory]
truncation_length = 3  # ~/projects/my-app → ~/p/my-app
```

**Features**:
- Shows git branch, status (dirty/clean)
- Package versions (Node version, Python version, etc.)
- Command execution time
- Minimal disk impact

**Tradeoffs**:
- **Pros**: Fast, minimal config, works across shells (bash/zsh/fish)
- **Cons**: Less ecosystem than oh-my-zsh

**Best for**: Developers valuing shell startup speed; minimalists.

## Dotfiles Management

Dotfiles (shell config, git config, editor settings, SSH keys, etc.) live in `~` but should be versioned.

**Pattern**: Store dotfiles in `~/dotfiles/`, symlink to `~`.

```bash
~/dotfiles/
├── .zshrc
├── .gitconfig
├── .gitignore_global
├── .config/
│   └── starship.toml
│   └── nvim/    (neovim config)
│   └── karabiner/  (macOS keyboard remapper)
└── Makefile  (setup automation)
```

**Symlink setup** (`~/dotfiles/Makefile`):
```makefile
install:
	ln -sf ~/dotfiles/.zshrc ~/.zshrc
	ln -sf ~/dotfiles/.gitconfig ~/.gitconfig
	ln -sf ~/dotfiles/.config/starship.toml ~/.config/starship.toml

update:
	cd ~/dotfiles && git pull && make install
```

**Benefits**:
- Version control for personal config
- Onboard to new machine: `git clone <dotfiles-url> && make install`
- Team standardization (shared base config, individual overrides)
- Audit trail of config changes

**Popular tools**:
- **GNU Stow**: Symlink manager dedicate to dotfiles
- **chezmoi**: Template-based dotfiles (handles machine-specific values)
- **bare git repo**: Minimal (no special tool needed)

**Tradeoffs**:
- **Pros**: Reproducible setup; version control; zero friction
- **Cons**: Requires discipline (remember to commit changes)

## CLI Tools & Alternatives

### fzf (Fuzzy Finder)

Interactive command-line search; fuzzy-matches input as you type.

**Common usage**:
```bash
# Search history with fzf
history | fzf

# Search files in project
find . -type f | fzf

# Preview before opening
fzf --preview 'cat {}'

# Multi-select (Tab to select, Enter to confirm)
fzf -m
```

**Shell integration** (oh-my-zsh has it by default):
```bash
# Ctrl+R: Fuzzy search history
# Ctrl+T: Fuzzy find files
# Alt+C: Fuzzy change directory
```

**Use cases**:
- Find file in large project: `fzf` → open in editor
- Jump to frequent directory: `export FZF_CD_COMMAND='fd -type d'`
- Select from long output: `git log --oneline | fzf`

**Tradeoff**: Learning curve for initial setup; huge time savings afterward.

### ripgrep (rg)

Fast regex search; replaces grep for 99% of use cases.

```bash
# Find "auth" in Python files
rg "auth" --type py

# Find in specific directory
rg "error" src/

# Show count, not matches
rg "error" --count-matches

# Whole word only (-w)
rg -w "error"

# Ignore .git, node_modules (respects .gitignore)
rg "error"  # automatically excludes ignored files
```

**Why faster than grep**:
- Respects .gitignore by default (skips node_modules, .git)
- Parallel search across files
- Regex engine optimizations

**Integration with editor**: Most editors (VS Code, Neovim) have ripgrep integration for project search.

### bat (Better cat)

Syntax highlighting + line numbers for file viewing.

```bash
# View with syntax highlighting
bat file.js

# Use in pipe (no line numbers, colors only)
grep "error" file.log | bat --plain

# Line range
bat --line-range 10:20 file.js

# Git diff integration
git diff | bat --changes  # highlight only changed lines
```

**Why useful**:
- Instant visual understanding (keywords highlighted)
- Line numbers (reference in discussions)
- Git integration (see changes in context)

**Tradeoff**: Another tool to install; `less` with `--RAW-CONTROL-CHARS` achieves similar effect.

### exa / eza

Modern `ls` replacement.

```bash
# Standard listing (default colors + icons)
eza

# Long format with details
eza -l

# Tree view
eza -T

# Show git status
eza -l --git

# Sort by modification time (newest first)
eza -lt modified --reverse
```

**Why better than ls**:
- Colored output by default
- Git status (modified/added/ignored already visible)
- Icons (requires Nerd Font)
- Simpler flags (vs BSD ls vs GNU ls differences)

**Tradeoff**: Icon rendering requires patched font; some terminals don't support it.

### delta

Syntax-highlighted git diff viewer.

**In `~/.gitconfig`**:
```ini
[core]
    pager = delta

[delta]
    theme = Dracula
    line-numbers = true
```

**Output**:
- Syntax highlighting for code
- Line numbers on both sides
- Clear visual diff (added/removed lines)
- Side-by-side view (`delta --side-by-side`)

**Tradeoff**: Slower than plain diff for very large diffs; visual clarity worth it.

### jq

JSON query/transform tool.

```bash
# Extract field
echo '{"name": "Alice", "age": 30}' | jq '.name'
# Output: "Alice"

# Array of objects
echo '[{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]' | jq '.[] | select(.id == 1)'
# Output: {"id": 1, "name": "Alice"}

# Transform
echo '{"users": [{"name": "Alice"}]}' | jq '.users[].name'
# Output: "Alice"

# Pretty-print (default)
curl https://api.github.com/repos/owner/repo | jq .

# API filtering
curl https://api.github.com/repos/owner/repo/issues | jq '.[] | select(.state == "open") | .title'
```

**Use cases**:
- Parse API responses in scripts
- Extract specific fields from logs (if JSON structured)
- Transform data pipelines

**Alternative**: `gron` (grepping JSON), `jless` (interactive jq explorer).

## Workspace Management

### tmux (Terminal Multiplexer)

Detachable sessions; run long-lived commands in background.

**Workflow**:
```bash
# Start named session
tmux new-session -s work

# Inside session: create panes/windows
# Prefix: Ctrl+B

# Create vertical split (Ctrl+B %)
# Create horizontal split (Ctrl+B ")
# Navigate panes (Ctrl+B arrow)

# Detach (Ctrl+B d) → terminal still runs in background
# Reattach after hours/reconnecting over SSH
tmux attach-session -t work
```

**Benefits**:
- SSH sessions don't die when connection drops
- Layout persistence (replicate yesterday's window setup)
- Co-editing (multiple users attach to same session)

**Configuration** (`~/.tmux.conf`):
```bash
set -g default-terminal "screen-256color"
set -g mouse on
bind -n C-h select-pane -L
bind -n C-j select-pane -D
bind -n C-k select-pane -U
bind -n C-l select-pane -R
```

### direnv

Auto-load/unload environment variables based on directory.

**In project root** (`.envrc`):
```bash
export DATABASE_URL="postgresql://localhost/mydb"
export NODE_ENV="development"
```

**Workflow**:
```bash
cd myproject/  # direnv loads .envrc
echo $DATABASE_URL  # postgresql://localhost/mydb

cd ../  # direnv unloads
echo $DATABASE_URL  # (unset)
```

**Benefits**: Never commit `.env` files; each developer has local environment; one-time setup.

**Popular alternatives**: `asdf` (runtime version manager), `nvm` (Node version manager).

## Automation Scripts

Reduce repetitive tasks with shell scripts in `~/bin/`.

**Examples** (from project experience):
- `git-upload`: Stage, commit, push (optional AI message)
- `git-fucked-the-push`: Undo last pushed commit, keep changes staged
- `deploy-staging`: Build, push image, deploy to staging cluster
- `db-seed`: Reset database, load fixtures

**Best practices**:
- Use `#!/usr/bin/env bash` (portable across systems)
- Set `set -euo pipefail` (fail on error, undefined vars, pipe failures)
- Add to `$PATH` (symlink from `~/bin/` in GitHub dotfiles repo)
- Document with `--help` flag

**Discovery**: `ls ~/bin/ | fzf | xargs open -a $EDITOR` (browse scripts quickly).

## Productivity Measurement

How to measure if tools actually increase productivity?

### Metrics

- **Context switches**: Count per hour (lower is better)
- **Keystrokes to goal**: Compare before/after tool adoption
- **TTFR** (time to first response): How fast to find information

### Reality

- Tools can reduce repetition, but complexity adds cognitive load
- New tool needs 1-2 weeks to become automatic (don't judge early)
- Diminishing returns (50th tool doesn't help as much as first 5)

**Guidance**: Choose tools that directly address your slowest workflows, not "cool tools."

## Practical Setup Path

1. **Start with shell prompt** (Starship, fast startup, `~5ms`)
2. **Add git shortcuts** (oh-my-zsh git plugin or custom aliases)
3. **Add search** (fzf for file/history finding)
4. **Add viewing** (bat for files, delta for diffs, exa for listings)
5. **Add searching** (ripgrep for code)
6. **Add data manipulation** (jq for JSON APIs)
7. **Add multiplexing** (tmux if remote work or long-lived processes)
8. **Version dotfiles** (symlink setup for quick onboarding)

Each tool solves one problem well. Combine them for compound benefit.

## See Also

terminal-productivity, language-shell, developer-tools-landscape, process-developer-experience