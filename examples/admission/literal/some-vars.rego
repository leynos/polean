# Admission fixture: a some-vars literal.
package authz

import rego.v1

default allow := false

allow if {
    some x, y
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
