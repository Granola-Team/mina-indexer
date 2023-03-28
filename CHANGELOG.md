# Changelog

## [Unreleased]

### Added

- [#70](https://github.com/Granola-Team/mina-indexer/pull/70): More complex extension tests
- [#69](https://github.com/Granola-Team/mina-indexer/pull/69): Remove unused dependencies
- [#66](https://github.com/Granola-Team/mina-indexer/pull/66): Add Complex extension tests
- [#65](https://github.com/Granola-Team/mina-indexer/pull/65): More tests refactored
- [#64](https://github.com/Granola-Team/mina-indexer/pull/64): Clean up results and unnecessary previous hash function
- [#63](https://github.com/Granola-Team/mina-indexer/pull/63): Remove outdated comment and unused example block
- [#62](https://github.com/Granola-Team/mina-indexer/pull/62): finish ledger pinning
- [#60](https://github.com/Granola-Team/mina-indexer/pull/60): leaf height fix
- [#59](https://github.com/Granola-Team/mina-indexer/pull/59): Tests: witness tree simple proper extensions
- [#57](https://github.com/Granola-Team/mina-indexer/pull/57): Bump actions/checkout from 1 to 3
- [#56](https://github.com/Granola-Team/mina-indexer/pull/56): Run dependabot daily
- [#55](https://github.com/Granola-Team/mina-indexer/pull/55): Run cargo audit on pull request and main update
- [#53](https://github.com/Granola-Team/mina-indexer/pull/53): Fix previous state hash parsing
- [#52](https://github.com/Granola-Team/mina-indexer/pull/52): Fix nix develop branch mismatch
- [#50](https://github.com/Granola-Team/mina-indexer/pull/50): Add Beautified sequential blocks
- [#47](https://github.com/Granola-Team/mina-indexer/pull/47): Implement simple branch extensions
- [#46](https://github.com/Granola-Team/mina-indexer/pull/46): Add documentation on adding blocks to the witness tree
- [#43](https://github.com/Granola-Team/mina-indexer/pull/43): Add rocksdb blocks store
- [#42](https://github.com/Granola-Team/mina-indexer/pull/42): Improve block parsing code into one module and refactor tests to be more succinct
- [#41](https://github.com/Granola-Team/mina-indexer/pull/41): Fix mina-rs submodule
- [#40](https://github.com/Granola-Team/mina-indexer/pull/40): Add ./result/bin to PATH via nix shellHook and update readme
- [#39](https://github.com/Granola-Team/mina-indexer/pull/39): Fix formatting of badges
- [#37](https://github.com/Granola-Team/mina-indexer/pull/37): Add Security audit badge to README
- [#36](https://github.com/Granola-Team/mina-indexer/pull/36): Modify dockerfile to point to the right binary
- [#35](https://github.com/Granola-Team/mina-indexer/pull/35): Add witness tree documentation
- [#31](https://github.com/Granola-Team/mina-indexer/pull/31): Add cargo-nextest to repo and update CI and precommit hooks to use it
- [#30](https://github.com/Granola-Team/mina-indexer/pull/30): Address Cargo Clippy issues
- [#29](https://github.com/Granola-Team/mina-indexer/pull/29): Add Github CI workflow badge and warning
- [#28](https://github.com/Granola-Team/mina-indexer/pull/28): Add more state data-structures
- [#27](https://github.com/Granola-Team/mina-indexer/pull/27): Beautify blocklogs
- [#26](https://github.com/Granola-Team/mina-indexer/pull/26): Add precommit hook
- [#25](https://github.com/Granola-Team/mina-indexer/pull/25): Add test/ledger unit tests
- [#24](https://github.com/Granola-Team/mina-indexer/pull/24): Add and improve state data-structure
- [#22](https://github.com/Granola-Team/mina-indexer/pull/22): Add first draft of CLI to the indexer
- [#21](https://github.com/Granola-Team/mina-indexer/pull/21): Use anyhow and thiserror libraries
- [#20](https://github.com/Granola-Team/mina-indexer/pull/20): Start working on indexer state data-structure 
- [#19](https://github.com/Granola-Team/mina-indexer/pull/19): Add mina-indexer binary
- [#15](https://github.com/Granola-Team/mina-indexer/pull/15): Add log processing functionality to handle raw logs from the BP
- [#14](https://github.com/Granola-Team/mina-indexer/pull/14): Add aarch64-darwin,x86_64-linux, aarch64-linux, x86_64-darwin, x86_64-windows support to nix build
- [#13](https://github.com/Granola-Team/mina-indexer/pull/13): Add build instructions to the README
- [#12](https://github.com/Granola-Team/mina-indexer/pull/12): Add log preprocessing and deserialization
- [#11](https://github.com/Granola-Team/mina-indexer/pull/11): Add basic nix config to project
- [#9](https://github.com/Granola-Team/mina-indexer/pull/9): Add Dockerfile to project
- [#7](https://github.com/Granola-Team/mina-indexer/pull/7): Add Github CI workflow
- [#6](https://github.com/Granola-Team/mina-indexer/pull/6): License project under MPLv2

### Improvements

- [#68](https://github.com/Granola-Team/mina-indexer/pull/68): Improve test results output
- [#67](https://github.com/Granola-Team/mina-indexer/pull/67): Improve test module structure
- [#61](https://github.com/Granola-Team/mina-indexer/pull/61): upgrades CI to split single job into multiple steps and only build on stable rust
- [#58](https://github.com/Granola-Team/mina-indexer/pull/58): Fix openssl cargo test nix regression 
- [#54](https://github.com/Granola-Team/mina-indexer/pull/54): State improvement
- [#51](https://github.com/Granola-Team/mina-indexer/pull/51): Fix aarch64-darwin build
- [#49](https://github.com/Granola-Team/mina-indexer/pull/49): Fix security audit check in README
- [#48](https://github.com/Granola-Team/mina-indexer/pull/48): Update README to include high level architecture diagram
- [#45](https://github.com/Granola-Team/mina-indexer/pull/45): Change README verbiage
- [#44](https://github.com/Granola-Team/mina-indexer/pull/44): Improve and add more state code
- [#38](https://github.com/Granola-Team/mina-indexer/pull/38): Rename module name of transaction to command
- [#34](https://github.com/Granola-Team/mina-indexer/pull/34): Change to use Chainsafe/mina-rs for log parsing
- [#33](https://github.com/Granola-Team/mina-indexer/pull/33): Move security cargo audit into it's own Github workflow
- [#32](https://github.com/Granola-Team/mina-indexer/pull/32): Nixify beautify blocks script
- [#23](https://github.com/Granola-Team/mina-indexer/pull/23): Add Github actions improvements
- [#18](https://github.com/Granola-Team/mina-indexer/pull/18): Fix broken tests because of incorrect block path
- [#17](https://github.com/Granola-Team/mina-indexer/pull/17): Fix nix darwin build
- [#16](https://github.com/Granola-Team/mina-indexer/pull/16): Change nix to use rust stable channel
- [#10](https://github.com/Granola-Team/mina-indexer/pull/10): Ignore .gitconfig
- [#8](https://github.com/Granola-Team/mina-indexer/pull/8): Update README
