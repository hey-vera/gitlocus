<!-- SPDX-License-Identifier: Apache-2.0 -->
# Architecture Decision Records

Why things are the way they are. Each record states the context, the decision, and
what it costs — including the alternatives that were rejected and why.

A decision recorded here is not permanent. It is *explicit*, which is what makes it
possible to argue with later. If you think one is wrong, open a discussion and say
which part of the context has changed.

| # | decision | status |
|---|---|---|
| [0001](0001-evidence-not-a-forge.md) | Build the evidence layer, not a forge | accepted |
| [0002](0002-no-ai-authorship-detection.md) | No AI authorship detection | accepted |
| [0003](0003-evidence-classes.md) | Evidence classes are enforced in the type system | accepted |
| [0004](0004-rust-core-shared-by-cli-and-server.md) | One Rust core, shared by the CLI and the server | accepted |

## Writing a new one

Copy the shape of an existing record. Number sequentially. Keep it short — if it
runs past a page, the decision is probably two decisions.

A record must include what the decision **costs**. A record with only benefits is
not a decision, it is an advertisement.
