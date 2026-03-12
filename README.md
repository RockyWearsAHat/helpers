# NEW FEATURE RELEASED

Git Copilot Audit

Ask copilot to install this honestly idk if it will work, I will work on a full standalone installer soon. Once installed, globally exposes a `/copilot-devops-audit` command. This command throughly researches and identifies issues in the .github directory and fixes them automaticlly.

After installing this repo on your machine, this command MAY WORK:

`./git-copilot-devops-audit --update-agent --force`

It works on my machine, but no promises it works on yours.

Audit currently accurately identifies issues and potential resolutions, plus resolves them. There may be bugs or problems when using, however it was made to be extensively expandable and runnable & helpful in ANY workspace.

Using subagents, this command

1. Gets a context of the entire codebase and general project setup & overview, and throughly looks through every bit of the .github folder.
2. Pulls documentation from GitHub (Microsoft, Anthropics, GitHub) repositories sharing best practice and community created examples. Additionally fetches YouTube videos from Burke Holland & others and gets transcripts + directly pulls docs about VS code and agentic development.
3. With understanding that lead developers of projects have more understanding than people that are experimenting with the system, it weights each source accordingly and then sends all context and research to a full audit subagent.
4. Finally, once the subagent has returned, the manager itself can decide what is proper. It always bases upon the current codebase and updates docs to match current state first, then works on what is safe, with potential escalation to the user for unsafe or possibly harmful workflow changes.

Ideally this is meant to improve the speed of getting copilot to perform the task it is intented to be performing.

In practice, I cannot attest to how good it is, however it found great issues and offered valid solutions, and without being told to actually figured out that the inbuilt copilot instructions WITHIN ITSELF were a good reference after seeing that copilot referenced into the copilot-instructions for both copilot and claude. This was unprompted behavior but may be included in the prompt because this was not something I thought of and it is a really good thought, but you can review the output above and make up your own mind if this is helpful or more AI slop.

Here are the findings from a C++ server application meant for emulation + a home theature/streaming system + NAS. #3 is a good point given the visual development flow doesn't work, but if this was fully + well implemented it likely wouldn't say "cut this down" and instead give a much different result. Please don't let point #3 confuse you it was a never working agent/prompt from the start (it's been told everything in the workspace is correct to never touch source and that likely leads to some unintended results of "this file is correct but doesn't work", the agent proceeds to cry, think really hard, then just say "give up on the file" in the least "I'm giving up" way it can).

