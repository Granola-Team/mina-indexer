#  Build Software Faster, Better, and More Accurately

> It's a profoundly simple approach to design: make minimal, accurate
> solutions to real problems, nothing more or less.
>
> ~ Pieter Hintjens

## Purpose

This document outlines the core principles and practices that guide
our software development at Granola. Along with the C4 process, this
document should be treated like a compass when directionless.

## Software Design Principles

* **Problem-Solving:**

  > "Each patch solves exactly a genuine and agreed problem in a
  > brutally minimal fashion."

  Adopting a problem-solving mindset directs efforts toward addressing
  critical issues, leading to more efficient resource use and avoiding
  the development of unnecessary features. Pieter Hintjen's describes
  this process as Simplicity-Oriented Design.

* **Continuous Integration and Delivery:** Emphasize small yet
  consistent incremental changes that collectively yield substantial
  overall improvement. Implement CI/CD pipelines to ensure early,
  frequent, and automated reliable software delivery.

* **Build Deterministic Systems:** Avoid non-determinism by
  eliminating globally shared mutable state and favouring a functional
  programming style. Encapsulate state locally within threads to
  reduce complexity and enhance scalability. Use message passing with
  immutable values to prevent concurrency bugs like race
  conditions. Building a correct program is difficult enough without
  dealing with non-determinism.

* **Murphy's Law:** Design for failure, assuming the worst will
  happen. This approach results in more robust, resilient systems. We
  must be explicit about what error conditions are recoverable. Test
  every failure condition where an error is returned.

* **Don't Break Backwards Compatibility:** Once an API or protocol is
  stable, don't break backward compatibility. Handle new requirements
  by deprecating old APIs and creating new ones. The only exceptions
  are when fixing egregious security errors or when there is unanimous
  consensus.

* **Apply the Single Writer Principle:** Avoid scalability issues and
  code complexity by ensuring that all state mutations are handled by
  a single writer, preventing contention on shared resources.
