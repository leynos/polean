# Admission fixture: the unlocked atom alone.
package authz

import rego.v1

default allow := false

allow if {
    input.resource.locked == false
}
