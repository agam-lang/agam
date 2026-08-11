# Chapter 25: Metaprogramming, REPL, Notebooks & Tooling

> **Part VI: The Agam Language Programming Guide**  
> **Target Audience**: Advanced Developers & AI Practitioners

---

## 25.1 Interactive JIT REPL (`agamc repl`)

Launch the interactive REPL for rapid evaluation:

```bash
$ agamc repl
Agam v0.1.0 Interactive REPL
>>> let x = 42
x: Int = 42
>>> x * 2
84
```

---

## 25.2 Headless Agent Execution (`agamc exec`)

Execute Agam scripts in headless JSON-stream mode with strict resource limits for AI agent workflows:

```bash
agamc exec --json '{"source": "println(40 + 2)", "memory_limit_mb": 512}'
```

---

## 25.3 Formatter & Linter

```bash
# Format source code
agamc fmt src/main.agam

# Run compiler static linter
agamc lint src/main.agam
```
