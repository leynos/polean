# Admission fixture: a negated literal.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    not input.action == "read"
}
