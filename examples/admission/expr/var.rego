# Admission fixture: a variable comparison.
package authz

import rego.v1

default allow := false

allow if {
    x == x
}
