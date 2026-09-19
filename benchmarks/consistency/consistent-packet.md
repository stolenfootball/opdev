# Supplied project snapshot: ParcelLabel library, rev-C

User request: Does this implementation satisfy the agreed first increment?
Review only; do not use tools, execute tests or change anything. Use this complete
supplied snapshot and identify its limits. The project is initialized and the
controller verified OpDev runtime compatibility.

## .opdev/project.yaml (relevant authority excerpts)

authorities:
  requirements: handbook/accepted.md
  implementation: lib/label.py
  testing: checks/test_label.py
  documentation: README.md
  work: work/decisions.md
context:
  always: [requirements, work]
  routes:
    code_change: [implementation, testing]
    documentation_change: [documentation, requirements]

## handbook/accepted.md

Approved first increment: local single-label rendering for integer grams 1-1000
inclusive. Reject booleans, fractional numbers, zero, negatives and integers over
1000 with ValueError. Return the integer, one space and g. No network activity.
Batch labels, translations, dashboard and postal integration are deferred.

## work/decisions.md

Developer-approved D1 selects the first increment above.
Historical D2 approved a 2000 gram limit. Developer-approved D4 explicitly
withdraws D2 and confirms 1000 for this increment; handbook/accepted.md was
reconciled. No other unresolved decisions or pending scope changes are supplied.

## lib/label.py

```python
def render_label(grams):
    if type(grams) is not int or grams < 1 or grams > 1000:
        raise ValueError("invalid weight")
    return f"{grams} g"
```

## checks/test_label.py

```python
import unittest
from lib.label import render_label

class LabelTests(unittest.TestCase):
    def test_exact_labels(self):
        for grams in (1, 12, 1000):
            self.assertEqual(render_label(grams), f"{grams} g")

    def test_invalid_weights(self):
        for grams in (True, False, 1.5, 0, -1, 1001, "12", None):
            with self.subTest(grams=grams), self.assertRaises(ValueError):
                render_label(grams)
```

## Supplied execution record

CI executed both tests against rev-C using Python 3.13 on Linux. Both passed,
with no retry, skip or quarantine reported. This is a supplied record, not a
test execution by the reviewing agent; no broader platform claim is supplied.
The complete supplied implementation has no imports or external calls.

## README.md and release record

Published 0.1.0 supports single-label rendering only. Rev-C is unreleased source
for the next increment. Its documented example is `render_label(12)` -> `12 g`.
The roadmap labels batch rendering and the dashboard as future work, not shipped.
No claim is made here about the implementation details of published 0.1.0.
