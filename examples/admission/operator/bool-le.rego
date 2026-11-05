# Admission fixture: a <= comparison.
package authz

import rego.v1

default allow := false

allow if {
    1 <= 2
}
