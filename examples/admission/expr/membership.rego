# Admission fixture: a membership expression.
package authz

import rego.v1

default allow := false

allow if {
    x in [1]
}
