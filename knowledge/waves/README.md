# Knowledge Wave Manifests

Each JSON file defines a batch of KnowledgeBuilder agent assignments.

## Usage

Tell the KnowledgeWaveOrchestrator agent: "Fire wave 9" and it reads `wave-09.json`, then launches one KnowledgeBuilder subagent per assignment.

## Format

```json
{
  "wave": 9,
  "description": "CS Knowledge from Pop Culture & Informal Media",
  "agent": "KnowledgeBuilder",
  "prefix": "Shared instructions prepended to every assignment",
  "suffix": "Shared instructions appended to every assignment",
  "assignments": [
    {
      "id": 1,
      "label": "Short label for tracking",
      "topics": [
        {
          "filename": "category-topic.md",
          "description": "What to research and write about"
        }
      ]
    }
  ]
}
```

Each assignment becomes one subagent invocation. The full prompt sent to the agent is: `prefix + topics list + suffix`.
