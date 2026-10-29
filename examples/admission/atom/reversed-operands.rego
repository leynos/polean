# Admission fixture: an atom with reversed operands.
package authz

import rego.v1

default allow := false

allow if {
    input.resource.tenant == input.subject.tenant
}
