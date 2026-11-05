# Admission fixture: a Rego v0 partial set.
package authz

import rego.v1

default allow := false

allow[x] {
    some x
}
