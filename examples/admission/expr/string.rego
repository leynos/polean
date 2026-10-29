# Admission fixture: a string comparison.
package authz

import rego.v1

default allow := false

allow if {
    "a" == "a"
}
