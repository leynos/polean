# Admission fixture: an addition.
package authz

import rego.v1

default allow := false

allow if {
    1 + 1 == 2
}
