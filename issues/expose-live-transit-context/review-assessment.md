# Assessment: live-context handlers

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Alert IDs and duplicate modes are unvalidated | INCORRECT | — | — | — | HIGH | DROP (Rule 1a) |
| F2 | Trip and StopOnTrip alerts can be silently lost under route filters | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | HIGH | FIX |
| F3 | Present empty alert entity arrays lack unknown-scope warning | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX |
| F4 | Saturated 50-row board warning only appears with mode filters | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX |
| F5 | Operational-time re-filter can hide delayed provider rows | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX |
| F6 | Departure text omits scheduled/estimated and delay evidence | CONFIRMED | REGULAR | IMPROVES | NONE | HIGH | FIX |
| F7 | Mixed heading and row timezones are inconsistent | INCORRECT | — | — | — | HIGH | DROP (Rule 1a) |
| F8 | Invalid configured timezone is detected after provider I/O | INCORRECT | — | — | — | HIGH | DROP (Rule 1a) |
| F9 | Nearby coordinates are parsed twice | CONFIRMED | REGULAR | IMPROVES | NONE | HIGH | FIX |
| F10 | RouteType is mislabeled unknown instead of broad scope | CONFIRMED | OCCASIONAL | IMPROVES | NONE | MED | FIX |
| F11 | Alert text may show Debug casing or blank optional header | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX |
| F12 | Extract common helpers and schema fragments | CONFIRMED | RARE | NEUTRAL | MODERATE | HIGH | DROP (Rule 1b) |

FIX: 8   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 4

No residual met the filing bar; no issue was staged or filed.

## Applied source-backed corrections

The accepted localized fixes are implemented in the owned handler/test files. Command-layer typed ID and duplicate validation, explicit-argument/configured-display timezone behavior, and centrally added schema IDs were verified and therefore not changed. Shared helper/schema extraction, extra provider requests, and client-query redesign were rejected as unsupported or low-value churn. The final deterministic suite covers the material accepted findings. Review evidence is also retained in the worker scratch files `history/review-live-context-handlers.md` and `history/assessment-live-context-handlers.{json,md}` during this run.
