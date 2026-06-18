# Documentation style

The tone for `README.md` and `docs/` in this repo. Plain, declarative,
technical. A reader skims and learns, fast: what it is, what it does, how to
run it, how it works.

## Structure — inverted pyramid

Lead with one sentence: what it is (a CLI, a library, a service, a config
bundle) and what it does. Then what it does in detail, then quickstart, then
how-it-works, then the rest. A reader who stops after two sentences still knows
what the thing is.

## Voice — describe what it does

Functional description, one flat register. State what the thing does, not why
it is good or why it exists. The "why", when it earns a place, goes in a
parenthetical or an em-dash aside — not a section header.

- Present tense. Declarative or imperative. Third person.
- Short paragraphs. Scannable headers. Bullets for lists.
- Concrete claims with numbers: tests, tok/s, versions, frame counts.
- Exact commands in fenced code blocks, copy-paste-ready.
- Constraints and scope as plain boundaries: "Does X. Does not do Y." No
  apology, no defense.

## Cut

Hype:

- powerful, seamless, blazing-fast, cutting-edge, revolutionary, delightful,
  magical, robust, elegant, world-class, next-generation.

Filler:

- simply, just, easily, effortlessly, "in no time", "out of the box".

Editorializing — both directions, selling and defending:

- Justification arcs: "Why this exists", "Why we chose X", origin stories,
  motivational framing, "the journey", "imagine a world".
- Defensive framing: "we're strict about", "on the legitimate/right side",
  "defensible", "to stay safe", advocacy, stakes-raising.
- "What it means for us" asides that editorialize instead of stating the fact.
- Exclamation marks. Decorative emoji. Redundant restatement.

## Keep

- Every fact, number, command, caveat, and architectural detail. Concise is
  not shallow.
- Honest limits and scope, stated plainly.
- Legal disclaimers and proof-gating language verbatim — e.g. "capability
  shown; no client data", "not legal advice". Neutralize the register around
  them; never alter the load-bearing wording.
- Safety and hazard warnings stay emphatic — never soften. Keep the warning
  marker, the bold, and the imperative; neutralize only the prose around them,
  never the warning itself.
- True numbers even when they sound impressive. Drop the adjective, keep the
  figure: "2.43x speedup", not "a blazing-fast speedup".

## Examples

Hype to functional:

> Before: "X is a blazing-fast, delightful experience that seamlessly brings
> the magic of voice to your desktop. We're excited to share it!"
>
> After: "X is hold-to-talk dictation for Wayland Linux. Hold a key, speak,
> and the transcript is injected into the focused window. Runs faster-whisper
> (int8) locally — no network calls."

Defensive to neutral:

> Before: "Our response is to be very strict about what we document. This keeps
> the project clearly on the legitimate side of every plausible legal regime."
>
> After: "This project documents standard signals from public sources only.
> The scope is unambiguous across jurisdictions, independent of how the
> repair-rights debate resolves."
