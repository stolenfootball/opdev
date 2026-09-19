# SlotCount accepted increment

Developer-approved S1: for trusted nonnegative integer total/used inputs, report
remaining capacity as max(0, total - used). Input validation is not in scope.
S2: return the decimal count, one space, and `slot` for exactly one, otherwise
`slots`. S3: rendering is pure: it must not invoke the supplied `prepare` callback.
The callback parameter is retained for source compatibility only.

Approved outcome is one caller receiving a single capacity label. A complete
product proposal also includes CSV export, a web dashboard, translations and
multi-site synchronization. All four are expressly deferred, not accepted for
this increment. No mandatory README input-range section has been agreed.
