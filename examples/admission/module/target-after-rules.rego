# Admission fixture: a __target__ after the rules.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}

__target__ := "rego"
