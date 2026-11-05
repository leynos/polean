# Admission fixture: an unregistered attribute.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.subject.email == "a@b"
}
