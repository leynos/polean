# Admission fixture: a comment that looks like a body literal.
package authz

import rego.v1
# input.action == "read"

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
