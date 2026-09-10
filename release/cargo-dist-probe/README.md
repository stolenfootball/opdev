# cargo-dist feasibility probe

This is an isolated, non-publishing probe, not the active release pipeline.
Run with a checksum-verified cargo-dist 0.32.0 executable:

```sh
python3 scripts/probe_cargo_dist.py --dist /absolute/path/to/dist
```

The current result is `error`: `No GitHub hosting is defined!`. The probe exits
zero when it reproduces that known failure; it does not claim an installer was
generated. Unexpected behavior exits nonzero for review.

Despite documenting `hosting = ["simple"]`, version 0.32.0's
`InstallReceipt::from_metadata` in `cargo-dist/src/tasks.rs` requires GitHub
hosting. It also requires GitHub-shaped repository metadata; the probe names
our internal mirror only to get past that earlier validation. Its sole download
base remains the canonical GitLab release path, and its build command is
`false` so it cannot accidentally rebuild binaries. Source inspected at tag
v0.32.0, commit 6886366640dd4da83d33ba55cc04aa58423cbad2.

A second compatibility issue remains after the hosting failure is resolved:
cargo-dist expects a top-level directory in Unix tar archives, whereas OpDev's
existing archives contain `opdev` at their root. Future adoption must preserve
existing archives and package the same already-built binary additively, then
qualify the generated shell and PowerShell installers before publication.

Do not enable GitHub Releases or remove signature checks to work around this
probe. The first managed-runtime increment uses the existing signed GitLab
archives; cargo-dist adoption is deferred until this probe can generate useful
GitLab-only installers without a local fork or template rewriting.

Sources:
- https://axodotdev.github.io/cargo-dist/book/reference/config.html#simple-download-url
- https://github.com/axodotdev/cargo-dist/blob/v0.32.0/cargo-dist/src/tasks.rs
