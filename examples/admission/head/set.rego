# Admission fixture: a partial set rule head.
package authz

import rego.v1

default allow := false

allow contains x if {
    some x
}
