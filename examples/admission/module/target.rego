# Admission fixture: a __target__ before the rules.
package authz

import rego.v1

__target__ := "rego"

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
