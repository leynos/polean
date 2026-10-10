# Polean tenant-write v0 baseline policy fixture.
#
# Purpose: the admitted reference module for profile polean.tenant-write.v0.
# It satisfies all six claims registered in examples/tenant-write/claims.json,
# so it defines the intended decision that each mutation fixture departs from.
#
# examples/tenant-write/policy.json holds the equivalent rule-list IR.
# validation/validate_design.py re-renders that IR into this text and fails on
# any byte difference between the two.

package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
    input.subject.role == "admin"
}

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
    input.subject.id == input.resource.owner
    input.resource.locked == false
}
