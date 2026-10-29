# Admission fixture: a := assignment.
package authz

import rego.v1

default allow := false

allow if {
    x := 1
}
