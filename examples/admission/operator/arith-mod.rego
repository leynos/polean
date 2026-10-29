# Admission fixture: a remainder.
package authz

import rego.v1

default allow := false

allow if {
    1 % 1 == 0
}
