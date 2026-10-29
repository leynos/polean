# Admission fixture: a boolean comparison.
package authz

import rego.v1

default allow := false

allow if {
    true == true
}
