# Admission fixture: a v1 partial set rule head.
package authz

import rego.v1

default allow := false

allow[x] if {
    some x
}
