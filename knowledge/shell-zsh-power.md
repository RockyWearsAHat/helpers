# Zsh Power Features: Glob Qualifiers, Completion, Prompts, and Workflows

## Introduction

Zsh is a superset of POSIX shell with powerful interactive features and a sophisticated completion system. While bash excels at scripting, zsh excels at interactive productivity and visual customization. This note covers glob qualifiers (filesystem-aware filtering), the completion framework, prompt customization via `vcs_info` and modules, and ecosystem frameworks (Oh My Zsh, Prezto). The tension: zsh enables immense interactive customization, but complex configurations become maintenance burdens and slow login times.

## Glob Qualifiers: Filesystem-Aware Expansion

Zsh glob qualifiers filter file patterns by type, modification time, size, and permissions. Syntax: `pattern(qualifier)`.

### Common Qualifiers

```bash
# File type qualifiers
*.txt(.)                    # Regular files only
*.txt(/)                    # Directories
*.txt(@)                    # Symlinks
*.txt(*)                    # Executable files
*.txt(=)                    # Sockets

# Modification time (within N days)
*.log(mh-24)                # Files modified within 24 hours
*.log(mh+24)                # Files modified more than 24 hours ago
*.log(Mh-24)                # Symbolic: same as mh-24
*(m-1)                      # Files modified yesterday
*(m0)                       # Files modified today

# Size qualifiers
*.iso(Lm+100)               # Files larger than 100 MB (M = MB)
*.tmp(L-1m)                 # Files smaller than 1 MB
*(Lk+100)                   # Files larger than 100 KB

# Permissions
*.sh(x)                     # Executable files
*(u=alex)                   # Files owned by user 'alex'
*(g=wheel)                  # Files owned by group 'wheel'

# Sorting and limiting (returned in order)
*(om[1,5])                  # 5 oldest files (om = sort by mod time, [1,5] = range)
*(OL[1,3])                  # 3 largest files (OL = sort by size desc, [1,3] = take first 3)
*(om[-1])                   # Newest file
```

Combining qualifiers:

```bash
# Logical AND: all conditions must match
*(m-1.x)                    # Modified today AND executable

# Logical OR requires negation patterns (complex)
# Prefer: separate globs with (pattern1) (pattern2)

# Practical: find large, old logs
rm *.log(m+30Lm+10M)        # Modified over 30 days ago, larger than 10 MB

# Interactive: backup 5 largest files
cp *(OL[1,5]) backup/
```

Use glob qualifiers instead of piping to `find`:

```bash
# Traditional
find . -name "*.log" -type f -mtime +7 -size +10M

# Zsh elegant
rm **/*.log(m+7Lm+10M)      # ** = recursive glob

# Copy 10 most recent in subdir
cp archive/(m[-10]) /tmp/
```

## Completion System: compdef and _arguments

Zsh completion is more flexible than bash but requires learning the framework. The system centers on `compdef` (define completions) and completion functions starting with `_`.

### Simple Completion Definition

```bash
# Function-based completion
_mycommand() {
    # $1 = function name (can be reused)
    # Implement completion logic here
}

# Register
compdef _mycommand mycommand

# Alternative: use built-in completers
complete() {
    _command_names     # Suggest installed commands
}
```

### _arguments: Structured Argument Completion

The `_arguments` function parses your command's flags and options, providing context-aware completions.

```bash
# Declare command: myapp [--verbose] [--output FILE] [--config FILE] [FILES...]

_myapp() {
    _arguments \
        '(--verbose)--verbose[enable verbose logging]' \
        '(--output)--output[output file]:output file:_files' \
        '(--config)--config[config file]:config:_files -g "*.conf"' \
        '*:input files:_files'
}

compdef _myapp myapp
```

The `_arguments` syntax:

- `'(--verbose)--verbose[description]'` — Mutually exclusive `--verbose` flags; `[description]` shown in completion menu
- `--output` — Takes an argument (indicated by `:`)
- `output file:_files` — Argument label and completion function to call
- `_files -g "*.conf"` — Complete only `.conf` files
- `'*:label:function'` — Variadic positional arguments

### Common Completion Functions

```bash
_files                      # Suggest files and directories
_directories                # Directories only
_command_names              # Installed commands
_users                      # System users
_groups                      # System groups
_hosts                      # Hostnames from SSH config, /etc/hosts
_urls                       # URLs (requires network)
_git_branches               # Git branches (if git completion loaded)
```

Debugging completion:

```bash
# Trace completion execution
zstyle ':completion:*' use-cache off    # Disable caching during debugging
zstyle ':completion:*' verbose yes      # Verbose output
set -x                                  # Shell tracing

# Then Tab to trigger completion
```

