# Admission fixture: a default for another name.
package authz

import rego.v1

default permit := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
}
