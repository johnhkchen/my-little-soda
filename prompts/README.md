# My Little Soda Prompts

This folder contains the two core prompts used with the My Little Soda multi-agent orchestration system.

## The Two-Prompt System

The system uses a simple but effective approach:

1. **`initial-system-prompt.md`** - Complete workflow and orchestration instructions for autonomous coding agents
2. **`finishing-prompt.md`** - Cleanup and handoff instructions for ending work sessions
3. **Clear context** - Between sessions, the context is cleared to start fresh

## Usage Pattern

```
1. Load initial-system-prompt.md → Agent works autonomously
2. Clear context
3. Load finishing-prompt.md → Agent cleans up and hands off
4. Repeat cycle
```

## Structure

- `initial-system-prompt.md` - The main autonomous agent instructions
- `finishing-prompt.md` - Cleanup and repository state management
- `workflow-instructions.md` - (Stub - content is in initial-system-prompt.md)
- `quick-start-guide.md` - (Stub - content is in initial-system-prompt.md)
- `examples/` - Reserved for future prompt variations

## Philosophy

Simple, focused prompts that work effectively with clear context boundaries. The two-prompt approach ensures:
- Clean handoffs between work sessions
- Proper repository state management
- Autonomous operation with clear boundaries
- Scalable across multiple agents and repositories