1. Critical: [.github/instructions/subagent-research-and-discussion.instructions.md](.github/instructions/subagent-research-and-discussion.instructions.md#L2) is the wrong file type and the wrong scope. It applies to every request, but its content is generic orchestration methodology, including prompt templates and delegation rules at [.github/instructions/subagent-research-and-discussion.instructions.md](.github/instructions/subagent-research-and-discussion.instructions.md#L64). That belongs in a reusable skill or user-level workflow, not in repo-local always-on instructions.
2. Significant: the repo is carrying several broad, non-project-specific instruction files that add context cost without adding AIO Server facts: [.github/instructions/performance-optimization.instructions.md](.github/instructions/performance-optimization.instructions.md#L2), [.github/instructions/security-and-owasp.instructions.md](.github/instructions/security-and-owasp.instructions.md#L2), [.github/instructions/self-explanatory-code-commenting.instructions.md](.github/instructions/self-explanatory-code-commenting.instructions.md#L3), and [.github/instructions/object-calisthenics.instructions.md](.github/instructions/object-calisthenics.instructions.md#L2). They read like general policy packs, not repo-specific guidance.
3. Significant: [.github/instructions/visual-development-testing.instructions.md](.github/instructions/visual-development-testing.instructions.md#L2) contains useful repo facts, but it is far too large and procedural for an always-on instruction file. It includes full step-by-step task workflows at [.github/instructions/visual-development-testing.instructions.md](.github/instructions/visual-development-testing.instructions.md#L36) and [.github/instructions/visual-development-testing.instructions.md](.github/instructions/visual-development-testing.instructions.md#L437), and it lists recommended ROMs that are not present in the checked-in test ROM set while [.github/instructions/visual-development-testing.instructions.md](.github/instructions/visual-development-testing.instructions.md#L340) presents them as concrete local assets. Keep the repo facts, not the whole playbook.
4. Significant: [.github/instructions/cmake-vcpkg.instructions.md](.github/instructions/cmake-vcpkg.instructions.md#L6) contains unverified repo facts. It says the project uses vcpkg manifest mode and prefers CMakePresets.json at [.github/instructions/cmake-vcpkg.instructions.md](.github/instructions/cmake-vcpkg.instructions.md#L7), but no vcpkg.json or CMakePresets.json exists in this workspace, and the actual build entry point is the Make wrapper in [Makefile](Makefile#L1). This should be rewritten around the real build flow.
5. Significant: [.github/README.md](.github/README.md#L21) and [.github/README.md](.github/README.md#L31) include audit-specific process guidance. The evaluation skill explicitly treats audit references in repo Copilot files as a problem; these lines are about the audit apparatus, not about developing AIO Server.
6. Minor: [.github/instructions/memory.instructions.md](.github/instructions/memory.instructions.md#L3) is concise and useful, but it is still customization-maintenance guidance, not day-to-day coding context. Loading it on every request is unnecessary; its scope should be narrowed to Copilot-customization edits only.
7. Positive: [.github/copilot-instructions.md](.github/copilot-instructions.md), [.github/agents/expert-cpp-software-engineer.agent.md](.github/agents/expert-cpp-software-engineer.agent.md), [.github/agents/visual-dev-tester.agent.md](.github/agents/visual-dev-tester.agent.md), [.github/instructions/emulator-core.instructions.md](.github/instructions/emulator-core.instructions.md), and [.github/instructions/qt-ui.instructions.md](.github/instructions/qt-ui.instructions.md) already match the target state reasonably well: concise, project-specific, and aligned to actual repo workflows.

Since this is a one time command, feel free to continue to use the chat window normally as you would afterwards, it uses subagents to not pollute the context massively & should be able to give good follow ups as needed [atleast I sure hope lol]. Good luck, hope it works & if you try it I hope it's helpful.

Here is an example instructions file after it ran the audit in the AIOServer, newly created on the fly:

```md2w
---
description: "Accuracy, testing, and logging rules for emulator-core changes."
applyTo: "src/emulator/**/*.cpp,include/emulator/**/*.h,include/emulator/**/*.hpp,tests/**/*Tests.cpp"
---

# Emulator Core Workflow

- Fix root causes before adjusting timings, masking failures, or broadening tolerances.
- Preserve existing timing and correctness behavior unless the task explicitly changes emulation semantics.
- Prefer focused tests, characterization tests, or deterministic headless runs before broad manual verification.
- Keep hot-path logging minimal and use targeted trace flags plus `debug.log` when runtime evidence is needed.
```

# Git Shell Helpers

Small quality-of-life helpers wrapped as git subcommands:

- `git upload` – stage, commit, and push, with optional AI-generated commit messages using GitHub Copilot CLI.
- `git get` – initialize a local repo from a remote (like a lightweight `git clone` flow).
- `git initialize` – initialize the current directory as a repo, create an initial commit, set `origin`, and push.
- `git fucked-the-push` – destructive recovery helper to undo the last pushed commit while keeping changes staged.

## Installation options

### 1. macOS .pkg installer (recommended)

On macOS, the preferred way to install is via the native `.pkg` installer:

- Installs binaries into `/usr/local/bin` and man pages into `/usr/local/share/man/man1`.
- Does **not** modify your shell config files.

Grab the latest packaged macOS installer from the latest release:

- [github-shell-helpers latest release](https://github.com/RockyWearsAHat/github-shell-helpers/releases/latest)

After downloading `github-shell-helpers-<version>.pkg`:

1. Double-click the `.pkg`.
2. Follow the standard macOS Installer flow.

Once complete, the commands and man pages should be available immediately in any new shell:

- `git upload`
- `git get`
- `git initialize`
- `git fucked-the-push`
- `git help upload|get|initialize|fucked-the-push`

### 2. One-line script installer (portable alternative)

If you prefer a script-based install into your home directory, use the installer script. This works well on macOS and other Unixy environments where you control your shell config.

Direct download of the installer script:

- [Git-Shell-Helpers-Installer.sh](https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main/Git-Shell-Helpers-Installer.sh)

```sh
curl -fsSL \
  https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main/Git-Shell-Helpers-Installer.sh \
  | zsh
```

After installation, either open a new terminal or run:

```sh
source ~/.zshrc
```

Then you can use the same commands and help pages as with the `.pkg` installer.

## Why two installer methods?

- **Script installer (`Git-Shell-Helpers-Installer.sh`)**
  - Cross-shell friendly, very easy to share as a copy-pastable command.
  - Installs into your home directory and updates your shell config.
- **macOS `.pkg` installer**
  - Integrates with the native macOS Installer UI.
  - Installs into system-level locations (`/usr/local/...`) without touching your `~/.zshrc`.

Both install the same commands and man pages; they just target different installation styles.

## Development

- Update the version number in `VERSION` before cutting a new release.
- Build artifacts locally:
  - Script installer dist: `./scripts/build-dist.sh` → `dist/Git-Shell-Helpers-Installer.sh`
  - macOS pkg: `./scripts/build-pkg.sh` → `dist/github-shell-helpers-<version>.pkg`
- VS Code tasks:
  - **Build installer** – runs `./scripts/build-dist.sh`.
  - **Build macOS pkg** – runs `./scripts/build-pkg.sh`.

CI (see `.github/workflows/build-installer.yml`) ensures both installers build cleanly on each push to `main`.
