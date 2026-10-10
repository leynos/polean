# Polean tenant-write v0 mutation fixture: deny-all.
#
# Purpose: a deliberately weakened policy that keeps no non-default rules, so
# both availability claims must fail while the four safety claims still hold.
# This fixture is what stops the remaining claims from passing by vacuity.
#
# examples/tenant-write/mutations/deny-all/policy.json holds the equivalent
# rule-list IR. validation/validate_design.py re-renders that IR into this
# text, requires the variant to differ from the baseline only inside its rule
# list, and records the counterexample in validation/witnesses/deny-all.json.

package authz

import rego.v1

default allow := false
