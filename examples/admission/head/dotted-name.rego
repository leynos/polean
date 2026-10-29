# Admission fixture: a dotted rule name.
package authz

import rego.v1

default allow := false

allow.x if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
