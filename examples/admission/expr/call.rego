# Admission fixture: a built-in call.
package authz

import rego.v1

default allow := false

allow if {
    count([]) == 0
}
