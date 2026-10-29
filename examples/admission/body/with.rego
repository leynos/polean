# Admission fixture: a with modifier.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write" with input as {}
}