## Prompt Customization: PROMPT, RPROMPT, vcs_info

### Basic Prompt Variables

```bash
# Left prompt (default location)
PROMPT='%n@%m:%~ %# '                   # user@host:path $ (or #)

# Right prompt (right-aligned)
RPROMPT='[%D{%H:%M:%S}]'                # Time on far right

# Initial prompt after login without command
PS1='Welcome to %N'                     # Shows user name

# Continuation (for multi-line input)
PS2='> '                                # Shown for line continuation

# Simple prompt escape sequences
%n                                      # Username
%m                                      # Machine name (short hostname)
%~                                      # Current directory (~ for $HOME)
%#                                      # # for root, % for non-root
%D{format}                              # Date in strftime format
%T or %*                                # Time
%w                                      # Weekday and date
%(x.true.false)                        # Conditional (if condition x)
```

### vcs_info: Version Control Integration

`vcs_info` displays VCS status (git branch, dirty status) in the prompt.

```bash
# Enable and configure
autoload -Uz vcs_info
precmd() {
    vcs_info
}

# Use in prompt
PROMPT='%n:%~ ${vcs_info_msg_0_} %# '

# Customize appearance
zstyle ':vcs_info:*' formats '%F{green}[%b]%f'                     # Branch
zstyle ':vcs_info:*' actionformats '%F{red}[%b|%a]%f'             # Action (rebase, merge)
zstyle ':vcs_info:*' enable git hg svn                            # VCS types to support

# Advanced: display dirty/clean status
zstyle ':vcs_info:*' check-for-changes true
zstyle ':vcs_info:*' check-for-staged-changes true
zstyle ':vcs_info:*' stagedstr ' %F{yellow}●%f'                   # Staged indicator
zstyle ':vcs_info:*' unstagedstr ' %F{red}●%f'                    # Unstaged indicator

# Format with indicators
zstyle ':vcs_info:*' formats '%F{green}[%b%u%c]%f'
```

## Zsh Modules: Extending Functionality

Zsh modules add capabilities without recompilation. Official modules include:

```bash
# Common modules
autoload -U colors                      # Color support in prompt
zsh/datetime                           # Date/time formatting
zsh/mathfunc                           # Math functions
zsh/regex                              # Regular expression matching
zsh/zpty                               # Pseudo-terminal control (debugging)
zsh/terminfo                           # Terminal capabilities (colors, etc.)

# Load module
zmodload zsh/datetime
zmodload zsh/mathfunc

# Use module functions
(( pi = atan2(0) * 4 ))                 # Math functions available
echo $((sin(pi/2)))
```

## Extended Glob Operators

Zsh glob operators enable complex patterns without external commands.

```bash
# Enable extended globs (off by default)
setopt extended_glob

# Operators
*(pattern)                              # Zero or more matches of pattern
+(pattern)                              # One or more matches
?(pattern)                              # Zero or one match
!(pattern)                              # Anything except pattern
@(pattern)                              # Exactly one match (alternative syntax)

# Examples
ls *(^*.log)                            # All files except .log
ls *(~*.bak)                            # Negation ~: everything except .bak
ls **/(*.txt|*.md)                      # Recursive: txt or md files

# Complex: match anything except specific names
rm *(^(config|README))                  # Delete all except config and README

# Match nested globs
ls **/*.{js,ts}(^node_modules)          # JS/TS files, exclude node_modules
```

## Oh My Zsh and Prezto: Configuration Frameworks

### Oh My Zsh

Oh My Zsh is the most popular zsh config framework: 200+ plugins, themes, and auto-configuration.

```bash
# Installation (from https://ohmyz.sh/)
sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"

# Configuration: ~/.zshrc
export ZSH="$HOME/.oh-my-zsh"
ZSH_THEME="agnoster"                   # Visual theme

plugins=(
    git                                # Git aliases and functions
    ruby                               # Ruby helpers
    history-substring-search           # Arrow keys search history
    fzf                                # Fuzzy finder integration
    zsh-autosuggestions                # Command suggestions
    zsh-syntax-highlighting            # Syntax coloring
)

source $ZSH/oh-my-zsh.sh
```

Trade-offs:
- **Pros**: Easy setup, huge plugin ecosystem, active community
- **Cons**: Slow startup (100ms+); cumbersome to customize; couples your config to the framework

### Prezto: Minimal Alternative

Prezto is a lighter-weight configuration framework emphasizing speed and minimalism.

