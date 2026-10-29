# Admission fixture: an escaped spelling of write.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "wr\u0069te"
}
