# Admission fixture: a some-in literal.
package authz

import rego.v1

default allow := false

allow if {
    some x in [1]
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
