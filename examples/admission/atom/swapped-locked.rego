# Admission fixture: a string in place of the lock boolean.
package authz

import rego.v1

default allow := false

allow if {
    input.resource.locked == "false"
}
