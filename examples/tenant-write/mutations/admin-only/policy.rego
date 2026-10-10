# Polean tenant-write v0 mutation fixture: admin-only.
#
# Purpose: a deliberately weakened policy that drops the owner rule, so
# owner-write-available must fail while the other five claims still hold.
#
# examples/tenant-write/mutations/admin-only/policy.json holds the equivalent
# rule-list IR. validation/validate_design.py re-renders that IR into this
# text, requires the variant to differ from the baseline only inside its rule
# list, and records the counterexample in validation/witnesses/admin-only.json.

package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
    input.subject.role == "admin"
}
