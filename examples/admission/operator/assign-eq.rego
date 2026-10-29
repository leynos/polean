# Admission fixture: a = unification.
package authz

import rego.v1

default allow := false

allow if {
    x = 1
}
