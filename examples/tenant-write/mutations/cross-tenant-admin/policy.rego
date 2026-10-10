# Polean tenant-write v0 mutation fixture: cross-tenant-admin.
#
# Purpose: a deliberately weakened policy that drops the administrator tenant
# guard, so tenant-isolation must fail while the other five claims still hold.
#
# examples/tenant-write/mutations/cross-tenant-admin/policy.json holds the
# equivalent rule-list IR. validation/validate_design.py re-renders that IR into
# this text, requires the variant to differ from the baseline only inside its
# rule list, and records the counterexample in
# validation/witnesses/cross-tenant-admin.json.

package authz

import rego.v1

default allow := false

allow if {
    input.action == "write"
    input.subject.role == "admin"
}

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
    input.subject.id == input.resource.owner
    input.resource.locked == false
}
