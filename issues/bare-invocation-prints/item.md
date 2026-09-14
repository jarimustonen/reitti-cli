---
created: 2026-09-14
updated: 2026-09-14
type: bug
reporter: jarimustonen
status: fixed
priority: normal
closed: 2026-09-14
closed_by: jarimustonen
commits:
- hash: 7fd4e1d
  summary: show readable help for a bare invocation
---

# Bare invocation prints escaped help

## Description

Running `reitti` without arguments prints the complete help text as one error line with literal `\n` escape sequences. The result is difficult for both humans and agents to read.

## Reproduction

```sh
reitti
```

Observed before the fix: exit 1, empty stdout, and an escaped multiline usage document on stderr.

## Expected behavior

A bare invocation displays the same readable root help as `reitti --help`, writes it to stdout without an error prefix or escaped newlines, and exits 0.

## Acceptance Criteria

- [x] A bare invocation emits readable root help to stdout.
- [x] Its output is byte-identical to `reitti --help`, stderr is empty, and the exit status is 0.
- [x] An integration test prevents regression.

## Resolution

The command now treats a truly bare invocation as root help. An integration regression test verifies byte-identical stdout, empty stderr, and successful exit status.

### 2026-09-14T13:12:05Z · @jarimustonen

Fixed and verified by the full local repository gate suite before this issue was recorded.

