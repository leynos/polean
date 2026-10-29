# Admission fixture: an every literal.
package authz

import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    every x in [1] {
        x == 1
    }
}
