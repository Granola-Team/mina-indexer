# Adding blocks to the witness tree

Here's a visualization of a general block witness tree

<!-- TODO include witness tree with dangling branches -->

There are several scenarios which can occur while adding a block to the witness tree. They fall broadly into two categories, each with several subcategories:

## Simple Extensions

Simple extensions either increase the length of a branch's path(es) by 1 (*proper*) or increase the number of paths in a branch by 1 (*improper*). A simple extension does not decrease the number of connected components of the witness tree. Both the main branch and dangling branches can be simply extended.

### Simple extensions of the main branch

#### Simple proper extension of the main branch

The incoming block is a *child of a leaf block* in the main branch and not a parent of any base blocks of the dangling branches. Adding this block will extend one of the paths on the main branch, increasing it's length by 1.

- new leaf replaces old
- one existing path is updated
- length of one path in the main branch increases by 1

<!-- TODO picture -->

#### Simple improper extension of the main branch

The incoming block is a *child of a non-leaf block* in the main branch and not a parent of any base blocks of the dangling branches. Adding this block will create a new path on the main branch.

<!-- TODO How should pathes in the branch be (re)ordered? New path can be appended to the branch's vec of pathes. -->

- one new leaf added
- one new path added
- number of pathes in the main branch increases by 1

<!-- TODO picture -->

### Simple extensions of dangling branches

Since dangling branches are disjoint from the main branch, they can be properly extended in either direction, *forward* or *backward*.

#### Simple proper forward extension of a dangling branch

The incoming block is a *child of a leaf block* in a dangling branch and not a parent of any base blocks of the other dangling branches. Adding this block will extend one of the paths on the dangling branch, increasing it's length by 1.

- new leaf replaces old
- one existing path is updated
- length of one path in the dangling branch increases by 1

<!-- TODO picture -->

#### Simple proper backward extension of a dangling branch

The incoming block is the *parent of the base block* in a dangling branch and not a child of any known block in the witness tree. Adding this block increases the lengths of all pathes in the dangling branch by 1 by adding a new base to the branch.

- leaves are unchanged
- all existing paths updated with new base
- length of all pathes in the dangling branch increase by 1

<!-- TODO picture -->

#### Simple improper extension of a dangling branch

Similar to the main branch, the incoming block is a *child of a non-leaf block* in a dangling branch and not a parent of any base blocks of the other dangling branches. Adding this block will create a new path on the dangling branch.

- one new leaf added
- one new path added
- number of pathes in the dangling branch increases by 1

<!-- TODO picture -->

### Other simple extensions

It is also possible for the incoming block to not be connected to any of the existing branches. There are two possibilities for this scenario:

#### Incoming block starts a new dangling branch

The incoming block is *not a parent or child of any block in the witness tree* and its height is such that we should retain it in the tree. This block starts a new dangling branch.

<!-- TODO picture -->

#### Incoming block can bypass the witness tree

The incoming block is a *child of a pruned block*. This block can go immediately to the *store*, bypassing the witness tree altogether.

<!-- TODO picture -->
