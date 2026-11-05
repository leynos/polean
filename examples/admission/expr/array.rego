# Admission fixture: an array comparison.
package authz

import rego.v1

default allow := false

allow if {
    [] == []
}