```bash
# Installation
git clone --depth 1 https://github.com/sorin-ionescu/prezto.git "${ZDOTDIR:-$HOME}/.zprezto"

# Configuration: ~/.zpreztorc
zstyle ':prezto:*:*' color 'yes'
zstyle ':prezto:module:prompt' theme 'agnoster'

zstyle ':prezto:load' pmodule \
    'environment' \
    'terminal' \
    'editor' \
    'history' \
    'directory' \
    'spectrum' \
    'utility' \
    'completion' \
    'prompt'
```

Prezto advantages:
- Faster startup (~30ms vs. 100ms+)
- Simpler modular design
- Easier to debug and customize

### Custom Configuration (No Framework)

For minimal startup time and full control, configure zsh manually:

```bash
# ~/.zshrc (annotated minimal setup)

# History
HISTSIZE=10000
SAVEHIST=10000
setopt HIST_IGNORE_DUPS

# Completion
autoload -Uz compinit
compinit

# Prompt
autoload -Uz vcs_info
precmd() { vcs_info }
PROMPT='%n:%~ ${vcs_info_msg_0_} %# '

# Useful options
setopt extended_glob
setopt prompt_subst          # Allow functions in PROMPT

# Aliases
alias grep='grep --color'
alias ls='ls -G'
```

Startup time without framework: ~5ms. With Oh My Zsh: 100ms+. Matters for rapid shell invocations (scripts, CI).

## History Substring Search and Other Interactive Features

### Substring Search

Built into zsh (via plugin in Oh My Zsh):

```bash
# ~/.zshrc
autoload -U history-search-{,back}ward
zle -N history-search-backward
zle -N history-search-forward
bindkey '^[OA' history-search-backward    # Up arrow
bindkey '^[OB' history-search-forward     # Down arrow

# Usage: type `git` and press Up arrow → shows only git commands in history
```

### Navigate Directories: Named Directory Hashing

```bash
# Define named directories (shortcuts to deep paths)
hash -d projects=~/projects
hash -d work=~/work/current_project

# Usage: change to named dir with ~name syntax
cd ~projects                            # Equivalent to cd ~/projects

# List all hashmarks
hash

# Dynamic hashmarks: auto-hash frequently accessed dirs
setopt auto_cd                          # `dirname` alone changes dir
setopt auto_pushd                       # Dir stack on pushd/popd
setopt pushd_ignore_dups                # Don't duplicate in stack
```

## zmv: Advanced Rename

`zmv` is a powerful batch rename tool, far simpler than `find` + `mv`:

```bash
# Enable
autoload -U zmv

# Rename: capture group syntax
zmv 'file_(*).txt' 'backup_$1.txt'      # file_1.txt → backup_1.txt

# Recursive with glob qualifiers
zmv '**/*.jpg' '**/*.jpeg'              # Rename all JPGs to JPEG recursively

# Conditional rename with glob qualifiers
zmv -Q '*.log(m+30)' 'archive/$1.log'  # Move old logs to archive/

# Dry-run before executing
zmv -n 'old_*' 'new_$1'                 # Shows what would happen
zmv 'old_*' 'new_$1'                    # Execute after verification

# Complex patterns: swap extensions
zmv '(*).(txt|md)' '$1.bak.$2'          # file.txt → file.bak.txt
```

## Trade-Offs and Pitfalls

| Feature              | Benefit                     | Cost                |
| -------------------- | --------------------------- | ------------------- |
| Glob qualifiers      | Concise file filtering      | Not portable (zsh 5+) |
| Completion system    | Rich interactive experience | Complex to customize |
| vcs_info             | Branch in prompt            | Performance if slow VCS |
| Extended globs       | Powerful pattern matching   | `setopt` required   |
| Oh My Zsh            | Easy setup                  | Slow startup        |
| zmv                  | Elegant batch rename        | zsh-only            |

Performance considerations:
- **Slow prompt**: If zsh prompt processes git status on every command, PS1 becomes a bottleneck. Use `vcs_info` with caching or precmd throttling.
- **Large history**: If `HISTSIZE` is huge and completion enabled, first Tab is slow. Tune history size and completion caching.
- **Plugin sprawl**: Each plugin adds to startup time. Remove unused plugins ruthlessly.

## Cross-References

See also: [shell-posix-mastery.md](shell-posix-mastery.md) (portable alternatives), [shell-bash-advanced.md](shell-bash-advanced.md) (bash equivalents), [shell-testing-quality.md](shell-testing-quality.md) (testing zsh scripts), [cli-ux-engineering.md](cli-ux-engineering.md) (interactive CLI design).