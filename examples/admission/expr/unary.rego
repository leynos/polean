# Admission fixture: a unary expression.
package authz

import rego.v1

default allow := false

allow if {
    -x == 0
}
