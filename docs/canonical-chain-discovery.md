# Canonical Chain Discovery

## Overview

The canonical chain discovery algorithm takes a list of precomputed
block paths and classifies them into 3 distinct categories:

1. **Deep Canoinical Blocks:** These are blocks that form a canonical
   chain from the genesis block up to the witness root.

2. **Recent Blocks:** These are blocks that extend the deep canonical
   blocks but do not meet the threshold criteria to be classified as
   deep canonical.

3. **Orphaned Blocks:** These are blocks that aren't part of the deep
   canonical chain or recent blocks.

## Definitions

**Witness Root:** The highest block in the lowest contiguous chain
with a sufficient number of ancestors.

**Canonical Threshold:** The minimum number of consecutive ancestors
blocks that a block must have to be consideredd part of the deep
canonical chain.

## Algorithm

1. Sort blocks by length: Sort block paths in ascending order based on
   their heights.

2. Identify contiguous sequences: Identify the starting indices and
   differences in heights between contiguous sequences of blocks.

3. Find the witness tree root: Determine the root block of the deep
   canonical chain, which has the required number of ancestors
   (canonical threshold).

4. Validate the deep canonical chain: From the witness tree root,
   obtain the `previous state hash` from the precomputed blocks and
   walk back to the genesis block to collect all deep canonical
   blocks.

5. Categorize the remaining blocks: Classify blocks higher than the
   witness tree root as recent blocks and the remaining blocks as
   orphaned blocks.

