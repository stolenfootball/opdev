# Development

Use Python's standard library. Tests use unittest and should run locally and in
GitLab. No formatter, linter, coverage tool, dependency scanner or accessibility
target has been selected. There is no documented effectiveness evaluation policy.

Feature branches originate from develop and merge into develop. Main is the
release branch: merge develop into main when making a release. Preserve public
CLI output. Users read the terminal output directly, sometimes with screen readers.

The project runs as a local process, not a hosted service. A failure should yield
a useful message and nonzero exit status. There is no automated recovery exercise
or artifact promotion pipeline yet.
