// Inspection-only fixture. No execution requested by the review.
import assert from 'node:assert/strict';
import { capacityLabel } from '../package/capacity.js';
assert.ok(capacityLabel(8, 3, () => {}));
