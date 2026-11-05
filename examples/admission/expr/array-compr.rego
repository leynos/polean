# Admission fixture: an array comprehension.
package authz

import rego.v1

default allow := false

allow if {
    [x | x := 1] == []
}
