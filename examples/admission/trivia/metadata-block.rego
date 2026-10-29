# Admission fixture: a METADATA comment block.
# METADATA
# title: locked-owner
# custom:
#   owner: authz
package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
