# Release Notes Template

Use this format for GitHub Releases. Avoid publishing a release body that only contains a compare link.

````markdown
## What Changed

- Short user-facing change.
- Short user-facing change.
- Short user-facing change.

## Why It Matters

Explain the practical impact in 2-4 sentences. Focus on what users can now do, what became safer, or what became easier to verify.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.sh | bash
# follow the interactive dev-flow setup
cd your-project
```

## Upgrade

```bash
dow update
dow self-check
```

## Compatibility

- Claude Code: supported
- Codex CLI: supported
- Kiro CLI: testing

## Full Changelog

https://github.com/daphnee-ovo/dev-flow/compare/<previous>...<current>
````
