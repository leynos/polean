# Admission fixture: the locked atom alone.
package authz

import rego.v1

default allow := false

allow if {
    input.resource.locked == true
}
